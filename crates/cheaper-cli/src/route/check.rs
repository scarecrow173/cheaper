//! `cheaper route check` — 全 route layer の read-only diagnostics。
//! route/workflow/worker/process は実行せず、sample evaluation と
//! static readiness のみを allowlist 済み report へ投影する。

use std::collections::BTreeSet;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::exec::ResolvedExecPolicy;
use cheaper_core::policy::load::{
    load_policy_documents, LoadPolicyDocuments, PolicyDocumentLayer, PolicyLayerStatus,
    PolicyLayerStatuses,
};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::worker::resolve::ResolveWorkerError;
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_tools::tools::exec::pin_exec_grant;
use cheaper_tools::tools::process::find_cheaper_error;
use cheaper_workflow::WorkerCatalog;
use serde_json::{json, Map, Value};

use crate::cli::{BoxError, CliHooks, CliIo, CommandOut};
use crate::envelope::utility_success_envelope;
use crate::hook::targets::TargetCapability;
use crate::workflow_context::WorkflowContext;

use super::resolve::{self, ExecPolicyView, Viability};
use super::routes::{Route, RouteLayer, RouteLayerStatus, RouteMergeRow, RouteMergeStatus};
use super::supply::{self, RouteCheckSources, RouteLayerSource, RouteWorkflowInput};

const SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Default)]
struct SeverityCounts {
    info: usize,
    warning: usize,
    error: usize,
}

impl SeverityCounts {
    fn add(&mut self, severity: Severity) {
        match severity {
            Severity::Info => self.info += 1,
            Severity::Warning => self.warning += 1,
            Severity::Error => self.error += 1,
        }
    }
}

fn metadata_label(raw: &str) -> String {
    const MAX_CHARS: usize = 64;
    let mut bounded = raw.chars().take(MAX_CHARS).collect::<String>();
    if raw.chars().count() > MAX_CHARS {
        bounded.push('…');
    }
    serde_json::to_string(&bounded).unwrap_or_else(|_| "\"\"".to_string())
}

#[derive(Debug)]
struct Diagnostic {
    code: &'static str,
    severity: Severity,
    layer: &'static str,
    route: Option<String>,
    source: Option<String>,
    message: String,
    remediation: Option<String>,
}

impl Diagnostic {
    fn new(
        code: &'static str,
        severity: Severity,
        layer: &'static str,
        route: Option<&str>,
        source: Option<String>,
        message: impl Into<String>,
        remediation: Option<&'static str>,
    ) -> Self {
        Self {
            code,
            severity,
            layer,
            route: route.map(str::to_string),
            source,
            message: message.into(),
            remediation: remediation.map(str::to_string),
        }
    }

    fn to_json(&self) -> Value {
        let mut out = Map::new();
        out.insert("code".to_string(), json!(self.code));
        out.insert("severity".to_string(), json!(self.severity.as_str()));
        out.insert("layer".to_string(), json!(self.layer));
        if let Some(route) = &self.route {
            out.insert("route".to_string(), json!(route));
        }
        if let Some(source) = &self.source {
            out.insert("source".to_string(), json!(source));
        }
        out.insert("message".to_string(), json!(self.message));
        if let Some(remediation) = &self.remediation {
            out.insert("remediation".to_string(), json!(remediation));
        }
        Value::Object(out)
    }
}

struct PolicyState {
    statuses: PolicyLayerStatuses,
    policy: Option<RuntimePolicy>,
    denylist: Option<PathDenylist>,
    exec_deny: ExecDenyList,
    error: Option<Value>,
}

struct WorkflowEval {
    status: &'static str,
    source: String,
    workers: Vec<Value>,
    diagnostics: Vec<Diagnostic>,
}

struct GrantEval {
    value: Value,
    status: &'static str,
    diagnostics: Vec<Diagnostic>,
}

fn layer_str(layer: RouteLayer) -> &'static str {
    layer.as_str()
}

fn source_kind(source: &RouteLayerSource) -> &'static str {
    match source {
        RouteLayerSource::Filesystem { .. } => "filesystem",
        RouteLayerSource::Embedded { .. } => "embedded",
    }
}

fn route_layer_status(status: RouteLayerStatus) -> &'static str {
    match status {
        RouteLayerStatus::Absent => "absent",
        RouteLayerStatus::Loaded => "loaded",
        RouteLayerStatus::Failed => "failed",
    }
}

fn policy_layer_status(status: PolicyLayerStatus) -> &'static str {
    match status {
        PolicyLayerStatus::Absent => "absent",
        PolicyLayerStatus::Loaded => "loaded",
        PolicyLayerStatus::Failed => "failed",
    }
}

fn policy_document_layer_str(layer: PolicyDocumentLayer) -> &'static str {
    match layer {
        PolicyDocumentLayer::Builtin => "builtin",
        PolicyDocumentLayer::User => "user",
        PolicyDocumentLayer::Invocation => "invocation",
        PolicyDocumentLayer::Project => "project",
        PolicyDocumentLayer::Merged => "merged",
    }
}

fn policy_source(layer: PolicyDocumentLayer) -> String {
    match layer {
        PolicyDocumentLayer::Builtin => "builtin:policy.yaml".to_string(),
        PolicyDocumentLayer::User => "user:policy.yaml".to_string(),
        PolicyDocumentLayer::Project => "project:policy.yaml".to_string(),
        PolicyDocumentLayer::Invocation | PolicyDocumentLayer::Merged => {
            "policy:effective".to_string()
        }
    }
}

fn merge_status(status: RouteMergeStatus) -> &'static str {
    match status {
        RouteMergeStatus::Active => "active",
        RouteMergeStatus::Suppressed => "suppressed",
        RouteMergeStatus::Shadowed => "shadowed",
        RouteMergeStatus::Invalid => "invalid",
        RouteMergeStatus::Indeterminate => "indeterminate",
    }
}

fn viability_str(viability: Viability) -> &'static str {
    match viability {
        Viability::Runnable => "runnable",
        Viability::NoGrant => "no-grant",
        Viability::MissingTarget => "missing-target",
        Viability::InvalidRoute => "invalid-route",
        Viability::UnsafeCommand => "unsafe-command",
        Viability::StdinUnsupported => "stdin-unsupported",
    }
}

/// schema violation text から normalized field label だけを取り出す。
/// `violation` の free-form detail・値・snippet は report に残さない。
fn normalized_field(error: &CheaperError) -> Option<String> {
    fn normalize(label: &str) -> Option<String> {
        if label.is_empty()
            || !label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '[' | ']' | '-'))
        {
            return None;
        }
        Some(label.to_string())
    }

    let details = error.details()?;
    if let Some(field) = details.get("field").and_then(Value::as_str) {
        return normalize(field);
    }
    let violation = details.get("violation").and_then(Value::as_str)?;
    normalize(
        violation
            .split([':', ' '])
            .next()
            .unwrap_or_default()
            .trim(),
    )
}

fn bounded_reason(error: &CheaperError, fallback: &'static str) -> &'static str {
    let Some(reason) = error
        .details()
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
    else {
        return fallback;
    };
    match reason {
        "read-failed" | "yaml-parse" | "not-object" | "schema" | "denied" | "absent" => {
            match reason {
                "read-failed" => "read-failed",
                "yaml-parse" => "yaml-parse",
                "not-object" => "not-object",
                "schema" => "schema",
                "denied" => "denied",
                _ => "absent",
            }
        }
        _ => fallback,
    }
}

fn allowlist_error(
    code: &'static str,
    error: &CheaperError,
    source: &str,
    fallback_reason: &'static str,
    pseudo_layer: Option<&'static str>,
) -> Value {
    let mut out = Map::new();
    out.insert("code".to_string(), json!(code));
    out.insert(
        "reason".to_string(),
        json!(bounded_reason(error, fallback_reason)),
    );
    if let Some(layer) = pseudo_layer {
        out.insert("layer".to_string(), json!(layer));
    }
    out.insert("source".to_string(), json!(source));
    if let Some(field) = normalized_field(error) {
        out.insert("field".to_string(), json!(field));
    }
    Value::Object(out)
}

fn route_layer_error(error: &CheaperError, source: &str) -> Value {
    allowlist_error("route.layer-load", error, source, "invalid", None)
}

fn policy_error(error: &CheaperError, layer: PolicyDocumentLayer, absent: bool) -> Value {
    let fallback = if absent { "absent" } else { "invalid-policy" };
    allowlist_error(
        "route.policy-load",
        error,
        &policy_source(layer),
        fallback,
        Some(policy_document_layer_str(layer)),
    )
}

fn policy_statuses_json(statuses: PolicyLayerStatuses) -> Value {
    json!({
        "builtin": policy_layer_status(statuses.builtin),
        "user": policy_layer_status(statuses.user),
        "project": policy_layer_status(statuses.project),
    })
}

fn evaluate_policy(sources: &RouteCheckSources, counts: &mut SeverityCounts) -> PolicyState {
    let mut documents = Vec::with_capacity(3);
    let mut acquisition_errors = Vec::new();
    for (layer, source) in sources.layers() {
        match source.policy_document(layer) {
            Ok(document) => documents.push(document),
            Err(error) => {
                acquisition_errors.push((layer, error));
                documents.push(None);
            }
        }
    }

    let options = LoadPolicyDocuments {
        builtin: documents[0].as_ref(),
        user: documents[1].as_ref(),
        invocation: None,
        project: documents[2].as_ref(),
    };
    let loaded = load_policy_documents(&options);
    let mut statuses = match &loaded {
        Ok(loaded) => loaded.layers,
        Err(error) => error.layers,
    };
    for (layer, _) in &acquisition_errors {
        match layer {
            RouteLayer::Builtin => statuses.builtin = PolicyLayerStatus::Failed,
            RouteLayer::User => statuses.user = PolicyLayerStatus::Failed,
            RouteLayer::Project => statuses.project = PolicyLayerStatus::Failed,
        }
    }

    let mut error_projection = None;
    if let Some((layer, error)) = acquisition_errors.first() {
        counts.add(Severity::Error);
        let policy_layer = match layer {
            RouteLayer::Builtin => PolicyDocumentLayer::Builtin,
            RouteLayer::User => PolicyDocumentLayer::User,
            RouteLayer::Project => PolicyDocumentLayer::Project,
        };
        error_projection = Some(policy_error(error, policy_layer, false));
        for _ in acquisition_errors.iter().skip(1) {
            counts.add(Severity::Error);
        }
        return PolicyState {
            statuses,
            policy: None,
            denylist: None,
            exec_deny: ExecDenyList::default(),
            error: error_projection,
        };
    }

    let loaded = match loaded {
        Ok(loaded) => loaded,
        Err(load_error) => {
            counts.add(Severity::Error);
            let absent = matches!(load_error.layer, PolicyDocumentLayer::Builtin)
                && statuses.builtin == PolicyLayerStatus::Absent;
            error_projection = Some(policy_error(&load_error.error, load_error.layer, absent));
            return PolicyState {
                statuses,
                policy: None,
                denylist: None,
                exec_deny: ExecDenyList::default(),
                error: error_projection,
            };
        }
    };

    let deny: Vec<&str> = loaded
        .policy
        .filesystem
        .deny_paths
        .iter()
        .map(String::as_str)
        .collect();
    let denylist = create_path_denylist(&deny);
    let exec_deny = match ExecDenyList::compile(&loaded.policy.security.exec_deny_patterns) {
        Ok(exec_deny) => exec_deny,
        Err(error) => {
            counts.add(Severity::Error);
            error_projection = Some(policy_error(&error, PolicyDocumentLayer::Merged, false));
            return PolicyState {
                statuses,
                policy: None,
                denylist: None,
                exec_deny: ExecDenyList::default(),
                error: error_projection,
            };
        }
    };
    PolicyState {
        statuses,
        policy: Some(loaded.policy),
        denylist: Some(denylist),
        exec_deny,
        error: error_projection,
    }
}

fn layer_reports(
    evaluation: &crate::route::supply::RouteCheckRouteEvaluation,
    sources: &RouteCheckSources,
    counts: &mut SeverityCounts,
) -> Vec<Value> {
    evaluation
        .layers
        .iter()
        .zip(sources.layers().into_iter().map(|(_, source)| source))
        .map(|(layer_eval, source)| {
            let error = layer_eval.error.as_ref().map(|error| {
                counts.add(Severity::Error);
                route_layer_error(error, &layer_eval.source)
            });
            json!({
                "layer": layer_str(layer_eval.layer),
                "sourceKind": source_kind(source),
                "status": route_layer_status(layer_eval.status),
                "routeCount": layer_eval.routes.len(),
                "invalidCount": layer_eval
                    .invalid
                    .iter()
                    .map(|group| group.names.len())
                    .sum::<usize>(),
                "error": error.unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn merge_report(evaluation: &crate::route::supply::RouteCheckRouteEvaluation) -> Value {
    let mut active = 0;
    let mut suppressed = 0;
    let mut shadowed = 0;
    let mut invalid = 0;
    let mut indeterminate = 0;
    for row in &evaluation.merged.rows {
        match row.status() {
            RouteMergeStatus::Active => active += 1,
            RouteMergeStatus::Suppressed => suppressed += 1,
            RouteMergeStatus::Shadowed => shadowed += 1,
            RouteMergeStatus::Invalid => invalid += 1,
            RouteMergeStatus::Indeterminate => indeterminate += 1,
        }
    }
    json!({
        "active": active,
        "suppressed": suppressed,
        "shadowed": shadowed,
        "invalid": invalid,
        "indeterminate": indeterminate,
        "effective": evaluation.merged.effective.routes.len(),
    })
}

fn worker_error_code(error: Option<&CheaperError>) -> &'static str {
    match error.map(CheaperError::code) {
        Some(ErrorCode::WorkerNotFound) => "route.worker-missing",
        Some(
            ErrorCode::InputSchemaNotFound
            | ErrorCode::InvalidInputSchema
            | ErrorCode::OutputSchemaNotFound
            | ErrorCode::InvalidOutputSchema,
        ) => "route.worker-schema",
        _ => "route.worker-invalid",
    }
}

fn worker_evals(
    raw: &Value,
    sources: &RouteCheckSources,
    route: &Route,
) -> (Vec<Value>, Vec<Diagnostic>) {
    let index = sources.resolve_worker_index(raw);
    let mut workers = Vec::new();
    let mut diagnostics = Vec::new();
    for name in index.names() {
        match index.get(name) {
            Ok(Some(worker)) => workers.push(json!({
                "name": name,
                "status": "valid",
                "origin": worker.origin.as_str(),
            })),
            Ok(None) => {
                let (status, code) = match sources.resolve_worker(name) {
                    Err(ResolveWorkerError::Cheaper(error)) => {
                        let code = worker_error_code(Some(&error));
                        (
                            if code == "route.worker-missing" {
                                "missing"
                            } else {
                                "invalid"
                            },
                            code,
                        )
                    }
                    Err(_) => ("invalid", "route.worker-invalid"),
                    Ok(_) => ("invalid", "route.worker-invalid"),
                };
                workers.push(json!({"name": name, "status": status}));
                diagnostics.push(Diagnostic::new(
                    code,
                    Severity::Error,
                    layer_str(route.layer),
                    Some(&route.name),
                    None,
                    format!("worker {} is {status}", metadata_label(name)),
                    Some("Fix the referenced worker declaration or schema."),
                ));
            }
            Err(error) => {
                let code = worker_error_code(find_cheaper_error(error.as_ref()));
                workers.push(json!({"name": name, "status": "invalid"}));
                diagnostics.push(Diagnostic::new(
                    code,
                    Severity::Error,
                    layer_str(route.layer),
                    Some(&route.name),
                    None,
                    format!("worker {} is invalid", metadata_label(name)),
                    Some("Fix the referenced worker declaration or schema."),
                ));
            }
        }
    }
    (workers, diagnostics)
}

async fn evaluate_workflow(
    route: &Route,
    sources: &RouteCheckSources,
    context: Option<&WorkflowContext>,
) -> WorkflowEval {
    let layer = layer_str(route.layer);
    let input = sources
        .layers()
        .into_iter()
        .find(|(source_layer, _)| *source_layer == route.layer)
        .map(|(_, source)| source.workflow_input(route))
        .expect("route layer source exists");
    let (input, source) = match input {
        RouteWorkflowInput::Missing { source } => {
            return WorkflowEval {
                status: "missing",
                source: source.clone(),
                workers: Vec::new(),
                diagnostics: vec![Diagnostic::new(
                    "route.workflow-missing",
                    Severity::Warning,
                    layer,
                    Some(&route.name),
                    Some(source),
                    "route target workflow is missing",
                    Some("Create the declared workflow or update target.path."),
                )],
            };
        }
        RouteWorkflowInput::Document(document) => (
            RouteWorkflowInput::Document(document.clone()),
            document.source,
        ),
        RouteWorkflowInput::File {
            source,
            file_root,
            path,
        } => (
            RouteWorkflowInput::File {
                source: source.clone(),
                file_root,
                path,
            },
            source,
        ),
    };

    if matches!(&input, RouteWorkflowInput::File { .. }) && !sources.target_exists(route) {
        return WorkflowEval {
            status: "missing",
            source: source.clone(),
            workers: Vec::new(),
            diagnostics: vec![Diagnostic::new(
                "route.workflow-missing",
                Severity::Warning,
                layer,
                Some(&route.name),
                Some(source),
                "route target workflow is missing",
                Some("Create the declared workflow or update target.path."),
            )],
        };
    }

    let Some(context) = context else {
        // workflow compiler/tool capability は policy dependent。embedded
        // document だけは policy 非依存の worker/schema validity を評価する。
        let (raw, workers, mut diagnostics) = match &input {
            RouteWorkflowInput::Document(document) => {
                match serde_json::from_str::<Value>(&document.text) {
                    Ok(raw) => {
                        let (workers, diagnostics) = worker_evals(&raw, sources, route);
                        (Some(raw), workers, diagnostics)
                    }
                    Err(_) => {
                        let mut diagnostics = vec![Diagnostic::new(
                            "route.workflow-invalid",
                            Severity::Error,
                            layer,
                            Some(&route.name),
                            Some(source.clone()),
                            "route target workflow is invalid",
                            Some("Fix the workflow JSON definition."),
                        )];
                        return WorkflowEval {
                            status: "invalid",
                            source,
                            workers: Vec::new(),
                            diagnostics: {
                                diagnostics.push(Diagnostic::new(
                                    "route.workflow-unknown",
                                    Severity::Error,
                                    layer,
                                    Some(&route.name),
                                    None,
                                    "policy-dependent workflow validation is unavailable",
                                    Some("Fix the policy failure and rerun route check."),
                                ));
                                diagnostics
                            },
                        };
                    }
                }
            }
            _ => (None, Vec::new(), Vec::new()),
        };
        let _ = raw;
        diagnostics.push(Diagnostic::new(
            "route.workflow-unknown",
            Severity::Error,
            layer,
            Some(&route.name),
            None,
            "policy-dependent workflow validation is unavailable",
            Some("Fix the policy failure and rerun route check."),
        ));
        return WorkflowEval {
            status: "unknown",
            source,
            workers,
            diagnostics,
        };
    };

    let loaded = match &input {
        RouteWorkflowInput::Document(document) => {
            context
                .load_workflow_document(&document.text, |raw| sources.resolve_worker_index(raw))
                .await
        }
        RouteWorkflowInput::File {
            file_root, path, ..
        } => {
            context
                .load_workflow_from_with(path, file_root, |raw| sources.resolve_worker_index(raw))
                .await
        }
        RouteWorkflowInput::Missing { .. } => unreachable!("missing returned early"),
    };

    let loaded = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            let cheaper = find_cheaper_error(error.as_ref());
            let code = cheaper.map(CheaperError::code);
            let diagnostic_code = match code {
                Some(ErrorCode::WorkerNotFound) => "route.worker-missing",
                Some(ErrorCode::InvalidRequest)
                    if cheaper
                        .map(CheaperError::message)
                        .is_some_and(|m| m.starts_with("unknown worker:")) =>
                {
                    "route.worker-missing"
                }
                Some(
                    ErrorCode::InvalidWorkerName
                    | ErrorCode::InvalidWorkerDefinition
                    | ErrorCode::WorkerNameMismatch,
                ) => "route.worker-invalid",
                Some(
                    ErrorCode::InputSchemaNotFound
                    | ErrorCode::InvalidInputSchema
                    | ErrorCode::OutputSchemaNotFound
                    | ErrorCode::InvalidOutputSchema,
                ) => "route.worker-schema",
                _ => "route.workflow-invalid",
            };
            let message = match diagnostic_code {
                "route.worker-missing" => "route target workflow references a missing worker",
                "route.worker-invalid" => "route target workflow references an invalid worker",
                "route.worker-schema" => {
                    "route target workflow references a worker with an invalid schema"
                }
                _ => "route target workflow failed validation",
            };
            return WorkflowEval {
                status: "invalid",
                source: source.clone(),
                workers: Vec::new(),
                diagnostics: vec![Diagnostic::new(
                    diagnostic_code,
                    Severity::Error,
                    layer,
                    Some(&route.name),
                    Some(source),
                    message,
                    Some("Fix the workflow definition or its referenced resources."),
                )],
            };
        }
    };

    let raw = loaded.definition.raw();
    if crate::workflow_run::validate_routed_convention(raw).is_err() {
        return WorkflowEval {
            status: "invalid",
            source: source.clone(),
            workers: Vec::new(),
            diagnostics: vec![Diagnostic::new(
                "route.workflow-convention",
                Severity::Error,
                layer,
                Some(&route.name),
                Some(source),
                "route target workflow does not satisfy the routed convention",
                Some("Fix the workflow input/output contract for routed execution."),
            )],
        };
    }
    let (workers, diagnostics) = worker_evals(raw, sources, route);
    WorkflowEval {
        status: "valid",
        source,
        workers,
        diagnostics,
    }
}

fn grant_projection(
    route: &Route,
    name: &str,
    policy: Option<&RuntimePolicy>,
    root: &str,
    denylist: Option<&PathDenylist>,
    exec_deny: &ExecDenyList,
) -> (Value, &'static str, Option<Diagnostic>) {
    let layer = layer_str(route.layer);
    let Some(policy) = policy else {
        return (json!({"name": name, "status": "unknown"}), "unknown", None);
    };
    let Some(grant) = policy.exec.grants.get(name) else {
        return (
            json!({"name": name, "status": "missing"}),
            "missing",
            Some(Diagnostic::new(
                "route.grant-missing",
                Severity::Warning,
                layer,
                Some(&route.name),
                None,
                format!("grant {} is not declared", metadata_label(name)),
                Some("Declare the grant in trusted policy or update the route grant reference."),
            )),
        );
    };
    if !ResolvedExecPolicy::is_route_eligible(grant) {
        return (
            json!({"name": name, "status": "ineligible"}),
            "ineligible",
            Some(Diagnostic::new(
                "route.grant-ineligible",
                Severity::Warning,
                layer,
                Some(&route.name),
                None,
                format!("grant {} is not route-eligible", metadata_label(name)),
                Some("Use a trusted, non-fixed grant for routed execution."),
            )),
        );
    }
    match pin_exec_grant(
        name,
        grant,
        root,
        policy.filesystem.symlinks,
        denylist,
        exec_deny,
    ) {
        Ok(_) => (
            json!({"name": name, "status": "environment-dependent"}),
            "environment-dependent",
            None,
        ),
        Err(_) => (
            json!({"name": name, "status": "pin-unresolved"}),
            "pin-unresolved",
            Some(Diagnostic::new(
                "route.grant-pin-unresolved",
                Severity::Warning,
                layer,
                Some(&route.name),
                None,
                format!(
                    "grant {} cannot be pinned in the current environment",
                    metadata_label(name)
                ),
                Some("Check the grant executable and PATH-dependent pin policy."),
            )),
        ),
    }
}

fn evaluate_grants(route: &Route, policy_state: &PolicyState, root: &str) -> GrantEval {
    let mut names = BTreeSet::new();
    names.insert(route.grant.clone());
    names.extend(route.grants.iter().cloned());
    if let Some(substitute) = &route.substitute {
        names.insert(substitute.grant.clone());
    }

    let mut anchor = Value::Null;
    let mut members = Vec::new();
    let mut diagnostics = Vec::new();
    let mut status = "environment-dependent";
    for name in names {
        let (projection, grant_status, diagnostic) = grant_projection(
            route,
            &name,
            policy_state.policy.as_ref(),
            root,
            policy_state.denylist.as_ref(),
            &policy_state.exec_deny,
        );
        if name == route.grant {
            anchor = projection;
        } else {
            members.push(projection);
        }
        if let Some(diagnostic) = diagnostic {
            diagnostics.push(diagnostic);
        }
        status = match (status, grant_status) {
            ("environment-dependent", "environment-dependent") => "environment-dependent",
            (_, "unknown") | ("unknown", _) => "unknown",
            (_, "missing") | ("missing", _) => "missing",
            (_, "ineligible") | ("ineligible", _) => "ineligible",
            (_, "pin-unresolved") | ("pin-unresolved", _) => "pin-unresolved",
            _ => status,
        };
    }
    GrantEval {
        value: json!({
            "status": status,
            "anchor": anchor,
            "members": members,
        }),
        status,
        diagnostics,
    }
}

fn substitute_json(route: &Route) -> Value {
    match &route.substitute {
        Some(substitute) => json!({
            "status": "declared",
            "grant": substitute.grant,
            "argvCount": substitute.argv.len(),
        }),
        None => json!({"status": "not-applicable"}),
    }
}

fn targets_json(policy: Option<&RuntimePolicy>) -> Value {
    Value::Array(
        TargetCapability::all()
            .iter()
            .map(|capability| {
                let rewrite_policy = policy.map(|policy| &policy.route.rewrite);
                json!({
                    "target": capability.target.as_str(),
                    "rewrite": capability.rewrite.as_str(),
                    "approvalBypass": capability.approval_bypass_status(rewrite_policy).as_str(),
                    "registration": capability.registration.as_str(),
                    "approval": capability.approval.as_str(),
                    "execution": capability.execution.as_str(),
                })
            })
            .collect(),
    )
}

fn sample_json(
    row: &RouteMergeRow,
    sample_provided: bool,
    layer_failure: bool,
    matched: Option<&resolve::RouteCommandMatch<'_>>,
    resolution: Option<&resolve::RouteResolution>,
    policy_loaded: bool,
) -> (Value, Vec<Diagnostic>) {
    let layer = layer_str(row.layer());
    let name = row.name().to_string();
    if !sample_provided {
        return (
            json!({
                "provided": false,
                "status": "requires-sample-input",
                "matched": "not-applicable",
                "selected": "not-applicable",
            }),
            vec![Diagnostic::new(
                "route.requires-sample-input",
                Severity::Info,
                layer,
                Some(&name),
                None,
                "sample input is required for dynamic route viability",
                None,
            )],
        );
    }
    if layer_failure {
        return (
            json!({
                "provided": true,
                "status": "indeterminate",
                "matched": "indeterminate",
                "selected": "indeterminate",
            }),
            vec![Diagnostic::new(
                "route.sample-indeterminate",
                Severity::Error,
                layer,
                Some(&name),
                None,
                "route sample evaluation is indeterminate because a route layer failed",
                Some("Fix the failed route layer and rerun route check."),
            )],
        );
    }
    let matched = matched.expect("sample without layer failure has match state");
    let row_matched = matched.matched_command(row.command());
    let row_selected = row.route().is_some_and(|route| matched.is_selected(route));
    let mut sample = Map::new();
    sample.insert("provided".to_string(), json!(true));
    sample.insert("status".to_string(), json!("evaluated"));
    sample.insert("matched".to_string(), json!(row_matched));
    sample.insert("selected".to_string(), json!(row_selected));
    let mut diagnostics = Vec::new();
    if row_selected {
        let viability = resolution
            .and_then(|resolution| resolution.viability)
            .map(viability_str)
            .unwrap_or("unknown");
        sample.insert("viability".to_string(), json!(viability));
        if let Some(suppressed) = resolution
            .and_then(|resolution| resolution.suppressed.as_ref())
            .or(matched.suppressed.as_ref())
        {
            sample.insert("suppressed".to_string(), json!(suppressed));
        }
        match viability {
            "unsafe-command" => diagnostics.push(Diagnostic::new(
                "route.unsafe-command",
                Severity::Info,
                layer,
                Some(&name),
                None,
                "sample command uses unsafe shell syntax",
                None,
            )),
            "stdin-unsupported" => diagnostics.push(Diagnostic::new(
                "route.stdin-unsupported",
                Severity::Warning,
                layer,
                Some(&name),
                None,
                "sample command requires stdin that routed execution does not provide",
                None,
            )),
            "no-grant" => diagnostics.push(Diagnostic::new(
                "route.no-grant",
                Severity::Warning,
                layer,
                Some(&name),
                None,
                "selected route does not have a usable grant for the sample",
                Some("Declare a route-eligible grant or choose a different sample."),
            )),
            "unknown" if !policy_loaded => {}
            _ => {}
        }
    }
    (Value::Object(sample), diagnostics)
}

struct RowEval<'a> {
    sources: &'a RouteCheckSources,
    policy_state: &'a PolicyState,
    context: Option<&'a WorkflowContext>,
    sample_provided: bool,
    layer_failure: bool,
    matched: Option<&'a resolve::RouteCommandMatch<'a>>,
    resolution: Option<&'a resolve::RouteResolution>,
    counts: &'a mut SeverityCounts,
}

fn selected_viability(row: &RouteMergeRow, eval: &RowEval<'_>) -> Option<Viability> {
    let route = row.route()?;
    if !eval
        .matched
        .is_some_and(|matched| matched.is_selected(route))
    {
        return None;
    }
    eval.resolution.and_then(|resolution| resolution.viability)
}

async fn route_row(row: &RouteMergeRow, eval: &mut RowEval<'_>) -> Value {
    let layer = layer_str(row.layer());
    let status = row.status();
    let mut diagnostics = Vec::new();
    match status {
        RouteMergeStatus::Invalid => diagnostics.push(Diagnostic::new(
            "route.invalid",
            Severity::Error,
            layer,
            Some(row.name()),
            None,
            "route declaration is invalid",
            Some("Fix the duplicate or invalid route declaration."),
        )),
        RouteMergeStatus::Suppressed => diagnostics.push(Diagnostic::new(
            "route.suppressed",
            Severity::Warning,
            layer,
            Some(row.name()),
            None,
            "route is suppressed by a lower-layer declaration with the same match",
            Some("Rename the project route or remove the lower-layer collision."),
        )),
        RouteMergeStatus::Shadowed => diagnostics.push(Diagnostic::new(
            "route.shadowed",
            Severity::Warning,
            layer,
            Some(row.name()),
            None,
            "route is shadowed by a user-layer declaration",
            Some("Rename or remove the user-layer override to reactivate this route."),
        )),
        RouteMergeStatus::Indeterminate => diagnostics.push(Diagnostic::new(
            "route.indeterminate",
            Severity::Error,
            layer,
            Some(row.name()),
            None,
            "route merge status is indeterminate because a sibling layer failed",
            Some("Fix the failed route layer and rerun route check."),
        )),
        RouteMergeStatus::Active => {}
    }

    let (
        workflow,
        grants,
        substitute,
        exec_readiness,
        targets,
        sample,
        route_command_len,
        invalid_reason,
    ) = match row {
        RouteMergeRow::Invalid {
            command,
            invalid_reason,
            ..
        } => (
            json!({"status": "not-applicable"}),
            json!({"status": "not-applicable"}),
            json!({"status": "not-applicable"}),
            json!({"status": "not-applicable"}),
            Value::String("not-applicable".to_string()),
            Value::Null,
            command.len(),
            Some(*invalid_reason),
        ),
        RouteMergeRow::Route { route, .. } => {
            let workflow_eval = evaluate_workflow(route, eval.sources, eval.context).await;
            diagnostics.extend(workflow_eval.diagnostics);
            let grants_eval = evaluate_grants(route, eval.policy_state, &eval.sources.root);
            diagnostics.extend(grants_eval.diagnostics);
            let exec_status = if eval.policy_state.policy.is_none() {
                "unknown"
            } else if !eval.sources.target_exists(route) {
                "missing-target"
            } else {
                grants_eval.status
            };
            let workflow = json!({
                "status": workflow_eval.status,
                "source": workflow_eval.source,
                "workers": workflow_eval.workers,
            });
            let sample = sample_json(
                row,
                eval.sample_provided,
                eval.layer_failure,
                eval.matched,
                eval.resolution,
                eval.policy_state.policy.is_some(),
            );
            diagnostics.extend(sample.1);
            if selected_viability(row, eval) == Some(Viability::MissingTarget) {
                // sample viability の error severity は same-cause diagnostic
                // を増やさず `route.workflow-missing` に適用する。
                for diagnostic in diagnostics
                    .iter_mut()
                    .filter(|diagnostic| diagnostic.code == "route.workflow-missing")
                {
                    diagnostic.severity = Severity::Error;
                }
            }
            let targets = targets_json(eval.policy_state.policy.as_ref());
            diagnostics.push(Diagnostic::new(
                "route.renderer-unverified",
                Severity::Info,
                layer,
                Some(&route.name),
                None,
                "host registration, approval, and execution are unverified",
                None,
            ));
            (
                workflow,
                grants_eval.value,
                substitute_json(route),
                json!({"status": exec_status}),
                targets,
                sample.0,
                route.command.len(),
                None,
            )
        }
    };
    if matches!(status, RouteMergeStatus::Invalid) {
        let sample = sample_json(
            row,
            eval.sample_provided,
            eval.layer_failure,
            eval.matched,
            eval.resolution,
            eval.policy_state.policy.is_some(),
        );
        diagnostics.extend(sample.1);
        let mut out = Map::new();
        out.insert("name".to_string(), json!(row.name()));
        out.insert("layer".to_string(), json!(layer));
        out.insert(
            "trust".to_string(),
            json!(if row.trust() == super::routes::RouteTrust::Trusted {
                "trusted"
            } else {
                "untrusted"
            }),
        );
        out.insert(
            "match".to_string(),
            json!({"commandTokens": route_command_len}),
        );
        out.insert("configuration".to_string(), json!(merge_status(status)));
        if let Some(reason) = invalid_reason {
            out.insert("invalidReason".to_string(), json!(reason));
        }
        out.insert("workflow".to_string(), workflow);
        out.insert("grants".to_string(), grants);
        out.insert("substitute".to_string(), substitute);
        out.insert("execReadiness".to_string(), exec_readiness);
        out.insert("targets".to_string(), targets);
        out.insert("sample".to_string(), sample.0);
        out.insert(
            "diagnostics".to_string(),
            Value::Array(diagnostics.iter().map(Diagnostic::to_json).collect()),
        );
        for diagnostic in &diagnostics {
            eval.counts.add(diagnostic.severity);
        }
        return Value::Object(out);
    }

    let mut out = Map::new();
    out.insert("name".to_string(), json!(row.name()));
    out.insert("layer".to_string(), json!(layer));
    out.insert(
        "trust".to_string(),
        json!(if row.trust() == super::routes::RouteTrust::Trusted {
            "trusted"
        } else {
            "untrusted"
        }),
    );
    out.insert(
        "match".to_string(),
        json!({"commandTokens": route_command_len}),
    );
    out.insert("configuration".to_string(), json!(merge_status(status)));
    out.insert("workflow".to_string(), workflow);
    out.insert("grants".to_string(), grants);
    out.insert("substitute".to_string(), substitute);
    out.insert("execReadiness".to_string(), exec_readiness);
    out.insert("targets".to_string(), targets);
    out.insert("sample".to_string(), sample);
    out.insert(
        "diagnostics".to_string(),
        Value::Array(diagnostics.iter().map(Diagnostic::to_json).collect()),
    );
    for diagnostic in &diagnostics {
        eval.counts.add(diagnostic.severity);
    }
    Value::Object(out)
}

fn input_sample(
    command: &Option<String>,
    argv: &Option<String>,
) -> Result<Option<(Vec<String>, bool)>, CheaperError> {
    match (command, argv) {
        (Some(command), None) => crate::route_cmd::parse_command(command).map(Some),
        (None, Some(argv)) => crate::route_cmd::parse_argv(argv).map(|argv| Some((argv, false))),
        (None, None) => Ok(None),
        (Some(_), Some(_)) => unreachable!("route check parse guarantees at-most-one"),
    }
}

fn sanitize_internal_error(error: BoxError) -> BoxError {
    let is_input = error.downcast_ref::<CheaperError>().is_some_and(|e| {
        matches!(
            e.code(),
            ErrorCode::InvalidRequest | ErrorCode::InvalidInput
        )
    });
    if is_input {
        return error;
    }
    Box::new(CheaperError::new(
        ErrorCode::InternalError,
        "route check failed",
    ))
}

async fn build_report(
    sources: &RouteCheckSources,
    sample: Option<(Vec<String>, bool)>,
) -> (Map<String, Value>, i32) {
    let mut counts = SeverityCounts::default();
    let evaluation = sources.evaluate_route_layers();
    let policy_state = evaluate_policy(sources, &mut counts);
    let context = policy_state.policy.as_ref().map(|policy| {
        WorkflowContext::from_policy(sources.root.clone(), sources.dirs.clone(), policy.clone())
    });

    let layer_failure = evaluation
        .layers
        .iter()
        .any(|layer| layer.status == RouteLayerStatus::Failed);
    let sample_provided = sample.is_some();
    let (sample_argv, unsafe_command) = sample.unwrap_or_default();
    let matched = if sample_provided && !layer_failure {
        Some(resolve::match_command(
            &sample_argv,
            &evaluation.merged.effective,
        ))
    } else {
        None
    };
    let resolution = if sample_provided && !layer_failure {
        policy_state.policy.as_ref().and_then(|policy| {
            policy_state.denylist.as_ref().map(|denylist| {
                let view = ExecPolicyView {
                    policy: &policy.exec,
                    root: &sources.root,
                    symlink_policy: policy.filesystem.symlinks,
                    denylist: Some(denylist),
                    exec_deny: &policy_state.exec_deny,
                };
                resolve::resolve_with_target_exists(
                    &resolve::ResolveInput {
                        argv: &sample_argv,
                        cwd: &sources.root,
                        routes: &evaluation.merged.effective,
                        exec_policy: &view,
                        unsafe_command,
                    },
                    |route| sources.target_exists(route),
                )
                .resolution
            })
        })
    } else {
        None
    };

    let mut routes = Vec::new();
    for row in &evaluation.merged.rows {
        let mut eval = RowEval {
            sources,
            policy_state: &policy_state,
            context: context.as_ref(),
            sample_provided,
            layer_failure,
            matched: matched.as_ref(),
            resolution: resolution.as_ref(),
            counts: &mut counts,
        };
        routes.push(route_row(row, &mut eval).await);
    }

    let layers = layer_reports(&evaluation, sources, &mut counts);
    let mut policy = Map::new();
    policy.insert(
        "status".to_string(),
        json!(if policy_state.policy.is_some() {
            "loaded"
        } else {
            "failed"
        }),
    );
    policy.insert(
        "layers".to_string(),
        policy_statuses_json(policy_state.statuses),
    );
    policy.insert(
        "error".to_string(),
        policy_state.error.clone().unwrap_or(Value::Null),
    );

    let mut summary = Map::new();
    summary.insert("totalRoutes".to_string(), json!(routes.len()));
    summary.insert(
        "activeRoutes".to_string(),
        json!(evaluation.merged.effective.routes.len()),
    );
    summary.insert(
        "diagnostics".to_string(),
        json!({
            "info": counts.info,
            "warning": counts.warning,
            "error": counts.error,
        }),
    );

    let mut report = Map::new();
    report.insert("schemaVersion".to_string(), json!(SCHEMA_VERSION));
    report.insert("sampleProvided".to_string(), json!(sample_provided));
    report.insert("merge".to_string(), merge_report(&evaluation));
    report.insert("layers".to_string(), Value::Array(layers));
    report.insert("policy".to_string(), Value::Object(policy));
    report.insert("routes".to_string(), Value::Array(routes));
    report.insert("summary".to_string(), Value::Object(summary));

    let exit_code = if counts.error > 0 {
        3
    } else if counts.warning > 0 {
        1
    } else {
        0
    };
    (report, exit_code)
}

/// `route check` dispatch。sample parse・cwd canonicalize 以外の
/// diagnosable failure は report diagnostic へ fold し、escape した
/// 非入力 error は detail/cause を持たない InternalError へ wrap する。
pub(crate) async fn execute(
    command: &Option<String>,
    argv: &Option<String>,
    cwd: &Option<String>,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let sample = input_sample(command, argv)
        .map_err(|error| Box::new(error) as BoxError)
        .map_err(sanitize_internal_error)?;
    let sources = supply::check_sources(cwd.as_deref().unwrap_or(&io.cwd), io.env, hooks)
        .map_err(|error| sanitize_internal_error(Box::new(error)))?;
    let (report, exit_code) = build_report(&sources, sample).await;
    Ok(CommandOut {
        envelope: utility_success_envelope(report, crate::cli::now(hooks) - started_at),
        exit_code,
        model_line: None,
    })
}

fn text_field(value: Option<&Value>) -> String {
    let raw = value.and_then(Value::as_str).unwrap_or_default();
    let encoded = serde_json::to_string(raw).unwrap_or_else(|_| "\"\"".to_string());
    encoded[1..encoded.len() - 1].to_string()
}

/// dedicated `--format text` renderer。layer → route → diagnostic の
/// deterministic lines のみを使い、raw sample/host path/error detail は
/// report 側の allowlist projection に限定される。
pub(crate) fn text_report(report: &Value) -> String {
    let get = |key: &str| report.get(key);
    let mut lines = vec!["route check".to_string()];
    lines.push(format!(
        "sample: {}",
        if get("sampleProvided").and_then(Value::as_bool) == Some(true) {
            "provided"
        } else {
            "not provided"
        }
    ));
    lines.push("layers:".to_string());
    if let Some(layers) = get("layers").and_then(Value::as_array) {
        for layer in layers {
            lines.push(format!(
                "  {}: {} (source={}, routes={}, invalid={})",
                text_field(layer.get("layer")),
                text_field(layer.get("status")),
                text_field(layer.get("sourceKind")),
                layer.get("routeCount").and_then(Value::as_u64).unwrap_or(0),
                layer
                    .get("invalidCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            ));
            if let Some(error) = layer.get("error").filter(|e| !e.is_null()) {
                lines.push(format!(
                    "    error: {} ({})",
                    text_field(error.get("code")),
                    text_field(error.get("reason"))
                ));
            }
        }
    }
    if let Some(policy) = get("policy") {
        lines.push(format!("policy: {}", text_field(policy.get("status"))));
        if let Some(error) = policy.get("error").filter(|e| !e.is_null()) {
            lines.push(format!(
                "  error: {} ({})",
                text_field(error.get("code")),
                text_field(error.get("reason"))
            ));
        }
    }
    lines.push("routes:".to_string());
    if let Some(routes) = get("routes").and_then(Value::as_array) {
        for route in routes {
            lines.push(format!(
                "  - {} [{}:{}:{}]",
                text_field(route.get("name")),
                text_field(route.get("layer")),
                text_field(route.get("trust")),
                text_field(route.get("configuration")),
            ));
            lines.push(format!(
                "    workflow: {}",
                text_field(
                    route
                        .pointer("/workflow/status")
                        .or_else(|| route.get("workflow"))
                )
            ));
            lines.push(format!(
                "    grants: {}",
                text_field(route.pointer("/grants/status"))
            ));
            lines.push(format!(
                "    execReadiness: {}",
                text_field(route.pointer("/execReadiness/status"))
            ));
            let sample = route.get("sample").cloned().unwrap_or(Value::Null);
            let mut sample_line = format!(
                "    sample: {} matched={} selected={}",
                text_field(sample.get("status")),
                sample
                    .get("matched")
                    .map(|v| match v {
                        Value::Bool(v) => v.to_string(),
                        _ => v.as_str().unwrap_or("").to_string(),
                    })
                    .unwrap_or_default(),
                sample
                    .get("selected")
                    .map(|v| match v {
                        Value::Bool(v) => v.to_string(),
                        _ => v.as_str().unwrap_or("").to_string(),
                    })
                    .unwrap_or_default(),
            );
            if sample.get("viability").is_some() {
                sample_line.push_str(&format!(
                    " viability={}",
                    text_field(sample.get("viability"))
                ));
            }
            lines.push(sample_line);
            if let Some(diagnostics) = route.get("diagnostics").and_then(Value::as_array) {
                for diagnostic in diagnostics {
                    lines.push(format!(
                        "    {} {}: {}",
                        text_field(diagnostic.get("severity")),
                        text_field(diagnostic.get("code")),
                        text_field(diagnostic.get("message")),
                    ));
                }
            }
        }
    }
    if let Some(summary) = get("summary") {
        lines.push(format!(
            "summary: routes={} active={} info={} warning={} error={}",
            summary
                .get("totalRoutes")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("activeRoutes")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .pointer("/diagnostics/info")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .pointer("/diagnostics/warning")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .pointer("/diagnostics/error")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    fn config_env<'a>(config_dir: &'a std::path::Path) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            (name == "CHEAPER_CONFIG_DIR").then(|| config_dir.to_string_lossy().into_owned())
        }
    }

    #[tokio::test]
    async fn build_report_folds_layer_failure_into_allowlisted_projection() {
        let workspace = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        std::fs::write(config.path().join("routes.yaml"), "version: [").unwrap();
        let env = config_env(config.path());
        let sources = supply::check_sources(
            workspace.path().to_str().unwrap(),
            &env,
            &CliHooks::default(),
        )
        .unwrap();

        let (report, exit_code) = build_report(&sources, None).await;
        let report = Value::Object(report);
        assert_eq!(exit_code, 3);
        assert_eq!(report["layers"][1]["layer"], json!("user"));
        assert_eq!(report["layers"][1]["status"], json!("failed"));
        assert_eq!(
            report["layers"][1]["error"]["code"],
            json!("route.layer-load")
        );
        assert_eq!(
            report["layers"][1]["error"]["source"],
            json!("user:routes.yaml")
        );
        assert_eq!(report["summary"]["diagnostics"]["error"], json!(2));
        let serialized = report.to_string();
        assert!(!serialized.contains(config.path().to_string_lossy().as_ref()));
        assert!(!serialized.contains(workspace.path().to_string_lossy().as_ref()));
    }

    #[tokio::test]
    async fn build_report_marks_no_sample_without_claiming_runnable() {
        let workspace = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        let env = config_env(config.path());
        let sources = supply::check_sources(
            workspace.path().to_str().unwrap(),
            &env,
            &CliHooks::default(),
        )
        .unwrap();

        let (report, exit_code) = build_report(&sources, None).await;
        let report = Value::Object(report);
        assert_eq!(exit_code, 1);
        let cargo = report["routes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|route| route["name"] == "cargo-test")
            .unwrap();
        assert_eq!(cargo["sample"]["status"], json!("requires-sample-input"));
        assert_eq!(cargo["sample"]["matched"], json!("not-applicable"));
        assert!(cargo["sample"].get("viability").is_none());
        assert_eq!(cargo["grants"]["anchor"]["status"], json!("missing"));
    }

    #[test]
    fn non_input_escape_is_wrapped_without_details_or_cause() {
        let error = CheaperError::new(ErrorCode::PolicyViolation, "raw policy detail")
            .with_details(Map::from_iter([(
                "secret".to_string(),
                Value::from("value"),
            )]))
            .with_cause(std::io::Error::other("raw cause"));
        let sanitized = sanitize_internal_error(Box::new(error));
        let error = sanitized.downcast_ref::<CheaperError>().unwrap();
        assert_eq!(error.code(), ErrorCode::InternalError);
        assert_eq!(error.message(), "route check failed");
        assert!(error.details().is_none());
        assert!(error.source().is_none());
    }

    #[test]
    fn internal_error_envelope_maps_to_exit_seventy_without_details_or_cause() {
        let error = CheaperError::new(ErrorCode::InternalError, "raw internal detail")
            .with_details(Map::from_iter([(
                "secret".to_string(),
                Value::from("value"),
            )]))
            .with_cause(std::io::Error::other("raw cause"));
        let envelope = crate::cli::route_check_error_envelope(&error, None);
        assert_eq!(envelope["ok"], json!(false));
        assert_eq!(envelope["error"]["code"], json!("INTERNAL_ERROR"));
        assert_eq!(envelope["error"]["message"], json!("route check failed"));
        assert_eq!(envelope["error"]["exitCode"], json!(70));
        assert_eq!(
            envelope["error"]["details"]["reason"],
            json!("internal-error")
        );
        assert!(envelope["error"].get("cause").is_none());
        assert!(!envelope.to_string().contains("raw internal detail"));
        assert!(!envelope.to_string().contains("secret"));
    }

    #[test]
    fn schema_violation_projection_keeps_only_normalized_field() {
        let error = CheaperError::new(ErrorCode::InvalidInput, "schema violation").with_details(
            Map::from_iter([(
                "violation".to_string(),
                Value::from("routes[0].match.command: raw value SECRET"),
            )]),
        );
        assert_eq!(
            normalized_field(&error),
            Some("routes[0].match.command".to_string())
        );

        let injected = CheaperError::new(ErrorCode::InvalidInput, "schema violation").with_details(
            Map::from_iter([("field".to_string(), Value::from("routes[0].name\nSECRET"))]),
        );
        assert_eq!(normalized_field(&injected), None);
    }

    #[test]
    fn dedicated_text_renderer_uses_report_projection() {
        let report = json!({
            "schemaVersion": 1,
            "sampleProvided": false,
            "layers": [{
                "layer": "builtin",
                "sourceKind": "embedded",
                "status": "loaded",
                "routeCount": 1,
                "invalidCount": 0,
                "error": null,
            }],
            "policy": {"status": "loaded", "error": null},
            "routes": [{
                "name": "cargo-test",
                "layer": "builtin",
                "trust": "trusted",
                "configuration": "active",
                "workflow": {"status": "valid"},
                "grants": {"status": "missing"},
                "execReadiness": {"status": "missing-target"},
                "sample": {"status": "requires-sample-input", "matched": "not-applicable", "selected": "not-applicable"},
                "diagnostics": [{"severity": "warning", "code": "route.grant-missing", "message": "grant cargo-test is not declared"}],
            }],
            "summary": {"totalRoutes": 1, "activeRoutes": 1, "diagnostics": {"info": 1, "warning": 1, "error": 0}},
        });
        let text = text_report(&report);
        assert!(text.starts_with("route check\n"));
        assert!(text.contains("builtin: loaded (source=embedded, routes=1, invalid=0)"));
        assert!(text.contains("warning route.grant-missing"));
        assert_eq!(text_field(Some(&json!("forged\nname"))), "forged\\nname");
        assert_eq!(metadata_label("worker\nname"), "\"worker\\nname\"");
        assert_eq!(text, text_report(&report));
    }
}

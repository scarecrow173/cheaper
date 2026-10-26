use std::time::{SystemTime, UNIX_EPOCH};

use cheaper_core::errors::ErrorCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::identity::{ContextProvenance, ExecutionIdentity};

pub const SCHEMA_VERSION: u8 = 1;
pub const MAX_EVENT_BYTES: usize = 8 * 1024;
const MAX_DEPENDENCY_IDS: usize = 128;

const MAX_DURATION_MS: u64 = 315_360_000_000;
const MAX_TURNS: u64 = 1_000_000;
const MAX_TOOL_CALLS: u64 = 1_000_000;
const MAX_FILE_BYTES: u64 = 1_099_511_627_776;
const MAX_TOKEN_METRIC: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionKind {
    Worker,
    Workflow,
    Job,
    Route,
    #[serde(rename = "hook-decision")]
    HookDecision,
    #[serde(rename = "route-decision")]
    RouteDecision,
    #[serde(rename = "route-refetch")]
    RouteRefetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
    Skipped,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionEvent {
    Started,
    Decision,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasonCode {
    #[serde(rename = "read.below_threshold")]
    ReadBelowThreshold,
    #[serde(rename = "read.byte_fast_path")]
    ReadByteFastPath,
    #[serde(rename = "read.threshold_exceeded_delegate_advised")]
    ReadThresholdExceededDelegateAdvised,
    #[serde(rename = "read.measurement_limited")]
    ReadMeasurementLimited,
    #[serde(rename = "read.measurement_failed_fail_open")]
    ReadMeasurementFailedFailOpen,
    #[serde(rename = "read.no_candidate")]
    ReadNoCandidate,
    #[serde(rename = "read.out_of_scope")]
    ReadOutOfScope,
    #[serde(rename = "search.no_candidate")]
    SearchNoCandidate,
    #[serde(rename = "shell.no_candidate")]
    ShellNoCandidate,
    #[serde(rename = "route.no_match")]
    RouteNoMatch,
    #[serde(rename = "route.no_grant")]
    RouteNoGrant,
    #[serde(rename = "route.no_pin")]
    RouteNoPin,
    #[serde(rename = "route.not_runnable")]
    RouteNotRunnable,
    #[serde(rename = "route.rewrite_emitted")]
    RouteRewriteEmitted,
    #[serde(rename = "route.rewrite_unsupported_fallback_deny")]
    RouteRewriteUnsupportedFallbackDeny,
    #[serde(rename = "workflow.job_dependency_failed")]
    WorkflowJobDependencyFailed,
    #[serde(rename = "run.worker_resolution_failed")]
    RunWorkerResolutionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionVerdict {
    Allow,
    DenyDelegationAdvised,
    RewriteEmitted,
    NotRouted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteViability {
    Runnable,
    NoGrant,
    MissingTarget,
    InvalidRoute,
    UnsafeCommand,
    StdinUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidatedIdentifier(String);

impl ValidatedIdentifier {
    pub fn new(value: &str) -> Option<Self> {
        let valid = (1..=128).contains(&value.len())
            && value.is_ascii()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte));
        valid.then(|| Self(value.to_owned()))
    }

    fn is_valid(&self) -> bool {
        Self::new(&self.0).is_some()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CatalogIdentifier(String);

impl CatalogIdentifier {
    pub fn new(value: &str) -> Option<Self> {
        let id = ValidatedIdentifier::new(value)?;
        if ["://", "?", "#", "@"]
            .iter()
            .any(|marker| value.contains(marker))
        {
            return None;
        }
        Some(Self(id.0))
    }

    fn is_valid(&self) -> bool {
        Self::new(&self.0).is_some()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteLogRef(String);

impl RouteLogRef {
    pub fn new(value: &str) -> Option<Self> {
        crate::route::log_store::is_valid_run_id(value).then(|| Self(value.to_owned()))
    }

    fn is_valid(&self) -> bool {
        crate::route::log_store::is_valid_run_id(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageRecordRef {
    pub execution_id: String,
}

impl UsageRecordRef {
    pub fn new(execution_id: &str) -> Option<Self> {
        canonical_v4(execution_id).then(|| Self {
            execution_id: execution_id.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordedAt(String);

impl RecordedAt {
    pub fn now() -> Self {
        Self::from_system_time(SystemTime::now())
    }

    pub fn from_system_time(value: SystemTime) -> Self {
        let millis = value
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        Self(iso8601_millis(millis))
    }

    pub fn parse_utc(value: &str) -> Option<Self> {
        is_utc_timestamp(value).then(|| Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn sort_key(&self) -> &str {
        &self.0
    }

    fn is_valid(&self) -> bool {
        is_utc_timestamp(&self.0)
    }
}

/// One immutable, allowlisted lifecycle event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryEvent {
    schema_version: u8,
    recorded_at: RecordedAt,
    execution_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_execution_hint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_provenance: Option<ContextProvenance>,
    kind: ExecutionKind,
    event: ExecutionEvent,
    status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<ReasonCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verdict: Option<DecisionVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    viability: Option<RouteViability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_id: Option<ValidatedIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_id: Option<ValidatedIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<ValidatedIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route_id: Option<ValidatedIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_id: Option<CatalogIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<CatalogIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    turns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    threshold: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measured_tokens: Option<Option<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_ref: Option<RouteLogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_record_ref: Option<UsageRecordRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dependency_ids: Vec<ValidatedIdentifier>,
}

impl HistoryEvent {
    pub fn new(
        identity: &ExecutionIdentity,
        kind: ExecutionKind,
        event: ExecutionEvent,
        status: ExecutionStatus,
    ) -> Self {
        Self::new_at(identity, kind, event, status, RecordedAt::now())
    }

    pub fn new_at(
        identity: &ExecutionIdentity,
        kind: ExecutionKind,
        event: ExecutionEvent,
        status: ExecutionStatus,
        recorded_at: RecordedAt,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            recorded_at,
            execution_id: identity.execution_id().to_string(),
            trace_id: Some(identity.trace_id().to_string()),
            parent_execution_id: identity.parent_execution_id().map(|id| id.to_string()),
            parent_execution_hint_id: identity.parent_execution_hint_id().map(|id| id.to_string()),
            context_provenance: identity.context_provenance(),
            kind,
            event,
            status,
            reason_code: None,
            verdict: None,
            viability: None,
            error_code: None,
            worker_id: None,
            workflow_id: None,
            job_id: None,
            route_id: None,
            provider_id: None,
            model_id: None,
            duration_ms: None,
            turns: None,
            tool_calls: None,
            file_bytes: None,
            threshold: None,
            measured_tokens: None,
            exit_code: None,
            log_ref: None,
            usage_record_ref: None,
            dependency_ids: Vec::new(),
        }
    }

    pub fn with_reason_code(mut self, value: Option<ReasonCode>) -> Self {
        self.reason_code = value;
        self
    }

    pub fn with_verdict(mut self, value: Option<DecisionVerdict>) -> Self {
        self.verdict = value;
        self
    }

    pub fn with_viability(mut self, value: Option<RouteViability>) -> Self {
        self.viability = value;
        self
    }

    pub fn with_error_code(mut self, value: Option<ErrorCode>) -> Self {
        self.error_code = value;
        self
    }

    pub fn with_worker_id(mut self, value: &str) -> Self {
        self.worker_id = ValidatedIdentifier::new(value);
        self
    }

    pub fn with_workflow_id(mut self, value: &str) -> Self {
        self.workflow_id = ValidatedIdentifier::new(value);
        self
    }

    pub fn with_job_id(mut self, value: &str) -> Self {
        self.job_id = ValidatedIdentifier::new(value);
        self
    }

    pub fn with_route_id(mut self, value: &str) -> Self {
        self.route_id = ValidatedIdentifier::new(value);
        self
    }

    /// Accepts only the typed catalog ID projection, never a base URL or raw model input.
    pub fn with_provider_id(mut self, value: &str) -> Self {
        self.provider_id = CatalogIdentifier::new(value);
        self
    }

    /// Accepts only the typed catalog ID projection, never a display label or raw model input.
    pub fn with_model_id(mut self, value: &str) -> Self {
        self.model_id = CatalogIdentifier::new(value);
        self
    }

    pub fn with_duration_ms(mut self, value: Option<f64>) -> Self {
        self.duration_ms = checked_metric(value, MAX_DURATION_MS);
        self
    }

    pub fn with_turns(mut self, value: Option<f64>) -> Self {
        self.turns = checked_metric(value, MAX_TURNS);
        self
    }

    pub fn with_tool_calls(mut self, value: Option<f64>) -> Self {
        self.tool_calls = checked_metric(value, MAX_TOOL_CALLS);
        self
    }

    pub fn with_file_bytes(mut self, value: Option<f64>) -> Self {
        self.file_bytes = checked_metric(value, MAX_FILE_BYTES);
        self
    }

    pub fn with_threshold(mut self, value: Option<f64>) -> Self {
        self.threshold = checked_metric(value, MAX_TOKEN_METRIC);
        self
    }

    /// `None` omits an unprovided measurement; `Some(None)` explicitly stores JSON null.
    pub fn with_measured_tokens(mut self, value: Option<Option<f64>>) -> Self {
        self.measured_tokens = match value {
            None => None,
            Some(None) => Some(None),
            Some(Some(value)) => checked_metric(Some(value), MAX_TOKEN_METRIC).map(Some),
        };
        self
    }

    pub fn with_exit_code(mut self, value: Option<f64>) -> Self {
        self.exit_code = value.and_then(checked_exit_code);
        self
    }

    pub fn with_log_ref(mut self, value: &str) -> Self {
        self.log_ref = RouteLogRef::new(value);
        self
    }

    pub fn with_usage_record_ref(mut self, execution_id: &str) -> Self {
        self.usage_record_ref = UsageRecordRef::new(execution_id);
        self
    }

    pub fn with_dependency_ids(mut self, values: &[String]) -> Self {
        self.dependency_ids = values
            .iter()
            .take(MAX_DEPENDENCY_IDS)
            .filter_map(|value| ValidatedIdentifier::new(value))
            .collect();
        self
    }

    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    pub fn kind(&self) -> ExecutionKind {
        self.kind
    }

    pub fn event(&self) -> ExecutionEvent {
        self.event
    }

    pub fn status(&self) -> ExecutionStatus {
        self.status
    }

    pub fn recorded_at(&self) -> &RecordedAt {
        &self.recorded_at
    }

    pub fn worker_id(&self) -> Option<&str> {
        self.worker_id.as_ref().map(ValidatedIdentifier::as_str)
    }

    pub(crate) fn same_lifecycle_identity(&self, other: &Self) -> bool {
        self.execution_id == other.execution_id
            && self.trace_id == other.trace_id
            && self.parent_execution_id == other.parent_execution_id
            && self.parent_execution_hint_id == other.parent_execution_hint_id
            && self.context_provenance == other.context_provenance
            && self.kind == other.kind
    }

    pub fn is_valid(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && self.recorded_at.is_valid()
            && canonical_v4(&self.execution_id)
            && self.trace_id.as_deref().is_none_or(canonical_v4)
            && self.parent_execution_id.as_deref().is_none_or(canonical_v4)
            && self
                .parent_execution_hint_id
                .as_deref()
                .is_none_or(canonical_v4)
            && !(self.parent_execution_id.is_some() && self.parent_execution_hint_id.is_some())
            && (self.parent_execution_hint_id.is_none()
                || self.context_provenance == Some(ContextProvenance::InheritedUnverified))
            && match self.event {
                ExecutionEvent::Started => self.status == ExecutionStatus::Running,
                ExecutionEvent::Decision | ExecutionEvent::Terminal => {
                    self.status != ExecutionStatus::Running
                }
            }
            && self
                .worker_id
                .as_ref()
                .is_none_or(ValidatedIdentifier::is_valid)
            && self
                .workflow_id
                .as_ref()
                .is_none_or(ValidatedIdentifier::is_valid)
            && self
                .job_id
                .as_ref()
                .is_none_or(ValidatedIdentifier::is_valid)
            && self
                .route_id
                .as_ref()
                .is_none_or(ValidatedIdentifier::is_valid)
            && self
                .provider_id
                .as_ref()
                .is_none_or(CatalogIdentifier::is_valid)
            && self
                .model_id
                .as_ref()
                .is_none_or(CatalogIdentifier::is_valid)
            && self.log_ref.as_ref().is_none_or(RouteLogRef::is_valid)
            && self.usage_record_ref.as_ref().is_none_or(|usage| {
                canonical_v4(&usage.execution_id) && usage.execution_id == self.execution_id
            })
            && self.dependency_ids.len() <= MAX_DEPENDENCY_IDS
            && self
                .dependency_ids
                .iter()
                .all(ValidatedIdentifier::is_valid)
            && self.duration_ms.is_none_or(|n| n <= MAX_DURATION_MS)
            && self.turns.is_none_or(|n| n <= MAX_TURNS)
            && self.tool_calls.is_none_or(|n| n <= MAX_TOOL_CALLS)
            && self.file_bytes.is_none_or(|n| n <= MAX_FILE_BYTES)
            && self.threshold.is_none_or(|n| n <= MAX_TOKEN_METRIC)
            && self
                .measured_tokens
                .is_none_or(|value| value.is_none_or(|n| n <= MAX_TOKEN_METRIC))
    }

    pub fn to_bounded_json(&self) -> Option<Vec<u8>> {
        if !self.is_valid() {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= MAX_EVENT_BYTES).then_some(bytes)
    }

    pub fn from_bounded_json(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_EVENT_BYTES {
            return None;
        }
        let event: Self = serde_json::from_slice(bytes).ok()?;
        event.is_valid().then_some(event)
    }
}

fn checked_metric(value: Option<f64>, cap: u64) -> Option<u64> {
    let value = value?;
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= cap as f64)
        .then_some(value as u64)
}

fn checked_exit_code(value: f64) -> Option<i64> {
    (value.is_finite()
        && value.fract() == 0.0
        && value >= i64::MIN as f64
        && value < 9_223_372_036_854_775_808.0)
        .then_some(value as i64)
}

fn canonical_v4(value: &str) -> bool {
    Uuid::parse_str(value)
        .is_ok_and(|id| id.get_version_num() == 4 && id.hyphenated().to_string() == value)
}

fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 24
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[23] != b'Z'
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 4 | 7 | 10 | 13 | 16 | 19 | 23) && !byte.is_ascii_digit()
        })
    {
        return false;
    }
    let number = |from, to| value.get(from..to)?.parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day > 0 && day <= days_in_month && hour <= 23 && minute <= 59 && second <= 59
}

fn iso8601_millis(ms_epoch: i64) -> String {
    let secs = ms_epoch.div_euclid(1000);
    let millis = ms_epoch.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        secs_of_day / 3600,
        secs_of_day % 3600 / 60,
        secs_of_day % 60,
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::history::identity::ExecutionIdentity;

    use super::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent, ReasonCode};

    #[test]
    fn history_schema_validates_projected_ids_and_preserves_official_catalog_ids() {
        let identity = ExecutionIdentity::root();
        let event = HistoryEvent::new(
            &identity,
            ExecutionKind::Worker,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )
        .with_worker_id("reader.worker")
        .with_provider_id("provider://secret?key=x")
        .with_model_id("amazon.nova-2-lite-v1:0");
        let json: Value = serde_json::from_slice(&event.to_bounded_json().unwrap()).unwrap();

        assert_eq!(json["workerId"], "reader.worker");
        assert!(json.get("providerId").is_none());
        assert_eq!(json["modelId"], "amazon.nova-2-lite-v1:0");
        assert!(json.get("baseUrl").is_none());
    }

    #[test]
    fn history_schema_keeps_valid_metrics_and_omits_invalid_values() {
        let identity = ExecutionIdentity::root();
        let event = HistoryEvent::new(
            &identity,
            ExecutionKind::Worker,
            ExecutionEvent::Terminal,
            ExecutionStatus::Completed,
        )
        .with_duration_ms(Some(0.0))
        .with_turns(Some(1_000_000.0))
        .with_tool_calls(Some(-1.0))
        .with_file_bytes(Some(1.5))
        .with_threshold(Some(f64::INFINITY))
        .with_measured_tokens(Some(None))
        .with_exit_code(Some(-7.0));
        let json: Value = serde_json::from_slice(&event.to_bounded_json().unwrap()).unwrap();

        assert_eq!(json["durationMs"], 0);
        assert_eq!(json["turns"], 1_000_000);
        assert!(json.get("toolCalls").is_none());
        assert!(json.get("fileBytes").is_none());
        assert!(json.get("threshold").is_none());
        assert_eq!(json["measuredTokens"], Value::Null);
        assert_eq!(json["exitCode"], -7);
    }

    #[test]
    fn history_schema_omits_over_cap_metrics_without_dropping_event() {
        let event = HistoryEvent::new(
            &ExecutionIdentity::root(),
            ExecutionKind::Worker,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )
        .with_duration_ms(Some(315_360_000_001.0))
        .with_turns(Some(1_000_001.0))
        .with_file_bytes(Some(1_099_511_627_777.0))
        .with_threshold(Some(1_000_000_001.0));
        let json: Value = serde_json::from_slice(&event.to_bounded_json().unwrap()).unwrap();

        assert_eq!(json["event"], "started");
        for field in ["durationMs", "turns", "fileBytes", "threshold"] {
            assert!(json.get(field).is_none(), "{field} must be omitted");
        }
    }

    #[test]
    fn history_schema_rejects_events_larger_than_eight_kib() {
        let dependencies = (0..128).map(|_| "a".repeat(128)).collect::<Vec<_>>();
        let event = HistoryEvent::new(
            &ExecutionIdentity::root(),
            ExecutionKind::Job,
            ExecutionEvent::Decision,
            ExecutionStatus::Skipped,
        )
        .with_reason_code(Some(ReasonCode::WorkflowJobDependencyFailed))
        .with_dependency_ids(&dependencies);

        assert!(event.to_bounded_json().is_none());
    }

    #[test]
    fn history_schema_drops_unallowlisted_raw_fields_when_reading() {
        let raw = br#"{"schemaVersion":1,"recordedAt":"2026-10-27T00:00:00.000Z","executionId":"6ba7b810-9dad-41d1-80b4-00c04fd430c8","traceId":"6ba7b810-9dad-41d1-80b4-00c04fd430c8","kind":"worker","event":"started","status":"running","argv":"secret"}"#;

        assert!(HistoryEvent::from_bounded_json(raw).is_none());
    }
}

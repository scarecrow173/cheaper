//! `route::routes` — routes.yaml schema 検証・3 層 load・effective
//! route set 構築(spec R1-R7、design §routes.yaml schema と 3 層解決)。
//!
//! - 層: builtin(`<builtin_dir>/routes.yaml`)→
//!   user(`<userConfigDir>/routes.yaml`)→
//!   project(`<project_dir>/routes.yaml`)。`match.command` token 列で
//!   key 化し builtin → user → project の順に merge する。
//! - load は fail-loud — malformed YAML・未知 key・required 欠落・
//!   `match.command` 空・`kind` 非 `"workflow"`・`target.path` の
//!   絶対パス/traversal は config error(`invalid-route`)。壊れた
//!   route を黙って drop しない。
//! - merge 規則: user は builtin を無条件 shadow。project は
//!   下位層を shadow しない(trust = 宣言層 provenance)—
//!   `match.command` が衝突した project route は `suppressed`
//!   provenance として記録される(静かに無視しない)。
//! - 同一層内の同一 `match.command` 重複は `invalid` bucket へ
//!   隔離し resolve 対象から除外する(silent last/first-wins 禁止)。
//! - `target.path` は層起点相対パスのみ許容 — builtin は
//!   `<builtin_dir>/workflows/`、user/project は routes.yaml と同
//!   dir。`..`・absolute・NUL を含む宣言は config error。

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use cheaper_core::errors::{io_errno_display, is_absent_file_error, CheaperError, ErrorCode};
use cheaper_core::nodepath;
use serde_json::{json, Map, Value};

/// route を宣言した層(provenance — trust 評価と診断報告に使う)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLayer {
    Builtin,
    User,
    Project,
}

impl RouteLayer {
    /// 宣言主体から導かれる trust — `trusted` 宣言値ではなく
    /// layer provenance が source of truth。
    #[allow(dead_code)]
    pub(crate) fn trust(&self) -> RouteTrust {
        match self {
            Self::Builtin | Self::User => RouteTrust::Trusted,
            Self::Project => RouteTrust::Untrusted,
        }
    }

    /// diagnostic source の `<layer>:` 部分。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

/// layer provenance から導く route trust。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RouteTrust {
    Trusted,
    Untrusted,
}

/// `substitute` — substitute-exec mode の宣言ペア(R1/O1a)。
/// `grant` は既存 `command.grants` 参照名(route は grant を
/// 生成しない — pin が executable を供給)、`argv` は executable
/// を含まない実行 args 列(command tool input `args` と同形、
/// `[]` = executable 単独起動の明示宣言)。presence が mode
/// switch になる — `Some` で substitute mode。
/// trusted 層(builtin/user)のみ宣言可(project 層は load 時拒否)。
#[derive(Debug, Clone, PartialEq)]
pub struct RouteSubstitute {
    /// 実行 grant 名 — viability 完全検査・expected_input name・
    /// lifecycle bound・投影 member の execution anchor。
    /// `route.grant` と同名可(同一 grant の argv 差替)。
    pub grant: String,
    /// 実行 argv — grant の argv pattern が bound する宣言固定値。
    /// typed args の転送・補間はしない(宣言凍結 — R12)。
    pub argv: Vec<String>,
}

/// schema 検証済み・`target.path` 層起点解決済みの route 宣言。
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub name: String,
    /// `match.command` token 列(空不可)。
    pub command: Vec<String>,
    /// `target.path` — 宣言層起点で join 済みの絶対パス。
    pub target_path: PathBuf,
    /// `command.grants` 参照名(route は grant を生成しない)。
    /// 発火 gate・record attribution の単一 anchor — `grants`
    /// member は含めない。実行 anchor(expected_input・viability
    /// 完全検査対象)は byte-matched では本 grant、substitute
    /// mode では `substitute.grant` へ移る(`exec_pair` が選ぶ)。
    pub grant: String,
    /// `grants` — route.grant 以外の追加委譲 grant 名(宣言順)。
    /// parse 時に member 間重複と `route.grant` 自身の参照を
    /// 冪等に除去済み — `[]` ≡ 未指定。有効 set は
    /// `grant_set()` = {grant} ∪ grants ∪ {substitute.grant}。
    pub grants: Vec<String>,
    /// `substitute` — substitute-exec mode 宣言(Option D)。
    /// `Some` のとき実行対象は `{substitute.grant, substitute.argv}`
    /// で、typed command は `match.command` prefix の発火契機に留まる。
    /// firing gate・record attribution anchor は `route.grant` のまま。
    pub substitute: Option<RouteSubstitute>,
    /// `output.tokenThreshold`(routed workflow の conditional
    /// worker 閾値の per-route override)。
    pub token_threshold: Option<f64>,
    /// `passthroughEnv` — hook env から capture を許す env 名。
    pub passthrough_env: Vec<String>,
    /// 宣言層 provenance — trust 判定(`layer ∈ {Builtin, User}`
    /// が trusted)#168・target.path 層起点解決・診断の単一 source。
    /// `Route` は trust flag を持たない — trust は宣言値ではなく
    /// 宣言主体(層)が決める(O1a)。
    pub layer: RouteLayer,
}

impl Route {
    /// 有効投影 set — `{route.grant} ∪ grants ∪ {substitute.grant}`。
    /// substitute mode でも `route.grant` は投影 member として残り
    /// (firing gate・attribution anchor・member job からの呼出し可 —
    /// O7a)。「投影名は全て viability 検証済み」の不変を維持する。
    pub fn grant_set(&self) -> std::collections::BTreeSet<String> {
        let mut set = std::collections::BTreeSet::new();
        set.insert(self.grant.clone());
        set.extend(self.grants.iter().cloned());
        if let Some(substitute) = &self.substitute {
            set.insert(substitute.grant.clone());
        }
        set
    }

    /// 実行ペア `{grant名, argv}` — substitute mode は宣言
    /// `{substitute.grant, substitute.argv}`、byte-matched は
    /// `{route.grant, typed_args}`(caller は `resolution.exec_args`
    /// を渡す)。viability 完全検査の対象・`expected_input`・inputs
    /// `grant`/`execArgs`・deadline 導出が全てこの anchor に従う
    /// (firing/attribution anchor の `route.grant` とは別系統 —
    /// O2a/O4a/R6)。
    pub fn exec_pair<'a>(&'a self, typed_args: &'a [String]) -> (&'a str, &'a [String]) {
        match &self.substitute {
            Some(substitute) => (substitute.grant.as_str(), substitute.argv.as_slice()),
            None => (self.grant.as_str(), typed_args),
        }
    }
}

/// merge で負けた route の provenance — 同一 `match.command`
/// で下位層(builtin/user)route に負けた project route
/// (`route resolve`・`kind:"route"` record が報告)。project
/// 層は下位層を shadow しないため displaced 下位層 route は
/// 発生しない(#168 — #167 の displaced 記録 arm は到達不能で
/// 除去)。
#[derive(Debug, Clone, PartialEq)]
pub struct SuppressedRoute {
    pub name: String,
    pub command: Vec<String>,
}

/// 同一層内で `match.command` が重複した route 群 — effective set
/// から除外し、`invalid-route` viability 報告の材料とする。
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidRouteGroup {
    pub command: Vec<String>,
    pub names: Vec<String>,
    pub layer: RouteLayer,
}

/// 3 層 merge 済みの effective route set。
#[derive(Debug, Clone, Default)]
pub struct EffectiveRoutes {
    pub routes: Vec<Route>,
    pub suppressed: Vec<SuppressedRoute>,
    pub invalid: Vec<InvalidRouteGroup>,
}

/// route 層の independent load 状態。`absent` は正常空層で、
/// `failed` はその層だけを失敗扱いにし sibling の valid row を残す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteLayerStatus {
    Absent,
    Loaded,
    Failed,
}

impl RouteLayerStatus {
    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// 1 層分の read/validate 結果。`error` は layer failure のみ保持し、
/// merge は valid route と invalid bucket だけを見る。
#[derive(Debug)]
pub(crate) struct RouteLayerEvaluation {
    pub(crate) layer: RouteLayer,
    pub(crate) status: RouteLayerStatus,
    #[allow(dead_code)]
    pub(crate) source: String,
    pub(crate) routes: Vec<Route>,
    pub(crate) invalid: Vec<InvalidRouteGroup>,
    pub(crate) error: Option<CheaperError>,
}

impl RouteLayerEvaluation {
    pub(crate) fn absent(layer: RouteLayer, file: &Path) -> Self {
        layer_evaluation(
            layer,
            diagnostic_source(layer, file),
            RouteLayerStatus::Absent,
            Vec::new(),
            Vec::new(),
            None,
        )
    }
}

/// route 1 行分の merge 判定。`invalid` は layer validation で
/// 隔離された宣言、`indeterminate` は sibling layer failure で
/// 確定できない状態を表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RouteMergeStatus {
    Active,
    Suppressed,
    Shadowed,
    Invalid,
    Indeterminate,
}

/// diagnostics 向け merge row。invalid は duplicate group の浅い
/// provenance(name/command/layer/trust/invalidReason)だけを持つ。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum RouteMergeRow {
    Route {
        route: Route,
        status: RouteMergeStatus,
    },
    Invalid {
        name: String,
        command: Vec<String>,
        layer: RouteLayer,
        invalid_reason: &'static str,
    },
}

#[allow(dead_code)]
impl RouteMergeRow {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Route { route, .. } => route.name.as_str(),
            Self::Invalid { name, .. } => name.as_str(),
        }
    }

    pub(crate) fn layer(&self) -> RouteLayer {
        match self {
            Self::Route { route, .. } => route.layer,
            Self::Invalid { layer, .. } => *layer,
        }
    }

    pub(crate) fn status(&self) -> RouteMergeStatus {
        match self {
            Self::Route { status, .. } => *status,
            Self::Invalid { .. } => RouteMergeStatus::Invalid,
        }
    }

    pub(crate) fn route(&self) -> Option<&Route> {
        match self {
            Self::Route { route, .. } => Some(route),
            Self::Invalid { .. } => None,
        }
    }

    pub(crate) fn trust(&self) -> RouteTrust {
        self.layer().trust()
    }
}

/// shared merge の provenance 付き結果。runtime `load` が使う
/// `EffectiveRoutes` と、diagnostics が使う row 状態を分離する。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteMergeOutcome {
    pub(crate) effective: EffectiveRoutes,
    pub(crate) shadowed: Vec<Route>,
    pub(crate) rows: Vec<RouteMergeRow>,
}

/// `load` の層 path 入力 — caller が dir 解決責務を持つ(純粋評価)。
pub struct RouteLoadOptions<'a> {
    pub builtin_dir: &'a Path,
    pub user_config_dir: &'a Path,
    pub project_dir: &'a Path,
}

/// `route::routes::load` — 3 層を読み merge して effective set を
/// 返す。file read のみ(bounded)。file absent は空層として扱う。
pub fn load(opts: &RouteLoadOptions<'_>) -> Result<EffectiveRoutes, CheaperError> {
    // 層起点: builtin は workflows/、user/project は routes.yaml と
    // 同 dir(宣言層を起点にすることで project route が workspace 外を
    // 指せない)。
    let mut evaluations = Vec::with_capacity(3);
    for (layer, file, root) in [
        (
            RouteLayer::Builtin,
            opts.builtin_dir.join("routes.yaml"),
            opts.builtin_dir.join("workflows"),
        ),
        (
            RouteLayer::User,
            opts.user_config_dir.join("routes.yaml"),
            opts.user_config_dir.to_path_buf(),
        ),
        (
            RouteLayer::Project,
            opts.project_dir.join("routes.yaml"),
            opts.project_dir.to_path_buf(),
        ),
    ] {
        let mut evaluation = evaluate_filesystem_layer(layer, &file, &root);
        if let Some(error) = evaluation.error.take() {
            return Err(error);
        }
        evaluations.push(evaluation);
    }
    Ok(merge_route_layers(&evaluations).effective)
}

/// filesystem routes.yaml を 1 層分評価する。diagnostics では
/// sibling 層を止めず、runtime `load` は最初の failure で従来通り
/// fail-loud する。
pub(crate) fn evaluate_filesystem_layer(
    layer: RouteLayer,
    file: &Path,
    layer_root: &Path,
) -> RouteLayerEvaluation {
    let source = diagnostic_source(layer, file);
    match read_routes_file(file) {
        Ok(None) => layer_evaluation(
            layer,
            source,
            RouteLayerStatus::Absent,
            Vec::new(),
            Vec::new(),
            None,
        ),
        Ok(Some(doc)) => evaluate_routes_doc(doc, layer, layer_root, file, source),
        Err(error) => layer_evaluation(
            layer,
            source,
            RouteLayerStatus::Failed,
            Vec::new(),
            Vec::new(),
            Some(error),
        ),
    }
}

/// embedded/logical source から得た routes.yaml text を同じ schema・
/// duplicate 隔離で評価する。`file` は error details と declaration
/// source 表示に使う logical path。
pub(crate) fn evaluate_routes_text(
    layer: RouteLayer,
    file: &str,
    text: &str,
    layer_root: &Path,
) -> RouteLayerEvaluation {
    let file_path = Path::new(file);
    let source = diagnostic_source(layer, file_path);
    match parse_routes_text(text, file) {
        Ok(doc) => evaluate_routes_doc(doc, layer, layer_root, file_path, source),
        Err(error) => layer_evaluation(
            layer,
            source,
            RouteLayerStatus::Failed,
            Vec::new(),
            Vec::new(),
            Some(error),
        ),
    }
}

fn evaluate_routes_doc(
    doc: Map<String, Value>,
    layer: RouteLayer,
    layer_root: &Path,
    file: &Path,
    source: String,
) -> RouteLayerEvaluation {
    match validate_routes_doc(&doc, layer, layer_root, file) {
        Ok((routes, invalid)) => layer_evaluation(
            layer,
            source,
            RouteLayerStatus::Loaded,
            routes,
            invalid,
            None,
        ),
        Err(error) => layer_evaluation(
            layer,
            source,
            RouteLayerStatus::Failed,
            Vec::new(),
            Vec::new(),
            Some(error),
        ),
    }
}

/// report へそのまま出せる layer source 形(`<layer>:<relative>`)。
/// routes.yaml は層ごとの固定名のため basename だけを採る。
fn diagnostic_source(layer: RouteLayer, file: &Path) -> String {
    let relative = file
        .file_name()
        .map(|name| name.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "routes.yaml".to_string());
    format!("{}:{}", layer.as_str(), relative)
}

fn layer_evaluation(
    layer: RouteLayer,
    source: String,
    status: RouteLayerStatus,
    routes: Vec<Route>,
    invalid: Vec<InvalidRouteGroup>,
    error: Option<CheaperError>,
) -> RouteLayerEvaluation {
    RouteLayerEvaluation {
        layer,
        status,
        source,
        routes,
        invalid,
        error,
    }
}

/// shared merge。runtime `load` と diagnostics が同じ precedence・
/// quarantine・suppression 規則を通る。layer failure は route row を
/// 作らず、merge-dependent row だけを `indeterminate` にする。
pub(crate) fn merge_route_layers(layers: &[RouteLayerEvaluation]) -> RouteMergeOutcome {
    let status = |layer| {
        layers
            .iter()
            .find(|evaluation| evaluation.layer == layer)
            .map(|evaluation| evaluation.status)
            .unwrap_or(RouteLayerStatus::Absent)
    };
    let routes = |layer| {
        layers
            .iter()
            .find(|evaluation| evaluation.layer == layer)
            .map(|evaluation| evaluation.routes.as_slice())
            .unwrap_or(&[])
    };
    let invalid = |layer| {
        layers
            .iter()
            .find(|evaluation| evaluation.layer == layer)
            .map(|evaluation| evaluation.invalid.as_slice())
            .unwrap_or(&[])
    };

    let builtin_failed = status(RouteLayer::Builtin).is_failed();
    let user_failed = status(RouteLayer::User).is_failed();
    let mut effective: HashMap<Vec<String>, Route> = HashMap::new();
    let mut order: Vec<Vec<String>> = Vec::new();
    let mut suppressed = Vec::new();
    let mut all_invalid = Vec::new();
    let mut shadowed = Vec::new();
    let mut rows = Vec::new();

    for layer in [RouteLayer::Builtin, RouteLayer::User, RouteLayer::Project] {
        for route in routes(layer) {
            let merge_status = match layer {
                RouteLayer::Builtin if user_failed => RouteMergeStatus::Indeterminate,
                RouteLayer::Builtin => {
                    if routes(RouteLayer::User)
                        .iter()
                        .any(|user_route| user_route.command == route.command)
                    {
                        RouteMergeStatus::Shadowed
                    } else {
                        RouteMergeStatus::Active
                    }
                }
                RouteLayer::User => RouteMergeStatus::Active,
                RouteLayer::Project => {
                    if effective.contains_key(&route.command) {
                        RouteMergeStatus::Suppressed
                    } else if builtin_failed || user_failed {
                        RouteMergeStatus::Indeterminate
                    } else {
                        RouteMergeStatus::Active
                    }
                }
            };
            match layer {
                // builtin/user は key 衝突しない(層内 dup は invalid
                // 隔離済み)。user は builtin を無条件 shadow する。
                RouteLayer::Builtin | RouteLayer::User => {
                    if !effective.contains_key(&route.command) {
                        order.push(route.command.clone());
                    }
                    if let Some(previous) = effective.insert(route.command.clone(), route.clone()) {
                        shadowed.push(previous);
                    }
                }
                // project 層 route は下位層を shadow しない — trust は宣言値
                // ではなく宣言層 provenance が決める(O1a)。`match.command`
                // 衝突は常に project 側が負けて `suppressed` provenance へ
                // 記録される(静かに無視しない — R6)。project 層の `trusted`
                // key は load 時に層 gate で拒否済みのため、merge が flag を
                // 読む経路は存在しない。
                RouteLayer::Project => {
                    if effective.contains_key(&route.command) {
                        suppressed.push(SuppressedRoute {
                            name: route.name.clone(),
                            command: route.command.clone(),
                        });
                    } else {
                        order.push(route.command.clone());
                        effective.insert(route.command.clone(), route.clone());
                    }
                }
            }
            rows.push(RouteMergeRow::Route {
                route: route.clone(),
                status: merge_status,
            });
        }
        all_invalid.extend(invalid(layer).iter().cloned());
        rows.extend(invalid(layer).iter().flat_map(|group| {
            group.names.iter().map(|name| RouteMergeRow::Invalid {
                name: name.clone(),
                command: group.command.clone(),
                layer: group.layer,
                invalid_reason: "duplicate-match-command",
            })
        }));
    }

    RouteMergeOutcome {
        effective: EffectiveRoutes {
            routes: order
                .into_iter()
                .map(|key| effective.remove(&key).expect("order key from effective"))
                .collect(),
            suppressed,
            invalid: all_invalid,
        },
        shadowed,
        rows,
    }
}

/// routes.yaml file の read → lossy utf8 → yaml parse → object
/// check(`read_yaml_layer` と同一骨格 — 層 file 規約の route 版)。
/// absent は `Ok(None)`。
fn read_routes_file(file: &Path) -> Result<Option<Map<String, Value>>, CheaperError> {
    let file_path = file.display().to_string();
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => {
            if is_absent_file_error(&e) {
                return Ok(None);
            }
            let mut details = Map::new();
            details.insert("path".to_string(), json!(file_path));
            details.insert("reason".to_string(), json!("read-failed"));
            details.insert("errno".to_string(), json!(io_errno_display(&file_path, &e)));
            return Err(CheaperError::new(
                ErrorCode::InvalidRuntimePolicy,
                format!("route file read failed: {file_path}"),
            )
            .with_details(details)
            .with_cause(e));
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    tracing::trace!(
        file.name = nodepath::basename(&file_path, None).as_str(),
        "route file read",
    );
    parse_routes_text(&text, &file_path).map(Some)
}

fn parse_routes_text(text: &str, file_path: &str) -> Result<Map<String, Value>, CheaperError> {
    let parsed: Value = cheaper_core::yaml::parse_yaml(text).map_err(|e| {
        route_file_error("route yaml parse failed", file_path, "yaml-parse", Some(e))
    })?;
    match parsed {
        Value::Object(doc) => Ok(doc),
        _ => Err(route_file_error(
            "route file is not an object",
            file_path,
            "not-object",
            None::<std::io::Error>,
        )),
    }
}

/// route file の error — `invalid-route` config error として
/// `InvalidRuntimePolicy` で surface する(load 失敗は caller が
/// `invalid-route` viability へ写像する)。
fn route_file_error<E: std::error::Error + Send + Sync + 'static>(
    message: &str,
    file_path: &str,
    reason: &str,
    cause: Option<E>,
) -> CheaperError {
    let mut details = Map::new();
    details.insert("path".to_string(), json!(file_path));
    details.insert("reason".to_string(), json!(reason));
    let err = CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("{message}: {file_path}"),
    )
    .with_details(details);
    match cause {
        Some(c) => err.with_cause(c),
        None => err,
    }
}

/// schema 違反 — 層 file 全体を config error にする(fail-loud、
/// 壊れた route を黙って drop しない)。
fn schema_error(file: &Path, reason: &str) -> CheaperError {
    let mut details = Map::new();
    details.insert("path".to_string(), json!(file.display().to_string()));
    details.insert("reason".to_string(), json!("schema"));
    details.insert("violation".to_string(), json!(reason));
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("invalid route file: {reason}: {}", file.display()),
    )
    .with_details(details)
}

/// routes.yaml document の schema 検証 + route 変換。
/// 同一層内 `match.command` 重複は error ではなく `invalid` bucket
/// へ隔離する。
fn validate_routes_doc(
    doc: &Map<String, Value>,
    layer: RouteLayer,
    layer_root: &Path,
    file: &Path,
) -> Result<(Vec<Route>, Vec<InvalidRouteGroup>), CheaperError> {
    // top-level: `{version: 1, routes: [...]}` — 未知 key 拒否。
    for key in doc.keys() {
        if key != "version" && key != "routes" {
            return Err(schema_error(file, &format!("unknown key: {key}")));
        }
    }
    match doc.get("version") {
        Some(v) if v == &json!(1) => {}
        _ => return Err(schema_error(file, "missing or unsupported version")),
    }
    let Some(routes) = doc.get("routes") else {
        return Err(schema_error(file, "missing routes"));
    };
    let Some(routes) = routes.as_array() else {
        return Err(schema_error(file, "routes is not an array"));
    };
    let mut parsed = Vec::with_capacity(routes.len());
    for (index, entry) in routes.iter().enumerate() {
        let label = format!("routes[{index}]");
        let Some(entry) = entry.as_object() else {
            return Err(schema_error(file, &format!("{label} is not an object")));
        };
        parsed.push(validate_route(entry, layer, layer_root, file, &label)?);
    }
    // 同一層内 dup — collided `match.command` を持つ route は全て
    // `invalid` へ隔離(silent last/first-wins 禁止)。
    let mut counts: HashMap<&Vec<String>, usize> = HashMap::new();
    for route in &parsed {
        *counts.entry(&route.command).or_default() += 1;
    }
    let collided: HashSet<Vec<String>> = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(command, _)| (*command).clone())
        .collect();
    let mut routes = Vec::new();
    let mut invalid: Vec<InvalidRouteGroup> = Vec::new();
    for route in parsed {
        if !collided.contains(&route.command) {
            routes.push(route);
            continue;
        }
        if let Some(group) = invalid
            .iter_mut()
            .find(|g: &&mut InvalidRouteGroup| g.command == route.command)
        {
            group.names.push(route.name);
        } else {
            invalid.push(InvalidRouteGroup {
                command: route.command.clone(),
                names: vec![route.name],
                layer,
            });
        }
    }
    Ok((routes, invalid))
}

/// 1 route 宣言の検証 + `target.path` 解決。
fn validate_route(
    entry: &Map<String, Value>,
    layer: RouteLayer,
    layer_root: &Path,
    file: &Path,
    label: &str,
) -> Result<Route, CheaperError> {
    const KNOWN_KEYS: &[&str] = &[
        "name",
        "match",
        "target",
        "grant",
        "grants",
        "substitute",
        "output",
        "passthroughEnv",
        "trusted",
    ];
    for key in entry.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(schema_error(file, &format!("{label}: unknown key: {key}")));
        }
    }
    let name = required_string(entry, "name", file, label)?;
    // `match: {command: [token, …]}` — 空・非 string token は拒否。
    let match_obj = required_object(entry, "match", file, label)?;
    for key in match_obj.keys() {
        if key != "command" {
            return Err(schema_error(
                file,
                &format!("{label}.match: unknown key: {key}"),
            ));
        }
    }
    let command = match_obj
        .get("command")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            schema_error(
                file,
                &format!("{label}.match.command: missing or not an array"),
            )
        })?
        .iter()
        .map(|token| {
            token
                .as_str()
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    schema_error(
                        file,
                        &format!("{label}.match.command: token must be a non-empty string"),
                    )
                })
        })
        .collect::<Result<Vec<String>, _>>()?;
    if command.is_empty() {
        return Err(schema_error(
            file,
            &format!("{label}.match.command: must not be empty"),
        ));
    }
    // `target: {kind: "workflow", path}`。
    let target = required_object(entry, "target", file, label)?;
    for key in target.keys() {
        if key != "kind" && key != "path" {
            return Err(schema_error(
                file,
                &format!("{label}.target: unknown key: {key}"),
            ));
        }
    }
    match target.get("kind").and_then(Value::as_str) {
        Some("workflow") => {}
        _ => {
            return Err(schema_error(
                file,
                &format!("{label}.target.kind: must be \"workflow\""),
            ))
        }
    }
    let raw_path = target.get("path").and_then(Value::as_str).ok_or_else(|| {
        schema_error(
            file,
            &format!("{label}.target.path: missing or not a string"),
        )
    })?;
    let target_path = resolve_target_path(layer_root, raw_path).ok_or_else(|| {
        schema_error(
            file,
            &format!("{label}.target.path: must be a relative path within the layer: {raw_path}"),
        )
    })?;
    let grant = required_string(entry, "grant", file, label)?;
    // `grants` は trusted 層限定(O7a)— member argv は typed
    // command と独立のため project 層への member 宣言権委譲は
    // O4c の審査迂回を route 宣言経由で再発させる。
    // project 層の `trusted` key 自体も本 file 後段の層 gate で
    // 拒否される — 判別子は宣言層 provenance。
    if layer == RouteLayer::Project && entry.contains_key("grants") {
        return Err(schema_error(
            file,
            &format!("{label}.grants: not declarable in the project layer"),
        ));
    }
    // `grants: [name, …]` — route.grant 以外の追加委譲 grant 名。
    // member 間重複・route.grant 自身の参照は冪等に除去(順序は
    // 意味を持たない — 宣言順の先勝ちを保持)。
    let mut grants = Vec::new();
    match entry.get("grants") {
        None => {}
        Some(v) => {
            let items = v
                .as_array()
                .ok_or_else(|| schema_error(file, &format!("{label}.grants: not an array")))?;
            for item in items {
                let name = item.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
                    schema_error(
                        file,
                        &format!("{label}.grants: member must be a non-empty string"),
                    )
                })?;
                if name != grant && !grants.iter().any(|g| g == name) {
                    grants.push(name.to_string());
                }
            }
        }
    }
    // `substitute` は trusted 層限定(O3a)— substitute mode は typed
    // command を一切実行せず `{substitute.grant, substitute.argv}` を
    // 実行するため、`expected_input` byte-match が typed command へ
    // 課していた拘束が乗り移らない。project 層が宣言できると
    // untrusted authoring が「任意 typed prefix に対して任意の
    // trusted grant を宣言 argv で起動する」実実行ペアの選択権を得る
    // — `grants` member 密輸として O7a が塞いだ confused-deputy hole
    // の、さらに強い形での再発を load 時に拒否する。
    // project 層の `trusted` key 自体も本 file 後段の層 gate で
    // 拒否される — 判別子は宣言層 provenance。
    if layer == RouteLayer::Project && entry.contains_key("substitute") {
        return Err(schema_error(
            file,
            &format!("{label}.substitute: not declarable in the project layer"),
        ));
    }
    // `substitute: {grant, argv}` — 宣言ペア。nested 未知 key は
    // `match`/`target`/`output` と同じ fail-loud 規約で拒否する。
    let substitute = match entry.get("substitute") {
        None => None,
        Some(v) => {
            let Some(obj) = v.as_object() else {
                return Err(schema_error(
                    file,
                    &format!("{label}.substitute: not an object"),
                ));
            };
            for key in obj.keys() {
                if key != "grant" && key != "argv" {
                    return Err(schema_error(
                        file,
                        &format!("{label}.substitute: unknown key: {key}"),
                    ));
                }
            }
            let grant = obj
                .get("grant")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    schema_error(
                        file,
                        &format!("{label}.substitute.grant: missing or not a string"),
                    )
                })?;
            let argv = obj
                .get("argv")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    schema_error(
                        file,
                        &format!("{label}.substitute.argv: missing or not an array"),
                    )
                })?
                .iter()
                .map(|item| {
                    item.as_str()
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| {
                            schema_error(
                                file,
                                &format!(
                                    "{label}.substitute.argv: element must be a non-empty string"
                                ),
                            )
                        })
                })
                .collect::<Result<Vec<String>, CheaperError>>()?;
            Some(RouteSubstitute { grant, argv })
        }
    };
    // `output: {tokenThreshold?}` — number のみ許容。
    let token_threshold = match entry.get("output") {
        None => None,
        Some(output) => {
            let Some(output) = output.as_object() else {
                return Err(schema_error(
                    file,
                    &format!("{label}.output: not an object"),
                ));
            };
            for key in output.keys() {
                if key != "tokenThreshold" {
                    return Err(schema_error(
                        file,
                        &format!("{label}.output: unknown key: {key}"),
                    ));
                }
            }
            match output.get("tokenThreshold") {
                None => None,
                Some(v) => Some(v.as_f64().ok_or_else(|| {
                    schema_error(
                        file,
                        &format!("{label}.output.tokenThreshold: not a number"),
                    )
                })?),
            }
        }
    };
    // `passthroughEnv: [name, …]` — env 名は `[A-Za-z_][A-Za-z0-9_]*`。
    let passthrough_env = match entry.get("passthroughEnv") {
        None => Vec::new(),
        Some(v) => v
            .as_array()
            .ok_or_else(|| schema_error(file, &format!("{label}.passthroughEnv: not an array")))?
            .iter()
            .map(|item| {
                let name = item.as_str().filter(|n| is_env_name(n)).ok_or_else(|| {
                    schema_error(file, &format!("{label}.passthroughEnv: invalid env name"))
                })?;
                Ok(name.to_string())
            })
            .collect::<Result<Vec<String>, CheaperError>>()?,
    };
    // `trusted` は trust 申告そのもの — project 層は申告を置く
    // こと自体ができない(O2a)。`grants`/`substitute` と同じ
    // 判別子(宣言層 provenance)・同じ presence-first 規約で、
    // 値の形状・値(`true`/`false`/非 bool)に関わらず拒否する。
    // trust は `route.layer` が決め、project 自己申告の経路は
    // load 時に閉じる。
    if layer == RouteLayer::Project && entry.contains_key("trusted") {
        return Err(schema_error(
            file,
            &format!("{label}.trusted: not declarable in the project layer"),
        ));
    }
    // `trusted` — builtin/user 層のみ受理される inert bool(その層
    // では元来意味を持たない documented 現状維持 — trust は宣言層
    // provenance が決め、`Route` は flag を持たない)。値は保持せず
    // bool 形状のみ検証する。
    if let Some(v) = entry.get("trusted") {
        if !v.is_boolean() {
            return Err(schema_error(
                file,
                &format!("{label}.trusted: not a boolean"),
            ));
        }
    }
    Ok(Route {
        name,
        command,
        target_path,
        grant,
        grants,
        substitute,
        token_threshold,
        passthrough_env,
        layer,
    })
}

/// `target.path` を層起点で join する — relative path のみ許容し、
/// absolute・`..` component・NUL を含む宣言を `None` で拒否する
/// (層境界を潜る escape の構造的拒否)。
fn resolve_target_path(layer_root: &Path, raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw.contains('\0') {
        return None;
    }
    let path = Path::new(raw);
    for component in path.components() {
        match component {
            // normal segment のみ許容 — `.` は normalize される無害
            // 成分として許可し、`..`/root/prefix は拒否。
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(layer_root.join(path))
}

/// env 名 validation — `[A-Za-z_][A-Za-z0-9_]*`。
fn is_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn required_string(
    entry: &Map<String, Value>,
    key: &str,
    file: &Path,
    label: &str,
) -> Result<String, CheaperError> {
    entry
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .ok_or_else(|| schema_error(file, &format!("{label}.{key}: missing or not a string")))
}

fn required_object<'a>(
    entry: &'a Map<String, Value>,
    key: &str,
    file: &Path,
    label: &str,
) -> Result<&'a Map<String, Value>, CheaperError> {
    entry
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| schema_error(file, &format!("{label}.{key}: missing or not an object")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    /// 3 層分の tempdir を用意する。
    fn layer_dirs() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let base = tempfile::tempdir().unwrap();
        let root = base.path();
        let (builtin, user, project) = (root.join("builtin"), root.join("user"), root.join("proj"));
        std::fs::create_dir_all(builtin.join("workflows")).unwrap();
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        // base の TempDir を返して test 終了まで dir を生かす。
        (base, builtin, user, project)
    }

    fn opts<'a>(builtin: &'a Path, user: &'a Path, project: &'a Path) -> RouteLoadOptions<'a> {
        RouteLoadOptions {
            builtin_dir: builtin,
            user_config_dir: user,
            project_dir: project,
        }
    }

    const ROUTE_YAML: &str = r#"
version: 1
routes:
  - name: cargo-test
    match: {command: [cargo, test]}
    target: {kind: workflow, path: routed-exec.workflow.json}
    grant: cargo-test
    output: {tokenThreshold: 8192}
    passthroughEnv: [CI]
"#;

    #[test]
    fn loads_valid_layer_file() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(&builtin, "routes.yaml", ROUTE_YAML);
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes.len(), 1);
        let route = &loaded.routes[0];
        assert_eq!(route.name, "cargo-test");
        assert_eq!(route.command, vec!["cargo", "test"]);
        assert_eq!(route.grant, "cargo-test");
        assert!(route.grants.is_empty());
        assert_eq!(route.token_threshold, Some(8192.0));
        assert_eq!(route.passthrough_env, vec!["CI"]);
        assert_eq!(route.layer, RouteLayer::Builtin);
        // builtin 層起点は `workflows/`。
        assert_eq!(
            route.target_path,
            builtin.join("workflows").join("routed-exec.workflow.json")
        );
        assert!(loaded.suppressed.is_empty() && loaded.invalid.is_empty());
    }

    #[test]
    fn absent_files_load_as_empty_set() {
        let (_tmp, builtin, user, project) = layer_dirs();
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert!(loaded.routes.is_empty());
    }

    #[test]
    fn malformed_and_unknown_fields_are_config_errors() {
        let (_tmp, builtin, user, project) = layer_dirs();
        for content in [
            "routes: []",                                    // version 欠落
            "version: 1\nroutes: {}\n",                      // routes 非 array
            "version: 1\nroutes: [{name: x}]\n",             // required 欠落
            "version: 1\nroutes: [{name: x, match: {command: []}, target: {kind: workflow, path: a}, grant: g}]\n",
            "version: 1\nroutes: [{name: x, match: {command: [a]}, target: {kind: task, path: a}, grant: g}]\n",
            "version: 1\nroutes: [{name: x, match: {command: [a]}, target: {kind: workflow, path: a}, grant: g, extra: 1}]\n",
            "version: 2\nroutes: []\n",
        ] {
            write(&user, "routes.yaml", content);
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(
                err.code(),
                ErrorCode::InvalidRuntimePolicy,
                "content: {content}"
            );
        }
        // malformed YAML も fail-loud。
        write(&user, "routes.yaml", "version: [");
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn user_shadows_builtin_and_project_never_shadows() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(&builtin, "routes.yaml", ROUTE_YAML);
        // user が同一 match.command で builtin を無条件 shadow
        // (同一 trusted principal の宣言 override — silent、
        // suppressed 記録なし)。
        let user_yaml = ROUTE_YAML.replace("grant: cargo-test", "grant: user-grant");
        write(&user, "routes.yaml", &user_yaml);
        // project 層 route は下位層を shadow しない(trust = 宣言層
        // provenance)— 衝突した project route は負けて suppressed
        // provenance へ記録され、user が勝つ。
        let project_yaml = r#"
version: 1
routes:
  - name: proj-test
    match: {command: [cargo, test]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#;
        write(&project, "routes.yaml", project_yaml);
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes.len(), 1);
        assert_eq!(loaded.routes[0].grant, "user-grant");
        assert_eq!(loaded.routes[0].layer, RouteLayer::User);
        assert_eq!(loaded.suppressed.len(), 1);
        assert_eq!(loaded.suppressed[0].name, "proj-test");
        // 非衝突の project route は通常有効化。
        let unique = r#"
version: 1
routes:
  - name: proj-only
    match: {command: [make, build]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#;
        write(&project, "routes.yaml", unique);
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes.len(), 2);
        assert!(loaded.suppressed.is_empty());
    }

    #[test]
    fn same_layer_duplicates_are_quarantined_as_invalid() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(
            &user,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: first
    match: {command: [cargo, test]}
    target: {kind: workflow, path: a.json}
    grant: g1
  - name: second
    match: {command: [cargo, test]}
    target: {kind: workflow, path: b.json}
    grant: g2
  - name: other
    match: {command: [cargo, build]}
    target: {kind: workflow, path: c.json}
    grant: g3
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        // collided 2 件は invalid 隔離、非衝突の other は有効。
        assert_eq!(loaded.routes.len(), 1);
        assert_eq!(loaded.routes[0].name, "other");
        assert_eq!(loaded.invalid.len(), 1);
        assert_eq!(loaded.invalid[0].command, vec!["cargo", "test"]);
        assert_eq!(loaded.invalid[0].names, vec!["first", "second"]);
    }

    #[test]
    fn target_path_must_be_layer_relative() {
        let (_tmp, builtin, user, project) = layer_dirs();
        for path in [
            "../escape.json",
            "a/../../escape.json",
            "/abs/escape.json",
            "..",
            "",
        ] {
            let yaml = format!(
                "version: 1\nroutes:\n  - name: x\n    match: {{command: [a]}}\n    target: {{kind: workflow, path: \"{path}\"}}\n    grant: g\n"
            );
            write(&project, "routes.yaml", &yaml);
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy, "path: {path}");
        }
        // project 層起点は `.cheaper/`(routes.yaml と同 dir)。
        let yaml = "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: \"w/f.json\"}\n    grant: g\n";
        write(&project, "routes.yaml", yaml);
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(
            loaded.routes[0].target_path,
            project.join("w").join("f.json")
        );
    }

    /// `grants` 宣言の parse/dedupe(#166 R1)— member のみ宣言順で
    /// 保持し、member 間重複・`route.grant` self-ref・`grants: []`
    /// は冪等に除去される。
    #[test]
    fn grants_parse_dedupe_and_empty() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(
            &user,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: multi
    match: {command: [cargo, test]}
    target: {kind: workflow, path: a.json}
    grant: cargo-test
    grants: [fmt, report]
  - name: dupes
    match: {command: [cargo, build]}
    target: {kind: workflow, path: b.json}
    grant: cargo-build
    grants: [fmt, cargo-build, fmt]
  - name: empty
    match: {command: [cargo, check]}
    target: {kind: workflow, path: c.json}
    grant: cargo-check
    grants: []
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        let by_name = |name: &str| loaded.routes.iter().find(|r| r.name == name).unwrap();
        assert_eq!(by_name("multi").grants, vec!["fmt", "report"]);
        // member 間重複と route.grant self-ref は冪等に除去。
        assert_eq!(by_name("dupes").grants, vec!["fmt"]);
        // `grants: []` ≡ 未指定。
        assert!(by_name("empty").grants.is_empty());
    }

    /// malformed `grants` — 非 array・非 string member・空 string
    /// member は `InvalidRuntimePolicy` + `details.{path, reason:
    /// "schema", violation}` で file と entry を名指しする
    /// route-load error。`grants` typo の unknown key 拒否は不変。
    #[test]
    fn grants_malformed_is_schema_error() {
        let (_tmp, builtin, user, project) = layer_dirs();
        for (grants, expect) in [
            ("grants: fmt", "not an array"),
            ("grants: [1]", "member must be a non-empty string"),
            ("grants: ['']", "member must be a non-empty string"),
        ] {
            let yaml = format!(
                "version: 1\nroutes:\n  - name: x\n    match: {{command: [a]}}\n    target: {{kind: workflow, path: f.json}}\n    grant: g\n    {grants}\n"
            );
            let file = write(&user, "routes.yaml", &yaml);
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy, "{grants}");
            let details = err.details().unwrap();
            assert_eq!(details["reason"], json!("schema"), "{grants}");
            assert_eq!(
                details["path"].as_str().unwrap(),
                file.display().to_string(),
                "{grants}"
            );
            let violation = details["violation"].as_str().unwrap();
            assert!(
                violation.contains("routes[0].grants") && violation.contains(expect),
                "{grants}: {violation}"
            );
        }
        // `grants` の typo は unknown key 拒否のまま(fail-loud 不変)。
        write(
            &user,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    grantss: [a]\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(err.details().unwrap()["violation"]
            .as_str()
            .unwrap()
            .contains("unknown key: grantss"));
    }

    /// `grants` 宣言の層制約(#166 O7a)— project 層 entry の
    /// `grants` key presence は member 形状検査より先に fail-loud
    /// `InvalidRuntimePolicy` で拒否する。builtin/user 層は受理
    /// され、project 層の `grant` 単数宣言は従来どおり有効。
    #[test]
    fn project_layer_grants_are_rejected() {
        let (_tmp, builtin, user, project) = layer_dirs();
        // member 形状が不正でも presence が先に拒否される
        // (untrusted 層の宣言は内容を評価しない fail-closed)。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    grants: not-an-array\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        let violation = err.details().unwrap()["violation"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            violation.contains("routes[0].grants: not declarable in the project layer"),
            "{violation}"
        );
        // project 層の `trusted` key 自体も層 gate で拒否される —
        // gate 発火順は schema 記述順で grants が先に報告される。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    trusted: true\n    grants: [m]\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        // project 層の `grant` 単数宣言は従来どおり受理。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n",
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes[0].grant, "g");
        assert!(loaded.routes[0].grants.is_empty());
        // builtin/user 層の `grants` 宣言は受理される。
        std::fs::remove_file(project.join("routes.yaml")).unwrap();
        for dir in [&builtin, &user] {
            write(
                dir,
                "routes.yaml",
                "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    grants: [m]\n",
            );
            let loaded = load(&opts(&builtin, &user, &project)).unwrap();
            assert_eq!(loaded.routes[0].grants, vec!["m"]);
            std::fs::remove_file(dir.join("routes.yaml")).unwrap();
        }
    }

    /// `Route::grant_set()` — `{route.grant} ∪ grants` の有効 set
    /// の単一供給点。`grants: []` なら anchor のみを返す。
    #[test]
    fn grant_set_unions_anchor_and_members() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(
            &user,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: multi
    match: {command: [cargo, test]}
    target: {kind: workflow, path: a.json}
    grant: cargo-test
    grants: [fmt, report]
  - name: solo
    match: {command: [cargo, build]}
    target: {kind: workflow, path: b.json}
    grant: cargo-build
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        let by_name = |name: &str| loaded.routes.iter().find(|r| r.name == name).unwrap();
        let set = by_name("multi").grant_set();
        assert_eq!(
            set.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["cargo-test", "fmt", "report"]
        );
        let set = by_name("solo").grant_set();
        assert_eq!(
            set.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["cargo-build"]
        );
    }

    /// `substitute: {grant, argv}` 宣言の parse(#167 R1/O1a)—
    /// `Route.substitute` が `Some(RouteSubstitute)` になり、`argv: []`
    /// は executable 単独起動の明示宣言として受理する。`grant` 単数は
    /// substitute の有無に関わらず required(firing gate・attribution
    /// anchor・投影 member)。
    #[test]
    fn substitute_parse_and_empty_argv() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(
            &user,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: sub
    match: {command: [git, push]}
    target: {kind: workflow, path: w.json}
    grant: git-push
    substitute: {grant: git-dry, argv: [push, --dry-run]}
  - name: solo
    match: {command: [git, fetch]}
    target: {kind: workflow, path: w.json}
    grant: git-fetch
    substitute: {grant: git-fetch, argv: []}
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        let by_name = |name: &str| loaded.routes.iter().find(|r| r.name == name).unwrap();
        assert_eq!(
            by_name("sub").substitute,
            Some(RouteSubstitute {
                grant: "git-dry".to_string(),
                argv: vec!["push".to_string(), "--dry-run".to_string()],
            })
        );
        // `argv: []` は受理、`substitute.grant == route.grant` も合法。
        assert_eq!(
            by_name("solo").substitute,
            Some(RouteSubstitute {
                grant: "git-fetch".to_string(),
                argv: Vec::new(),
            })
        );
        // `grant` 単数は substitute 宣言の有無に関わらず required。
        write(
            &user,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    substitute: {grant: g, argv: []}\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(err.details().unwrap()["violation"]
            .as_str()
            .unwrap()
            .contains("routes[0].grant: missing or not a string"));
    }

    /// malformed `substitute` — 非 object・`grant`/`argv` 欠落・nested
    /// 未知 key・`grant` 非 string/空・`argv` 非 array・要素非
    /// string/空は `InvalidRuntimePolicy` + `details.{path,
    /// reason:"schema", violation}` で file と entry を名指しする
    /// route-load error(`match`/`target`/`output` と同じ fail-loud
    /// 規約)。`substitute` typo の unknown key 拒否は不変。
    #[test]
    fn substitute_malformed_is_schema_error() {
        let (_tmp, builtin, user, project) = layer_dirs();
        for (sub, expect) in [
            ("substitute: str", "substitute: not an object"),
            (
                "substitute: {argv: [a]}",
                "substitute.grant: missing or not a string",
            ),
            (
                "substitute: {grant: g}",
                "substitute.argv: missing or not an array",
            ),
            (
                "substitute: {grant: g, argv: [a], extra: 1}",
                "substitute: unknown key: extra",
            ),
            (
                "substitute: {grant: 1, argv: [a]}",
                "substitute.grant: missing or not a string",
            ),
            (
                "substitute: {grant: '', argv: [a]}",
                "substitute.grant: missing or not a string",
            ),
            (
                "substitute: {grant: g, argv: x}",
                "substitute.argv: missing or not an array",
            ),
            (
                "substitute: {grant: g, argv: [1]}",
                "substitute.argv: element must be a non-empty string",
            ),
            (
                "substitute: {grant: g, argv: ['']}",
                "substitute.argv: element must be a non-empty string",
            ),
        ] {
            let yaml = format!(
                "version: 1\nroutes:\n  - name: x\n    match: {{command: [a]}}\n    target: {{kind: workflow, path: f.json}}\n    grant: g\n    {sub}\n"
            );
            let file = write(&user, "routes.yaml", &yaml);
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy, "{sub}");
            let details = err.details().unwrap();
            assert_eq!(details["reason"], json!("schema"), "{sub}");
            assert_eq!(
                details["path"].as_str().unwrap(),
                file.display().to_string(),
                "{sub}"
            );
            let violation = details["violation"].as_str().unwrap();
            assert!(
                violation.contains("routes[0].substitute") && violation.contains(expect),
                "{sub}: {violation}"
            );
        }
        // `substitute` の typo は unknown key 拒否のまま(fail-loud 不変)。
        write(
            &user,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    substitut: {grant: g, argv: []}\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(err.details().unwrap()["violation"]
            .as_str()
            .unwrap()
            .contains("unknown key: substitut"));
    }

    /// `substitute` 宣言の層制約(#167 O3a)— project 層 entry の
    /// `substitute` key presence は形状検査より先に fail-loud
    /// `InvalidRuntimePolicy` で拒否する。builtin/user 層は受理
    /// され、project 層の `grant:` 単数・byte-matched route は
    /// 従来どおり有効。
    #[test]
    fn project_layer_substitute_is_rejected() {
        let (_tmp, builtin, user, project) = layer_dirs();
        // 形状が不正でも presence が先に拒否される(untrusted 層の
        // 宣言は内容を評価しない fail-closed — `grants` O7a と同型)。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    substitute: not-an-object\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        let violation = err.details().unwrap()["violation"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            violation.contains("routes[0].substitute: not declarable in the project layer"),
            "{violation}"
        );
        // project 層の `trusted` key 自体も層 gate で拒否される —
        // gate 発火順は schema 記述順で substitute が先に報告される。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    trusted: true\n    substitute: {grant: g, argv: []}\n",
        );
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
        // project 層の `grant:` 単数・byte-matched route は従来どおり受理。
        write(
            &project,
            "routes.yaml",
            "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n",
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes[0].grant, "g");
        assert!(loaded.routes[0].substitute.is_none());
        // builtin/user 層の `substitute` 宣言は受理される。
        std::fs::remove_file(project.join("routes.yaml")).unwrap();
        for dir in [&builtin, &user] {
            write(
                dir,
                "routes.yaml",
                "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    substitute: {grant: s, argv: [x]}\n",
            );
            let loaded = load(&opts(&builtin, &user, &project)).unwrap();
            assert_eq!(loaded.routes[0].substitute.as_ref().unwrap().grant, "s");
            std::fs::remove_file(dir.join("routes.yaml")).unwrap();
        }
    }

    /// `Route::grant_set()` の substitute 拡張(#167 O7a)—
    /// `{route.grant} ∪ grants ∪ {substitute.grant}` の union。
    /// `substitute.grant == route.grant`・`substitute.grant ∈ grants[]`
    /// は dedupe で単一要素になる。
    #[test]
    fn grant_set_includes_substitute_grant() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(
            &user,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: multi
    match: {command: [git, push]}
    target: {kind: workflow, path: w.json}
    grant: git-push
    grants: [fmt]
    substitute: {grant: git-dry, argv: [push, --dry-run]}
  - name: same
    match: {command: [git, fetch]}
    target: {kind: workflow, path: w.json}
    grant: git-fetch
    grants: [fmt]
    substitute: {grant: fmt, argv: []}
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        let by_name = |name: &str| loaded.routes.iter().find(|r| r.name == name).unwrap();
        assert_eq!(
            by_name("multi")
                .grant_set()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["fmt", "git-dry", "git-push"]
        );
        // `substitute.grant ∈ grants[]` は dedupe。
        assert_eq!(
            by_name("same")
                .grant_set()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["fmt", "git-fetch"]
        );
    }

    /// `Route::exec_pair()`(#167 O2a/O4a)— substitute mode は宣言
    /// `{substitute.grant, substitute.argv}`、byte-matched は
    /// `{route.grant, typed_args}` を返す実行ペア選択の単一供給点。
    #[test]
    fn exec_pair_selects_execution_pair() {
        let typed = vec!["push".to_string(), "origin".to_string(), "main".to_string()];
        let mut route = Route {
            name: "r".to_string(),
            command: vec!["git".to_string(), "push".to_string()],
            target_path: PathBuf::from("w.json"),
            grant: "git-push".to_string(),
            grants: Vec::new(),
            substitute: Some(RouteSubstitute {
                grant: "git-dry".to_string(),
                argv: vec!["push".to_string(), "--dry-run".to_string()],
            }),
            token_threshold: None,
            passthrough_env: Vec::new(),
            layer: RouteLayer::User,
        };
        let (name, args) = route.exec_pair(&typed);
        assert_eq!(name, "git-dry");
        assert_eq!(
            args.to_vec(),
            vec!["push".to_string(), "--dry-run".to_string()]
        );
        // substitute 無しは `{route.grant, typed_args}`(byte-matched 不変)。
        route.substitute = None;
        let (name, args) = route.exec_pair(&typed);
        assert_eq!(name, "git-push");
        assert_eq!(args, typed.as_slice());
    }

    /// merge suppressed provenance(#168)— project 層 route は下位層を
    /// shadow しないため、同一 `match.command` 衝突では project 側が
    /// 常に負けて自身が `suppressed` へ記録される(displaced 下位層
    /// route は発生しない — #167 の displaced 記録 arm は到達不能で
    /// 除去)。builtin/user 層間の shadow は同一 trusted principal の
    /// 宣言 override として silent のまま(suppressed 記録なし)。
    #[test]
    fn merge_records_losing_project_route() {
        let (_tmp, builtin, user, project) = layer_dirs();
        // builtin/user 層間 shadow は silent(user → builtin は
        // 同一 trusted principal の宣言 override)。
        write(&builtin, "routes.yaml", ROUTE_YAML);
        let user_yaml = ROUTE_YAML.replace("grant: cargo-test", "grant: user-grant");
        write(&user, "routes.yaml", &user_yaml);
        // 衝突する project route は user route に負けて自身が
        // suppressed へ記録。非衝突の project route は通常有効化。
        write(
            &project,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: proj-shadow
    match: {command: [cargo, test]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
  - name: proj-new
    match: {command: [make, build]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#,
        );
        let loaded = load(&opts(&builtin, &user, &project)).unwrap();
        assert_eq!(loaded.routes.len(), 2);
        // 勝つのは user 層 route — project route は下位層を shadow しない。
        assert_eq!(loaded.routes[0].name, "cargo-test");
        assert_eq!(loaded.routes[0].layer, RouteLayer::User);
        assert_eq!(loaded.routes[1].name, "proj-new");
        // 負けた project route 自身が suppressed へ記録される。
        assert_eq!(loaded.suppressed.len(), 1);
        assert_eq!(loaded.suppressed[0].name, "proj-shadow");
        assert_eq!(loaded.suppressed[0].command, vec!["cargo", "test"]);
    }

    /// `trusted` 宣言の層制約(#168 O2a)— project 層 entry の
    /// `trusted` key presence は bool 形状検査より先に fail-loud
    /// `InvalidRuntimePolicy` で拒否する(presence-first —
    /// `true`/`false`/非 bool の全形を値を見ずに拒否)。builtin/user
    /// 層の `trusted` は受理・bool 検証・inert(値は `Route` に保持
    /// しない — field は削除済み)。
    #[test]
    fn project_layer_trusted_is_rejected() {
        let (_tmp, builtin, user, project) = layer_dirs();
        // 値が非 bool でも presence が先に拒否される(untrusted 層の
        // 宣言は内容を評価しない fail-closed — `grants`/`substitute`
        // と同型)。`details.{path, reason:"schema", violation}` が
        // file と entry を名指しする。
        for trusted in ["trusted: true", "trusted: false", "trusted: str"] {
            let file = write(
                &project,
                "routes.yaml",
                &format!(
                    "version: 1\nroutes:\n  - name: x\n    match: {{command: [a]}}\n    target: {{kind: workflow, path: f.json}}\n    grant: g\n    {trusted}\n"
                ),
            );
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy, "{trusted}");
            let details = err.details().unwrap();
            assert_eq!(details["reason"], json!("schema"), "{trusted}");
            assert_eq!(
                details["path"].as_str().unwrap(),
                file.display().to_string(),
                "{trusted}"
            );
            let violation = details["violation"].as_str().unwrap();
            assert!(
                violation.contains("routes[0].trusted: not declarable in the project layer"),
                "{trusted}: {violation}"
            );
        }
        // builtin/user 層の `trusted: true`/`false` 宣言は受理・inert。
        std::fs::remove_file(project.join("routes.yaml")).unwrap();
        for dir in [&builtin, &user] {
            for trusted in ["trusted: true", "trusted: false"] {
                write(
                    dir,
                    "routes.yaml",
                    &format!(
                        "version: 1\nroutes:\n  - name: x\n    match: {{command: [a]}}\n    target: {{kind: workflow, path: f.json}}\n    grant: g\n    {trusted}\n"
                    ),
                );
                let loaded = load(&opts(&builtin, &user, &project)).unwrap();
                assert_eq!(loaded.routes[0].name, "x", "{trusted}");
            }
            // 非 bool 値は受理層の型検査規約どおり schema_error。
            write(
                dir,
                "routes.yaml",
                "version: 1\nroutes:\n  - name: x\n    match: {command: [a]}\n    target: {kind: workflow, path: f.json}\n    grant: g\n    trusted: str\n",
            );
            let err = load(&opts(&builtin, &user, &project)).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
            let violation = err.details().unwrap()["violation"].as_str().unwrap();
            assert!(
                violation.contains("routes[0].trusted: not a boolean"),
                "{violation}"
            );
            std::fs::remove_file(dir.join("routes.yaml")).unwrap();
        }
    }

    fn eval_fs(layer: RouteLayer, layer_root: &Path, dir: &Path) -> RouteLayerEvaluation {
        evaluate_filesystem_layer(layer, &dir.join("routes.yaml"), layer_root)
    }

    #[test]
    fn merge_diagnostics_exposes_shadowed_suppressed_and_invalid_provenance() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(&builtin, "routes.yaml", ROUTE_YAML);
        write(
            &user,
            "routes.yaml",
            &ROUTE_YAML.replace("grant: cargo-test", "grant: user-grant"),
        );
        write(
            &project,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: proj-shadow
    match: {command: [cargo, test]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
  - name: proj-new
    match: {command: [make, build]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#,
        );
        // user 層の same-layer duplicate は有効 route から隔離され、
        // merge status としても常に invalid のまま。
        let duplicated = r#"
version: 1
routes:
  - name: first
    match: {command: [dup, run]}
    target: {kind: workflow, path: a.json}
    grant: g1
  - name: second
    match: {command: [dup, run]}
    target: {kind: workflow, path: b.json}
    grant: g2
"#;
        let mut user_eval = eval_fs(RouteLayer::User, &user, &user);
        let extra_eval =
            evaluate_routes_text(RouteLayer::User, "/logical/extra.yaml", duplicated, &user);
        user_eval.invalid.extend(extra_eval.invalid);
        user_eval.routes.extend(extra_eval.routes);

        let merged = merge_route_layers(&[
            eval_fs(RouteLayer::Builtin, &builtin.join("workflows"), &builtin),
            user_eval,
            eval_fs(RouteLayer::Project, &project, &project),
        ]);
        let statuses: Vec<_> = merged
            .rows
            .iter()
            .map(|row| (row.name(), row.layer(), row.status()))
            .collect();
        assert_eq!(
            statuses,
            vec![
                (
                    "cargo-test",
                    RouteLayer::Builtin,
                    RouteMergeStatus::Shadowed
                ),
                ("cargo-test", RouteLayer::User, RouteMergeStatus::Active),
                ("first", RouteLayer::User, RouteMergeStatus::Invalid),
                ("second", RouteLayer::User, RouteMergeStatus::Invalid),
                (
                    "proj-shadow",
                    RouteLayer::Project,
                    RouteMergeStatus::Suppressed
                ),
                ("proj-new", RouteLayer::Project, RouteMergeStatus::Active),
            ]
        );
        assert_eq!(merged.shadowed.len(), 1);
        assert_eq!(merged.shadowed[0].layer, RouteLayer::Builtin);
        assert_eq!(merged.effective.routes.len(), 2);
        assert_eq!(merged.effective.suppressed.len(), 1);
        assert_eq!(merged.effective.invalid.len(), 1);
    }

    #[test]
    fn failed_user_layer_makes_builtin_merge_dependent_rows_indeterminate() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(&builtin, "routes.yaml", ROUTE_YAML);
        write(&user, "routes.yaml", "version: [");
        write(
            &project,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: proj-only
    match: {command: [make, build]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#,
        );
        let layers = [
            eval_fs(RouteLayer::Builtin, &builtin.join("workflows"), &builtin),
            eval_fs(RouteLayer::User, &user, &user),
            eval_fs(RouteLayer::Project, &project, &project),
        ];
        assert_eq!(layers[0].status, RouteLayerStatus::Loaded);
        assert_eq!(layers[1].status, RouteLayerStatus::Failed);
        assert_eq!(layers[2].status, RouteLayerStatus::Loaded);

        let merged = merge_route_layers(&layers);
        assert_eq!(merged.rows[0].status(), RouteMergeStatus::Indeterminate);
        assert_eq!(merged.rows[1].status(), RouteMergeStatus::Indeterminate);
        // merge 失敗を表現しても従来の load は最初の失敗層で
        // fail-loud のまま。
        let err = load(&opts(&builtin, &user, &project)).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn known_lower_collision_stays_suppressed_when_sibling_layer_fails() {
        let (_tmp, builtin, user, project) = layer_dirs();
        write(&builtin, "routes.yaml", ROUTE_YAML);
        write(&user, "routes.yaml", "version: [");
        write(
            &project,
            "routes.yaml",
            r#"
version: 1
routes:
  - name: proj-shadow
    match: {command: [cargo, test]}
    target: {kind: workflow, path: p.json}
    grant: proj-grant
"#,
        );
        let merged = merge_route_layers(&[
            eval_fs(RouteLayer::Builtin, &builtin.join("workflows"), &builtin),
            eval_fs(RouteLayer::User, &user, &user),
            eval_fs(RouteLayer::Project, &project, &project),
        ]);
        assert_eq!(merged.rows[0].status(), RouteMergeStatus::Indeterminate);
        assert_eq!(merged.rows[1].status(), RouteMergeStatus::Suppressed);
    }

    #[test]
    fn embedded_route_document_uses_same_schema_and_merge_rules() {
        let layer = evaluate_routes_text(
            RouteLayer::Builtin,
            "/builtin/routes.yaml",
            ROUTE_YAML,
            Path::new("/builtin/workflows"),
        );
        assert_eq!(layer.source, "builtin:routes.yaml");
        let merged = merge_route_layers(&[layer]);
        assert_eq!(merged.rows.len(), 1);
        assert_eq!(merged.rows[0].status(), RouteMergeStatus::Active);
        assert_eq!(
            merged.rows[0].route().unwrap().target_path,
            PathBuf::from("/builtin/workflows/routed-exec.workflow.json")
        );
    }
}

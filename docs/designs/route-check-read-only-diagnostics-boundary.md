---
id: "DESIGN-034N3kihuoqVnngaNEt7q2"
type: "design"
status: "approved"
title: "Route check read-only diagnostics boundary"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/185"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/route-check-diagnostics-issue-185.md"
    - "../adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
    - "../discovery/explore-route-check-diagnostics-issue-185.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check read-only diagnostics boundary

## Context

`cheaper route check` は、全 route の宣言・merge・grant・workflow・executable readiness を実行せずに診断する utility command である。既存の `route resolve` は単一 sample の match 結果を返し、`RouteSupply::load` は embedded builtin resource を temp dir へ materialize する。さらに `run_cli` は parse/dispatch 前に observability を初期化し、dispatch の generic error path でも builtin dir 解決を行うため、`route check` の strict read-only contract と競合する経路が複数ある。

ADR-034N3bHdJMNN63xqMHvKH9 で採用した方針に従い、本設計は embedded resource を filesystem path として供給せず、read-only document/source boundary 経由で route・policy・workflow・worker・schema を共有 validation contract に渡す。diagnostics 側で route loader・policy predicate・workflow convention・viability 優先順を複製しない。一方で、既存の shared predicate が filesystem 前提で持つ参照点（target existence・worker schema file・policy-derived guard）は、source-aware seam として明示的に注入する。

## Scope

### In Scope

- `route check` CLI action、引数解析、dispatch、text/JSON 出力、exit status mapping。
- pre-parse の command detection による `route check` 判定。判定は option の位置・`--` terminator をまたぐ strict parse と同じ positional 規約を共有し、logging suppression と generic error-path gate の両方に使う。
- `route check` の tracing/observability sink 完全抑止。`CHEAPER_LOG`・`--log` が有効でも file/stderr sink を初期化しない。
- `route check` の generic catch で Usage/History 記録 context を解決しない。error envelope と stderr diagnostic は返すが、`resolved_user_config_dir`・`resolve_builtin_dir`・`record_usage*`・`finalize_worker_history` には進まない。
- `embedded.rs` の read-only resource catalog。`builtin_dir()` materialization を呼ばず、embedded `routes.yaml`・`policy.yaml`・`workflows/*`・`workers/*`・`workers/schemas/*` を内容参照する。
- builtin/user/project 各 layer を独立して診断する route loading report。absent layer は正常な空 layer、read/parse/schema failure は layer diagnostic、valid sibling layer の route 行は保持する。
- `routes::load` と diagnostics の両方が使う shared layer validate/merge step。merge は user-over-builtin `shadowed` provenance を記録し、layer failure 時の merge-dependent status を `indeterminate` にする。
- core policy loading の document 入力経路。builtin policy を embedded content から、user/project policy を既存 file read contract から読み、同一 schema/merge/normalize pipeline を通す。
- filesystem/embedded 両 source に対応した workflow・worker・worker-schema validation。`validate_definition`・`validate_worker_definition`・`load_schema` の検証規約は共有し、file content の取得方法だけを注入する。
- `route::resolve` の match/viability predicate を使った sample-aware evaluator。embedded builtin target existence は source-aware predicate として shared viability へ注入し、`missing-target` 判定だけを複製しない。
- renderer capability の静的報告。host registration/approval/execution は常に `unverified`。
- CLI help、`docs/guides/parent-agent-integration.md`、`docs/guides/configuration.md` の利用・制約説明。

### Out of Scope

- route/workflow/worker/process の実行、provider/model/auth/network access。
- route/policy/credential/History/Usage/RouteLogs/observability file/diagnostic cache 等の書き込み。
- embedded builtin temp cache の生成や既存 `route resolve`/`route run`/`hook` 経路の materialization 契約変更。
- host-side hook registration/approval/rewrite の実機検証（#162）。
- 設定修復、grant 発行、policy 緩和、project override の自動追加。
- `load_or_none` fail-open loader、診断専用 grant/policy/workflow predicate の複製。

## Components and Boundaries

- `args.rs` — `RouteAction::Check { cwd, command, argv }` を追加し、`--command`/`--argv` は at-most-one（XOR、両方省略 = sample 無し）、`--cwd` 規約は `route resolve` に揃える。`resolve` arm の shared cardinality check に巻き込まず、`check` arm 側で「0 または 1」の判定を行う。`route check` の pre-parse 判定は、strict parse と同じ `STRING_OPTIONS`/`BOOL_OPTIONS` arity table と `--` terminator 規約で positional token を畳む helper（例: `args::is_route_check_invocation`）に置く。known string option は inline/次 token の値を消費し（次 token が option-like でも pre-parse 判定では値として消費し、ambiguous 判定は strict parse に任せる）、bool option は消費せず、unknown `--*`/`-*` option は arity 0 として次 token を消費しない。`--` 以後の token は全て positional として畳み、positionals の先頭が `["route", "check"]` なら true（余分な positional があっても true — strict parse が後で reject する malformed invocation でも suppression は fail-safe に有効化する）。`route checks`/`route CHECK` のような一致しない語は detection 対象外とし、通常の unknown-command error に任せる。`cheaper --log debug route check`、`cheaper --quiet route check`、`cheaper --format json route check`、`cheaper route --log debug check`、`cheaper route -- check`、`cheaper -- route check`、`cheaper route check --bogus` を同じ detection contract で扱う。global `--help`/`--version` は strict parse 上 route action より先に評価される既存 precedence を維持する。
- `cli.rs` — `args::is_route_check_invocation` を observability init 前に評価する。true の場合、`flag_level`/`flag_format`/`env_level`/`env_format` を全て `None`、stderr sink も無効化した `Activation` を作り、logging を完全 Off にする（この経路では `global_log_flags`/`resolve_log_root` を評価しない）。`Activation.log_root` には unused placeholder として `PathBuf::new()` を渡す。resolved level Off は sink 構築前に return するため、この値は読まれない。dispatch Err arm では、同じ flag で `resolved_user_config_dir`/`record_usage*`/`finalize_worker_history` を bypass する。`route check` の Err は sanitized error envelope + stderr diagnostic + mapped exit code のみを返し、builtin dir 解決へ進まない。sanitized envelope は stable error code・generic message・bounded reason のみを許可し、raw argv/command/env value、`--cwd` 値、OS error Display、cause/details の host path を serialize しない。
- `embedded.rs` — `builtin_resources()` を materialization 用の private list のまま、public(crate) `EmbeddedResources` accessor（例: `read/contains`）で包む。key は `/` 正規化の logical relative path。返すのは byte/text/document であり、filesystem path ではない。
- `route::check` — command orchestration、diagnostic assembly、stable code/status/exit mapping、secret-safe projection を所有する。route merge・policy 判定・workflow schema・worker schema・exec grant predicate・viability 優先順は既存 module の共有関数を呼ぶ。
- `route::routes` — `read_routes_file`/`validate_routes_doc` と merge fold を、filesystem `load` と diagnostics の両方が使う shared step へ整理する。merge result は `EffectiveRoutes` に加えて `shadowed` provenance を返し、user-over-builtin replacement を記録する。diagnostics entrypoint は各 layer を独立して評価し、1 layer の失敗で valid sibling layer を捨てない。`suppressed`/`shadowed` row は merge provenance を per-layer parsed `Route` に join して宣言値を保持し、`invalid` row は同層 duplicate group の name(s)/command/layer/trust のみを持つ shallow row とする。
- `cheaper-core::policy::load` — path-based `load_policy` は維持し、builtin/user/project document を既に取得済みの caller が同一 validation/merge/normalize pipeline を通せる入力 API（例: `load_policy_documents`）を追加する。
- `route::supply` — 実行経路の `RouteSupply::load` は変更しない。`route check` 専用の supply builder は同一 config-dir/root 解決規約を使うが、builtin source は embedded/file のいずれかを注入し、`embedded::builtin_dir()` は呼ばない。
- `workflow_context.rs` / `cheaper-workflow::loader` — file read を行う既存 wrapper と、document text から `validate_definition` までを行う shared validation 経路を分離する。`validate_routed_convention` は `pub(crate)` seam にして `workflow_run` 実行 path と同じ関数を再利用する。
- `worker::resolve` / `worker::validate` / `worker::schema` — worker definition 本体と `schema: {file: ...}` の両方を `WorkerContentSource`/`SchemaContentSource` 相当で取得する。filesystem layer は既存 path read、embedded builtin layer は catalog read に写す。`resolve_worker` の builtin candidate と `resolve_worker_index` が作る `WorkerCatalog` は同じ source-aware resolver を使い、project → user → builtin precedence を維持する。`source_path` は display/error attribution 用の logical path とし、schema base-dir resolution も同じ source に渡す。
- `route::resolve` — env-prefix strip・self-bypass・prefix match・longest/tie・suppressed attribution を `resolve()` から `match_command` 相当の shared phase へ分離し、`resolve()` はその結果を使って viability を評価する。shared phase は normalized argv と「1 route の宣言 command が normalized argv に prefix match するか」を返す predicate を提供し、`route check` は全 layer が non-failed のとき effective/non-effective row を問わず `matched`/`selected` を同じ predicate で得る。`ResolveInput` または viability 入力には `target_exists` 相当の source-aware predicate を追加する。既定は `Path::is_file` で runtime 挙動を保持し、`route check` は layer source へ解決する predicate を渡す。`missing-target` の優先順位置は変えない。
- `hook::targets` — `HookTarget::ALL` と rewrite renderer capability を route check へ提供する。capability は render contract と同じ owner に置き、codex の policy-dependent approval-bypass と静的 rewrite support を分離する。

## Source model

`RouteLayerSource` は layer が所有する resource の read-only provider である。

```text
RouteLayerSource =
  Filesystem { routes_file, workflow_root, worker_base_dir }
| Embedded   { catalog }
```

- builtin production source は `Embedded`。dev/test で `CliHooks.builtin_dir` が指定された場合のみ `Filesystem`。
- user/project source は常に `Filesystem`。`worker_base_dir` は `workers/` 自体ではなく layer base dir（builtin dir、user config dir、project dir）であり、resolver が `workers/<name>.md` を join する。
- `Route` の内部 `target_path` は現行 `PathBuf` 契約を維持する。diagnostics は `Route` の `layer` と宣言時の layer root から target relative path を復元し、source-aware existence/read に使う。embedded builtin の synthetic layer root は logical `/builtin/` で固定し、ConfigDirs/target attribution には host path を使わない。
- 出力の `source` は `<layer>:<relative>` 形式の logical path に限定する（例: `builtin:routes.yaml`、`builtin:workflows/routed-exec.workflow.json`、`user:routes.yaml`、`project:routes.yaml`）。host filesystem の absolute path・home path・temp path・`searched` candidate path は出力しない。
- worker/schema lookup は `workers/<name>.md` と `workers/schemas/<file>` を同じ source 規約で読む。embedded schema base-dir は `/builtin/` を root とする `/` 正規化 logical path であり、`SchemaContentSource` が `{file:}` の join を所有する。`nodepath::resolve`・platform `Path::parent` の結果を catalog key へ直接渡さず、`\\` は `/` へ normalize して `/builtin/` prefix を strip した relative key で catalog lookup する。

## Data and Control Flow

1. `run_cli` は parse 前に `args::is_route_check_invocation(argv)` を評価する。true の場合、observability `Activation` は level Off・stderr sink 無効として扱い、file/stderr tracing sink を作らない。
2. `args` は `RouteAction::Check` を生成する。`--command` は `route resolve` の `parse_command` を `pub(crate)` seam として再利用して argv tokens と `unsafe_command` flag を得、`--argv` は同じく `parse_argv` を `pub(crate)` seam として再利用して JSON string array を parse する。sample 未指定は `None`。
3. `route::check` は `--cwd`/`io.cwd` から workspace root を canonicalize し、config dirs を既存規約で解決する。cwd canonicalize 失敗は `InvalidRequest` → exit 2 の CLI input error として `Err` を返し、report diagnostic には変換しない。`resolve_workspace_layout`/`RouteSupply::load`/`embedded::builtin_dir()` は呼ばない。
4. builtin source は production では embedded catalog、dev/test で `CliHooks.builtin_dir` が指定された場合のみ filesystem source とする。user/project source は常に filesystem。
5. route layer report は builtin → user → project の順に作る。各 layer で `routes.yaml` を read/parse/validate し、失敗はその layer の error entry として収めて次へ進む。valid layer の route は shared merge step へ渡す。
6. merge result は全 configured route を失わない。effective route は `active`、project が負けた route は `suppressed`、user が builtin を置き換えた route は `shadowed`、同一 layer duplicate は `invalid` とする。merge-dependent status は、その row に影響し得る下位/同位 layer が non-failed（`loaded` または `absent`）の場合に限り確定し、必要 layer が `failed` の場合は `indeterminate` にする。
   - builtin route は user layer が non-failed なら `active`/`shadowed` を確定できる（**invalid bucket ではない** user route の衝突があれば `shadowed`）。user layer failed なら `indeterminate`。
   - user route は builtin/project がその優先位を変えられないため、valid な route row は常に `active`。
   - project route は loaded な builtin/user 側の **effective（非 invalid）route** に同一 `match.command` があれば `suppressed`。無ければ builtin+user が両方 non-failed のときだけ `active`、どちらか failed なら `indeterminate`。invalid bucket の下位 route は merge 対象外なので、builtin/user row を shadow せず project row も suppress しない。
   - failed layer 自身からは route 行を作らない。project layer だけ failed の場合、project が下位層を shadow しない規約により builtin/user row は確定できる。
   - 同一 layer duplicate の `invalid` は他 layer の失敗に関わらず確定する。
7. shared policy/denylist load に成功した場合のみ、resolved policy・exec view・denylist を diagnostics へ渡す。失敗時は route 行を保持し、policy-dependent surface を `unknown`/`unverified` にする。少なくとも次は policy-dependent である。
   - grant eligibility、`pin_exec_grant`、`check_exec_execution`、sample viability、denylist/exec deny。
   - filesystem workflow read の `symlink_policy`/`PathDenylist` guard。
   - `ToolCatalog::from_resolved_policy` を使う workflow `validate_definition` と worker tool capability 判定。
   - codex `route.rewrite.approvalBypass` の enabled 判定。
8. workflow validation は route の宣言 layer から target document を取得する。builtin route は embedded catalog、user/project route は filesystem read。JSON parse・shared `validate_definition`・routed convention・worker reference/schema resolution の失敗は route diagnostic に写す。policy が無い場合は policy-guarded fs read・compiler catalog を実行せず、workflow validation status を `unknown` にする。
9. worker validation は project → user → builtin precedence を維持する。filesystem candidate は既存 worker/schema resolver、embedded builtin candidate は `WorkerContentSource` 経由で `.md` と `{file:}` schema を同じ validation pipeline に渡す。worker 定義/schema 自体の validity は policy 非依存で報告してよいが、tool capability の有効性と workflow `validate_definition` は `ToolCatalog` が必要なため policy 不在時は `unknown` にする。workflow compiler が要求する `WorkerCatalog`/`resolve_worker_index` も同じ source-aware resolver を使い、builtin worker の `WorkerNotFound` は compile port ではなく route diagnostic として報告する。
10. static grant/pin 判定は `route.grant`、`grants`、`substitute.grant` の有効投影 set を対象にし、policy が利用できる場合だけ既存 predicate を使う。`PATH` 探索や executable metadata read は許可するが、spawn はしない。ambient `PATH` に依存する結果は `environment-dependent` とする。
11. sample がある場合、まず `match_command` で env-prefix strip・self-bypass・prefix match・longest/tie・suppressed attribution を評価する。`matched` は normalized argv がその row の宣言 `match.command` を prefix として満たすこと（suppressed/shadowed/invalid row の declaration command も対象）、`selected` は effective route の longest-prefix winner であることを意味する。`selected: true` と `viability` は effective row にだけ存在し、non-effective row を含む non-winner は `matched: true, selected: false` で viability を報告しない。
    - 1 つでも route layer が `failed` の場合、runtime の effective set を再現できないため全 row の `matched`/`selected` を `indeterminate`、winner `viability` と suppressed attribution を省略し、`route.sample-indeterminate` diagnostic を付ける。
    - 全 layer が non-failed で policy が無い場合、`matched`/`selected` は `match_command` で確定できるが `viability` は `unknown` とする。policy-independent な unsafe/invalid/missing-target も viability としては部分評価せず、route の static diagnostics が別途報告する。
    - 全 layer が non-failed で policy がある場合だけ shared `resolve`/`viability` を呼び、unsafe-command/invalid-route/missing-target/no-grant/stdin-unsupported/runnable の固定優先順を再利用する。
    - sample が無い場合は sample `status` を `requires-sample-input`、`matched`/`selected` を `not-applicable`、winner `viability` と suppressed attribution を省略し、`runnable` と断定しない。
12. renderer capability は `HookTarget::ALL` について `hook::targets` が所有する capability descriptor から静的に報告する。codex approval-bypass は policy がある場合だけ評価し、無い場合は `unknown`。host registration/approval/execution は `unverified` 固定とする。
13. 最後に route rows・layer diagnostics・summary を JSON result/text lines へ投影する。raw `argv`・`args`・`execArgs`・`command`・env value・secret-bearing error detail は出力しない。`message`/`remediation`/`RouteLayerReport.error` は stable code・bounded reason enum・normalized schema field label（例: `routes[0].match.command`）・declared layer-source logical path から組み立て、schema violation の free-form detail・offending key/value・OS error Display・errno・`cause` chain・`searched` host path・source snippet を serialize しない。内部評価には sample を使ってよいが、出力するのは matched/selected 状態・normalized argv token count・viability/code 等の分類値のみとする。canonicalized `cwd` も内部評価にだけ使い、report field として serialize しない。`--format text` は generic `utility_text` fallback ではなく dedicated route-check renderer（layer → route → diagnostic の deterministic lines）を使い、`--quiet` は auxiliary stderr diagnostics の抑制のみで stdout report・exit code・必須の stderr error diagnostic を変えない。
14. `route check` は diagnostics が正常に生成された限り `Ok(CommandOut)` を返し、exit 0/1/3 では `ok: true` の utility envelope `result` に report を載せる。layer/policy/workflow/worker/schema の diagnosable `CheaperError` は必ず `RouteCheckReport` の diagnostic へ fold し、`Err` へ伝播しない。`Err` に到達してよいのは CLI input error（`InvalidRequest`/`InvalidInput` → exit 2）と unexpected internal error（`InternalError` → exit 70）だけである。`route::check` boundary で input error 以外の error（`CheaperError` と他の error type の両方）が diagnostic folding を漏れた場合は、detail/cause を引き継がない `InternalError` へ wrap してから返す。input error についても `route check` Err path は既存 `error_envelope` の raw details/cause をそのまま使わず、stable reason/flag name だけを残す sanitized projection を通す。Err path でも `route check` flag により Usage/History/builtin-dir context 解決は行わない。

## Diagnostic model

- `RouteCheckReport`: `schemaVersion`, `sampleProvided`, `merge`, `layers`, `policy`, `routes`, `summary`。canonicalized `cwd` は内部入力であり serialize しない。
- `RouteLayerReport`: `layer`, `sourceKind`, `status` (`absent|loaded|failed`), `routeCount`, `invalidCount`, `error`。`error` は `{code, reason, source, field?}` の allowlist object で、`code` は raw `CheaperError` code ではなく stable `route.*` diagnostic code、`reason` は bounded enum、`field` は normalized schema label とする。raw OS/cause text・offending key/value を持たない。
- `RoutePolicyReport`: `status` (`loaded|failed`), `layers` (`builtin|user|project` それぞれ `absent|loaded|failed`), `error`（`RouteLayerReport.error` と同じ allowlist shape）。builtin absent/read/parse/schema failure と user/project failure は `policy.status: failed` かつ該当 layer diagnostic にする。post-merge validation・default resolution・denylist failure は `layers` map には追加せず、`error.layer` の pseudo-layer `merged`・source `policy:effective` として report する。
- `RouteCheckRow`: `name`, `layer`, `trust` (`trusted|untrusted`), `match`, `configuration` (`active|suppressed|shadowed|invalid|indeterminate`), `workflow`, `grants`, `substitute`, `execReadiness`, `sample`, `targets`, `diagnostics`。
- `trust` は宣言 `trusted` key や policy ではなく layer provenance から導く（builtin/user = `trusted`、project = `untrusted`）。invalid row でも layer が分かるため同じ値を報告する。
- `invalid` row は同層 duplicate group の name/command/layer/trust と group `invalidReason` のみを持つ shallow row とし、workflow/grants/substitute/execReadiness/targets は `not-applicable` にする。`suppressed`/`shadowed`/`indeterminate` row は merge provenance を per-layer parsed `Route` に join して通常宣言値を保持する。
- `RouteCheckSample`: `provided`, `status` (`requires-sample-input|evaluated|indeterminate`), `matched` (`true|false|indeterminate|not-applicable`), `selected` (`true|false|indeterminate|not-applicable`), `viability?` (`runnable|no-grant|missing-target|invalid-route|unsafe-command|stdin-unsupported|unknown`), `suppressed?` (suppressed route name)。
- `viability` は sample winner がある場合にだけ serialize し、no sample・layer failure・non-winner では省略する。serialized `unknown` は「sample winner は確定したが policy 不在で viability を評価できない」状態だけを表す。`suppressed` は selected winner の command に prefix match する suppressed route がある場合にだけ serialize し、既存 `RouteResolution.suppressed` と同じ route-name 値を載せる。
- `RouteCheckDiagnostic`: `code`, `severity` (`info|warning|error`), `layer`, `route?`, `source?`, `message`, `remediation?`。`message`/`remediation` は stable code・bounded reason enum・logical source・route/name metadata から生成する。
- stable code の namespace は `route.*`。layer read/parse/schema、route invalid/suppressed/shadowed/indeterminate、workflow missing/invalid/convention/unknown、worker missing/invalid/schema、grant missing/ineligible/pin-unresolved、policy/denylist load、sample-indeterminate、requires-sample-input、unsafe-command、stdin-unsupported、renderer-unverified を個別 code にする。

## Status and severity mapping

`status` は route/resource の事実状態、`severity` は exit class を決める diagnostic の重要度として分離する。

- `info`: absent layer、`requires-sample-input`、host `unverified`、environment-dependent、`matched` non-winner、`not-applicable`、sample-derived `unsafe-command`、static capability 情報。
- `warning`: actionable な non-blocking 状態（no-grant、suppressed、shadowed、pin unresolved、workflow missing が route を inactive にする場合、sample-derived `stdin-unsupported` 等）。
- `error`: layer read/parse/schema failure、invalid route declaration、workflow JSON/schema/convention failure、worker definition/schema failure、route が参照する worker missing、shared policy/denylist failure、内部 contract failure。
- `unknown`/`unverified`/`indeterminate` は severity を自動決めしない。固定 informational status は `info`、policy/layer failure 由来の status は親 diagnostic の severity を使う。
- sample-derived viability の severity: `runnable`/`unsafe-command` = `info`、`no-grant`/`stdin-unsupported` = `warning`、`missing-target`/`invalid-route` = `error`（同じ原因の configuration diagnostic が既にある場合は diagnostic を増やさず severity だけを使う）、`unknown` = policy failure diagnostic の severity。

## Exit status mapping

- `0`: error/warning severity が無い（`info` のみ）。
- `1`: warning severity があり、error severity が無い。
- `2`: CLI 入力自体が不正（`--command`/`--argv` の同時指定、malformed `--argv`、unknown option 等）。
- `3`: layer read/parse/schema failure、shared policy/denylist failure、workflow schema/convention failure 等の error severity が 1 件以上ある。
- `70`: 上記で分類できない internal failure。

同時に warning と error がある場合は `3`。exit code は診断結果の分類であって、route/workflow/worker/process の実行結果ではない。exit 0 は host `unverified` や no-sample `requires-sample-input` があっても到達可能である。

## Non-Functional Constraints

- `route check` は filesystem mutation・process spawn・network/auth・History/Usage/RouteLogs/credential/tracing file write を一切行わない。対象は strict parse が `RouteAction::Check` へ到達する invocation と、malformed でも `is_route_check_invocation` が true の parse-error invocation に限る。global `--help`/`--version` は既存 precedence で先に評価され、route check command contract の対象外とする。特に `route check --version`/`route check --help` は既存 Version/Help action として扱われ、route check diagnostics・no-mutation guarantee の対象ではない。
- logging suppression と Err-path gate は同じ `route check` detection を共有し、`route check` の option 順や `--` terminator に依存しない。
- read は bounded に行い、route/policy/workflow/worker/schema file は既存 loader の read 境界と schema cap を再利用する。diagnostic cache は作らない。
- ambient `PATH`・cwd により結果が変わり得る項目は `environment-dependent` と明示する。ambient `PATH` で解決した executable pin も spawn success を意味しない。
- 既存 `route run`・`route resolve`・hook evaluation・usage/history recording の observable behavior は変更しない。
- output projection は allowlist ベースにし、sample-derived raw values・env values・secret-bearing file contents/cause text を載せない。必要なのは layer/source、route 宣言 metadata、status/code、remediation、非秘密の token count に留める。`RouteResolution.executable` は sample-derived なので `RouteCheckRow.execReadiness` には写さない。
- renderer capability は `hook::targets` の per-target descriptor に単一 source を置く。descriptor と canonical rewrite render の一致は contract test で確認する。
- dependency は追加しない。

## Risks and Trade-offs

- `Route.target_path` は現行 `PathBuf` 前提なので、embedded layer では diagnostics 専用の synthetic layer root を使って `target.path` を相対へ戻す。`missing-target` 判定は `Path::is_file` ではなく `target_exists` predicate を shared viability へ注入して解決する。
- 実行時の `ResolveInput` へ predicate を足すことで、route check 専用の viability 複製を避けられるが、call site の field 追加が必要になる。既存 caller は `None`/filesystem default に留めて挙動を保持する。
- worker/schema source seam は `validate_worker_definition` と `load_schema` の入力面を広げる。既存 file-based API は wrapper として残し、embedded validation は content resolver を渡す。
- logging を完全 Off にすると `CHEAPER_LOG=debug` での内部調査は効かないが、spec の no-log guarantee を優先する。CLI stderr diagnostics と exit code で不足分は返す。
- `route check` は `route resolve` より多くの source を読むため、ambient PATH や file permission の差で結果が変わる。これを隠さず diagnostic status に反映する。

## Deferred Design Concerns

- `route resolve`/`route run` の builtin temp materialization 統一: `route check` 専用の read-only source で要件を満たせるため、既存実行経路への波及は本設計では行わない。将来全 utility を no-write にする場合に再検討する。変更を延期しないと、runtime path の compatibility リスクが増える。
- Host-side hook rewrite validation: #162 の実機検証が必要であり、`route check` の静的診断範囲外。未対応のままだと host 登録/approval 不整合を `unverified` でしか示せない。
- 既存 repository-wide document warnings: #185 の causal blocker ではないため本設計では扱わない。放置すると discovery index 等の drift が残る。

## Verification Notes

- [ ] `route check` unit/integration tests が `CHEAPER_LOG=debug`・`CHEAPER_LOG_FORMAT`・`--log debug`・`--log-format` の両方で `.cheaper/logs`・tracing subscriber・file write を生じないことを確認する。
- [ ] `--log debug route check`、`--quiet route check`、`--format json route check`、`route --log debug check`、`route -- check`、`-- route check`、unknown-option/missing-value を含む malformed `route check` invocation で suppression と Err-path gate が効くことを確認する。no-log probe は process-global subscriber の first-wins edge を避けるため fresh process で実行する。
- [ ] clean temp dir で embedded builtin `routes.yaml`/`policy.yaml`/workflow/worker/`{file:}` schema が読め、`cheaper-builtin-*` temp dir を作らないことを確認する。
- [ ] `builtin_dir: None` の production-path test で builtin route が false `missing-target`・false worker/schema failure にならず、`WorkerCatalog`/`resolve_worker_index` が embedded builtin worker を解決することを確認する。filesystem dev-seam fixture だけで判定しない。
- [ ] absent user/project routes が normal empty layer として merge determinacy を維持し、malformed layer が per-layer diagnostic、valid sibling layer の route 行が残ることを確認する。
- [ ] user-over-builtin `shadowed`、project `suppressed`、same-layer `invalid` shallow row、failed sibling layer 時の `indeterminate` を確認する。
- [ ] shared policy/denylist failure で routes が残り、`matched`/`selected` は `match_command` で確定し、grant/pin/viability/workflow guard/compiler/codex bypass status が `unknown`/`unverified` になることを確認する。
- [ ] layer failure がある sample で `matched`/`selected` が `indeterminate`・viability が省略され、`route.sample-indeterminate` が出ることを確認する。
- [ ] no sample で `runnable` が出ず、sample ありで `route resolve` と同一 viability 優先順になり、non-winner/non-effective match が `matched:true, selected:false`、policy 不在時の winner が `viability:"unknown"` になることを確認する。
- [ ] spawn/network/auth/History/Usage/RouteLogs/credential/log write の probe を全て fixture 側で否定する。
- [ ] JSON/text output が raw `argv`/`args`/`execArgs`/command/env/OS error/cause/errno/host path detail を含まないことを確認する。
- [ ] renderer capability が static で、host registration/approval/execution が `unverified` のままであること、capability descriptor と render contract が一致することを確認する。`route check --help`/`route check --version` が既存 precedence を維持し、`RouteAction::Check` の no-mutation contract 対象外であることも fixture で固定する。
- [ ] exits 0/1/3 が `ok:true` report envelope、2/70 が `ok:false` error envelope であること、diagnosable config failure が `ok:false` error envelope へ漏れて native exit 3 と混同されないことを確認する。
- [ ] `mise run check` が通ることを確認する。

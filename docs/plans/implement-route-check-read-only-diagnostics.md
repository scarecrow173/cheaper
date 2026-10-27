---
id: "PLAN-034N6OiHMEfqWbPgQDUNcy"
type: "plan"
status: "approved"
title: "Implement route check read-only diagnostics"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
  implemented-by:
    - "docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md"
    - "docs/tasks/add-embedded-route-check-source-supply.md"
    - "docs/tasks/share-route-workflow-and-worker-validation-seams.md"
    - "docs/tasks/share-route-sample-matching-and-renderer-capability.md"
    - "docs/tasks/implement-route-check-diagnostics-report.md"
    - "docs/tasks/verify-route-check-contract-and-document-usage.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
    - "docs/discovery/explore-route-check-diagnostics-issue-185.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-check-read-only-invocation-boundary.md"
    - "docs/test-specs/route-check-layer-and-policy-diagnostics.md"
    - "docs/test-specs/route-check-validation-readiness.md"
    - "docs/test-specs/route-check-sample-resolution-semantics.md"
    - "docs/test-specs/route-check-output-and-exit-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement route check read-only diagnostics Implementation Plan

## Goal

`cheaper route check` を実装し、builtin/user/project の全 route layer、policy、workflow、grant、executable readiness、renderer capability を一切の副作用なしに診断する。stable diagnostic code、remediation guidance、allowlist 済み text/JSON 出力、exit status class を提供し、`route resolve`/`route run` と同一の schema・merge・policy・viability 規約を共有する。

## Scope

1 つの implementation stream とする。CLI detection/gating、embedded read-only resource、shared validation seam、diagnostic orchestration/reporting、test、user doc は同一の read-only contract と release goal を共有するため分割しない。

境界:

- `route run`、`route resolve`、hook evaluation、通常の observability、Usage/History recording、builtin temp-cache materialization の runtime 挙動は変更しない。
- `route check` は strictly read-only: process spawn、workflow/worker 実行、network/auth、config/credential write、History/Usage/RouteLogs/tracing file write、diagnostic cache、embedded temp materialization を一切行わない。
- host registration/approval/execution は `unverified` 固定とし、#162 の real-host validation は対象外とする。

## Upstream Documents

- Approved requirements: `../specs/route-check-diagnostics-issue-185.md`
- Accepted boundary decision: `../adr/use-a-read-only-embedded-resource-boundary-for-route-check.md`

## Design Inputs

- Approved implementation boundary: `../designs/route-check-read-only-diagnostics-boundary.md`
- Source exploration and alternatives: `../discovery/explore-route-check-diagnostics-issue-185.md`

## File Map

| Files | Owner slice | Responsibility |
| --- | --- | --- |
| `crates/cheaper-cli/src/args.rs` | CLI grammar | `RouteAction::Check { cwd, command, argv }`、0/1 sample cardinality、`is_route_check_invocation` pre-parse detection |
| `crates/cheaper-cli/src/route_cmd.rs` | Sample parsing/dispatch | `parse_command`/`parse_argv` の `pub(crate)` seam 化、`RouteAction::Check` の dispatch arm |
| `crates/cheaper-cli/src/cli.rs`, `crates/cheaper-cli/src/text_format.rs` | Dispatch/gating | observability init 前の logging Off 強制、Err path での builtin-dir/user-config/Usage/History bypass、sanitized error envelope projection、`format_text` から route-check renderer への dispatch |
| `crates/cheaper-cli/src/embedded.rs` | Embedded catalog | materialization なしの read-only `EmbeddedResources` accessor（`/` 正規化 logical relative key、`read`/`contains`、synthetic root `/builtin/`） |
| `crates/cheaper-cli/src/route/routes.rs` | Layer loading/merge | shared per-layer read/validate step、merge fold の `shadowed`/`suppressed`/`invalid` provenance、layer 独立評価 entry point |
| `crates/cheaper-cli/src/route/supply.rs`, `crates/cheaper-core/src/policy/load.rs` | Source/policy supply | `RouteLayerSource`（Filesystem/Embedded）と check 専用 supply builder、`load_policy_documents` 相当の document 入力 API |
| `crates/cheaper-cli/src/workflow_context.rs`, `crates/cheaper-cli/src/workflow_run.rs`, `crates/cheaper-workflow/src/loader.rs`, `crates/cheaper-core` の worker resolve/validate/schema files | Validation seams | document 入力の shared workflow validation、`validate_routed_convention` の `pub(crate)` 化、`WorkerContentSource`/`SchemaContentSource` 相当の source-aware resolver |
| `crates/cheaper-cli/src/route/resolve.rs` | Sample evaluation seam | env-prefix strip/self-bypass/prefix match/longest-tie/suppressed attribution の `match_command` 相当 phase 分離、viability 入力への `target_exists` predicate 注入 |
| `crates/cheaper-cli/src/hook/targets.rs` | Renderer capability | `HookTarget::ALL` の静的 rewrite capability descriptor、codex approval-bypass 判定 seam |
| `crates/cheaper-cli/src/route/check.rs` (new), `crates/cheaper-cli/src/route/mod.rs` | Orchestration/report | layer/policy/route/sample diagnostic assembly、status/severity/exit mapping、allowlist JSON projection、route-check 専用 deterministic text renderer |
| `crates/cheaper-cli/tests/integration/` 配下の新規 `route_check` module と `tests/integration/main.rs` | Integration tests | read-only probe、detection、merge determinacy、sample/viability、output sanitize、exit class fixture |
| `docs/guides/parent-agent-integration.md`, `docs/guides/configuration.md` | User docs | usage、exit class、read-only 保証、`unverified`/`requires-sample-input` の意味、制約 |

File ownership は task ごとに排他とする。`cli.rs` は dispatch/gating slice が所有し、`route/resolve.rs`・`route/routes.rs`・policy/worker seam の既存 caller は filesystem default のまま維持する。

## Implementation Sequence

1. `args.rs` に `RouteAction::Check { cwd, command, argv }` を追加する。`--command`/`--argv` は XOR で 0 または 1 個とし、`resolve` arm の cardinality check には巻き込まない。`route_cmd.rs` の `parse_command`（shell tokenization + `unsafe_command` flag）と `parse_argv`（JSON string array）を `pub(crate)` seam へ昇格する。`is_route_check_invocation` を STRING_OPTIONS/BOOL_OPTIONS arity table と `--` terminator 規約を共有する pre-parse helper として実装し、reordered global、malformed flag、余分な positional を fail-safe に検出する。`route checks`/`route CHECK` は検出せず、global `--help`/`--version` の既存 precedence は維持する。
2. `cli.rs` で `run_cli` が parse 前に `is_route_check_invocation` を評価し、true なら `flag_level`/`flag_format`/`env_level`/`env_format` 全て `None`・stderr sink 無効・`log_root` に `PathBuf::new()` placeholder を持つ `Activation` を作り、sink 構築前に return させる。dispatch Err arm では同じ flag で `resolved_user_config_dir`/`record_usage*`/`finalize_worker_history`/builtin dir 解決を bypass し、stable code・generic message・bounded reason のみを持つ sanitized error envelope を返す。
3. `embedded.rs` に `EmbeddedResources` accessor を追加する。synthetic root `/builtin/` を唯一の builtin logical root とし、`/` 正規化 relative key で `routes.yaml`/`policy.yaml`/`workflows/**`/`workers/**`/`workers/schemas/**` の byte/text document を返す。`builtin_dir()`/`materialize_builtin_dir()` は呼ばない。
4. `route/routes.rs` の `read_routes_file`/`validate_routes_doc` と merge fold を、filesystem `load` と diagnostics の両方が使う shared step へ整理する。merge result は `shadowed` provenance を返し、same-layer duplicate を `invalid`、project 敗者を `suppressed`、user-over-builtin replacement を `shadowed` として記録する。diagnostics entry point は builtin→user→project の各 layer を独立評価し、failed layer は自身の error entry のみ生成して valid sibling row を保持する。`invalid` row は name/command/layer/trust/`invalidReason` のみの shallow row とする。
5. `route/supply.rs` に `RouteLayerSource`（`Filesystem { routes_file, workflow_root, worker_base_dir }` / `Embedded { catalog }`）と check 専用 supply builder を追加する。production builtin は Embedded、dev/test で `CliHooks.builtin_dir` 指定時のみ Filesystem、user/project は常に Filesystem とし、`worker_base_dir` は layer base dir（resolver が `workers/<name>.md` を join）とする。`cheaper-core::policy::load` に document 入力 API（`load_policy_documents` 相当）を追加し、builtin/user/policy document を同一 validation/merge/normalize pipeline に通す。実行経路の `RouteSupply::load` は変更しない。
6. `cheaper-workflow/src/loader.rs` と `workflow_context.rs` を file-read wrapper と document-based shared validation に分離し、`workflow_run.rs` の `validate_routed_convention` を `pub(crate)` にする。worker resolve/validate/schema に `WorkerContentSource`/`SchemaContentSource` 相当の seam を追加し、`resolve_worker`/`resolve_worker_index`/`WorkerCatalog` が project → user → builtin precedence を同じ source-aware resolver で維持するようにする。`source_path` は display 用 logical path とし、schema base-dir resolution は `\\` → `/` normalize と `/builtin/` prefix strip を経て relative key で catalog lookup する。
7. `route/resolve.rs` から env-prefix strip・self-bypass・prefix match・longest/tie・suppressed attribution を `match_command` 相当の shared phase へ分離し、viability 入力へ `target_exists` predicate を追加する。既定は `Path::is_file`、route check は layer source 解決 predicate を渡す。viability 優先順 unsafe-command → invalid-route → missing-target → no-grant → stdin-unsupported → runnable と `missing-target` の位置は変更しない。
8. `hook/targets.rs` に `HookTarget::ALL` の静的 rewrite capability descriptor を追加し、codex の policy-dependent approval-bypass と静的 rewrite support を分離する。descriptor は render contract と同じ owner に置き、host registration/approval/execution は常に `unverified` とする。
9. `route/check.rs`（新規）を `route/mod.rs`/`route_cmd.rs` に登録し、orchestration・`RouteCheckReport`（`schemaVersion`/`sampleProvided`/`merge`/`layers`/`policy`/`routes`/`summary`）・`RouteCheckSample`・`route.*` diagnostic code・status/severity/exit mapping・allowlist projection・route-check 専用 deterministic text renderer を実装する。`RoutePolicyReport.layers` は builtin/user/project ごとの `absent|loaded|failed` を保持し、per-layer の read/parse/schema 失敗はその layer diagnostic にする。post-merge validation/default-resolution/denylist 失敗のみ pseudo-layer `merged`・source `policy:effective` で報告する。PATH 依存の pin/readiness 結果は `environment-dependent` と表示する。worker 定義/schema 妥当性は policy 非依存に評価し、policy が無い場合は tool capability/`validate_definition`/codex approval-bypass のみ `unknown` にする。`--quiet` は auxiliary stderr diagnostics の抑制のみで、stdout report・exit code・必須 stderr error diagnostic は変えない。diagnosable `CheaperError` は report diagnostic に fold し、exit 2 は argument parse・`--command`/`--argv` parse・cwd canonicalize など input stage で構築された error のみに限定する（`ErrorCode` ではなく construction site で判定）。shared loader 由来の `InvalidRequest`/`InvalidInput` を含む他の escape は全て detail/cause を引き継がない `InternalError` wrap（exit 70）にする。canonicalized `cwd` と raw sample 値は内部評価のみに使い serialize しない。`text_format.rs` は `RouteCheckReport` envelope を検出して `route::check` の専用 renderer へ dispatch し、generic `utility_text` fallback は使わない。
10. `route_check` integration module を `tests/integration/main.rs` に登録し、read-only probe・detection・merge determinacy・sample/viability・sanitize・exit class fixture に加えて、builtin `cargo-test` の no-grant/grant-enabled parity・missing/invalid workflow・unresolved/`environment-dependent` executable readiness fixture を揃える。`cli.rs` の `HELP` に `route check` を追加し、`docs/guides/parent-agent-integration.md` と `docs/guides/configuration.md` に usage・exit class・制限を記載し、canonical `mise run check` を実行する。

## Dependency Graph

- Step 1 → Step 2: detection helper が無いと logging suppression と Err-path gate を配線できない。
- Step 3 → Step 5: check 専用 supply builder が embedded catalog を消費する。
- Step 5 → Step 6: worker/schema source seam が `RouteLayerSource` 規約を共有する。
- Step 4、Step 7、Step 8 は Step 1-3 完了後に互いに独立して進められる（Step 6 は Step 5 待ち）。
- Step 9 は Step 1-8 全てに依存する。
- Step 10 は Step 9 に依存する。

## Dependencies and Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| データ損失: route check path が誤って file write し、user config/History/credential/workspace を破壊する | Low | High | detection と Err-path gate を単一 source にする、ambient logging 有効を含む write probe fixture、designated error は全て diagnostic fold、`Err` は `InternalError` wrap のみ |
| 破壊的変更: shared seam（resolve/routes/policy/worker/workflow）の refactor が `route resolve`/`route run`/hook/Usage/History の挙動を変える | Medium | High | 既存 caller は `Path::is_file`/filesystem default のまま維持、既存 API を wrapper として残す、`route resolve` parity test と既存 integration suite で固定 |
| diagnostics 専用 predicate の複製による route/policy/workflow 判定の drift | Medium | Medium | seam 注入のみ許可、fail-open `load_or_none` 禁止、capability descriptor は render contract owner と同一、contract test で一致を確認 |
| raw argv/command/env・cwd・host path・OS error・cause・secret の output/log 漏出 | Medium | High | allowlist projection、stable code + bounded reason + normalized field label + logical source のみ、sanitized envelope、private fixture audit |
| ambient logging 下での suppression 漏れ（`.cheaper/logs` 生成） | Medium | High | STRING_OPTIONS/BOOL_OPTIONS arity と `--` 規約を strict parse と共有、malformed でも fail-safe 検出、fresh-process fixture で subscriber first-wins edge を回避 |
| embedded production path（`builtin_dir: None`）の未検証による false `missing-target`/worker failure | Medium | Medium | filesystem dev-seam fixture だけで判定せず、`builtin_dir: None` production-path test を必須化 |
| 外部依存: 新規 package/version の追加が必要になる | Low | Low | 追加しない。既存 `include_dir` embedded resource を read-only source として再利用 |
| panic boundary が `run_cli` Err arm を bypass し、raw panic payload/OS error を generic envelope/stderr に出す残余経路が残る | Low | Medium | route-check code は user input/host path を panic payload に含めない設計とし、既存 product-wide panic sanitization は本計画の範囲外として Review Handoff に残す |

## Task Breakdown

plan 承認後、owner slice ごとに task document を作成する: (a) CLI grammar/detection/gating、(b) embedded catalog と `RouteLayerSource`/policy document supply、(c) shared validation seam（routes merge、workflow、worker/schema）、(d) resolve seam と renderer capability、(e) orchestration/report/renderer、(f) integration test と user doc。各 task は対象 fixture を owner code 変更前に追加し、既存 public boundary に対して compile 可能な場合は expected-red evidence を残す。各 slice で named focused command を実行し、canonical `mise run check` は integration 完了時に 1 度だけ実行する。

## Verification

| Slice | Sequence step | Command or check | Pass condition |
| --- | --- | --- | --- |
| Invocation detection/grammar | 1 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam args::` と `route_check` detection fixture | `cheaper --log debug route check`、`--quiet`、`--format json`、`route --log debug check`、`route -- check`、`-- route check`、unknown-option/missing-value を含む malformed invocation を全て検出し、`route checks`/`route CHECK` は非検出、`route check --help`/`--version` は既存 precedence を維持。`--command`/`--argv` 同時指定と 2 個以上の sample が parse error になる |
| Logging/Err-path gating | 2 | `CHEAPER_LOG=debug`/`CHEAPER_LOG_FORMAT`/`--log debug`/`--log-format` 下の fresh-process `route_check` probe | `.cheaper/logs`・tracing subscriber・file sink を一切作らず、Err path で `resolved_user_config_dir`/Usage/History/builtin-dir 解決へ進まない。exit 2/70 が `ok:false` sanitized envelope を返す |
| Embedded catalog | 3 | clean-temp `route_check` fixture + `embedded::` unit test | pre-existing builtin cache 無しで builtin `routes.yaml`/`policy.yaml`/workflow/worker/`{file:}` schema が読め、`cheaper-builtin-*` temp dir を作らない。key が `/` 正規化 logical relative path になる |
| Layer loading/merge | 4 | `route_check` layer fixture + `route::routes` unit test | absent user/project が normal empty layer、malformed が per-layer diagnostic、valid sibling row が残る。`shadowed`/`suppressed`/`invalid` shallow row/failed sibling 時の `indeterminate` が design の merge determinacy 規約どおり |
| Source/policy supply | 5 | `route_check` policy fixture + `policy::load` unit test | shared policy/denylist 失敗で route 行が残り、`RoutePolicyReport.layers` が builtin/user/project の `absent|loaded|failed` を保持する。per-layer 失敗は layer diagnostic、post-merge 失敗のみ pseudo-layer `merged`/`policy:effective`。grant/pin/viability/workflow guard/compiler/codex bypass は `unknown`/`unverified`、check supply が `embedded::builtin_dir()` を呼ばない |
| Workflow/worker/schema seams | 6 | `builtin_dir: None` production-path test + workflow/worker validation fixture | filesystem dev-seam だけでなく production embedded path で builtin worker/schema が解決され、false `missing-target`/false worker/schema failure が無い。`WorkerCatalog`/`resolve_worker_index` が embedded builtin worker を返し、`WorkerNotFound` が route diagnostic になる。policy 不在でも worker 定義/schema 妥当性は評価され、policy 依存の tool capability/`validate_definition`/codex bypass のみ `unknown` |
| Sample evaluation seam | 7 | `route_check` sample fixture + `route::resolve` parity test | no-sample は `requires-sample-input` で `runnable` にならない。sample ありは `route resolve` と同一 viability 優先順、non-winner は `matched:true, selected:false` で viability 無し、policy 不在 winner は `viability:"unknown"`、layer failure では `matched`/`selected` が `indeterminate` で `route.sample-indeterminate` が出る |
| Renderer capability | 8 | `hook::targets` contract test | capability descriptor と canonical rewrite render が一致し、host registration/approval/execution が `unverified`、policy 不在の codex approval-bypass が `unknown` |
| Orchestration/report/output | 9 | `route_check` output/exit fixture | JSON/text に raw `argv`/`args`/`execArgs`/command/env value/cwd/host path/OS error/cause/errno/`searched` path/source snippet/schema-violation free-form value/secret を含まない。exit 0/1/3 が `ok:true` report envelope、input-stage error のみ 2、その他の escape は 70 で `ok:false` error envelope、diagnosable config failure が `Err` へ漏れない。`--quiet` は auxiliary stderr のみ抑制し、stdout/exit/必須 error diagnostic を保持する。PATH 依存結果が `environment-dependent` になる。`--format text` が generic `utility_text` ではなく route-check renderer を使う |
| Read-only probes | 2-9 横断 | `route_check` probe fixture（ambient logging 有効を含む、cross-platform） | spawn/network/auth/History/Usage/RouteLogs/credential/log file write/embedded temp materialization が全て否定される |
| Docs and registry | 10 | `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` | blocking document/index/relations finding が無い。CLI `HELP`/route usage が `check` を含み、両 guide に usage・exit class・read-only 制限が記載される。builtin `cargo-test` no-grant/grant-enabled parity、missing/invalid workflow、unresolved/`environment-dependent` executable readiness fixture が存在する |
| Full canonical | 10 | `mise run check` | exit code 0 |

focused test target が repository の integration layout と異なる場合は、module filter を維持したまま test file owner が指名する実 target を使い、target の disable/skip はしない。

## Rollback Strategy

- Trigger: shared seam refactor が `route resolve`/`route run`/hook/Usage/History の observable behavior を変えた場合、suppression 失敗で何らかの file write が発生した場合、`route resolve` との viability parity が崩れた場合に rollback する。
- feature branch の #185 実装 commit のみを revert する。`route check` は read-only で永続 data を生成しないため、user data の cleanup 対象は存在しない。
- `git status --short`、既存 route/hook/observability の integration test、`mise run check` で rollback を検証する。shared seam を残す場合は filesystem default path が旧挙動と同一であることを parity test で確認してから残す。

## Open Questions / Gaps

- blocking な gap は無い。`EmbeddedResources` accessor 名、`load_policy_documents`・`match_command`・`WorkerContentSource`/`SchemaContentSource` の具体的 signature・variant 名、`route.*` stable code の個別名は、implementation task が approved design の vocabulary と severity/exit mapping の範囲内で決定する。free-form reason text の追加や cap・優先順の変更は許可しない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated/subagent-capable implementation。ただし task DAG validation 後に、shared seam が安定してから独立 slice を dispatch する。
- Independent work streams: Step 3/5/6（embedded catalog・source supply・workflow/worker seam）、Step 7（resolve seam）、Step 8（renderer capability）は Step 1-2 の detection/gating と並行して進められる。Step 9 の orchestration は全 seam 完了後、Step 10 はその後に実施する。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Assumptions: 新規 dependency は追加しない。既存 caller は全て filesystem/`Path::is_file` default を維持する。`cheaper-runtime/dev-seam`/`cheaper-cli/dev-seam` feature が test で利用可能。exit 0/1/3 の `ok:true` report envelope と 2/70 の `ok:false` error envelope は既存 utility envelope 規約を再利用する。既存 product-wide panic boundary は `run_cli` Err arm を bypass する residual channel であり、route-check code は user input/host path を panic payload に含めない（panic sanitization 自体は本計画外）。
- Review focus: diagnostic 専用 predicate の複製が無いこと、logging suppression と Err-path gate が同一の `is_route_check_invocation` を使うこと、merge determinacy が design 規約どおりであること、output projection が allowlist のみで raw value/host path/cause を含まないこと、no-mutation contract が ambient logging と malformed invocation の両方で成立すること。
- Approval evidence (2026-10-27): Reviewer A/B、Validator、Judge による review-protocol 完了。Judge verdict は `ACCEPT WITH NON-BLOCKING NOTES`、確認済み blocker は 0。

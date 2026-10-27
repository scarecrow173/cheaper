---
id: "TASK-034N7DbHZTQLhRywkbRruo"
type: "task"
status: "done"
title: "Implement route check diagnostics report"
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
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
    - "docs/tasks/share-route-workflow-and-worker-validation-seams.md"
    - "docs/tasks/share-route-sample-matching-and-renderer-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-check-layer-and-policy-diagnostics.md"
    - "docs/test-specs/route-check-validation-readiness.md"
    - "docs/test-specs/route-check-sample-resolution-semantics.md"
    - "docs/test-specs/route-check-output-and-exit-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement route check diagnostics report

## Work

approved plan の step 9 に対応する implementation slice。`route::check` の orchestration、`RouteCheckReport`、`route.*` diagnostic、severity/exit mapping、allowlist projection、route-check 専用 text renderer を実装する。

- `route/check.rs`（新規）を `route/mod.rs`/`route_cmd.rs` に登録し、layer/policy/route/sample diagnostic を orchestrate する。
- `RouteCheckReport`（`schemaVersion`/`sampleProvided`/`merge`/`layers`/`policy`/`routes`/`summary`）と `RouteCheckSample` を実装する。canonicalized `cwd` と raw sample 値は内部評価のみに使い serialize しない。
- `RoutePolicyReport.layers` は builtin/user/project ごとの `absent|loaded|failed` を保持する。per-layer の read/parse/schema 失敗はその layer diagnostic、post-merge validation/default-resolution/denylist 失敗のみ pseudo-layer `merged`・source `policy:effective` で報告する。
- diagnostic code は stable `route.*` namespace に限定し、error object は stable code・bounded reason enum・normalized schema field label（例: `routes[0].match.command`）・`<layer>:<relative>` logical source のみを持つ allowlist object とする。
- readiness 規約: PATH 依存の pin/readiness 結果は `environment-dependent` と表示し spawn しない。worker 定義/schema validity は policy 非依存に評価し、policy 不在時は tool capability・`validate_definition`・codex approval-bypass のみ `unknown`、host registration/approval/execution は常に `unverified`。grant/pin 判定は既存 predicate を使い diagnostics 専用 predicate を複製しない。
- sample 規約: no-sample は `requires-sample-input`・`matched`/`selected` は `not-applicable` で `runnable` にしない。layer failed 時は全 row の `matched`/`selected` を `indeterminate`・`route.sample-indeterminate` diagnostic を付す。policy 不在では winner の `viability` を `unknown`。non-winner は `matched: true, selected: false` で viability を報告しない。suppressed attribution は `RouteResolution.suppressed` と同じ route-name 値のみ serialize する。
- exit class: 0 = info のみ、1 = warning あり・error なし、2 = input-stage error（argument parse・`--command`/`--argv` parse・cwd canonicalize 失敗のみ、construction site で判定）、3 = error severity diagnostic 1 件以上、70 = 分類不能な internal failure。exit 0/1/3 は `ok:true` report envelope、2/70 は `ok:false` sanitized error envelope。
- diagnosable `CheaperError` は report diagnostic に fold し `Err` へ伝播しない。input-stage error 以外の escape（shared loader 由来の `InvalidRequest`/`InvalidInput` を含む）は detail/cause を引き継がない `InternalError` wrap（exit 70）とする。
- `text_format.rs` は `RouteCheckReport` envelope を検出して `route::check` の route-check 専用 deterministic renderer（layer → route → diagnostic の確定的行）へ dispatch し、generic `utility_text` fallback は使わない。`--quiet` は auxiliary stderr diagnostics の抑制のみで、stdout report・exit code・必須 stderr error diagnostic は変えない。
- 本 slice は strictly read-only: spawn・network/auth・config/credential/History/Usage/RouteLogs/tracing write・embedded temp materialization を一切行わない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- `docs/tasks/share-route-workflow-and-worker-validation-seams.md`
- `docs/tasks/share-route-sample-matching-and-renderer-capability.md`

## Affected Files

- `crates/cheaper-cli/src/route/check.rs` （新規） — orchestration・`RouteCheckReport`・diagnostic・exit mapping・renderer
- `crates/cheaper-cli/src/route/mod.rs` — module 登録
- `crates/cheaper-cli/src/route_cmd.rs` — `RouteAction::Check` dispatch arm
- `crates/cheaper-cli/src/text_format.rs` — `RouteCheckReport` 検出と専用 renderer dispatch
- `crates/cheaper-cli/src/cli.rs` — envelope seam（必要最小限のみ）

## Done When

- [x] `RouteCheckReport` が `schemaVersion`/`sampleProvided`/`merge`/`layers`/`policy`/`routes`/`summary` を持ち、canonicalized `cwd`・raw `argv`/`args`/`execArgs`/command/env 値・host path・OS error・errno・cause・`searched` path・source snippet・schema-violation free-form detail・secret を一切 serialize しない。
- [x] `RoutePolicyReport.layers` が per-layer `absent|loaded|failed` を保持し、post-merge 失敗のみ pseudo-layer `merged`/`policy:effective` で報告される。
- [x] sample 規約（`requires-sample-input`/`indeterminate`/`unknown`/non-winner viability 省略/suppressed attribution）と PATH 依存 readiness の `environment-dependent`、policy 非依存 worker/schema validity、policy 不在時の `unknown`/`unverified` が design 規約どおりに報告される。
- [x] exit 0/1/3 が `ok:true` report envelope、input-stage error のみ 2、その他の escape が detail/cause なし `InternalError` wrap で 70 となり、diagnosable failure が `Err` へ漏れない。
- [x] `--format text` が dedicated route-check renderer を使い generic `utility_text` fallback を経由せず、`--quiet` が auxiliary stderr のみ抑制して stdout/exit/必須 error diagnostic を維持する。

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::check` — report shape・diagnostic fold・exit mapping・sanitized projection の unit coverage が pass する。
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — output sanitize・exit class・`--quiet`・dedicated text renderer・`environment-dependent` 表示の fixture が pass する。

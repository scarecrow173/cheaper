---
id: "IMPL-034MLMZ6JF1p7PeYMeOIDd"
type: "impl"
status: "completed"
title: "Close execution history integration"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/192"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/benches/history_hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_history.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_history.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/history/schema.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/history/store.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/logs.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/history.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/engine.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/engine.rs"
      - type: "file"
        path: "docs/designs/execution-history-issue-192.md"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/execution-history-integration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-execution-history-for-issue-192.md"
    - "docs/tasks/execution-history-core.md"
    - "docs/tasks/execution-history-query-cli.md"
    - "docs/tasks/execution-history-worker-usage.md"
    - "docs/tasks/execution-history-carrier-catalog.md"
    - "docs/tasks/execution-history-route.md"
    - "docs/tasks/workflow-job-history.md"
    - "docs/tasks/hook-history-evidence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md"
    - "docs/designs/execution-history-issue-192.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/usage-analysis-scope-issue-184.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Close execution history integration

## Summary

Issue #192 の全 owner slice を統合し、per-execution History、既存 Usage、Workflow Job、Route/refetch、Hook decision、correlation carrier、catalog identity projection、および利用者向けガイドの cross-surface 契約を閉じた。History の失敗は記録対象の product result や既存 #184 集計を変えず、Usage の `usageRecordRef` は append 成功確認後だけを正本とする。

## Context

本 IR は `docs/tasks/execution-history-integration.md` を実装した記録であり、Issue [#192](https://github.com/scarecrow173/cheaper/issues/192)、`docs/specs/execution-history-issue-192.md`、`docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md`、`docs/designs/execution-history-issue-192.md`、および `docs/plans/implement-execution-history-for-issue-192.md` に従う。#184 の Usage 集計互換性は `docs/specs/usage-analysis-scope-issue-184.md` を境界として維持した。

## Implementation

### Changes

- Standalone Worker は resolution 前に started を発行し、`PartialRunMeta` を成功・usage-bearing failure の terminal 投影へ引き継いだ。既存 Usage append が成功した場合だけ同じ execution ID の `usageRecordRef` を terminal に ack し、append 失敗や pre-usage failure では参照を作らない。
- Workflow は config/workspace 解決後、grant/policy/workflow loading と compilation より前の boundary で開始し、各 Job は独立 identity と Workflow parent を持つ。resolved provider/model は Job に保持し、output-schema failure でも解決済み identity を失わない。Job Worker は standalone `kind:"run"` Usage 行を新設しない。
- Route は runnable run と refetch を別 execution として terminalize し、signed `exitCode` を History に保つ。no-match/no-grant/not-runnable は decision-only とし、read-only の `route resolve` は History/Usage を書かない。
- Store は sweep lease と reservation-credit を使って count/byte capacity を best-effort に予約し、credit 不足・lease contention・sweep failure では History write だけを skip する。遅い terminal が削除済み execution directory を再生成することはなく、既存の product result と Usage の独立性を保つ。
- Hook decision evidence、explicit trace/parent carrier、History-only provider/model projection を既存の CLI/runtime/tool 境界へ配線した。受信 carrier は unverified hint のままとし、raw input、endpoint、prompt、argv、environment、credential、error body は保存しない。
- `docs/guides/usage.md` に `history list|show|trace`、privacy projection、retention、query/write の best-effort semantics を反映した。
- 依存追加はない。既存 workspace の `uuid` pin を `uuid.workspace = true` で利用し、新 package/version、APM dependency、runtime dependency は導入していない。

### Decisions During Implementation

- History/Usage の lifecycle と cardinality は分離し、History write failure は command outcome に伝播させない。Usage の correlation は `traceId` や parent traversal ではなく、同一 owner が発行した execution ID の成功 ack に限定した。
- Store の reservation-credit は hard global quota ではなく、同時 writer が一時的に超過し得る bounded best-effort とした。capacity を証明できない writer は publication を行わず、partial event を残さない。

## Related Experiments

### Adopted

なし。承認済みの plan/design に実装境界が定義されており、独立した Experiment Log は作成していない。

### Rejected

なし。

## Validation

### Final evidence

- Windows で `mise run check` は exit 0。Rust pipeline の全テスト、format、clippy、lockfile、payload および document checks が pass した。
- CLI integration は 368 passed / 3 ignored、Workflow は 19/19、History Store は 17/17、Route focused は 10 passed / 3 ignored。
- Hook benchmark は local storage 上の 1,000 decisions で p50 added latency 1.263 ms、p95 1.559 ms。`cargo fmt --check` と `cargo clippy` も pass した。
- 独立 final Judge は `ACCEPT`。Ubuntu workflow_dispatch run 40385078264 は expected head `dc12a5153ed68a3a0166696b575a83fbeb5f7097` で setup と `mise run check` が成功した。

### Corrections during verification

- 最初の canonical run は、テストが fake timestamp と wall-clock `recordedAt` を混在させたため失敗した。これは test-only の clock 修正で解消し、production behavior を変更せずに最終 `mise run check` を exit 0 まで再実行した。
- 最初の concurrent Store assertion は、best-effort admission なのに全 writer の publication を期待していた。assertion を、unique execution IDs、skip された write に partial event がないこと、および少なくとも一つの writer が admitted であることへ改め、reservation-credit semantics を受け入れる最終 Store 17/17 を得た。

### Worktree paths observed after `git status`

以下は final integration 時点の `git status --short --untracked-files=all` で実際に変更または追加として観測できた Issue #192 の実装・検証・guide paths だけを列挙したもの。既存の task/index/IR の編集はこの IR の ownership には含めず、IR README row は別途 managed index に反映した。

- `crates/cheaper-cli/Cargo.toml`
- `crates/cheaper-cli/src/cli.rs`
- `crates/cheaper-cli/src/history/schema.rs`
- `crates/cheaper-cli/src/history/store.rs`
- `crates/cheaper-cli/src/hook/eval_cache.rs`
- `crates/cheaper-cli/src/hook/routing_policy.rs`
- `crates/cheaper-cli/src/hook/targets.rs`
- `crates/cheaper-cli/src/route/logs.rs`
- `crates/cheaper-cli/src/route/run.rs`
- `crates/cheaper-cli/src/run.rs`
- `crates/cheaper-cli/src/usage_log.rs`
- `crates/cheaper-cli/src/workflow_run.rs`
- `crates/cheaper-cli/tests/integration/history.rs`
- `crates/cheaper-cli/tests/integration/hook_history.rs`
- `crates/cheaper-cli/tests/integration/main.rs`
- `crates/cheaper-cli/tests/integration/route_history.rs`
- `crates/cheaper-cli/tests/integration/usage_log.rs`
- `crates/cheaper-cli/tests/workflow.rs`
- `crates/cheaper-cli/benches/history_hook.rs`
- `crates/cheaper-rig-adapter/tests/adapter.rs`
- `crates/cheaper-workflow/src/engine.rs`
- `crates/cheaper-workflow/tests/engine.rs`
- `docs/designs/execution-history-issue-192.md`
- `docs/guides/usage.md`

## Risks

Canonical verification passed on Windows and Ubuntu CI. History retention and persistence remain intentionally best-effort; a missing History record or Usage reference must not be interpreted as a changed product outcome.

## Follow-ups

Coordinator synchronized task/index status and recorded the Ubuntu CI result. No additional dependency or code change is opened by this record.

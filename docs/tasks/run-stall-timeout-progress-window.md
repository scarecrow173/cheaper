---
id: "TASK-lLkjLAEBHJc2S02t4gVnRW"
type: "task"
status: "done"
title: "run 経路の timeout を進捗リセット型 stall window へ変更"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/111"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  blocks:
    - "docs/tasks/issue-111-docs-and-final-verification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "docs/adr/execution-timeout-as-progress-reset-stall-window.md"
  defers: []
  deferred-by: []
---

# run 経路の timeout を進捗リセット型 stall window へ変更

## Work

1. `crates/cheaper-runtime/src/run/boundary.rs` — 単発
   `tokio::time::sleep(timeout_ms)` を deadline loop へ変更:
   `last_progress + timeout` で起床し、進捗があれば次の期限へ延長、
   無ければ `TIMEOUT` abort。進捗通知は既存 `on_event` funnel を共有し、
   repair 経路の event も同じ通知点を経由させる
2. 進捗とみなす event: text/reasoning delta、tool call/result、
   `CompletionCall`、`ModelTurnRetried`、`Finished` など観測される
   `RunEvent` 全般
3. `cheaper workflow run` 側（`engine.rs` / `spawn_deadline`）は従来どおり
   run 全体の absolute deadline のまま — 変更しない
4. run 経路 fixture test を追加:
   - T3: 500ms の stall window 内に 50ms 間隔で `emit`/`tool_calls` が
     続く run が完走（wide margin で CI flake 回避）
   - T4: event が timeout を超えて途絶えた run が `TIMEOUT` で終了
5. 既知の経路非対称に注意: setup boundary 中の timeout は
   `setup_abort_error` 経由で `details` を落とす既存契約のため、
   回帰 fixture では `errorDetails` を pin しない（PR #109 で確定）

## Done criteria

- T3/T4 が dev-seam fixture test として pass
- workflow 経路の timeout 意味論が不変（既存 test が pass）
- `cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test run_dev_seam` pass

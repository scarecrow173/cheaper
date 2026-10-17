---
id: "TASK-rWWDt7x2fqKommosSX8tnb"
type: "task"
status: "done"
title: "budget 枯渇時の checkpoint 保存と envelope 最外層 details 付与"
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
    - "docs/tasks/nullable-maxturns-policy-model-and-propagation.md"
  blocks:
    - "docs/tasks/run-resume-cli.md"
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
    - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
  defers: []
  deferred-by: []
---

# budget 枯渇時の checkpoint 保存と envelope 最外層 details 付与

## Work

1. run 経路で `MaxTurns` / `MaxToolCalls` の budget abort 確定時に
   checkpoint を `<userConfigDir>/checkpoints/<id>.json` へ atomic write
   （temp + rename）。`TIMEOUT`・signal abort・workflow job は対象外。
   ID は `fs_util::temp_path` 型の pid+nanos 生成（UUID 依存は追加しない）。
   `checkpoints/` 親 dir は `create_dir_all` で明示作成し unix mode `0o600`
2. 観測収集: `RunEvent::ToolCommitted`（hook patch 後の args）と
   `ToolResult` を `call_id` で join し、`ModelTurnRetried` で rollback された
   call と admission-denial error result を除外。main session のみ
   （repair は tool 無し）。各 entry ≤2000 文字、合計 ≤100KB で打ち切り
   truncation marker を付す
3. error details: 最外層 error の `details.checkpoint = {path,
   observationsCount}` を付与 — `PARTIAL_MUTATION` wrap 時は wrapper 側
   （envelope 直列化が inner cause details を落とすため）。`reason`・
   `changes` 等の既存 key を保持し、observations 配列は envelope に載せない
4. 保存失敗は best-effort: abort 分類・exit code を変えず
   `details.checkpoint` を省略（stderr warning も出さない）
5. `crates/cheaper-cli/tests/regression/normalize.rs` に checkpoint path の
   run-id 部分を scrub する rule を追加し、`run-max-turns` /
   `run-max-tool-calls` の expected fixture を `details.checkpoint` 込みで更新
6. T6: budget abort で checkpoint file が書かれ envelope 最外層に
   `details.checkpoint` が出る dev-seam/CLI test

## Done criteria

- T6 pass（checkpoint 内容 schemaVersion=1・worker・input・abortReason・
  counters・observations の型検証含む）
- 回帰 fixture が新 `details.checkpoint` を反映し CI で再現可能
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test regression` pass

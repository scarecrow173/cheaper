---
id: "TASK-nOwbfXxas6V704CAtb76b7"
type: "task"
status: "done"
title: "cheaper run --resume <path> による checkpoint 再開"
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
    - "docs/tasks/budget-exhaustion-checkpoint-persistence.md"
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
    - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
  defers: []
  deferred-by: []
---

# cheaper run --resume <path> による checkpoint 再開

## Work

1. `crates/cheaper-cli/src/args.rs` — run/alias 系コマンドに
   `--resume <path>` を受理。run 以外のコマンドでは明示的に拒否
   （黙受しない）
2. `crates/cheaper-cli/src/run.rs` — checkpoint を load:
   `schemaVersion == 1` と基本型を検証し、不正は `INVALID_INPUT`。
   worker 一致検証は通常 run では positional worker 名、
   `--worker-file` では resolved `def.name` と比較
3. input 優先順位: 明示 `--input`（`--input -` の stdin 指定を含む）が
   checkpoint input に優先。明示 `--input` 非指定時は checkpoint input を
   使い stdin を読まない。`option_tokens` で指定有無を検知
4. resume prompt: 継続節（abortReason・turns/toolCalls 等の counter・
   observations）を user input へ追記し「続きから最終 JSON を返せ」と指示。
   新しい session/policy/budget を組み立て、観測のみ注入
5. `--grant` は checkpoint に永続化しない — resume 時に必要なら再指定。
   chained resume で旧 + 新の観測が合計 byte cap 内に累積されることを確認
6. T7: resume が注入観測付きで完走、明示 `--input` が優先。
   T8: chained resume の観測累積。T9: 非 run コマンド拒否・worker 不一致・
   不正 checkpoint の `INVALID_INPUT`・input 有無の検知

## Done criteria

- T7/T8/T9 が CLI integration/dev-seam test で pass
- `--input` を変えた resume で観測が古い task 由来になり得ることが
  文書化されている（docs task で反映）
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration` pass

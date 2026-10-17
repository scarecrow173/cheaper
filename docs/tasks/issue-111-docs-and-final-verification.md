---
id: "TASK-6uOJTepTvP210AgeeaBtkq"
type: "task"
status: "done"
title: "issue-111 文書追随・回帰 fixture・最終検証"
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
    - "docs/tasks/run-stall-timeout-progress-window.md"
    - "docs/tasks/budget-exhaustion-checkpoint-persistence.md"
    - "docs/tasks/run-resume-cli.md"
  blocks: []
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
  defers: []
  deferred-by: []
---

# issue-111 文書追随・回帰 fixture・最終検証

## Work

1. `docs/guides/configuration.md` — `maxTurns` の nullable/無制限既定、
   run 経路の stall timeout 意味、run vs workflow の timeout 意味差、
   `retry.transientAttempts` の対象範囲（provider transient 429/5xx/
   transport のみで run-level abort は retry 対象外）、`--resume` の
   使用法と `--grant` 非永続・`--input` 変更時の注意を記載
2. `packages/apm/.apm/skills/cheaper-usage/SKILL.md` および
   `cheaper-bulk-read` SKILL.md — `maxTurns: null` 既定・stall timeout・
   resume 契約を消費者向けに反映
3. 回帰 fixture `run-max-turns` / `run-max-tool-calls` の expected 更新と
   normalize.rs の checkpoint path scrub rule を整備（T10）
4. `node scripts/doc_maintenance.js plan --force-index` / `apply` で
   docs/plans・docs/tasks の管理 index を再生成し、新規文書の相互参照を
   整合させる
5. 最終検証: `mise run check`（biome → fmt → clippy → version/doc-id →
   workspace test → node --test → lockfile → payload）

## Done criteria

- docs/guides/configuration.md と SKILL.md が新契約を記述
- 管理 index が再生成され doc_status lint が本件起因の警告を出さない
- `mise run check` が pass

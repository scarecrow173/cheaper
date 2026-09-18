---
id: "TASK-034r2ruj7LGYlehohv1Xjs"
type: "task"
status: "todo"
title: "Align usage-log docs with recording contract"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/35"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/usage-log-recording-scope-docs.md"
  implemented-by: []
  depends-on:
    - "docs/plans/usage-log-recording-scope-docs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/usage-log-recording-scope-docs.md"
    - "docs/designs/usage-log-recording-scope-docs.md"
    - "docs/adr/usage-jsonl-contract-canonical.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/usage-log-recording-scope-docs.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r2ruj7LGYlehohv1Xjs: Align usage-log docs with recording contract

## Work

PLAN-034r2puWPUDxxDyfTSffZR の唯一の task。DESIGN-034r2lHf6FAFVelj9LrhZh
の canonical contract 表に従い、3 文書の記録対象記述を
`buildUsageRecord()` の実際の条件に揃える。

### 1. `docs/guides/usage.md`

「計測 log」節を改訂し、以下を明示する:

- `run`（worker alias 含む）: success と、error envelope が
  `meta.model`+`meta.usage` を持つ failure（usage-bearing
  failure）を記録
- worker/model 解決前の input/config error 等 usage を持たない
  failure・`auth`/`--help`/`--version` は記録対象外
- `count-tokens`: token count が成功し `tokens` を持つ場合のみ
  記録（`FILE_TOO_LARGE`・file not found 等は対象外）
- 保存先・best-effort・`--quiet`・record shape の記述は維持

### 2. `AGENTS.md`

「delegation 計測ログ」節を同じ contract に揃える（guide より
簡潔な要約。集計コマンド未実装の注記は維持）。

### 3. `docs/specs/delegation-usage-measurement-log.md`

- In Scope の「`run` 系コマンド … の成功・error-with-meta
  envelope から usage 記録」を現行 contract と一致させる
- Acceptance Criteria の「`run` 実行ごとに」「`count-tokens`
  実行ごとに」を、success・usage-bearing error・tokens を持つ
  count success に限定する表現に修正
- 記録対象外（usage を持たない failure・auth/help/version）を
  Out of Scope 側で明確化

### 4. 整合確認

- `packages/` に diff が無いこと
- 3 文書に「実行ごとに記録」系の曖昧表現が残っていないこと
- `audit_docs.js --type all` no findings・`mise run check` pass

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/usage-log-recording-scope-docs.md`
- `docs/designs/usage-log-recording-scope-docs.md`
- `docs/specs/usage-log-recording-scope-docs.md`
- `docs/adr/usage-jsonl-contract-canonical.md`

## Dependencies

- なし（上流 doc は全て approved）

## Affected Files

- `docs/guides/usage.md`（modify）
- `AGENTS.md`（modify）
- `docs/specs/delegation-usage-measurement-log.md`（modify）

## Done When

- [ ] 3 文書の記録対象記述が `buildUsageRecord()` の条件と一致
- [ ] usage-bearing failure / pre-run failure の区別が明記
- [ ] successful count / failed count の区別が明記
- [ ] auth/help/version が対象外として一貫
- [ ] 保存先・best-effort・quiet の記述が不変
- [ ] `git diff -- packages/` が空

## Verification

- [ ] `mise run check` exit 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all` no findings
- [ ] 3 文書の grep で曖昧表現（「実行ごとに記録」等）が残らない

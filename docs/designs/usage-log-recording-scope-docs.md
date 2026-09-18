---
id: "DESIGN-034r2lHf6FAFVelj9LrhZh"
type: "design"
status: "approved"
title: "Align usage.jsonl recording-scope docs with implemented contract"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/usage-log-recording-scope-docs.md"
    - "../adr/usage-jsonl-contract-canonical.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/usage-log-recording-scope-docs.md"
  implemented-by:
    - "../plans/usage-log-recording-scope-docs.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./delegation-usage-measurement-log.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/usage-jsonl-contract-canonical.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Align usage.jsonl recording-scope docs with implemented contract

## Context

SPEC-034r2eqGCyFZr7j0uSfst0 / ADR-034r2eId2hXJGWDjBylKVE の実装設計。
`buildUsageRecord()` の記録条件を canonical contract として、
user-facing・spec-level の 3 文書の記述を揃える。実装・
envelope・record shape・保存先は一切変更しない。

## Canonical contract

`buildUsageRecord()`（`packages/cli/src/usage-log.ts`）が返す
record と、その呼出し位置（`packages/cli/src/cli.ts` の 3 箇所:
count-tokens success 分岐・`executeRun` 戻り・catch の error
envelope）から導かれる記録対象は以下で一意に定まる。

| 対象 | 記録される | 記録されない |
|---|---|---|
| `run`（worker alias 含む） | success envelope / `meta.model`+`meta.usage` を持つ error envelope（usage-bearing failure） | `meta.usage`・`meta.model` を持たない failure（worker/model 解決前の input/config error 等） |
| `count-tokens` | `result.tokens` を持つ success envelope | `FILE_TOO_LARGE`・file not found・workspace escape 等の error envelope |
| `auth`・`help`・`version` | —（記録対象外） | 全実行 |

## Scope

### In Scope

- `docs/guides/usage.md`「計測 log」節: 「`run` と
  `count-tokens` は … append する」を、上記 canonical contract
  の記録対象・対象外を明示する表現に改訂する
- `AGENTS.md`「delegation 計測ログ」節: 「実行結果は … 継続
  記録される」を同じ contract に揃える
- `docs/specs/delegation-usage-measurement-log.md`:
  「`run` 実行ごとに … append される」「`count-tokens` 実行
  ごとに … append される」の Acceptance Criteria と In Scope
  の記述を、実際の記録条件と一致させる

### Out of Scope

- `packages/` 以下の code・test・bundle
- DESIGN-034r05iHy2gdnoXu6h71XL・ADR-034r05iHrZUE6WljM1ZMiX の
  本文（既に実装と整合しており変更不要）
- 記録対象の拡大・保存先・best-effort/quiet semantics の変更

## Components and Boundaries

- **guide（usage.md）**: user-facing の contract 説明。
  「何が記録されるか」を run/count/対象外の 3 区分で述べる。
  record shape・保存先・best-effort の既存記述は維持。
- **AGENTS.md**: agent 向けの同じ contract の要約。guide と同じ
  3 区分を 1〜2 文で表現し、集計コマンド未実装の注記は維持。
- **SPEC-034r05iHK8bA7gveeDA5HC**: 実装済み spec の記録対象
  記述の修正。history 改変ではなく contract の明確化なので、
  修正対象は In Scope 1 項目と Acceptance Criteria 2 項目に
  限定し、record shape・best-effort 等の他記述には触れない。

## Data and Control Flow

- 各文書は同じ canonical contract（上表）を異なる粒度で述べる。
  表現の変換はこの design の表に集約し、3 文書間で解釈が
  分かれないようにする。

## Non-Functional Constraints

- documentation-only: `git diff -- packages/` が空であること。
- 表現ドリフト回避: 記録条件は `buildUsageRecord()` の分岐
  （`env.ok`・`env.worker===null`・`result.tokens`・
  `meta.usage`/`meta.model`）と用語を一致させる。

## Risks and Trade-offs

- 過剰な内部実装 detail の暴露: guide/AGENTS.md に関数レベルの
  分岐条件をそのまま書くと読みにくい。spec には条件名
  （`meta.model`+`meta.usage`）を残し、guide/AGENTS.md は
  「usage を持つ error のみ」レベルの自然言語に留める。
- 修正漏れによる残余の曖昧記述: 対象 3 文書を grep で再確認し、
  「実行ごと」「全て記録」系の残存表現が無いことを verification
  に含める。

## Deferred Design Concerns

- failed count-tokens・usage を持たない run failure の記録 |
  ADR-034r2eId2hXJGWDjBylKVE で現行維持と決定 | 挙動変更要求
  が出たとき | reader が記録欠落を契約違反と誤認する（本 design
  で文書化して緩和）

## Verification Notes

- [ ] 3 文書の記述が canonical contract 表と一致
- [ ] `git diff -- packages/` が空
- [ ] `mise run check` が pass
- [ ] `audit_docs.js --type all` が no findings

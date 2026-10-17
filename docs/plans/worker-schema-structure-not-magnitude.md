---
id: "PLAN-MtzbsOB5Isq6zESyqf6thT"
type: "plan"
status: "approved"
test-spec-skip: "Verified by existing schema/golden regression corpus plus new dev-seam tests (rawOutputPath persistence, best-effort failure, empty-raw skip, INVALID_INPUT errors cap) and mise run check; no dedicated test-spec doc authored."
title: "worker schema magnitude 撤廃・maxFindings nullable・INVALID_OUTPUT raw output 永続化の実装計画"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/worker-schema-structure-not-magnitude.md"
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  implemented-by:
    - "docs/tasks/worker-schema-magnitude-removal.md"
    - "docs/tasks/invalid-output-raw-output-persistence.md"
    - "docs/tasks/invalid-input-errors-cap.md"
    - "docs/tasks/worker-prose-maxfindings-null.md"
    - "docs/tasks/consumer-docs-schema-magnitude.md"
    - "docs/tasks/regression-and-final-verification-113-114.md"
  depends-on:
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/worker-schema-structural-validation-not-magnitude.md"
    - "docs/adr/invalid-output-raw-output-persistence.md"
    - "docs/designs/worker-schema-structure-not-magnitude.md"
    - "docs/discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/worker-schema-structure-not-magnitude.md"
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/worker-schema-structural-validation-not-magnitude.md"
    - "docs/adr/invalid-output-raw-output-persistence.md"
  defers: []
  deferred-by: []
---

# worker schema magnitude 撤廃・maxFindings nullable・INVALID_OUTPUT raw output 永続化の実装計画

## Goal

issue #113(input `task` の未文書化 4000 字上限)・#114(完走済み run が
schema の magnitude 違反1件で `INVALID_OUTPUT` 全廃棄)を、統一原則
「worker JSON Schema は構造を検証し magnitude を検証しない」で解消し、
`INVALID_OUTPUT` の raw output 全文を file 回復可能にする
(DESIGN-EmeEdXi9TWZZW2PzZsp29y)。

## Scope

- In: built-in worker schema 8 件の magnitude 撤廃、`maxFindings`
  nullable 化、`repair_output` の INVALID_OUTPUT 時 raw output 永続化、
  `INVALID_INPUT` errors 20 件 cap、worker prose・consumer skill 文書追随、
  regression normalize rule、dev-seam/schema corpus test。
- Out: workflow output materialization の `INVALID_OUTPUT` details 統一、
  provider structured output API 配線、policy 値・tool pagination cap。

## File Map

| 領域 | 対象 |
|---|---|
| input schema | `crates/cheaper-cli/resources/workers/schemas/*.input.schema.json`(4件) |
| output schema | 同 `*.output.schema.json`(4件) |
| raw output 保存 | `crates/cheaper-runtime/src/run/repair.rs` + 新規 `invalid_output.rs` または既存 helper 追記、`run/mod.rs` の `RepairOptions` 構築、`run/boundary.rs`?(`RunWorkerDeps` 変更不要 — `deps.runtime.user_config_dir()` 利用) |
| INVALID_INPUT cap | `crates/cheaper-core/src/worker/validate.rs` |
| worker prose | `crates/cheaper-cli/resources/workers/bulk-reader.md`、`tool-operator.md` |
| consumer docs | `.agents/skills/cheaper-bulk-read/SKILL.md`、`packages/apm/.apm/skills/cheaper-{bulk-read,tool-operate,usage}/SKILL.md` |
| test | schema corpus・dev-seam run fixture・regression `normalize.rs`・`crates/cheaper-core` validate test |

## Task 分解(依存順)

1. **TASK-u2PM** schema magnitude 撤廃 + `maxFindings` nullable(8 schema)
2. **TASK-Ensm** `INVALID_OUTPUT` raw output file 永続化(repair 経路)
3. **TASK-Y48p** `INVALID_INPUT` errors 20 件 cap(validate.rs)
4. **TASK-Qv0y** worker prose 更新(maxFindings null semantics)← T1
5. **TASK-mCX3** consumer skill/guides 文書追随 ← T1,T2
6. **TASK-bFmQ** regression normalize・fixture 追随・最終検証 ← T1–T5

## Verification Matrix

| 要件 | 検証 |
|---|---|
| input magnitude 撤廃 | schema corpus: 4000/6000 字超 `task` 受理・構造違反は拒否 |
| output magnitude 撤廃 | corpus: `title` 121 字超受理、`required`/`type`/pattern 違反は拒否 |
| `maxFindings` nullable | corpus: 省略/null/正整数受理、`0`/負拒否 |
| raw output 回復 | dev-seam: `INVALID_OUTPUT` → `details.rawOutputPath` + file 内容一致、write 不可で省略、空 raw で skip |
| errors cap | unit: 21 件違反で `details.errors` が 20 件 |
| 全体 | `mise run check` 全段 |

## Risks

- 巨大 output が親 context に返る(受容済み trade-off — ADR1 短所)。
- regression snapshot の `rawOutputPath` 非決定性 → normalize rule で対応。
- stale materialized builtin dir(`<temp>/cheaper-builtin-<version>` が
  `.ready` 以降更新されない)で schema 変更が即座に反映されない環境が
  あり得る — 本 PR とは別件の発見として issue 起票候補に記録。

## Rollback

schema JSON・runtime 変更を revert するだけで復旧(永続化 file は
best-effort additive、除去しても既存挙動へ戻る)。

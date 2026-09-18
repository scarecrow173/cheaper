---
id: "IMPL-034r2HoPHxqYBy0c98U70r"
type: "impl"
status: "completed"
title: "Align usage.jsonl recording-scope docs (PLAN-034r2puWPUDxxDyfTSffZR)"
created: "2026-09-18"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/35"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/specs/delegation-usage-measurement-log.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/align-usage-log-docs-with-recording-contract.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Align usage.jsonl recording-scope docs (PLAN-034r2puWPUDxxDyfTSffZR)

## Summary

`usage.jsonl` の記録対象について、user-facing 文書
（`docs/guides/usage.md`・`AGENTS.md`）と
SPEC-034r05iHK8bA7gveeDA5HC の記述を、`buildUsageRecord()` が
実際に記録する条件に揃えた。実装・envelope・record shape・
保存先・best-effort/quiet semantics は一切変更していない
（documentation-only）。

## Context

TASK-034r2ruj7LGYlehohv1Xjs（PLAN-034r2puWPUDxxDyfTSffZR /
SPEC-034r2eqGCyFZr7j0uSfst0 / ADR-034r2eId2hXJGWDjBylKVE /
DESIGN-034r2lHf6FAFVelj9LrhZh）に従う。Issue #35。3 文書の記述が
「run/count-tokens の全実行・全 success/error が記録される」と
読めたため、現行実装を canonical behavior として文書側を
揃えた。

## Implementation

### Changes

- `docs/guides/usage.md`: 「計測 log」節に「記録対象は以下に
  限られる」の明示リストを追加。`run` は success と
  `meta.model`・`meta.usage` を持つ error（usage-bearing
  failure）、`count-tokens` は `tokens` を返した成功のみ、
  `auth`/`--help`/`--version` は対象外と明記。
- `AGENTS.md`: 「delegation 計測ログ」節に同じ記録対象の
  限定を 1 文で追記（guide より簡潔な要約。保存先・
  best-effort・集計コマンド未実装の注記は維持）。
- `docs/specs/delegation-usage-measurement-log.md`:
  In Scope を「success envelope と `meta.model`・`meta.usage`
  を持つ error envelope」に限定、count 側に「`tokens` を持つ
  実行に限る」を追記。Out of Scope に usage を持たない run
  failure・失敗した count-tokens の記録を追加。Acceptance
  Criteria の「`run` 実行ごとに」「`count-tokens` 実行ごとに」
  を記録条件に限定する表現へ修正し、`refined-by` に
  SPEC-034r2eqGCyFZr7j0uSfst0 を記録。

### Decisions During Implementation

- **修正は記録対象の記述に限定**: 保存先・record shape・
  best-effort・`--quiet`・exit code/stdout 不変の記述は全て
  維持。ADR-034r2eId2hXJGWDjBylKVE の採用 option（実装を
  canonical とし文書を揃える）に従う。
- **用語は実装の分岐条件に合わせる**: `meta.model`・
  `meta.usage`・`result.tokens` を条件名として使い、
  guide/AGENTS.md では「usage を持たない failure」「失敗した
  count-tokens」の自然言語に留めて関数レベルの detail を
  露出しすぎない（DESIGN-034r2lHf6FAFVelj9LrhZh の方針）。
- **DESIGN-034r05iHy2gdnoXu6h71XL・ADR-034r05iHrZUE6WljM1ZMiX は
  無修正**: 両者の記述は実装と既に整合していることを確認済み
  （error envelope に usage/model がある場合のみ記録・
  count-tokens の success のみ・auth/help/version 対象外）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- 3 文書の記述を `buildUsageRecord()`
  （`packages/cli/src/usage-log.ts`）の分岐条件と突合:
  `env.ok && env.worker!==null` → run record、
  `env.ok && env.worker===null && result.tokens` → count
  record、`!env.ok && meta.usage/model` → run error record、
  それ以外は null — 一致を確認。
- `packages/cli/test/usage-log.test.ts` の test 名でも
  contract を確認（"returns null for an error envelope without
  usage"・"returns null for a utility envelope without a token
  count"）。
- `git diff -- packages/` が空（documentation-only）。
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all`: no findings。
- `mise run check`: exit 0。

## Risks

- 「失敗した count-tokens・usage を持たない run failure も
  記録したい」需要は doc 修正では実現できない — 別 issue の
  spec 変更として扱う（SPEC-034r2eqGCyFZr7j0uSfst0 の
  Deferred Design Concerns に記録済み）。

## Follow-ups

- なし（集計コマンド・rotation は従来通り
  SPEC-034r05iHK8bA7gveeDA5HC の Out of Scope）。

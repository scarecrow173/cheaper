---
id: "ADR-034r2eId2hXJGWDjBylKVE"
type: "adr"
status: "accepted"
title: "usage.jsonl の記録対象 contract は実装を canonical として文書を揃える"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/usage-log-recording-scope-docs.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/usage-log-recording-scope-docs.md"
  depends-on:
    - "./usage-jsonl-in-user-config-dir-best-effort-append.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/delegation-usage-measurement-log.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/usage-log-recording-scope-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 30. usage.jsonl の記録対象 contract は実装を canonical として文書を揃える

## Context and Problem Statement

`buildUsageRecord()`（`packages/cli/src/usage-log.ts`）の記録
contract — `run` は success と `meta.model`+`meta.usage` を持つ
error のみ、`count-tokens` は `tokens` を持つ success のみ記録 —
は DESIGN-034r05iHy2gdnoXu6h71XL・ADR-034r05iHrZUE6WljM1ZMiX と
整合している。一方 `docs/guides/usage.md`・`AGENTS.md`・
SPEC-034r05iHK8bA7gveeDA5HC の記述は「全実行・全 success/error
が記録される」と解釈できる。どちらを canonical にして不整合を
解消するかを決める。

## Decision Drivers

- Issue #35 が「実装を変えるのではなく、何が記録対象になるかを
  canonical に揃える」「現行実装を canonical behavior として
  文書側を合わせる」と方針を明示している。
- 現行の記録 contract（usage を持たない failure は記録しない）
  は ADR-034r05iHrZUE6WljM1ZMiX で既に決定・実装・検証済み。
- 記録対象の拡大（全 error・失敗した count-tokens も記録）は
  contract 変更であり、doc 修正ではなく spec 変更として扱う
  べき挙動差分。

## Considered Options

- A: 実装を canonical とし、文書（guide・AGENTS.md・spec）を
  実装の記録条件に揃える。
- B: 文書を canonical とし、実装を「全実行・全 envelope を
  記録」に拡張する。
- C: 両方を中間で揃える（一部の failure のみ追加記録し文書も
  修正）。

## Decision Outcome

Chosen option: "A: 実装を canonical とし文書を揃える", because
Issue #35 が承認済みの方針であり、現行 contract は
ADR-034r05iHrZUE6WljM1ZMiX の decision をそのまま実装したもので、
文書だけが広く読める表現になっていたため。

### Consequences

- Good, because `packages/` 以下の変更・test 更新・bundle 再生成
  が不要で、doc-only で完結する。
- Good, because reader は「記録されない実行」の存在を文書から
  学べ、log 欠落を bug と誤認しなくなる。
- Bad, because 「失敗した count-tokens も記録したい」要求は
  doc 修正では実現できず、別 issue の spec 変更が必要になる。

### Confirmation

- `docs/guides/usage.md`・`AGENTS.md`・
  `docs/specs/delegation-usage-measurement-log.md` の記述が
  `buildUsageRecord()` の分岐条件と 1 対 1 で対応することを
  review・audit で確認する。
- `git diff -- packages/` が空であることで documentation-only
  を確認する。

## Implementation Plan

- Affected paths: `docs/guides/usage.md`、`AGENTS.md`、
  `docs/specs/delegation-usage-measurement-log.md`
- Patterns to follow: ADR-034r05iHrZUE6WljM1ZMiX の「記録対象」
  記述（success + meta を持つ error・count-tokens の success・
  auth/help/version 対象外）を canonical 表現として使う
- Patterns to avoid: 「実行ごとに記録」のように failure の
  有無を曖昧にする表現
- Dependencies or configuration: なし

## Verification

- [ ] 3 文書の記録対象記述が `buildUsageRecord()` の条件と一致
- [ ] `git diff -- packages/` が空
- [ ] `mise run check` が pass
- [ ] `audit_docs.js --type all` が no findings

## Pros and Cons of the Options

### Option A（採用）

- Good, because 挙動変更が無く regression risk がゼロ。
- Good, because 既存 ADR の decision をそのまま canonical 表現
  として再利用できる。
- Bad, because 「もっと記録してほしい」需要は別途 spec 変更が
  必要。

### Option B

- Good, because 全 envelope が記録され集計が単純になる。
- Bad, because ADR-034r05iHrZUE6WljM1ZMiX の decision を覆す
  contract 変更であり、test・e2e・bundle の更新コストと
  regression risk を伴う。usage の無い failure を記録しても
  削減効果の実績としては価値が薄い。

### Option C

- Good, because 文書・実装の双方を少しずつ変えて落とし所を
  探れる。
- Bad, because canonical が一意に定まらず、どこまでが記録
  対象かの線引きが恣意的になる。

## More Information

- Issue #35（本 ADR の直接の起点）
- SPEC-034r05iHK8bA7gveeDA5HC / DESIGN-034r05iHy2gdnoXu6h71XL /
  ADR-034r05iHrZUE6WljM1ZMiX: usage.jsonl の既存 contract
- `packages/cli/src/usage-log.ts`: `buildUsageRecord()` の
  記録条件

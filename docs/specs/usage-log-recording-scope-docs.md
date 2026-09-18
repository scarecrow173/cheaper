---
id: "SPEC-034r2eqGCyFZr7j0uSfst0"
type: "spec"
status: "implemented"
title: "Usage log recording scope documentation contract"
created: "2026-09-18"
updated: "2026-09-19"
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
  implements: []
  implemented-by:
    - "../designs/usage-log-recording-scope-docs.md"
  depends-on:
    - "./delegation-usage-measurement-log.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/delegation-usage-measurement-log.md"
  refines:
    - "./delegation-usage-measurement-log.md"
  refined-by: []
  derives-from: []
  derived-by:
    - "../adr/usage-jsonl-contract-canonical.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Usage log recording scope documentation contract

## What Should Be Built

`usage.jsonl` の記録対象について、user-facing 文書
（`docs/guides/usage.md`・`AGENTS.md`）と
SPEC-034r05iHK8bA7gveeDA5HC（`docs/specs/delegation-usage-measurement-log.md`）
の記述を、`buildUsageRecord()`（`packages/cli/src/usage-log.ts`）
が実際に記録する条件と一致させる文書修正。実装は変えず、
文書側を現行 contract に揃える。

## Why Now

Issue #35。実装は DESIGN-034r05iHy2gdnoXu6h71XL・
ADR-034r05iHrZUE6WljM1ZMiX と整合しているが、guide・AGENTS.md・
spec の記述は「run/count-tokens の全実行・全 success/error が
記録される」と解釈できる。reader が記録対象外の実行で log に
record が無いことを bug と誤認したり、存在しない contract を
前提に集計処理を組んだりするリスクがある。

## Users and Value

- parent agent（consumer）: `usage.jsonl` に期待した record が
  無い場合、それが contract 通りの非記録か実装 bug かを文書から
  判断できる。
- 開発者: 文書と実装の contract が一致し、「失敗した
  count-tokens も記録してほしい」等の挙動変更要求が doc 修正では
  なく spec 変更として正しく扱われる。

## Scope

### In Scope

- `docs/guides/usage.md` の「計測 log」節の記録対象記述
- `AGENTS.md` の「delegation 計測ログ」節の記録対象記述
- `docs/specs/delegation-usage-measurement-log.md` の
  In Scope・Acceptance Criteria の記録対象表現
- 記述が実装と一致することの整合確認（
  DESIGN-034r05iHy2gdnoXu6h71XL・ADR-034r05iHrZUE6WljM1ZMiX・
  `packages/cli/src/usage-log.ts`・
  `packages/cli/test/usage-log.test.ts` との突合）

### Out of Scope

- 実装変更（`packages/` 以下の code・test・bundle は変更しない）
- 記録対象の拡大（usage を持たない failure・失敗した
  count-tokens の記録など — 別 issue の spec 変更として扱う）
- 保存先（`<userConfigDir>/usage.jsonl`）・best-effort append・
  `--quiet`・exit code/stdout 不変の contract 変更
- 集計・summary コマンド、log rotation/retention

## Requirements

1. **run の記録対象を明示**: 文書は `run` 系コマンド（
   `run <worker>`・worker alias）のうち、success envelope と、
   error envelope が `meta.model` と `meta.usage` を持つ
   failure（usage-bearing failure）のみが記録対象であることを
   明示する。
2. **run の非記録を明示**: worker/model 解決前の input/config
   error など `meta.usage` を持たない failure、および
   auth/help/version 等の utility command は記録対象外である
   ことを明示する。
3. **count の記録対象を明示**: `count-tokens` は token count が
   成功し result が `tokens` を持つ場合のみ記録され、
   `FILE_TOO_LARGE`・file not found・workspace escape 等の
   failure は記録対象外であることを明示する。
4. **既存 contract の維持**: 保存先・record shape・best-effort・
   `--quiet` の記述 semantics を変えない。
5. **documentation-only**: 変更は上記 3 文書と本 chain の
   canonical doc に限定し、`packages/` 以下に diff を作らない。

## Acceptance Criteria

- [ ] `docs/guides/usage.md`・`AGENTS.md`・
      `docs/specs/delegation-usage-measurement-log.md` の記録対象
      記述が `buildUsageRecord()` の条件（success run・
      `meta.model`+`meta.usage` を持つ error・`tokens` を持つ
      count success）と一致する。
- [ ] usage-bearing failure と pre-run failure（usage を持たない
      失敗）の区別が各文書から読み取れる。
- [ ] successful count measurement と failed count の区別が
      各文書から読み取れる。
- [ ] auth/help/version が記録対象外であることが各文書で一貫する。
- [ ] 保存先・best-effort・quiet の記述が現行 semantics を維持
      する。
- [ ] `packages/` 以下に変更が無い（documentation-only）。
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js
      --type all` が no findings。

## Deferred Design Concerns

- failed count-tokens・usage を持たない run failure の記録 |
  現行 contract を維持する判断（ADR-034r2eId2hXJGWDjBylKVE）|
  挙動変更の要求が出たとき | contract を望む reader が期待と
  異なる記録欠落を見る
- usage 集計・summary コマンド・log rotation | 従来通り
  follow-up（SPEC-034r05iHK8bA7gveeDA5HC の Out of Scope）|
  集計要件の具体化 | `usage.jsonl` が無限に伸びる

## Sources and Relations

- Issue #35: 要件の起点。現行実装を canonical とする方針・対象
  file・acceptance criteria が記載されている。
- SPEC-034r05iHK8bA7gveeDA5HC / DESIGN-034r05iHy2gdnoXu6h71XL /
  ADR-034r05iHrZUE6WljM1ZMiX: usage.jsonl の canonical contract。
  本 spec は spec の記述面のみを修正する。
- `packages/cli/src/usage-log.ts`: `buildUsageRecord()` の記録
  条件（canonical behavior）。

---
id: "IMPL-034Zji13BkaQ2V9RC2Wprw"
type: "impl"
status: "completed"
title: "Document workflow catalog contract and usage"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/document-workflow-catalog-contract-and-usage.md"
  implemented-by: []
  depends-on: []
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
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Document workflow catalog contract and usage

## Summary

`workflow list` と catalog manifest 契約を 3 本の user-facing guide
へ記載した。catalog が発見用 index であり権限・実行の正本でない
こと、named lookup が存在しないこと、manifest schema/bounds/
diagnostics と登録・検証手順を規約として固定した。

## Context

`TASK-034ZiGQrT3mNkVx9sLpW7d`。approved plan の slice 3、
spec AC8(guide 記載)に対応。

## Implementation

### Changes

- `docs/guides/usage.md`: command 一覧に `workflow list` 行を
  追加。option 専用表の `--limit` 記述を「`history`(上限 500)と
  `workflow list`(上限 256)」へ訂正。`## workflow list` 節を新設し、
  受理 option・utility envelope の result 形状・fail-closed と
  entry 診断の二分・`--limit` の truncated 規約・named lookup
  非対応を記載。
- `docs/guides/workflow-usage.md`: `## Project workflow catalog
  （workflow list）` 節を新設。manifest 配置場所
  (`.cheaper/workflows/catalog.json`)・schemaVersion 1 の field
  表(id/path/summary/tags の必須・形式・上限)・manifest 全体の
  bound(64 KiB・256 entry)・登録→`workflow list` 検証の手順・
  `status`/`diagnostics` の kind 一覧・stale entry が残る規約・
  発見用 index で正本/権限/実行経路でないことを記載。
- `docs/guides/parent-agent-integration.md`: `## workflow の発見
  （project catalog）` 節を新設。parent agent が `workflow list`
  で private DAG なしに catalog 済み workflow を列挙できること、
  catalog が権限・実行の正本でないこと、entry 異常が明示 path
  実行へ影響しないことを記載。

### Decisions During Implementation

- `workflow list` の詳細契約は `usage.md`、manifest の書き方は
  `workflow-usage.md`、parent 視点の境界は
  `parent-agent-integration.md` と guide の役割分担に合わせて
  分割し、相互に anchor link で参照させた(重複記載の回避)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js
  audit --type all --blocking --json` — blocking finding なし。
- `mise run check` — exit 0。

## Risks

- なし。

## Follow-ups

- #204(workflow create skill)が catalog 登録の正規経路になる段階で、
  登録手順の記述を skill 生成へ更新する余地がある。

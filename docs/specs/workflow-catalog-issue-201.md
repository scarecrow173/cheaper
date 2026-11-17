---
id: "SPEC-034ZixoNtKPep3iXNKSv5n"
type: "spec"
status: "approved"
title: "Workflow catalog and read-only list for issue 201"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/201"
    - "https://github.com/scarecrow173/cheaper/issues/200"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "workflow-file-interface.md"
    - "workflow-contract-compiler.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-catalog-issue-201.md"
  derived-by:
    - "../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow catalog and read-only list for issue 201

## What Should Be Built

project/workspace scope の軽量 Workflow catalog
(`<workspace>/.cheaper/workflows/catalog.json`)と、それを読み取る read-only
`cheaper workflow list` を追加する。catalog は `{id, path, summary, tags?}`
の bounded manifest で、発見用の index であって Workflow 定義・policy・
grant の正本ではない。実行は従来通り明示 path の
`workflow interface <path>` / `workflow run <path>` のみ。

## Why Now

Epic #200 の3層導線(L0 判定 → L1 discover → L2 create)の共通基盤。
L1 discover(#203)は既存候補との重複・再利用比較をカタログから行い、
L2 create(#204)は承認後の登録先として同じ契約を使う。明示 path 以外の
探索手段が無いまま discover を作ると、親 Agent が各 Workflow の
private DAG を開く設計になり省トークン目的に反する。

## Users and Value

親 Coding Agent が、保存済み Workflow の候補を private definition を
context に読み込まずに列挙・重複確認できる。利用者は登録済み Workflow
の所在と用途 summary を一覧でき、異常な entry(missing path・path
escape・duplicate id・stale)を誤選択せずに済む。

## Scope

### In Scope

- `<workspace>/.cheaper/workflows/catalog.json` の schemaVersion 1
  manifest 契約(entries `{id, path, summary, tags?}`、bounded 上限)。
- read-only `cheaper workflow list`(utility envelope、bounded `--limit`、
  `--cwd`/`--format`/`--quiet`/`--log`/`--log-format` のみ受理)。
- manifest/entry の検証と diagnostics(missing path・malformed・
  duplicate id・path escape・stale entry)。
- create skill(#204)が従う catalog 更新の file 契約文書化。
- project 内の重複・保存場所規約の guide 反映。

### Out of Scope

- `workflow run <name>` の named lookup、自動 routing、registry
  service、daemon/watcher、全 file glob、全 workflow compile、暗黙実行。
- catalog entry からの権限付与・grant・policy 変更・workflow 実行。
- user-global scope の catalog(loader 整合が未設計。将来の別 Issue)。
- catalog の自動修復・stale entry の自動削除。
- catalog への書込み CLI(L2 create #204 の設計対象。本 issue は file
  契約の文書化まで)。

## Requirements

- catalog は発見用 index であり、実行・権限・定義の信頼源にしない。
- manifest 欠如は空一覧として成功扱いし、明示 path の既存機能を妨げ
  ない。manifest の構造的破損(JSON parse 不可・schemaVersion 非対応・
  上限超過)は error envelope で fail closed する。
- entry 単位の異常は `ok:true` の一覧内で per-entry status +
  diagnostics として報告し、黙って別 Workflow へ置き換えない。
- entry の `path` は workspace root 相対の明示 path。resolve 時に
  `resolve_workspace_path` の workspace 境界・symlink policy・deny
  path を既存 loader と同じ policy 入力で適用する。
- 一覧時に compiler・resource inference・worker/tool/model を実行せず、
  private jobs/prompt/binding を出力に含めない。
- catalog file 自体の read は bounded(bytes/entries/field 長の上限)。
- 重複 id は黙った last-wins 上書きをせず、両 entry を diagnostics で
  報告する。

## Acceptance Criteria

- [ ] AC1: `workflow list` が utility envelope の `result` に
  `{catalog, workflows, diagnostics}` を返し、private DAG/prompt/
  binding が出力に含まれない。`worker` は `null`、exit code は 0。
- [ ] AC2: manifest 欠如の workspace では空一覧を返し、既存
  `workflow interface`/`workflow run` が従来通り動く。
- [ ] AC3: malformed JSON・非対応 schemaVersion・上限超過の manifest
  は error envelope + 非 0 exit とし、空カタログと誤認させない。
- [ ] AC4: missing path・path escape・deny path・stale entry・
  duplicate id を per-entry status と diagnostics で報告し、黙って
  別 entry を選ばない。
- [ ] AC5: 一覧は compiler・DAG/prompt・worker/tool/model を一切
  実行しない(検証は stat/resolve まで)。
- [ ] AC6: `--limit` で entries 数を制限でき、truncated が分かる。
- [ ] AC7: catalog が grant/policy/権限・workflow 実行・usage 記録を
  一切行わない(read-only)。
- [ ] AC8: create skill 向けの catalog 更新 file 契約(schemaVersion、
  entry 項目、検証規則、更新手順の例)を文書化し、project 内の重複・
  保存場所規約を guide に反映する。
- [ ] AC9: CLI unit/integration test と `mise run check` 対象の検証が
  通る。

## Deferred Design Concerns

- user-global catalog scope | 現行 loader は workspace 境界で、user
  scope の Workflow は実行経路と整合しない | `workflow run` が
  `<userConfigDir>` 上の Workflow を正式に扱う設計時 | catalog が
  実行不可能な file を示唆する誤解。
- catalog 書込み CLI(`workflow register` 等) | #204 で L2 create の
  保存・登録手段を設計する際に再評価 | create skill が file 契約だけで
  十分か、CLI 補助が必要かを #204 が判定 | なくても file 契約で登録
  可能。ただし検証の二重化リスクは残る。

## Sources and Relations

Issue #201(親 Epic #200)が product source。#69(保存場所は規約、
named lookup は延期)、#70(private DAG を隠した公開 I/O)の確定方針を
継承する。discovery: `../discovery/workflow-catalog-issue-201.md`。

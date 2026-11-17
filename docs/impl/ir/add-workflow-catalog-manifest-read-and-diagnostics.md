---
id: "IMPL-034Zjezcm59lyRuRGuYUnl"
type: "impl"
status: "completed"
title: "Add workflow catalog manifest read and diagnostics"
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
    - "docs/tasks/add-workflow-catalog-manifest-read-and-diagnostics.md"
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

# Add workflow catalog manifest read and diagnostics

## Summary

`crates/cheaper-workflow/src/catalog.rs` に project scope workflow catalog
(`.cheaper/workflows/catalog.json`)の bounded read/validate/diagnose を
実装した。design の data model・fail-closed/一覧診断の二分・
diagnostic kind 分類どおりで、compiler・workflow file 本文・resource
inference には一切触れない。

## Context

`TASK-034ZiFaW8ZtvJ5XKVksixu`。approved plan の slice 1、
`workflow-catalog-issue-201` spec、
`workflow-catalog-manifest-and-read-only-list` design、
catalog index ADR、manifest/list contract test spec に従う。

## Implementation

### Changes

- `catalog.rs` 新規。`read_catalog(root, &FilesystemPolicy) ->
  Result<CatalogRead, CheaperError>` を公開し、`CatalogRead{present,
  workflows, diagnostics}`・`CatalogEntry{id,path,summary,tags,status}`・
  `CatalogDiagnostic{kind,index,id?,path?,message}`・enum
  `EntryStatus{Ok,Missing,Invalid,Duplicate}`・
  `DiagnosticKind{MalformedEntry,DuplicateId,MissingPath,
  PathOutOfWorkspace,DeniedPath,NotRegularFile}` を serde 射影で定義。
- bounded read(64KiB・256 entries・id<=64・path<=512・summary<=500・
  tags<=16×64)を manifest-level で fail-closed(`INVALID_INPUT`)。
  JSON parse 失敗・`schemaVersion!=1`・`workflows` 非 array も同様。
- entry path は `resolve_workspace_path` + `create_path_denylist` +
  `policy.filesystem.symlinks` の loader と同一規則で
  resolve→deny→stat のみ行い、file 本文を読まない。
- loader の deny 照合を `pub(crate) fn check_workflow_path_denied`
  へ抽出し、catalog と共有。独自 deny 判定の複製(表現ドリフト)を
  防ぐため、plan の file map を超えて `loader.rs` に触れた。
- `lib.rs` から必要型・`CATALOG_RELATIVE_PATH`・`MAX_CATALOG_ENTRIES`
  を export。
- `tests/filesystem_contracts/catalog.rs` 新規 13 test。
  absent/empty manifest、fail-closed bound 群、射影 field 限定
  (private key 非出力)、missing/deny/escape/not-regular-file、
  symlink policy、duplicate 非 last-wins、malformed slot 保持を固定。

### Decisions During Implementation

- `CatalogRead` の field 名は manifest key と同じ `workflows` に
  統一(表現ドリフト回避)。
- string field の長 cap 超過は manifest 破損として全体を
  `INVALID_INPUT` で失敗させる(test spec どおり)。shape 不備のみ
  entry 単位の `malformed-entry`。
- entry `path` の絶対 path は「workspace-relative 必須」より
  `malformed-entry`。stat が not-found 以外で失敗した場合も
  `missing-path` として扱い、message に error kind を残す。
- catalog file の read 自体の失敗は `INVALID_REQUEST`(loader の
  file read 規約に合わせる)。
- malformed entry の出力は「string field は書かれた通り、非 string
  field は `null`、tags は string array でない限り `[]`」とし、
  unknown key は射影しない。
- duplicate 判定は path 検査より先(missing path でも duplicate-id)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-workflow --test
  filesystem_contracts catalog::` — 13 pass。
- `mise exec -- cargo test -p cheaper-workflow --test
  filesystem_contracts` — 57 pass(loader 共有化の回帰なし)。
- mutation check: duplicate 判定・deny 判定を各々無効化し、対応
  test が各 2 件 fail することを確認(検出能力の実証)。
- `cargo fmt --all -- --check` — pass。

## Risks

- `resolve_workspace_path` の error は全て `path-out-of-workspace`
  kind に集約する(symlink policy 違反も含む)。kind を細分化すると
  loader 内部表現が漏れるため、message 側に詳細を残す設計とした。

## Follow-ups

- slice 2(`workflow list` CLI)でこの pub API を dispatch へ接続。

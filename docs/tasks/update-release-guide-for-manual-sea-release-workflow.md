---
id: "TASK-034z2pbpAynIVRedenPgXa"
type: "task"
status: "todo"
title: "Update release guide for manual SEA release workflow"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Update release guide for manual SEA release workflow

## Work

`docs/guides/release.md` の release 手順を更新する。現行 step 6
「tag / GitHub Release は現時点では自動化していない。必要なら
commit 後に `git tag v<X.Y.Z>` を手動で打つ」を、以下の流れに
書き換える:

1. `git tag v<X.Y.Z>` で tag を打ち push する
2. Actions の `release` workflow を手動 dispatch し `tag` input に
   `v<X.Y.Z>` を指定する（自動 trigger は無いため dispatch 必須）
3. workflow が 3 platform の executable を build・smoke し、
   Release `v<X.Y.Z>` に添付する

tag と package version の不一致時に workflow が fail する挙動、
macOS artifact が現時点で無い点も明記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md

## Dependencies

- release workflow の構成（trigger・matrix・version 検証）が確定
  していること（`create-manual-dispatch-release-workflow-...` の
  成果に依存するため内容は揃える）

## Affected Files

- `docs/guides/release.md`

## Done When

- [ ] release 手順が manual dispatch ベースの手順に更新されている
- [ ] version 不一致時の fail 挙動と macOS 非対象が明記されている

## Verification

- [ ] guide の記述が `release.yml` の trigger・input・matrix と一致

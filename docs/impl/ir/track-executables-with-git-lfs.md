---
id: "IMPL-034z3CE24rUsOGqD0BfeqZ"
type: "impl"
status: "completed"
title: "track executables with git lfs"
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
    - "docs/tasks/track-apm-executables-with-git-lfs-and-keep-freshness-check-valid.md"
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

# track executables with git lfs

## Summary

`.gitattributes` に `cli/bin/*/cheaper*` の LFS rule を追加し、
exe が pointer file として commit されることを確認した。
`git lfs` 前提を AGENTS.md に追記。

## Context

docs/plans/assemble-apm-executable-payload-with-platform-launchers.md task 3。executable コミットの repo 肥大化を
避けるため pointer のみ履歴に残す（ユーザー決定: LFS 管理）。

## Implementation

### Changes

- `.gitattributes`:
  `packages/apm/.apm/hooks/cli/bin/*/cheaper* filter=lfs
  diff=lfs merge=lfs -text`
- `AGENTS.md`: セットアップ節に `git lfs install` 前提と CI の
  `lfs: true` 対応を追記
- staged: `cli/bin/win32-x64/cheaper.exe`（pointer のみ）

### Decisions During Implementation

- パターンを `cli/bin/*/cheaper*` に限定し、既存の
  `bin/cheaper.mjs`（text）を巻き込まない形にした
- freshness diff は LFS pointer 比較になるため、exe 再生成で
  内容が変われば diff として検出される既存仕組みを維持

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `git check-attr` → filter/diff/merge=lfs・text unset
- `git lfs ls-files` に `cheaper.exe` 列挙
- `git cat-file -p :...cheaper.exe` が LFS pointer text
  （`version https://git-lfs...`, `oid sha256:435312...`,
  `size 117718856`）を返す
- `mise run check` 全緑（bundle freshness diff を含む）

## Risks

- `git lfs` 未 install の clone/pull では pointer text が
  smudge されず exe が実体を得られない。AGENTS.md に前提を
  記載済み

## Follow-ups

- remote（GitHub）での LFS quota 監視

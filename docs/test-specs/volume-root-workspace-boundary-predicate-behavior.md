---
id: "TSPEC-034D9bHOWe5zvPoIeMncr9"
type: "test-spec"
status: "draft"
title: "Volume-root workspace boundary predicate behavior"
created: "2026-10-09"
updated: "2026-10-09"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/volume-root-workspace-boundary-predicate-fix.md"
  derived-by: []
  verifies:
    - "docs/specs/volume-root-workspace-boundary-acceptance.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Volume-root workspace boundary predicate behavior

## Purpose

`is_inside_workspace_root` が workspace root の trailing separator の
有無に依らず inside 判定 contract（root 自身 or `root + SEP`
prefix）を満たすことを保証する。volume-root 以外の root 形状では
従来どおりの判定を維持し、本 spec は contract 違反の regression
検出のために残す。

## Feature

Volume-root workspace boundary acceptance — filesystem volume root
（win32 `C:\`、posix `/`）を workspace root にした場合の
inside/outside 判定。

## Rules

- Rule: root が `nodepath::SEP` で終わる場合、prefix は root その
  ものを使い separator を二重化しない。
- Rule: prefix は normalized 比較用文字列（`case_insensitive` 時は
  lowercased 済みの `r`）から構築する。
- Rule: boundary は fail-closed — trailing-sep root の配下でない
  path（別 drive、sibling prefix 衝突）は引き続き outside。
- Rule: `resolve_workspace_path` は volume-root root で inside path
  を `outside-workspace` なしに resolve し、`real`/`display` を
  期待値どおりに返す。

## Examples

- Example: win32 build で
  `is_inside_workspace_root("C:\\", "C:\\resource-probe", true)` →
  `true`（`cfg!(windows)` gate。修正前は `false`）
- Example: posix build で
  `is_inside_workspace_root("/", "/a", false)` → `true`
  （`cfg!(unix)` gate）
- Example: `is_inside_workspace_root("C:\\", "D:\\x", true)` →
  `false`（platform 不問の ungated assertion）
- Example: posix で `resolve_workspace_path(canonicalize_workspace_root("/"),
  "<uniq>/a", Deny)` → `real = "/<uniq>/a"`、`display = "<uniq>/a"`
  （`<uniq>` は `cheaper-volroot-probe-<pid>` 形式の nonexistent
  一意名。win32 は raw `"C:\\"` root で同形）
- Example: root 自身（`c == r`）は trailing-sep の有無に依らず
  inside。

## Guarantees

- 修正後の guard は `crates/cheaper-core/src/resources/predicates.rs`
  `resource_contains` の `ends_with(nodepath::SEP)` 分岐と同一
  pattern である。
- golden fixture・既存 boundary test（escape、symlink、prefix 衝突）
  は全て pass し、fail-closed 性質を維持する。
- CI は posix shape のみ検証する。win32 shape の検証は手動 Windows
  `mise run check` に依存することを verification 記録に明記する。

## Non-goals

- non-trailing-sep root の判定 semantics の再検証（既存 test が
  担保）
- `C:/` のような non-canonical root の扱い（入力 contract は
  canonicalize 済み root）
- walker・deny_paths・ResourceManifest 経路の追加検証（同 predicate
  を共有するが別 spec の責務）

## Risk

本 spec が失われると、volume-root workspace での boundary 誤拒否が
再導入されても検出できない（fail-closed のため silent に全 tool
操作が拒否される）。win32 shape は CI 未検証のため、win32 側の
regression は手動実行に依存する。

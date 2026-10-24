---
id: "SPEC-034D85I40UBKtNgeam655z"
type: "spec"
status: "implemented"
title: "Volume-root workspace boundary acceptance"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/77"
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
    - "../adr/guard-trailing-separator-in-workspace-boundary-predicate.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/volume-root-workspace-boundary-check-double-separator.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Volume-root workspace boundary acceptance

## What Should Be Built

`crates/cheaper-core/src/workspace/paths.rs` の `is_inside_workspace_root`
を修正し、workspace root が path separator で終わる場合（volume root
`C:\`、POSIX `/`）でもその配下の path を正しく inside と判定できる
ようにする。あわせて volume-root ケースの regression test を追加する。

## Why Now

issue #77。TS 実装由来の defect が Rust port に持ち越されており、
filesystem root を workspace root にした実行では root 自身を除く全ての
配下 path が `outside-workspace` で fail-closed に reject される。
小さいが contract 違反であり、修正の欠陥ではなく欠落した guard の補完。

## Users and Value

`--cwd` に drive root や filesystem root を渡す consumer、および
workflow / tool layer が workspace boundary を正しく扱えることに依存する
parent agent。誤拒否の解消により boundary contract が root 形状に
依らず一貫する。

## Scope

### In Scope

- `is_inside_workspace_root` の trailing-separator guard（root が既に
  `nodepath::SEP` で終わる場合は separator を追加しない）
- win32 shape（`C:\` + `C:\x`）と posix shape（`/` + `/x`）の unit test
- `resolve_workspace_path` が volume-root workspace で inside path を
  resolve できることの test（可能なら両 platform 形状）
- 既存 `is_inside_workspace_root_cases` / boundary test の非 regress

### Out of Scope

- `canonicalize_workspace_root` の正規化仕様変更
- `nodepath` 層の lexical resolve/relative 実装変更
- `resource_contains`（既に正しい）や他 predicate への波及修正
- deny_paths、symlink policy、walker の挙動変更
- volume-root workspace 経路の UX（`--cwd` の使い勝手）改善

## Requirements

- predicate contract「candidate が root と等しい、または `root + SEP`
  で始まる」を、root が既に SEP で終わる場合にも二重化せず維持する
- 修正は `crates/cheaper-core/src/resources/predicates.rs`
  `resource_contains` と同じ guard pattern（`ends_with(SEP)` 分岐）に
  揃える
- predicate の入力 contract は「`canonicalize_workspace_root` 等で
  canonicalize 済みの root」を前提とする（`resource_contains` と同じ
  前提。dunce canonicalize は win32 で `\` 正規形を返すため、host SEP
  guard で十分。非 canonical root が将来流入する場合は別途検討）
- boundary は引き続き fail-closed: volume-root 以外の outside path
  （別 drive、sibling prefix 衝突 `C:\work` vs `C:\workx` 等）の拒否
  semantics を変えない
- `case_insensitive` 比較の semantics を変えない
- 変更後も `mise run check`（fmt/clippy/test 等）が pass する

## Acceptance Criteria

- [ ] win32 build で `is_inside_workspace_root("C:\\", "C:\\resource-probe",
  true)` が `true` を返す（`nodepath::SEP` が `\` の build でのみ成立。
  test は `cfg!(windows)` で gate する）
- [ ] posix build で `is_inside_workspace_root("/", "/a", false)` が
  `true` を返す（`cfg!(unix)` 等で gate。各 platform は自身の shape を
  検証し、修正前に fail・修正後に pass する assertion であること）
- [ ] `is_inside_workspace_root("C:\\", "D:\\x", true)` が `false`
  を返す（別 drive は引き続き拒否）
- [ ] `resolve_workspace_path` が volume-root workspace で inside path を
  `outside-workspace` なしに resolve する。test は実 FS 上の symlink
  （macOS の `/tmp` → `/private/tmp` 等）を避けるため、root 直下の
  nonexistent tail（存在しない component で walk が早期 break する）を
  使って構成する
- [ ] 既存の boundary test（escape、symlink、prefix 衝突 case）が
  全て pass する
- [ ] `mise run check` が pass する

## Deferred Design Concerns

- workspace root の trailing separator をどこまで canonicalize するかは
  `canonicalize_workspace_root` の責務論として本変更では触らない。

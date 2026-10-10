---
id: "ADR-034D85IukRi32huPbZDbCT"
type: "adr"
status: "proposed"
title: "Guard trailing separator in workspace boundary predicate"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "../discovery/volume-root-workspace-boundary-check-double-separator.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
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
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Guard trailing separator in workspace boundary predicate

## Context and Problem Statement

`crates/cheaper-core/src/workspace/paths.rs` の `is_inside_workspace_root`
は `c == r || c.starts_with(root + SEP)` で inside 判定する。root が
path separator で終わる場合（`C:\`、`/`）に prefix が `C:\\` / `//`
に二重化し、配下の全 path を誤って outside と判定する（issue #77）。
TS 実装由来の defect が Rust port に持ち越された。

同一リポジトリ内で同形状の ancestor 判定を行う
`crates/cheaper-core/src/resources/predicates.rs` の `resource_contains`
は `parent.ends_with(SEP)` の guard 付きで既に正しい。

## Decision Drivers

* predicate contract（root 自身 or `root + SEP` prefix）を維持しつつ
  volume-root 形状でも正しく動作させる
* boundary check は fail-closed を維持する（誤許可に転じない）
* 同一概念の判定ロジックはリポジトリ内で一貫した形に揃える
* 変更は最小の coherent な範囲に留める

## Considered Options

* Option A: predicate 側で `root.ends_with(SEP)` を guard し、separator
  の二重化を防ぐ
* Option B: `canonicalize_workspace_root` で trailing separator を除去し
  root を正規形にする
* Option C: 文字列 prefix 判定をやめ component 分割比較に切り替える

## Decision Outcome

Chosen option: "Option A", because predicate の欠落した guard を補完する
だけで contract を満たせ、`resource_contains` と同一 pattern に揃って
一貫性が保たれ、root 正規化が引き起こす `C:` / `C:\` の意味差異を
踏まないため。

### Consequences

* Good, because 修正が predicate 1 箇所に閉じ、既存 test への影響が
  最小で、boundary の fail-closed 性質が変わらない。
* Good, because `resource_contains` と同一 guard pattern になり、同じ
  知識の表現が統一される。
* Bad, because trailing separator を含む root は他にもあり得るが、本
  修正は predicate 契約内の吸収に留まり、呼び出し側の正規化責務には
  踏み込まない（scoped な許容）。

### Confirmation

`is_inside_workspace_root` の unit test に volume-root case を追加する
（`nodepath::SEP` は build 時確定なので win32 shape は `cfg!(windows)`、
posix shape は `cfg!(unix)` で gate し、各 platform は自身の shape を
検証する）。`resolve_workspace_path` の volume-root resolve は root
直下の nonexistent tail で構成して実 FS symlink（macOS `/tmp` 等）に
左右されない test とする。`mise run check` の pass をもって実装完了と
する。

## Implementation Plan

* Affected paths: `crates/cheaper-core/src/workspace/paths.rs`（
  `is_inside_workspace_root` と `is_inside_workspace_root_cases` test）
* Patterns to follow:
  `crates/cheaper-core/src/resources/predicates.rs` `resource_contains`
  の `ends_with(nodepath::SEP)` guard
* Patterns to avoid: `canonicalize_workspace_root` での trailing separator
  除去（drive-relative path への意味変化）、component 分割比較への全面
  切替
* Dependencies or configuration: なし

## Verification

* [ ] `mise exec -- cargo test -p cheaper-core` で paths.rs test が全て
  pass する
* [ ] `mise run check` が pass する
* [ ] `is_inside_workspace_root` の guard が `resource_contains` と同一
  pattern であることを code review で確認する

## Pros and Cons of the Options

### Option A: predicate 側の trailing-sep guard

* Good, because `resource_contains` と同一 pattern で一貫し、diff が
  最小で、contract を変えない。
* Bad, because trailing-sep root が他の呼び出し経路にも現れた場合に
  predicate 以外の吸収が必要になる可能性は残る。

### Option B: root 正規化で trailing sep 除去

* Good, because 下流全部に効き、predicate を単純に保てる。
* Bad, because `C:\` → `C:` は drive-relative path に意味が変わり
  危険。root の display 値・保存値にも影響し、scope が広がる。

### Option C: component 分割比較

* Good, because 構造的に最も正確。
* Bad, because `nodepath` の byte-parity 層を迂回する過剰な変更で、
  pure string predicate としての単純さを失う。

## More Information

- issue #77: <https://github.com/scarecrow173/cheaper/issues/77>
- 正規形参照実装:
  `crates/cheaper-core/src/resources/predicates.rs`（#65-A）

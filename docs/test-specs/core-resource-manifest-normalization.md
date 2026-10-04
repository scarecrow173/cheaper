---
id: "TSPEC-034zQHrk9A6Zc7rrSLGeHA"
type: "test-spec"
status: "draft"
title: "Core resource manifest normalization"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
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
    - "docs/designs/core-resource-manifest-normalization.md"
  derived-by: []
  verifies:
    - "docs/specs/core-resource-manifest-normalization.md"
    - "docs/designs/core-resource-manifest-normalization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---


# Core resource manifest normalization

## Purpose

identity と safety / ownership の区別が後続 lease / compiler で失われないよう保証する。共有 D4/D5 契約が廃止された場合のみ見直す。

## Feature

workspace-relative declaration を canonical real identity に変換し、manifest と pure conflict 判定を全 consumer で共有する。

## Rules

### Rule: identity は存在状態ではなく明示 scope で決まる

future exact / subtree は identity を持つ。subtree は本体と descendant を含み、src/a と src/ab を混同しない。Windows 比較は case-insensitive、POSIX は case-sensitive。

### Rule: access safety と producer ownership を区別する

unknown read/mutation は root shared/exclusive。glob reads/writes は policy 別に保守近似し、produces は部分集合を推測しない。canonical claim の mode 不整合は reject。

### Rule: 入力契約と boundary は fail closed

D5 宣言に mode を足さず、unknown key / mode-hint / 混在 / path / scope / policy を検査。boundary error を wrap せず、契約違反は INVALID_REQUEST。

### Rule: predicates は canonical identity と mode のみを比較する

read/read は並行可、overlapping read/write と write/write は conflict。独立 path は conflict しない。mode coverage は exclusive が shared を覆い、逆は不可。

## Examples

- AC1/2: 不存在 src/a/f、subtree src/a、本体、descendant、sibling src/ab。workspace ../escape、deny junction、within-workspace alias、外部 alias、platform case。
- (a): resources 省略 / {}、各 bucket omitted / null / []。reads/writes null は unknown、produces null は reject、空配列は宣言済み access なし。
- (b): mode-hint / unknown field / 同一 bucket の declaration/claim 混在 / canonical bucket-mode mismatch を reject。
- (c): dir/*.json は within-workspace root、deny dir。produces dir/** は両 policy で dir ownership を維持。
- (d): POSIX d\ir/*.json は root、Windows dir\*.json は deny dir。
- (e): produces dir/** + subtree は受理。dir/*.json、dir/**/file、exact dir/**、glob tail .. を reject。leading ! は root safety。
- (f): canonical manifest を再正規化して恒等。canonical path の glob meta は literal identity。
- (g): permutation、duplicate、case alias は入力順に依存せず deterministic。
- (h): WorkspaceEscapeError をそのまま伝播。その他入力違反は CheaperError(INVALID_REQUEST)。
- AC5: exact/exact、subtree/exact、subtree/subtree、ancestor、独立 path と shared/exclusive 全組合せ。

## Guarantees

上記 named cases を executable tests で保護する。future path の stat による scope 推測、filesystem 現存一覧による producer 推測、glob key 比較を導入しない。

## Non-goals

lease / DAG inference / hard-link alias / claim 内 symlink の到達集合保証 / 外部 process 差替え / filesystem-root boundary 修正。POSIX host を現在 Windows run で検証したとは主張しない。

## Risk

within-workspace literal exact/subtree の内部 symlink は canonical claim 外へ到達し得る。producer ownership を拡大して解決しない。境界契約が変わる際に本保証と residual-risk を再検討する。

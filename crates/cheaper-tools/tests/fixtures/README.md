# Differential fixtures (frozen corpus)

`differential.rs` が replay する TS 参照 fixture。生成元の
`dump-tool-fixtures.mjs`（TypeScript 実装側）は既にリポジトリに存在しない
ため、本 corpus は **frozen** — 生成・再収録は行わず、Rust 側の
regression 防壁として現状のまま固定する。

- case の追加・変更は行わない（新挙動は別テストでカバーする）
- `expectRust` は設計固定ルールによる既知 divergence の pin 用 sparse
  override であり、divergence を直したくなった場合も fixture を
  書き換えて合わせるのではなく仕様判断を先に行う
- 詳細な fixture format は `tests/differential.rs` 冒頭コメントを参照

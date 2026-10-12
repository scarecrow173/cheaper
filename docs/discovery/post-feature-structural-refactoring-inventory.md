---
id: "DISC-034EDjXUQnxTiHz4batpQu"
type: "discovery"
status: "resolved"
title: "Post-feature structural refactoring inventory"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source:
    - "../research/structural-refactoring-inventory/refactoring-plan.html"
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
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../designs/post-feature-structural-refactoring.md"
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-feature structural refactoring inventory

## Exploration Goal

最低限の基本機能が揃った時点（v1.0.0、PR #97 merge 後）で、
実装を変更せずに全 workspace（8 crate・約 95.5k 行）を並列調査し、
まとめてリファクタリングすべき箇所と、その設計を可視化すること。
調査結果は
`docs/research/structural-refactoring-inventory/refactoring-plan.html`
（canonical 保存版。Cytoscape.js の crate 依存グラフ現状/改善後の
同一座標比較 + Mermaid UML + 49 候補の 3 フェーズ表）に集約済み。

## Key Issues and Assumptions

- crate トポロジ（依存 DAG）は一方向・循環なしで健全。再編・統合・分割は不要。
- 課題は crate 内の巨大ファイル・移植由来の小規模重複・責務の同居に集中。
- 観測可能挙動（envelope/error code/message・code-unit sort・abort 意味論・
  usage.jsonl）を変えない内部整理に限定することが前提。
- 調査は read-only 静的解析であり、`mise run check` は実行していない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| crate 再編（統合/分割） | 境界の明確化 | DAG は既に健全・大差なし・リスク大 | no |
| crate 内 module 再編 + 共有 helper 集約 | 観測挙動不変で段階実施可能 | crate 内に課題が残る範囲は限定的 | **yes** |
| 触らない | リスクなし | 巨大ファイル・重複が育つ | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: 49 候補を 3 フェーズに整理。Phase 0 = hygiene 19 件（小重複の
  解消）、Phase 1 = structural 16 件（crate 内再編）、Phase 2 = deep/deferred
  14 件（設計判断・影響大）。なお spec 段階で `catalog/mod.rs` 分割を
  hygiene ではなく structural へ再分類し、workflow の
  `ToolCatalog`/`ToolMetadata` 統一を Phase 2 相当へ defer したため
  spec scope は Phase 0 = 18 件 / Phase 1 = 17 件（計 35）となっている。
- **confirmed**: cli の composition root が run.rs に同居（models.rs・
  workflow_run.rs が `crate::run::` 経由で参照）。
- **confirmed**: tools の「resolve→escape→deny→observe」パターンが
  6〜8 箇所に分散。単一 adapter 化は有効だが characterization test が先行必須。
- **confirmed**: workflow engine.rs が borrow checker 回避の `Pump` 再構築 ×3
  + `stop!` macro を使用。RunState struct 化で可読性改善。
- **confirmed（contract drift 3 件）**: `workflow run --timeout` の details
  欠落、`extension_models` のコメント/実装齟齬、`usage_report` の
  canonicalize 省略差分。
- **hypothesis**: JS 互換層（js/js16/json/nodepath/errors）を core 内
  compat 名前空間に集約する整理余地。別 crate 化は YAGNI。
  → spec の Out of Scope に「実施判断自体を別途行う項目」として記録済み
  （49 候補の numbered item ではなく hypothesis のため deferred 扱い）。

## Open Questions

- `extension_models` の「default を埋めない」コメントと実装（128k/16k を
  埋める）の齟齬 — 実装が正とみなしてコメント修正でよいか、TS parity の
  意図確認が要る。
- `usage_report` の canonicalize 省略が意図かどうか。
- Phase 2（args.rs 宣言的 table、ToolError enum、!Send 解消等）は個別判断 —
  本 work chain では deferred として記録する。

## Promotion Candidates

- [x] spec-doc needed: Phase 0+1 の scope・受入基準・Phase 2 の deferred 記録
- [x] adr-doc needed: crate トポロジ維持・crate 内 module 再編の採否記録

## Sources and Relations

- `docs/research/structural-refactoring-inventory/refactoring-plan.html` —
  7 並列サブエージェント調査の統合レポート（crate 依存グラフ・実行
  パイプライン UML・tool runtime 内部構造・crate 別所見・3 フェーズ
  計画・contract drift 3 件・非目標）。元生成物 `tmp/refactoring-plan.html`
  は gitignore 対象のため恒久保存版として research 配下へ移設。

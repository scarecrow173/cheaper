---
id: "TSPEC-034rzVrqJiXNDl2AF8eI5s"
type: "test-spec"
status: "approved"
title: "pricing metadata resolution and models command"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/designs/model-pricing-catalog-and-curated-exceptions.md"
  derived-by: []
  verifies:
    - "docs/specs/model-pricing-catalog.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# pricing metadata resolution and models command

## Purpose

normalized `ModelPricing` の source 優先順位（runtime catalog →
bundled snapshot → unpriced）と `cheaper models` の露出範囲を固定する。
#44/#46 が同じ contract を消費するため、source/freshness 表現の
drift を防ぐ。

## Feature

pricing metadata 解決と models 一覧表示

## Rules

- runtime catalog の cost があれば `source: "runtime-catalog"` の
  pricing を返す
- runtime 欠損時は bundled snapshot entry を
  `source: "bundled-snapshot"` + `snapshotVersion` で返す
- どちらにも無い model は unpriced（error にしない）
- `cheaper models` は各 model の available/excluded・pricing・
  tier・tierSource・pricing source/freshness を含む
- `--provider` / `--tier` filter が機能する

## Examples

- runtime cost あり + snapshot にも同 key あり → runtime-catalog 優先
- runtime cost 欠落 + snapshot entry あり → bundled-snapshot
- 両方無し → pricing フィールドが無い/undefined で error にならない
- `cheaper models --tier low` → effective tier が low の model のみ

## Guarantees

- offline（runtime refresh なし）でも bundled snapshot が読める
- #44/#46 が `source` と `snapshotVersion`/`updatedAt` で
  freshness を判断できる

## Non-goals

- snapshot の自動生成・鮮度閾値の強制
- subscription の実請求額の再現

## Risk

- pricing source の判別不能になると #44 の cost 推定が根拠を失う

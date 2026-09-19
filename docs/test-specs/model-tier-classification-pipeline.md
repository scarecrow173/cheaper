---
id: "TSPEC-034rzVqBmfD2pkA9f82Eoa"
type: "test-spec"
status: "approved"
title: "model tier classification pipeline"
created: "2026-09-19"
updated: "2026-09-20"
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

# model tier classification pipeline

## Purpose

分類 priority（exclude → explicit-override → curated-exception →
unclassified）と `tierSource` 記録の意図を保持する。auto classifier
stage（`tierSource: "auto"`）は #46 の scope であり #45 では生成
しない。#46 が auto classifier を実装する際も、この spec が示す
priority と source 命名は不変条件として残る。priority 自体が設計から
外れた場合に廃止できる。

## Feature

モデルの tier 分類パイプライン

## Rules

- 分類 priority は `overrides > curated exceptions > unclassified`
  の順で最初に一致したものを採用する（auto classifier は #46 の
  seam であり #45 では `auto` を生成しない）
- 各結果は `tierSource`（`explicit-override` / `curated-exception` /
  `auto` / `unclassified`）を持つ
- unclassified model の effective tier は `medium`（low にしない）
- curated exception entry は `tier`・`reason`・`source`・
  `checkedAt` を持ち、exact key と `provider/*` wildcard を解釈する
- pricing 欠落（unpriced）の model は error にならず unclassified まで
  落ちる

## Examples

- overrides に `"p/m": "low"` があり exception にも `p/m: high` が
  ある → tier=low, source=explicit-override
- exception `ollama/*` に `ollama/qwen3` が一致 → source=
  curated-exception
- exception 非該当で pricing.output=0.5 → #45 では自動分類しないため
  tier=medium, source=unclassified
- pricing も exception も無い model → tier=medium, source=unclassified

## Guarantees

- override が例外より常に優先される（#46 で auto classifier が
  追加されてもこの順位は維持される）
- tier source が呼び出し側（resolver・`cheaper models`・meta）へ
  そのまま伝播する

## Non-goals

- #46 の正式 auto tier formula の閾値妥当性
- 個別 exception entry の内容正当性（運用で更新される）

## Risk

- priority が崩れると user override が効かず高コスト model を選択し得る

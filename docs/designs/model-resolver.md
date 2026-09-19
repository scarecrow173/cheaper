---
id: "DESIGN-034r05iHrZUE6Wu1cMdeJ1"
type: "design"
status: "approved"
title: "Model Resolver"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
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
    - "worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "pi-execution-engine.md"
    - "auto-tier-classification.md"
    - "../discovery/auto-tier-classification-effective-cost.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../adr/model-tier-model-resolver.md"
  derived-by:
    - "../plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "../plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Model Resolver

## Context

ADR-034r05iHlWosDpLI5iJRbx の tier 抽象を実装する。Pi `ModelRuntime` が提供する利用可能モデル集合から、Worker の要求 tier と Runtime Policy の `maxTier`・`models.yaml` の override/preference を交差して 1 つの `Model` を選ぶ。実機プローブで確定: `ModelRuntime.create({authPath, modelsPath})` は未認証でも動作し、`getAvailable()` は認証済みモデルのみを返し（未認証環境では空配列、例外ではない）、`getModel()` は認証なしで静的 metadata（cost/contextWindow/maxTokens/reasoning）を返す。

## Scope

### In Scope

- `models.yaml`（user/project）のスキーマ・ロード・マージ
- tier 分類（overrides > built-in 表 > cost heuristic > medium）
- 候補選択と fallback（higher のみ）
- 選択結果の stderr 通知

### Out of Scope

- 認証フローそのもの（auth.json/models.json/env var は Pi に委ねる）
- provider 別の追加認可設定
- model の動的再選択・mid-run 切替

## Components and Boundaries

- 入力: `ModelRuntime.create()`（デフォルト agentDir = Pi の `~/.pi/agent`、`PI_CODING_AGENT_DIR` 尊重）→ `getAvailable()`。0 件なら `MODEL_TIER_UNAVAILABLE`（details: `{ noAuthenticatedModels: true }`）
- `model/config.ts`（core）: `models.yaml` スキーマ。`version: 1` 必須、未知 field 拒否 → `INVALID_MODEL_CONFIG`。merge は policy と同規則: `overrides` は object deep merge、`preferences.<tier>` は array 全体 replace
- `model/classify.ts`（core）: tier 分類
- `model/resolve.ts`（core）: 候補選択・fallback・並び順

```yaml
version: 1
overrides:            # "<provider>/<modelId>": tier
  openai/gpt-5-mini: low
preferences:          # tier ごとの優先順。"<provider>/<modelId>" または "<provider>/*"
  low: ["anthropic/claude-haiku-4-5", "openai/*"]
```

## Data and Control Flow

- 分類 priority: `overrides` > built-in 表 > cost heuristic > `medium`
- built-in 表（model id 小文字、正規表現。high → low → medium の順に評価し最初の一致を採用）:
  - high: `opus`, `gpt-5(\.\d+)?-pro`, `\bo1(-pro)?\b`, `o3-pro`, `heavy`, `ultra`
  - low: `haiku`, `\bmini\b`, `nano`, `flash`, `lite`, `small`, `codex-mini`, `\b(1|3|7|8)b\b`
  - medium: `sonnet`, `gpt-4o`, `gpt-4\.1`, `gpt-5(\.\d+)?$`, `gemini-.*-pro`, `\bo3\b`, `codex`
- cost heuristic（Pi `Model.cost.output`、USD/1M token）: `< 2` → low、`≤ 20` → medium、`> 20` → high。cost 欠落・0 → medium
- 選択: `requested > maxTier` → `POLICY_VIOLATION`（exit 4）。候補 = available ∩ tier。順序 = `preferences` のパターン順 → 残りを `(cost.input + cost.output)` 昇順 → provider → id で決定論的に
- fallback: 候補が空で `tierFallback: higher` のとき 1 段ずつ上へ（maxTier まで）。それでも無ければ `MODEL_TIER_UNAVAILABLE`（details: 各 tier の候補数）
- 選択結果は stderr に `model: <provider>/<id> (tier <t>)` を 1 行出力（`--quiet` で抑止）

## Non-Functional Constraints

- 決定論: 同一入力（available 集合・config・policy）に対し同一モデルを選ぶ。並び順はスコア同値時も provider/id で total order
- unknown を low にしない（分類不能は medium）

## Risks and Trade-offs

- built-in 正規表現表は新モデル登場で陳腐化する。`overrides` が逃げ道であり、表の変更は patch 更新で対応
- cost heuristic は Pi metadata 依存。provider が cost を報告しない場合 medium へ倒れる（誤って安い枠に入れない方向に倒す）
- 既知の限界: `o1-mini` / `o3-mini` 系は high パターン `\bo1(-pro)?\b` が先に評価され high に分類される（実装レビューで確認）。実際には安価な派生モデルだが、`models.yaml` の `overrides` が逃げ道として機能する。誤分類を直すにはパターンの後読みが必要になるため、V1 では記録のみで対応しない

## Deferred Design Concerns

- tier の動的キャリブレーション（実測品質に基づく再分類） | V1 は静的規則 | 品質データが蓄積した時点 | 低品質モデルが low に残り続けるリスク

## Verification Notes

- [ ] 未認証環境で `getAvailable()` が空 → `MODEL_TIER_UNAVAILABLE` exit 5
- [ ] overrides/built-in/cost の分類優先順位がドキュメント順に適用される
- [ ] preferences パターン順・cost 昇順・provider/id の決定論的順序
- [ ] `requested > maxTier` → `POLICY_VIOLATION`、`tierFallback: higher` で maxTier までの段階的上昇
- [ ] `models.yaml` の未知 field → `INVALID_MODEL_CONFIG` exit 3

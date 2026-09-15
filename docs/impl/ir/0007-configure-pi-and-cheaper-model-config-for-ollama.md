---
id: "IMPL-0007"
type: "impl"
status: "completed"
title: "Configure Pi models.json and cheaper models.yaml for Ollama"
created: "2026-09-16"
updated: "2026-09-16"
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
  implements:
    - "docs/tasks/0054-configure-pi-models-json-and-cheaper-models-yaml-for-ollama-provider.md"
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

# Configure Pi models.json and cheaper models.yaml for Ollama

## Summary

Pi の custom provider 設定 `%USERPROFILE%\.pi\agent\models.json` に Ollama プロバイダを新規作成し、cheaper のモデル選択設定 `~/.config/cheaper/models.yaml` に `ollama/qwen3.5:4b` の low tier ピン留めを追加した。`createModelRuntime()` 無引数経路で `getAvailable()` が該当モデルを返し、`resolvePiModel` が tier `low` で解決することを確認した。

## Context

TASK-0054（PLAN-0004 / ADR-0009）に従う。両ファイルとも Git 管理外の環境依存設定であり、リポジトリ内の変更は IR と task 文書のみ。

## Implementation

### Changes

- `%USERPROFILE%\.pi\agent\models.json`（新規）: `providers.ollama` = `{ api: "openai-completions", baseUrl: "http://localhost:11434/v1", apiKey: "ollama" }`、モデル `qwen3.5:4b`（reasoning: true、cost 全 0、contextWindow 262144、maxTokens 32768）
- `~/.config/cheaper/models.yaml`（新規）: `overrides: {"ollama/qwen3.5:4b": low}`、`preferences.low: ["ollama/*"]`

### Decisions During Implementation

- `apiKey: "ollama"` のリテラルを採用。`provider-composer.js` の `composeApiKeyAuth` で `rawKey !== undefined` かつ env var 参照でない値は "configured API key" として認証済み扱いになり、`no authentication method configured` を回避できる（Ollama 側は任意の Bearer を受理）
- `qwen3.5:4b` は `4b` が built-in low パターン `\b(1|3|7|8)b\b` に合致せず、cost 0 も medium 判定になるため、`overrides` で明示的に low へピン留め（cost を偽って low に寄せる方法は取らない）
- `models.yaml` は `CHEAPER_CONFIG_DIR` 未設定のため既定の `~/.config/cheaper/` に配置。project `.cheaper/` は smoke 実行 workspace に依存するため user config 側が安定
- `reasoning: true` を指定。qwen3.5 は thinking モデルで推論が `message.reasoning` に返ることを `/v1/chat/completions` で確認済み

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `mise exec -- pnpm -r build` → 全パッケージ成功
- `createModelRuntime()` → `getAvailable()` = `[{provider:"ollama", id:"qwen3.5:4b", maxTokens:32768, cost:{0,0,0,0}}]`
- `resolvePiModel({requestedTier:"low", policy:{maxTier:"medium",tierFallback:"none"}})` → `{provider:"ollama", id:"qwen3.5:4b", tier:"low"}`
- `loadModelConfig({userConfigDir: ~/.config/cheaper, projectDir: <repo>/.cheaper})` → `{overrides:{"ollama/qwen3.5:4b":"low"}, preferences:{low:["ollama/*"]}}`、`classifyModel` → `low`

## Risks

- thinking モデルの推論トークン消費により content が空になる場合がある（maxTokens 32768 で緩和済み、TASK-0055 で実測確認）
- `models.json` / `models.yaml` は Git 管理外のため再現手順は Experiment Log に記録が必要

## Follow-ups

- TASK-0055: 実 provider smoke test 実行（bulk-read / cheap-write）
- TASK-0056: Experiment Log 記録と `pnpm -r test` 回帰確認

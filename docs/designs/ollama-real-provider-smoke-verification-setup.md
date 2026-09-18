---
id: "DESIGN-034r05iHrZUE6WBzjS90b8"
type: "design"
status: "approved"
title: "Ollama real-provider smoke verification setup"
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
    - "docs/specs/ollama-real-provider-smoke-verification.md"
    - "docs/adr/use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ollama real-provider smoke verification setup

## Context

`cheaper` の実 provider 経路を Ollama（Docker コンテナ）で検証するための環境構成。ランタイムのコード変更は行わず、Pi の custom provider 機構（`~/.pi/agent/models.json`）と cheaper の `models.yaml` だけで model resolution に乗せる（ADR-034r05iHlWosDpQ6Z1LgJo, SPEC-034r05iHK8bA7g9pm5z8s2）。

## Scope

### In Scope

- リポジトリ内の `docker-compose.yml` による Ollama コンテナ定義とモデル pull 手順
- `%USERPROFILE%\.pi\agent\models.json` への Ollama provider 追加（非破壊マージ）
- `$CHEAPER_CONFIG_DIR/models.yaml` による low tier へのモデル割り当て
- tmp workspace での `bulk-read` / `cheap-write` 実実行と envelope 検証
- `docs/impl/exp/` への Experiment Log 記録、AGENTS.md へのコンテナ手順記載

### Out of Scope

- cheaper / Pi のコード変更、CI 組み込み、他 provider、モデル品質評価

## Components and Boundaries

- `docker-compose.yml`: `ollama/ollama` イメージ、`11434:11434` ポート公開、`ollama` named volume でモデル永続化。`docker compose up -d` で起動、`docker compose exec ollama ollama pull <model>` でモデル取得
- `models.json`（リポジトリ外）: provider id `ollama`、`api: "openai-completions"`、`baseUrl: "http://localhost:11434/v1"`、`apiKey: "ollama"`（literal・任意値。未設定だと provider-composer が "no authentication method configured" で provider を構成しない）、`models[]` に `id`/`cost`（0 固定）/`contextWindow`/`maxTokens`
- `models.yaml`（リポジトリ外 `$CHEAPER_CONFIG_DIR`、未設定時は `~/.config/cheaper`）: `version: 1`、`overrides` で `"ollama/<id>": "low"` をピン留め（cost 0 は medium 判定のため必須。id が `\b(1|3|7|8)b\b` 等の low パターンに合致する場合は省略可だが明示する）、`preferences.low: ["ollama/*"]`
- 検証対象: `node packages/cli/bin/cheaper.js bulk-read` / `cheap-write`（`--cwd <tmp workspace>`）

## Data and Control Flow

1. `docker compose up -d` → `curl http://localhost:11434/v1/models` で応答確認 → モデル pull
2. `models.json` マージ → `mise exec -- pnpm -r build`
3. `echo '<input json>' | node packages/cli/bin/cheaper.js bulk-read --cwd <tmp>` → exit 0・stdout が envelope 1 行・`meta.model.provider === "ollama"`・`meta.usage` 実測を確認
4. `cheap-write` を tmp workspace で実行 → 実ファイル変更と `meta.changes` の実測を確認
5. 結果を Experiment Log に記録し、AGENTS.md にコンテナ検知・起動手順を追記
6. `pnpm -r test` で回帰が無いことを確認

## Non-Functional Constraints

- 秘密情報・環境依存設定はリポジトリへコミットしない（`models.json` / `models.yaml` はリポジトリ外）
- Ollama の小モデルは出力品質が低く、output schema 検証で repair 経路を通る可能性がある。repair も成功扱いの範囲内とし、最終的に schema 適合の envelope が返ればよい
- コンテナ未起動時は fail closed でエラーになることが期待される（provider 到達不能 → structured error）

## Risks and Trade-offs

- リスク: 小モデルが tool calling / structured output を安定して返せない → 緩和: repair（`repairAttempts: 1`）が既定で有効。それでも失敗する場合は結果を Experiment Log に記録し報告
- リスク: `models.json` の既存内容を破壊 → 緩和: 事前バックアップ + マージのみ

## Deferred Design Concerns

- CI での実 provider smoke: Docker 前提のため別途検討

## Verification Notes

- [ ] `/v1/models` が pull 済みモデルを返す
- [ ] `bulk-read` が exit 0・envelope 1 行・`meta.model.provider` が `ollama`
- [ ] `cheap-write` が `meta.changes` の実測を返す

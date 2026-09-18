---
id: "IMPL-034r05iHy2gdnpngzmKvJT"
type: "impl"
status: "completed"
title: "Start Ollama container and pull model"
created: "2026-09-15"
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
    - "docs/tasks/start-ollama-container-and-pull-qwen3-5-quantized-model.md"
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

# Start Ollama container and pull model

## Summary

`docker compose up -d` で Ollama コンテナ（`cheaper-ollama-1`、`ollama/ollama:latest`）を起動し、`docker compose exec ollama ollama pull qwen3.5:4b` でモデルを取得した。`/v1/models` が `qwen3.5:4b` を返し、`/v1/chat/completions` が実測 usage を返すことを確認した。

## Context

TASK-034r05iHQeQpDpB3QaYlPY（PLAN-034r05iHy2gdnpLvJw9CNY / ADR-034r05iHlWosDpQ6Z1LgJo）に従う。リポジトリ変更は発生せず、Docker コンテナと Ollama モデルという外部状態のみを準備した。

## Implementation

### Changes

- リポジトリ内ファイルの変更なし
- 外部状態: Docker コンテナ `cheaper-ollama-1`（起動中）、named volume `cheaper_ollama`、モデル `qwen3.5:4b`（4.7B / Q4_K_M / context length 262144、capabilities: completion / vision / tools / thinking）

### Decisions During Implementation

- モデルは `qwen3.5:4b` のデフォルトタグ（Q4_K_M 量子化）を採用。`4b` は built-in low パターン（`(1|3|7|8)b`）に合致せず medium 判定になるため、TASK-034r05iHQeQpDpByADDPCn で `models.yaml` の `overrides` により low へピン留めする前提
- API キーは OpenAI 互換エンドポイントで任意値を受け付けることを `Authorization: Bearer ollama` で確認済み

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `curl http://localhost:11434/v1/models` → `{"object":"list","data":[{"id":"qwen3.5:4b",...}]}`（exit 0）
- `docker compose ps` → `cheaper-ollama-1 Up`
- `docker compose exec ollama ollama show qwen3.5:4b` → parameters 4.7B / quant Q4_K_M / context 262144
- `curl /v1/chat/completions`（`qwen3.5:4b`, max_tokens=8）→ usage `{prompt_tokens:15, completion_tokens:8, total_tokens:23}`、finish_reason `length`（thinking モデルのため reasoning フィールドに出力、content は空）

## Risks

- `qwen3.5:4b` は thinking モデルであり、推論出力が `message.reasoning` に入る。`max_tokens` が小さいと content が空になる可能性がある。Pi / cheaper の応答パースに影響する場合は TASK-034r05iHQeQpDpDygNnvfq で観測する
- `4b` の medium 判定は overrides で解決する予定だが、overrides の適用範囲が限定的な場合は別タグ（`qwen3:8b` 等）への切替が必要

## Follow-ups

- TASK-034r05iHQeQpDpByADDPCn: `models.json` / `models.yaml` 設定と low tier 解決の確認
- TASK-034r05iHQeQpDpDygNnvfq: 実 provider smoke test 実行

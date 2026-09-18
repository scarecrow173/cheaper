---
id: "TASK-034r05iHQeQpDpB3QaYlPY"
type: "task"
status: "done"
title: "Start Ollama container and pull qwen3.5 quantized model"
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
    - "docs/plans/verify-cheaper-real-provider-path-with-ollama-on-docker.md"
  implemented-by:
    - "docs/impl/ir/start-ollama-container-and-pull-model.md"
  depends-on:
    - "docs/plans/verify-cheaper-real-provider-path-with-ollama-on-docker.md"
    - "docs/tasks/add-ollama-docker-compose-and-document-container-usage-in-agents-md.md"
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
---

# Start Ollama container and pull qwen3.5 quantized model

## Work

`docker compose up -d` で Ollama コンテナを起動し、`docker compose exec ollama ollama pull qwen3.5:4b` でモデルを取得した。`curl http://localhost:11434/v1/models` でモデルが一覧に載ることを確認した。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/verify-cheaper-real-provider-path-with-ollama-on-docker.md`

## Dependencies

- `docs/tasks/add-ollama-docker-compose-and-document-container-usage-in-agents-md.md`

## Affected Files

- なし（外部状態: Docker コンテナ・Ollama モデル）

## Done When

- [x] Ollama コンテナが起動し `/v1/models` が pull 済みモデルを返す

## Verification

- [x] `curl -s http://localhost:11434/v1/models` が exit 0 でモデル id を含む JSON を返す（`qwen3.5:4b`）
- [x] `docker compose ps` で `cheaper-ollama-1` が Up
- [x] `/v1/chat/completions` が実測 usage（`total_tokens`）を返す

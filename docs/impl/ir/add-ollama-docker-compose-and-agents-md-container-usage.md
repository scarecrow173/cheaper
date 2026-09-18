---
id: "IMPL-034r05iHy2gdnpmfo09FGX"
type: "impl"
status: "completed"
title: "Add Ollama docker-compose and AGENTS.md container usage"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docker-compose.yml"
    modified:
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-ollama-docker-compose-and-document-container-usage-in-agents-md.md"
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

# Add Ollama docker-compose and AGENTS.md container usage

## Summary

Ollama 検証用の `docker-compose.yml` を追加し、AGENTS.md にコンテナの検知・起動・停止手順を追記した。

## Context

TASK-034r05iHQeQpDpySHHPmpY / PLAN-034r05iHy2gdnpLvJw9CNY / DESIGN-034r05iHrZUE6WBzjS90b8 / ADR-034r05iHlWosDpQ6Z1LgJo に従う。実 provider smoke の前提となる Ollama コンテナの定義と手順。

## Implementation

### Changes

- `docker-compose.yml` — `ollama/ollama:latest`、`127.0.0.1:11434:11434` バインド、`ollama` named volume で `/root/.ollama` を永続化
- `AGENTS.md` — 「検証用 Ollama コンテナ」節を追加。起動・モデル pull・稼働確認（`curl /v1/models`）・停止手順と、未起動時の挙動（provider 到達不能で失敗 → 起動して再実行）を記載

### Decisions During Implementation

- **127.0.0.1 バインド**: ローカル検証用途のため外部露出を避ける
- **latest タグ**: 検証用のためピン留めは行わない（再現性が必要になったらタグ固定を検討）

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `docker compose config` → exit 0（パース成功）
- `grep -n "11434" docker-compose.yml AGENTS.md` → 双方でマッチ

## Risks

- `ollama/ollama:latest` はタグ非固定のため将来イメージ差分が入りうる（検証用途の範囲では許容）

## Follow-ups

- なし。

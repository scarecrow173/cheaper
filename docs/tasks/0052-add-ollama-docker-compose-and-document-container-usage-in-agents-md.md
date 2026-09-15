---
id: "TASK-0052"
type: "task"
status: "done"
title: "Add Ollama docker-compose and document container usage in AGENTS.md"
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
    - "docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md"
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

# Add Ollama docker-compose and document container usage in AGENTS.md

## Work

リポジトリ直下に `docker-compose.yml` を作成し、`ollama/ollama` イメージでポート `11434` を公開・named volume でモデルを永続化する。AGENTS.md にコンテナ未起動時の検知方法（`curl http://localhost:11434/v1/models`）と起動・モデル pull 手順を追記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md`

## Dependencies

- なし

## Affected Files

- `docker-compose.yml`（新規）
- `AGENTS.md`（変更）

## Done When

- [ ] `docker-compose.yml` が存在し `ollama/ollama` サービスとポート・volume が定義されている
- [ ] AGENTS.md にコンテナ検知・起動・pull 手順が記載されている

## Verification

- [ ] `docker compose config` が exit 0 でパースできる
- [ ] `grep -n "11434" docker-compose.yml AGENTS.md` がマッチを返す

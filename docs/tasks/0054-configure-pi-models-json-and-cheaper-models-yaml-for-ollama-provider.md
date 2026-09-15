---
id: "TASK-0054"
type: "task"
status: "done"
title: "Configure Pi models.json and cheaper models.yaml for Ollama provider"
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
  implemented-by:
    - "docs/impl/ir/0007-configure-pi-and-cheaper-model-config-for-ollama.md"
  depends-on:
    - "docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md"
    - "docs/tasks/0053-start-ollama-container-and-pull-qwen3-5-quantized-model.md"
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

# Configure Pi models.json and cheaper models.yaml for Ollama provider

## Work

`%USERPROFILE%\.pi\agent\models.json`（新規）に Ollama custom provider を追加し、`~/.config/cheaper/models.yaml`（新規、`CHEAPER_CONFIG_DIR` 未設定時の既定 user config dir）に tier 設定を追加した。いずれも Git 管理外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md`

## Dependencies

- `docs/tasks/0053-start-ollama-container-and-pull-qwen3-5-quantized-model.md`

## Affected Files

- `%USERPROFILE%\.pi\agent\models.json`（Git 管理外）
- `~/.config/cheaper/models.yaml`（Git 管理外）

## Done When

- [x] Pi `getAvailable()` が `ollama/qwen3.5:4b` を返し、tier `low` で解決される

## Verification

- [x] `createModelRuntime().getAvailable()` が `ollama/qwen3.5:4b`（cost 0, ctx 262144, maxTokens 32768）を返す
- [x] `resolvePiModel({requestedTier:'low'})` → `ollama/qwen3.5:4b` / tier `low`
- [x] `loadModelConfig` が `~/.config/cheaper/models.yaml` の overrides/preferences を読み、`classifyModel` が `low` を返す

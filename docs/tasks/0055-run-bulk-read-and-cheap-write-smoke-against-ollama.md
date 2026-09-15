---
id: "TASK-0055"
type: "task"
status: "done"
title: "Run bulk-read and cheap-write smoke against Ollama"
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
    - "docs/impl/ir/0008-run-real-provider-smoke-test-with-ollama.md"
  depends-on:
    - "docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md"
    - "docs/tasks/0054-configure-pi-models-json-and-cheaper-models-yaml-for-ollama-provider.md"
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

# Run bulk-read and cheap-write smoke against Ollama

## Work

一時 workspace で `bulk-read` / `cheap-write` を実 provider（Ollama `qwen3.5:9b`）経由で実行し、envelope・meta.model・meta.usage・meta.changes（mutation journal）を実測確認した。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0004-verify-cheaper-real-provider-path-with-ollama-on-docker.md`

## Dependencies

- `docs/tasks/0054-configure-pi-models-json-and-cheaper-models-yaml-for-ollama-provider.md`

## Affected Files

- なし（外部状態: 一時 workspace、`~/.config/cheaper/policy.yaml`）

## Done When

- [x] `bulk-read` / `cheap-write` が exit 0 で envelope 1 行を返す

## Verification

- [x] `bulk-read`: exit 0、`meta.model.provider=ollama` / `id=qwen3.5:9b` / `tier=low`、実測 usage（total 2347 tokens）、`changes` 空（read-only として正しい）、`repairs: 2`
- [x] `cheap-write`: exit 0、`meta.changes.files=[{path:"src/farewell.ts",kind:"created",linesAdded:9}]`（journal 実測）、ファイルが実際に作成された

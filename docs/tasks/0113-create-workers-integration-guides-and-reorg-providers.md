---
id: "TASK-0113"
type: "task"
status: "done"
title: "Create workers/integration guides and reorganize providers.md"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0023-user-facing-docs-information-architecture.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0023-user-facing-docs-information-architecture.md"
    - "docs/tasks/0112-create-usage-configuration-guides-and-contributing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0028-user-facing-docs-information-architecture.md"
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0113: Create workers/integration guides and reorganize providers.md

## Work

### 1. `docs/guides/workers.md`

- worker とは: front matter（name/description/tools/cheaper
  section）+ 本文 prompt の定義 file
- built-in 3 worker: `bulk-reader`（read-only 大規模 read/抽出/
  要約、tools: read/grep/find/ls/count_tokens）・`code-writer`
  （定義済み生成・変更、write/edit 追加）・`doc-writer`（document
  生成）— input の required/properties は各 schema JSON と一致
  させる
- custom worker: `run --worker-file <path>`・worker discovery
  （builtin/project/user の解決順は `resolveWorker` 準拠）
- runtime policy との関係（policy.yaml の tools.allow・
  model.maxTier 等、詳細は configuration.md へ link）

### 2. `docs/guides/parent-agent-integration.md`

- parent coding agent との役割分担（判断・統合 vs 機械的・
  大 context 処理の委譲）
- integration primitive: skills（`cheaper-usage`・
  `cheaper-bulk-read`・`cheaper-code-write`・`cheaper-doc-write`）
  と hooks（`pre-read-code.js` — read 前に `count-tokens` で
  threshold 超過 file を block し bulk-read へ誘導、default
  8192・`CHEAPER_READ_TOKEN_THRESHOLD`）
- install: consumer の `apm.yml` で `scarecrow173/cheaper/
  packages/apm` を依存に指定（virtual package 参照 or local
  path）
- target 別挙動: Windsurf は `pre_read_code` が native event で
  機能、他 target では同じ登録が no-op（ADR-0016/DISC-0009
  準拠の正確な記述。本ブランチの hooks.json には
  `pre_read_code` のみ）
- 確認方法: install 後に hook config・bundled CLI（
  `cheaper.mjs --version` / `count-tokens`）を確認

### 3. `docs/guides/providers.md` 再編

- 残す: config file 場所表・Ollama/OpenAI の models.json・
  models.yaml 例・tier 判定・auth 手順・response_format の
  使い分け（user 影響あり）
- 移す: docker compose smoke 手順・NVIDIA toolkit 注記 →
  CONTRIBUTING.md 側。EXP-* 実測メモ・repair 内部実装詳細 →
  削るか EXP/IMPL 参照に圧縮

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0023-user-facing-docs-information-architecture.md`
- `docs/designs/0028-user-facing-docs-information-architecture.md`

## Dependencies

- `docs/tasks/0112-create-usage-configuration-guides-and-contributing.md`
  （CONTRIBUTING.md が smoke 手順の移動先として存在する前提）

## Affected Files

- `docs/guides/workers.md`（add）
- `docs/guides/parent-agent-integration.md`（add）
- `docs/guides/providers.md`（modify）

## Done When

- [ ] 2 文書が作成され、worker schema・hooks 登録・skill 名・
      threshold・target 挙動が実装と一致する
- [ ] providers.md が user-facing 中心に再編され、移動先 link が
      残っている

## Verification

- [ ] worker 名・required/properties が各 schema JSON と一致
- [ ] hooks.json・skill 名・threshold default が実装と一致
- [ ] providers.md の残存内容が実装と一致・移動先が存在

## Verification results

- `docs/guides/workers.md`・`docs/guides/parent-agent-integration.md`
  を作成、`docs/guides/providers.md` を user-facing に再編。
- worker 名・tool 一覧・input schema（bulk-reader: task/paths/
  hints/maxFindings、code-writer: task/targets/references/
  constraints、doc-writer: task/documentType/targets/decisions/
  facts/references/constraints）は各 schema JSON・worker 定義と
  一致。writeScope: targets の記載も worker 定義と一致。
- hooks.json の `pre_read_code`・skill 名（cheaper-usage/
  cheaper-bulk-read/cheaper-code-write/cheaper-doc-write）・
  threshold 8192・CHEAPER_READ_TOKEN_THRESHOLD・fail-open は
  `pre-read-code.js` と一致。
- providers.md から docker compose/NVIDIA toolkit・EXP-* 実測・
  repair 内部実装（repair.ts 言及）を除去し、smoke 手順は
  CONTRIBUTING.md の「実 provider 検証」節へ集約済み。

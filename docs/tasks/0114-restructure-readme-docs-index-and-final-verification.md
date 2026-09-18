---
id: "TASK-0114"
type: "task"
status: "done"
title: "Restructure README, create docs index, and run final verification"
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
    - "docs/tasks/0113-create-workers-integration-guides-and-reorg-providers.md"
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

# TASK-0114: Restructure README, create docs index, and run final verification

## Work

### 1. `README.md` 再構成（issue の 11 項目）

- Header・Why cheaper?・Core capabilities・How it works・
  Basic usage（public CLI のみ）・Built-in workers 表・
  Configuration overview・Parent-agent integration 概要・
  Documentation link 一覧・Development gateway（CONTRIBUTING.md）・
  Project status（experimental 旨）
- main flow から外す: `mise run setup/check` 詳細・source tree
  直接実行・`packages/apm/.apm` 内部構造・bundle path・hook
  deploy path・smoke 履歴

### 2. `docs/README.md` 作成

- user guides（usage/configuration/providers/workers/
  parent-agent-integration）・contributor（../CONTRIBUTING.md）・
  architecture 記録（specs/adr/designs/discovery/plans/tasks/
  impl/test-specs/handoff）の 3 分類。各分類・各 directory に
  「何のための記録か」1 文を付ける。

### 3. 最終検証

- README・docs/README.md・全 guide の relative link 存在確認
- 全記載 command・option・worker 名・config path・env var が
  実装と一致するか rg で総当たり確認
- README に build/bundle/内部 path が無いこと
- `audit_docs.js --type all` no findings
- `mise run check`（contract 確認 + freshness）
- SPEC-0023・PLAN-0023・TASK status の同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0023-user-facing-docs-information-architecture.md`
- `docs/designs/0028-user-facing-docs-information-architecture.md`

## Dependencies

- `docs/tasks/0112-create-usage-configuration-guides-and-contributing.md`
- `docs/tasks/0113-create-workers-integration-guides-and-reorg-providers.md`
  （README の link 先が全て存在する前提）

## Affected Files

- `README.md`（modify）
- `docs/README.md`（add）

## Done When

- [ ] README が 11 項目構成で、main flow に内部手順が無い
- [ ] `docs/README.md` が 3 分類 index として機能する
- [ ] 全 relative link が実在 file を指す
- [ ] 全記載 contract が実装と一致する

## Verification

- [ ] link 検証（rg/スクリプト）
- [ ] contract 突合（command・option・worker・path・env var）
- [ ] `audit_docs.js --type all` no findings
- [ ] `mise run check` exit 0・bundle freshness clean

## Verification results

- `README.md` を 11 項目構成で再構成（header/why/capabilities/
  how-it-works/basic-usage/workers/config/integration/docs/
  development/status）。main flow に `mise`・`pnpm`・build・
  bundle・`.apm`・source path・smoke 履歴が無いことを rg で
  確認済み。
- `docs/README.md` を user/contributor/architecture の 3 分類
  index として作成。各 directory に「何の記録か」1 文を付与。
- 全 relative link（README・docs/README・CONTRIBUTING・
  5 guide）を node スクリプトで機械検証 — 全て実在、broken 0。
- contract 突合: command・option は `args.ts`/HELP と一致、
  exit code 一覧は `errors.ts` と一致、config dir・env var は
  `dirs.ts`/`cli.ts`/`pre-read-code.js` と一致、policy 実値は
  `packages/cli/policy.yaml` と一致、worker schema は各
  schema JSON と一致、hooks/skills/threshold は `hooks.json`/
  `pre-read-code.js` と一致。
- `audit_docs.js --type all`: 211 files、no findings。
- `mise run check` exit 0（lint warning 4 = 既存、typecheck、
  core 270 + runtime-pi 255 + cli 143 tests、real-provider e2e
  3 件 live pass、bundle freshness clean）。

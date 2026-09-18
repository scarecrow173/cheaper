---
id: "PLAN-034r05iHE1FbkoTVD7CTNi"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Restructure built-in writer Workers (code-writer + doc-writer)"
created: "2026-09-16"
updated: "2026-09-17"
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
    - "docs/specs/code-writer-and-doc-writer-built-in-workers.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/writer-worker-contracts.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/writer-worker-contracts.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Restructure built-in writer Workers Implementation Plan

## Goal

`cheap-writer` を `code-writer` へ直接置き換え、`doc-writer` を新規追加する。
旧名称の alias・互換 shim は残さない。Runtime には触れず、Worker Definition・
schema・CLI alias・test・doc の変更のみで完結させる。

## Scope

単一パッケージ（`packages/cli`）内の変更 + リポジトリ docs。`code-writer` 化は
rename 主体、`doc-writer` は新規 contract。両者は直列でも並行でも成立するが、
同一ファイル（`args.ts`・HELP・workers test）を触るため同一ストリームで行う。

## Upstream Documents

- `docs/specs/code-writer-and-doc-writer-built-in-workers.md`
- `docs/designs/writer-worker-contracts.md`
- `docs/discovery/v2-productization-initiative-briefing.md`

## Design Inputs

- `docs/designs/writer-worker-contracts.md`（approved）

## File Map

- `packages/cli/workers/cheap-writer.md` → `code-writer.md`（内容を code-writer 用に更新）
- `packages/cli/workers/doc-writer.md`（新規）
- `packages/cli/workers/schemas/cheap-writer.*.schema.json` → `code-writer.*.schema.json`
- `packages/cli/workers/schemas/doc-writer.{input,output}.schema.json`（新規）
- `packages/cli/src/args.ts` — `ALIAS_WORKERS` を `bulk-read`/`code-write`/`doc-write` へ
- `packages/cli/src/cli.ts` — HELP テキスト
- `packages/cli/test/{args,workers,envelope,text-format}.test.ts`、`test/e2e/exec.test.ts`
- `packages/cli/test/e2e/fixtures/cheap-write-fake.js` → `code-write-fake.js`
- `README.md` — 使い方の alias 記述
- `docs/specs/0003`、`docs/designs/0007` — superseded へ更新、各 index README 更新

## Implementation Sequence

- [ ] code-writer worker + schemas（cheap-writer から rename）と alias 切替
- [ ] doc-writer worker + schemas 新規作成
- [ ] test fixture・unit/e2e テストの新名称移行
- [ ] HELP・README・docs index・supersede 関係の更新
- [ ] `pnpm -r build` / `pnpm -r test` / `pnpm lint` / `pnpm typecheck` 全 green

## Dependencies and Risks

- なし（runtime 非依存の rename + 新規 worker 追加）
- fake stream fixture は output schema 準拠 JSON を返すだけなので worker 差替えは容易

## Task Breakdown

- TASK-034r05iHQeQpDpMK3ZMC7C: code-writer への移行（worker・schema・alias・test・fixture）
- TASK-034r05iHQeQpDpOWHdF7Wo: doc-writer の追加（worker・schema・alias・test）
- TASK-034r05iHQeQpDpQ62tRnRU: docs 更新（supersede・index・README）と全体検証

## Verification

- [ ] `cheaper code-write` / `cheaper doc-write` が worker 解決する
- [ ] `cheap-write` が `INVALID_REQUEST`（unknown command）で拒否される
- [ ] `run cheap-writer` が worker 解決失敗で拒否される
- [ ] e2e（fake stream）で code-write/doc-write の envelope + `meta.changes` が返る
- [ ] 全 baseline green

## Open Questions / Gaps

- なし

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一パッケージの直列変更）
- User approval required before dispatch: yes

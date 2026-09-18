---
id: "PLAN-034r05iHE1FbkoVrzp9sac"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Implement canonical Token Counter, count_tokens tool, and count-tokens CLI"
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
    - "docs/specs/canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/restructure-built-in-writer-workers.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/canonical-tokenizer-js-tiktoken.md"
    - "docs/designs/token-counter-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/token-counter-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Canonical Token Counter Implementation Plan

## Goal

`@cheaper/core` に js-tiktoken `o200k_base` ベースの canonical Token Counter を
実装し、`count_tokens` tool と `count-tokens` CLI の 2 入口から共有する。

## Upstream Documents

- `docs/specs/canonical-token-counter.md`
- `docs/adr/canonical-tokenizer-js-tiktoken.md`
- `docs/designs/token-counter-contract.md`

## Design Inputs

- `docs/designs/token-counter-contract.md`（approved）

## File Map

- `packages/core/package.json` — `js-tiktoken` pin 追加
- `packages/core/src/token/count.ts`（新規）+ `src/index.ts` export
- `packages/core/src/tools/vocabulary.ts` — `count_tokens` 追加
- `packages/core/test/token/count.test.ts`（新規）
- `packages/runtime-pi/src/tools/registry.ts` + `tools/count-tokens.ts`（新規）
- `packages/runtime-pi/test/tools/` — tool test
- `packages/cli/src/args.ts`・`cli.ts` — `count-tokens` command
- `packages/cli/policy.yaml`・`workers/*.md` — `count_tokens` allow/付与
- `packages/cli/test/{args,main,e2e/*}` — command test

## Implementation Sequence

- [ ] core: js-tiktoken 導入 + `countFileTokens` + vocabulary 追加 + test
- [ ] runtime-pi: `count_tokens` tool + registry 配線 + test
- [ ] cli: `count-tokens` command + args/help + e2e test
- [ ] builtin policy/worker frontmatter 更新 + 全体検証

## Task Breakdown

- TASK-034r05iHQeQpDpQjb4veqT: core Token Counter + vocabulary + js-tiktoken 導入
- TASK-034r05iHQeQpDpSeJahHce: `count_tokens` tool（runtime-pi registry + builtin 付与）
- TASK-034r05iHQeQpDpSS51up2r: `count-tokens` CLI command
- TASK-034r05iHQeQpDpUao5Hjj3: CLI/tool parity・境界 test と全体検証 + docs index

## Verification

- [ ] CLI と tool が同一 file で同一 count
- [ ] workspace escape・symlink・不存在・binary の振る舞いが test 済み
- [ ] `count-tokens` が model runtime を起動しない
- [ ] 全 baseline green

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（core→runtime-pi→cli の直列依存）
- User approval required before dispatch: yes

---
id: "PLAN-0012"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Huge-file byte cap guard and fail-closed hook"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0017-huge-file-byte-cap-guard-and-fail-closed-hook.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0017-huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../specs/0012-huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
  implemented-by:
    - "../tasks/0083-core-byte-cap-for-countfiletokens.md"
    - "../tasks/0084-cli-count-tokens-env-override-and-file-too-large-envelope.md"
    - "../tasks/0085-hook-fast-path-and-fail-closed-file-too-large-block.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0017-huge-file-byte-cap-guard-and-fail-closed-hook.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Huge-file byte cap guard and fail-closed hook Implementation Plan

## Goal

`countFileTokens` に byte cap（default 16MB・env 上書き可）を入れ、cap
超過 file を `FILE_TOO_LARGE` error として扱い、hook がそれを閾値超過
同等の block（fail-closed）に変換する。

## Scope

単一 implementation stream。core → cli → hook の依存順で 3 task に
分割するが、同一 plan 内で完結する規模。`count_tokens` tool は同一
core 実装のため変更不要。

## Upstream Documents

- `docs/specs/0012-huge-file-token-count-guard-with-fail-closed-hook-blocking.md`
- `docs/adr/0015-treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md`
- `docs/discovery/0008-huge-file-token-count-guard-and-fail-closed-hook-behavior.md`

## Design Inputs

- `docs/designs/0017-huge-file-byte-cap-guard-and-fail-closed-hook.md`

## File Map

- `packages/core/src/token/count.ts` — `DEFAULT_COUNT_MAX_BYTES`、
  `FileTooLargeError`、`maxBytes` option、stat 事前チェック
- `packages/core/src/errors.ts` — `FILE_TOO_LARGE` error code 追加
- `packages/core/src/index.ts`（または相当の export 面）— 新型の export
- `packages/core/test/token/count.test.ts` — cap の unit test
- `packages/cli/src/cli.ts` — `CHEAPER_COUNT_MAX_BYTES` 解釈・
  `FileTooLargeError` → `FILE_TOO_LARGE` map
- `packages/cli/test/` — env 上書き・error envelope の test
- `.apm/hooks/pre-read-code.js` — fast-path + FILE_TOO_LARGE block
- `packages/cli/test/e2e/pre-read-hook.test.ts` — hook e2e 追加
- `.apm/hooks/cli/bin/cheaper.mjs` — `pnpm bundle` 再生成（committed
  生成物、freshness 検証対象）

## Implementation Sequence

- [ ] core: `FileTooLargeError`・`maxBytes`・stat チェック・
      `FILE_TOO_LARGE` code・export
- [ ] cli: env 解釈・error map
- [ ] hook: fast-path・envelope error code 検査・guidance
- [ ] test: core unit / cli / hook e2e
- [ ] `pnpm bundle` 再生成 + build/lint/typecheck/test 全検証

## Dependencies and Risks

- `.apm/hooks/cli/bin/cheaper.mjs` は committed 生成物。cli.ts 変更を
  含むため `pnpm bundle` 再生成が必須（CI freshness check 対象）
- hook の fast-path は `tokens ≤ bytes` の不変条件に依存（encoding 非
  依存で成立）
- false positive（高圧縮率 file）は許容済み（ADR-0015）

## Task Breakdown

- TASK-A: core byte cap + unit test
- TASK-B: cli env 解釈 + FILE_TOO_LARGE envelope + test（depends on A）
- TASK-C: hook fast-path + fail-closed + e2e + bundle 再生成（depends on B）

## Verification

- [ ] `pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck`
- [ ] cap 超過 file で `count-tokens` が FILE_TOO_LARGE envelope・非0
      exit・OOM しない
- [ ] `CHEAPER_COUNT_MAX_BYTES` 上書き・不正値フォールバック
- [ ] hook e2e: cap error → exit 2 + guidance、fast-path → spawn なし
      exit 0
- [ ] bundle freshness（`pnpm bundle` 再生成との一致）

## Open Questions / Gaps

- なし（設計で確定済み）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none（TASK-A → B → C の直列）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- bundle 再生成を task 内に明示（freshness check で落ちないため）
- hook fast-path の不変条件（tokens ≤ bytes）が ADR-0015 に記録済み

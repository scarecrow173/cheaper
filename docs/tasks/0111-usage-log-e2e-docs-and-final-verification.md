---
id: "TASK-0111"
type: "task"
status: "done"
title: "Usage-log e2e assertions, docs, bundle regen, status sync, final verification"
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
    - "docs/plans/0022-delegation-usage-measurement-log.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0022-delegation-usage-measurement-log.md"
    - "docs/tasks/0110-implement-usage-log-writer-and-cli-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0027-delegation-usage-measurement-log.md"
    - "docs/specs/0022-delegation-usage-measurement-log.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0022-delegation-usage-measurement-log.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0111: e2e assertions, docs, bundle regen, final verification

## Work

PLAN-0022 の仕上げ task。e2e での記録 assert・ドキュメント・
bundle 再生成・status 同期・全量検証を行う。

### 1. e2e assert 追加

- fake-stream e2e（`exec.test.ts` または専用 test）:
  `CHEAPER_CONFIG_DIR` を tmpdir に向けた consumer run で
  `usage.jsonl` に `kind:"run"` record があることを assert。
  `count-tokens` 実行で `kind:"count"` record があることを
  assert。
- `real-provider-ollama.test.ts`: 各 worker run 後に
  `usage.jsonl` へ `kind:"run"`・`model.provider==="ollama"`・
  `usage.input > 0` の record が append されたことを assert
  （既存の CHEAPER_CONFIG_DIR fixture をそのまま使う）。

### 2. `AGENTS.md` 記載

計測 log（`<userConfigDir>/usage.jsonl`、record shape、
`CHEAPER_CONFIG_DIR` で隔離可能、best-effort）を 2〜3 行で
記載。

### 3. bundle 再生成

`node scripts/bundle-cli.mjs` で bundled CLI を再生成し、
`git diff` で `cheaper.mjs` に usage-log が含まれることを確認
（freshness gate 対象のため必須）。

### 4. status 同期

- SPEC-0022 → `implemented`
- PLAN-0022 → `completed`
- TASK-0110・0111 → `done`
- 各 index の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0022-delegation-usage-measurement-log.md`
- `docs/designs/0027-delegation-usage-measurement-log.md`
- `docs/specs/0022-delegation-usage-measurement-log.md`

## Dependencies

- `docs/tasks/0110-implement-usage-log-writer-and-cli-wiring.md`
  （writer + 配線が実装済みである前提）

## Affected Files

- `packages/cli/test/e2e/exec.test.ts`（modify）
- `packages/cli/test/e2e/real-provider-ollama.test.ts`（modify）
- `AGENTS.md`（modify）
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（generated）
- status 同期対象の各 doc・index

## Done When

- [ ] fake-stream e2e で run・count record の append を assert
- [ ] real-provider e2e で実測 record を assert
- [ ] AGENTS.md に計測 log の記載がある
- [ ] bundle 再生成済みで freshness clean
- [ ] SPEC-0022/PLAN-0022/TASK-0110..0111 と各 index が同期

## Verification

- [ ] `pnpm test` 全件 pass（Ollama 稼働時は real-provider 含む）
- [ ] `mise run check` exit 0
- [ ] `audit_docs.js --type all` が no findings

## Verification results

（未実施）

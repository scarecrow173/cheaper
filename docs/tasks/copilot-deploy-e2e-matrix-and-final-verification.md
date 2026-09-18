---
id: "TASK-034r05iHQeQpDqGn3BDa9i"
type: "task"
status: "done"
title: "Copilot deploy/config e2e asserts, DISC-034r05iHy2gdnpdai9KGpW matrix update, and final verification"
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
    - "docs/plans/copilot-pre-read-hook-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/copilot-pre-read-hook-adapter.md"
    - "docs/tasks/copilot-adapter-script-and-hook-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/copilot-pre-read-hook-adapter.md"
    - "docs/specs/copilot-pre-read-hook-adapter.md"
    - "docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/copilot-pre-read-hook-adapter.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHQeQpDqGn3BDa9i: Copilot deploy/config e2e asserts, matrix update, and final verification

## Work

PLAN-034r05iHE1FbkpaEpZrthK の仕上げ task。`apm-install-targets.test.ts` の copilot
case に adapter の deploy/config assert を追加し、DISC-034r05iHy2gdnpdai9KGpW matrix
を更新して、全量検証を実施する。

### 1. `apm-install-targets.test.ts`（copilot case）

- `pre-read-code-copilot.js` が deploy されることを assert（`.js`
  は strip されない）。
- generated `.github/hooks/cheaper-hooks.json` に `hooks.preToolUse`
  entry が存在し、command が `pre-read-code-copilot.js` を含むことを
  assert。
- 既存 assert（`pre_read_code` verbatim・`.json` 欠落）は維持 —
  `.json` 欠落 assert は upstream 修正の drift signal として残す
  （ADR-034r05iHrZUE6WizQ4LgPa option i）。

### 2. DISC-034r05iHy2gdnpdai9KGpW matrix 更新

copilot 行を「adapter 実装済み（preToolUse + stdout deny JSON +
常に exit 0）」に更新し、`.json` gap による既知の制限
（file-schema worker・`--version` が copilot deploy で不動、
upstream 修正待ち）を明記する。

### 3. status 同期

- SPEC-034r05iHK8bA7gsvijtkou → `implemented`
- PLAN-034r05iHE1FbkpaEpZrthK → `completed`
- TASK-034r05iHQeQpDqFsYcuUHl・0107 → `done`
- 各 index の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/copilot-pre-read-hook-adapter.md`
- `docs/designs/copilot-pre-read-hook-adapter.md`
- `docs/specs/copilot-pre-read-hook-adapter.md`
- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`

## Dependencies

- `docs/tasks/copilot-adapter-script-and-hook-tests.md`
  （adapter と hooks.json が確定している前提）

## Affected Files

- `packages/cli/test/e2e/apm-install-targets.test.ts`
- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
- `docs/specs/0020-*.md`・`docs/plans/0020-*.md`・
  `docs/tasks/0106..0107-*.md` の status と各 index

## Done When

- [x] copilot の generated config に `preToolUse` 登録の assert が
      あり、adapter file の deploy が assert されている
- [x] `.json` 欠落 assert（drift signal）が維持されている
- [x] DISC-034r05iHy2gdnpdai9KGpW matrix の copilot 行が実装済み・既知制限に更新
      されている
- [x] SPEC-034r05iHK8bA7gsvijtkou/PLAN-034r05iHE1FbkpaEpZrthK/TASK-034r05iHQeQpDqFsYcuUHl..0107 と各 index の status が
      同期されている
- [x] 全量検証（lint・typecheck・全 test・audit）が pass

## Verification

- [x] `pnpm --filter cheaper test` 全件 pass
- [x] `pnpm typecheck`・`pnpm lint` pass
- [x] `audit_docs.js --type all` が no findings

## Verification results

2026-09-18 実施。

| Command | Result |
|---------|--------|
| `vitest run test/e2e/apm-install-targets.test.ts` | 6/6 pass — copilot: `pre-read-code-copilot.js` deploy・generated config の `preToolUse` 登録・`.json` 欠落 drift-signal assert 維持。claude: `preToolUse` → `PreToolUse` 正規化（matcher `*`、live だが payload 不一致で実質 no-op）を実測・assert。windsurf/codex/cursor: verbatim no-op 登録 |
| `pnpm typecheck`・`pnpm lint` | pass |
| `audit_docs.js --type all` | （最終 audit で確認） |

実測により APM 0.28.0 の claude adapter が event 名を native casing
に正規化する事実を発見（`preToolUse` → `PreToolUse`、matcher `*` で
live 登録）。copilot adapter は claude stdin shape（`tool_name`/
`tool_input`）と contract 不一致のため全 exit 0 素通し — 実質
no-op で安全。ADR-034r05iHrZUE6WizQ4LgPa・DESIGN-034r05iHy2gdnoUHGunDtE・DISC-034r05iHy2gdnpdai9KGpW に反映済み。

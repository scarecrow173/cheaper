---
id: "PLAN-0020"
type: "plan"
status: "completed"
title: "Copilot pre-read hook adapter — implementation plan"
test-spec-skip: "Verified by package e2e test suites (stdin-injection and apm-install-targets); no dedicated test-spec doc authored."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0020-copilot-pre-read-hook-adapter.md"
    - "../designs/0025-copilot-pre-read-hook-adapter.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0025-copilot-pre-read-hook-adapter.md"
  implemented-by: []
  depends-on:
    - "../adr/0023-copilot-adapter-separate-script-never-nonzero-exit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0025-copilot-pre-read-hook-adapter.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "medium"
    verification-policy: "task-verified"
---

# Copilot pre-read hook adapter — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `packages/apm/.apm/hooks/pre-read-code-copilot.js` | add | copilot adapter（self-contained、常に exit 0） |
| `packages/apm/.apm/hooks/hooks.json` | modify | `preToolUse` event 登録を追加 |
| `packages/cli/test/e2e/copilot-pre-read-hook.test.ts` | add | adapter の stdin 注入検証 |
| `packages/cli/test/e2e/apm-install-targets.test.ts` | modify | copilot deploy/config assert 追加 |
| `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md` | modify | copilot 行を adapter 実装済み・既知制限に更新 |

## Tasks

- **TASK-0106**: adapter script + hooks.json 登録 + stdin 注入 test
  （TDD: test 先 → 実装 → green）
- **TASK-0107**: apm-install-targets e2e の copilot assert 追加、
  DISC-0009 matrix 更新、最終検証（lint/typecheck/test/audit）

## Dependency Graph

```
TASK-0106 (adapter + tests)
   └─> TASK-0107 (deploy e2e + matrix + final verification)
```

## Risk Register

- claude の `PreToolUse`（PascalCase）と camelCase `preToolUse` の衝突:
  verbatim deploy で全 target config に登録されるため、case 非区別
  runtime なら claude で誤発火しうる → adapter は非 copilot shape
  payload を exit 0 素通しするため無害（ADR-0023 Open Question に記録）。
- copilot `view` の toolArgs key が `path` 系でない可能性:
  `path`/`file_path`/`filePath` の複数 key 許容で吸収。実機未検証。
- hooks.json 構造変更が windsurf 登録を壊す: pre_read_code entry は
  無変更、既存 test で回帰検知。

## Rollback

- `hooks.json` の `preToolUse` entry 除去 + adapter file 削除で copilot
  は従来の no-op 状態へ即時復帰（windsurf 側は無影響）。

## Verification Matrix

| Requirement | Verification |
|---|---|
| toolArgs normalize（object/string） | copilot-pre-read-hook.test.ts の両 shape block test |
| 常に exit 0（fail-closed 安全） | never-nonzero test 群（全異常経路で status===0 assert） |
| block = stdout deny JSON | deny JSON parse + permissionDecision/reason assert |
| block 条件 parity | threshold・FILE_TOO_LARGE・fast-path test（windsurf 流儀） |
| deploy/config 登録 | apm-install-targets copilot case assert |
| .json gap 方針 | ADR-0023 + 既存 expected-failure assert 維持 + matrix 明記 |
| 回帰 | 既存 test 全件・lint・typecheck |

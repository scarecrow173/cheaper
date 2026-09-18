---
id: "PLAN-034r05iHE1FbkoZibpg86T"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "cheaper auth command (provider login absorbed into the CLI)"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/auth-command.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/auth-command.md"
  implemented-by:
    - "../tasks/auth-command.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/auth-command.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# PLAN-034r05iHE1FbkoZibpg86T: cheaper auth command

## File map

- `packages/cli/src/auth.ts` (new) — status/login/logout + terminal interaction
- `packages/cli/src/args.ts` — `auth` サブコマンド parse
- `packages/cli/src/cli.ts` — dispatch + help
- `packages/cli/test/` — args + status/logout envelope + interaction unit test
- `docs/guides/providers.md` — pi 不要の手順へ更新
- bundle 再生成

## Task

TASK-034r05iHQeQpDqapXCLs3G: 実装 + 検証 + index 更新

## Verification

unit test（status/logout/unknown provider/非 TTY select error）+
bundle 再生成 + `auth status` 手動確認。実 OAuth login はユーザー操作で検証。

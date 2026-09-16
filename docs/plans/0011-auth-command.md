---
id: "PLAN-0011"
type: "plan"
status: "completed"
title: "cheaper auth command (provider login absorbed into the CLI)"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0016-auth-command.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0016-auth-command.md"
  implemented-by:
    - "../tasks/0082-auth-command.md"
  depends-on: []
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
metadata: {}
---

# PLAN-0011: cheaper auth command

## File map

- `packages/cli/src/auth.ts` (new) — status/login/logout + terminal interaction
- `packages/cli/src/args.ts` — `auth` サブコマンド parse
- `packages/cli/src/cli.ts` — dispatch + help
- `packages/cli/test/` — args + status/logout envelope + interaction unit test
- `docs/guides/providers.md` — pi 不要の手順へ更新
- bundle 再生成

## Task

TASK-0082: 実装 + 検証 + index 更新

## Verification

unit test（status/logout/unknown provider/非 TTY select error）+
bundle 再生成 + `auth status` 手動確認。実 OAuth login はユーザー操作で検証。

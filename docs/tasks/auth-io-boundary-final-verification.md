---
id: "TASK-AZSy7cWtCdKaL5D2ro8TWy"
type: "task"
status: "todo"
title: "Auth I/O boundary final verification and status sync"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/auth-io-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/auth-io-boundary.md"
    - "docs/tasks/auth-io-boundary-prompt-routing-and-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/auth-io-boundary.md"
    - "docs/specs/auth-io-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/auth-io-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Auth I/O boundary final verification and status sync

## Work

PLAN-zDQIdChPXY1lrGJyzZTiQc の final verification task。

- `mise run check`（lint・version・typecheck・test・bundle freshness）を
  実行し exit 0 を確認する。
- `auth.ts` の prompt 経路に `process.stderr` 直接参照が残らないことを
  確認する。
- 新規 TTY select/text テストと既存 auth テスト群の pass を確認する。
- 実装内容を Implementation Record（`docs/impl/ir/`）として記録し、
  SPEC-WdXXEoaAmbOafvuIpM6FFT の status を `implemented` へ同期する。
- issue #36 の Acceptance Criteria をすべて checked にする。

## Acceptance

- `mise run check` が pass する。
- Implementation Record が作成され、SPEC status が `implemented`。
- SPEC の Acceptance Criteria がすべて checked。

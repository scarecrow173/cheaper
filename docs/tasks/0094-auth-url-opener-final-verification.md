---
id: "TASK-0094"
type: "task"
status: "todo"
title: "Auth URL opener final verification and status sync"
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
    - "docs/plans/0015-auth-url-opener-seam.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0015-auth-url-opener-seam.md"
    - "docs/tasks/0093-auth-url-opener-seam-and-test-isolation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0020-auth-url-opener-seam.md"
    - "docs/specs/0015-auth-tests-do-not-launch-os-browser.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0015-auth-tests-do-not-launch-os-browser.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Auth URL opener final verification and status sync

## Work

PLAN-0015 の最終検証 task。TASK-0093 の実装を全体検証し、ライフ
サイクル status を同期する。

### 1. 検証

- `pnpm lint` / `pnpm typecheck` / `pnpm test` を実行し全件成功を
  確認する。
- `pnpm test` 実行中に OS の URL opener（`rundll32` / `open` /
  `xdg-open`）が spawn されないことを、テスト側の spy 構造と
  `--method device` の fake event から確認する。

### 2. Status 同期

検証成功後、以下を更新する。

- `docs/specs/0015-auth-tests-do-not-launch-os-browser.md` の
  `status` を `implemented` にし、index を同期する。
- `docs/plans/0015-auth-url-opener-seam.md` の `status` を
  `completed` にし、index を同期する。
- 本 task と TASK-0093 の `status` を `done` にし、index を同期する。

## Acceptance

- lint / typecheck / test がすべて成功する。
- SPEC-0015 の Acceptance Criteria が満たされる。
- ドキュメント status と index が実態と一致する。

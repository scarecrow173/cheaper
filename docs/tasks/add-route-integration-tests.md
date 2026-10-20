---
id: "TASK-034Jg6IVuYRxFj2Zn3JPI9"
type: "task"
status: "todo"
title: "Add route integration and compile-contract tests"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
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
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
    - "docs/tasks/add-route-cli-wiring.md"
    - "docs/tasks/add-builtin-route-resources.md"
    - "docs/tasks/add-hook-entry-route-loading.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-route-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route integration and compile-contract tests

## Work

`tests/integration/route.rs` — route resolve/run/logs の CLI
contract、hook → route run e2e(resolve 一致・exit mirror・cwd/env
保持・fallback verdict)。`compile_contracts` へ `when` 系を
追加し、docs/guides へ route 利用法を追記。spec R1-R40・ADR
D1-D13 の verification mapping を完走させる。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase E)

## Dependencies

- docs/tasks/add-route-cli-wiring.md
- docs/tasks/add-builtin-route-resources.md
- docs/tasks/add-hook-entry-route-loading.md
- docs/tasks/add-hook-rewrite-target-rendering.md
- docs/tasks/add-route-dedup-fingerprint.md

## Affected Files

- crates/cheaper-cli/tests/integration/route.rs
- crates/cheaper-workflow/tests/compile_contracts/
- docs/guides/

## Done When

- [ ] hook eval resolve と `route resolve` が一致する
- [ ] hook payload が direct `route run` と同一 contract で処理
- [ ] e2e で exit/cwd/env-prefix/bounded output が保持される
- [ ] `mise run check` 全 green

## Verification

- [ ] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route::`
- [ ] `mise run check`

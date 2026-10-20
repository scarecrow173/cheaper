---
id: "TASK-034Jg6lAjcLYbfDm0Y370r"
type: "task"
status: "done"
title: "Add route resolver payload codec and command preflight seam"
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
    - "docs/tasks/add-route-declaration-loader.md"
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
    - "docs/test-specs/route-resolution.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route resolver payload codec and command preflight seam

## Work

`route::resolve` — `ResolveInput{argv,cwd,routes,command_policy}`
純粋評価(self-bypass → INTERACTIVE_COMMANDS → viability)、
args/exec_args 区別、env-prefix skip、`route resolve` dry-run arm。
`route::payload` — base64url codec + schema + allowlist subsetting
の単一 owner。cheaper-tools に `pub fn check_command_execution`
(仮)公開 seam を追加し `command::execute` と predicate 共有。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase B2)

## Dependencies

- docs/tasks/add-route-declaration-loader.md

## Affected Files

- crates/cheaper-cli/src/route/resolve.rs
- crates/cheaper-cli/src/route/payload.rs
- crates/cheaper-tools/src/tools/command/mod.rs
- crates/cheaper-cli/src/args.rs / cli.rs (resolve arm)

## Done When

- [ ] `route resolve --command/--argv` が viability 全分岐を返す
- [ ] payload encode/decode round-trip と allowlist subset が pass
- [ ] check_command_execution が command::execute 受理判定と一致

## Verification

- [ ] route resolve contract test(各 viability 分岐)
- [ ] payload codec unit test・preflight 一致 test

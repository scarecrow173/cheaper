---
id: "TASK-034ZiG2kRrXqHk9bVf8pQs"
type: "task"
status: "done"
title: "Add workflow list CLI command"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/plans/implement-workflow-catalog-and-list.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/add-workflow-catalog-manifest-read-and-diagnostics.md"
  blocks:
    - "docs/tasks/document-workflow-catalog-contract-and-usage.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-catalog-manifest-and-list-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Add workflow list CLI command

## Work

`cheaper workflow list` を read-only utility command として実装する。
`args.rs` に `WorkflowListCliArgs { cwd, format, quiet, limit }` と
`ParsedCli::WorkflowList` を追加し、`workflow` arm で `list`
(positionals==2)を受理する。allowed option は
`cwd`/`format`/`quiet`/`log`/`log-format`/`limit` のみ。新規
`workflow_list.rs` の `execute_list` は
`resolve_workspace_layout` → `load_policy` →
`catalog::read_catalog` → `utility_success_envelope` で
`{catalog:{path,present}, workflows, diagnostics, truncated}` を投影し、
`cli.rs` に dispatch arm・`command_name="workflow-list"`・HELP 行を
追加する。`interface|run` の grammar・envelope・exit は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- plan: `../plans/implement-workflow-catalog-and-list.md`
- design: `../designs/workflow-catalog-manifest-and-read-only-list.md`
- test-spec: `../test-specs/workflow-catalog-manifest-and-list-contract.md`

## Dependencies

- `add-workflow-catalog-manifest-read-and-diagnostics.md`(catalog pub API)。

## Affected Files

- `crates/cheaper-cli/src/args.rs`
- `crates/cheaper-cli/src/workflow_list.rs` (new)
- `crates/cheaper-cli/src/lib.rs`
- `crates/cheaper-cli/src/cli.rs`
- `crates/cheaper-cli/tests/workflow.rs`
- `crates/cheaper-cli/tests/integration/args.rs`

## Done When

- [ ] `workflow list` が `worker:null` の utility envelope で
  `{catalog, workflows, diagnostics, truncated}` を返し exit 0。
- [ ] no-catalog workspace で `present:false` の空一覧。
- [ ] malformed manifest が error envelope + 非 0 exit。
- [ ] `--limit` で `truncated:true`。
- [ ] `list` への他 command option が INVALID_REQUEST。
- [ ] `interface|run` の既存挙動・HELP・`command_name` が変わらない。
- [ ] usage record を残さず・file 書込み・compiler/model 起動なし。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test workflow`
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::`

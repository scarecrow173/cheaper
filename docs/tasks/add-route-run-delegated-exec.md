---
id: "TASK-034Jg6GTgeZjILEwH5ZvE6"
type: "task"
status: "todo"
title: "Add route run delegated workflow execution"
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
    - "docs/tasks/add-route-resolver-and-payload.md"
    - "docs/tasks/add-route-context-scope-options.md"
    - "docs/tasks/add-proc-output-tee.md"
    - "docs/tasks/add-route-log-store.md"
    - "docs/tasks/add-route-usage-records.md"
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
    - "docs/test-specs/route-run-delegated-exec.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route run delegated workflow execution

## Work

`route::run` — payload decode → re-resolve/revalidate(fail-closed、
raw command 実行禁止)→ env-prefix 再導出 → route context 合成
(resolved RuntimePolicy 単一 grant 投影・extra_env/exec_cwd/
accept_any_exit_code/log_tee)→ 宣言層 root の workflow load +
payload cwd の resource inference で in-process `run_workflow` →
run timeout(grant.timeout+120s)→ `execResult.details.exitCode`
mirror・postprocessFailures・stdout raw compact emit。route usage
record を記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase D1)

## Dependencies

- docs/tasks/add-route-resolver-and-payload.md
- docs/tasks/add-route-context-scope-options.md
- docs/tasks/add-proc-output-tee.md
- docs/tasks/add-route-log-store.md
- docs/tasks/add-route-usage-records.md
- docs/tasks/add-command-grant-exitcodes.md

## Affected Files

- crates/cheaper-cli/src/route/run.rs
- crates/cheaper-cli/src/workflow_run.rs
- crates/cheaper-cli/src/run.rs (route context 合成参照)

## Done When

- [ ] revalidation 失敗が raw command を実行せず fail する
- [ ] execResult.details.exitCode が process exit へ mirror される
- [ ] post-processing 失敗が exit/output を維持して記録される
- [ ] env-prefix 再導出 precedence(env-prefix > ambient > grant env)
- [ ] route usage record が append される

## Verification

- [ ] integration test: exit mirror・fail-closed・postprocessFailures

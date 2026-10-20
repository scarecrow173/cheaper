---
id: "TSPEC-034Jg5Kw1kunKPtgczOZ9L"
type: "test-spec"
status: "draft"
title: "Route run revalidation and delegated workflow execution"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route run revalidation and delegated workflow execution

## Purpose

`route run` が payload decode → re-resolve/revalidate → in-process
workflow 実行 → exit mirror の全経路を spec R30-R34 通り fail-closed
に実行することを pin する。

## Feature

`cheaper route run --payload <token>` が route context を合成し
declared workflow を in-process で起動する。

## Rules

- payload decode 直後に argv・cwd・routes を再 resolve し、viability
  を再評価する。失敗時は原 command を実行せず non-zero で fail。
- env-prefix 値は payload.command から再導出し、precedence は
  env-prefix > ambient capture > grant env。
- resolved RuntimePolicy.command.grants を対象 grant 1 件へ投影して
  から Shared/invocation を構築する(invocation 層注入ではない)。
- route context は extra_env/env-prefix・exec_cwd=cwd・
  accept_any_exit_code・log_tee=<runId>.log.tmp を注入する。
- workflow file は宣言層 dir を load root、resource inference の
  workspace_root は payload cwd とする 2 root 区別。
- run timeout = grant.timeout + post-processing headroom(+120s)。
- execResult.details.exitCode を process exit code へ mirror し、
  post-processing 失敗は postprocessFailures に記録し exit を保つ。
- stdout は compact result の raw emit(generic envelope を bypass)、
  env 値・credential を含めない。

## Examples

- exit 7 で終了する routed command → route run は 7 で終了。
- payload の routes 版が古い → revalidate で不整合検出し fail。
- workflow analyze 失敗 → execResult の exit/output を維持。

## Guarantees

- revalidation 失敗時に原 raw command は絶対に実行されない。
- post-exec 失敗でも exit code・bounded output・logRef が保存される。

## Non-goals

- subprocess workflow 起動、recursive worker delegation。

## Risk

- revalidate 省略は stale payload での grant 迂回を許す。

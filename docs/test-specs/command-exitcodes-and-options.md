---
id: "TSPEC-034Jg5UNPJ1sKpPwYxH8L3"
type: "test-spec"
status: "draft"
title: "Command grant exitCodes and route context options"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Command grant exitCodes and route context options

## Purpose

`command.grants.<name>.exitCodes` schema・route context scope
option(extra_env/exec_cwd/accept_any_exit_code/log_tee)・resolved
policy 単一 grant 投影が design 通り動くことを pin する。

## Feature

command tool が grant 宣言 exitCodes に応じて非 0 exit を result
成功として返し、route 経由では全 exit を受理する。

## Rules

- exitCodes 未宣言時 default `[0]` で現行挙動を維持する。
- 宣言 exitCodes に含まれる exit は error ではなく result(exitCode
  details 付き)として返る。
- accept_any_exit_code 有効時は spawn/timeout/abort 以外の全 completed
  exit を result 化する。
- extra_env は command tool child env へ注入され log/usage/worker
  input には出力しない。
- exec_cwd は prepare_cwd の resolved cwd を override する。
- 単一 grant 投影は Shared/invocation 構築前に resolved
  RuntimePolicy.command.grants map を置き換え、worker parity check を
  維持する。
- check_command_execution は grant 解決・pin・denied・cwd check を
  command::execute と同一 predicate で公開する。

## Examples

- `exitCodes: [0,1]` の grant で exit 1 → state=completed result。
- routed scope で exit 7 → accept_any_exit_code により result 化。
- 投影外 grant を input.name で参照 → job 失敗。

## Guarantees

- route 経由の非 0 exit が workflow error に誤変換されない。

## Non-goals

- grant schema の他 field 拡張。

## Risk

- invocation 層注入は parity check を壊し worker scope と不整合。

---
id: "IMPL-034JhsoS4CooCbWSXWcx77"
type: "impl"
status: "completed"
title: "Add command grant exitCodes and nonzero-exit result reporting"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/monitor.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-command-grant-exitcodes.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add command grant exitCodes and nonzero-exit result reporting

## Summary

`command.grants.<name>.exitCodes`(default `[0]`)を policy schema・
resolved `CommandGrant` へ追加し、`ProcLimits.accepted_exit_codes`
(`Option<Vec<i64>>`)経由で proc monitor が宣言 exit を completed
result として返すようにした。route scope の
`accept_any_exit_code`(A3)向けに `None` = 全 exit 受理の経路も
確保。

## Context

Task `add-command-grant-exitcodes`(plan Phase A1、design の
exit mirror 基盤)。routed 実行で grant 未宣言の非 0 exit が
workflow error に誤変換されないための前提機構。

## Implementation

### Changes

- `cheaper-core/policy/command.rs`: grant schema へ `exitCodes`
  (integer 0..u32::MAX、maxItems 64)追加。`CommandGrant.exit_codes:
  Vec<i64>` + resolve で default `[0]` 畳み込み。
- `cheaper-tools/proc`: `ProcLimits.accepted_exit_codes:
  Option<Vec<i64>>` 追加。`None` は全 completed exit 受理
  (`exit_code` を持たない signal 終了は常に失敗)。
- `monitor.rs`: `exit_code == Some(0)` 固定を受理集合 check へ
  置き換え。`state`/`exitCode` details・error 分岐・完了 text が
  実 exit code を報告(`exitCode=0` 固定から `exitCode={code}` へ、
  exit 0 の表示は不変)。
- `command/mod.rs`: `accepted_exit_codes:
  Some(grant.exit_codes.clone())`。
- `process/mod.rs`: `Some(vec![0])` で従来挙動を維持。
- golden fixture `policy/load.json`: resolved grant expected に
  `exitCodes: [0]` を追加(4 grant)。

### Decisions During Implementation

- exit code 型は `i64`(`status_parts` の `Option<i64>` に一致、
  Windows u32 を包摂)。schema 側は 0..u32::MAX の integer。
- `accept_any_exit_code` 自体は A3 の scope option であり、A1 では
  `Option` の `None` 分岐のみ機構として用意し command は
  `Some(grant.exit_codes)` を渡す。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-tools --test proc_command` — 27 pass
  (新規 3: `declared_exitcodes_report_completed_result`・
  `undeclared_nonzero_exit_remains_failure`・
  `invalid_exitcodes_rejected_at_policy_resolution`)。
- `cargo test -p cheaper-tools` — 全 suite pass(259+10+11+27+9+13+9+7)。
- `cargo test -p cheaper-core --test golden` — 22 pass(fixture 更新後)。

## Risks

- 完了 text の `exitCode={code}` 化は exit 0 で表示不変だが、
  exit code を非 0 で受理する consumer は text 側も変化する点に
  留意。

## Follow-ups

- A3 `add-route-context-scope-options` で
  `accepted_exit_codes: None`(accept any)への切替 option を接続。

---
id: "TASK-034EOHORutYi73P8DaumoH"
type: "task"
status: "done"
title: "T3: command tool 実装と catalog/registry/invocation 配線"
created: "2026-10-12"
updated: "2026-10-13"
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
    - "docs/plans/implement-policy-gated-command-tool.md"
  implemented-by:
    - "docs/impl/ir/command-tool-implementation-and-catalog-registry-invocation-wiring.md"
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
    - "docs/tasks/command-core-policy-and-propagation.md"
    - "docs/tasks/proc-extraction.md"
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
    - "docs/test-specs/policy-gated-command-tool.md"
  references: []
  defers: []
  deferred-by: []
---

# T3: command tool 実装と catalog/registry/invocation 配線

## Scope

T1 が置いた deny-stub を実装へ差し替え、`command` tool を catalog `NAMES` /
registered set に登場させる。registry の信頼境界 gate（nonempty-grants
fail-closed・writeScope 拒否・pin map）と invocation の `inner.command` 格納 +
declared-shape parity check を実装する。

### 所有ファイル

- `crates/cheaper-tools/src/tools/command/mod.rs` — deny-stub を実装へ差し替え:
  grant 名を埋め込んだ tool description、grant 解決（`resolve_command_grant`）+
  argv `ArgPattern` match + metachar check（`argv_shell_re`）+ cwd prepare +
  pin 済み executable の spawn 前再検証の execute glue、
  `command_operation_declaration`（Exclusive claim + 独自 pool `"command"` +
  `command_grant_resources` の保守的 manifest）への差し替え。実行は T2 の
  `proc::run` を経由し、result は `{grant, argv, state, exitCode, signal,
  durationMs, stdout, stderr}`。
- `crates/cheaper-tools/src/catalog.rs` — `NAMES` に `Command` を追加。
  `proc_runtime.rs:119-121` の `exposes_eight_schemas` → `exposes_nine_schemas`
  同時更新（NAMES 追加と同一 task）。
- `crates/cheaper-tools/src/registry.rs` — nonempty-grants gate（`names` に
  `Command` を含む `create_tool_runtime` で grants が空なら fail-closed）、
  writeScope × command 拒否、pin map 構築（pin は `create_tool_runtime` 境界の
  per-scope、invocation をまたぐ cache なし）、`ToolCtx.command`（resolved +
  pin map）、`is_process` → `is_managed` 一般化（:845 — Process|Command の
  managed tracking）、`execute_inner` の declaration arm（:810-824）。
- `crates/cheaper-tools/src/invocation.rs` — `create_invocation_tool_runtime` で
  `inner.command` を `resolve_command_policy` により格納 + declared-shape
  parity check（`options.process` 同等の re-resolve → `PartialEq`、:210-216
  先例。pin は行わない — `names` が無いため declared-shape resolve + parity
  のみ）。
- `crates/cheaper-tools/tests/proc_runtime.rs` — `exposes_nine_schemas` 更新
  （catalog NAMES 変更と同コミット）。
- `src/access.rs`・`src/scheduler.rs` — 無変更（`is_mutating_tool`・pool name で
  自動追従することを確認するのみ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T3）
- `docs/designs/policy-gated-command-tool.md` — §(b) pin・§(c) writeScope・
  §(f) pool・§(g) registry plumbing
- `docs/specs/policy-gated-command-tool.md` — AC2/AC3/AC4
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- `docs/tasks/command-core-policy-and-propagation.md`（T1 —
  `resolve_command_policy`・deny set・`ToolName::Command`・options field/
  deny-stub arm）
- `docs/tasks/proc-extraction.md`（T2 — `proc::run`・shared prepare・
  `resolve_process_executable` 移設先）

## Steps

1. `tools/command/` の deny-stub を実装へ差し替える（description・grant 解決・
   argv match・metachar・cwd prepare・pin 再検証・`proc::run` 実行）。
2. `command_operation_declaration` を実装し、registry の declaration arm と
   `is_process` → `is_managed` 一般化を行う。
3. registry に nonempty-grants gate・writeScope 拒否・pin map 構築・
   `ToolCtx.command` を実装する。
4. invocation に `inner.command` 格納 + declared-shape parity check を実装する。
5. catalog `NAMES` に `Command` を追加し、`exposes_nine_schemas` を更新する。

## Done When

- [x] `command` が catalog `NAMES` / registered set に登場し、grant 経由で
  実行可能（deny-stub が完全に置換済み）
- [x] grants 空で `names` に `Command` を含む runtime が `POLICY_VIOLATION` で
  fail-closed、`names` に含まない runtime は dormant で構築できる
- [x] writeScope × command が fail-closed reject される
- [x] `{name,args}` 以外の input key が `INVALID_INPUT` で reject される
- [x] declared-shape parity check が `process` と同型で動作する

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools` exits 0（全 test target を
  compile+run — `tests/proc_invocation.rs` 等の integration target 込み）
- [x] `mise exec -- cargo test -p cheaper-tools --test proc_invocation` exits 0
  （gate/parity focused）

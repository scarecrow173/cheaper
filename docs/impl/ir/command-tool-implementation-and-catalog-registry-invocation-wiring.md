---
id: "IMPL-034EQ1gcdKphCuBrbPf3c0"
type: "impl"
status: "completed"
title: "command tool implementation and catalog registry invocation wiring"
created: "2026-10-12"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - {"type":"code","path":"crates/cheaper-tools/src/tools/command/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/catalog.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/registry.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/invocation.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/proc/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/proc/monitor.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/proc/executable.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/process/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_runtime.rs"}
      - {"type":"doc","path":"docs/tasks/command-tool-and-registry.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/impl/ir/command-tool-implementation-and-catalog-registry-invocation-wiring.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/command-tool-and-registry.md"
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

# command tool implementation and catalog registry invocation wiring

## Summary

T3。T1 が置いた `command` deny-stub を grant 経由実行の実装へ差し替え、
catalog `NAMES` / registered set に 9 番目の canonical tool として登場
させた。execute glue は grant 解決(`resolve_command_grant`)→ argv
`ArgPattern` match(`argv_shell_re` metachar 事前 check)→ `proc::prepare_cwd`
→ pin 済み executable の spawn 前再検証 → shared `proc::run` の順で、
details は `{grant, argv, state, exitCode, signal, durationMs, stdout,
stderr}`。registry は nonempty-grants gate・writeScope 拒否・per-scope
pin map 構築・`is_process` → `is_managed` 一般化を実装し、invocation は
`inner.command` 格納 + `process` 同型の declared-shape parity check を
追加した。`access.rs`/`scheduler.rs` は無変更(`is_mutating_tool`・pool
name で自動追従)。

## Context

TASK-034EOHORutYi73P8DaumoH / PLAN-034EO1kmKwJ8MAHSHWyCws。issue #101
の policy-gated `command` tool の第 3 task。T1(b706188)の core policy
vocabulary(`ToolName::Command`・`resolve_command_policy`/`resolve_command_grant`/
`command_grant_resources`/`ArgPattern`/`argv_shell_re`/deny set・per-grant
override)と T2(2cb6965)の `tools/proc/` shared 機構(`proc::run`/
`ProcLimits`/`ProcPrepared`/`ProcIdentity`/`prepare_cwd`/`minimal_env`/
`resolve_process_executable`)の上に実装する。design §(a) argv match・
§(b) pin・§(c) writeScope・§(e) details/module・§(f) pool・§(g) registry
plumbing・§(i) strict schema が対象セクション。

## Implementation

### Changes

- `tools/command/mod.rs`: deny-stub を実装へ全面差し替え。
  - `command_schema` — `{name, args}` のみ・`required: ["name","args"]`・
    `additionalProperties: false`。`executable`/`cwd`/`env` は model 入力
    経路に乗らず schema 層の `INVALID_INPUT` に落ちる(design §(i))。
    `name` は grant 名 pattern、`args` は 256 要素・4096 文字上限。
  - `command_details_schema` — `{grant, argv, state, exitCode, signal,
    durationMs, stdout, stderr}` 全 required・`additionalProperties: false`。
    `durationMs`/`argv` は spec-mandated の process 超過 field(design §(e))。
  - `command_operation_declaration` — grant 解決(未知 grant は
    `POLICY_VIOLATION` で schedule 前 fail-closed)→ `ClaimMode::Exclusive`
    + 独自 pool `"command"`(`max_concurrent = policy.max_concurrent`)+
    `command_grant_resources` の保守的 manifest(workspace root subtree の
    reads+writes、`produces` なし)。
  - `pin_command_grants`(registry 呼出し、`pub(crate)`)— 各 grant の
    declared executable を `resolve_process_executable` で ambient PATH
    解決し realpath 済み canonical path へ pin。workspace-resident
    executable は二段拒否: (a) `is_relative_executable_path`(separator 付き
    非 absolute)を resolver 委譲前に `POLICY_VIOLATION`、(b) realpath 後の
    canonical path が `within_workspace` なら `POLICY_VIOLATION`。resolved
    basename の `is_denied_command_executable` 再検査も実施(alias/symlink
    が denied executable へ着地する迂回を塞ぐ — design §(d) 適用点 2)。
  - `verify_pinned_executable` — spawn 直前に `native_executable()`
    (pin 時と同じ candidate 検証: realpath → shell/shim → is_file →
    ext → X_OK → native magic)を pin path へ再適用し、realpath 変化・
    deny basename・workspace residency を fail-closed reject。
  - `authorize_args` — 全 token へ `argv_shell_re` metachar check を
    pattern match 前の独立 check として適用し、その後 `ArgPattern` で
    positional match(pattern 無し grant は任意 argv を許容)。pattern は
    resolved grant の source string から `ArgPattern::compile` で match 時
    再 compile(policy resolve で compile 検証済み)。
  - `execute` — pre-abort check → `{name,args}` 抽出 → `resolve_command_grant`
    → `authorize_args` → `prepare_cwd(root, grant.cwd, ...)`(manifest/
    writeScope coverage 検査は持たない)→ pin map lookup(無ければ
    `POLICY_VIOLATION`)+ `verify_pinned_executable` → `ProcLimits`(per-grant
    `timeout`/`kill_grace`/`max_output_bytes` + section-level heartbeat)・
    `ProcPrepared`(pin 済み executable・args・`minimal_env(&grant.env)`)・
    `ProcIdentity`(`{grant,argv}` details・`"command {name}"` display・
    `event_prefix "command"`・`report_duration_ms: true`)→ `proc::run`。
  - unit test 11 件: description grant 名埋め込み・Exclusive+pool
    declaration・argv match(wildcard/no-pattern/metachar)・relative 及び
    absolute workspace-resident pin 拒否・unresolvable/denied pin
    fail-closed・pin map 構築・spawn-time 再検証(missing/swapped/
    workspace path)・`within_workspace` boundary。
- `tools/proc/mod.rs`: `ProcIdentity.report_duration_ms: bool` を追加 —
  `command` details schema の required `durationMs` を spec-mandated 差分
  として identity flag で流す(`process` は false で不変、AC1)。`run` が
  spawn 前 `Instant::now()` を起点に取り、spawn 失敗 details にも
  `durationMs` を挿入。
- `tools/proc/monitor.rs`: timeout details と終了 details の両方で
  `identity.report_duration_ms` 時のみ `durationMs`(diagnostics 起点
  elapsed)を挿入。
- `tools/proc/executable.rs`: `native()` を `pub(crate) fn
  native_executable` wrapper で公開(pin 再検証が同一 candidate 検証を
  再適用するため — design §(b))。
- `tools/process/mod.rs`: `ProcIdentity` literal に
  `report_duration_ms: false` を追加のみ(details 不変)。
- `catalog.rs`: `NAMES` に `ToolName::Command` を追加(9 tool 順序の末尾)。
  catalog arm は T1 で既に `command_schema`/`command_details_schema`/
  `command_grant_resources` へ配線済みのため変更不要だった。order test を
  `command` 込みへ更新。
- `registry.rs`:
  - `ToolCtx.command: ResolvedCommandPolicy`・`ToolCtx.command_pins:
    BTreeMap<String,String>`・`ToolCtx::command() → CommandToolOptions`。
  - `create_tool_runtime` — `resolve_command_policy(options.command)` を
    無条件 resolve(dormant runtime は resolve のみ、pin しない)。
    `names` に `Command` を含む場合、grants 空なら
    `POLICY_VIOLATION "command exposure requires nonempty authorized
    grants"`、`write_scope` があれば `POLICY_VIOLATION` で fail-closed
    (design §(c)/(g))。gate 通過後、per-scope `pin_command_grants` を
    `resources.root`+`symlink_policy`+`denylist` で構築(`names` に
    `Command` を含まない runtime は pin 試行せず dormant)。
  - `registered_definition` — `ToolName::Command` arm を
    `command_tool_definition(ctx.command())` へ差し替え(deny-stub 撤去)。
  - `execute_inner` — declaration arm で submitted `{name}` を事前解決し
    `command_operation_declaration(&command, grant)`(未知 grant は
    schedule 前 `POLICY_VIOLATION`)。
  - `is_process` → `is_managed` 一般化 —
    `matches!(name, Process|Command)` で `managed.track()`/`drain()`/
    `wait_idle` の managed-subprocess tracking が `command` の子 process
    にも効く(design §(g))。
- `invocation.rs`: `create_invocation_tool_runtime` で
  `resolve_command_policy(options.command)` → `InvocationInner.command`
  へ格納。`create_scoped_runtime` 入口で `options.runtime.command` を
  re-resolve し `inner.command` と `PartialEq` 比較 — 不一致は
  `process` と同型の `invalid_execution_binding("command policy
  mismatch")` で reject(design §(g))。pin は行わない(`names` を持たない
  ため — pin は各 scope の `create_tool_runtime` 責務)。`runtime.command`
  は caller 指定 raw を scope options へ passthrough(T4 で確定する再注入
  方針の前提変更なし)。
- `tests/proc_runtime.rs`: `exposes_eight_schemas` → `exposes_nine_schemas`
  (`command` 込み 9 schema 順序 assertion)。

### Decisions During Implementation

- **`durationMs` は `ProcIdentity` flag で注入**(generic `proc::run` へ
  duration field を恒常追加するのではなく): `process` details schema は
  `durationMs` を持たないため、追加すると AC1(process contract 不変)を
  破る。`report_duration_ms` を identity の一部として caller が選べる形に
  した。spawn 失敗 details にも挿入するため `run` 入口で起点を取り、
  timeout/終了 details は既存 `Diagnostics.started` を再利用。
- **pin は `create_tool_runtime` 境界のみ**: invocation options には
  `names` が無く、`Command` exposure の有無を判断できないため、dormant
  runtime で ambient PATH 解決不能な executable を持つ grant でも構築を
  fail させない。scope parity check は pin 前 declared-shape
  (`ResolvedCommandPolicy` 同士の `PartialEq`)で行い、pin map は比較対象
  外(design §(b)/(g) 通り)。
- **workspace-resident 拒否は pre-resolution と post-realpath の二段**:
  relative path form(`bin/tool` 等)は resolver の workspace-relative 解決へ
  着地するため委譲前に拒否し、absolute path / PATH 上 symlink が
  workspace 内へ解決される迂回は realpath 後の `within_workspace` で塞ぐ。
  `within_workspace` は `nodepath::relative` の `..` segment / absolute
  判定で segment-aware に実装し、`root-sibling` のような prefix 誤判定を
  避ける。
- **pin 再検証は `native()` の再適用**: pin 時と同一の candidate 検証を
  spawn 直前に再実行し、realpath 差分(=symlink 差替え)も reject。これは
  `verify_pinned_executable` で pin map entry の存在 check(missing →
  `POLICY_VIOLATION`)と合わせて fail-closed。
- **declaration は submitted `{name}` で grant 解決**: schedule 前に未知
  grant を `POLICY_VIOLATION` で落とすため、`process` declaration arm の
  task 解決先例に倣い `submitted_input.get("name")` を使う。schema 未検証
  の入力では空文字 → 未知 grant → `POLICY_VIOLATION` と fail-closed。
- **`is_process` → `is_managed`**: `command` も `proc::run` 経由で子
  process を spawn するため、`managed.track()` による runtime drain/
  wait_idle の対象に含める必要がある。最小の同型変更として `matches!`
  へ一般化(helper 分離や名前変更以上の構造変更は行わなかった)。

## Related Experiments

### Adopted

なし。design §(a)-(i)・task doc が確定済みで追加 experiment は起こさなかった。

### Rejected

- `command` 固有の spawn/monitor 実装 — design §(e) が `proc::run` 共有を
  規定しており、lifecycle 二重化は却下済み。
- `durationMs` を `proc::run` の無条件 details field にする案 — AC1 の
  `process` details 不変を破るため `ProcIdentity` flag で opt-in とした。
- invocation 層での pin — `names` を持たない invocation は `Command`
  exposure を判断できず、dormant runtime の構築を過剰に fail させるため
  `create_tool_runtime` 境界の per-scope pin のみとした。

## Validation

- `mise exec -- cargo test -p cheaper-tools` — 全 target pass。
  unit 259/259(command::tests 11 件新規含む)、`proc_runtime` 7/7
  (`exposes_nine_schemas`)、`proc_invocation` 10/10、`proc_process` 9/9、
  `characterization` 10/10、`differential` 11/11。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo fmt --all -- --check` — clean。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` — exit 0。

## Risks

- pin map は per-scope で invocation をまたぐ cache がない — 同一
  invocation 内で複数 scope が `Command` を expose する場合、ambient
  PATH が変化すれば pin path が scope 間でずれ得るが、これは design §(b)
  の確定仕様(各 scope が自身の時点で pin)。
- `verify_pinned_executable` は spawn 毎に realpath+magic sniff を再実行
  する — pin 済み file の IO コストは process spawn 自体に対して無視
  できるが、極端な高頻度呼出しでは観測対象。
- `command` の integration 経路(worker/workflow 経由の end-to-end 実行・
  runtime gate の integration test)は T5 の `proc_command.rs` suite が
  担う — 本 task は unit-level coverage と `proc_runtime` の schema
  assertion まで。

## Follow-ups

- T4: builtin worker policy への command grant 配線・invocation resolved
  policy 再注入方針の確定(`options.runtime.command` passthrough の
  `inner.command` 再注入化)、CLI surface。
- T5: `tests/proc_command.rs` integration suite(grant 実行 E2E・gate・
  parity・lifecycle drain の integration 検証)。

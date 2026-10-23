---
id: "TASK-034KLYAfuIu8es8ZgpuQRl"
type: "task"
status: "todo"
title: "cheaper-tools への exec_deny 配管と pin/verify 適用点注入"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_runtime.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/characterization.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/tasks/add-core-security-policy-and-schema-wiring.md"
  blocks:
    - "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  references:
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/discovery/opt-in-exec-deny-patterns.md"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/registry.rs"
    - "crates/cheaper-tools/src/invocation.rs"
  defers: []
  deferred-by: []
---

# cheaper-tools への exec_deny 配管と pin/verify 適用点注入

## Work

plan `docs/plans/opt-in-exec-deny-patterns.md`
（PLAN-034KLWpEpVSlofJ7Idynh5）の T2。T1 が追加した
`ExecDenyList`/`resolve_exec_deny_list`/`SecurityPolicy` を
cheaper-tools へ配管し、適用点 2（`pin_command_grant` resolved
basename）・適用点 3（`verify_pinned_executable` spawn 前再検証）
を注入する。design（DESIGN-034KLwE8GiJQ2zvQY20ob8）の signature
契約・options 配管表・適用点 error 契約をそのまま実装する —
再解釈しない。

### 1. `tools/command/mod.rs` — signature + 適用点 2/3

- `pin_command_grants`（:167-179、pub(crate)）→
  `(policy, root, symlink_policy, denylist, exec_deny:
  &ExecDenyList) -> Result<BTreeMap<String,String>, ToolError>`。
  各 grant の `pin_command_grant` 呼出しへ `exec_deny` を転送。
- `pin_command_grant`（:185-202、pub — `route::resolve` の
  viability seam）→ `(name, grant, root, symlink_policy,
  denylist, exec_deny: &ExecDenyList) -> Result<String,
  ToolError>`。`_name` → `name`（deny error の
  `grant_violation` で使用 — discovery Open Question 確定）。
  **適用点 2**: `resolve_process_executable` の戻りを
  `let resolved = ...?;` で受け、`exec_deny.is_denied(&resolved)`
  で `return Err(grant_violation(name, "resolved executable is
  denied by security.execDenyPatterns"))`（`PolicyViolation` +
  `details.grant`）。
- `verify_pinned_executable`（:210-224、private）→ `(name,
  pinned, exec_deny: &ExecDenyList) -> Result<(), ToolError>`。
  **適用点 3**: pin-equality（`resolved != pinned`、:217-222）の
  **直後**に `exec_deny.is_denied(&resolved)` → 同規約
  `grant_violation`。構造的には pin 時判定と等価だが「全適用点で
  同一 predicate を再適用する」uniform 契約を維持するため残す
  （design・discovery 適用点 3 注記と同一判断）。
- `CommandToolOptions`（:233-244）に `pub exec_deny:
  ExecDenyList` を追加 — `denylist: Option<PathDenylist>` 先例の
  owned field。非 Option — 空 `Default` が no-op を表現する
  単一の形。
- `check_command_execution`（:368）の signature は**不変** —
  `verify_pinned_executable(name, pinned)` 呼出し（:391-394）に
  `options.exec_deny` を渡すのみ。
- file 内 test churn（compiler が全て捕捉 — design Churn 一覧）:
  `resolve_command_policy` 呼出し（:583,:750）、
  `CommandToolOptions` literal（:603,:794）、pin test 呼出し
  （:712,730,746,767）、`verify_pinned_executable` 呼出し
  （:778,781,789）。

### 2. `registry.rs` — `ToolRuntimeOptions.security` + `ToolCtx.exec_deny`

- `ToolRuntimeOptions`（:90-133）に `pub security:
  Option<Value>` を追加 — `command`/`process`/`concurrency` と
  同じ raw JSON 規約（caller は `serde_json::to_value(
  &policy.security)` を渡し、`create_tool_runtime` 内で
  compiled 化）。
- `create_tool_runtime`（:1114-1115）—
  `let exec_deny = resolve_exec_deny_list(
  options.security.as_ref())?`（error は `map_err` で
  `ToolError` 化）を得て
  `resolve_command_policy(options.command.as_ref(), &exec_deny)`。
- per-scope pin（:1208-1214）— `pin_command_grants(&command,
  &resources.root, symlink, denylist, &exec_deny)`。
- `ToolCtx`（:213-233）に `exec_deny: ExecDenyList` field 追加、
  `command()`（:317-327）が `exec_deny:
  self.exec_deny.clone()` を `CommandToolOptions` へ、
  `ToolCtx` literal（:1298）に `exec_deny` を追加。

### 3. `invocation.rs` — `InvocationToolRuntimeOptions.security` + `InvocationInner`

- `InvocationToolRuntimeOptions`（:61-78）に `pub security:
  Option<Value>` を追加（`command` と同型の raw JSON 規約）。
- `InvocationInner`（:91-…）に `security: SecurityPolicy`（
  parity check の比較対象 — `inner.command` と同型の
  declared-shape 保持）と `exec_deny: ExecDenyList`（scope 構築・
  parity 再 resolve・再注入で共有する compiled 形）を追加。
- `create_invocation_tool_runtime`（:390-398）—
  `let security = resolve_security_policy(
  options.security.as_ref())?;` +
  `let exec_deny = ExecDenyList::compile(
  &security.exec_deny_patterns)?;` →
  `resolve_command_policy(options.command.as_ref(), &exec_deny)`。
  `InvocationInner` 構築へ `security`・`exec_deny` を畳み込む。
- parity check（:219-238）— `options.runtime.security` が
  `Some` なら `resolve_security_policy(Some(raw))` で再 resolve
  し `inner.security` と `PartialEq` 比較、不一致は
  `invalid_binding("security policy mismatch")`
  （`InvalidRequest` + `details.reason`）。`options.runtime.
  command` の再 resolve（:233-234）には `&inner.exec_deny` を
  渡す — caller 供給 command の declared executable が deny に
  match した場合の error は `invalid_binding` ではなく
  `InvalidRuntimePolicy` で返る（`with_invocation_layer` が
  inner code を保持する `load.rs:92-98` 先例と同型 — 共有
  validation は内側 code を維持する規約）。
- scope 再注入（:271-274 付近の `runtime_options` literal）—
  `command` と同型に `security: Some(serde_json::to_value(
  &inner.security).expect("SecurityPolicy は必ず serialize
  可能"))` を追加（caller の `None`/任意 raw に関わらず scope
  は常に invocation-resolved の security を受け取る）。

### 4. tools test churn（signature/field 追随 — design Churn 一覧）

| file | site（現行） | 変更 |
|------|------------|------|
| `tests/proc_command.rs` | :206-227 `options()` `ToolRuntimeOptions` literal | `security: None`（または deny を持たせる test は policy 由来値） |
| `tests/proc_command.rs` | :430・:1560 `resolve_command_policy` 呼出し | `&ExecDenyList::default()` 等へ追随 |
| `tests/proc_command.rs` | :1577 `pin_command_grant` 呼出し・:1564,:1585 `CommandToolOptions` literal | `exec_deny` param/field 追加（`:1585` の `..options` spread は自動継承 — 差分確認対象） |
| `tests/proc_invocation.rs` | :92-… `InvocationToolRuntimeOptions`・:106-131 `ToolRuntimeOptions`・:436-… literal | `security: None` 追加 |
| `tests/proc_degrade.rs` | :47-… `ToolRuntimeOptions` literal | `security: None` 追加 |
| `tests/proc_runtime.rs` | :88-… `ToolRuntimeOptions` literal | `security: None` 追加 |
| `tests/characterization.rs` | :73-… `ToolRuntimeOptions` literal | `security: None` 追加 |
| `tests/differential.rs` | :669-… `ToolRuntimeOptions` literal | `security: None` 追加 |

### 5. 新規 test（TSPEC の Rules に対応）

- `command/mod.rs` unit — 適用点 2/3: tempdir 内の実在
  executable（stem が pattern に match する名、または test
  binary + matching pattern）で `pin_command_grant` /
  `verify_pinned_executable` が `PolicyViolation` +
  `details.grant` を返す（message `command grant "{name}":
  resolved executable is denied by
  security.execDenyPatterns`）。
- `proc_command.rs` — `ToolRuntimeOptions.security` に raw
  JSON（`{"execDenyPatterns":[...]}`）を渡した `create_tool_
  runtime` が deny を適用（適用点 1 が scope 構築で発火 /
  denied resolved grant で `pin_command_grants` 全体が `Err`
  — blast radius pin）。`check_command_execution` 経由の
  適用点 3 deny。
- `proc_invocation.rs` — `InvocationToolRuntimeOptions.
  security` の resolve + compile、parity check の `security`
  不一致が `invalid_binding("security policy mismatch")`
  （`InvalidRequest` + `details.reason`）、scope 再注入が
  `inner.security` を運ぶ（caller `None`/mismatch raw に
  関わらず）、caller 供給 `runtime.command` の declared
  executable が inner deny に match → `InvalidRuntimePolicy`
  （`invalid_binding` でない inner code 保持 pin）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/opt-in-exec-deny-patterns.md`
  （PLAN-034KLWpEpVSlofJ7Idynh5 — T2）
- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8 — signature 契約表・
  options/struct 配管表・call site 配線・適用点 2/3 error
  契約が AUTHORITATIVE）
- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH — R6/R8/R9/R10）
- `docs/test-specs/opt-in-exec-deny-patterns.md`
  （TSPEC-034KLXPTSxUL1oOQI06HM3 — 適用点 2/3・既定 no-op・
  options/scope 配管 Rules）
- `docs/tasks/add-core-security-policy-and-schema-wiring.md`
  （TASK-034KLYvyO70lji85Lv8jvN — 前提）

## Dependencies

- `docs/tasks/add-core-security-policy-and-schema-wiring.md`
  （T1） — `ExecDenyList`・`resolve_exec_deny_list`・
  `SecurityPolicy`・`resolve_command_policy` の新 signature
  が必要。
- 既知の transient: 本 task で `pin_command_grant`（pub）の
  signature・`CommandToolOptions` の field が変わるため、
  cheaper-cli の `route/resolve.rs`（:311・:320-332）が T3 の
  追随まで compile 不可。本 task の検証は `cargo test -p
  cheaper-tools` に scope する。

## Affected Files

- `crates/cheaper-tools/src/tools/command/mod.rs`（modify）
- `crates/cheaper-tools/src/registry.rs`（modify）
- `crates/cheaper-tools/src/invocation.rs`（modify）
- `crates/cheaper-tools/tests/proc_command.rs`（modify）
- `crates/cheaper-tools/tests/proc_invocation.rs`（modify）
- `crates/cheaper-tools/tests/proc_degrade.rs`（modify）
- `crates/cheaper-tools/tests/proc_runtime.rs`（modify）
- `crates/cheaper-tools/tests/characterization.rs`（modify）
- `crates/cheaper-tools/tests/differential.rs`（modify）

## Done When

- [ ] `pin_command_grants`・`pin_command_grant`・
      `verify_pinned_executable` が `exec_deny: &ExecDenyList`
      param を持ち、適用点 2（resolved basename）・適用点 3
      （pin-equality 直後の再検証）が `grant_violation` 規約で
      `PolicyViolation` を返す
- [ ] `pin_command_grant` の `_name` が `name` へ改名され
      `grant_violation` が grant 名を `details.grant` に載せる
- [ ] `CommandToolOptions.exec_deny` が非 Option の
      `ExecDenyList` field として存在し、
      `check_command_execution` が `options.exec_deny` を
      `verify_pinned_executable` へ渡す（`check_command_execution`
      の signature 不変）
- [ ] `ToolRuntimeOptions.security`・
      `InvocationToolRuntimeOptions.security` が `Option<Value>`
      として存在し、`create_tool_runtime` が
      `resolve_exec_deny_list` で compiled deny を
      `resolve_command_policy`・`pin_command_grants` へ共有する
- [ ] `InvocationInner.{security,exec_deny}` が保持され、parity
      check の security 不一致が `invalid_binding("security
      policy mismatch")`、command 再 resolve が
      `&inner.exec_deny`、scope 再注入が `inner.security` を
      運ぶ
- [ ] tools test churn + 新規 test（適用点 2/3・options deny・
      parity・再注入）が存在し `cargo test -p cheaper-tools` が
      pass

## Out of Scope

- `crates/cheaper-cli`/`cheaper-runtime`/`cheaper-workflow`/
  `cheaper-rig-adapter` 側の call site・literal churn（T3 —
  `route/resolve.rs` の `pin_command_grant`/`CommandToolOptions`
  追随を含む）
- docs（T4）
- `check_command_execution`・`ToolRuntimeOptions` 以外の tool
  options 変更・pin map 構造の変更
- `process.tasks` への deny（別 trust boundary）

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools` exits 0
      （unit test + proc_* integration + characterization +
      differential 全 pass — single-crate では dev-seam
      feature 適用外のため flag 無し）
- [ ] `mise exec -- cargo clippy -p cheaper-tools --all-targets`
      exits 0

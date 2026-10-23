---
id: "TASK-034KLYAKiwETFc4o0BnVI1"
type: "task"
status: "todo"
title: "cheaper-cli/runtime への exec_deny 配管と残 test churn"
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
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/tasks/plumb-exec-deny-through-tools.md"
  blocks:
    - "docs/tasks/verify-exec-deny-patterns.md"
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
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
  defers: []
  deferred-by: []
---

# cheaper-cli/runtime への exec_deny 配管と残 test churn

## Work

plan `docs/plans/opt-in-exec-deny-patterns.md`
（PLAN-034KLWpEpVSlofJ7Idynh5）の T3。T2 で signature が変わった
`pin_command_grant`・`CommandToolOptions`・`resolve_command_policy`
・options types を cheaper-cli・cheaper-runtime・cheaper-workflow・
cheaper-rig-adapter 側へ追随させ、`RuntimePolicy.security` から
route viability・worker scope・invocation parity まで deny を
端到端に届ける。design（DESIGN-034KLwE8GiJQ2zvQY20ob8）の
options 配管表・call site 配線をそのまま実装する。

### 1. `route/resolve.rs` — `CommandPolicyView.exec_deny` + viability

- `CommandPolicyView<'a>`（:69-76）に `pub exec_deny:
  &'a ExecDenyList` を追加 — 全 field borrowed の struct 規約に
  従い `&'a`（`Copy` 維持）。
- viability（:311-319）の `pin_command_grant` 呼出しに
  `input.command_policy.exec_deny` を渡す。
- viability の `CommandToolOptions` literal（:320-332）に
  `exec_deny: input.command_policy.exec_deny.clone()` を追加。
- test 内 `policy_view` helper（:365-372）の
  `CommandPolicyView` literal と `resolve_command_policy`
  呼出し（:385）を signature 追随。
- 新規 unit test — denied resolved-stem grant の viability が
  `Viability::NoGrant` に畳まれること（error ではなく route
  不成立 — unpinnable と同じ分類。declared ≠ resolved の
  rename/symlink 経路で構成 — declared match は `load_policy`
  段階で全体失敗するため viability へ到達しない）。

### 2. `route/supply.rs` — `RouteSupply.exec_deny`

- `RouteSupply`（:28-35）に `pub exec_deny: ExecDenyList` を
  追加。
- `load_with_builtin`（:104-123）で
  `ExecDenyList::compile(&policy.security.exec_deny_patterns)?`
  → `RouteSupply.exec_deny` を populate（load 済み policy 由来
  のため compile 失敗は到達不能想定だが `?` で伝搬 — design
  確定）。
- `command_view()`（:40-47）が `exec_deny: &self.exec_deny` を
  `CommandPolicyView` へ渡す。

### 3. `hook/routing_policy.rs` — literal churn のみ

- `RouteEvalInput.command_policy`（:142）は
  `CommandPolicyView` — view が deny-aware になるため本経路は
  **自動適用**（hook eval の viability も同一 deny を見る）。
  signature 変更不要。
- test literal の `CommandPolicyView`（:2100-2106 付近）と
  `resolve_command_policy` 呼出し（:2068 付近）を signature
  追随。

### 4. `cheaper-runtime/src/run/mod.rs` — options literal 2 site

- `InvocationToolRuntimeOptions` literal（:335-354）に
  `security: Some(serde_json::to_value(&policy.security)
  .unwrap_or_default())` を追加（`command`/`process` と同型の
  resolved policy serialize — invocation runtime の parity
  inner 値になる）。
- worker scope `ToolRuntimeOptions` literal（:487-524）に同じく
  `security: Some(serde_json::to_value(&policy.security)
  .unwrap_or_default())` を追加（`command` と同型の parity
  check 入力）。

### 5. `cheaper-cli/src/workflow_run.rs` — options literal 2 site

- `ensure_invocation` の `InvocationToolRuntimeOptions`
  （:236-271）に `security: Some(serde_json::to_value(
  &shared.policy.security).map_err(|e| -> ToolError {
  Box::new(CheaperError::new(ErrorCode::InternalError,
  e.to_string())) })?)` を追加（`process`/`command` literal と
  同型の error 変換）。
- `kind: tool` worker scope `ToolRuntimeOptions`（:491-518）に
  `security: None` を追加（`command: None` と同型 — parity
  skip、inner の値が再注入される）。

### 6. `cheaper-cli/src/policy.rs` — doc restate のみ

- module doc（:8-9）の検査内容列挙と `write_validated` doc
  （:245-247）に `security.execDenyPatterns` の deny 照合
  （declared executable が user policy の deny に match する
  spec を add 時点で `InvalidRuntimePolicy`）を追記。
  code 変更不要 — `write_validated` は merged full object を
  `validate_policy_schema(Partial)` で検査するため既に
  fail-closed。

### 7. `RuntimePolicy` literal churn（`security` field 追加）

- `crates/cheaper-runtime/src/dev/fixture.rs`（:759）—
  `default_policy()` の literal に `security:
  SecurityPolicy::default()`。
- `crates/cheaper-runtime/tests/run_dev_seam.rs`（:177,189）—
  `policy_with`/`policy` literal に同上。
- `crates/cheaper-runtime/tests/ollama_e2e.rs`（:219）— e2e
  policy literal に同上。

### 8. cross-crate options literal churn（`security` field 追加）

| file | site（現行） | 変更 |
|------|------------|------|
| `crates/cheaper-rig-adapter/tests/adapter.rs` | :314-… `ToolRuntimeOptions` literal | `security: None` 追加 |
| `crates/cheaper-workflow/tests/process.rs` | :32-… `ToolRuntimeOptions`・:108-… `InvocationToolRuntimeOptions` literal | `security: None` 追加 |
| `crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs` | :69-… `ToolRuntimeOptions`・:102-… `InvocationToolRuntimeOptions` literal | `security: None` 追加 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/opt-in-exec-deny-patterns.md`
  （PLAN-034KLWpEpVSlofJ7Idynh5 — T3）
- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8 — options/struct 配管表・
  call site 配線 14 row・route viability の NoGrant 分類が
  AUTHORITATIVE）
- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH — R6/R7）
- `docs/test-specs/opt-in-exec-deny-patterns.md`
  （TSPEC-034KLXPTSxUL1oOQI06HM3 — 配管・route viability・
  serialized shape Rules）
- `docs/tasks/plumb-exec-deny-through-tools.md`
  （TASK-034KLYAfuIu8es8ZgpuQRl — 前提）

## Dependencies

- `docs/tasks/plumb-exec-deny-through-tools.md`（T2） —
  `pin_command_grant` の新 signature・`CommandToolOptions.
  exec_deny`・`ExecDenyList` を消費する。T2 完了まで
  `route/resolve.rs`（:311・:320-332）は compile 不可
  （T2 の既知 transient の解消側）。
- T4（docs）とは並列実行可能 — 共有 file を持たない。

## Affected Files

- `crates/cheaper-cli/src/route/resolve.rs`（modify）
- `crates/cheaper-cli/src/route/supply.rs`（modify）
- `crates/cheaper-cli/src/hook/routing_policy.rs`（modify）
- `crates/cheaper-cli/src/workflow_run.rs`（modify）
- `crates/cheaper-cli/src/policy.rs`（modify — doc restate のみ）
- `crates/cheaper-runtime/src/run/mod.rs`（modify）
- `crates/cheaper-runtime/src/dev/fixture.rs`（modify）
- `crates/cheaper-runtime/tests/run_dev_seam.rs`（modify）
- `crates/cheaper-runtime/tests/ollama_e2e.rs`（modify）
- `crates/cheaper-rig-adapter/tests/adapter.rs`（modify）
- `crates/cheaper-workflow/tests/process.rs`（modify）
- `crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs`
  （modify）

## Done When

- [ ] `CommandPolicyView.exec_deny`（`&'a`・`Copy` 維持）が
      `RouteSupply.exec_deny`（`load_with_builtin` で compiled）
      から供給され、viability の `pin_command_grant`・
      `CommandToolOptions` が deny-aware になる
- [ ] denied resolved-stem grant の viability が
      `Viability::NoGrant` になる unit test が存在する
- [ ] `run/mod.rs`・`workflow_run.rs` の
      `InvocationToolRuntimeOptions`/`ToolRuntimeOptions`
      literal が `security` を運び（invocation 系は policy
      由来値、`kind: tool` scope は `None` + inner 再注入）
- [ ] `hook/routing_policy.rs`・`route/resolve.rs` の test
      literal が signature 追随し、hook eval 経路が同一 deny
      を見る
- [ ] `policy.rs` の doc が deny 照合を含む検査内容を正直に
      記述する
- [ ] runtime/workflow/rig-adapter の `RuntimePolicy`・options
      literal churn が完了し、workspace が compile 回復する
- [ ] cli test target（integration・run_dev_seam・workflow・
      workflow_dev_seam）+ workflow/rig-adapter test が pass

## Out of Scope

- docs（`command.md`・`configuration.md`・
  `resources/policy.yaml`）（T4 — 並列実行可能）
- `security` の merge 意味論・invocation layer guard の変更
  （既存機構で到達不能/拒否済み）
- `Viability` 分類・reporting UX の変更（`NoGrant` 畳み込みの
  現行分類を維持）
- `RouteSupply` の fail-open（`load_or_none`）経路の変更 —
  invalid policy で route arm 全体が不活性化するのは既存
  convention（design 受理済み）

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test contracts --test run_dev_seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-workflow --test compile_contracts --test filesystem_contracts` exits 0
- [ ] `mise exec -- cargo test -p cheaper-rig-adapter` exits 0
- [ ] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0（workspace compile 回復）

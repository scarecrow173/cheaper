---
id: "TASK-034EOHDmemAJi6IbUQel4x"
type: "task"
status: "done"
title: "T1: command core policy・schema 分割と compile-forced propagation"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/impl/ir/command-core-policy-vocabulary-and-propagation.md"
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
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

# T1: command core policy・schema 分割と compile-forced propagation

## Scope

canonical tool vocabulary の 9 個目 `command` を core に導入し、grant 解決・
trusted-layer-only schema 境界（`PolicySchemaMode::ProjectPartial`）を実装する。
`ToolName` closed enum 拡張と `RuntimePolicy`/`ToolRuntimeOptions`/
`InvocationToolRuntimeOptions` の必須 field 追加は全構築点を compile error にするため、
compile-forced propagation の全 mechanical 変更を本 task が所有し、
**完了時点で `cargo build --workspace --all-targets` が green** であることが必須条件。
`command` は catalog `NAMES` / registered set には入れず dormant とする（NAMES 追加は T3）。

### 所有ファイル

- `crates/cheaper-core/src/tools/vocabulary.rs` — `TOOL_NAMES` に `"command"`、
  `ToolName::Command`、`as_str`/`from_canonical`。
- `crates/cheaper-core/src/policy/command.rs`（新規）— `command_policy_properties`、
  `resolve_command_policy`、`ResolvedCommandPolicy`/`CommandGrant`/
  `resolve_command_grant`（serde-native: `Serialize` + `PartialEq`、argv は pattern
  source string を保持）、`command_grant_resources`（reads+writes の保守的 manifest の
  単一所有者、`produces` は返さない）、`is_denied_command_executable`/
  `command_denied_executable_re`（共に `pub` — cheaper-tools が cross-crate で使う
  interpreter/exec-wrapper deny set）、`ArgPattern` compile+match、
  `command_invalid`（message prefix `"invalid command policy"`）。
- `crates/cheaper-core/src/policy/process.rs` — `argv_shell_re` を `pub` 化、
  `relative`/`str_vec` を `pub(crate)` 化、`bounded_duration`（:175-184）を section
  label 引数化して `pub(crate)` で共有（`invalid` は process 側に残す）。
- `crates/cheaper-core/src/policy/schema.rs` — complete + user partial schema への
  `command` section 追加、`PolicySchemaMode::ProjectPartial` + project partial schema
  （`command` 除外）、`policy_validators()` の 3-validator tuple 化、
  `RuntimePolicy.command` field、`normalize_policy`/`validate_policy_schema` 配線。
- `crates/cheaper-core/src/policy/load.rs` — project call site を `ProjectPartial` へ
  切替（`Partial` は user layer 用に温存、`merge.rs` は無変更）。
- `crates/cheaper-core/src/worker/validate.rs` — `is_mutating_tool` に `Command`。
- `crates/cheaper-core/src/policy/{capability,restrict}.rs` 等他 `RuntimePolicy`
  literal 構築点（capability.rs:86・restrict.rs:100,113・schema.rs:457）— `command`
  field 追加。
- `crates/cheaper-core/tests/golden/` + `fixtures/` — `vocabulary.json` 8→9 要素、
  policy schema mode mapping に `"project-partial"`、project-partial grant 拒否 /
  user-partial 受理 fixture、`policy/{load,restrict,schema}.json` 再生成、
  `worker_validate.rs` golden に command entry。`policy/merge.json` は変更不要。
- `crates/cheaper-tools/src/tools/command/mod.rs`（新規）— `command_schema`
  （`{name,args}` input schema）・`command_details_schema`・deny-stub
  `command_tool_definition`（execute は `POLICY_VIOLATION "command unavailable"` で
  失敗。catalog/registry の closed enum arm の compile 充足用、実装は T3）。
- `crates/cheaper-tools/src/tools/mod.rs` — `pub mod command;`（T2 の
  `pub mod proc;` と共有する唯一の file、1 行ずつの trivial conflict のみ）。
- `crates/cheaper-tools/src/catalog.rs` — `input_schema`/`output_schema`/`resources`
  の `ToolName::Command` arm（stub ではなく実 `command_schema`/
  `command_details_schema`/`command_grant_resources` を返す）。
- `crates/cheaper-tools/src/registry.rs` — `ToolRuntimeOptions.command` field、
  `registered_definition`（:475-584 の closed match）へ `Command` deny-stub arm。
- `crates/cheaper-tools/src/invocation.rs` — `InvocationToolRuntimeOptions.command`
  field（:60-72）+ `create_worker_scope` 内 `runtime_options` literal（:235-243）への
  機械的 field 追加（この時点では `options.runtime.command` passthrough、
  `inner.command` 再注入への確定は T4）。
- `crates/cheaper-runtime/src/run/mod.rs` — `InvocationToolRuntimeOptions`（:279-295）
  /`ToolRuntimeOptions`（:424-455）literal へ `command:
  serde_json::to_value(&policy.command)` を配線（`process` と同型の機械的 serialize）。
- `crates/cheaper-runtime/src/dev/fixture.rs`（:747）・`tests/run_dev_seam.rs`（:146）・
  `tests/ollama_e2e.rs`（:219）— `RuntimePolicy` literal に `command` field。
- `crates/cheaper-cli/src/workflow_run.rs` — `ensure_invocation` の
  `InvocationToolRuntimeOptions`（:227-255）と `ToolJobExecutor` の
  `ToolRuntimeOptions`（:455-473）へ `command` field 配線。
- `crates/cheaper-workflow/tests/process.rs`（:32,105）・`tests/artifacts.rs`（:69,99）—
  `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal に `command` field。
- `crates/cheaper-tools/tests/`（proc_runtime.rs:89・characterization.rs:74・
  differential.rs:670・proc_invocation.rs:93,106,432）・
  `crates/cheaper-rig-adapter/tests/adapter.rs`（:314）— options literal に
  `command` field。`exposes_eight_schemas`→nine は T3（NAMES 更新と同期）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T1）
- `docs/designs/policy-gated-command-tool.md` — §(a)(d)(h)(k) の設計判断
- `docs/specs/policy-gated-command-tool.md` — AC4（project schema 拒否）
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- なし（plan のみ。T2 と並行可能、共有 file は `tools/mod.rs` の `pub mod` 行のみ）

## Steps

1. `vocabulary.rs` に `ToolName::Command` を追加する。
2. `policy/command.rs` を新規実装し、`process.rs` の可視性調整と
   `bounded_duration` の label 引数化を行う。
3. `schema.rs` に `command` section・`ProjectPartial`・`RuntimePolicy.command`・
   3-validator tuple を実装し、`load.rs` の project call site を切替、
   `normalize_policy`/`validate_policy_schema`/`is_mutating_tool` を配線する。
4. `tools/command/` stub module + `pub mod command`、catalog/registry/invocation の
   deny-stub arm と options field を追加する。
5. 全 `RuntimePolicy`/`ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal に
   `command` field を追加する（compile-forced propagation）。
6. golden fixture を再生成し、project-partial grant 拒否 / user-partial 受理の
   focused fixture を追加する。
7. workspace 全体の build + core test が green であることを確認する。

## Done When

- [x] `cargo build --workspace --all-targets`（dev-seam feature 付き）が green —
  compile-forced propagation 完遂の証明
- [x] grant authoring key を含む project `policy.yaml` が load 時
  `InvalidRuntimePolicy` で拒否され、同内容の user policy は受理される
- [x] golden（vocabulary 8→9・policy load/restrict/schema・worker_validate）が
  再生成され `cargo test -p cheaper-core` が pass
- [x] `command` は catalog `NAMES` / registered set に入っておらず dormant
  （deny-stub arm は到達不能の compile 充足のみ）

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` exits 0
- [x] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

---
id: "TASK-034EUvBkRkmTjWxeq3iG0h"
type: "task"
status: "done"
title: "T1: optionalTools core machinery と compile-forced propagation"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
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
    - "../plans/implement-tool-operator-built-in-worker.md"
  implemented-by:
    - "docs/impl/ir/optionaltools-core-machinery-and-propagation.md"
  depends-on:
    - "../plans/implement-tool-operator-built-in-worker.md"
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
    - "../test-specs/tool-operator-built-in-worker.md"
  references: []
  defers: []
  deferred-by: []
---

# T1: optionalTools core machinery と compile-forced propagation

## Scope

worker frontmatter の opt-in key `cheaper.optionalTools` の parse・validate
機構と、縮退許可集合を registry まで運ぶ `ToolRuntimeOptions.degradable` の
propagation を導入する。`ToolName::resolve`（canonical 名 → `tool_alias`
alias table の single-sourced resolver）と `is_scope_gated_tool`
（`Process` / `Command` の一権威 predicate — gated 集合の二重定義を生まない）
を vocabulary.rs に追加する。

`WorkerCheaper.optional_tools` と `ToolRuntimeOptions.degradable` の必須
field 追加は全 literal 構築点を compile error にするため、design §(i)/(j)
が列挙した compile-forced propagation の全 mechanical 変更を本 task が
所有する（#101 precedent — compile-forced propagation は導入 task に同梱）。
**T1 完了時点で
`cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
が green** でなければならない。漏れは compile error で検出されるため
silent miss はない。

- In: `ToolName::resolve` + `is_scope_gated_tool` + `normalize_tool_names`
  の per-element refactor、`CHEAPER_KEYS` への `"optionalTools"` 追加と
  `WorkerCheaper.optional_tools` parse、design §(b) の 7 validate rule +
  unit test、`ToolRuntimeOptions.degradable` と全 literal site の更新、
  invocation.rs pass-through、run/mod.rs 配線、golden `dump_worker_def`
  の conditional emit。
- Out: registry degrade path 本体・`degraded_names()`・prompt surfacing・
  `tools.degraded` span field（全て T2）、worker 定義・schema・alias（T3）、
  fixture DSL `FixtureWorker.optional_tools`（T4 — `src/dev/fixture.rs` は
  serde DSL で literal を持たず compile-forced 対象外）、docs/APM（T5）。

### 所有ファイル

- `crates/cheaper-core/src/tools/vocabulary.rs` — `pub fn ToolName::resolve`
  （canonical 名 → `tool_alias` alias table の順の single-sourced resolver）、
  `is_scope_gated_tool` predicate、`normalize_tool_names` の per-element
  path（:111）を `resolve` 呼び出しへ refactor。
- `crates/cheaper-core/src/worker/validate.rs` — `CHEAPER_KEYS`（:94）へ
  `"optionalTools"` 追加、`WorkerCheaper.optional_tools: Vec<ToolName>`
  （未指定時は空 vec。構築 literal :250 — compile-forced）、design §(b) の
  7 rule（`optionalTools-invalid` / `optionalTools-empty` /
  `optionalTools-unknown-tool` / `optionalTools-undeclared` /
  `optionalTools-not-gated` / `optionalTools-duplicate` /
  `optionalTools-not-proper-subset`。全て `InvalidWorkerDefinition` を
  `invalid()` 経由で返す — `tools` field の `UnknownTool` code は使わない）
  + unit test。
- `crates/cheaper-core/tests/golden/common/mod.rs` — `dump_worker_def`
  （:415-455）へ `optionalTools` の conditional emit（非空時のみ
  `cheaper.optionalTools` を出力 — `input` / `writeScope` と同じ pattern。
  T4 の accept golden の前提）。
- `crates/cheaper-tools/src/registry.rs` —
  `ToolRuntimeOptions.degradable: Vec<ToolName>` field（:90-123）追加。
  degrade path 本体は T2。
- `crates/cheaper-tools/src/invocation.rs` — `create_worker_scope` 内の
  `ToolRuntimeOptions` 再構成（:257-287）へ `degradable` pass-through 1 行。
  `WorkerScopeOptions` / `InvocationToolRuntimeOptions` は shape 不変。
- `crates/cheaper-runtime/src/run/mod.rs` — `create_tool_scope` の
  `ToolRuntimeOptions` literal（:427-459）へ
  `degradable: worker.cheaper.optional_tools.clone()` 配線。
- `crates/cheaper-cli/src/workflow_run.rs` — `ToolJobExecutor` の
  `ToolRuntimeOptions` literal（:462-482）へ `degradable: Vec::new()`
  （`kind: tool` job は frontmatter を持たず opt-in 不能。`kind: worker`
  job は run 経路で自動的に縮退が効く）。
- `crates/cheaper-tools/tests/{proc_command.rs:202, proc_invocation.rs:107,
  proc_runtime.rs:89, characterization.rs:74, differential.rs:670}` — 各
  `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及）。
- `crates/cheaper-rig-adapter/tests/adapter.rs:314` —
  `ToolRuntimeOptions` literal に `degradable` field。
- `crates/cheaper-workflow/tests/{artifacts.rs:69, process.rs:32,
  common/mod.rs:109}` — `ToolRuntimeOptions` literal 2 箇所と
  `WorkerCheaper` literal（:109 — compile-forced 波及。`WorkerCheaper`
  構築 site は validate.rs:250 とここの 2 箇所のみ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-tool-operator-built-in-worker.md` — 実装対象 plan（T1）
- `docs/designs/tool-operator-built-in-worker.md` — §(b) validate rule・
  §(c) `is_scope_gated_tool`・§(d) propagation・§(i)(j) file map / 波及面
- `docs/specs/tool-operator-built-in-worker.md` — AC4・AC8（挙動機構の前提）
- `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`
  — Option C1（per-tool opt-in 縮退）
- `docs/test-specs/tool-operator-built-in-worker.md` — 検証 spec
  （7 reject reason + 非 opt-in 不変の行為保証）

## Dependencies

- なし（plan のみ。critical path の root で T2・T3・T4 の必須先行条件）

## Steps

1. `vocabulary.rs` に `ToolName::resolve` と `is_scope_gated_tool` を追加し、
   `normalize_tool_names` の per-element path を `resolve` 呼び出しへ
   refactor する。
2. `validate.rs` の `CHEAPER_KEYS` に `"optionalTools"` を追加し、
   `WorkerCheaper` に `optional_tools: Vec<ToolName>`（serde default = 空
   vec）を追加して 7 rule を全て `invalid()` 経由 `InvalidWorkerDefinition`
   で実装、unit test を書く。
3. `registry.rs` に `ToolRuntimeOptions.degradable: Vec<ToolName>` field を
   追加する（degrade path は T2）。
4. `invocation.rs` の `ToolRuntimeOptions` 再構成に `degradable`
   pass-through を追加し、`run/mod.rs` に
   `degradable: worker.cheaper.optional_tools.clone()` を配線、
   `workflow_run.rs` に `degradable: Vec::new()` を配線する。
5. 全 compile-forced literal site（cheaper-tools 5 test・rig-adapter
   adapter.rs・cheaper-workflow 3 file・`WorkerCheaper` literal 2 箇所）に
   `degradable` / `optional_tools` field を追加する。
6. `dump_worker_def` に `optionalTools` の conditional emit（非空時のみ）を
   追加する。
7. workspace 全体の build + core test が green であることを確認する。

## Done When

- [x] `cargo test -p cheaper-core` が pass — `optionalTools` の 7 reject
  rule + accept + 未指定時の既存挙動不変の unit test を含む
- [x] `cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が green —
  compile-forced propagation 完遂の証明（dev-seam feature 付き
  `--all-targets` が exit criteria。dev-seam 無しでは seam 依存 test
  target が検証から抜け落ちる — ADR-034DyGyITd0YaATg8fkld7）
- [x] `is_scope_gated_tool` が gated 集合の一権威 predicate として
  定義され、`optionalTools` key は `optional_tools` 空の非 opt-in worker
  の意味論を一切変えない

## Rollback

- task 単位の coherent commit を `git revert`。**T1+T2 は T3 が land する
  前なら unit として revert 可能**（機構だけを戻しても参照する worker が
  存在しない）。T3 以降に T1/T2 を revert する場合は
  `resources/workers/tool-operator.md`（と schema file、必要なら alias）の
  除去を必ず伴わせる — `optionalTools` key が validate から消えた状態で
  worker file が残ると strict-key validation で built-in resource load が
  死ぬ。
- revert 後に
  `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  と `mise exec -- cargo test -p cheaper-core -p cheaper-tools` が pass
  すること。

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` exits 0
- [x] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

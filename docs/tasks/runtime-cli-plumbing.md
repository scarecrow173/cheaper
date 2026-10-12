---
id: "TASK-034EOHPDg1Gg2LSKLw8WRT"
type: "task"
status: "done"
title: "T4: runtime/CLI plumbing と builtin policy 配線"
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
    - "docs/impl/ir/command-runtime-cli-plumbing-and-builtin-policy-wiring.md"
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
    - "docs/tasks/command-core-policy-and-propagation.md"
    - "docs/tasks/command-tool-and-registry.md"
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

# T4: runtime/CLI plumbing と builtin policy 配線

## Scope

invocation scope が invocation-resolved grants（`inner.command`）を受け取る配線を
確定させ、builtin `policy.yaml` に `command` section と `tools.allow` 登録を追加して
Worker / Workflow `kind: tool` の両経路を実動作させる。T1 で機械的に配線した
`options.runtime.command` passthrough を `inner.command` 再注入へ確定するのが
本 task の本体。

### 所有ファイル

- `crates/cheaper-tools/src/invocation.rs` — `create_worker_scope` の
  `runtime_options.command` を `options.runtime.command` passthrough から
  `inner.command` 再注入へ確定（:240-243 の `process` 再注入と同型 — scope は
  invocation-resolved grants を受け取り `None` にはしない）。
- `crates/cheaper-cli/resources/policy.yaml` — `command: {maxConcurrent: 1,
  grants: {}}` section 追加 + `tools.allow` に `command` を追加し、complete
  schema validation を通す。
- `crates/cheaper-cli/src/workflow_run.rs` — `ensure_invocation`（:227-255）経由の
  scope options への `inner.command` 再注入が効くことを確認し、`ToolJobExecutor`
  の `ToolRuntimeOptions` literal（:455-473、T1 で `command` field 追加済みの
  second literal site）が `names: vec![name]` 共通経路で grants gate を通すことを
  確認する。
- `crates/cheaper-runtime/src/run/mod.rs` — T1 で配線済みの
  `command: serde_json::to_value(&policy.command)` threading（:279-295・:424-455）
  が Worker 経路で invocation-resolved policy を正しく運ぶことを確認する
  （追加の機械変更が必要なら本 task で実施）。
- `crates/cheaper-runtime/src/dev/fixture.rs`・`tests/run_dev_seam.rs` —
  dev fixture の `RuntimePolicy.command`（T1 で literal 追加済み）が builtin
  policy shape と整合することを確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T4）
- `docs/designs/policy-gated-command-tool.md` — §(g) registry plumbing・
  §(i) builtin policy・Data and Control Flow の両経路
- `docs/specs/policy-gated-command-tool.md` — AC3（両経路）
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- `docs/tasks/command-core-policy-and-propagation.md`（T1 — options field・
  機械的 serialize 配線済み）
- `docs/tasks/command-tool-and-registry.md`（T3 — `inner.command` 格納・
  parity・registry gate）

## Steps

1. `invocation.rs` の `create_worker_scope` で `runtime_options.command` を
   `inner.command` 再注入へ確定する。
2. builtin `policy.yaml` に `command: {maxConcurrent: 1, grants: {}}` と
   `tools.allow` の `command` を追加し、complete schema validation を通す。
3. `workflow_run.rs` の scope options / second literal site と
   `run/mod.rs` の policy threading が `inner.command` 由来の resolved grants
   を運ぶことを確認・確定する。
4. Worker 経路（tools 宣言 ∩ tools.allow ∩ grants）と Workflow `kind: tool`
   経路（tools.allow admission + nonempty-grants gate）の両方で `command` が
   到達可能であることを test で確認する。

## Done When

- [x] `create_worker_scope` が `inner.command`（invocation-resolved grants）を
  scope options へ再注入し、passthrough ではなくなっている
- [x] builtin `policy.yaml` が `command` section + `tools.allow` の `command`
  を含み、complete schema validation を通過する
- [x] Worker 経路と Workflow `kind: tool` 経路の両方で `command` tool が
  同一 canonical 実装へ到達する

## Verification

- [x] `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

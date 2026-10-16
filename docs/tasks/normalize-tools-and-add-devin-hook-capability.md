---
id: "TASK-034GtBzfyOaeVS6nlm1ZkC"
type: "task"
status: "done"
title: "Normalize tools and add Devin hook capability"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  blocks:
    - "docs/tasks/register-devin-hooks-from-apm-skill.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/native-tool-name-normalization-and-devin-hook-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Normalize tools and add Devin hook capability

## Work

plan の T1。`tool_name` を ASCII lowercase へ一度だけ正規化して
exact match する共有 `tool_use_normalize` を `targets.rs` に導入し、
`grep` → `FileRead` 接続、`HookTarget::Devin`（`shell_tools: &[]`、
`hook_event_name=="PreToolUse"` gate、`devin_render`）、argv
完全一致 `["targets","--json"]` capability 分岐、配布
`hooks.json` matcher の char class 化を実装する。exact interface・
値は approved design をそのまま適用する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md`（approved）
- `docs/specs/case-insensitive-native-tools-and-devin-registration.md`（approved）
- `docs/designs/devin-native-hooks-and-apm-bootstrap.md`（approved）

## Dependencies

- なし（plan 内の先頭 task）。blocks: T2（capability contract を
  preflight に使う）。

## Affected Files

- `crates/cheaper-cli/src/hook/targets.rs`
- `crates/cheaper-cli/src/hook/mod.rs`
- `crates/cheaper-cli/tests/hook.rs`
- `packages/apm/.apm/hooks/hooks.json`
- `scripts/test/devin-hooks-install.test.mjs`（source matcher 部分のみ）

## Done When

- [x] `hook.rs` の fail test（case variant 同一 routing、malformed/
      unknown fail-open、grep file/dir/omitted、shell guard 維持、
      Devin event gate/response/reason、`ALL` roundtrip、targets
      query）が追加され、red を確認してから実装で green になる。
- [x] `hooks.json` matcher が spec の char class exact。
- [x] matcher test が hooks.json 変更前に作成され red を観測済み。

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features dev-seam --test hook` — exit 0（24 passed / 0 failed）。
- [x] `mise exec -- node --test scripts/test/devin-hooks-install.test.mjs` — matcher 部分 pass（2 passed / 0 failed）。

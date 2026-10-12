---
id: "TASK-034EOHY1ZKj6PcEy0zVz2P"
type: "task"
status: "todo"
title: "T5: command tool の unit・integration test と golden 整備"
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
  implemented-by: []
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
    - "docs/tasks/command-core-policy-and-propagation.md"
    - "docs/tasks/command-tool-and-registry.md"
    - "docs/tasks/runtime-cli-plumbing.md"
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

# T5: command tool の unit・integration test と golden 整備

## Scope

`command` tool の unit + integration test を整備し、spec AC1–AC6 を網羅する。
design Verification Notes の未チェック項目を本 task の checklist に引き継ぐ。

### 所有ファイル

- `crates/cheaper-tools/tests/proc_command.rs`（新規）— grant gate / argv match /
  interpreter・exec-wrapper・loader deny / shell・workspace-resident executable
  拒否 / metachar / cwd escape / timeout / abort / nonzero / output-cap /
  tree-cleanup。Windows/POSIX 差分は既存 `proc_process.rs` の fixture pattern を
  踏襲。
- `crates/cheaper-workflow/tests/*.rs` — `kind: tool` job の
  `resource_inference` 挙動固定 test（code 変更不要 — generic tool 分岐が
  catalog `resources()` 経由の保守的 manifest を返し、underclaim が
  `declared {bucket} underclaims known access` で fail-closed することを固定）。
- `crates/cheaper-core/tests/` — project-partial grant 拒否 / user-partial 受理の
  focused test（T1 の fixture を execute 側からも pin）。
- `crates/cheaper-tools/src/tools/command/`（in-file unit test）— `ArgPattern`
  match・grant 解決・deny set 3 適用点（declared basename・pin 時 resolved
  basename・spawn 前再検証）の unit coverage。
- fake provider / dev-seam 経路の run test — `command` 使用 worker で
  `savings.directlyComparable == false`、`meta.changes` が未観測副作用を
  主張しないこと、独自 pool `"command"` の cap（`command.maxConcurrent`
  default 1）が効くこと。
- 残りの golden / 構築点更新（T1–T4 で後送りになったものがあれば全件解消）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T5）
- `docs/designs/policy-gated-command-tool.md` — Verification Notes
- `docs/specs/policy-gated-command-tool.md` — AC1–AC6
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- `docs/tasks/command-core-policy-and-propagation.md`（T1）
- `docs/tasks/command-tool-and-registry.md`（T3）
- `docs/tasks/runtime-cli-plumbing.md`（T4 — workflow 経路 test の前提）

## Steps

1. `tests/proc_command.rs` を新規作成し、grant gate / argv match / deny /
   metachar / cwd / timeout / abort / nonzero / output-cap / tree-cleanup を
   Windows・POSIX で網羅する。
2. Worker 経路と Workflow `kind: tool` 経路の両方で grant gate（nonempty
   fail-closed・dormant runtime）を検証する。
3. `kind: tool` job の `resource_inference` underclaim fail-closed 挙動固定
   test を追加する。
4. `{name,args}` 以外の input key の `INVALID_INPUT` surface、parity check、
   `directlyComparable == false`、独自 pool cap を test で固定する。
5. 残りの golden / constructor 更新を全件解消し、workspace 全体の test を
   green にする。

## Done When

- [ ] `proc_command.rs` が上記ケースを Windows/POSIX で網羅し pass
- [ ] grant gate が worker + workflow `kind: tool` 両経路で検証済み
- [ ] `savings.directlyComparable == false`・manifest underclaim fail-closed・
  独自 pool `"command"` の cap が test で固定されている
- [ ] `process` 既存 test が無変更で pass（AC1 継続確認）

## Verification

- [ ] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_command` exits 0

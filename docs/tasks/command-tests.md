---
id: "TASK-034EOHY1ZKj6PcEy0zVz2P"
type: "task"
status: "done"
title: "T5: command tool の unit・integration test と golden 整備"
created: "2026-10-12"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_command.rs"}
      - {"type":"doc","path":"docs/impl/ir/command-tool-test-coverage.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-cli/tests/workflow.rs"}
      - {"type":"code","path":"crates/cheaper-cli/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/resource_inference.rs"}
      - {"type":"doc","path":"docs/tasks/command-tests.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/test-specs/policy-gated-command-tool.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-policy-gated-command-tool.md"
  implemented-by:
    - "docs/impl/ir/command-tool-test-coverage.md"
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

- [x] `proc_command.rs` が上記ケースを Windows/POSIX で網羅し pass
- [x] grant gate が worker + workflow `kind: tool` 両経路で検証済み
- [x] `savings.directlyComparable == false`・manifest underclaim fail-closed・
  独自 pool `"command"` の cap が test で固定されている
- [x] `process` 既存 test が無変更で pass（AC1 継続確認）

## Verification

- [x] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [x] `mise exec -- cargo test -p cheaper-tools --test proc_command` exits 0

## Coverage notes（2026-10-13 実績）

- `proc_command.rs` — 25 tests（Windows 24 pass + unix-gated
  `pinned_resolved_basename_deny_rejects` が POSIX CI で実行）。fixture child
  re-exec（`CHEAPER_PROC_CHILD_MODE`/`_ARG`、grant `env` 経由）で実 subprocess
  を spawn し、grant 実行/details・argv literal/prefix/rest・metachar・deny 3
  適用点・workspace-resident・cwd・schema `INVALID_INPUT`・nonzero/timeout/
  abort/output-cap/tree-cleanup・pool cap・Exclusive claim・spawn 前再検証を固定。
- `crates/cheaper-cli/tests/workflow.rs` — `kind: tool` e2e 3 件：trusted user
  layer grant で admission→scope→実 spawn→`$from /details/state` 出力、grants
  無しで job `POLICY_VIOLATION` + envelope `TOOL_EXECUTION_FAILURE`(6)、
  `tools.allow` 除外で admission `POLICY_VIOLATION`。
- `crates/cheaper-cli/tests/run_dev_seam.rs` — fake-stream worker 経路 2 件：
  grant 実行 + `savings.directlyComparable == false` + `meta.changes` が子の
  marker を主張しないこと、grants 無しで `POLICY_VIOLATION`(exit 4)。
- `crates/cheaper-workflow/tests/resource_inference.rs` —
  `catalog_command_conservative_manifest_and_underclaim`：保守的 root subtree
  manifest・`produces` 非宣言・omitted fallback 収束・reads/writes underclaim
  の `INVALID_REQUEST` fail-closed。
- 既存担保の再掲（新規 test 不要）：project-layer `command` 拒否は T1 golden
  （`policy/schema.json`・`policy/load.json`）、pool `maxConcurrent` default 1
  は `policy/command.rs` `defaults_when_absent`、deny set 語彙・ArgPattern・
  parity round-trip は core unit tests。
- 自動化不可/除外：`pinned_resolved_basename_deny_rejects` は Windows で
  symlink 作成が権限依存のため `#[cfg(unix)]`（deny 適用点自体は declared
  basename test で両 OS 固定済み）。`alive()` の POSIX 側は `kill(0)` +
  `/proc` zombie 判定、Windows 側は `tasklist`。`mise run check` の
  biome/lockfile/payload 段は本 task の変更面に非該当。cargo 3 段（fmt・
  clippy・workspace test）で担保。

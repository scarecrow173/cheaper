---
id: "TASK-034KLYFkg2bLCiG6WDNNpx"
type: "task"
status: "done"
title: "exec deny 最終検証 — targeted tests・doc-id・audit_docs・mise run check"
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
        path: "docs/plans/opt-in-exec-deny-patterns.md"
      - type: "file"
        path: "docs/tasks/add-core-security-policy-and-schema-wiring.md"
      - type: "file"
        path: "docs/tasks/plumb-exec-deny-through-tools.md"
      - type: "file"
        path: "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
      - type: "file"
        path: "docs/tasks/document-exec-deny-patterns-profile.md"
      - type: "file"
        path: "docs/tasks/verify-exec-deny-patterns.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
    - "docs/tasks/document-exec-deny-patterns-profile.md"
  blocks: []
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
  defers: []
  deferred-by: []
---

# exec deny 最終検証 — targeted tests・doc-id・audit_docs・mise run check

## Work

plan `docs/plans/opt-in-exec-deny-patterns.md`
（PLAN-034KLWpEpVSlofJ7Idynh5）の T5。T1-T4 で完了した実装が
design の Verification Notes・spec の Acceptance Criteria 通りに
成立していることを、targeted test・TSPEC Rule 対応確認・full
verification pipeline で閉じる。product code は変更しない
（検証と doc status 更新のみ — gap 検出時のみ該当 test file
へ pin 追加）。

### 1. spec Acceptance Criteria ↔ TSPEC Rule の対応確認

spec の Acceptance Criteria 各項目に対応する test が存在し pass
していることを確認する（TSPEC-034KLXPTSxUL1oOQI06HM3 の
Rules に対応 — 欠落があれば該当 task の責務として pin 追加）:

| AC | 対応 test 領域 |
|----|----------------|
| `security.execDenyPatterns` が schema に存在し既定が空 | `schema.rs` unit（schema accept/reject Rule） |
| builtin/user 層で authoring 可・invocation は `InvalidRequest`+`details.layer="invocation"`・project partial は `InvalidRuntimePolicy` | `schema.rs` + `load.rs` unit（trust boundary Rule） |
| 宣言時のみ 3 適用点で deny・`--grant` 合成 grant も対象 | `command.rs`/`load.rs`/`command/mod.rs`/`proc_command.rs`（適用点 1/2/3 + invocation 対象化 Rules） |
| 既定（空）で `python`/`ssh`/`tar` 等が受理 | 既存受理 pin test（既定 no-op Rule — signature 追随で維持） |
| stem 完全一致・case-insensitive（`sh` ≠ `bash`/`sshd`・`python.*` → `python3`/`PYTHON.EXE`） | `security.rs` unit（predicate 意味論 Rule） |
| invalid regex が load 時 `InvalidRuntimePolicy` | `security.rs`/`schema.rs`/`load.rs` unit（全層 fail-closed Rule） |
| 旧 builtin 一覧 + migration seed が docs に記録 | `command.md`/`configuration.md` doc review（docs contract Rule） |
| `mise run check` が通る | §3 |

### 2. 配管の端到端確認（deny 注入漏れの検出）

- `security` raw JSON を持つ options が compiled `ExecDenyList`
  で終端する全経路が test で pin されていること:
  `create_tool_runtime`（適用点 1+2）・
  `create_invocation_tool_runtime`（inner.{security,exec_deny}
  保持）・scope 再注入（inner.security 常時供給）・parity
  check（security mismatch → `invalid_binding`、command
  declared deny → `InvalidRuntimePolicy` inner code 保持）・
  `check_command_execution`（適用点 3）・route viability
  （denied resolved-stem grant → `Viability::NoGrant`）。
- merged deny の attribution gap pin — user deny + `--grant`
  matching grant の `load_policy` error が
  `InvalidRuntimePolicy` かつ `details.layer` を持たないこと
  が明示 test で固定されている（accepted gap を誤って
  「修正」しない）。
- grep sanity — `grep -rn "execDenyPatterns\|exec_deny\|
  ExecDenyList\|security" crates/` で注入点（`policy/
  security.rs`・`schema.rs`・`command.rs`・`load.rs`・
  `registry.rs`・`invocation.rs`・`command/mod.rs`・
  `route/resolve.rs`・`route/supply.rs`・`run/mod.rs`・
  `workflow_run.rs`）が全て繋がっていることを確認。`TODO`・
  `unimplemented!`・stub の残存が無いこと。

### 3. full verification・status 更新

- `mise run check` が pass すること（biome・fmt・clippy
  `--all-targets`・version・doc-ids・`cargo test --workspace`・
  `node --test`・lockfile・apm-payload — `--features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きの
  canonical pipeline）。
- T1-T5 の task status を `done`・plan status を `completed`
  へ更新する（実測結果を各 task の `## Verification` 欄へ
  記録）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/opt-in-exec-deny-patterns.md`
  （PLAN-034KLWpEpVSlofJ7Idynh5 — T5）
- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8 — Verification Notes が
  AUTHORITATIVE）
- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH — Acceptance Criteria）
- `docs/test-specs/opt-in-exec-deny-patterns.md`
  （TSPEC-034KLXPTSxUL1oOQI06HM3 — 全 Rules の最終確認）

## Dependencies

- `docs/tasks/plumb-exec-deny-through-cli-and-runtime.md`
  （T3） — 全 signature 追随・配管が完了し workspace が
  compile していること。
- `docs/tasks/document-exec-deny-patterns-profile.md`（T4） —
  docs contract が完了していること。
- （T1・T2 は T3 経由で推移的に完了前提）

## Affected Files

- `docs/tasks/add-core-security-policy-and-schema-wiring.md`・
  `docs/tasks/plumb-exec-deny-through-tools.md`・
  `docs/tasks/plumb-exec-deny-through-cli-and-runtime.md`・
  `docs/tasks/document-exec-deny-patterns-profile.md`・
  `docs/tasks/verify-exec-deny-patterns.md`（status 更新）
- `docs/plans/opt-in-exec-deny-patterns.md`（status 更新）
- 観測 pin 欠落が見つかった場合のみ、該当 test file へ pin
  追加（本来は T1-T4 の責務 — 本 task で新規 test を書くのは
  gap 検出時に限る）

## Done When

- [x] spec Acceptance Criteria 全項目に対応する test が存在し
      pass（TSPEC Rule との対応表が §1 で確認済み）
- [x] 配管の全経路（scope 構築・invocation parity・再注入・
      route viability・check_command_execution）が deny-aware
      であり、attribution gap pin が存在する
- [x] `mise run check` exits 0
- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `audit_docs` が本 issue の新規 doc で error 0
- [x] 5 task + plan の status が更新されている

## Out of Scope

- product code の追加変更（実装漏れが見つかった場合は該当
  task へ差し戻す — T5 で大規模な実装をやり直さない）
- `process.tasks` deny・docs sweep（#169）・後続分解 issue の
  作業
- `mise run check` 以外の新規検証基盤

## Verification

- [x] 上記 §1-§3 の全項目が pass（コマンド実測を記録）

      targeted cargo test（2026-10-23 実測、全 pass / 0 failed）:

      - `mise exec -- cargo test -p cheaper-core` — 253 pass
        （unit 231 + golden/他 target 22。security.rs 6・
        schema.rs 3・command.rs 1・load.rs 4 の新規 deny test
        を含む）
      - `mise exec -- cargo test -p cheaper-tools` — 357 pass
        （lib 259・characterization 10・differential 11・
        proc_command 35・proc_degrade 9・proc_invocation 17・
        proc_process 9・proc_runtime 7。proc_process の
        `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`
        が初回 run で timeout flake → 単独再実行で pass、#164
        非対象 file の既知 flake 扱い）
      - `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`
        — 350 pass（integration 288・regression 1・
        run_dev_seam 38・workflow 18・workflow_dev_seam 5）
      - `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam --test contracts --test run_dev_seam`
        — 99 pass（contracts 71・run_dev_seam 28）
      - `mise exec -- cargo test -p cheaper-workflow --test compile_contracts --test filesystem_contracts`
        — 89 pass（46 + 43）
      - `mise exec -- cargo test -p cheaper-rig-adapter` —
        21 pass

      §1 対応表の pin 実測（全 pass）— schema accept/reject:
      `security_section_schema_and_normalize`（complete/partial
      受理・project partial 拒否・malformed 拒否）、predicate
      意味論: `security.rs` unit 6 本（stem 正規化・anchor・
      case-insensitive OR・empty no-op・invalid regex
      attribution）、適用点 1: `security_deny_rejects_matching_
      declared_executable` + `declared_executable_deny_is_opt_in_
      and_fail_closed` + `security_exec_deny_applies_at_scope_
      construction`、適用点 2/3: `security_exec_deny_pin_blast_
      radius` + `check_command_execution_applies_exec_deny_at_
      verify`、既定 no-op: `declared_executable_names_pass_
      policy_resolution`（`python`/`ssh`/`tar`/`sh`/`bash`/
      `cmd.exe`/`powershell` 受理）+ `empty_deny_list_is_noop`、
      invocation 対象化 + attribution gap:
      `invocation_grant_hits_user_exec_deny_without_layer_
      attribution`（`InvalidRuntimePolicy`・`details.layer`
      無し pin）+ `invocation_layer_cannot_author_security`
      （`InvalidRequest` + `details.layer="invocation"`）、
      merge replace: `security_exec_deny_patterns_merge_and_
      invocation_scope`、配管: `invocation_resolves_and_
      compiles_security`・`worker_scope_rejects_security_
      policy_mismatch`・`worker_scope_receives_invocation_
      resolved_security`・`worker_scope_denied_declared_keeps_
      inner_error_code`、viability NoGrant:
      `denied_resolved_grant_is_no_grant`（unix gate）、全層
      fail-closed: `security_invalid_regex_fails_closed_at_
      all_layers` + `invalid_exec_deny_regex_fails_load_at_layer`。
      docs contract は `command.md`:34-136・
      `configuration.md`:160-253・`resources/policy.yaml`:64
      の `security` 節を実読確認（旧既定値・migration seed・
      match 規約・trust boundary・非 containment 明記）。

      §2 grep sanity — `security`/`exec_deny` 注入点が
      `policy/security.rs`・`schema.rs`・`command.rs`・
      `load.rs`・`registry.rs`・`invocation.rs`・
      `command/mod.rs`・`route/resolve.rs`・`route/supply.rs`・
      `run/mod.rs`・`workflow_run.rs` に全て接続（`TODO`・
      `unimplemented!`・stub 残存なし）。

- [x] `node scripts/check-doc-ids.mjs` exits 0
      （1132 markdown files scanned）
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で本 issue の新規 doc 7 件（plan・test-spec・task×5）が
      error 0 — `--type plan`（98 files）・`--type test-spec`
      （80 files）・`--type task`（359 files）各 audit とも
      finding は warning（`blocking: false`）のみで error 0。
      warning には本 issue 由来の reciprocal-relation drift と
      tasks/README index の stale status（本 task で更新）を含む
- [x] `mise run check` exits 0（biome 33 files・fmt・clippy
      `--all-targets` + dev-seam・version 1.0.0・doc-ids・
      `cargo test --workspace`（50 test target 全 ok / 0
      failed）・`node --test` scripts 43 + benchmarks 12・
      lockfile・apm payload — 全 step pass）

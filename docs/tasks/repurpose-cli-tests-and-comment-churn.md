---
id: "TASK-034KJu3PCBQDeImZEcznWv"
type: "task"
status: "done"
title: "cheaper-cli の test・regression・comment churn を追随"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/cases/run-grant-invalid-spec.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-grant-invalid-spec.json"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow_dev_seam.rs"
    deleted:
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/cases/run-grant-deny.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-grant-deny.json"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/exec-contract-content-audit-removal.md"
  implemented-by: []
  depends-on:
    - "docs/plans/exec-contract-content-audit-removal.md"
    - "docs/tasks/remove-tools-content-audit.md"
  blocks:
    - "docs/tasks/verify-content-audit-removal.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/exec-contract-content-audit-removal.md"
  references:
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/discovery/exec-contract-content-audit-removal.md"
    - "crates/cheaper-cli/src/policy.rs"
    - "crates/cheaper-cli/tests/run_dev_seam.rs"
    - "crates/cheaper-cli/tests/integration/policy.rs"
  defers: []
  deferred-by: []
---

# cheaper-cli の test・regression・comment churn を追随

## Work

plan `docs/plans/exec-contract-content-audit-removal.md`
(PLAN-034KJtLyNUE46XjtrMOG60)の T3。T1/T2 で除去された内容審査を
観測していた cheaper-cli 側の integration・dev-seam・regression
test と、deny/workspace-resident 前提の doc・test comment を
design の Churn Plan・message/comment churn 表通りに追随させる。
`route/resolve.rs` 本体は触らない(issue 4 scope)。

### 1. `crates/cheaper-cli/src/policy.rs`

- module doc(:8-9)・`write_validated` doc(:245-247)の「deny
  list」言及を除去後の検査内容(argv pattern compile・env/cwd
  規則・`version` const・`maxProperties` 等、現行記述の deny list
  以外の部分)へ restate。

### 2. `tests/integration/policy.rs`

- `policy_grant_add_rejects_invalid_spec_without_writing` を
  **更新** — `{"executable":"python"}` spec は受理されるように
  なるため、`{"executable":"git","timeout":"5x"}`(bounded
  duration 違反 → `INVALID_RUNTIME_POLICY`)へ置き換え。
  `{"argv":["a*b"]}`・`{"argv":["log"]}`(schema 違反)は維持。

### 3. `tests/run_dev_seam.rs`

- `run_fake_stream_grant_deny_listed_executable_rejected`(:907)
  を**再用途化** — `--grant` spec を `{"name":"x",
  "executable":"python","argv":["a*b"]}` へ変更(invocation 層の
  shared validation で `INVALID_RUNTIME_POLICY` +
  `details.layer="invocation"` を deterministic に維持。
  `python` 単体 spec は受理→pin が PATH 依存になるため使わない)。
  rename: `run_fake_stream_grant_invalid_spec_rejected` 系。
  assertion(ok:false・code・layer・exit 3)は不変。
- `self_grant` doc(:359-362)の「workspace-resident 拒否に掛から
  ない」記述を「pin は ambient PATH 非依存で deterministic」の
  趣旨へ訂正。

### 4. regression case rename

- `tests/regression/cases/run-grant-deny.json` +
  `tests/regression/expected/run-grant-deny.json` を**再用途化
  (rename)** — `extraArgs` の spec を `{"name":"deny-me",
  "executable":"python","argv":["a*b"]}` へ変更、case `name` を
  `run_grant_invalid_spec_rejected` 系へ・`files_note` を
  invalid-spec 趣旨(「wildcard 構造違反を含む `--grant` spec は
  run 開始前に reject — inner code INVALID_RUNTIME_POLICY 保持
  + `details.layer=invocation`」系)へ変更し、file pair を
  `run-grant-invalid-spec.json` へ rename。expected の内容は不変
  (`code` + `details.layer` のみ pin で message 非依存)。
  `git mv` ではなく delete+add として commit してよい。

### 5. `tests/workflow.rs`・`tests/workflow_dev_seam.rs`

- `workflow.rs` :505・`workflow_dev_seam.rs` :166 の
  `command_user_policy`/`workflow_command_grant_flag` 系 comment
  「workspace 外のため workspace-resident 拒否に掛からず」記述を
  削除し、「pin は ambient PATH 非依存で deterministic」の趣旨
  だけ残す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/exec-contract-content-audit-removal.md`
  (PLAN-034KJtLyNUE46XjtrMOG60 — T3)
- `docs/designs/exec-contract-content-audit-removal.md`
  (DESIGN-034KIsSCvVMAxGDZY1dQTd — dev-seam/regression/CLI
  integration churn 表・message/comment churn 関連 site が
  AUTHORITATIVE)
- `docs/specs/exec-contract-content-audit-removal.md`
  (SPEC-034KI3mWcW92kMljP5xoa8 — R10)
- `docs/test-specs/exec-contract-content-audit-removal.md`
  (TSPEC-034KJtUqMNXjkbjsbz5YPa — `a*b` repurpose
  determinism・stale-vocabulary Rules)

## Dependencies

- `docs/tasks/remove-tools-content-audit.md`(T2) — 除去が完了して
  いること。本 task が触る旧 spec(`{"executable":"python"}`)は
  T2 完了時点で受理されるため、旧 assertion はすでに失敗状態。

## Affected Files

- `crates/cheaper-cli/src/policy.rs`(modify)
- `crates/cheaper-cli/tests/integration/policy.rs`(modify)
- `crates/cheaper-cli/tests/run_dev_seam.rs`(modify)
- `crates/cheaper-cli/tests/workflow.rs`(modify)
- `crates/cheaper-cli/tests/workflow_dev_seam.rs`(modify)
- `crates/cheaper-cli/tests/regression/cases/run-grant-deny.json`
  (delete → `run-grant-invalid-spec.json` として再作成)
- `crates/cheaper-cli/tests/regression/expected/run-grant-deny.json`
  (delete → `run-grant-invalid-spec.json` として再作成)

## Done When

- [x] `{"executable":"python"}` を使った deny 前提の case/test が
      残らず、再用途先が `a*b`(wildcard 構造違反)または
      `git`+`timeout:"5x"`(duration 違反)で deterministic に
      `INVALID_RUNTIME_POLICY` を pin する
- [x] regression file pair が `run-grant-invalid-spec.json` へ
      rename され expected の code+layer pin が維持される
- [x] cheaper-cli の comment から「workspace-resident 拒否」
      「deny list」前提の記述が消える(`route/resolve.rs:279-281`
      は known-stale として触らない)
- [x] cli の各 test target が pass

## Out of Scope

- `crates/cheaper-core`/`cheaper-tools` の除去(T1/T2)
- `route/resolve.rs`(:279-288 の viability comment・
  `argv_shell_re` 分類)・`route/resolve.rs` test の `a;rm` →
  UnsafeCommand assert(不変 — issue 4 scope)
- `run_fake_stream_*` の他 case・workflow 系 test 本体(前提は
  test binary 自身の grant で不変 — comment のみ churn)
- `proc_invocation.rs` の `self_grant` doc(T2 で実施 — tools 側)
- `resources/policy.yaml`(grants/tasks 空 — 変更不要)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`
      exits 0
- [x] `mise exec -- cargo clippy --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      exits 0(`mise run check` 内で実測)
- [x] `grep -rn "run-grant-deny\|run_grant_deny\|deny_listed\|deny list" crates/cheaper-cli/`
      が 0 hit(`denyPaths`/denylist 語彙を除く)

## Verification results

2026-10-23 実施(Windows / `feature/issue-163-exec-contract-cleanup`)。
T3 commit: `44c9df7 test(cli): repurpose grant-deny cases to
invalid-spec pins`。

| Command | Result |
|---------|--------|
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam` | exit 0(integration 288・regression golden 1・run_dev_seam 38・workflow 18・workflow_dev_seam 5 — 全 pass) |
| `cargo clippy --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | exit 0(`mise run check` 内で実測) |
| stale-name grep(`crates/cheaper-cli/`) | 0 hit |

T4 の stale-vocabulary sweep で case 内 grant name `"deny-me"` の
残存を検出し `"x"` へ rename 済み(expected には echo されず
挙動不変 — T4 commit に含む)。

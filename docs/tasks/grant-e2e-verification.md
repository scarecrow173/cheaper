---
id: "TASK-034FpqBkFDhANMoaYZLw5G"
type: "task"
status: "done"
title: "T5: grant-e2e-verification — dev-seam / regression / parity の結合検証"
created: "2026-10-13"
updated: "2026-10-14"
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
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
    - "docs/tasks/invocation-policy-layer.md"
    - "docs/tasks/grant-flag-and-plumbing.md"
    - "docs/tasks/policy-grant-command.md"
    - "docs/tasks/grant-requests-contract.md"
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
    - "docs/test-specs/invocation-scoped-command-grants.md"
  references: []
  defers: []
  deferred-by: []
---

# T5: grant-e2e-verification — dev-seam / regression / parity の結合検証

## Scope

issue #105 の結合検証 test を整備し、AC1–AC5・AC6/AC7（行為側半分）・
AC8・AC10・AC13–AC15 を網羅する。前提として `tests/common/mod.rs` の
`Fixture::run` を multi-call 対応に修正する（現状は `out.0`/`err.0`
を clear せず蓄積するため 2 回目の呼出しが連結 envelope で
`serde_json::from_str` panic する — `run()` 冒頭で clear-before を
採用し、caller が call ごとの stderr assert を行うため read 後 clear
より自然。`PipeStdin::read_all` が `mem::take` で stdin を消費するため、
2 回目以降の呼出し前に caller が `f.stdin.data` を再装填する必要が
ある旨を doc comment に明記する）。

- In: `Fixture::run` multi-call 修正、`run_dev_seam.rs` の fixture
  test 群、`workflow_dev_seam.rs` の AC14 + file 内 self-reshim
  fixture、`observability_dev_seam.rs` の span field file-sink
  assert、regression corpus case + expected、`proc_invocation.rs`
  parity の無変更 green 確認。
- Out: docs/APM（T6 — 本 task が記述対象挙動の確定を担う）、
  `ollama_e2e.rs` 実 provider e2e（test-spec Non-goals — dev-seam
  fixture + regression corpus が primary 検証）。

### 所有ファイル

- `crates/cheaper-cli/tests/common/mod.rs` — `Fixture::run`（:168-187）
  の multi-call 対応（`run()` 冒頭で `self.out.0`/`self.err.0` を
  clear + stdin 再装填の doc comment）。同一 `Fixture` の連続
  `f.run`（`policy grant` → flag 無し run）が前提とする。
- `crates/cheaper-cli/tests/run_dev_seam.rs` —
  AC1/AC2（`run_fake_stream_grant_flag_un_degrades_command_and_spawns`
  — `opt_in_fixture`/`OPT_IN_COMMAND_WORKER_MD` + `FIXTURE_COMMAND_CALL`
  + `command_grant_flag(marker)` helper が組み立てる flag JSON で
  degrade 解消 + marker spawn・`run_fake_stream_grant_flag_is_not_
  persisted` — flag 無し再実行で再縮退 + `policy.yaml` 不存在）、
  AC3（`run_fake_stream_grant_deny_listed_executable_rejected` —
  `InvalidRuntimePolicy` + `details.layer="invocation"`）、
  AC4/AC13（`run_fake_stream_grant_collides_with_user_policy`・
  `run_fake_stream_grant_duplicate_name_rejected` — `InvalidRequest`
  + `details.grant` + `details.layer`）、
  AC5（`run_fake_stream_tool_operator_blocked_emits_grant_requests` —
  envelope `result` parse + schema 適合）、
  AC6/AC7 の行為側半分（`run_fake_stream_policy_grant_add_then_run_
  executes`・`run_fake_stream_policy_grant_remove_then_run_degrades`
  — `tests/policy.rs` は dev-seam 非 gate target で model session を
  持てないため本 file が所有。`common/mod.rs` の multi-call 修正が
  前提）、
  AC8（`run_fake_stream_grant_emits_stderr_diagnostic` — stderr
  `grant:` 行・`--quiet` 抑制）、
  AC10（`run_fake_stream_tool_operate_alias_accepts_grant` —
  `tool-operate` alias parity）、
  AC15（`run_fake_stream_multiple_grants_injected` — 複数 grant 同時
  注入 + `grant:` 行の宣言順列挙）、
  非露出 warn（`run_fake_stream_grant_on_worker_without_command_
  warns` — `command` 非宣言 worker で warning + exit 0）+
  `tools.allow` 除外の `POLICY_VIOLATION` は既存
  `run_fake_stream_opt_in_capability_denied_is_policy_violation` の
  無変更 green で固定。
- `crates/cheaper-cli/tests/workflow_dev_seam.rs` — AC14（
  `workflow_run_grant_reaches_worker_job` — opt-in worker job で縮退
  解消 + `grant:` stderr 行、`workflow_run_grant_reaches_tool_job` —
  `kind: tool` job の `command` call が `inner.command` 再注入経由で
  grant を受け spawn）。実 spawn を伴うため file 内に
  `workflow_dev_seam_command_fixture_child` self-reshim（`current_exe`
  + `[<fn-name>,--exact,--nocapture]` + `CHEAPER_PROC_CHILD_MODE=probe`
  で marker 書込み）を追加する — `tests/workflow.rs` の
  `workflow_command_fixture_child` は別 test binary のため再利用不可。
- `crates/cheaper-cli/tests/observability_dev_seam.rs` —
  `grants.invocation` span field の確定 assert（`CHEAPER_LOG=debug`
  subprocess の `spawn_logged_run` 経路 + `.cheaper/logs` JSON file
  sink の span record parse で `cheaper.run.tool_scope` の field を
  検証 — `tools.degraded` と同じ sync 区間 record のため
  deterministic。in-process Fixture 経路は global subscriber を
  install できないため subprocess 経路で担保）。
- `crates/cheaper-cli/tests/regression/{cases,expected}/` —
  `run-grant-*.json` case に `extraArgs` の `--grant`（case schema は
  camelCase — `harness.rs` の `rename_all = "camelCase"` で serde
  field は `extraArgs`。snake_case `extra_args` は silent ignore され
  false-green hazard）+ `config.files` の user policy seeding を追加。
  static corpus は positive spawn を表現不能（grant executable は
  絶対・非 workspace path 必須で `current_exe` self-reshim は case
  JSON に書けない）ため、policy-layer 結果のみを固定する: grant
  受理・collision・deny reject + error envelope の `details.layer`・
  `compare_stderr` opt-in による stderr `grant:` 行。expected は
  `CHEAPER_REGRESSION_CAPTURE=1` で生成して diff review。
- `crates/cheaper-tools/tests/proc_invocation.rs` — parity test 群が
  変更無しで green であること（検証のみ、編集なし）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T5）
- `docs/designs/invocation-scoped-command-grants.md` — §(i) Verification
  Notes
- `docs/specs/invocation-scoped-command-grants.md` — AC1–AC5・AC6/AC7・
  AC8・AC10・AC13–AC15
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （Rules の対象 test 名・assert 内容の正本）

## Dependencies

- `docs/tasks/invocation-policy-layer.md`（T1 — invocation layer）
- `docs/tasks/grant-flag-and-plumbing.md`（T2 — `--grant` flag と
  plumbing）
- `docs/tasks/policy-grant-command.md`（T3 — `policy grant` command。
  AC6/AC7 の行為側半分が前提）
- `docs/tasks/grant-requests-contract.md`（T4 — AC5 の schema 適合
  検証が前提）

## Steps

1. `tests/common/mod.rs` の `Fixture::run` を multi-call 対応に修正
   する（clear-before + stdin 再装填 doc comment）。
2. `run_dev_seam.rs` に AC1/AC2・AC3・AC4/AC13・AC5・AC6/AC7 行為側
   半分・AC8・AC10・AC15・非露出 warn の fixture test を追加する。
3. `workflow_dev_seam.rs` に AC14 test + file 内 self-reshim fixture
   を追加する。
4. `observability_dev_seam.rs` に `grants.invocation` span field の
   file-sink assert を追加する。
5. regression corpus に `run-grant-*.json` case を追加し、expected を
   `CHEAPER_REGRESSION_CAPTURE=1` で生成して diff review する。
6. `proc_invocation` parity が無変更 green であることと workspace
   test が pass することを確認する。

## Done When

- [x] `--grant` re-run で opt-in 縮退が解消し実 spawn し、flag 無し
  再実行で再縮退 + `policy.yaml` 不変（AC1・AC2）
- [x] deny/collision/intra-invocation duplicate が run 開始前に正しい
  error 分類 + `details.layer="invocation"` で fail（AC3・AC4・AC13）
- [x] `grantRequests` を含む blocked 出力が schema 適合 + envelope
  `result` parse 可能（AC5）、`policy grant add/remove` 後の flag
  無し run が grant に従い実行/縮退する（AC6・AC7 行為側半分）
- [x] stderr `grant:` 行（`--quiet` 抑制）と `grants.invocation`
  span field（observability file sink）が確定 assert される（AC8）、
  `tool-operate` alias・`workflow run`（worker job + tool job）の
  parity・複数 grant 注入・非露出 warn と `POLICY_VIOLATION` の区別
  が固定される（AC10・AC14・AC15）
- [x] regression case（policy-layer 結果のみ — positive spawn は
  dev-seam のみ）が生成・diff review 済みで、`proc_invocation`
  parity が無変更 green

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --test run_dev_seam --features cheaper-cli/dev-seam` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --test workflow_dev_seam --features cheaper-cli/dev-seam` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --test observability_dev_seam --features cheaper-cli/dev-seam` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --test regression --features cheaper-cli/dev-seam` exits 0（expected 生成は `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression`）
- [x] `mise exec -- cargo test -p cheaper-tools --test proc_invocation` exits 0
- [x] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

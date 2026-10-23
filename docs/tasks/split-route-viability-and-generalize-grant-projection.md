---
id: "TASK-034KVxxl3lkopkjHoRid7V"
type: "task"
status: "done"
title: "viability 分割・argv_shell_re 除去・project_grants set 投影"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
    deleted:
      - type: "item"
        path: "crates/cheaper-core/src/policy/exec.rs::argv_shell_re"
        note: "crates 内唯一の consumer(route/resolve.rs)が外れたため定義ごと削除(R5)"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
    - "docs/tasks/add-route-grants-declaration-and-grant-set.md"
  blocks:
    - "docs/tasks/split-route-record-executed-attribution.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/route-grant-set-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-grant-set-projection.md"
  references:
    - "docs/designs/route-grant-set-projection.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/discovery/route-grant-set-projection-issue-166.md"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
    - "crates/cheaper-tools/src/tools/exec/mod.rs"
  defers: []
  deferred-by: []
---

# viability 分割・argv_shell_re 除去・project_grants set 投影

## Work

plan `docs/plans/route-grant-set-projection.md`
(PLAN-034KVx51kVYoMwO8UmJ8VE)の T2。T1 が追加した
`Route.grants`/`Route::grant_set()` を消費し、viability の
unsafe-command 分岐から `argv_shell_re` を除去・定義削除、
no-grant 段へ member 検査ループ(存在 + `is_route_eligible` +
`pin_exec_grant` → `NoGrant` fold)を追加し、`project_grant` →
`project_grants(&BTreeSet)` へ投影を一般化する。
design(DESIGN-034KUgF5QB3WuzFG6PmVvr)§(b)/(c)/(h)/(i) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

### 1. `route/resolve.rs` — unsafe-command 分岐縮小(:287-291)

- `input.unsafe_command` + `CWD_CHANGING_COMMANDS` の 2 条件のみへ
  縮小(design §(b-1) verbatim — `out.exec_args` の
  `argv_shell_re().is_match` 条件を除去)。
- import(:10)を `use cheaper_core::policy::exec::ResolvedExecPolicy;`
  へ縮小(`argv_shell_re` を落とす)。
- :282-285 の stale comment(「grant 側は metachar を拒否するため
  安全性分類へ倒す」— #163 で grant argv pattern が metachar を
  拒否しなくなった後の誤記述)を design §(b-1) の正確な根拠へ
  書き換え(tokenize 済み argv の metachar 内容は分類しない —
  authorization は grant argv 契約が担い routing 層は内容審査を
  持たない、D2)。
- 効果: quoted metachar(`git log -G 'a|b'`・`"a;rm"`)が
  `UnsafeCommand` でなく grant argv 照合まで到達し `Runnable`
  に成り得る — :488-494 の `a;rm` 期待値は `Runnable` へ反転。
- 維持される structural gate(tokenize-level・3 入口)と
  `CWD_CHANGING_COMMANDS`・`StdinUnsupported`・`is_interactive`
  は不変。

### 2. `route/resolve.rs` — no-grant 段 member ループ(:351 直後)

- anchor `route.grant` の既存完全検査(lookup + eligibility →
  `pin_exec_grant` → `ExecToolOptions` → `check_exec_execution`)
  は**一切変更せず**、直後(stdin-unsupported :352-355 より前)
  に design §(b-2) verbatim の member ループを挿入:
  `for member in &route.grants` — `input.exec_policy.policy.
  grants.get(member)` + `is_route_eligible` filter →
  `pin_exec_grant(member, member_grant, root, symlink_policy,
  denylist, exec_deny)`、いずれか失敗で `return Viability::NoGrant`。
- member には `check_exec_execution`・`verify_pinned_executable`・
  cwd prepare を呼ばない(pin 解決結果は検査のみで破棄 — member
  argv は宣言されていないため照合対象が存在しない、O2a)。
- 実装注記: 失敗 member 名を `tracing::debug!` へ載せると
  viability 値上は anchor 失敗と区別不能でも debug log で帰属
  できる(record・`route resolve` 出力 shape は不変 — O3a)。
- test literal(:369-380 — T1 で `grants` field 追随済み)の
  member viability case 追加 + :488-494 期待値反転。

### 3. `policy/exec.rs` — `project_grants` 一般化 + `argv_shell_re` 削除

- `project_grant`(:118-130)を design §(c) verbatim の
  `pub fn project_grants(&mut self, names: &BTreeSet<String>)
  -> bool` へ置換 — `names.is_empty()` または 1 件でも
  未存在/非 `is_route_eligible` で false(policy 不変)、
  通過時 `self.grants.retain(|n, g| names.contains(n) || g.fixed)`
  で `{names} ∪ {fixed grants}` を残す。呼出し側は
  `route/run.rs:121` のみのため第 2 API を増やさず signature を
  置き換える。
- `names` は caller が `route.grant_set()` で供給する有効 set —
  anchor を含まない集合を渡すと投影から anchor が落ちる旨の
  doc comment を verbatim で維持。
- `argv_shell_re`(:144-150)の定義 + comment を削除 —
  `resolve.rs` の import 除去後は crates 内 consumer が存在
  しない(spec AC — `grep -r argv_shell_re crates/` が 0 件)。
- `is_route_eligible`(:112-116)・`pin_exec_grant(s)`・
  `check_exec_execution`・`authorize_args` は不変 — member も
  anchor と同じ seam を通る。
- `route_eligibility_and_project_grant` test(:1039-1062)を
  set 呼出し + multi-member case へ更新。

### 4. `route/run.rs` — 投影呼出し(:121-123)

- `project_grant(&route.grant)` →
  `project_grants(&route.grant_set())`(import 追加不要 —
  `grant_set()` の戻り型がそのまま渡る)。false なら
  `denied("no-grant")` `PolicyViolation` 不変。
- `prepare_projects_grants_to_single_route_grant` test
  (:701-719)を set 投影名・内容へ更新。

### 5. test churn + 新規 test(TSPEC の Rules に対応)

| file | 変更 |
|------|------|
| `resolve.rs` unit | member 未存在/project-scope/fixed/pin 不能 → `NoGrant`・member 全通過 → `Runnable`・`a;rm` 期待値 `Runnable` 化・`\0` 含 arg が unsafe-command でなく runnable 到達可 |
| `exec.rs` unit(:1039-1062) | `project_grants` set 呼出し — `{a,b}` 全 eligible で `{a,b}∪{fixed}` retain・1 件 ineligible で false・空集合で false・policy 不変 |
| `run.rs` test(:701-719) | prepare の set 投影 — `grant_set()` 全件 retain・undeclared 名 job 拒否前提の投影 policy shape |
| `tests/integration/route.rs`(:8-19,:50-118) | `grants` 宣言 fixture + member 未存在/fixed → `no-grant`・quoted metachar → `runnable` case |
| `tests/integration/route_run.rs` | `grants:[a,b]` 宣言 route の routed workflow で `input.name` が `a`/`b` の `job_id != "exec"` command job が実行できる・undeclared 名 job が `PolicyViolation` で拒否される・member argv pattern 外 args が job 実行時 `check_exec_execution` で拒否されるが route 発火は止まらない・`job_id=="exec"` で member 名を指定すると `exec-input-mismatch` |
| `tests/proc_invocation.rs`(:706-720) | `project_grant` 相当 comment を set 投影へ更新 + multi-grant 投影 case(投影外 `input.name` 拒否は既存構造の回帰) |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/route-grant-set-projection.md`
  (PLAN-034KVx51kVYoMwO8UmJ8VE — T2)
- `docs/designs/route-grant-set-projection.md`
  (DESIGN-034KUgF5QB3WuzFG6PmVvr — §(b)/(c) verbatim・§(f)
  confused-deputy 境界・§(i) error 契約が AUTHORITATIVE)
- `docs/specs/route-grant-set-projection.md`
  (SPEC-034KTMfXYD5ZlGZCRb8Paj — R2/R3/R4/R5/R8)
- `docs/test-specs/route-grant-set-projection.md`
  (TSPEC-034KVxaBMzQE5YtVOx4j3w — viability 分割・
  argv_shell_re 除去・投影境界 Rules)
- `docs/tasks/add-route-grants-declaration-and-grant-set.md`
  (TASK-034KVxrNr5J7VrsdNaZDAP — 前提)

## Dependencies

- `docs/tasks/add-route-grants-declaration-and-grant-set.md`
  (T1) — `Route.grants`・`Route::grant_set()`・literal churn が
  必要。
- 既知の transient: `project_grant` → `project_grants` の
  signature 置換は `route/run.rs:121` と同 commit で追随させる
  (呼出し側は 1 件のみ — 第 2 API を残さない)。本 task の検証は
  core/cli/tools の 3 crate test に scope する。

## Affected Files

- `crates/cheaper-cli/src/route/resolve.rs`(modify)
- `crates/cheaper-cli/src/route/run.rs`(modify — :121-123 + test)
- `crates/cheaper-core/src/policy/exec.rs`(modify + `argv_shell_re` 削除)
- `crates/cheaper-cli/tests/integration/route.rs`(modify)
- `crates/cheaper-cli/tests/integration/route_run.rs`(modify)
- `crates/cheaper-tools/tests/proc_invocation.rs`(modify)

## Done When

- [ ] unsafe-command 分岐が `input.unsafe_command` +
      `CWD_CHANGING_COMMANDS` の 2 条件のみで、stale comment が
      正確な根拠へ書き換わり、`argv_shell_re` の定義・import・
      参照が crates 内に残らない(`grep -r argv_shell_re crates/`
      0 件)
- [ ] anchor 完全検査が不変で、member ループが anchor 直後に
      存在 + `is_route_eligible` + `pin_exec_grant` のみを検査し
      失敗は `Viability::NoGrant` へ fold する(新規 viability
      値・enum 順序・出力 shape の変更なし)
- [ ] `project_grants(&BTreeSet)` が `{names} ∪ {fixed}` を
      retain し、空集合・1 件 ineligible で false(policy 不変)、
      `route/run.rs` が `project_grants(&route.grant_set())` を
      呼ぶ
- [ ] quoted metachar argv が `runnable` になり、`\0` 含 arg
      が runnable 到達可 → spawn `InvalidInput` の bounded job
      error として pin されている
- [ ] member job が declared set 内を `input.name` で実行でき、
      undeclared 名が `PolicyViolation`、member argv 不一致が
      実行時 `check_exec_execution` で拒否(route 発火は不発に
      ならない)、`job_id=="exec"` + member 名が
      `exec-input-mismatch` になる各 case が存在する
- [ ] `cargo test -p cheaper-core`・`-p cheaper-cli`・
      `-p cheaper-tools` が pass

## Out of Scope

- record `executed`・`RouteRecordInput`・finish() 抽出(T3 —
  `run.rs` の :465 以降・:575-588 は T3 が編集)
- docs・routes.yaml comment(T4)
- viability enum/優先順位の変更・`route resolve` `grants` echo・
  pin cache 新設・member timeout の max-of-set 化
- `workflow_run.rs`・`route_cmd.rs`・`supply.rs` 本体(不変の
  回帰対象)

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` exits 0
      (exec.rs unit — single-crate では dev-seam feature 適用外) —
      実測: 244 通常 + 22 doc/additional test pass(commit 471865b 時点)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route` exits 0(route::・route_run:: の新規 case 含む) — 実測: `route::resolve_member_grants_viability` を含む integration target 全件 pass、最終 `mise run check` でも 0 failed
- [x] `mise exec -- cargo test -p cheaper-tools --test proc_invocation` exits 0 — 実測: 17 test pass(`grant_set_projection_preserves_scope_parity` 含む、`mise run check` 内)
- [x] `grep -r argv_shell_re crates/` が 0 件 — 実測: 0 件
- [x] `mise exec -- cargo clippy -p cheaper-core -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 — 実測: `mise run check` の `cargo clippy --workspace --all-targets` が exit 0

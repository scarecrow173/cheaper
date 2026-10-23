---
id: "PLAN-034KVx51kVYoMwO8UmJ8VE"
type: "plan"
status: "approved"
title: "route grant-set projection — grants 宣言・viability 条件緩和・record split"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    deleted:
      - type: "item"
        path: "crates/cheaper-core/src/policy/exec.rs::argv_shell_re"
        note: "crates 内唯一の consumer(route/resolve.rs)が外れたため定義ごと削除(R5)"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/route-grant-set-projection.md"
  implemented-by:
    - "docs/tasks/add-route-grants-declaration-and-grant-set.md"
    - "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
    - "docs/tasks/split-route-record-executed-attribution.md"
    - "docs/tasks/document-route-grant-set-projection.md"
    - "docs/tasks/verify-route-grant-set-projection.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/route-grant-set-projection-issue-166.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/discovery/route-grant-set-projection-issue-166.md"
  derived-by:
    - "docs/tasks/add-route-grants-declaration-and-grant-set.md"
    - "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
    - "docs/tasks/split-route-record-executed-attribution.md"
    - "docs/tasks/document-route-grant-set-projection.md"
    - "docs/tasks/verify-route-grant-set-projection.md"
  verifies: []
  verified-by:
    - "docs/test-specs/route-grant-set-projection.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/guides/usage.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route/payload.rs"
    - "crates/cheaper-cli/src/route_cmd.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
    - "crates/cheaper-tools/src/tools/exec/mod.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-cli/resources/routes.yaml"
    - "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
  defers: []
  deferred-by: []
---

# route grant-set projection — grants 宣言・viability 条件緩和・record split Implementation Plan

## Goal

DESIGN-034KUgF5QB3WuzFG6PmVvr の実装契約に従い、byte-matched
route の exec 委譲面を単一 `grant` から宣言 set `{route.grant} ∪
grants` へ拡張する。構成は 4 面: (1) routes.yaml route entry の
optional `grants: [name,…]`(冪等 dedupe・`[]` ≡ 未指定・宣言は
builtin/user 層限定で project 層 presence は fail-loud
`InvalidRuntimePolicy` route-load error — O7a)+
`Route::grant_set()`、(2) viability 分割(anchor `route.grant` の
完全検査を不変で維持しつつ、member は存在 + `is_route_eligible`
+ `pin_exec_grant` のみ → 失敗は `NoGrant` 一種へ fold)と unsafe-
command 分岐からの `argv_shell_re` 除去 + 定義削除、(3)
`project_grant` → `project_grants(&BTreeSet)` 一般化による
`{route.grant} ∪ grants ∪ {fixed}` 投影、(4) `kind:"route"`
record への additive `executed:{grant,argv}` 追加
(`schemaVersion: 1` 不変・captured `details.{grant,argv}` 由来・
欠落は `InternalError`)。`expected_input.name = route.grant` の
anchor 規約と member job の `job_id != "exec"` authoring 規約は
code 変更なしで契約固定する。

## Scope

単一の implementation stream。「宣言 set 投影」という 1 つの
behavioral 変更に、schema/declaration・viability・projection・
record の 4 面と signature churn(`Route` literal・
`project_grant` 改名・`RouteRecordInput` field)が横断的に付随
する構成であり、API 形状・error 契約・churn 一覧は design が
file:line 精度で固定済み — 分割するのは commit の論理境界
(route schema → viability/projection → record → docs → 検証)
のみで、plan は 1 つに収める。

| In | Out |
|---|---|
| `Route.grants: Vec<String>` field・`KNOWN_KEYS` への `"grants"` 追加・`validate_route` の parse + dedupe + project 層 presence 拒否(`:309` の既存 `layer` param)・`Route::grant_set()` | substitute mode(#167)— `expected_input`/`executed` anchor 切替・`substitute.grant` 宣言・非 byte-matched 起動形(spec Deferred と同一) |
| viability の unsafe-command 分岐を caller flag + `CWD_CHANGING_COMMANDS` のみへ縮小 + `argv_shell_re` 定義削除 + stale comment 訂正 | `route resolve` 出力への `grants` echo(R11 出力 shape `route_cmd.rs:105-128` は不変) |
| no-grant 段の member 検査ループ(存在 + `is_route_eligible` + `pin_exec_grant` → `NoGrant` fold) | viability enum・優先順位の変更 — `NoGrant` fold のみで新規値を追加しない(O3a) |
| `project_grant` → `project_grants(&BTreeSet)` 置換 + `route/run.rs:121` 呼出し追随 | viability 事前検査と scope 構築 pin の二重評価 gap を埋める pin cache 新設(現行 anchor と同じ race domain — KISS) |
| `RouteRecordInput` への `executed_grant`/`executed_argv`・`build_route_record` の `executed` 挿入・`route/run.rs` finish の `details.{grant,argv}` 抽出(欠落 `InternalError`) | captured 前 exec job 失敗経路の record(record timing は captured result 必須の現行どおり)・`usage_report` code 変更(additive field は無視) |
| anchor 規約の明文化(`expected_input.name = route.grant`・member job `job_id != "exec"`・member は宣言 lifecycle/cwd/env/exitCodes・route context 非適用) | member job への routed-exec context 拡張・workflow deadline の max-of-set 化・`grants` 宣言数上限(全て Deferred) |
| unit/integration test・`resources/routes.yaml` shape comment・docs 4 file + pretooluse spec の content update | `route/supply.rs`・`route_cmd.rs`・`route/payload.rs`・`workflow_run.rs`・`usage_report.rs`・`hook.rs`/`hook_cmd.rs` 本体(code 不変の回帰対象) |

## Upstream Documents

- `docs/specs/route-grant-set-projection.md`
  （SPEC-034KTMfXYD5ZlGZCRb8Paj、approved — R1-R8・Acceptance
  Criteria）
- `docs/adr/route-grant-set-projection.md`
  （ADR-034KTMnPFDF25x710LFSKY、accepted — O1a-O7a 全採用:
  `grant` required + optional `grants`・anchor 完全検査 +
  member 軽量検査・`is_route_eligible` 単一 seam・NoGrant fold・
  `argv_shell_re` 除去・additive `executed`・`expected_input`
  anchor・trusted 層限定宣言）
- `docs/discovery/route-grant-set-projection-issue-166.md`
  （DISC-034KTzzHPIbmtD8j3iHBtg、resolved — 全接点の実測
  inventory）

## Design Inputs

- `docs/designs/route-grant-set-projection.md`
  （DESIGN-034KUgF5QB3WuzFG6PmVvr、approved — AUTHORITATIVE な
  HOW: (a) route declaration(schema verbatim rust・層制約・
  `grant_set`)・(b) viability(unsafe 分岐縮小 + member ループ
  verbatim)・(c) `project_grants` signature・(d) record split
  (`RouteRecordInput`/`build_route_record`/finish 抽出
  verbatim)・(e) anchor 規約・(f) confused-deputy 境界・(g)
  signature 契約表・(h) call site 配線全一覧・(i) error 契約表・
  Churn 一覧・Docs 更新契約・Verification Notes）。
  本 plan・task は design の行指定を転記するだけで、API 形状・
  error 文言・適用位置を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/route/routes.rs` | `Route.grants` field(:37-55 の `grant` 直後)・`KNOWN_KEYS`(:314-322)に `"grants"`・`validate_route` の grants parse + dedupe(:399 `grant` 確定直後)+ project 層 `grants` presence 拒否(`layer` param 判別、`schema_error` → `InvalidRuntimePolicy`)・`Route::grant_set()` 新設・literal 構築(:451-460)・unit test | T1。declaration 層の全責務をこの file に閉じ込める |
| `crates/cheaper-cli/src/route/resolve.rs` | `Route` literal(:369-380)へ `grants` 追加(T1 の field churn)、import(:10)縮小・stale comment(:282-285)訂正・`argv_shell_re` 条件除去(:287-291)・no-grant 段の member ループ(:351 直後)・`:488-494` の `a;rm` 期待値 `Runnable` 化 + member viability case | T1(literal のみ)/T2(本体) |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | `Route` literal(:2080-2091)へ `grants: Vec::new()` 追加のみ | T1(field churn — 評価本体は不変) |
| `crates/cheaper-core/src/policy/exec.rs` | `project_grant` → `project_grants(&BTreeSet<String>)`(:118-130 — `{names} ∪ {fixed}` retain・空/非 eligible で false)・`argv_shell_re` 定義 + comment 削除(:144-150)・test(:1039-1062)を set 呼出し + multi-member case へ | T2。`is_route_eligible`・`pin_exec_grant(s)`・`check_exec_execution`・`authorize_args` は不変 |
| `crates/cheaper-cli/src/route/run.rs` | `project_grant(&route.grant)` → `project_grants(&route.grant_set())`(:121-123)・finish()(:465 以降)の `details.{grant,argv}` 抽出(欠落・非 string は `InternalError` — `exitCode` と同一 invariant)・`RouteRecordInput` literal(:575-588)・`prepare_projects_grants…` test(:701-719)・finish test `completed_exec_result`(:899-912)の `executed` assertion | T2(投影 call)/T3(record 入力) |
| `crates/cheaper-cli/src/usage_log.rs` | `RouteRecordInput`(:332-357)へ `executed_grant: String`・`executed_argv: Vec<String>`・`build_route_record`(:362-404)の `commandHash` 直後へ `executed:{grant,argv}` 必須挿入(`schemaVersion:1` 不変) + test | T3。record 形状の owner — `{grant,argv}` object 組み立てを caller へ漏らさない |
| `crates/cheaper-tools/tests/proc_invocation.rs` | `project_grant` 相当 comment(:706-720)を set 投影へ更新 + multi-grant 投影 case(投影外 `input.name` 拒否は既存構造の回帰) | T2 |
| `crates/cheaper-cli/tests/integration/route.rs` | `grants` 宣言 fixture(:8-19)+ member 未存在/fixed → `no-grant`・quoted metachar → `runnable` case(:50-118) | T2 |
| `crates/cheaper-cli/tests/integration/route_run.rs` | member job が declared set 内を `input.name` で呼べる・undeclared 名が `PolicyViolation` で拒否される routed workflow case・record `executed` case | T2/T3 |
| `crates/cheaper-cli/tests/integration/usage_log.rs` | `route_record_shape`(:299-338)に `executed:{grant,argv}` shape + `commandHash`/`schemaVersion` 不変の assertion | T3 |
| `crates/cheaper-cli/resources/routes.yaml` | `grant:` 直下へ `grants` の shape comment 追加(宣言面記述のみ — builtin route は member 非宣言) | T4 |
| `docs/guides/{parent-agent-integration,configuration,command,usage}.md` | routes.yaml `grants` field 説明 + trusted 層限定(project 拒否)・member eligibility/argv bounds/job authoring 規約・record `executed` additive field + `executed.argv` 平文 arg 露出の明記 | T4。design Docs 更新契約の 4 file |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | route field 一覧へ `grants?`(trusted 層限定)・viability 記述へ member 検査・record field へ `executed` の content update | T4。content update のみ — relation は張らない |

## Implementation Sequence

- [ ] TASK-034KVxrNr5J7VrsdNaZDAP (T1): route declaration —
      `Route.grants` field + `KNOWN_KEYS` + `validate_route` の
      grants parse(member dedupe・self-ref 除去・形状 error)+
      project 層 `grants` presence → `InvalidRuntimePolicy`
      schema_error + `Route::grant_set()` + 全 `Route` literal
      構築追随(routes.rs・resolve.rs・routing_policy.rs)+
      routes.rs unit test。`cargo test -p cheaper-cli routes`
      scope で閉じる
- [ ] TASK-034KVxxl3lkopkjHoRid7V (T2): viability/projection —
      `resolve.rs` の unsafe-command 分岐縮小 + comment 訂正 +
      member 検査ループ(NoGrant fold)+ `exec.rs` の
      `project_grants` 置換 + `argv_shell_re` 定義削除 +
      `run.rs:121` 呼出し追随 + resolve/exec/run/proc_invocation
      test 更新 + `route.rs` integration case。
      `cargo test -p cheaper-core`・`-p cheaper-cli`・
      `-p cheaper-tools` で閉じる
- [ ] TASK-034KVxxS09XbljJlhLD9Ev (T3): record split —
      `RouteRecordInput` 2 field + `build_route_record` の
      `executed` 挿入 + `run.rs` finish の `details.{grant,argv}`
      抽出(欠落 `InternalError`)+ usage_log unit/integration
      test + `route_run.rs` record case。
      `cargo test -p cheaper-cli` で閉じる
- [ ] TASK-034KVxDgWqxNof9qYv0dbf (T4): docs —
      `parent-agent-integration.md`・`configuration.md`・
      `command.md`・`usage.md`・`resources/routes.yaml` comment・
      pretooluse spec content update。T1 後なら T2/T3 と並列
      実行可能
- [ ] TASK-034KVxDMCEUdI5WtH1SJxd (T5): 最終検証 — targeted
      cargo tests・`grep -r argv_shell_re crates/` 0 件・
      `node scripts/check-doc-ids.mjs`・`audit_docs`・
      `mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 route schema + grant_set | — | T2, T4 |
| T2 viability + projection + argv_shell_re | T1 | T3 |
| T3 record split | T2 | T5 |
| T4 docs | T1 | T5 |
| T5 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T3 → T5。T4 は T1 完了後に T2/T3 と
並列実行可能(docs は compile 依存を持たない)。

既知の transient compile 断:

- T1 境界 — `Route` struct への `grants` field 追加は全 literal
  構築(routes.rs:451-460・resolve.rs:369-380・
  routing_policy.rs:2080-2091)の同 commit 追随が必須。T1 で
  cli crate 全体を compile 可能に保つ(`cargo test -p
  cheaper-cli` が通る状態で閉じる)。
- T2 境界 — `project_grant` → `project_grants` の signature 置換は
  `route/run.rs:121` と同 commit で追随させる(呼出し側は 1 件のみ
  — 第 2 API を残さない)。`resolve.rs` の member ループは
  `route.grants`(T1 成果物)を読むため T1 前提。T2 の検証は
  core/cli/tools の 3 crate test に scope。
- T3 は T2 と独立に compile 可能だが `route/run.rs` の同一 file
  を T2(:121)と T3(:465 以降・:575-588)で編集するため直列化し、
  T2 の投影変更を先に着地させる。

各 task は 1 commit で、file-local の unit test・doc comment は
同 commit で追随する。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1/T2 commit 境界の transient compile 断(`Route` field・`project_grants` signature — 必発) | high(必発) | low | design 確定の churn 一覧どおり literal/呼出しを同 commit で追随 — T1 は cli crate 全体・T2 は 3 crate を検証 scope に |
| 2 | member 失敗と anchor 失敗が `no-grant` で区別不能(診断性低下) | high(確定値) | low | spec/ADR 確定の fail-safe fold — `tracing::debug!` へ失敗 member 名を載せ debug log で帰属可能にする(design §(b-2) 実装注記)。`grants` echo は Deferred |
| 3 | member pin の scope 構築時再失敗 — viability 通過後の PATH 変動/policy 更新で `pin_exec_grants` が投影全 `!fixed` grant を pin し直し、当該 member を呼ばなくても routed run 全体が `PolicyViolation` で hard error | med | med | 新規失敗 domain として design §(b-2)/(i) で受理済み — `verify_pinned_executable` の spawn 前検出と同じ race domain。pin cache は新設しない |
| 4 | `\0` 含 arg が `Runnable` まで到達し `route run` → spawn `InvalidInput` で exec job が fail する(fail 位置の変化 — 旧 `UnsafeCommand` fallback ではない) | low | low | O5b 不採用の確定帰結 — viability 層へ `[\0\r\n]` regex を再導入しない。bounded job error として test で固定(TSPEC Rule) |
| 5 | `executed.argv` が usage.jsonl へ初めての平文 arg 値を記録する(機微値露出) | high(必発) | low | design Risk で受理済み — `commandHash` は一方向 hash のまま・env-prefix 値は prefixed argv 拒否で混入しない。parent-agent-integration・usage.md で露出を明記(T4 の必須記述) |
| 6 | `details.{grant,argv}` 欠落を silent に許すと record attribution が壊れる — 一方で gate 保証外の経路から来た場合に過剰に fail する誤 InternalError | low | med | `expected_input` gate が `job_id=="exec" && tool=="command"` に `details.{grant,argv}` を保証済み(ProcIdentity が `command::execute` で書込み)— `exitCode` 欠落(:467-474)と同一 invariant として `InternalError` で pin。test は欠落 case を明示 |
| 7 | golden/test fixture の `Route` literal・`RouteRecordInput` literal 追随漏れ | med | low | compiler が全 literal site を捕捉 — design Churn 一覧の行番号で網羅。`grep -r "Route {" crates/` sanity は T5 |
| 8 | project 層 `grants` presence 拒否を member 形状検査より後に置くと untrusted 層の malformed 内容を評価してしまう | low | med | design §(a) 確定の順序(presence → 形状)を task の Done When で pin — fail-closed を優先 |

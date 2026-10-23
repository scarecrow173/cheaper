---
id: "PLAN-034L2MF3X78MvKvcSN8jmV"
type: "plan"
status: "approved"
title: "substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・実行 anchor 切替"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
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
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/substitute-exec-route-mode.md"
  implemented-by:
    - "docs/tasks/add-route-substitute-declaration-and-exec-pair.md"
    - "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
    - "docs/tasks/document-substitute-exec-route-mode.md"
    - "docs/tasks/verify-substitute-exec-route-mode.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
  derived-by:
    - "docs/tasks/add-route-substitute-declaration-and-exec-pair.md"
    - "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
    - "docs/tasks/document-substitute-exec-route-mode.md"
    - "docs/tasks/verify-substitute-exec-route-mode.md"
  verifies: []
  verified-by:
    - "docs/test-specs/substitute-exec-route-mode.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/plans/route-grant-set-projection.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/route-run-delegated-exec.md"
    - "docs/test-specs/route-usage-records.md"
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

# substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・実行 anchor 切替 Implementation Plan

## Goal

DESIGN-034L0RxGVVTOqtaUUThowk の実装契約に従い、route が
typed command を実行せず宣言済みの substitute ペア
`{substitute.grant, substitute.argv}` を実行する
**substitute-exec mode** を導入する(非 exec routing deferred
sketch の採用要素 Option D — ADR-034KtTzOJGqmzmBW3SwGp5)。
構成は 4 面: (1) routes.yaml route entry の optional
`substitute: {grant, argv}`(required ペア・nested 未知 key
拒否・`argv: []` 合法・trusted 層(builtin/user)限定で
project 層 presence は fail-loud `InvalidRuntimePolicy`
route-load error — O1a/O3a)+ `Route::grant_set()` への
`substitute.grant` 追加 + `Route::exec_pair()` 新設、
(2) viability の anchor 完全検査を `exec_pair` 経由の実行ペア
へ retarget し substitute mode の `route.grant` を member 級
検査(存在 + `is_route_eligible` + pin)へ降格 + stdin
分類へ substitute ペアの宣言名∪解決名 union 評価追加
(O2a/O6a)、(3) `route/run.rs` の `exec_pair` 配線
(`expected_input`・deadline・inputs `grant`/`execArgs` を
実行ペアへ再 anchor — O4a)、(4) merge で project
`trusted:true` 置換の displaced route を `suppressed`
provenance へ記録。record・exit mirror・convention は code
不変のまま substitute ペアを運ぶ(O5a/O8a)。

## Scope

単一の implementation stream。「実行対象を route 宣言の
substitute ペアへ切り替える」という 1 つの behavioral 変更に、
schema/declaration・viability・run 配線・merge provenance の
4 面と signature churn(`Route` literal)が横断的に付随する
構成であり、API 形状・error 契約・churn 一覧は design が
file:line 精度で固定済み — 分割するのは commit の論理境界
(route schema → viability → run 配線 → docs → 検証)のみで、
plan は 1 つに収める。

| In | Out |
|---|---|
| `RouteSubstitute {grant, argv}` struct・`Route.substitute` field・`KNOWN_KEYS` への `"substitute"` 追加・`validate_route` の nested object parse(required ペア・nested 未知 key・non-empty 要素)+ project 層 presence 拒否(`:329` の既存 `layer` param — `grants` O7a と同型) | Option E(exec-less route)— exec job 不要の routed workflow・`outputs.exitCode` 等の workflow 宣言値 exit mirror(親 chain の open 事項、R11/O8a) |
| `Route::grant_set()` への `substitute.grant` 追加(`{route.grant} ∪ grants ∪ {substitute.grant}`)+ `Route::exec_pair()` 新設(実行ペア選択の単一 owner) | workflow 宣言の substitute・typed args の substitute argv への転送・補間(宣言凍結 — R12) |
| viability の anchor 完全検査を `route.exec_pair(&out.exec_args)` へ retarget + member ループへの `route.grant` 先頭連結(member 級降格・`NoGrant` fold)+ stdin 分類への substitute union 評価 | viability enum・`route resolve` 出力 shape の変更 — substitute 失敗は `NoGrant` fold、resolve echo なし(R11) |
| `route/run.rs` の `exec_pair` 配線(grant_timeout lookup :361・`RouteExecution::new` :378-379・inputs :385/:392) | record schemaVersion bump・新規 record field・firing-gate anchor grant 明示 field(O5a/O5b — `executed` は実測 echo で auto-capture) |
| merge の `RouteLayer::Project` 置換 branch で displaced route を `suppressed` provenance へ記録(`SuppressedRoute` semantics を「merge 敗北 route」へ拡張) | builtin/user 層間 shadow の suppressed 記録(同一 trusted principal の宣言 override — silent のまま) |
| 旧 semantics pin 2 test の assert 更新(`routes.rs` unit `user_shadows_builtin_and_project_needs_trusted`・`route_run.rs` `suppressed_project_provenance_reaches_route_record`) | non-runnable best match からの shorter match への fall-through(match precedence 全体の別設計 — known semantics として pin のみ) |
| unit/integration test・`resources/routes.yaml` shape comment・docs 4 file + pretooluse spec content update・comment 追随(routes.rs/resolve.rs/run.rs/exec.rs) | `grants[]` member の interactive screening・member argv の route 解決時照合・`is_route_eligible`/`project_grants`/`check_exec_execution`/`pin_exec_grant`/`RouteExecution::new`/`validate_routed_convention` の signature/predicate 変更(全て不変) |

## Upstream Documents

- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h、approved — R1-R12・
  Acceptance Criteria AC1-AC12)
- `docs/adr/substitute-exec-route-mode.md`
  (ADR-034KXk8h1AtwCkf2cQrb7m、accepted — O1a-O8a 全採用:
  nested `{grant,argv}` object・anchor 完全検査の substitute
  retarget + `route.grant` member 級降格・trusted 層限定宣言・
  `expected_input`/inputs の実行ペア再 anchor・record shape
  不変・stdin union 分類・投影 `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}`・Option E 残置)
- `docs/discovery/substitute-exec-route-mode-issue-167.md`
  (DISC-034KWGjJMiSKU5wBxcznho、resolved — 全接点の実測
  inventory)

## Design Inputs

- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk、approved — AUTHORITATIVE な
  HOW: (a) route declaration(`RouteSubstitute` verbatim rust・
  層制約・`grant_set`/`exec_pair`・merge suppressed 記録)・
  (b) viability(retarget verbatim・member ループ連結・stdin
  union)・(c) projection(code 不変の確認)・(d) 実行 anchor
  配線 verbatim・(e) anchor 規約・(f) confused-deputy 境界・
  (g) signature 契約表・(h) call site 配線全一覧・(i) error
  契約表・Churn 一覧・Docs 更新契約・Verification Notes)。
  本 plan・task は design の行指定を転記するだけで、API 形状・
  error 文言・適用位置を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/route/routes.rs` | `RouteSubstitute` struct 新設 + `Route.substitute` field(`grants` :52 直後)・`KNOWN_KEYS`(:334-343)に `"substitute"`・`validate_route` の project 層 presence 拒否 + `{grant,argv}` parse(:454 直後)・`Route` literal(:506-516)・`grant_set()` union + `exec_pair()` 新設(:64-75)・merge project `trusted:true` 置換で displaced → `suppressed` 記録(:154-166)・unit test(:574 以降 — substitute parse/error + project 層拒否 + builtin/user 受理 + `grant_set` union + `exec_pair` 選択 + merge provenance)+ `user_shadows_builtin_and_project_needs_trusted`(:710)assert 反転 | T1。declaration 層の全責務をこの file に閉じ込める |
| `crates/cheaper-cli/src/route/resolve.rs` | anchor 完全検査を `route.exec_pair(&out.exec_args)` へ retarget(:305-348 — lookup 名・pins key・`check_exec_execution` 引数)・member ループへ substitute mode のみ `route.grant` 先頭連結(:353-377 — ループ本体不変)・stdin 段へ substitute union 分類追加(:378-381 — 宣言名 `command_name(&grant.executable)` ∪ 解決名 `command_name(&pinned)`)・test module `use`(:387-390)へ `RouteSubstitute` import + `route()` literal(:395-407)へ `substitute: None` + substitute viability case | T1(literal のみ)/T2(本体) |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | `route()` test literal(:2080-2091)へ `substitute: None` 追加のみ(評価本体 :1451-1486 は不変) | T1(field churn) |
| `crates/cheaper-cli/src/route/run.rs` | `exec_pair` binding 新設 + grant_timeout lookup(:361)・`RouteExecution::new` 呼出し(:378-379)・inputs `grant`/`execArgs`(:385/:392)の実行ペア化・`prepare_projects_grant_set` test(:733-770)の substitute 含有 case + deadline 選出 test | T3 |
| `crates/cheaper-cli/tests/integration/route.rs` | substitute 宣言 fixture・viability retarget case・project 層拒否・`route resolve` 出力 shape 不変の確認 | T2 |
| `crates/cheaper-cli/tests/integration/route_run.rs` | `suppressed_project_provenance_reaches_route_record`(:1386-1456)assert 反転 + substitute e2e(exec job が substitute ペアを実行・`exec-input-mismatch`・exit mirror・deadline/lifecycle anchor・record `executed`・member 併用・`job_id != "exec"` で substitute.grant 到達・convention 拒否) | T3 |
| `crates/cheaper-cli/resources/routes.yaml` | `grants:` 直下へ `substitute` の shape comment 追加(:6-8 — builtin route は substitute 非宣言、記述のみ) | T4 |
| `crates/cheaper-core/src/policy/exec.rs` | `project_grants` doc comment(:118-123)の set 式へ `substitute.grant` 追記のみ(code・predicate 不変) | T4 |
| `docs/guides/{parent-agent-integration,configuration,command,usage}.md` | `substitute` 宣言面・trusted 層限定(project 拒否)・viability retarget・interactive union・`expected_input`/record `executed` の substitute anchor + stale 記述の mode 追随(:303/:296-299/:319-321/:352-354) | T4。design Docs 更新契約の 4 file |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | route field 一覧へ `substitute?`(trusted 層限定)・viability 記述へ実行ペア retarget・`expected_input`/record 記述へ substitute anchor 規約の content update | T4。content update のみ — relation は張らない |
| `crates/cheaper-cli/src/route/routes.rs`・`resolve.rs`・`run.rs` comment 追随 | `Route.grant` doc(:44-46)・`Route.grants` doc(:48-51)・`SuppressedRoute` doc(:77-78)・file header merge 規約(:12-14)・`RouteResolution.suppressed` doc(resolve.rs:101)・`executed` attribution comment(run.rs:475-479) | T4(code 不変・文言のみ) |
| `route_cmd.rs`・`route/supply.rs`・`route/payload.rs`・`workflow_run.rs`(:467-484,:521,:884,:902-945)・`usage_log.rs`・`usage_report.rs`・`tools/exec/mod.rs`・`tools/command/mod.rs`・`hook/targets.rs`・`routed-exec.workflow.json`・`tests/integration/usage_log.rs`・`crates/cheaper-tools/tests/proc_invocation.rs` | **code 不変** — R11 出力 shape・fail-open・D11 gate・convention・record pipeline・shared predicate seam・parity 構造 | T5 で回帰確認(無改訂で既存挙動が維持される churn 境界) |

## Implementation Sequence

- [ ] TASK-034L2N4rLJgtw6UXk7EuK3 (T1): route declaration —
      `RouteSubstitute` struct + `Route.substitute` field +
      `KNOWN_KEYS` + `validate_route` の substitute parse
      (required ペア・nested 未知 key・non-empty 要素)+
      project 層 `substitute` presence → `InvalidRuntimePolicy`
      schema_error + `Route::grant_set()` union +
      `Route::exec_pair()` 新設 + merge の project `trusted:true`
      置換で displaced → `suppressed` 記録 + 全 `Route` literal
      構築追随(routes.rs・resolve.rs・routing_policy.rs)+
      routes.rs unit test(`user_shadows_builtin_and_project_
      needs_trusted` 更新含む)。`cargo test -p cheaper-cli`
      scope で閉じる
- [ ] TASK-034L2N9r4zJdpE3jtoHivQ (T2): viability retarget —
      `resolve.rs` の anchor 完全検査を `exec_pair` 由来の実行
      ペアへ retarget + member ループへ `route.grant` 連結
      (member 級降格)+ stdin 分岐へ substitute union 分類追加
      + resolve.rs unit test(retarget・demote・NoGrant fold・
      union 両側)+ `route.rs` integration(substitute fixture・
      viability case・resolve 出力 shape 不変)。
      `cargo test -p cheaper-cli` で閉じる
- [ ] TASK-034L2NdmhEj1VYd7riHhtL (T3): run 配線 + e2e —
      `run.rs` の `exec_pair` binding・grant_timeout lookup・
      `RouteExecution::new` 呼出し・inputs `grant`/`execArgs`
      の実行ペア化 + run.rs unit test(投影 substitute 含有・
      deadline 選出)+ `route_run.rs` の suppressed assert
      反転 + substitute e2e(exec pair・exit mirror・record
      `executed`・member 併用・convention 拒否)。
      `cargo test -p cheaper-cli` で閉じる
- [ ] TASK-034L2Nh979kTbVrrg1v0zA (T4): docs + comment 追随 —
      `parent-agent-integration.md`・`configuration.md`・
      `command.md`・`usage.md`・`resources/routes.yaml`
      comment・pretooluse spec content update + code comment
      追随(routes.rs :44-46/:48-51/:77-78/:12-14・
      resolve.rs :101・run.rs :475-479・exec.rs :118-123)。
      T3 完了後(comment 対象 file が全て確定してから)
- [ ] TASK-034L2NsdLuC7G3uiTACWd3 (T5): 最終検証 — targeted
      cargo tests・TSPEC Rule ↔ spec AC 対応確認・
      `node scripts/check-doc-ids.mjs`・`audit_docs`・
      `mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 route schema + merge + grant_set/exec_pair | — | T2, T3, T4 |
| T2 viability retarget + interactive union | T1 | T3, T4 |
| T3 run exec_pair 配線 + substitute e2e | T2 | T4, T5 |
| T4 docs + comment 追随 | T3 | T5 |
| T5 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T3 → T4 → T5 の完全直列。T2 は
`RouteSubstitute`/`exec_pair`(T1 成果物)を消費、T3 の e2e は
T2 の viability retarget が無いと substitute route が発火
しない、T4 の comment 追随 site は T1-T3 の全 code file に
またがるため全 code 着地後に置く。

既知の transient compile 断:

- T1 境界 — `Route` struct への `substitute` field 追加は全
  literal 構築(routes.rs:506-516・resolve.rs:395-407・
  routing_policy.rs:2080-2091)の同 commit 追随が必須
  (`route_with_grants()` :411-422 は `..route()` spread の
  ため明示不要)。T1 で cli crate 全体を compile 可能に保つ
  (`cargo test -p cheaper-cli` が通る状態で閉じる)。
- T2/T3 境界 — signature churn は T1 に閉じる(`exec_pair`・
  `grant_set` は T1 で追加済み)。T3 の run.rs 変更は T2 なし
  でも compile するが、substitute e2e は T2 の viability
  retarget 前提のため直列化する。`resolve.rs`・`run.rs` の
  同一 file を T2/T3 で編集しない(file 境界は T2=resolve.rs・
  T3=run.rs で分離済み)。

各 task は 1 commit で、file-local の unit test は同 commit で
追随する。code comment 追随は T4 で一括(design の churn
一覧「comment 追随」項の site を全件)。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1 commit 境界の transient compile 断(`Route.substitute` field — literal 3 箇所、必発) | high(必発) | low | design 確定の churn 一覧どおり literal を同 commit で追随 — T1 は cli crate 全体を検証 scope に |
| 2 | semantic substitution の意味差 — 親 agent は substitute の exit/output を原 command のものとして解釈する(user 打鍵と実 exec の乖離) | high(確定値) | med | trusted 層限定(O3a)が主要根拠 — project 層 presence 拒否を T1 で pin、docs で意味差を明記(T4) |
| 3 | authorization-of-intent の消滅 — 原 argv は `route.grant` pattern に照合されなくなり発火 gate は `match.command` prefix まで後退(作者制御) | high(確定値) | med | 意図的 trade-off(ADR Consequences — 「認可したくない command こそ置き換えたい」用途)— retarget・原 argv 非照合・member 級降格を T2 test で pin |
| 4 | substitute/member 級 `route.grant`/`grants[]` member 失敗が `no-grant` で区別不能(診断性低下 — `route resolve` の `grant` field は anchor を指し substitute 実行対象が見えない) | high(確定値) | low | O2a fold 確定 — `tracing::debug!` へ失敗 member 名を載せる。resolve echo は Deferred(spec Deferred と同一) |
| 5 | interactive substitute が resolve を通過し発火後に grant timeout まで hang して失敗 | low | med | R10 union 分類(宣言 executable 名 ∪ pin 解決名 + `substitute.argv`)を T2 で pin — canonical 名ずらし・shim 名の両穴を塞ぐ |
| 6 | project `trusted:true` route が trusted substitute route を silent に置き換えて追跡不能になる(silent displacement) | med | med | merge で displaced → `suppressed` provenance 記録(T1 — in-scope で塞ぐ)+ consumer 不変の pin + 旧 semantics assert 反転 2 件 |
| 7 | longer-prefix match による substitute 迂回 — project が non-runnable な長い `match.command` を書くだけで trusted substitute を bypass | med | med | known semantics として維持(byte-matched と同じ非対称 — fail-open 方向と整合)。shorter match fall-through は別設計で Deferred — precedence pin を TSPEC に記録 |
| 8 | `grant_set()` への `substitute.grant` 追加漏れで exec job の `resolve_exec_grant` が投影 policy 解決失敗 → substitute route が必ず不発 | low | med | `grant_set()` union を T1 unit test で pin + `run.rs` prepare の substitute 含有 case(T3)|
| 9 | `Route` literal・fixture churn 追随漏れ | med | low | compiler が全 literal site を捕捉 — design Churn 一覧の行番号で網羅、`grep -rn "Route {" crates/` sanity は T5 |

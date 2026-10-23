---
id: "IMPL-034L3VjzecIvExiORZFjPA"
type: "impl"
status: "completed"
title: "substitute-exec route mode"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
  changes:
    added:
      - type: "item"
        path: "crates/cheaper-cli/src/route/routes.rs::RouteSubstitute"
        note: "substitute:{grant,argv} 宣言ペア struct — presence が mode switch(R1/O1a)"
      - type: "item"
        path: "crates/cheaper-cli/src/route/routes.rs::Route::exec_pair"
        note: "実行ペア選択規則の単一 owner — substitute mode は宣言ペア、byte-matched は {route.grant, typed_args}"
      - type: "item"
        path: "crates/cheaper-cli/src/route/run.rs::exec_deadline_ms"
        note: "deadline 導出を実行ペア grant へ一般化した helper(design の inline lookup 記述からの小規模 deviation — deadline 選出 unit test の呼出し対象として抽出)"
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
    - "docs/tasks/add-route-substitute-declaration-and-exec-pair.md"
    - "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
    - "docs/tasks/document-substitute-exec-route-mode.md"
    - "docs/tasks/verify-substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
    - "docs/impl/ir/route-grant-set-projection.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/substitute-exec-route-mode.md"
  references:
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/route-run-delegated-exec.md"
    - "docs/test-specs/route-usage-records.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# substitute-exec route mode

## Summary

issue #167(#151 実装分解 5/7)— route が typed command を実行
せず宣言済みの substitute ペア `{substitute.grant,
substitute.argv}` を実行する **substitute-exec mode** を導入
した(非 exec routing deferred sketch・
ADR-034KtTzOJGqmzmBW3SwGp5 の採用要素 Option D —
ADR-034KXk8h1AtwCkf2cQrb7m O1a-O8a 全採用)。typed command は
`match.command` prefix の発火契機に留まり、routed workflow の
`exec` job は宣言 substitute ペアを実行する。`substitute:
{grant, argv}` nested object 宣言(trusted 層限定・project 層
presence 拒否)・anchor 完全検査の実行ペア retarget・
`route.grant` の member 級降格・投影 set `{route.grant} ∪
grants ∪ {substitute.grant} ∪ {fixed}`・実行 anchor
(`expected_input`・deadline・inputs `grant`/`execArgs`)の
切替・record `executed` の実測 auto-capture( shape 不変 )・
merge の displaced route `suppressed` provenance 記録が
実装された。`Viability` enum・`RouteResolution` shape・
record `schemaVersion:1`・`validate_routed_convention` は
不変。

1. `routes.rs`(T1・`17b7107` + fix `944cfd3`): `RouteSubstitute`
   struct・`Route.substitute` field・`KNOWN_KEYS` への
   `"substitute"` 追加・`validate_route` の project 層
   presence 拒否 + `{grant,argv}` required ペア nested parse・
   `Route::grant_set()` union 拡張・`Route::exec_pair()` 新設・
   merge の project `trusted:true` 置換で displaced 下位層
   route を `suppressed` provenance へ記録。`Route` literal
   構築(routes.rs・resolve.rs・routing_policy.rs)追随 +
   suppressed 旧 semantics pin 2 test の assert 反転。fix —
   unix test の `resolve_command_policy` 誤呼出を
   `resolve_exec_policy` へ修正(#164 由来の unix compile 破損)。
2. viability retarget(T2・`df6dfd8`): 完全検査の対象を
   `route.exec_pair(&out.exec_args)` 由来の実行ペアへ一本化
   (byte-matched 同値・substitute は宣言ペア)。substitute
   mode の `route.grant` を `demoted` 連結で member ループ
   位置へ降格(存在 + eligibility + pin のみ・原 argv 照合
   除去)。stdin-unsupported へ substitute ペアの宣言名 ∪
   pin 解決名 union 分類を追加(typed 側不変)。resolve.rs
   unit + route.rs integration(fixture・project 層拒否・
   R11 shape・displaced provenance)。
3. run 配線(T3・`6eaaf66`): `execute()` の deadline lookup・
   `RouteExecution::new`(`expected_input`)・inputs
   `grant`/`execArgs` を `exec_pair` 選出ペアへ切替。
   `exec_deadline_ms` helper 抽出(実行 grant 宣言 timeout +
   120s — design inline lookup からの小規模 deviation)。
   record `executed`・exit mirror は既存 details capture で
   auto-capture(code 不変)。run.rs unit + route_run.rs e2e。
4. docs(T4・`076b9ce`): routes.yaml shape comment・
   parent-agent-integration/configuration/command/usage guide
   更新・pretooluse spec content update・comment 追随
   (routes.rs ×4・resolve.rs `suppressed` doc・run.rs
   `executed` doc・exec.rs `project_grants` doc — code 不変)。
5. 検証(T5・`48cd27c`): targeted test 全 pass・AC1-AC12 ↔
   TSPEC Rule 対応確認・端到端構造確認・`check-doc-ids` 1168
   files・`audit_docs` error 0・`mise run check` exit 0・
   plan/task status 更新。

## Context

TASK-034L2N4rLJgtw6UXk7EuK3(T1)・TASK-034L2N9r4zJdpE3jtoHivQ
(T2)・TASK-034L2NdmhEj1VYd7riHhtL(T3)・
TASK-034L2Nh979kTbVrrg1v0zA(T4)・TASK-034L2NsdLuC7G3uiTACWd3
(T5)。PLAN-034L2MF3X78MvKvcSN8jmV /
DESIGN-034L0RxGVVTOqtaUUThowk / SPEC-034KXdoK2bMG3gQtIj8l6h /
ADR-034KXk8h1AtwCkf2cQrb7m / TSPEC-034L2MB0m2NOWZsyuTHsjA に
従う。全接点の実測 inventory は DISC-034KWGjJMiSKU5wBxcznho
(resolved)で固定済み。前提となる #166 grant-set 投影
(`project_grants` set 汎用・`{command, executed}` record
split・viability anchor/member 分割)と #165 unified exec
grant model(`is_route_eligible`・`pin_exec_grant`・
`check_exec_execution` の単一 seam)は実装済みであり、
`substitute.grant` は同じ seam・同じ投影・同じ record
pipeline を通る — core/tools 側 code は不変(doc comment のみ
set 式へ `substitute.grant` 追記)。本 IR は
`derives-from`/`implements`/`verified-by`/`depends-on` を
chain 内 doc 集合で閉じ、cross-chain doc(#163/#164/#165/
#166 の IR・unified-exec/grant-set/pretooluse/permission-
model chain の design/spec/ADR/test-spec・guide)は
`related`/`references` でのみ参照する — lineage
edge(refines)は持たない。実装は design 確定の verbatim rust
どおりで、design からの deviation は下記 2 件のみ:

- `exec_deadline_ms` helper 抽出 — design §(d) は deadline
  lookup を inline で `exec_grant` へ差替える記述だったが、
  実装は policy + grant 名を取る helper として抽出し
  deadline_ms を呼ぶ形にした(deadline 選出 unit test が
  呼べる呼出し面を用意するため。挙動は design の契約どおり
  「実行 grant 宣言 timeout + 120s」)。
- `944cfd3` — T1 内で発覚した unix-only compile 破損の修正
  (#164 の `denied_resolved_grant_is_no_grant` test が
  `resolve_command_policy` を誤呼出し — `resolve_exec_policy`
  へ置換。#167 機能とは独立の fix)。

## Implementation

### Changes

- `crates/cheaper-cli/src/route/routes.rs`: `RouteSubstitute`
  struct(:46 — `grant: String` + `argv: Vec<String>` の
  required ペア。presence が mode switch で `Some` =
  substitute mode・`argv:[]` は executable 単独起動の明示
  宣言として受理)。`Route.substitute`(:79 — `grants` 直後。
  firing gate・record attribution anchor は `route.grant` の
  まま)。`grant_set()`(:96)へ `substitute.grant` を union
  (`{route.grant} ∪ grants ∪ {substitute.grant}` — substitute
  mode でも `route.grant` は投影 member として残り firing
  gate/attribution anchor/member job 呼出し可、O7a)。
  `exec_pair()`(:113 — substitute mode は
  `{substitute.grant, substitute.argv}`、byte-matched は
  `{route.grant, typed_args}` を返す実行ペア選択規則の単一
  owner — viability 完全検査・`expected_input`・inputs・
  deadline が全てこの anchor に従う)。`KNOWN_KEYS`(:394)へ
  `"substitute"` を `"grants"` 直後に追加 — unknown-key
  fail-loud 規約内で typo は route-load error。merge の
  `RouteLayer::Project` 置換 branch(:213-217)で `insert`
  戻り値の displaced 下位層 route を `suppressed` へ記録
  (`SuppressedRoute` semantics を「trust 不在で負けた
  project route」から「merge 敗北 route」へ拡張 — shape
  `{name,command}`・consumer・builtin/user 層間 shadow の
  silent は不変。silent displacement を残さない)。project 層
  `substitute` key presence を形状検査より先に
  `schema_error("{label}.substitute: not declarable in the
  project layer")` → `InvalidRuntimePolicy`(:520-526 — O3a。
  `grants` O7a と同じ presence-first 規約・判別子は宣言層
  provenance)。`{grant,argv}` nested object parse(:528-581 —
  非 object・nested 未知 key・`grant` 欠落/非 string/空・
  `argv` 欠落/非 array/要素非 string/空は全て `schema_error`
  — `match`/`target`/`output` と同じ fail-loud 規約)。
  `Route` literal 構築(:634)追随。unit test — parse/
  `argv:[]`/malformed 各 case・project 層拒否 + builtin/user
  受理・`grant_set` union・`exec_pair` 選択・merge displaced
  provenance(:1094-1400)・旧 semantics pin の
  `user_shadows_builtin_and_project_needs_trusted` assert 反転
  (:839)。
- `crates/cheaper-cli/src/route/resolve.rs`: no-grant 段の
  anchor 完全検査を `route.exec_pair(&out.exec_args)` の
  実行ペアへ retarget(:319-355 — lookup 名・`pins` map
  key・`check_exec_execution` の name/args を実行ペアへ。
  `ExecToolOptions` 構築内容・内部照合順序は不変。`pinned`
  は substitute 側 interactive 分類で再利用するため clone)。
  member ループ(:366-391)へ `route.substitute.is_some().
  then_some(&route.grant)` の `demoted` を先頭連結 —
  substitute mode の `route.grant` は member 級(存在 +
  `is_route_eligible` + `pin_exec_grant` のみ、argv 照合
  なし)へ降格し、ループ本体は不変。原 argv への
  `route.grant` pattern 照合は除去(実行されない command の
  形骸化検査 — O2a/R4)。失敗は member 名帰属の
  `tracing::debug!` + `NoGrant` 一種へ fold(新規 viability
  値なし)。stdin-unsupported 段(:395-409)へ substitute ペア
  分類を追加 — `command_name(&grant.executable)`(宣言
  executable 名)∪ `command_name(&pinned)`(pin 解決済み
  実体名)の union + `substitute.argv` で `is_interactive`
  を評価(canonical 名ずらし `python3`→`python3.11`・
  `vim`→`vim.basic`・`vi`→`nvi` と shim 名の両方を塞ぐ —
  O6a/R10。typed 側 `is_interactive(name, &out.exec_args)` は
  不変 — interactive な typed command は substitute 宣言の
  有無に関わらず routing で置き換えない)。test module
  `use` へ `RouteSubstitute` import + `route()` literal へ
  `substitute: None` + substitute case 新設(:839-1270 —
  `substitute_retargets_viability_and_demotes_anchor` で
  retarget/降格/fold/R11 shape・`substitute_interactive_
  union`・`substitute_interactive_union_canonical_names` で
  union 両側・`non_runnable_best_does_not_fall_back_to_
  shorter` で longest-match precedence の known semantics
  pin)。`944cfd3`: unix test の `resolve_command_policy` を
  `resolve_exec_policy` へ修正。
- `crates/cheaper-cli/src/route/run.rs`: `exec_deadline_ms`
  helper 新設(:338 — 投影済み policy + 実行ペア grant 名から
  宣言 `timeout` + 120s の deadline を導出。未存在は
  `denied("no-grant")`)。`execute()` で `exec_pair` binding
  を新設(:377)し、deadline(:378 — `route.grant` 固定から
  実行ペア側へ)・`RouteExecution::new` 呼出し(:392-399 —
  `expected_input = {name: exec_grant, args: exec_argv}` が
  substitute mode で自動的に宣言ペアになる)・inputs
  `grant`/`execArgs`(:405/:412)を実行ペアへ切替。
  `argv`/`args`/`executable` inputs は原 command identity
  を維持(builtin `routed-exec.workflow.json` の `{name:
  $input.grant, args: $input.execArgs}` binding が両 mode で
  無変更動作 — O4a)。D11 byte-match gate・`route_context`
  attach・`finish()`・`build_route_record` は一切不変 —
  `details.{grant,argv}` の実測 echo が substitute ペアを
  auto-capture。test — `prepare_projects_substitute_grant`
  (:793 — 投影内容・undeclared 除外・`exec_pair` 選択)・
  `exec_deadline_uses_selected_exec_grant_timeout`(:845 —
  実行ペア側 timeout 選出・anchor 非採用・未存在 error)。
- `crates/cheaper-cli/src/hook/routing_policy.rs`: `route()`
  literal へ `substitute: None` 追加のみ(評価本体は resolve
  呼出し構造不変 — substitute mode も同じ seam を通り、
  schema error は `load_or_none` fail-open で route arm
  不活性へ fold — 既存規約。precedence pin は既存
  `non_runnable_falls_through` が担保)。
- `crates/cheaper-core/src/policy/exec.rs`: `project_grants`
  doc comment(:118-123)の set 式へ `{substitute.grant}` 追記
  のみ — code・predicate 不変(`is_route_eligible` 単一 seam
  が `route.grant`・`grants[]`・`substitute.grant` の 3 参照を
  同じ `scope=="trusted" && !fixed` で判定)。
- `crates/cheaper-cli/resources/routes.yaml`: `grants:` 直下へ
  `substitute` shape comment(optional・builtin/user 層限定)。
- `crates/cheaper-cli/tests/integration/route.rs`: substitute
  fixture + `resolve_substitute_retargets_viability`・
  `resolve_project_substitute_rejected`・`resolve_project_
  trusted_displacement_reports_suppressed`(displaced provenance
  の resolve 出力 pin)。
- `crates/cheaper-cli/tests/integration/route_run.rs`:
  `substitute_fx` fixture + e2e 5 case(`substitute_exec_pair_
  runs_and_mirrors_exit` — 実測ペア実行 + exit mirror・
  `substitute_exec_job_input_mismatch` — `route.grant`/原
  exec_args/member 名の 3 面で `exec-input-mismatch`・
  `substitute_grant_env_reaches_child_not_anchor` — lifecycle
  anchor が substitute.grant 宣言値・`substitute_member_jobs_
  reach_members_and_substitute` — member 併用 + `input.name`
  = `substitute.grant` の member job 到達・`substitute_routed_
  convention_still_enforced` — Option E 境界)。record
  `executed:{substitute.grant, substitute.argv}`・
  `suppressed_project_provenance_reaches_route_record` の
  assert 反転(T1 semantics 拡張へ追随)。
- `docs/guides/parent-agent-integration.md`: routes.yaml
  宣言例 + `substitute` field 説明(`{substitute.grant,
  substitute.argv}` が typed command の代わりに実行・
  `argv` executable 不含・`argv:[]` 合法・trusted 層限定・
  `grant:` は substitute 有無に関わらず required)・
  viability retarget・typed/substitute 両側 interactive 分類・
  `expected_input` が substitute ペアを要求・record
  `executed` が substitute ペアを記録・exec job binding
  不変の authoring 規約・有効 set 式・束縛対象・exit
  mirror の mode 追随。`configuration.md`: route grant 参照
  eligibility 記述へ `substitute.grant` 追加 + 宣言層限定
  併記。`command.md`: 同 eligibility 規則へ更新。`usage.md`:
  `executed` が substitute mode で宣言 substitute ペアを
  記録する旨 1 文追記。`pretooluse-route-resolver-and-
  workflow-delegation.md` spec: route field 一覧・viability・
  `expected_input`/record 記述へ substitute 記述の content
  update(relation は張らず spec chain 側管理)。

### Decisions During Implementation

- **O1a — `substitute: {grant, argv}` nested object** —
  `match`/`target`/`output` と同型の compound-value-as-object
  規約どおり nested object で実装。`grant`・`argv` required
  ペア・nested 未知 key 拒否・`argv` は executable を含まない
  args 列で `[]` 合法・`grant`(単数)は substitute の有無に
  関わらず required(firing gate・attribution anchor・投影
  member)。`substitute.grant` の検証は non-empty string のみ
  — `grant` 単数と同じく存在/eligibility は viability へ
  委譲(第二の検証規則を作らない)。`substitute.grant ==
  route.grant`・`substitute.grant ∈ grants[]` はともに合法
  (`grant_set()` が dedupe)。
- **O2a — anchor 完全検査を substitute ペアへ retarget +
  `route.grant` member 級降格** — 完全検査対象は
  `exec_pair` が選ぶ実行ペアへ一本化(byte-matched 同値)。
  substitute argv は route 解決時点で確定済みの宣言値で
  あり member の「job 実行時へ遅延」と質的に異なるため
  「宣言通り実行する command」への完全検査が意味を持つ
  (precheck strict 維持)。`route.grant` 降格は `demoted`
  連結で member ループ位置を共有し、原 argv の `route.grant`
  照合は行わない(実行されない command の形骸化検査の除去
  —「認可したくない command こそ置き換えたい」用途を壊さ
  ない)。失敗は substitute 側・member 級 `route.grant`・
  `grants[]` member とも `NoGrant` 一種へ fold。
- **O3a — `substitute` は trusted 層限定** — project 層の
  `substitute` key presence を形状検査より先に
  `schema_error`/`InvalidRuntimePolicy` へ倒す(`grants` O7a
  と同じ判別子・同じ presence-first 規約 — untrusted 層の
  宣言は内容を評価しない)。byte-match が typed command へ
  課していた拘束は substitute に乗り移らないため、project
  層の「実実行 `{grant,argv}` 選択権」委譲は O4c/O7a が
  塞いだ confused-deputy hole のさらに強い再発として load
  時に拒否(§(f) — `trusted:true` は project 自己申告の
  shadowing gate で判別に使えず、判別子は宣言層
  provenance)。anchor `grant:` 単数・byte-matched route は
  project 層で従来どおり受理。
- **O4a — `expected_input = {name: substitute.grant, args:
  substitute.argv}`** — `RouteExecution::new` 呼出しに実行
  ペアを渡すだけで `:176` の `json!({"name": grant, "args":
  exec_args})` 構築式・signature・D11 byte-match gate・
  `reason:"exec-input-mismatch"`/`PolicyViolation` 分類は
  一切不変。inputs `grant`/`execArgs` は実行ペア、
  `argv`/`args`/`executable` は原 command identity —
  builtin `routed-exec.workflow.json` が binding 無変更で
  両 mode に動く契約。deadline は実行 grant 宣言 `timeout`
  + 120s(R8)。
- **O5a — record shape 不変** — `route`(宣言 identity・
  `route.grant` anchor)+ `commandHash`(原 command)+
  `executed:{grant,argv}`(実測 substitute ペア)の 3 field
  で `schemaVersion:1`・新規 field なし。`command::execute`
  の `ProcIdentity.details` 実測 echo → `finish()` 抽出 →
  `build_route_record` 挿入の既存 pipeline が substitute
  ペアを運ぶために code 変更は一切不要だった(auto-capture
  の no-change 検証 — design §(e) どおり)。
- **O6a — stdin-unsupported は両側適用** — typed 側を維持
  しつつ substitute ペア(宣言 executable 名 `command_name(
  &grant.executable)` と pin 解決済み実体名
  `command_name(&pinned)` の union + 宣言 argv)へ同じ分類を
  適用。片側だけだと canonical 名ずらし・shim 名で系統的に
  回避される。`(b-1)` で解決済みの `grant`/`pinned` を再利用
  するため追加コストなし。`grants[]` member は screening
  対象外のまま(known semantics)。
- **O7a — 投影 set = `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}`** — `grant_set()` に
  `substitute.grant` を union するのみで `project_grants`
  呼出し(`run.rs:121`)は文面不変。`route.grant` を投影から
  外す O7b は不採用(firing gate・attribution anchor・member
  job 呼出し・set 検査規約の複雑化を避ける — 半径は argv
  pattern bound に閉じる)。
- **O8a — Option E(exec-less route)は open 事項として残置**
  — `validate_routed_convention` 不変: exec job を持たない・
  `outputs.execResult` whole binding を持たない workflow は
  substitute 宣言の有無に関わらず routed convention 違反で
  拒否(`substitute_routed_convention_still_enforced` で
  pin)。author 宣言 exit を信用する成功偽装 hole が未解決
  のため採否は別 issue。
- **merge displaced provenance(design §(a) 追加入力)** —
  project `trusted:true` による下位層置換で displaced route
  を `suppressed` へ記録する merge 拡張を T1 に同梱(trusted
  substitute route が project 層の同名宣言で無痕跡に
  置き換わると `route resolve`/record の双方で追跡不能に
  なる穴を provenance 側で塞ぐ — in-scope)。`SuppressedRoute`
  semantics を「merge 敗北 route」へ拡張しつつ shape・
  consumer(resolve 出力 `suppressed`・record `suppressed`)・
  builtin/user 層間 shadow の silent は不変。旧 semantics
  pin の 2 test assert を新 semantics へ反転(設計どおりの
  回帰例外 — AC12)。
- **2 系統 anchor の境界固定(code 不変の契約)** — 実行
  anchor(substitute mode では `substitute.grant`:
  `expected_input`・deadline・grant 宣言由来の
  `timeout`/`killGrace`/`exitCodes`/`maxOutputBytes`・
  `env`)と route 帰属 anchor(`route` 宣言名・`route.grant`
  firing gate・`commandHash` 原 command)の分離を design §(e)
  どおり実装境界として維持。member job(`job_id != "exec"`)
  は expected_input gate 非適用・`route_context: None` で
  member grant 宣言 lifecycle で動き、`input.name =
  substitute.grant` の member job 到達は投影内名として
  member 規約で合法( e2e で pin)。

## Related Experiments

### Adopted

なし(known-solution task。design が schema/parse・viability
retarget・member 降格・実行 anchor 配線・merge provenance・
error 契約・churn 一覧を verbatim rust + file:line 精度で
固定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

targeted cargo test・doc check(2026-10-24 実測、全 pass / 0
failed — T5 task 記録):

- `mise run check`: exit 0 — biome → `cargo fmt --check` →
  clippy `--workspace --all-targets`(dev-seam)→ version →
  doc-ids(1168 files)→ `cargo test --workspace`(dev-seam)
  50 binary 全件 0 failed → `node --test`(scripts 43 /
  benchmarks 12 pass)→ lockfile → apm-payload、全 stage pass
- `node scripts/check-doc-ids.mjs`: exit 0 — "doc id check
  OK: 1168 markdown files scanned"
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all --json`: exit 0 — 本 issue の新規 doc 7 件
  (plan・test-spec・task×5)で severity "error" 0 件
  (inconsistent-reciprocal-relation warning は repo 全域の
  既存 pattern で blocking:false)
- spec Acceptance Criteria AC1-AC12 全項目に対応する test が
  存在し pass(TSPEC Rule ↔ spec AC 対応表は T5 task §1 で
  確認済み): `routes.rs` unit 16(`substitute_parse_and_
  empty_argv`・`substitute_malformed_is_schema_error`・
  `project_layer_substitute_is_rejected`・`grant_set`
  union・`exec_pair` 選択・merge displaced provenance)・
  `resolve.rs` unit 7(`substitute_retargets_viability_and_
  demotes_anchor`・`substitute_interactive_union`・
  `substitute_interactive_union_canonical_names`・
  `non_runnable_best_does_not_fall_back_to_shorter`)・
  `run.rs` unit 21(`prepare_projects_substitute_grant`・
  `exec_deadline_uses_selected_exec_grant_timeout`)・
  `route::` integration 10(`resolve_substitute_retargets_
  viability`・`resolve_project_substitute_rejected`・
  `resolve_project_trusted_displacement_reports_suppressed`)・
  `route_run` integration 37(`substitute_exec_pair_runs_
  and_mirrors_exit`・`substitute_exec_job_input_mismatch`・
  `substitute_grant_env_reaches_child_not_anchor`・
  `substitute_member_jobs_reach_members_and_substitute`・
  `substitute_routed_convention_still_enforced` + record
  `executed` case)
- grep sanity(T5 実測): `grep -rn "Route {" crates/` の
  literal site 全件 `substitute` field あり(`..route(..)`
  spread 経由含む)・`TODO`/`unimplemented!`/`todo!` 0 件・
  `RouteResolution` に substitute field なし・
  `check_exec_execution` が `exec_pair` 由来ペアのみを対象
  (resolve.rs:319/354 — `route.grant` の原 argv 照合残件
  なし)・`route_cmd.rs` が merge-base から無差分(R11 出力
  shape 不変)
- 端到端構造確認(T5 実測): member ループが `demoted.
  chain(&route.grants)` で存在+eligibility+pin のみ
  (:366-391)・union が `command_name(&grant.executable)` +
  `command_name(&pinned)`(:405-406)・`project_grants(
  &route.grant_set())` が `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}` を残し undeclared 名が
  `resolve_exec_grant` で `PolicyViolation`・
  `expected_input`/deadline/inputs が exec_pair 由来
  (:377-412)・merge は builtin/user 層間 silent のまま
  displaced を `suppressed` へ記録(routes.rs:192-218)・
  `load_or_none` が substitute schema error を fail-open で
  route arm 不活性へ fold

証跡 commit: planning chain `50a105f`(seed discovery)→
`d1a1a8f`(spec/ADR + discovery resolve)→ `d171d56`(design)→
`2e50f66`(design review 指摘反映)→ `a7c0caf`(design
approved 昇格)→ `d519678`(plan + test-spec + tasks)→
実装 `17b7107`(T1 substitute schema + 層拒否 + merge
suppressed + grant_set/exec_pair)→ `944cfd3`(T1 fix —
unix test resolve_exec_policy)→ `df6dfd8`(T2 viability
retarget + route.grant 降格 + interactive union)→
`6eaaf66`(T3 run exec_pair 配線 + substitute e2e)→
`076b9ce`(T4 docs + comment 追随)→ `48cd27c`(T5 検証
記録 + plan/task status 更新)。本 IR はその後に追加する。

## Risks

- **authorization-of-intent の消滅** — substitute mode で原
  argv は `route.grant` の argv pattern に照合されなくなり、
  「typed command が認可済みのときのみ発火」の gate は
  `match.command` prefix まで後退する(作者制御)。「認可
  したくない command こそ置き換えたい」用途を可能にする
  意図的な trade-off であり、`substitute` を trusted 層限定
  にした理由そのもの(design §Risks・ADR Consequences)。
- **semantic substitution の意味差** — 親 agent は
  substitute の exit/output を原 command のものとして解釈
  する。user が打鍵した command と実際に走った command が
  異なることを trusted authoring opt-in で受容する(routing
  層が新たに開く意味差 — spec R15 再記述済み)。
- **NoGrant fold の診断性** — substitute 失敗・member 級
  `route.grant` 失敗・`grants[]` member 失敗が `no-grant`
  で区別不能(#166 と同じ構造)。`route resolve` 出力の
  `grant` field は anchor(`route.grant`)を指し substitute
  実行対象が見えない — 発火診断需要が確立した時点で
  resolve echo を再検討(Deferred)。
- **longer-prefix match による substitute 迂回(known
  semantics)** — 最長 `match.command` の best 1 件のみが
  viability 評価され、non-runnable best から shorter
  match への fall-through はない。project 層が longer
  `match.command` + ineligible `grant:` の通常 route を
  書くだけで trusted substitute を「最も効いてほしい場面」
  で silent に bypass できる(substitute 固有の hole では
  なく longest-match 規約の帰結 — byte-matched でも同じ非
  対称は既存。substitute は safety 置換を担うため悪化度が
  大きい)。fail-open 既定挙動と同方向のため precedence は
  既存 semantics として維持し、pin test で固定
  (`non_runnable_best_does_not_fall_back_to_shorter`)—
  fall-through は別設計として Deferred。provenance 側の穴
  (`trusted:true` 置換)は merge 拡張で in-scope に塞いだ。
- **interactive 不発面の増加** — interactive executable を
  pin する `substitute.grant`・interactive flag を含む
  `substitute.argv`・interactive な typed command はいずれ
  も `stdin-unsupported` で不発に倒れる(意図的な保守方向
  —「発火 → hang → timeout 失敗」より resolve 時点分類が
  良い)。
- **record の anchor 時点性** — `kind:"route"` record だけ
  では firing-gate 認可 grant が `route` 名経由の宣言 join
  に依存する。routes.yaml 編集で `route.grant` が改名される
  と時点性の anchor が遡れない(現行 record 規約と同じ制約
  — O5b 不採用、監査需要確立時に additive field で拡張)。
- **`route.grant` は投影に残る** — 実行しない grant を投影
  するため半径は僅かに広いが、member job 非対称・set 検査
  規約複雑化・record attribution 矛盾を避けるため O7b
  不採用(substitute argv は宣言固定で半径は argv pattern
  bound に閉じる — design 受理済み)。
- **member timeout の silent clamp(known semantics)** —
  workflow deadline は実行 grant(substitute mode では
  `substitute.grant`)の `timeout` のみ由来。member が長い
  `timeout` を宣言しても黙って clamp される — max-of-set
  化は Deferred(#166 と同じ)。
- **interactive member は筛かれない(known semantics)** —
  `grants[]` member job は非検査のまま(自身 grant bound で
  動く別経路 — #166 と同じ operator 責務)。
- **env-prefix/passthrough env は substitute process へ届く
  (quirk)** — `extra_env` は「実際に spawn される側」の
  `grant.env` へ override merge されるため、typed command
  宛の env は substitute process が受け取る。byte-matched
  と同一機構・同一規則(「実行される process へ届く」)で
  substitute 固有の新規面ではないが意味差として明記済み
  (design §Risks)。
- **substitute pin の 2 回評価 gap(#166 同型)** — 実行ペア
  pin は viability と scope 構築の 2 回解く現行構造不変。
  viability 通過後の PATH 変動/policy 更新で routed run
  全体が `PolicyViolation` hard error し得る —
  `verify_pinned_executable` の spawn 前検出と同じ race
  domain で pin cache は新設しない(#166 で受理済みの
  error 契約と同型)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #167)由来の未分類・未処理の follow-up はない。
下記は全て spec/design「Deferred Design Concerns」・各 task
「Out of Scope」で scope 外確定・記録済みの defer、別 issue
へ移管済みの項目、または design で受理・記録済みの既知
semantics であり、新規 chain は起こさない。

- **documented-deferred — Option E(exec-less route)の採否** —
  親 chain の open 事項(ADR-034KtTzOJGqmzmBW3SwGp5)。author
  宣言 exit(`outputs.exitCode` mirror)を信用する成功偽装
  hole の未解決・lint-only/analysis route の product 需要
  未確認のため本 design では実装せず、`grant` field は
  firing gate として留保する設計のみ確定(R11/O8a —
  `validate_routed_convention` 不変、e2e pin 済み)。採否は
  需要確認と exit-code contract の信頼モデル設計が揃った
  時点の別 issue で判断。
- **documented-deferred — `route resolve` 出力への
  substitute echo** — R11 出力 shape(:105-128)を維持した
  ため substitute mode の発火診断は `route`/`grant` field
  (anchor echo)だけでは「何が実行されるか」が読めない。
  発火診断需要が確立した時点で `substitute`/`execPair`
  echo の additive 追加を検討(spec/design Deferred と同一)。
- **documented-deferred — usage record への firing-gate
  anchor grant 明示 field** — record は実測 + route
  identity の規約を維持(O5a/O5b 不採用)。routes.yaml 編集
  で `route.grant` が改名された場合に時点性の anchor が
  `route` 名経由の宣言 join でしか遡れない。監査需要が
  確立したら additive field として別途追加(spec Deferred
  と同一)。
- **documented-deferred — substitute argv への typed args
  転送・補間** — `substitute.argv` は宣言時点の固定値
  (宣言凍結 R12)。「prefix は substitute 化しつつ末尾
  args は typed のまま」の需要が確立した時点で、
  byte-match 対象を宣言形に再設計する別 issue で検討。
- **documented-deferred — non-runnable best match から
  shorter match への fall-through** — longest `match.
  command` の best 1 件のみを viability 評価する現行
  precedence は全 mode 共通の match 規約として維持(known
  semantics — §Risks の longer-prefix hole-punch 項目)。
  「候補列走査 + per-candidate viability」への変更は全
  mode の precedence を変える別設計であり、route 解決
  semantics 全般の別 issue で検討(design Deferred と同一)。
- **documented-deferred — `substitute` の project 層開放
  (O3b/O3c)再評価** — byte-match 保護が乗り移らない
  confused-deputy hole として不採用。project route が
  substitute を必要とする実需要が確立した場合、typed argv
  への何らかの宣言拘束(authorization-of-intent 相当)を
  セットに再評価する(design Deferred と同一)。
- **documented-deferred — `grants[]` member の interactive
  screening** — member job は非検査のまま(known
  semantics — 自身 grant bound で動く別経路)。member の
  interactive 失敗が運用問題化した場合に member 側
  screening を別途検討。
- **documented-deferred — workflow deadline の max-of-set
  化** — deadline は実行 grant(substitute mode では
  `substitute.grant`)の `timeout` のみ由来(silent clamp
  は Risks 記録済み)。member の timeout 期待が発火側と
  乖離する運用が出た場合に `deadline = max(exec,
  members)` への変更を検討(#166 Deferred と同一 —
  substitute で構造は変わらない)。
- **documented-deferred — viability 値の substitute 分解・
  record schemaVersion bump** — substitute 失敗は `NoGrant`
  一種 fold(新規 viability 値なし — O2a/O3a 確定)・record
  は `schemaVersion:1` additive 規約維持(O5a)。発火診断・
  監査需要が確立した時点で resolve echo/record field と
  セットで再評価。
- **moved-to-issue — trusted flag 発行主体** — issue #168
  (#151 分解 6/7)の scope —
  https://github.com/scarecrow173/cheaper/issues/168。
- **moved-to-issue — docs 全体の正直性 sweep + migration
  guide** — issue #169 の scope —
  https://github.com/scarecrow173/cheaper/issues/169。
- **closed — `trusted:true` 置換の silent displacement** —
  merge の project `trusted:true` 置換で displaced 下位層
  route を `suppressed` provenance へ記録する拡張を本
  issue の in-scope として実装済み(T1・design §(a)。
  builtin/user 層間 shadow は同一 trusted principal の宣言
  override として silent 維持の設計確定)。
- **closed — substitute pin の scope 構築時再失敗(新規失敗
  domain)** — viability 通過後の PATH 変動/policy 更新で
  routed run 全体が `PolicyViolation` hard error し得る点は
  #166 と同じ race domain で、pin cache 新設なしの KISS
  確定を維持(#166 で受理・記録済みの error 契約と同型 —
  design で変更不要と確定)。

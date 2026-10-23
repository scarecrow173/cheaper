---
id: "IMPL-034KWrmkyelp896hjwVsq9"
type: "impl"
status: "completed"
title: "route grant-set projection"
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
    renamed:
      - type: "item"
        path: "crates/cheaper-core/src/policy/exec.rs::project_grant -> crates/cheaper-core/src/policy/exec.rs::project_grants"
        note: "単一名版を set 版(&BTreeSet<String>)へ一般化 — 呼出し側は route/run.rs のみのため第 2 API を残さず置換"
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-grants-declaration-and-grant-set.md"
    - "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
    - "docs/tasks/split-route-record-executed-attribution.md"
    - "docs/tasks/document-route-grant-set-projection.md"
    - "docs/tasks/verify-route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/route-grant-set-projection-issue-166.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-grant-set-projection.md"
  references:
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
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

# route grant-set projection

## Summary

issue #166(#151 実装分解 4/7)— byte-matched route の exec 委譲面を
単一 `grant` から宣言 set `{route.grant} ∪ grants` へ拡張し、
viability を「anchor 完全検査 + member 軽量検査」へ分割し、
`kind:"route"` usage record に実測 `executed:{grant,argv}` を
additive 追加した(ADR-034KTMnPFDF25x710LFSKY — O1a-O7a 全採用:
`grant` required + optional `grants`・anchor 完全検査 + member
軽量検査・`is_route_eligible` 単一 seam・`NoGrant` 一種へ
fold・`argv_shell_re` 除去・additive `executed`・
`expected_input.name = route.grant` anchor 維持・`grants` 宣言は
builtin/user 層限定)。byte-matched 発火条件・`expected_input`
byte match・record `command`/`commandHash`/`schemaVersion:1` は
不変。

1. `routes.rs`(T1・`434f4ca`): `Route.grants: Vec<String>` field・
   `KNOWN_KEYS` への `"grants"` 追加・`validate_route` の grants
   parse(member dedupe・self-ref 除去・形状 error)+ project 層
   `grants` key presence を member 形状検査より先に
   `schema_error`(`InvalidRuntimePolicy`)で拒否する層制約(O7a)+
   `Route::grant_set()` helper。`Route` literal 構築(routes.rs・
   resolve.rs・routing_policy.rs)追随。
2. viability/projection(T2・`471865b`): `resolve.rs` の
   unsafe-command 分岐を caller flag + `CWD_CHANGING_COMMANDS`
   executable 名のみへ縮小(`argv_shell_re` 除去・stale comment
   訂正)+ no-grant 段へ member 検査ループ(存在 +
   `is_route_eligible` + `pin_exec_grant` のみ → `NoGrant` fold)。
   `exec.rs` の `project_grant(&str)` →
   `project_grants(&BTreeSet<String>)` 置換(`{names} ∪ {fixed}`
   retain)+ `argv_shell_re` 定義削除。`run.rs:121` 呼出し追随 +
   route/route_run/proc_invocation test。
3. record split(T3・`f5e8d54`): `RouteRecordInput` へ
   `executed_grant`/`executed_argv`・`build_route_record` の
   `commandHash` 直後へ `executed:{grant,argv}` 必須挿入・
   `run.rs` finish() の captured `details.{grant,argv}` 抽出
   (欠落・型違反は `InternalError`)。
4. docs(T4・`0419697`): `resources/routes.yaml` shape comment・
   `parent-agent-integration`/`configuration`/`command`/`usage`
   guide 更新・pretooluse spec の content update(R1 `grants?`・
   R12 member 検査・R34 `executed`)。
5. 検証(T5・`25529ff`): targeted test 全 pass・grep sanity
   (`argv_shell_re` 0・`project_grant` 残件なし・`Route` literal
   全件 `grants` field)・`check-doc-ids` 1156 files・`audit_docs`
   error 0・`mise run check` exit 0・plan/task status 更新。

## Context

TASK-034KVxrNr5J7VrsdNaZDAP(T1)・TASK-034KVxxl3lkopkjHoRid7V(T2)・
TASK-034KVxxS09XbljJlhLD9Ev(T3)・TASK-034KVxDgWqxNof9qYv0dbf(T4)・
TASK-034KVxDMCEUdI5WtH1SJxd(T5)。PLAN-034KVx51kVYoMwO8UmJ8VE /
DESIGN-034KUgF5QB3WuzFG6PmVvr / SPEC-034KTMfXYD5ZlGZCRb8Paj /
ADR-034KTMnPFDF25x710LFSKY / TSPEC-034KVxaBMzQE5YtVOx4j3w に従う。
全接点の実測 inventory は DISC-034KTzzHPIbmtD8j3iHBtg(resolved)
で固定済み。前提となる unified exec grant model(#165 —
`ExecGrant`・`is_route_eligible`・`project_grant`・
`check_exec_execution`・`pin_exec_grant` の単一 seam)は実装済み
であり、本 issue はその seam を set 投影へ一般化する。本 IR は
`derives-from`/`implements`/`verified-by`/`depends-on` を chain 内
doc 集合で閉じ、cross-chain doc(#163/#164/#165 の IR・
unified-exec chain の design/spec/ADR・#151 chain の design/spec・
pretooluse chain・canonical tool spec 群)は `related`/`references`
でのみ参照する — lineage edge(refines)は持たない。実装は design
確定の verbatim rust どおりで、design/spec からの deviation は
ない。

## Implementation

### Changes

- `crates/cheaper-cli/src/route/routes.rs`: `Route.grants:
  Vec<String>`(:52 — `grant` 直後。doc comment で anchor 単数と
  member set の役割分離・有効 set は `grant_set()` 規則を明記)。
  `Route::grant_set()`(:69 — `{route.grant} ∪ grants` を
  `BTreeSet` で供給する有効 set 規則の単一 owner — 同一規則を
  viability・projection・doc で二度書かない)。`KNOWN_KEYS`
  (:334)へ `"grants"` を `"grant"` 直後に追加 — unknown-key
  fail-loud 規約内で `grants` typo は route-load error。
  `validate_route`(:309 の既存 `layer` param を判別子に利用 —
  新規 param 不要): project 層 entry の `grants` key presence を
  member 形状検査より先に `schema_error("{label}.grants: not
  declarable in the project layer")` → `InvalidRuntimePolicy`
  (:426-430 — O7a。`trusted:true` は project 自己申告の
  shadowing gate で判別に使えず、判別子は宣言層 provenance)。
  grants parse(:435-452 — array + non-empty string member、
  member 間重複と `route.grant` self-ref は冪等除去、`grants: []`
  ≡ 未指定)。`Route` literal 構築(:511)追随。unit test —
  parse/dedupe/`[]`/malformed(非 array・非 string・空 string)・
  project 層 `grants` presence 拒否・builtin/user 受理・project
  `grant` 単数受理の pin。
- `crates/cheaper-cli/src/route/resolve.rs`: import(:10)から
  `argv_shell_re` を落とす。unsafe-command 分岐(:282-288)を
  `input.unsafe_command` + `CWD_CHANGING_COMMANDS` 名の 2 条件
  のみへ縮小 — stale comment(#163 で grant argv pattern が
  metachar を拒否しなくなった後の残滓)を「tokenize 済み argv の
  内容審査は routing 層の責務ではない(D2)」の正確な根拠へ
  訂正。no-grant 段の anchor 完全検査(:315-348 — 存在 +
  `is_route_eligible` → `pin_exec_grant` → `ExecToolOptions` →
  `check_exec_execution` argv 照合)は不変のまま、直後に member
  ループ挿入(:349-378 — `route.grants` 各 member に存在 +
  `is_route_eligible` + `pin_exec_grant` のみ適用。argv 照合は
  job 実行時へ遅延、pin 解決結果は検査のみで破棄。失敗は
  `NoGrant` へ fold + `tracing::debug!` で member 名帰属)。
  `Route` literal(:409-)へ `grants` 追加。`:551` の `a;rm`
  quoted metachar 期待値を `Runnable` へ反転 +
  `member_grants_fold_into_no_grant`(:618)で member 未存在・
  `scope:"project"`・fixed・pin 不能・member 失敗が
  stdin-unsupported より先に fold される順序を pin。
- `crates/cheaper-cli/src/route/run.rs`: `project_grants(
  &route.grant_set())`(:121 — `{route.grant} ∪ grants ∪ {fixed}`
  投影・false は `denied("no-grant")` `PolicyViolation` —
  viability 通過後の二重防御)。finish()(:465 以降)で captured
  `details.{grant,argv}` を `detail_i64` accessor と同じ style で
  抽出(:475-503 — `expected_input` gate が `job_id=="exec" &&
  tool=="command"` に `details.{grant,argv}` を保証済みのため、
  欠落・非 string・非 array・非 string member は
  `"captured exec result has no executed grant"`/
  `"captured exec result has no executed argv"` の `InternalError`
  — `exitCode` 欠落(:467-474)と同一 invariant で record に
  silent absence を残さない)。`RouteRecordInput` literal
  (:608-609)へ `executed_grant`/`executed_argv` 供給 — record
  書込み位置・best-effort 規約は不変。test —
  `prepare_projects_grant_set`(:734 — set 投影内容・fixed
  retain)・`finish_record_carries_executed_attribution`(:1131 —
  `executed` echo)・`finish_missing_executed_details_is_
  internal_error`(:1165 — 欠落・型違反各 case)。
- `crates/cheaper-cli/src/usage_log.rs`: `RouteRecordInput`
  (:332)へ `executed_grant: String`(:341)・`executed_argv:
  Vec<String>`(:344) — record 形状の owner は `build_route_record`
  に残し caller は typed 値のみを渡す(`{grant,argv}` object 組立
  を run.rs へ漏らさない)。`build_route_record`(:369)が
  `commandHash` 直後へ `executed:{grant,argv}` を必須挿入
  (:388-398 — `schemaVersion:1` 不変の additive field。
  `executed.argv` は usage.jsonl 初の plaintext arg 値 —
  comment で露出を明記)。unit test で shape・`commandHash` 不変・
  `schemaVersion` を pin。
- `crates/cheaper-core/src/policy/exec.rs`:
  `project_grant(&mut self, name)` →
  `project_grants(&mut self, names: &BTreeSet<String>)`(:124 —
  全名が存在かつ `is_route_eligible` かつ `names` 非空でのみ
  `{names} ∪ {fixed grants}` を retain、それ以外は false で
  policy 不変 — viability との二重防御として投影関数自身の
  不変条件を維持。呼出し側は `run.rs:121` のみのため第 2 API を
  残さず置換)。`argv_shell_re` 定義 + comment 削除(crates 内
  唯一の consumer が外れたため定義ごと削除 — spec AC grep 0)。
  test `route_eligibility_and_project_grants`(:1036)を set 呼出し
  + multi-member・空集合・ineligible 混入・anchor 欠落名 case へ
  更新。
- `crates/cheaper-cli/src/hook/routing_policy.rs`: `Route`
  literal へ `grants: Vec::new()` 追加のみ(評価本体は resolve
  呼出し構造不変 — hook eval は T2 の viability 変更を自動適用。
  `load_or_none` fail-open で grants schema error は route arm
  不活性へ fold — 既存規約不変)。
- `crates/cheaper-tools/tests/proc_invocation.rs`:
  `grant_set_projection_preserves_scope_parity`(:708)— set 投影の
  parity 回帰 + multi-grant 投影 case(投影外 `input.name` 拒否は
  既存構造の回帰)。
- `crates/cheaper-cli/tests/integration/route.rs`: `grants`
  宣言 fixture + `resolve_member_grants_viability`(:137)—
  member 未存在/fixed → `no-grant`・quoted metachar → `runnable`
  case。
- `crates/cheaper-cli/tests/integration/route_run.rs`: member
  job が declared set 内を `input.name` で呼べる・undeclared 名が
  `PolicyViolation`・`exec_job_named_member_is_input_mismatch`
  (:594 — `job_id=="exec"` で member 名を指定すると
  `exec-input-mismatch` — authoring 規約の回帰)・record
  `executed` case。
- `crates/cheaper-cli/tests/integration/usage_log.rs`:
  `route_record_shape`(:299)に `executed:{grant,argv}` shape +
  `commandHash`/`schemaVersion` 不変の assertion。
- `crates/cheaper-cli/resources/routes.yaml`: `grant:` 直下へ
  `grants` shape comment(optional・builtin/user 層限定 — builtin
  route は member 非宣言)。
- `docs/guides/parent-agent-integration.md`: routes.yaml 宣言例 +
  `grants` field 説明(有効 set・dedupe・`trusted && !fixed`
  eligibility・member argv は自身の grant pattern で bound・
  member job は宣言 lifecycle/cwd/env/exitCodes・route context
  非適用)+ 層制約(project routes.yaml の `grants` は
  route-load error・anchor `grant:` 単数は project でも可)+
  member job authoring 規約(`job_id != "exec"` で宣言)+ record
  節へ `executed` additive field + `executed.argv` 平文 arg 露出
  の明記。`configuration.md`: route `grant:` 参照の trusted+
  `!fixed` 記述を `grants:` member も同 predicate へ拡張 +
  builtin/user 層限定の併記。`command.md`: route grant 参照記述を
  member 同 eligibility 規則へ更新。`usage.md`: additive-only
  規約節へ `kind:"route"` record の `executed` を additive field
  の例として追記 + 平文 arg 値の 1 文明記。`pretooluse-route-
  resolver-and-workflow-delegation.md` spec: R1 field 一覧へ
  `grants?`(trusted 層限定)・R12 viability 記述へ member 検査・
  R34 record field へ `executed` の content update(relation は
  張らず spec chain 側の管理へ委譲)。

### Decisions During Implementation

- **O1a — `grant` required + optional `grants`** — anchor 必須を
  型で表さないため「有効 set の構築責務は `Route::grant_set()`
  に置く」を契約として固定(anchor を含まない集合を
  `project_grants` へ渡すと anchor が投影から落ちて exec job が
  fail する)。viability の member 検査は `route.grants`
  (member のみ)を走査し、投影には `grant_set()` の全 set を
  渡す使い分け。
- **O7a — presence-before-shape の fail-closed 順序** — project
  層の `grants` key は member 形状検査より先に presence で拒否
  し、untrusted 層の malformed 内容を評価しない(plan risk #8
  の pin どおり)。判別子は `trusted: true` flag ではなく宣言層
  provenance(`layer == RouteLayer::Project` — `trusted: true`
  は project 自己申告の shadowing gate で trust 判別に使えない
  ため)。
- **O2a/O3a — member 軽量検査 + `NoGrant` fold** — member には
  `check_exec_execution`・`verify_pinned_executable`・cwd
  prepare を呼ばない(member argv は宣言されておらず route 解決
  時点で照合対象がそもそも存在しない — direct
  `workflow run --grant` と同じ検査位置へ遅延)。失敗 member 名を
  `tracing::debug!` で帰属し、`NoGrant` 一種 fold で viability
  値上の区別不能を debug log で補う(record・`route resolve`
  出力 shape は不変 — O3a)。
- **O5a — `argv_shell_re` 分岐と定義の両方除去** — quoted
  metachar(`git log -G 'a|b'`・`a;rm`)は `tokenize_shell` が
  `unsafe_token` を立てないため `Runnable` 到達可となり unit
  test 期待値を反転。`\0` 含 arg は tokenize・JSON 入力の両
  経路を通過して `Runnable` 到達可で、`route run` → spawn
  (`std::process::Command`)の `InvalidInput` で bounded job
  error として fail — viability 層へ `[\0\r\n]` regex を再導入
  しない(O5b 不採用の帰結 —
  `proc_command::metachar_args_pass_through_and_nul_degrades_to_
  spawn_error` で fail 位置を pin)。
- **O4a — 投影関数の二重防御** — `project_grants` 内部でも
  `is_route_eligible` を再適用(TOCTOU ではなく投影関数自身の
  不変条件 — 現行単一版と同構造)。`names.is_empty()` は
  fail-safe で false(空集合で retain すると anchor ごと全
  `!fixed` grant が落ちる到達不能入力への防御)。
- **executed provenance は `InternalError` fail-loud** —
  `expected_input` gate が `job_id=="exec" && tool=="command"`
  の `command::execute` 経路に `details.{grant,argv}` を保証
  済み(ProcIdentity.details 書込み)のため、欠落は infra
  invariant 違反として `exitCode` と同じ `InternalError` —
  record に silent absence を残さない。gate 保証外経路からの
  誤 InternalError は test で欠落 case を明示 pin(plan risk
  #6)。
- **member job への routed-exec context 非適用** — `exec_cwd`・
  passthrough `extra_env`・`accept_any_exit_code`・`log_tee` は
  exec job 専用(design §(e) 確定 — member は route firing の
  一部ではなく宣言済み capability を投影 policy の下で呼ぶ通常
  job。env-prefix/passthrough 意味論が原 shell command へ
  結び付くため member job へ横流しすると attribution が曖昧に
  なる)。
- **anchor 規約は code 不変で契約固定** — `expected_input.name =
  route.grant`・byte-match gate・reason `"exec-input-mismatch"`・
  exec job の bounds が `route.grant` 宣言に従う点は code 変更
  なしで明文化(O6a)。member を呼ぶ job は `job_id != "exec"`
  で宣言する authoring 規約を `route_run` test で回帰 pin。
- **T1→T2→T3 の compile transient は scoped verification** —
  T1 が `Route` field・T2 が `project_grants` signature・
  T3 が `RouteRecordInput` field を変えるため各 task を単一
  crate test で閉じ T3 で workspace compile を回復(#163/
  #164/#165 と同じ既知の直列依存処理 — design/plan で予告済み)。

## Related Experiments

### Adopted

なし(known-solution task。design が宣言 parse・viability 分割・
`project_grants` signature・record split・error 契約・call site
配線・churn 一覧を verbatim rust + file:line 精度で固定済みのため
Experiment Log は未作成)。

### Rejected

なし。

## Validation

targeted cargo test・doc check(2026-10-23 実測、全 pass / 0
failed — T5 task 記録):

- `mise run check`: exit 0 — biome → `cargo fmt --check` →
  clippy `--all-targets`(dev-seam)→ version → doc-ids →
  `cargo test --workspace`(dev-seam)0 failed(`route::
  resolve_member_grants_viability`・`route_run::
  exec_job_named_member_is_input_mismatch`・`usage_log::
  route_record_shape`・`proc_invocation::
  grant_set_projection_preserves_scope_parity` 含む)→
  `node --test`(scripts 43 / benchmarks 12 pass)→ lockfile →
  apm-payload、全 stage pass
- `node scripts/check-doc-ids.mjs`: exit 0 — "doc id check OK:
  1156 markdown files scanned"
- `node .agents/skills/doc-status/scripts/audit_docs.js --json`:
  exit 0 — 本 issue の新規 doc 7 件(plan・test-spec・task×5)で
  severity "error" 0 件
- spec Acceptance Criteria 全項目に対応する test が存在し pass
  (TSPEC Rule ↔ spec AC 対応表は T5 task §1 で確認済み):
  `routes.rs` unit(grants parse/dedupe/`[]`/malformed/project 層
  `grants` presence 拒否・builtin/user 受理・project `grant`
  単数受理)・`resolve.rs` unit(member 未存在/project-scope/
  fixed/pin 不能 → `NoGrant` fold・member 失敗が
  stdin-unsupported より先・`a;rm` quoted metachar → `Runnable`)
  ・`route.rs` integration(同上 + fixture)・`route_run.rs`
  integration(member 起動・undeclared 拒否・member argv 遅延
  検査・`exec-input-mismatch` authoring 規約・record
  `executed`)・`exec.rs` unit(set 投影・空集合/ineligible 混入
  false)・`proc_invocation.rs`(set 投影 parity)・
  `proc_command::metachar_args_pass_through_and_nul_degrades_to_
  spawn_error`(`\0` 含 arg は viability 層 regex 非再導入で
  spawn `InvalidInput` へ degrade)・`usage_log.rs` unit +
  `route_record_shape`(`executed` shape・`commandHash`/
  `schemaVersion` 不変)・`run.rs` test(`prepare_projects_
  grant_set`・`finish` の `executed` attribution・欠落
  `InternalError`)
- grep sanity(T5 実測): `grep -r argv_shell_re crates/` 0 件・
  `grep -rn "project_grant\b" crates/` 残件なし(複数形
  `project_grants` 参照 12 件のみ)・`Route` literal 全件
  `grants` field あり・`TODO`/`unimplemented!`/stub 残存なし
- 端到端構造確認: viability の member ループが anchor 完全検査
  直後にあり member へ `check_exec_execution`/
  `verify_pinned_executable`/cwd prepare を呼んでいない・
  `Runnable` 通過 route で `project_grants(&route.grant_set())`
  が `{route.grant} ∪ grants ∪ {fixed}` を残し undeclared 名が
  `resolve_exec_grant` で `PolicyViolation`・`finish()` の
  `executed` が captured `details` 由来で env-prefix 値を含ま
  ない・`load_or_none` が grants schema error を fail-open で
  route arm 不活性へ fold

証跡 commit: planning chain `3b9ea18`(design)→ `2bc0209`
(design review 指摘反映)→ `e921572`(design approved 昇格)→
`c00bed9`(plan + test-spec + tasks)→ 実装 `434f4ca`(T1 route
schema + grant_set)→ `471865b`(T2 viability split +
`argv_shell_re` 除去 + `project_grants`)→ `f5e8d54`(T3 record
executed split)→ `0419697`(T4 docs)→ `25529ff`(T5 検証記録 +
plan/task status 更新)。本 IR はその後に追加する。

## Risks

- **`NoGrant` fold の診断性** — anchor 失敗と member 失敗が
  `route resolve` の `no-grant` で区別不能。spec 確定の
  fail-safe fold で、`tracing::debug!` の member 名帰属で
  補完する。`grants` echo・viability 分解は Deferred(確立した
  需要が出た時点で検討)。
- **member argv の遅延評価** — member job argv は route 解決時に
  未確定のため検査不可能。宣言 argv pattern 外 args は job 実行時
  `check_exec_execution` で拒否されるが routed workflow 内の
  job failure として観測される(direct `workflow run --grant`
  と同じ検査位置 — O2a で確定)。
- **member pin の 2 回評価 gap(新規失敗 domain)** — member pin
  は viability と scope 構築(`pin_exec_grants` が投影全
  `!fixed` grant を pin)で二度解く。viability 通過後の PATH
  変動/policy 更新で scope 構築が `PolicyViolation` すると、
  当該 member を呼ばなくても routed run 全体が hard error —
  `verify_pinned_executable` の spawn 前検出と同じ race domain
  で pin cache は新設しない(design §(b-2)/(i) で受理・error
  契約として記録済み)。
- **`executed.argv` の平文 arg 露出** — usage.jsonl 初の
  cleartext arg 値(`commandHash` は一方向 hash・route log が
  raw output を平文保持する既存露出面と同等 — design 受理済み)。
  env-prefix 値は prefixed argv が prepare で拒否されるため
  混入しないが arg 値自体の secret は残る —
  parent-agent-integration・usage.md で明記済み。
- **member timeout の silent clamp(known semantics)** —
  workflow deadline は anchor grant `timeout` のみ由来
  (`run.rs` deadline_ms)。member が長い `timeout` を宣言しても
  workflow deadline に黙って clamp される — max-of-set 化は
  Deferred(design §(e)・Risks で記録済み)。
- **interactive member は筛かれない(known semantics)** —
  `is_interactive`/`StdinUnsupported` screening は原 command の
  `executable`/`exec_args` のみ対象。member grant が stdin 前提
  executable(REPL・editor 等)を pin しても member 検査では
  止まらず自身の `timeout` まで hang して fail — route 層へ
  member の interactive 分類を追加しない(operator 責務 —
  design §(e) で記録済み)。
- **member 失敗で route 全体が不発** — member 1 件の未存在・
  `scope:"project"`・fixed・pin 不能で route 全体が `NoGrant`
  → 原 command 直接実行へ fallback。宣言委譲面が壊れた route を
  中途半端な投影 policy で発火させない fail-safe fold(意図的)。
- **project 層 `grants` 拒否の非対称** — anchor `route.grant`
  単数は project 層でも宣言可(byte-match が typed command へ
  拘束 — R7 の argv-match が保護)だが member は trusted 層限定
  (member argv は typed command と独立で argv-match 保護が届か
  ず、project 層の member 選択は O4c が塞いだ審査迂回を route
  宣言経由で再発させる — design §(f) で境界を固定済み)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #166)由来の未分類・未処理の follow-up はない。下記は
全て spec/design「Deferred Design Concerns」・各 task「Out of
Scope」で scope 外確定・記録済みの defer、別 issue へ移管済みの
項目、または design で受理・記録済みの既知 semantics であり、
新規 chain は起こさない。

- **documented-deferred — `grants` の project 層開放(O7b)
  再評価** — O4c の審査迂回防止と非対称で不採用。project
  workflow が member を必要とする実需要が確立した場合、anchor
  相当の argv-match 拘束を member にも掛ける別機構
  (declared-member-argv の workflow 側照合等)とセットで再評価
  (design Deferred で記録済み)。
- **documented-deferred — workflow deadline の max-of-set 化** —
  deadline は anchor grant `timeout` のみ由来(known
  semantics — silent clamp は Risks 記録済み)。member の
  timeout 期待が発火側と乖離する運用が出た場合に
  `deadline = max(anchor, members)` への変更を検討。
- **documented-deferred — interactive member の screening** —
  member grant の stdin 前提 executable は member 検査で筛か
  れず自身の `timeout` まで hang(known semantics —
  design §(e) で operator 責務として記録済み)。route 層への
  member interactive 分類追加は行わない。
- **documented-deferred — `route resolve` 出力の `grants`
  echo** — R11 出力 shape(`route_cmd.rs`)は不変。member 失敗と
  anchor 失敗を分ける診断 field は viability 区別の需要が確立
  した時点で検討。
- **documented-deferred — member 失敗の viability 分解** —
  新規 viability 値は採らない(ADR O3a 確定 — enum 拡張と
  hook・route_cmd・全 test の downstream 影響を伴い発火可否の
  意味論が変わるため spec で却下)。
- **documented-deferred — captured 前 exec job 失敗経路の
  record** — record は captured result 必須の現行 timing を
  維持。exec 不発経路の観測盲區は route 観測性の別 issue。
- **documented-deferred — `grants` 宣言数上限** — 現行 no-cap
  (R1 確定・`passthroughEnv` と同規約)。各 member は viability +
  scope 構築で 2 回 PATH 解決されるため pin 解決コストが問題化
  した場合のみ cap 導入を検討。
- **documented-deferred — member job への routed-exec context
  適用** — `exec_cwd`・passthrough env・`accept_any_exit_code`・
  `log_tee` は exec job 専用(design §(e) で「適用しない」を
  固定)。需要が出た場合は原 command 再現の意味論との関係を
  含め別途設計。
- **moved-to-issue — substitute mode(#167)の anchor 切替** —
  `expected_input`/`executed` の意味は byte-matched mode で
  固定。`substitute.grant` 導入時に R6/R7 anchor 規約を再評価
  (spec/design Deferred と同一 —
  https://github.com/scarecrow173/cheaper/issues/167)。
- **moved-to-issue — trusted flag 発行主体** — issue #168
  (#151 分解 6)の scope —
  https://github.com/scarecrow173/cheaper/issues/168。
- **moved-to-issue — docs 全体の正直性 sweep + migration
  guide** — issue #169 の scope —
  https://github.com/scarecrow173/cheaper/issues/169。
- **closed — member pin の scope 構築時再失敗(新規失敗
  domain)** — viability 通過後の PATH 変動/policy 更新で
  routed run 全体が `PolicyViolation` hard error し得る点は
  design §(b-2)/(i) error 契約で受理・記録済み(既存
  `verify_pinned_executable` の spawn 前検出と同 race domain —
  pin cache は新設しない KISS 確定)。

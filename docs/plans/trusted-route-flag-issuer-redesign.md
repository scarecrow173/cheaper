---
id: "PLAN-034L59uXskRD4ink3h2a13"
type: "plan"
status: "approved"
title: "trusted route flag issuer redesign — Route.trusted 削除・merge 無条件化・project 層 trusted presence 拒否"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/168"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
  implemented-by:
    - "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
    - "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
    - "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
  derived-by:
    - "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
    - "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
    - "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
  verifies: []
  verified-by:
    - "docs/test-specs/trusted-route-flag-issuer-redesign.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/plans/route-grant-set-projection.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/test-specs/substitute-exec-route-mode.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route_cmd.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "crates/cheaper-cli/resources/routes.yaml"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign — Route.trusted 削除・merge 無条件化・project 層 trusted presence 拒否 Implementation Plan

## Goal

DESIGN-034L4tqYwtc8q88WEIXPh9 の実装契約に従い、route の
trust 判定を「宣言値(`Route.trusted` flag)」から「**宣言層
provenance**(`layer ∈ {Builtin, User}` = trusted、`layer ==
Project` = untrusted)」へ移す(ADR-034LT2o68CgcOinMEs8PZ8 の
O1a + O2a 採用 — #151 権限モデル再設計の実装分解 6/7)。
構成は 3 面 + 整合記述: (1) merge の project 分岐を
`route.trusted` を一切読まない無条件形へ書き換え(衝突 →
`suppressed` provenance 記録・非衝突 → 有効化)し、
displaced 下位層 route の `suppressed` 記録 arm(到達不能化)
を除去、(2) `validate_route` に project 層 `trusted` key
presence 拒否 gate を新設(`grants`/`substitute` と同じ
判別・gate 位置・error class・presence-first —
`{label}.trusted: not declarable in the project layer` →
`InvalidRuntimePolicy`)、(3) `Route.trusted` field を削除し
`Route.layer` provenance を trust・診断・target.path 層起点
解決の単一 source へ確定させる(builtin/user 層の `trusted`
key は受理・bool 検証・inert を維持、`KNOWN_KEYS` 登録も維持)。
route trust(宣言層 provenance)と grant `scope`(authoring 層
provenance)の「trust は宣言主体が決める」単一モデル整合は
`is_route_eligible` 単一 seam 不変として docs で固定する
(spec R5)。

## Scope

単一の implementation stream。「trust 判別子を宣言層
provenance へ畳む」という 1 つの semantic 変更に、field
削除の signature churn・schema gate・merge 差分・test
更新・docs 追従が横断的に付随する構成であり、API 形状・
error 契約・churn 一覧は design が file:line 精度で固定済み
— 分割するのは commit の論理境界(route code+test 着地 →
hook fail-open pin → docs 契約 → 検証)のみで、plan は
1 つに収める。

| In | Out |
|---|---|
| `Route.trusted` field 削除(`routes.rs:85-87` — doc comment ごと)+ `Route.layer` への trust provenance 単一 source 旨の doc 追記(:88) | operator/user 側 consent registry(O1b — project route shadow の個別許可機構 — 需要未確認で YAGNI、forward-compat は保持) |
| merge(`route::routes::load`)の project 分岐を無条件形へ(:198-220)— `route.trusted` 読取り(:201)・displaced 記録 arm(:210-218 — 到達不能)除去 | builtin/user 層の inert `trusted` key の schema 完全除去(routes.yaml `version` bump が必要な schema 破壊 — 受理・inert 維持は Deferred) |
| `validate_route` への project 層 `trusted` presence 拒否 gate(:627 直前 — `grants`/`substitute` と同型・presence-first)+ builtin/user 受理の bool 形状検査への縮小(:627-633 — 値は保持しない) | project route の新規 prefix 宣言権・`grant:` anchor 宣言権・発火・viability・`expected_input`・exit mirror の変更(不変) |
| `KNOWN_KEYS` の `"trusted"` 登録維持(:397)— 拒否は層 gate、未知 key 規約とは別 | grant `scope` モデル・`is_route_eligible`・`project_grants`・`route`/`command` policy 節の変更(#165/#166 確定のまま) |
| `SuppressedRoute` doc 狭義化(:121-124)・file header merge 規約 comment(:12-16)・`RouteResolution.suppressed` doc(`resolve.rs:101-104`)・`grants`/`substitute` gate comment(:476-480/:518-519)の `trusted` 言及追従 | `suppressed`/`invalid`/`route resolve` 出力 shape・`kind:"route"` record shape・`schemaVersion` の変更(不変) |
| 破壊的変更の test pin — `trusted` key を含む project routes.yaml は `true`/`false`/非 bool 問わず file 全体 load 失敗(`InvalidRuntimePolicy`) | Option E(exec-less route)— 親 chain の open 事項 |
| `routes.rs` unit test 更新 + 新規 `project_layer_trusted_is_rejected`・integration `route.rs` rewrite(`resolve_project_trusted_rejected`)+ `route_run.rs` `for trusted` loop 撤去 + `hook.rs` fail-open 回帰 test(project 層違反 file) | 文書正直性 sweep・migration guide 本文(#169 が担う — 本 plan は告知材料の供給のみ) |
| docs 追従 — `parent-agent-integration.md`(:233-246 rewrite + grant `scope` 整合 :291-295)・`configuration.md`(:215-225)・`command.md`(:42)・`resources/routes.yaml` comment(:8 直下)・pretooluse spec content update(:216-219/:253-256/:285-298) | consent registry・staleness semantics の設計(Deferred — design §Deferred) |

## Upstream Documents

- `docs/specs/trusted-route-flag-issuer-redesign.md`
  (SPEC-034LwdoInh3ie1KPlSjkJl、proposed — R1-R6・
  Acceptance Criteria AC1-AC5。status 昇格は spec chain 側の
  管理 — §Dependencies and Risks 参照)
- `docs/adr/trusted-route-flag-issuer-redesign.md`
  (ADR-034LT2o68CgcOinMEs8PZ8、proposed — O1a + O2a 採用:
  trust = 宣言層 provenance・project 層 `trusted` presence
  fail-loud 拒否。O2e 隔離案は審議済み・不採用)
- `docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md`
  (DISC-034L44O8jYRDhvhM16gDiM、resolved — 全接点の実測
  inventory)

## Design Inputs

- `docs/designs/trusted-route-flag-issuer-redesign.md`
  (DESIGN-034L4tqYwtc8q88WEIXPh9、approved — AUTHORITATIVE な
  HOW: (a) route declaration(field 削除 verbatim・`KNOWN_KEYS`
  処遇・層 gate verbatim rust)・(b) merge semantics(無条件形
  verbatim・displaced arm 除去)・(c) suppressed 報告経路
  (consumer 不変の確認)・(d) grant `scope` seam 整合(文書化
  契約)・(e) signature 契約表・(f) call site 配線全一覧・
  (g) error 契約表・Churn 一覧・Docs 更新契約・Verification
  Notes(11 checkbox — spec AC への写像))。
  本 plan・task は design の行指定を転記するだけで、API 形状・
  error 文言・適用位置を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/route/routes.rs` | `Route.trusted` field 削除(:85-87)+ `Route.layer` doc 追記(:88)・file header merge 規約 comment 追従(:12-16)・`SuppressedRoute` doc 狭義化(:121-124)・merge project 分岐の無条件化(:198-220 — `route.trusted` :201 除去・displaced arm :210-218 除去)・`grants`/`substitute` gate comment の `trusted` 言及追従(:476-480/:518-519)・project 層 `trusted` presence 拒否 gate 新設(:627 直前)+ `trusted` parse を bool 形状検査のみへ縮小(:627-633)・`Route` literal から `trusted,` 除去(:643)・unit test 更新(:800-857 rewrite+rename・:1025-1033/:1252-1260 comment 追従・:1350 literal 除去・:1366-1405 rewrite)+ 新規 `project_layer_trusted_is_rejected` | T1。declaration・merge・層 gate の全責務をこの file に閉じ込める |
| `crates/cheaper-cli/src/route/resolve.rs` | `RouteResolution.suppressed` field doc 追従(:101-104 — displaced 記述除去)+ test `route()` helper literal から `trusted: false` 除去(:434) | T1(field churn + comment のみ — resolve 本体不変) |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | test `route()` helper literal から `trusted: false` 除去(:2090 — 評価本体は不変) | T1(field churn のみ) |
| `crates/cheaper-cli/tests/integration/route.rs` | `resolve_project_trusted_displacement_reports_suppressed`(:285-336)を rewrite — `resolve_project_trusted_rejected`(`trusted` presence → `INVALID_RUNTIME_POLICY`/exit 3、:266-283 `resolve_project_substitute_rejected` と同型)+ untrusted 衝突 → `suppressed` 報告の回帰 arm | T1(旧 semantics test — 新 code では red になるため同 commit で rewrite) |
| `crates/cheaper-cli/tests/integration/route_run.rs` | `suppressed_project_provenance_reaches_route_record`(:1386-1455)— `for trusted` loop 撤去、untrusted arm(resolve 出力・record の `suppressed` = 負けた project route 名)のみ残す | T1(同上 — red 回避のため同 commit) |
| `crates/cheaper-cli/tests/integration/hook.rs` | hook fail-open 回帰 test 新規 — 違反 file は **project 層** `<workspace>/.cheaper/routes.yaml` に置く(既存 `routed_shell_fixture` :851 は `f.cfg` = user 層で層 gate の対象外 — 再使用不可)。user 層に valid route があっても `load_or_none`(`supply.rs:138-152`)が全層 route arm を不活性化し command は直接実行経路へ落ちることを pin | T2 |
| `docs/guides/parent-agent-integration.md` | route 宣言層説明(:233-246)rewrite「project は下位層を shadow しない — trust は宣言層 provenance が決める。project routes.yaml の `trusted` key presence は route load 全体を `InvalidRuntimePolicy` で失敗させる。builtin/user の `trusted:` は受理・inert」+ grant `scope` 整合(spec R5)を grant 参照記述(:291-295 近辺)へ 1 文追加 | T3 |
| `docs/guides/configuration.md` | unified exec 参照節(:215-225)の層 gate 列挙(「`grants`・`substitute` key が存在すると route load が失敗」)へ `trusted` を追加 | T3 |
| `docs/guides/command.md` | route grant 参照記述(:42)の層 gate 列挙へ `trusted` を追加 | T3 |
| `crates/cheaper-cli/resources/routes.yaml` | `substitute` shape comment(:8)直下へ `# trusted: bool  # optional — builtin/user 層のみ受理(inert);project 層宣言は load error` の 1 行追加(builtin route は `trusted` 非宣言 — 記述のみ) | T3 |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | R1 field 一覧(:216-219)の `trusted?` 注記・`trusted` 記述(:253-256)・R7 層間 trust(:285-298)を宣言層 provenance trust へ content update(`refines` relation は張らない — spec chain 側の管理) | T3 |
| `route/supply.rs`・`route/run.rs`・`route/payload.rs`・`route_cmd.rs`・`route/logs.rs`・`workflow_run.rs`・`usage_log.rs`・`usage_report.rs`・`policy/exec.rs`・`policy/load.rs`・`tools/exec/mod.rs`・`tools/command/mod.rs`・`hook/targets.rs`・`routed-exec.workflow.json`・`tests/integration/usage_log.rs`・`crates/cheaper-tools/tests/proc_invocation.rs` | **code 不変** — `load_or_none` fail-open 規約・`route.layer` の target.path 層起点解決(:366-370)・record `suppressed` 配線(:633→usage_log:418-419)・`is_route_eligible` 単一 seam(:114-116)・record shape pin(:298-353) | T4 で回帰確認(無改訂で既存挙動が維持される churn 境界) |

## Implementation Sequence

- [ ] TASK-034L59KS9oEjgDzshYZUSn (T1): route code + 結合
      test 着地 — `Route.trusted` field 削除 + `Route.layer`
      doc 追記 + merge project 分岐の無条件形化(displaced arm
      除去)+ `validate_route` の project 層 `trusted`
      presence 拒否 gate 新設 + `trusted` parse を bool 形状
      検査のみへ縮小 + `KNOWN_KEYS` 維持 + 全 `Route` literal
      churn(routes.rs:643/:1350・resolve.rs:434・
      routing_policy.rs:2090)+ code comment 追随
      (routes.rs :12-16/:121-124/:476-480/:518-519・
      resolve.rs :101-104)+ 破壊される test の同 commit
      rewrite(routes.rs unit `user_shadows_builtin_and_
      project_needs_trusted`→`user_shadows_builtin_and_
      project_never_shadows` rename・`merge_records_displaced_
      lower_layer` rewrite・integration `route.rs` →
      `resolve_project_trusted_rejected`・`route_run.rs`
      `for trusted` loop 撤去)+ 新規
      `project_layer_trusted_is_rejected`。
      `cargo test -p cheaper-cli` 全緑で閉じる
- [ ] TASK-034L59Ty2ObpwGwNFduACU (T2): hook fail-open 回帰
      test — project 層 `<workspace>/.cheaper/routes.yaml` の
      `trusted` presence 拒否を含む load 失敗が
      `load_or_none` 経由で全層 route arm 不活性へ畳まれ、
      command が直接実行経路を継続することを
      `tests/integration/hook.rs` で pin。
      `cargo test -p cheaper-cli --test integration hook` で
      閉じる
- [ ] TASK-034L59YCvXZOpmmELxVllt (T3): docs 更新契約 —
      `parent-agent-integration.md` rewrite + grant `scope`
      整合 1 文・`configuration.md`・`command.md` の層 gate
      列挙・`resources/routes.yaml` shape comment・
      pretooluse spec content update(R1 field 一覧・
      `trusted` 記述・R7 層間 trust)
- [ ] TASK-034L59Zaf7xiLEfPo8rlQF (T4): 最終検証 —
      targeted cargo tests・spec AC ↔ TSPEC Rule 対応確認・
      grep sanity(`Route {` literal 全件・`route.trusted`
      残存 0・`trusted` 層 gate 位置)・
      `node scripts/check-doc-ids.mjs`・`audit_docs`・
      `mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 route code + 結合 test 着地 | — | T2, T3, T4 |
| T2 hook fail-open 回帰 test | T1 | T4 |
| T3 docs 更新契約 | T1 | T4 |
| T4 最終検証 | T1, T2, T3 | — |

critical path は T1 → (T2 ∥ T3) → T4。T2 は T1 の層 gate が
存在して初めて検証対象になる(拒否経路を pin する test)、
T3 は T1 の確定 semantics を docs へ転記する — 両者は file
set が完全に disjoint(tests/integration/hook.rs vs
docs/*+resources/routes.yaml)なため T1 後に並列可能。T4 は
全着地後の検証。

既知の transient compile/test 断:

- T1 境界 — `Route.trusted` field 削除は全 `Route` literal
  (routes.rs:643/:1350・resolve.rs:434・routing_policy.rs:2090)
  の同 commit 追随が必須。加えて `trusted` を YAML で宣言する
  旧 semantics test(`user_shadows_builtin_and_project_needs_
  trusted` :800-857・`merge_records_displaced_lower_layer`
  :1366-1405・`resolve_project_trusted_displacement_reports_
  suppressed` route.rs:285-336・`suppressed_project_provenance_
  reaches_route_record` の trusted arm route_run.rs:1386-1455)
  は compile は通るが実行で red になる — #167 先例どおり
  semantics を反転させる test は semantic 変更 commit と同じ
  T1 で rewrite し、`cargo test -p cheaper-cli` 全緑を維持する。
- T2/T3 境界 — 両者の file set は disjoint。T4 の status 更新は
  T1-T4 全 task + plan を touched file として扱う。

各 task は 1 commit で、file-local の unit test は同 commit で
追随する。code comment 追随は全 site が T1 の file(routes.rs・
resolve.rs)に閉じるため T1 に含める(#167 で comment を別 task
へ分けたのは site が複数 code task に跨ったため — 本変更では
分離する利益がない)。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1 commit 境界の compile/test 断(`Route.trusted` 削除 — literal 4 site + YAML 前提 test 4 件、必発) | high(必発) | low | design 確定の churn 一覧どおり同 commit 追随 — T1 は `cargo test -p cheaper-cli` 全緑で閉じる |
| 2 | EXISTING key への presence 拒否(破壊的変更)— `trusted` を含む project routes.yaml は `false`/非 bool・非衝突 route も含め全件 load 失敗(strict は exit 3、hook は fail-open で全層 routing 不活性) | high(確定値) | med | 意図的 trade-off(ADR O2a — 「壊れるなら loud に壊れる」fail-loud 哲学)— 修復 = 当該 key 削除 1 行、#169 migration guide 材料を spec Scope/design §Deferred に固定済み。T3 で guides に症状・診断(`route resolve` まで不可視)を明記 |
| 3 | presence-first の配置漏れ — 値形状検査より後に層 gate を置くと `trusted: <非 bool>` が `not a boolean` で落ち spec R2 の error 文言と不整合 | med | med | design §(a-3) verbatim(:627 直前、`grants`/`substitute` gate と同型)をそのまま転記 — T1 unit test `project_layer_trusted_is_rejected` が非 bool presence を pin |
| 4 | `suppressed` semantics 狭義化の pin 漏れ — displaced 記録 arm 除去後に旧「下位層 route 名を報告する」assert が残ると semantics が曖昧化 | med | med | `merge_records_displaced_lower_layer` rewrite(rename + project-lose 記録のみ assert)・`resolve_project_trusted_rejected` rewrite・route_run untrusted arm pin を T1 Done When に明記 |
| 5 | hook fail-open test の fixture 層違い — 既存 `routed_shell_fixture` は `f.cfg` = user 層で project 層 gate の対象外。同 fixture への `trusted` 追加は user 層受理で無効な test になる | med(易犯) | med | design Verification Notes の必須条件を T2 に転記 — 違反 file は `<workspace>/.cheaper/routes.yaml` に置くことを Done When で pin |
| 6 | `KNOWN_KEYS` から `"trusted"` を外す・project gate を unknown-key 経路で実装すると builtin/user の inert 宣言まで壊れる(O2c 相当の gratuitous 破壊) | low | med | `KNOWN_KEYS` :397 維持を spec R2・T1 Done When で pin — 拒否は層 gate のみ |
| 7 | `route.trusted` を読む経路が merge 外へ復活する・`trusted` 値を別 field/表現へ移す表現 drift | low | med | ADR patterns to avoid — 判別子は宣言層 provenance 1 つ。T4 grep sanity(`route.trusted`・`.trusted` 残存確認)で機械検出 |
| 8 | spec(SPEC-034LwdoInh3ie1KPlSjkJl)・ADR(ADR-034LT2o68CgcOinMEs8PZ8)が proposed のまま — plan 起点の spec status 昇格が未処理 | low | low | design(approved)が file:line 契約を固定済みのため plan 実効性に影なし。spec chain 側の status 追随は T4 の status 更新で扱う(#167 先例) |

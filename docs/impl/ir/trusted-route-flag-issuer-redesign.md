---
id: "IMPL-034L5TOfRVoVwpS83OAebe"
type: "impl"
status: "completed"
title: "trusted route flag issuer redesign"
created: "2026-10-24"
updated: "2026-10-24"
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
    deleted:
      - type: "item"
        path: "crates/cheaper-cli/src/route/routes.rs::Route.trusted"
        note: "privilege flag — trust 判別を宣言値 flag から宣言層 provenance へ畳むため field + doc comment ごと削除(O1a/R3 — 唯一の consumer は merge shadowing gate)"
      - type: "item"
        path: "crates/cheaper-cli/src/route/routes.rs::load merge displaced-recording arm"
        note: "#167 の displaced 下位層 route `suppressed` 記録 arm — project shadow 廃止で `insert` 戻り値が常に `None` となり到達不能化、dead code として残さず arm ごと除去"
    renamed:
      - type: "item"
        path: "crates/cheaper-cli/src/route/routes.rs::merge_records_displaced_lower_layer -> crates/cheaper-cli/src/route/routes.rs::merge_records_losing_project_route"
        note: "displaced 記録 semantics が消えたため rewrite + rename — 「負けた project route のみ `suppressed` に載る」assert のみを残す(task が指定する rewrite+rename を新 semantics の命名で採用)"
    moved: []
    generated: []
  implements:
    - "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
    - "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
    - "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
  implemented-by: []
  depends-on:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
    - "docs/impl/ir/substitute-exec-route-mode.md"
    - "docs/impl/ir/route-grant-set-projection.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/trusted-route-flag-issuer-redesign.md"
  references:
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/test-specs/substitute-exec-route-mode.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# trusted route flag issuer redesign

## Summary

issue #168(#151 実装分解 6/7)— route の trust 判定を宣言値
(`Route.trusted` flag)から**宣言層 provenance**(`layer ∈
{Builtin, User}` = trusted、`layer == Project` = untrusted)へ
移した(ADR-034LT2o68CgcOinMEs8PZ8 — O1a + O2a 採用)。
`trusted` flag の発行主体を project 自己申告から宣言層その
ものへ移し、route trust 判定を `grants`(#166 O7a)・
`substitute`(#167 O3a)・grant `scope`(#165 M3)・`route`
policy 節・`command` 節除外と同じ「trust は宣言内容ではなく
宣言主体が決める」単一モデルへ揃えた。`Route.trusted` field
の削除・merge project 分岐の無条件形化(衝突 → `suppressed`
provenance 記録・非衝突 → 有効化)・`validate_route` への
project 層 `trusted` key presence 拒否 gate
(`{label}.trusted: not declarable in the project layer` →
`InvalidRuntimePolicy` — `grants`/`substitute` と同型・
presence-first)・displaced 記録 arm の除去と
`SuppressedRoute` semantics の「衝突で負けた project route」
への狭義化・builtin/user 層 `trusted:` の受理・inert 維持・
hook fail-open 回帰 pin・route trust/grant `scope` 単一モデル
整合の文書化が実装された。`RouteResolution`・`route resolve`
出力 shape・`kind:"route"` record shape・`is_route_eligible`
単一 seam・viability・`expected_input`・exit mirror は不変。

1. `routes.rs` + literal churn + test rewrite(T1・`d2daee4`):
   `Route.trusted` field 削除 + `Route.layer` doc 追記・merge
   project 分岐の無条件形(`route.trusted` 読取り・displaced
   記録 arm 除去)・project 層 `trusted` presence 拒否 gate +
   `trusted` parse の bool 形状検査のみへの縮小・`KNOWN_KEYS`
   維持・comment 追随 6 site・全 `Route` literal 追随・unit/
   integration test の rewrite + 新規
   `project_layer_trusted_is_rejected`。
2. hook fail-open 回帰(T2・`439ae8e`):
   `project_trusted_violation_disables_route_arm_fail_open` —
   project 層 `.cheaper/routes.yaml` の `trusted` presence
   を含む load 失敗が `load_or_none` で全層 route arm 不活性
   へ畳まれ、command が unrouted eval 経路で処理され続ける
   ことを pin(production code 不変・test のみ)。
3. docs(T3・`5ec9d0c`): `parent-agent-integration.md` rewrite
   + grant `scope` 整合 1 文 + fail-open 症状・vendor migration
   経路・`configuration.md`/`command.md` 層 gate 列挙・
   `resources/routes.yaml` shape comment・pretooluse spec
   content update(R1 field 一覧・`trusted` 記述・R7 層間
   trust — relation は張らず spec chain 側管理)。
4. 検証(T4・`cb154f4`): spec AC1-AC5 ↔ TSPEC Rule 対応確認・
   構造確認(層 gate 位置・merge 無条件形・field 削除完結・
   `KNOWN_KEYS` 維持・suppressed 狭義化・hook fail-open)・
   `check-doc-ids` 1179 files・`audit_docs` error 0・
   `mise run check` exit 0・status 更新。

## Context

TASK-034L59KS9oEjgDzshYZUSn(T1)・TASK-034L59Ty2ObpwGwNFduACU
(T2)・TASK-034L59YCvXZOpmmELxVllt(T3)・TASK-034L59Zaf7xiLEfPo8rlQF
(T4)。PLAN-034L59uXskRD4ink3h2a13 /
DESIGN-034L4tqYwtc8q88WEIXPh9 / SPEC-034LwdoInh3ie1KPlSjkJl /
ADR-034LT2o68CgcOinMEs8PZ8 / TSPEC-034L59Ghpuk8XWUN8qgXJu に
従う。全接点の実測 inventory は DISC-034L44O8jYRDhvhM16gDiM
(resolved)で固定済み — `route.trusted` を読む consumer は全
crate で merge の shadowing gate(`routes.rs:201` — 旧行番号)
のみであり、viability・grant 解決・record・`route run` は
`route.trusted` を読まない。**trust が分岐するのは merge
precedence の一点だけ**で、blast radius は `routes.rs` の
merge + schema gate + field 削除 churn に閉じる。前提となる
#165(`ExecGrantScope` provenance・`is_route_eligible` 単一
seam)・#166(`grants` 層 gate・`suppressed` provenance)・
#167(`substitute` 層 gate・displaced 記録 arm — 本 issue で
到達不能化のため除去)は実装済みであり、本変更は route 側の
trust 判別を同一判別子(宣言層 provenance)へ揃える収束変更で、
core/tools 側 code は一切不変(merge-base から無差分を T4 で
実測確認)。本 IR は `derives-from`/`implements`/`verified-by`/
`depends-on` を chain 内 doc 集合で閉じ、cross-chain doc
(#163-#167 の IR・pretooluse/permission-model/unified-exec/
grant-set/substitute chain の spec/ADR/design/test-spec・
file-issue task・guide)は `related`/`references` でのみ参照
する — lineage edge(refines)は持たない。実装は design 確定の
verbatim rust どおりで、design からの deviation は 2 件のみ
(T1 task §Verification 記録):

- `v.is_bool()` → `v.is_boolean()` — design §(a-3) verbatim
  の serde_json API 名が実在せず `cargo check` が E0599 を
  検出したため、実在する `is_boolean()` で実装(挙動は design
  の契約どおり「値を保持しない bool 形状検査」のまま)。
- test rename — task が指定した rewrite + rename を、新
  semantics(負けた project route のみを記録)に合わせた命名
  `merge_records_displaced_lower_layer` →
  `merge_records_losing_project_route` で採用。

## Implementation

### Changes

- `crates/cheaper-cli/src/route/routes.rs`: `Route.trusted`
  field を doc comment ごと削除(旧 :85-87)し、`Route.layer`
  (:84-88)へ「trust 判定(`layer ∈ {Builtin, User}` が
  trusted)#168・target.path 層起点解決・診断の単一 source —
  `Route` は trust flag を持たない」旨の doc 追記。file header
  merge 規約 comment(:12-15)を「project は `trusted: true`
  のときのみ下位層を shadow する」から「project は下位層を
  shadow しない(trust = 宣言層 provenance)— 衝突した
  project route は `suppressed` provenance として記録される」
  へ追従。`SuppressedRoute` doc(:121-125)を「merge で負けた
  route」から「同一 `match.command` で下位層(builtin/user)
  route に負けた project route」へ狭義化 — shape
  `{name, command}` 不変。merge loop の `RouteLayer::Project`
  分岐(:199-215)を無条件形へ書き換え — `effective.
  contains_key(&route.command)` のみで分岐(衝突 → 自身を
  `suppressed` 記録・非衝突 → `order.push` + `insert`)、
  `route.trusted` 読取り・displaced 記録 arm を除去。
  builtin/user 分岐は文面不変(user は builtin を無条件
  shadow — 同一 trusted principal の宣言 override・silent
  維持)。`KNOWN_KEYS`(:393)の `"trusted"` 登録は維持 —
  拒否は層 gate で行い未知 key 規約とは別(O2c 不採用)、
  `trustd` 等の typo は従来どおり unknown key で route-load
  error。`validate_route` へ層 gate 新設(:629-633 —
  `passthroughEnv` parse 後・`trusted` bool 検査前の
  presence-first 配置で `layer == RouteLayer::Project &&
  entry.contains_key("trusted")` → `schema_error(
  "{label}.trusted: not declarable in the project layer")`
  → `InvalidRuntimePolicy`。`grants`/`substitute` と同じ
  判別子・gate 位置・error class — 層 gate 群の発火順は
  schema 記述順 grants → substitute → trusted の逐次
  fail-loud)。`trusted` parse は builtin/user 受理の bool
  形状検査のみへ縮小(:639-645 — `if let Some` + `is_boolean()`
  で「検査するが使わない」を自明に、値は `Route` に保持
  しない。非 bool は `{label}.trusted: not a boolean` —
  受理層の型検査規約不変)。`grants`/`substitute` gate
  comment(:472-480・:506-519)の `trusted` 言及を「project
  層の `trusted` key 自体が本 file 後段の層 gate で拒否
  される — 判別子は宣言層 provenance」へ追従。`Route`
  literal から `trusted,` 除去。unit test —
  `user_shadows_builtin_and_project_needs_trusted` を
  `user_shadows_builtin_and_project_never_shadows` へ
  rename + trusted shadow arm 撤去(:812 — user→builtin
  無条件 shadow・untrusted 衝突負け + `suppressed` 記録・
  非衝突有効化 arm を維持)、`merge_records_displaced_lower_
  layer` を `merge_records_losing_project_route` へ
  rewrite + rename(:1366 — project-lose 記録のみ assert)、
  `project_layer_grants_are_rejected`・`project_layer_
  substitute_is_rejected` の `trusted:true` 併記 arm
  comment 追従(:1019/:1246 — assert 自体は不変、gate 順は
  grants/substitute が先)、新規
  `project_layer_trusted_is_rejected`(:1410 —
  `trusted: true`/`false`/`trusted: str` の 3 形 presence
  拒否 + builtin/user 層の `true`/`false` 受理・非 bool 値の
  `{label}.trusted: not a boolean` pin — spec R2 両面)。
- `crates/cheaper-cli/src/route/resolve.rs`:
  `RouteResolution.suppressed` field doc(:101-104)を
  「trust 不在で負けた project route + displaced 下位層
  route」から「同一 `match.command` で下位層に負けた
  project route(#168 で displaced 記録は project shadow
  廃止とともに除去)」へ追従 + test `route()` helper literal
  (:434)から `trusted: false` 除去 — resolve 本体・
  `suppressed` lookup(:221-228)・`Viability`・出力 shape
  は不変。
- `crates/cheaper-cli/src/hook/routing_policy.rs`: test
  `route()` helper literal(:2090)から `trusted: false`
  除去のみ — 評価本体は不変(`load_or_none` fail-open 規約に
  presence 拒否も乗る既存構造)。
- `crates/cheaper-cli/tests/integration/route.rs`:
  `resolve_project_trusted_displacement_reports_suppressed`
  を `resolve_project_trusted_rejected` へ rewrite(:292 —
  `resolve_project_substitute_rejected` と同型。project 層
  `.cheaper/routes.yaml` entry の `trusted: true` presence
  → `route resolve` が `INVALID_RUNTIME_POLICY`・exit 3・
  `env["ok"] == false`(presence が file 全体 load を失敗
  させる — issue AC2 の最強形 pin)+ `trusted` 非宣言の
  project route が同一 `match.command` で user 層 route に
  負け `result["route"]` が下位層 route 名・
  `result["suppressed"]` が負けた project route 名を報告
  する untrusted 衝突 arm(回帰固定 — spec R4)。
- `crates/cheaper-cli/tests/integration/route_run.rs`:
  `suppressed_project_provenance_reaches_route_record`
  (:1387)の `for trusted in [false, true]` loop を撤去し
  untrusted arm のみ残す — `trusted` 非宣言の project
  route が builtin workflow route に負け、resolve 出力と
  `kind:"route"` record の `suppressed` が負けた project
  route 名(`project-shadow`)を pin。
- `crates/cheaper-cli/tests/integration/hook.rs`: 新規
  `project_trusted_violation_disables_route_arm_fail_open`
  (:955)— user 層 `routed_shell_fixture` に valid
  routes.yaml + grant policy + workflow を配置した control
  構成へ、project 層 `<workspace>/.cheaper/routes.yaml` の
  `trusted: true` entry を違反 file として追加(target
  `w.json` 実在 — 層 gate 不在の旧 semantics では
  `trusted: true` project route が user 層 route を shadow
  して runnable → Rewrite 発火で `!eval_records(&fx).
  is_empty()` が fail する識別力を保持、fixture comment に
  明記)。presence 拒否 → file 全体 load 失敗 →
  `load_or_none`(`supply.rs:138-152`)が `None` へ fold →
  builtin/user/project 全層の route arm が不活性化し
  `cat big.txt` が unrouted eval の deny 側経路(exit 2 +
  dedup record publish — route arm 発火時の `Rewrite`
  verdict は persist されない対照)で処理され続けることを
  pin(fail-closed な schema 強制と fail-open な hook 合成の
  責務分離 — spec R6)。production code 不変・test のみ追加。
- `crates/cheaper-cli/resources/routes.yaml`: `substitute`
  shape comment 直下へ `# trusted: bool  # optional — builtin/user 層のみ受理(inert);project 層宣言は load error`
  の 1 行追加(builtin route 自身は `trusted` 非宣言 —
  記述のみ)。
- `docs/guides/parent-agent-integration.md`: route 宣言層説明
  を「project 層 route は下位層を shadow しない — trust は
  宣言値ではなく宣言層 provenance(builtin/user 層宣言 =
  trusted)が決める。衝突 project route は `suppressed`
  provenance として報告・下位 route は active のまま。
  project routes.yaml の `trusted` key presence は値に
  関わらず route load 全体を `InvalidRuntimePolicy` で失敗
  させる。builtin/user 層の `trusted:` は受理・inert」へ
  rewrite + route trust と grant `scope` の整合(spec R5 —
  宣言層/authoring 層 provenance の単一モデル 2 面・
  `is_route_eligible` `scope=="trusted" && !fixed` 単一
  seam 不変)を grant 参照記述へ 1 文追加 + hook 経路
  fail-open の症状(全層 routing 不活性・debug log のみで
  `route resolve` まで不可視 — strict 経路は exit 3)・
  修復(key 削除 1 行)・project routes.yaml の
  `<userConfigDir>/` vendor 経路(user 層では `trusted` inert
  受理で shadowing が無改修で復活する zero-edit migration)
  を明記。`configuration.md`・`command.md`: 層 gate 列挙を
  `grants`・`substitute`・`trusted` の 3 key 拒否へ更新・
  「project が宣言できるのは単数 `grant:` anchor 参照のみ」
  記述と整合。`docs/specs/pretooluse-route-resolver-and-
  workflow-delegation.md`: R1 field 一覧の `trusted?` 注記・
  `trusted` 記述・R7 層間 trust を宣言層 provenance trust
  へ content update(`refines` relation は張らず spec chain
  側管理)。

### Decisions During Implementation

- **O1a — trust = 宣言層 provenance** — merge の project
  分岐は `route.layer` provenance のみを読み `route.trusted`
  宣言値を一切読まない無条件形。`layer ∈ {Builtin, User}`
  の route は同一 `match.command` で下位層を無条件 shadow
  (現行不変 — user→builtin silent は同一 trusted principal
  の宣言 override)、`layer == Project` の route は `trusted`
  宣言の有無・値に関わらず衝突時に常に負けて `suppressed`
  provenance へ記録される。project 層が下位層を shadow する
  経路は構造的に消え(flag 申告も registry も要らない —
  判定は宣言位置に畳み込まれる)、trust 判別子は宣言層
  provenance 1 つに統一され flag 判別と provenance 判別の
  二重経路が解消した。
- **O2a — project 層 `trusted` presence を fail-loud 拒否** —
  `trusted` key は trust 申告そのものであり、untrusted 層は
  「申告を置くこと自体」ができない。`grants`/`substitute`
  と同じ判別・gate 位置(該当 key の値形状検査直前)・
  error class(`schema_error` → `InvalidRuntimePolicy`)・
  presence-first 規約で、`true`/`false`/非 bool の全形を値
  を見ずに拒否 — 「宣言しても判定を変えられない」ではなく
  「申告経路が load 時に閉じる」最強形(issue AC2)。`false`
  受理の O2d は presence-first discipline と矛盾する
  incoherent な意味論として不採用。O2e(違反 entry のみ
  `invalid` 隔離)は `invalid` bucket が同層 dup quarantine
  専用で schema 違反の観測格下げは ignore(O2b)へ近づくため
  不採用(P2 review fold で ADR へ記録)。
- **`KNOWN_KEYS` 維持(O2c 不採用)** — `"trusted"` 登録を
  維持し、builtin/user 層の inert 受理(その層では元来意味を
  持たない documented 現状)を壊す gratuitous な破壊を避ける。
  拒否は層 gate のみで行い、typo(`trustd`)は従来どおり
  unknown key で route-load error。
- **`Route.trusted` field 削除** — 唯一の consumer(merge
  gate)が provenance へ置き換わったため field + doc comment
  ごと削除。`Route` の意味論から privilege flag が消え、
  `Route.layer` provenance が trust・診断・target.path 層起点
  解決の単一 source になる(表現 drift なし — `trusted` の
  値を別 field・別表現へ移さない)。`SuppressedRoute`・
  `InvalidRouteGroup`・`RouteLayer`・`EffectiveRoutes`・
  `RouteResolution`・`route resolve` 出力 shape・
  `kind:"route"` record `schemaVersion` は不変。
- **displaced 記録 arm 除去 + `suppressed` 狭義化** — 新無条件
  形では project route の `insert` は非衝突時にのみ走り戻り
  値は常に `None`。#167 が導入した「`trusted: true` 置換で
  displaced 下位層 route を `suppressed` 記録」arm は到達
  不能のため dead code として残さず arm ごと除去(unreachable
  arm を防御的に残すと「project が insert で置換し得る」
  誤読を構造に残す — 消えた経路は code からも消す)。
  `SuppressedRoute` semantics を「merge 敗北 route」から
  「衝突で負けた project route」へ狭義化 — shape・consumer
  (`route resolve` `suppressed` field・`kind:"route"`
  record)・builtin/user 層間 shadow の silent は不変。
- **builtin/user 層 `trusted:` 受理・inert 維持** — その層
  では元来意味を持たない documented 現状維持(O2a)。値は
  `Route` に保持せず `if let Some` + `is_boolean()` で形状
  のみ検査。zombie key の schema 除去は routes.yaml
  `version` bump 時へ Deferred(§Follow-ups)。
- **fail-closed schema × fail-open hook の責務分離(R6)** —
  presence 拒否は fail-closed(file 全体が load error で明示
  停止 — 作者が変更に気付ける)、hook 経路は fail-open
  (`load_or_none` `supply.rs:138-152` が presence 拒否を
  含むあらゆる load 失敗を `None` へ畳み全層 route arm 不
  活性・command 直接実行継続)— 既存規約不変のため pin
  test のみ追加(T2)。
- **grant `scope` seam 整合(R5 — code 不変の文書化契約)** —
  route trust(宣言層 provenance)と grant `scope`(authoring
  層 provenance — project task → `scope:"project"`、trusted
  層 grant → `scope:"trusted"` 既定・`scope:"project"` 明示
  は自己弱体化)は「trust は宣言主体が決める」単一モデルの
  2 面。route grant 参照(`grant` anchor・`grants[]` member・
  `substitute.grant`)が `scope=="trusted" && !fixed` のみ
  解決する `is_route_eligible`(`exec.rs:114-116`)単一
  seam・project partial schema の `command` 節除外
  (`load.rs:242-247`)は不変 — merge-base 無差分を実測確認
  (issue AC3)。

## Related Experiments

### Adopted

なし(known-solution task。design が field 削除・merge 差分・
層 gate verbatim rust・error 契約・churn 一覧・docs 更新契約を
file:line 精度で固定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

targeted cargo test・doc check(2026-10-24 実測、全 pass / 0
failed — T4 task §Verification 記録):

- `mise run check`: exit 0 — biome → `cargo fmt --check` →
  clippy `--workspace --all-targets`(dev-seam features)→
  version 1.0.0 → doc-ids(1179 files)→ `cargo test
  --workspace`(dev-seam)全 suite 0 failed(50 "test result:"
  行全て 0 failed)→ `node --test`(scripts 43 pass /
  benchmarks 12 pass)→ lockfile → apm-payload、全 stage pass
- `node scripts/check-doc-ids.mjs`: exit 0 — "doc id check
  OK: 1179 markdown files scanned"
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all --json`: exit 0 — findings 727 件全て warning・
  error 0 件・issue-168 関連 error 0 件(`related` 非相互
  link の warning のみ — spec chain 側管理の契約どおり)
- spec Acceptance Criteria AC1-AC5 ↔ TSPEC Rule 対応(T4 task
  §1 で確認済み — issue AC mapping):
  - issue AC1 → spec AC1: trust 発行主体 = 宣言層 provenance
    が spec R1/R2 + ADR O1a/O2a Decision Outcome に決定・
    記録(実装済み判定は merge code + 層 gate の構造確認)
  - issue AC2 → spec AC2: `project_layer_trusted_is_rejected`
    (`true`/`false`/非 bool 3 形 presence 拒否)+
    `resolve_project_trusted_rejected`(exit 3)+ untrusted
    衝突 `suppressed` pin — `trusted: true` 宣言 project
    route の shadow は宣言経路自体が load 時に閉じ、
    `trusted` 非宣言 route は衝突で `suppressed` 報告継続
  - issue AC3 → spec AC3: `is_route_eligible`
    (`exec.rs:114-116`)merge-base 無差分 + spec R5・
    `parent-agent-integration.md` grant 参照節の整合記述確認
  - spec AC4: compile(field 削除完結)+ `usage_log.rs`
    record shape pin(:298-353)無改訂 + hook fail-open
    回帰 test + 全 suite pass(`trusted` 前提 arm は新規約
    へ更新済み)
  - spec AC5: 破壊的変更が spec Scope/AC・design §Deferred・
    guides(症状 + 修復 = key 削除 1 行 + user 層 vendor
    経路)に明記 — #169 migration guide 材料として供給済み
- targeted test(T1/T2 実測): `route::routes` unit 17 pass
  (`project_layer_trusted_is_rejected` 新規・
  `user_shadows_builtin_and_project_never_shadows`・
  `merge_records_losing_project_route` rewrite)・`route::`
  integration 10 pass(`resolve_project_trusted_rejected`)
  ・`route_run::` integration 35 pass(untrusted arm)・
  `hook::` integration 78 pass(新規
  `project_trusted_violation_disables_route_arm_fail_open`
  含む)・`cargo test -p cheaper-cli` 全緑(lib 115・
  integration 301・observability 17・observability_dev_seam
  3・regression 1・run_dev_seam 39・workflow 18・
  workflow_dev_seam 5、全て 0 failed)・
  `cargo clippy -p cheaper-cli --all-targets` exit 0
- grep sanity(T4 実測): `grep -rn "\.trusted" crates/` は
  `{label}.trusted` error 文言・test assert 文言のみ —
  `route.trusted`/`.trusted` field 参照 consumer 0 件。
  `grep -rn "Route {" crates/` の literal 5 site 全件
  `trusted` field なし。`KNOWN_KEYS`(:393)に `"trusted"`
  維持。`suppressed` assert は全て負けた project route 名
  (`proj-push`/`project-shadow`/`p-route`/`proj-test`/
  `proj-shadow` — 「下位層 route 名を報告する」assert 残件
  なし)。`TODO`/`unimplemented!`/`todo!` 残存なし
- 端到端構造確認(T4 実測): 層 gate は `routes.rs:629` で
  `passthroughEnv` parse 後・bool 形状検査(:639)前に配置
  (presence-first — 非 bool 値でも `not a boolean` ではなく
  `not declarable` が先)。merge の project 分岐(:199-215)
  は `effective.contains_key` のみで分岐し displaced 記録
  arm なし。hook fail-open test は違反 file を project 層
  `<workspace>/.cheaper/routes.yaml` に配置(user 層 `f.cfg`
  ではない — user 層受理で違反にならない必須条件を遵守)。
  `route/supply.rs`・`route/run.rs`・`route_cmd.rs`・
  `usage_log.rs`・`policy/exec.rs`・`policy/load.rs`・
  `tools/*`・`hook/targets.rs`・`routed-exec.workflow.json`
  は merge-base(`fecc9b1`)から無差分 — `is_route_eligible`
  単一 seam 不変・`load_or_none` fail-open 規約不変
- review-protocol: design review — cooperative + adversarial
  両 reviewer が verdict ACCEPT WITH NON-BLOCKING NOTES。
  P2 fold を design/ADR へ反映済み(`f4509bb`): hook
  fail-open の blast radius 明確化(全層 route arm 不活性・
  debug log のみ可視)・O2e per-entry `invalid` 隔離案の
  不採用理由を ADR へ記録・fail-open 回帰 test の違反 file
  を project 層へ pin(user 層 fixture 誤用防止の必須条件)・
  zero-edit vendoring migration 経路の記述追加・stale
  anchor 修正(`resolve.rs:278-412`・`run.rs:366-370`)。

証跡 commit: planning chain `c479c62`(seed discovery)→
`404ddb1`(spec/ADR + discovery resolve)→ `ccd7bdd`(design)→
`f4509bb`(design approved 昇格 + review fold)→ `3deea00`
(design index status 同期)→ `d97da5a`(plan + test-spec +
tasks)→ `b3b82f9`(spec/ADR/discovery index 同期)→
`db3c08e`(discovery derived-by へ plan link)→ 実装
`d2daee4`(T1 field 削除 + 層 gate + merge 無条件形 + test
rewrite)→ `439ae8e`(T2 hook fail-open 回帰 test)→
`5ec9d0c`(T3 docs 追従)→ `cb154f4`(T4 検証記録 + plan/task/
spec/ADR status 更新 — task=done・plan=completed・
spec=approved・ADR=accepted)。本 IR はその後に追加する。

## Risks

- **EXISTING key への presence 拒否(破壊的変更)** —
  `trusted` は routes.yaml v1 から受理される EXISTING key
  であり、`grants`/`substitute`(NEW key — 拒否で壊れる既存
  file が無かった)には無かった互換破壊を伴う。`trusted`
  key を含む project routes.yaml は `trusted: false` の
  inert 宣言・file 内の非衝突 route を含め**全件 load
  失敗**する — strict 経路(`route resolve`/`route run`
  prepare)は `INVALID_RUNTIME_POLICY`(exit 3)、hook 経路
  は fail-open でその workspace の **builtin/user/project
  全層の routing** が不活性化する(route arm が落ちて
  command は直接実行 — 実行自体は継続するが症状は debug
  log のみで `route resolve` を実行するまで可視化されない)。
  緩和材料: 対象は `trusted` を書いた file のみ・#169
  migration guide で告知・修復は当該 key の削除 1 行・
  失敗方向は fail-closed(file が黙って意味を失うのでは
  なく load error で明示的に止まる)—「壊れるなら loud
  に壊れる」fail-loud 哲学と整合(ADR O2a 採用根拠)。
- **project 層 shadowing 用途の構造的消滅** — D7 が残した
  「project-tuned workflow で routed command を tuned する」
  正当用途は構造的に消えた。operator の代替は workflow を
  `<userConfigDir>/` へ vendor して user 層 route を宣言
  (`target.path` 層起点制約で user 層は project file を
  直接指せない)— **project routes.yaml を userConfigDir
  へ丸ごとコピーすれば `trusted: true` は user 層で inert
  受理されるため意図した shadowing が無改修で復活する**
  zero-edit migration 経路を guides へ記述済み。
- **builtin/user 層の inert `trusted` key 残留(zombie)** —
  受理・bool 検証・inert の documented 現状維持(その層では
  元来意味を持たない)。表現 drift は小さいが、schema
  正直性は routes.yaml `version` bump 時に回収する
  (§Follow-ups — O2c 相当の全層拒否には踏み込まない)。
- **fail-open 不活性の可視性** — project 層違反で全層
  route arm が落ちても command 処理は継続し、症状は debug
  log のみ。workspace の routing 全体が黙って無効化される
  側面は、症状と診断手順(`route resolve` の exit 3 /
  `INVALID_RUNTIME_POLICY`)を guides・#169 migration
  guide 材料へ明記して対処。
- **`SuppressedRoute` semantics の狭義化** — 「merge 敗北
  route」→「衝突で負けた project route」へ doc 追従済み。
  merge semantics に新たな置換経路が将来追加された場合に
  provenance の意味を再評価(§Deferred)。
- **test churn(処理済み)** — 「project `trusted` が効く」
  前提の test arm(merge test の trusted shadow arm・
  integration の displaced provenance 記録 test)は削除
  ではなく「presence 拒否検証 + 負け側 suppressed 回帰
  固定」へ置き換え済み。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #168)由来の未分類・未処理の follow-up はない。
下記は全て spec/design「Deferred Design Concerns」・各 task
「Out of Scope」で scope 外確定・記録済みの defer、issue
#169 へ移管済みの項目、または design で受理・記録済みの既知
semantics であり、新規 chain・新規 issue は起こさない。

- **moved-to-issue — migration guide 本文・文書正直性
  sweep** — issue #169(#151 分解 7/7)の scope。破壊的変更の
  告知材料は本 chain で供給済み: 症状 = project 層
  `trusted` presence で hook 経路 fail-open・全層 routing
  不活性・debug log のみで `route resolve` まで不可視、
  診断 = `route resolve`/`route run` strict 経路の
  `INVALID_RUNTIME_POLICY`(exit 3)、修復 = `trusted` key
  削除 1 行・project routes.yaml の `<userConfigDir>/`
  vendor 経路(zero-edit — user 層受理で shadowing 復活)。
  guides・design §Deferred・spec AC5 に固定済み —
  https://github.com/scarecrow173/cheaper/issues/169。
- **documented-deferred — builtin/user 層 inert `trusted`
  key の schema 完全削除** — 現行は受理・bool 検証・inert
  (documented 現状維持)。routes.yaml `version` bump の機会
  に schema から除去するのが自然(spec/design Deferred と
  同一 — O2c 相当の全層拒否は user/builtin 層の inert 宣言
  を壊す gratuitous 破壊として不採用)。
- **documented-deferred — operator/user 側 consent
  registry(O1b)** — trusted 層が project route の shadow を
  個別許可する宣言機構。需要未確認で YAGNI・D7 却下理由
  (第二 config surface・staleness semantics・split-brain)
  存続 — 需要が確立した時点で trusted 層宣言機構として別
  schema で追加可能(本決定は project 層 `trusted` key を
  閉じるのみで forward-compat を損なわない — discovery
  Open Questions・spec/design Deferred と同一。consent
  key・staleness semantics の設計未着手)。
- **documented-deferred — 既存 test-spec 内の stale
  `trusted` 記述の bookkeeping sweep** —
  `route-resolution.md`・`substitute-exec-route-mode.md`・
  `route-grant-set-projection.md` 内の `trusted: true`
  shadow・displaced 記録の window 記述は本 chain の TSPEC
  が新規約の owner であり、旧 test-spec の stale 記述 sweep
  は test-spec chain 側管理・#169 正直性 sweep で回収
  (design §Docs 更新契約・T3 Out of Scope で scope 外確定)。
- **documented-deferred — `SuppressedRoute` semantics の
  再評価** — displaced 記録 arm 除去で「衝突で負けた
  project route」へ狭義化済み。merge semantics に新たな
  置換経路が将来追加された場合に provenance の意味を再評価
  (spec/design Deferred と同一 — 現時点では報告経路不変)。
- **documented-deferred — Option E(exec-less route)の採否**
  — 親 chain(#151 分解)の open 事項として引き続き残置
  (#167 IR と同一 — 本 issue の scope 外・
  `validate_routed_convention` 不変)。
- **closed — `trusted: true` project route の下位層
  shadow** — 宣言経路自体が load 時に閉じる最強形で本
  issue の in-scope として実装済み(T1 層 gate —「宣言して
  も判定を変えられない」ではなく「申告を受理しない」 —
  issue AC2)。
- **closed — merge displaced 記録 arm の到達不能化** —
  project shadow 廃止で `insert` 戻り値が常に `None` となり
  #167 の displaced 記録 arm は到達不能 — dead code と
  して残さず arm ごと除去済み(T1)。`suppressed` は「衝突で
  負けた project route」のみを運ぶ。

---
id: "DESIGN-034L4tqYwtc8q88WEIXPh9"
type: "design"
status: "draft"
title: "trusted route flag issuer redesign — `Route.trusted` 削除・merge 無条件化・project 層 `trusted` presence 拒否"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
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
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/designs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
    - "./permission-model-responsibility-redesign.md"
    - "./pretooluse-route-resolver-and-delegated-workflow-execution.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/trusted-route-flag-issuer-redesign.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "../discovery/trusted-route-flag-issuer-redesign-issue-168.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/trusted-route-flag-issuer-redesign.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "../discovery/trusted-route-flag-issuer-redesign-issue-168.md"
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../designs/route-grant-set-projection.md"
    - "../specs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../designs/substitute-exec-route-mode.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../specs/unified-exec-grant-model.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "../test-specs/route-resolution.md"
    - "../test-specs/route-grant-set-projection.md"
    - "../test-specs/substitute-exec-route-mode.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/configuration.md"
    - "../guides/command.md"
    - "../../crates/cheaper-cli/src/route/routes.rs"
    - "../../crates/cheaper-cli/src/route/resolve.rs"
    - "../../crates/cheaper-cli/src/route/run.rs"
    - "../../crates/cheaper-cli/src/route/supply.rs"
    - "../../crates/cheaper-cli/src/route_cmd.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/usage_log.rs"
    - "../../crates/cheaper-core/src/policy/exec.rs"
    - "../../crates/cheaper-core/src/policy/load.rs"
    - "../../crates/cheaper-cli/resources/routes.yaml"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign — `Route.trusted` 削除・merge 無条件化・project 層 `trusted` presence 拒否

## Context

issue #168(#151 権限モデル再設計の実装分解 6/7)は、route の trust
判定を「宣言値(route entry の `trusted: bool` field)」から
「**宣言層 provenance**(`layer ∈ {Builtin, User}` = trusted、
`layer == Project` = untrusted)」へ移す再設計。`trusted` flag
の発行主体を project 自己申告から宣言層そのものへ移し、
`grants`(#166 O7a)・`substitute`(#167 O3a)・grant `scope`
(#165 M3)・`route` policy 節・`command` 節除外と同じ「trust は
宣言内容ではなく宣言主体が決める」単一モデルへ揃える。

spec(SPEC-034LwdoInh3ie1KPlSjkJl・proposed — R1-R6 + AC)と
ADR(ADR-034LT2o68CgcOinMEs8PZ8・proposed — O1a + O2a 採用)で
方針は確定済みであり、discovery(DISC-034L44O8jYRDhvhM16gDiM・
resolved)が全接点を実測固定している: `route.trusted` を読む
consumer は全 crate で merge の shadowing gate
(`routes.rs:201`)のみ — viability(`resolve.rs:278-412`)・
grant 解決(`is_route_eligible`)・record・`route run` は
`route.trusted` を読まず、`route.layer` は target.path 層起点
解決(`run.rs:366-370`)と診断の provenance として残る。
**trust が分岐するのは merge precedence の一点だけ**であり、
flag 意味論を層 provenance へ畳む変更の blast radius は
`routes.rs` の merge + schema gate + field 削除 churn に閉じる。

確定済み方針(ADR Decision Outcome — encode のみ、再審議しない):

- **O1a** — trust = 宣言層 provenance。`layer ∈ {Builtin,
  User}` の route は同一 `match.command` で下位層を無条件
  shadow(現行不変)、`layer == Project` の route は `trusted`
  宣言の有無・値に関わらず `match.command` 衝突時に常に負けて
  `suppressed` provenance へ記録される。merge の project 分岐
  は `route.trusted` を一切読まない無条件形。
- **O2a** — project 層 route entry の `trusted` key presence
  (`true`/`false`/非 bool — presence-first で値を見ない)を
  `"{label}.trusted: not declarable in the project layer"`
  として `schema_error` → `InvalidRuntimePolicy` へ倒す。
  `grants`/`substitute` と同じ gate 位置・error class・
  presence-first 規約。builtin/user 層の `trusted` は受理・
  bool 検証・inert を維持し、`KNOWN_KEYS` 登録も維持する
  (reject は層 gate で行い、未知 key 拒否とは別規約)。
- **`Route.trusted` field 削除** — 唯一の consumer は merge
  gate であり provenance へ置き換わる。`SuppressedRoute`・
  `InvalidRouteGroup`・`RouteLayer`・`EffectiveRoutes`・
  `route resolve` 出力 shape・`kind:"route"` record shape は
  不変。displaced 下位層 route は二度と発生しないため
  `suppressed` は「衝突で負けた project route」のみを運ぶ
  (displaced 記録 arm は到達不能 — 除去)。
- **破壊的変更** — `trusted` key を含む project routes.yaml は
  全件 load 失敗(#169 migration guide 向け材料 — 修復は当該
  key の削除 1 行)。
- **整合** — route trust(宣言層 provenance)と grant `scope`
  (authoring 層 provenance — project task → `scope:"project"`、
  trusted 層 grant → `scope:"trusted"` 既定・`scope:"project"`
  明示は自己弱体化)は「trust は宣言主体が決める」単一モデルの
  2 面であり、route grant 参照の `is_route_eligible`
  (`scope=="trusted" && !fixed`)単一 seam は不変。

本 design の責務は、`Route` struct 変更・`KNOWN_KEYS` 処遇・
層 gate 配置と error 契約・merge 差分・`trusted` consumer/
literal の全 churn・docs 追従・test 計画(spec AC への写像)を
実装契約として固定すること。引用する file:line は全て branch
`feature/issue-168-trusted-route-flag` の現行 code で実測済み。

## Scope

### In Scope

- `Route.trusted` field の削除(`routes.rs:85-87` — doc comment
  ごと)と、全 `Route` literal/test constructor からの field
  除去(`routes.rs:643`・`routes.rs:1350`・`resolve.rs:434`・
  `routing_policy.rs:2090`)。
- merge(`route::routes::load`)の project 分岐を「衝突 →
  `suppressed` provenance 記録・非衝突 → 有効化」の無条件形へ
  変更(`routes.rs:198-220`)— `route.trusted` 読取り(:201)と
  displaced 下位層 route の `suppressed` 記録 arm(:210-218、
  到達不能化)の除去。
- `validate_route` への project 層 `trusted` key presence 拒否
  gate 追加 — `grants`/`substitute` と同じ判別・gate 位置・
  error class・presence-first 規約(§(a))。
- builtin/user 層の `trusted` key 受理・bool 検証・inert の維持
  — parse は値を保持しない形状検査のみへ縮小(`routes.rs:627-633`)。
  `KNOWN_KEYS` の `"trusted"` 登録(:397)は維持。
- `SuppressedRoute` doc comment(:121-124)・file header merge
  規約 comment(:12-16)・`RouteResolution.suppressed` field doc
  (`resolve.rs:101-104`)・`grants`/`substitute` gate の
  `trusted` 言及 comment(:476-480・:518-519)の semantics 追従。
- 関連 test の更新 — `routes.rs` unit(merge/layer-gate test)、
  integration `route.rs`・`route_run.rs` の `trusted` 前提 arm、
  test constructor の `trusted` field 除去。
- docs 追従 — `parent-agent-integration.md` の route 宣言層説明・
  `configuration.md`/`command.md` の層 gate 列挙・
  `resources/routes.yaml` shape 記述・
  `specs/pretooluse-route-resolver-and-workflow-delegation.md`
  R1 field 一覧・R7 層間 trust の追従記述。
- route trust と grant `scope` の整合記述(spec R5 — §(d))。

### Out of Scope

- **operator/user 側 consent registry**(O1b — trusted 層が
  project route の shadow を個別許可する宣言機構)— 需要未確認
  のため YAGNI。本決定は project 層 `trusted` key を閉じるだけ
  で、trusted 層側の新規宣言機構を別 schema として後付けする
  forward-compat は損なわない(§Deferred)。
- **builtin/user 層の inert `trusted` key の schema からの完全
  除去** — routes.yaml `version` bump が必要な schema 破壊で
  あり本 design では行わない(受理・inert を維持 — §Deferred)。
- **project route の新規 prefix 宣言権・`grant:` anchor 宣言権・
  発火の変更** — project 層 route は従来どおり非衝突 prefix で
  宣言・発火する(不変)。
- **grant `scope` モデル・`is_route_eligible`・`project_grants`・
  viability・`expected_input`・exit mirror の変更** — route
  grant 参照の eligibility は #165/#166 確定のまま(routing ≠
  authorization — R6)。
- **`suppressed`/`invalid`/`route resolve` 出力 shape・
  `kind:"route"` record shape・`schemaVersion` の変更**。
- **Option E(exec-less route)** — 引き続き親 chain の open 事項。
- **文書正直性 sweep・migration guide 本文** — issue 7(#169)
  が担う。本 design は破壊的変更の告知材料を供給するのみ。

## Components and Boundaries

### (a) route declaration — `crates/cheaper-cli/src/route/routes.rs`

`Route` struct(:58-89)から `trusted` field を削除し、project
層 `trusted` presence 拒否・builtin/user inert 受理の schema
責務をこの file に閉じ込める。

**(a-1) `Route.trusted` field 削除(:85-87)** — doc comment
(`/// `trusted` — project 層宣言のみ意味を持つ(builtin/user は
inert)。`)ごと除去する。`Route` の意味論から privilege flag
が消え、`layer` field(:88)が trust・診断・target.path 層起点
解決を担う単一 provenance source になる — `layer` の doc を
追記して単一 source であることを明文化する:

```rust
/// `passthroughEnv` — hook env から capture を許す env 名。
pub passthrough_env: Vec<String>,
/// 宣言層 provenance — trust 判定(`layer ∈ {Builtin, User}`
/// が trusted)#168・target.path 層起点解決・診断の単一 source。
/// `Route` は trust flag を持たない — trust は宣言値ではなく
/// 宣言主体(層)が決める(O1a)。
pub layer: RouteLayer,
```

`Route` literal(`validate_route` の :634-645)から `trusted,`
(:643)を除去する。`PartialEq`/`Clone`/`Debug` derive は不変 —
field 削除のみ。

**(a-2) `KNOWN_KEYS`(:388-398)は不変** — `"trusted"`(:397)を
登録したまま維持する。`KNOWN_KEYS` は全層共通のため `"trusted"`
は全層で recognized となり、project 層の拒否は unknown-key
(:399-403)ではなく明示の層 gate で行う(`grants`/`substitute`
と同じ構造 — O2c 不採用: builtin/user 層の inert 宣言まで一律に
壊す gratuitous な破壊を避ける)。`trusted` の typo
(`trustd` 等)は従来どおり unknown key として route-load error。

**(a-3) project 層 `trusted` presence 拒否 gate(O2a)** —
gate の配置は `grants` gate(:481-486)・`substitute` gate
(:520-525)と同じ「該当 key の値形状検査直前」規約に従い、
`passthroughEnv` parse(:612-626)と `trusted` bool 検査
(:627-633)の間 — すなわち :627 直前に置く。層 gate 群の発火
順は schema 記述順(grants → substitute → trusted)で、複数
gated key を持つ entry は先に来る gate の violation を報告する
(従来どおり逐次 fail-loud)。`layer` param は :383 で既に判別
可能(新規 param 不要):

```rust
// `trusted` は trust 申告そのもの — project 層は申告を置く
// こと自体ができない(O2a)。`grants`/`substitute` と同じ
// 判別子(宣言層 provenance)・同じ presence-first 規約で、
// 値の形状・値(`true`/`false`/非 bool)に関わらず拒否する。
// trust は `route.layer` が決め、project 自己申告の経路は
// load 時に閉じる。
if layer == RouteLayer::Project && entry.contains_key("trusted") {
    return Err(schema_error(
        file,
        &format!("{label}.trusted: not declarable in the project layer"),
    ));
}
// `trusted` — builtin/user 層のみ受理される inert bool(その層
// では元来意味を持たない documented 現状維持 — trust は宣言層
// provenance が決め、`Route` は flag を持たない)。値は保持せず
// bool 形状のみ検証する。
if let Some(v) = entry.get("trusted") {
    if !v.is_bool() {
        return Err(schema_error(
            file,
            &format!("{label}.trusted: not a boolean"),
        ));
    }
}
```

- **presence-first** — `trusted: false` も非 bool 値も拒否する
  (untrusted 層の宣言は内容を評価しない — `grants`/`substitute`
  と同じ fail-closed)。「`trusted` を書いた file」は shadowing
  を意図した file であり、黙って意味を失うのではなく load error
  で明示的に止まる(ADR O2a の採用根拠)。
- **builtin/user 受理・inert** — その層では元来意味を持たない
  documented 現状維持(O2a)。bool 非適合値は従来どおり
  `{label}.trusted: not a boolean` で `schema_error`(受理層の
  型検査規約不変)。値は `Route` に保持しない — parse は形状
  検査のみへ縮小し、`if let Some` 形で「検査するが使わない」
  を code 上で自明にする(`let _ = ` や discard binding は
  使わない)。
- **`grants`/`substitute` gate comment 追従(:476-480・
  :518-519)** — 「`trusted:true` は project 自己申告の
  shadowing gate であり判別に使えない — 判別子は宣言層
  provenance」の言及は flag 削除で stale になる。「project
  層の `trusted` key 自体が本 file 後段の層 gate で拒否される
  — 判別子は宣言層 provenance」旨へ追従する(判別子の記述
  自体は不変)。

### (b) merge semantics — `routes.rs::load`(:155-232)

merge loop(:182-222)の project 分岐(:198-220)を無条件形へ
書き換える。`route.trusted` 読取り(:201)を除去し、衝突は
常に project 側が負ける:

```rust
// project 層 route は下位層を shadow しない — trust は宣言値
// ではなく宣言層 provenance が決める(O1a)。`match.command`
// 衝突は常に project 側が負けて `suppressed` provenance へ
// 記録される(静かに無視しない — R6)。project 層の `trusted`
// key は load 時に層 gate で拒否済みのため、merge が flag を
// 読む経路は存在しない。
RouteLayer::Project => {
    if effective.contains_key(&route.command) {
        suppressed.push(SuppressedRoute {
            name: route.name,
            command: route.command,
        });
    } else {
        order.push(route.command.clone());
        effective.insert(route.command.clone(), route);
    }
}
```

- **builtin/user 分岐(:190-197)は文面不変** — user は
  builtin を無条件 shadow する現行規約(同一 trusted
  principal の宣言 override — silent のまま)。
- **displaced 記録 arm(:210-218)の除去** — 新無条件形では
  project route の `insert` は非衝突時にのみ走り、`insert` の
  戻り値は常に `None`。#167 が導入した「`trusted: true` 置換
  で displaced 下位層 route を `suppressed` 記録」arm は到達
  不能になり、dead code として残さず arm ごと除去する
  (unreachable arm を防御的に残すと「project が insert で
  置換し得る」誤読を構造に残す — 消えた経路は code からも
  消す)。
- **`SuppressedRoute` semantics の狭義化(:121-124)** —
  「trust 不在で負けた project route + displaced 下位層 route」
  から「**衝突で負けた project route**」へ doc comment を
  追従。shape `{name, command}`(:126-129)は不変:

```rust
/// merge で負けた route の provenance — 同一 `match.command`
/// で下位層(builtin/user)route に負けた project route
/// (`route resolve`・`kind:"route"` record が報告)。project
/// 層は下位層を shadow しないため displaced 下位層 route は
/// 発生しない(#168 — #167 の displaced 記録 arm は到達不能で
/// 除去)。
```

- **file header merge 規約 comment(:12-16)追従** —
  「project は `trusted: true` のときのみ下位層を shadow する」
  を「project は下位層を shadow しない(trust = 宣言層
  provenance)— `match.command` が衝突した project route は
  `suppressed` provenance として記録される」へ追従。
- **`RouteLayer` doc(:30)は文面ほぼ不変** — 「route を宣言
  した層(provenance — trust 評価と診断報告に使う)」は
  provenance が trust の単一 source になる変更と整合する
  (「使う」→「の単一 source」程度の微修正のみ)。

### (c) `suppressed` 報告経路 — consumer 不変

負けた project route の provenance 報告は shape・経路とも不変
(R6 — 「負けた側を記録する」観測義務の維持):

- `resolve.rs` の `suppressed()` lookup(:221-228 —
  `.find(|s| is_prefix(&s.command, command))`)は不変。
  losing project route は自身の `match.command` で記録される
  ため best-match route(:266)・no-match(:246)の両経路で
  既存どおり route 名を拾う。
- `RouteResolution.suppressed`(:101-104)の field doc 追従 —
  「trust 不在で負けた project route + displaced 下位層 route」
  →「同一 `match.command` で下位層に負けた project route
  (displaced 記録は #168 で project shadow 廃止とともに除去)」。
- `route resolve` 出力 `suppressed` field
  (`route_cmd.rs:126-127`)・`kind:"route"` record の
  `suppressed`(`run.rs:633` → `usage_log.rs:359,418-419`)は
  wire 不変 — Option 型のままで新規 field・shape 変更なし
  (`usage_log.rs` record shape pin test `usage_log.rs:298-353`
  も無改訂で pass する契約)。
- displaced route が `suppressed` に載る経路は消えるため、
  「`suppressed` が下位層 route 名を報告する」観測は二度と
  発生しない — test 側の assert も追従する(§Churn・
  §Verification Notes)。

### (d) grant `scope` seam との整合(spec R5 — 文書化契約)

route trust(宣言層 provenance)と grant `scope`(authoring 層
provenance)は同一層境界を共有する「trust は宣言値ではなく宣言
主体が決める」単一モデルの 2 面 — 本 design では code を
一切変更せず、整合を契約として固定する:

- `ExecGrantScope::Trusted|Project`(`exec.rs:38-45`)—
  project 層宣言 task は `load.rs:239-251` で merge 前に task
  名集合を捕捉し `exec.rs:513-517` で `scope:"project"` へ
  畳み込む。trusted 層宣言 grant は既定 `scope:"trusted"` で
  `scope:"project"` 明示は自己弱体化(`exec.rs:612-614`)。
- route の grant 参照(`grant` anchor・`grants[]` member・
  `substitute.grant`)は `is_route_eligible`
  (`exec.rs:114-116` — `scope=="trusted" && !fixed`)の単一
  seam で判定 — **predicate・comment ともに変更不要**。
- project partial schema の `command` 節除外
  (`load.rs:242-247` — grant authoring は trusted 層のみ)も
  不変 — route 側の「誰が宣言したか」と grant 側の「誰が
  authoring したか」が同じ層境界を共有する記述を
  guides(§Docs 更新契約)へ反映する。

### (e) signature 契約(変更後)

| function/struct | 変更後 signature | 備考 |
|-----------------|------------------|------|
| `Route` | `pub trusted: bool` を削除(:85-87) | field + doc comment ごと。`layer`(:88)に trust provenance の単一 source 旨を追記 |
| `KNOWN_KEYS` | 不変 — `"trusted"`(:397)維持 | unknown-key ではなく層 gate で拒否(O2c 不採用) |
| `validate_route` | signature 不変(`layer: RouteLayer` は :383 で既存) | project 層 `trusted` presence 拒否 gate(:627 直前)+ bool 形状のみ検査へ縮小(:627-633) |
| `load` merge | project 分岐を無条件形へ(:198-220) | `route.trusted` 読取り(:201)・displaced 記録 arm(:210-218)除去 |
| `SuppressedRoute` | signature・`{name, command}` shape 不変 — semantics を「衝突で負けた project route」へ狭義化 | doc comment(:121-124)・file header(:12-16)追従 |
| `RouteResolution.suppressed` | `Option<String>` 不変 — doc 追従(:101-104) | displaced 記述を除去 |

**変更しない signature**: `resolve::resolve`/`ResolveInput`/
`ExecPolicyView`/`Viability`/`RouteResolution` field・
`Route::grant_set`/`Route::exec_pair`/`RouteSubstitute`・
`is_route_eligible`/`project_grants`/`check_exec_execution`/
`pin_exec_grant(s)`・`RouteRecordInput`/`build_route_record`/
`record_route`/`append_usage_log`・`validate_routed_convention`・
`load_or_none`/`RouteSupply` — 全て不変。

### (f) call site 配線(全一覧)

| site | lines(現行) | 変更内容 |
|------|------------|----------|
| `route/routes.rs` | :12-16 | file header merge 規約 comment 追従(project shadow 不可・衝突 project は suppressed) |
| `route/routes.rs` | :85-87 | `Route.trusted` field + doc comment 削除 |
| `route/routes.rs` | :88 | `Route.layer` に trust provenance 単一 source 旨の doc 追記 |
| `route/routes.rs` | :121-124 | `SuppressedRoute` doc — 「衝突で負けた project route」へ狭義化 |
| `route/routes.rs` | :198-220 | merge project 分岐を無条件形へ — `route.trusted`(:201)・displaced arm(:210-218)除去 |
| `route/routes.rs` | :476-480,:518-519 | `grants`/`substitute` gate comment の `trusted` 言及追従(判別子=宣言層 provenance は不変) |
| `route/routes.rs` | :627 直前 | project 層 `trusted` presence 拒否 gate 追加(O2a — `grants`/`substitute` と同型・同文言) |
| `route/routes.rs` | :627-633 | `trusted` parse を bool 形状検査のみへ縮小(値は保持しない — builtin/user 受理・inert 維持) |
| `route/routes.rs` | :643 | `Route` literal から `trusted,` 除去 |
| `route/routes.rs` | :800-857 | `user_shadows_builtin_and_project_needs_trusted` — trusted shadow arm(:822-843)を撤去し rename。untrusted suppressed・非衝突有効化 arm は維持 |
| `route/routes.rs` | :1025-1033 | `project_layer_grants_are_rejected` の `trusted:true` 併記 arm — comment 追従(trusted presence 自体も拒否 — gate 順は grants が先) |
| `route/routes.rs` | :1252-1260 | `project_layer_substitute_is_rejected` 同上 |
| `route/routes.rs` | :1350 | `exec_pair_selects_execution_pair` の `trusted: false` literal 除去 |
| `route/routes.rs` | :1366-1405 | `merge_records_displaced_lower_layer` — #167 displaced 記録 semantics が消えたため rewrite(rename + project-lose 記録のみ assert) |
| `route/routes.rs` | test 新規 | `project_layer_trusted_is_rejected` — `true`/`false`/非 bool の presence 拒否 + builtin/user 受理・非 bool 型検査(§Verification Notes) |
| `route/resolve.rs` | :101-104 | `RouteResolution.suppressed` field doc 追従(displaced 記述除去) |
| `route/resolve.rs` | :434 | test `route()` helper literal から `trusted: false` 除去 |
| `hook/routing_policy.rs` | :2090 | test `route()` helper literal から `trusted: false` 除去 |
| `tests/integration/route.rs` | :285-336 | `resolve_project_trusted_displacement_reports_suppressed` — rewrite(displaced 経路は消滅): `trusted` presence → `INVALID_RUNTIME_POLICY`(`resolve_project_trusted_rejected`・:266-283 と同型)+ untrusted 衝突 → `suppressed` 報告 |
| `tests/integration/route_run.rs` | :1386-1455 | `suppressed_project_provenance_reaches_route_record` — `for trusted` loop 撤去、untrusted arm(resolve + record の suppressed=`project-shadow`)のみ残す |
| `resources/routes.yaml` | :7-8 近辺 | `trusted` key の shape comment(builtin/user 受理・inert・project 層宣言は load error)を 1 行追加 |
| `route/run.rs` | :366-370,:633 | **不変** — `route.layer` の target.path 層起点解決・record `suppressed` 配線は不変 |
| `route/supply.rs` | :136-152 | **不変** — `load_or_none` fail-open 規約に presence 拒否も乗る(新規 code なし) |
| `route_cmd.rs` | :105-134 | **不変** — R11 出力 shape・`suppressed` field 不変 |
| `usage_log.rs` | :359,:418-419 | **不変** — `RouteRecordInput.suppressed`・record 挿入不変 |
| `policy/exec.rs` | :38-45,:114-116 | **不変** — `ExecGrantScope`・`is_route_eligible` 単一 seam 不変 |
| `policy/load.rs` | :239-251 | **不変** — project task 名捕捉・`command` 節除外(:242-247)不変 |
| `tests/integration/usage_log.rs` | :298-353 | **不変** — `route_record_shape` は `suppressed` Option wire を既に pin |

### (g) error 契約

| 条件 | code / 分類 | 境界 | 備考 |
|------|------------|------|------|
| project 層 route entry が `trusted` key を含む(`true`/`false`/非 bool — presence-first) | `InvalidRuntimePolicy` — `schema_error(file,"{label}.trusted: not declarable in the project layer")` | `routes.rs::validate_route`(:627 直前 — :383 の `layer` param) | O2a — fail-loud route-load error。`details.{path,reason:"schema",violation}` で file と entry を名指し。値形状検査より先に presence で拒否 — `grants`(:481-486)/`substitute`(:520-525) と同型 |
| builtin/user 層の `trusted` が非 bool | 同上(`{label}.trusted: not a boolean`) | `routes.rs::validate_route`(gate 直後) | 受理層の型検査規約不変 — 値は保持しない |
| `trusted` typo(`trustd` 等) | 同上(unknown key) | `routes.rs:399-403` | `KNOWN_KEYS` 維持で fail-loud 規約不変 |
| `trusted` を含む project routes.yaml 全体 | `InvalidRuntimePolicy` — file 全体 load 失敗 | `routes.rs::load` → `supply::load` | schema_error は file 単位 — file 内の非衝突 route も不活性化(破壊的変更 — §Risks) |
| hook 経路の `trusted` presence 拒否 | `None` — route arm 不活性(fail-open) | `route/supply.rs:138-152`(不変) | `load_or_none` があらゆる load 失敗を fold する現行規約 — その workspace の project routing 全体が不活性化し command は直接実行継続 |
| `trusted` 非宣言の project route が `match.command` 衝突 | error ではない — `suppressed` provenance 記録 | `routes.rs::load` project 分岐 → `resolve.rs:221-228` | `route resolve` 出力(:126-127)・`kind:"route"` record(`run.rs:633` → `usage_log.rs:418-419`)が報告。下位層 route が勝ち・発火は不変 |
| `trusted` 宣言 project route が下位層を shadow | **発生しない** — 宣言経路が load 時に閉じる | `routes.rs::validate_route` | 「宣言しても判定を変えられない」ではなく「申告を受理しない」(AC2 を最強形で満たす) |

## Data and Control Flow

```
routes.yaml layers          declaration                    merge / resolve / record
───────────────────────────────────────────────────────────────────────────────────
builtin/user/project ─→ routes::load(層別 file — :161-177)
  └─ validate_route(layer 判別済み — :383):
       KNOWN_KEYS(不変 — "trusted" 登録維持 :397)
       → unknown-key 拒否(:399-403)
       → name/match/target/grant 解読(:404-475)
       → grants 層 gate+parse(:476-509)
       → substitute 層 gate+parse(:510-582)
       → output(:583-611) → passthroughEnv(:612-626)
       → trusted 層 gate(新設 :627 直前 — project presence 拒否)
       → trusted bool 形状検査(builtin/user 受理・inert — 値は保持しない)
       → Route{…, layer}  (trusted field なし)
       ※ 形状不正・project 層の trusted/grants/substitute presence は
         route-load error(fail-loud — InvalidRuntimePolicy)
  └─ load merge(:182-222):
       builtin/user 分岐(:190-197)— 無条件 insert(不変)
       project 分岐(:198-220)— 無条件形:
         衝突 → 自身を suppressed provenance へ記録
         非衝突 → order.push + insert(displaced arm なし)
       ※ route.trusted を一切読まない — trust = route.layer

hook eval / route resolve / route run prepare
  └─ supply::load → ExecPolicyView → resolve::resolve
       ├─ suppressed lookup(:221-228)— 負けた project route 名
       │    → RouteResolution.suppressed(:266 / :246)
       │    → `route resolve` 出力 suppressed(:126-127)
       │    → `kind:"route"` record(run.rs:633 → usage_log:418-419)
       ├─ viability(:278-412)— route.trusted/route.layer を
       │    trust 判別として読まない(不変 — D2)
       └─ grant 参照 → is_route_eligible(scope=="trusted" && !fixed
            の単一 seam — exec.rs:114-116 不変)

route run execute
  └─ route.layer → target.path 層起点 file_root(run.rs:366-370 不変)
     → record suppressed(provenance echo — 不変)

hook 経路(supply.rs:138-152)
  └─ trusted presence 拒否を含む load 失敗 → load_or_none
     → None(route arm 不活性 — fail-open・command 直接実行継続)
```

## Non-Functional Constraints

- **routing ≠ authorization(D2/R6)** — viability・grant
  eligibility・`expected_input`・record 経路は
  `route.trusted`/`route.layer` を trust 判別として読まない。
  `route.layer` は target.path 層起点解決(`run.rs:366-370`)と
  診断の provenance として残る。trust が影響するのは merge
  precedence のみ。
- **hook 経路の fail-open(R6)** — `load_or_none`
  (`supply.rs:138-152`)は `trusted` presence 拒否を含むあらゆる
  load 失敗を `None`(route arm 不活性)へ畳む現行規約不変。
  fail-closed な schema 強制と fail-open な hook 合成の既存
  責務分離を維持する。
- **`suppressed` provenance の報告義務(R6)** — 衝突で負けた
  project route は `route resolve` `suppressed` field と
  `kind:"route"` record が報告する — 静かに無視しない。
  shape・reporting 経路は不変で、運ぶのは「負けた project
  route」のみ(displaced 記録は到達不能で除去)。
- **fail-loud load(R6)** — 未知 key・required 欠落・schema
  違反・層 gate 違反は file 全体を config error にする現行規約
  不変。project 層 `trusted` presence は他の層 gate と同じ
  fail 経路(`schema_error` → `InvalidRuntimePolicy`)を辿る。
- **presence-first discipline** — untrusted 層の宣言は内容を
  評価しない。`trusted: false` も非 bool 値も拒否し、「`false`
  か absent のみ許容」という incoherent な意味論は作らない
  (O2d 不採用)。
- **判別子の単一化** — trust 判別子は宣言層 provenance 1 つ。
  `route.trusted` 宣言値を判別子として読む経路の復活・merge
  外での trust 分岐の新設は禁止(patterns to avoid — ADR)。
- **`KNOWN_KEYS` は全層共通** — `"trusted"` 登録は維持し、
  builtin/user 層の inert 受理を壊さない。拒否は層 gate で行い
  未知 key 拒否とは別規約とする(O2c 不採用)。
- **表現 drift なし** — `Route` の意味論から privilege flag
  が消え、`Route.layer` provenance が trust・診断・target.path
  層起点解決の単一 source になる。`trusted` の値を別 field・
  別表現へ移さない(semantic input として消滅)。

## Risks and Trade-offs

- **EXISTING key への presence 拒否(破壊的変更)** —
  `trusted` は routes.yaml v1 から受理される EXISTING key で
  あり、`grants`/`substitute`(NEW key — 拒否で壊れる既存
  file が無かった)には無かった互換破壊を伴う。`trusted` key
  を含む project routes.yaml は `trusted: false` の inert 宣言・
  file 内の非衝突 route を含め**全件 load 失敗**する —
  strict 経路(`route resolve`/`route run` prepare)は
  `INVALID_RUNTIME_POLICY`(exit 3)、hook 経路は fail-open
  でその workspace の project routing 全体が不活性化する
  (route arm が落ちて command は直接実行 — 実行自体は継続)。
  **緩和材料**: (a) 対象は `trusted` を書いた file のみ =
  まさに除去対象機能の利用者 (b) #169 migration guide で告知
  でき修復は当該 key の削除 1 行 (c) 失敗方向は fail-closed —
  file が黙って意味を失うのではなく load error で明示的に
  止まり作者が変更に気付ける (d) ignore(O2b)でも
  `trusted: true` の file は shadow を失う silent semantic
  変化になる以上、「壊れるなら loud に壊れる」方が本 repo の
  fail-loud 哲学と整合する。
- **project 層 shadowing 用途の構造的消滅** — D7 が残した
  「project-tuned workflow で routed command を tuned する」
  正当用途は構造的に消える。operator が同等を実現するには
  workflow を `<userConfigDir>/` へ vendor して user 層 route
  を宣言する必要がある(`target.path` の層起点制約で user 層は
  project file を直接指せない — §Deferred の consent registry
  余地は残る)。
- **builtin/user 層の inert `trusted` key 残留** — 受理・bool
  検証・inert の zombie key が残る(documented 現状維持 —
  その層では元来意味を持たない)。表現 drift は小さいが
  schema 正直性は routes.yaml `version` bump 時に回収する
  (§Deferred)。
- **test churn** — 「project `trusted` が効く」前提の test arm
  (merge test の trusted shadow arm・integration の displaced
  provenance 記録 test)は新規約への書き換えが要る — 削除
  ではなく「presence 拒否検証 + 負け側 suppressed 回帰固定」
  へ置き換える(§Churn・§Verification Notes)。
- **`SuppressedRoute` semantics の狭義化** — displaced 記録
  arm が到達不能になり「merge 敗北 route」→「衝突で負けた
  project route」へ doc 追従。merge semantics に新たな置換
  経路が将来追加された場合に provenance の意味を再評価する
  (§Deferred)。

### Churn 一覧(実測 — signature/field 追随)

- **route schema/merge** — `route/routes.rs`:`Route.trusted`
  field 削除(:85-87)+ `Route.layer` doc 追記(:88)、
  `SuppressedRoute` doc 狭義化(:121-124)、file header merge
  規約 comment(:12-16)、merge project 分岐の無条件化
  (:198-220 — `route.trusted` :201・displaced arm :210-218
  除去)、`trusted` 層 gate 新設(:627 直前)+ parse 縮小
  (:627-633)、`Route` literal から `trusted,`(:643)、
  `grants`/`substitute` gate comment の `trusted` 言及追従
  (:476-480・:518-519)。unit test —
  `user_shadows_builtin_and_project_needs_trusted`(:800-857)
  rename + trusted arm 撤去、`project_layer_grants_are_rejected`
  (:1025-1033)・`project_layer_substitute_is_rejected`
  (:1252-1260)の `trusted:true` 併記 arm comment 追従、
  `exec_pair_selects_execution_pair` の `trusted: false`
  (:1350)除去、`merge_records_displaced_lower_layer`
  (:1366-1405)rewrite、新規 `project_layer_trusted_is_rejected`。
- **resolve/run/record** — `route/resolve.rs`:`RouteResolution.
  suppressed` doc 追従(:101-104)、test `route()` helper
  `trusted: false`(:434)除去。`route/run.rs`:`route.layer`
  層起点解決(:366-370)・record `suppressed` 配線(:633)は
  不変、test comment(:752)の「trusted authoring」は trusted
  層概念の記述で不変。`hook/routing_policy.rs`:test `route()`
  helper `trusted: false`(:2090)除去。
- **integration** — `tests/integration/route.rs`:
  `resolve_project_trusted_displacement_reports_suppressed`
  (:285-336)を rewrite — `trusted` presence →
  `INVALID_RUNTIME_POLICY`(`resolve_project_trusted_rejected`
  新規・`resolve_project_substitute_rejected` :266-283 と同型)+
  untrusted 衝突 → `suppressed` 報告の回帰固定。
  `tests/integration/route_run.rs`:
  `suppressed_project_provenance_reaches_route_record`
  (:1386-1455)— `for trusted` loop 撤去、untrusted arm のみ
  (resolve 出力・record の `suppressed` = 負けた project
  route 名を pin)。
- **comment 追随(code 不変・文言のみ)** — `resolve.rs` の
  `RouteResolution.suppressed` doc(:101-104 — displaced 記述
  除去)、`routes.rs` file header(:12-16)・`SuppressedRoute`
  doc(:121-124)・`Route.layer` doc(:88)・`grants`/`substitute`
  gate comment(:476-480・:518-519)・`run.rs` test comment
  (:752 — 「trusted authoring」は trusted 層概念の記述で
  不変、触れない)。
- **fixture/docs** — `resources/routes.yaml`(:7-8 近辺 —
  `trusted` shape comment 1 行追加)、`docs/guides/
  parent-agent-integration.md`(:237-246 — merge 規約記述
  rewrite)、`docs/guides/configuration.md`(:215-225 —
  層 gate 列挙へ `trusted` 追加)、`docs/guides/command.md`
  (:42 — 同上)、`docs/specs/pretooluse-route-resolver-and-
  workflow-delegation.md`(R1 field 一覧 :216-219・`trusted`
  記述 :253-256・R7 :285-298 — 追従記述)、test-specs
  (§Docs 更新契約 — 後続 flow)。
- **code 不変のまま回帰対象** — `route/supply.rs`・
  `route/run.rs`・`route/payload.rs`・`route_cmd.rs`・
  `route/logs.rs`・`workflow_run.rs`・`usage_log.rs`・
  `usage_report.rs`・`policy/exec.rs`・`policy/load.rs`・
  `tools/exec/mod.rs`・`tools/command/mod.rs`・
  `hook/targets.rs`(`evaluate_with_routes`/`load_or_none`
  経路)・`resources/workflows/routed-exec.workflow.json`・
  `tests/integration/usage_log.rs`・
  `crates/cheaper-tools/tests/proc_invocation.rs`
  — 無改訂で既存挙動が維持されることを確認する churn 境界。

## Docs 更新契約

- `docs/guides/parent-agent-integration.md` — route 宣言層説明
  (:233-246)を rewrite: 「project 層が下位を shadow するのは
  `trusted: true` のときのみ」を「project 層 route は下位層を
  shadow しない — trust は宣言値ではなく宣言層 provenance
  (builtin/user 層宣言 = trusted)が決める。`match.command`
  衝突した project route は `suppressed` provenance として
  報告され、下位 route は active のまま残る。project
  routes.yaml の entry に `trusted` key が存在すると値に
  関わらず route load 全体が `InvalidRuntimePolicy` で失敗
  する。builtin/user 層の `trusted:` は受理されるが inert」
  へ追従。route trust と grant `scope` の整合(spec R5 —
  route は宣言層・grant は authoring 層の provenance が trust
  を決める同一モデルの 2 面、`is_route_eligible` 単一 seam
  不変)を grant 参照記述(:291-295 近辺)へ 1 文追加。
- `docs/guides/configuration.md` — unified exec 参照節
  (:215-225)の「project routes.yaml の entry に `grants`・
  `substitute` key が存在すると route load が失敗」へ
  `trusted` を追加し、「project が宣言できるのは単数 `grant:`
  anchor 参照のみ(非衝突 `match.command` で)」の記述と整合
  させる。
- `docs/guides/command.md` — route grant 参照記述(:42)の
  「project 層 entry に `grants`・`substitute` key が存在する
  と route load が失敗」へ `trusted` を追加。
- `crates/cheaper-cli/resources/routes.yaml` — `substitute`
  shape comment(:8)直下へ `# trusted: bool  # optional — builtin/user 層のみ受理(inert);project 層宣言は load error`
  の 1 行を追加(builtin route 自身は `trusted` を宣言しない
  — 記述のみ)。
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  — refine 対象 spec として R1 field 一覧(:216-219)の
  `trusted?` 注記を「builtin/user 層のみ受理される inert
  bool — project 層宣言は route-load error」へ、:253-256 の
  `trusted` 記述を宣言層 provenance trust へ、R7(:285-298)
  の層間 trust を「project 層 route は下位層を shadow しない
  — trust は `trusted: true` 宣言値ではなく宣言層
  provenance が決める(O1a/#168 — project 層 `trusted`
  presence は `InvalidRuntimePolicy` で拒否)」へ追従する
  (content update のみ — 本 design からの `refines` relation
  は張らず、spec chain 側の管理に委ねる)。
- `docs/test-specs/*` — merge/suppressed 契約は
  `route-resolution.md`(:52-56・:73-74 — `trusted: true`
  shadow 記述・project 採用 example)、`substitute-exec-route-
  mode.md`(:124・:174・:337-348・:356-360・:374・:473・
  :513・:567 — displaced 記録 rule/example)、
  `route-grant-set-projection.md`(:153 — 「`trusted: true`
  宣言は判別に使わない」)が owner — 契約更新は test-spec
  更新(または新規 test-spec)として後続 flow で扱い、本
  design では Verification Notes で AC 対応を固定する。

## Deferred Design Concerns

- **operator/user 側 consent registry(O1b)** — project route
  の shadow を operator が個別許可する宣言機構 | project-tuned
  workflow で trusted 層 route を置き換える正当用途が顕在化
  した場合の復元経路 | operator が workflow を
  `<userConfigDir>/` へ vendor して user 層 route を宣言する
  代替は可能(`target.path` 層起点制約で user 層は project
  file を直接指せない)が、需要が確立した時点で trusted 層
  宣言機構として別 schema で追加可能 — 本決定は project 層
  `trusted` key を閉じるのみで forward-compat を損なわない。
  D7 で却下済みの「user-level opt-in list」を provenance
  時代の形に再設計する必要がある(consent key・staleness
  semantics の設計未着手 — 第二 config surface・split-brain
  の却下理由は存続)。
- **builtin/user 層の inert `trusted` key の完全削除** — 現行
  は受理・bool 検証・inert(documented 現状維持) | routes.yaml
  `version` bump の機会に schema から除去するのが自然 |
  inert key を残す表現 drift は小さいが、schema としての
  正直性は version bump 時に回収する。本 design は version
  bump を行わないため O2c 相当の全層拒否には踏み込まない。
- **`SuppressedRoute` semantics の狭義化** — displaced 記録
  arm は到達不能で除去済み | merge semantics に新たな置換
  経路が将来追加された場合に provenance の意味を再評価 |
  現時点では「衝突で負けた project route」のみを運ぶ報告
  経路として不変。
- **operator migration path(#169 材料)** — `trusted` key を
  含む project routes.yaml は全件 load 失敗(修復 = 当該
  key の削除 1 行 — `trusted: true` で shadow を意図して
  いた場合は workflow を `<userConfigDir>/` へ vendor し
  user 層 route を宣言する)。migration guide 本文・文書
  正直性 sweep は issue 7(#169)が担い、本 design は破壊的
  変更の告知材料として本節を供給するのみ。

## Verification Notes

test 計画(spec Acceptance Criteria・issue AC1-AC3 への対応):

- [ ] **project `trusted` presence 拒否(issue AC2 直撃・spec
      AC2/R2)** — project 層 routes.yaml の `trusted: true`・
      `trusted: false`・非 bool 値(例: `trusted: str`)の 3 形が
      `InvalidRuntimePolicy` の route-load error で拒否され、
      `details.{path,reason:"schema",violation}` が
      `routes[N].trusted: not declarable in the project layer`
      を含む(`routes.rs` unit — `project_layer_trusted_is_rejected`
      新規・`project_layer_grants_are_rejected` :1006-1055 と
      同型)。presence-first — 値が非 bool 等の形状不正でも層
      gate が先に拒否する。
- [ ] **builtin/user 受理・inert(spec AC2/R2)** — builtin/user
      層の `trusted: true`・`trusted: false` 宣言は受理され、
      非 bool 値は `{label}.trusted: not a boolean` の
      `schema_error`(受理層の型検査規約不変 — `routes.rs`
      unit)。値は `Route` に保持されない(field なし —
      compile が証明)。
- [ ] **untrusted 衝突の suppressed provenance(spec AC2/R4)** —
      `trusted` 非宣言の project route が同一 `match.command`
      で builtin/user route に負け、`suppressed` provenance が
      `route resolve` 出力(`route_cmd.rs:126-127`)と
      `kind:"route"` record(`run.rs:633` → `usage_log.rs:418-419`)
      に到達する — merge unit(routes.rs)+ `route.rs`
      integration(rewrite 後の untrusted 衝突 arm)+
      `route_run.rs` `suppressed_project_provenance_reaches_route_record`
      の untrusted arm(負けた project route 名を pin)。
- [ ] **project `trusted` が下位層を shadow できない(issue
      AC2)** — `trusted` を宣言した project route の shadow は
      宣言経路自体が load 時に閉じるため発生しない — project
      `trusted:` routes.yaml が `route resolve` で
      `INVALID_RUNTIME_POLICY`(exit 3)になる integration
      case(`route.rs` — `resolve_project_trusted_rejected`
      新規・`resolve_project_substitute_rejected` :266-283
      と同型)。旧「trusted shadow」経路の回帰固定は presence
      拒否 test が担う(不可能になった arm を pin する形)。
- [ ] **builtin/user 無条件 shadow 回帰(spec AC4/R4)** —
      user が builtin を無条件 shadow する現行規約・同一層
      dup `invalid` 隔離(`same_layer_duplicates_are_quarantined_
      as_invalid` :859-889)に回帰がない — rewrite 後の
      `user_shadows_builtin_and_project_never_shadows`
      (routes.rs unit)で builtin/user silent shadow(project
      suppressed 記録なし)を pin。
- [ ] **非衝突 project route 有効化(spec AC4/R4)** — 非衝突
      `match.command` の project route は従来どおり有効化・
      発火する(`grant:` anchor・byte-matched/substitute mode
      の viability・`expected_input` byte 一致・exit mirror
      は不変 — routes.rs unit の非衝突 arm + 既存
      `route.rs`/`route_run.rs` test 群が無改訂で pass)。
- [ ] **field/shape 不変(spec AC4/R3)** — `Route.trusted`
      field・merge の `route.trusted` 読取りが除去され、
      `Route`/`SuppressedRoute`/`InvalidRouteGroup`/
      `EffectiveRoutes`/`RouteResolution`/`route resolve`
      出力 shape・`kind:"route"` record shape が不変 —
      compile(field 削除の churn 完結)+ `usage_log.rs`
      record shape pin(:298-353)・`route.rs` 出力 shape
      assertion が無改訂で pass。
- [ ] **hook fail-open(spec AC4/R6)** — `trusted` presence
      拒否を含む load 失敗が `load_or_none`
      (`supply.rs:138-152`)で route arm 不活性へ畳まれ、
      command 直接実行経路は継続する(hook integration test・
      fail-open 規約回帰)。
- [ ] **grant `scope` seam 不変・整合文書化(issue AC3/R5)** —
      route grant 参照が `scope=="trusted" && !fixed` のみ
      解決する `is_route_eligible`(`exec.rs:114-116`)単一
      seam に変更がなく、route trust(宣言層 provenance)と
      grant `scope`(authoring 層 provenance)の整合が
      guides/pretooluse spec へ文書化されている(§(d)・
      §Docs 更新契約 — issue AC1 は spec+ADR で済み、本項は
      AC3 の文書化確認)。
- [ ] **既存 route/hook/run test 群 pass(spec AC4)** —
      `trusted` 前提 arm を新規約へ更新後、`mise exec --
      cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` の
      route/hook/run 関連 test が pass — 更新対象は削除
      ではなく「presence 拒否検証 + 負け側 suppressed 回帰
      固定」への置き換え(§Churn)。
- [ ] **破壊的変更の告知材料(spec AC5)** — `trusted` key を
      含む project routes.yaml が file 全体 load 失敗する
      破壊的変更が spec Scope/AC と §Deferred(operator
      migration path — 修復 = 当該 key の削除 1 行)として
      明記され、#169 migration guide へ渡せる記述になって
      いる。

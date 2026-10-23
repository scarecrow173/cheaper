---
id: "TASK-034L59KS9oEjgDzshYZUSn"
type: "task"
status: "done"
title: "route trusted field 削除・merge 無条件化・project 層 trusted presence 拒否"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  implemented-by: []
  depends-on:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  blocks:
    - "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
    - "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/trusted-route-flag-issuer-redesign.md"
  references:
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# route trusted field 削除・merge 無条件化・project 層 trusted presence 拒否

## Work

plan `docs/plans/trusted-route-flag-issuer-redesign.md`
(PLAN-034L59uXskRD4ink3h2a13)の T1。`Route.trusted` field
を削除し、merge の project 分岐を `route.layer` provenance
のみを読む無条件形へ書き換え、`validate_route` に project
層 `trusted` key presence 拒否 gate を新設する。field 削除に
結合する全 `Route` literal churn・code comment 追随・旧
semantics を pin する test の rewrite/更新を同 commit で行い、
`cargo test -p cheaper-cli` 全緑で閉じる。
design(DESIGN-034L4tqYwtc8q88WEIXPh9)§(a)/(b)/(e)/(f)/(g) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

### 1. `route/routes.rs` — field 削除 + merge 無条件化 + 層 gate

- `Route` struct(:58-89)から `trusted` field(:85-87)を doc
  comment(`/// `trusted` — project 層宣言のみ意味を持つ
  (builtin/user は inert)。`)ごと除去する。
  `PartialEq`/`Clone`/`Debug` derive は不変。
- `Route.layer`(:88)へ design §(a-1) verbatim の doc 追記 —
  宣言層 provenance が trust 判定(#168)・target.path 層起点
  解決・診断の単一 source であり、`Route` は trust flag を
  持たない(trust は宣言値ではなく宣言主体が決める — O1a)。
- file header merge 規約 comment(:12-16)追従 — 「project は
  `trusted: true` のときのみ下位層を shadow する」を
  「project は下位層を shadow しない(trust = 宣言層
  provenance)— `match.command` が衝突した project route は
  `suppressed` provenance として記録される」へ。
- `SuppressedRoute` doc(:121-124)を design §(b) verbatim で
  狭義化 — 「merge で負けた route」から「同一
  `match.command` で下位層(builtin/user)route に負けた
  project route」へ(#167 の displaced 記録 arm は到達不能で
  除去済みの旨を記す)。shape `{name, command}` は不変。
- merge loop(:182-222)の `RouteLayer::Project` 分岐
  (:198-220)を design §(b) verbatim の無条件形へ書き換える:

```rust
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

  - `route.trusted` 読取り(:201)・displaced 下位層 route の
    `suppressed` 記録 arm(:210-218 — 新形では `insert`
    戻り値が常に `None` で到達不能)を除去する。unreachable
    arm を防御的に残さない(消えた経路は code からも消す)。
  - builtin/user 分岐(:190-197)は文面不変 — user が
    builtin を無条件 shadow する現行規約(silent)。
- `KNOWN_KEYS`(:388-398)は不変 — `"trusted"`(:397)を登録
  したまま維持する。拒否は層 gate で行い、未知 key 規約
  (:399-403)とは別規約(O2c 不採用 — builtin/user の inert
  受理を壊さない)。`trustd` 等の typo は従来どおり unknown
  key で route-load error。
- `grants`/`substitute` gate comment(:476-480・:518-519)の
  `trusted` 言及追従 — 「`trusted:true` は project 自己申告の
  shadowing gate であり判別に使えない」は flag 削除で stale
  になるため、「project 層の `trusted` key 自体が本 file 後段
  の層 gate で拒否される — 判別子は宣言層 provenance」旨へ
  追従(判別子の記述自体は不変)。
- `validate_route`(:381 — `layer: RouteLayer` は :383 の
  既存 param、新規 param 不要)の `passthroughEnv` parse
  (:612-626)と `trusted` bool 検査(:627-633)の間、:627
  直前へ design §(a-3) verbatim の層 gate を挿入:

```rust
if layer == RouteLayer::Project && entry.contains_key("trusted") {
    return Err(schema_error(
        file,
        &format!("{label}.trusted: not declarable in the project layer"),
    ));
}
// `trusted` — builtin/user 層のみ受理される inert bool。
// 値は保持せず bool 形状のみ検証する。
if let Some(v) = entry.get("trusted") {
    if !v.is_bool() {
        return Err(schema_error(
            file,
            &format!("{label}.trusted: not a boolean"),
        ));
    }
}
```

  - **presence-first** — `trusted: true`/`false`/非 bool の
    全形を値を見ずに拒否(untrusted 層の宣言は内容を評価
    しない — `grants`/`substitute` と同じ fail-closed)。
    層 gate 群の発火順は schema 記述順(grants → substitute
    → trusted)で、複数 gated key を持つ entry は先の gate
    の violation を報告する(逐次 fail-loud)。
  - builtin/user 受理・inert — 値は `Route` に保持しない
    (`let _ =` や discard binding は使わず `if let Some` 形
    で「検査するが使わない」を自明にする)。非 bool 値は
    `{label}.trusted: not a boolean`(受理層の型検査規約
    不変)。
- `Route` literal(:634-645)から `trusted,`(:643)を除去。

### 2. `Route` literal churn(field 削除の compile 追随)

| file | site(現行) | 変更 |
|------|------------|------|
| `route/resolve.rs` | :434 `route()` test helper literal | `trusted: false` 除去 |
| `hook/routing_policy.rs` | :2090 `route()` test helper literal | `trusted: false` 除去(評価本体 :1451-1486 は不変) |
| `route/routes.rs` | :1350 `exec_pair_selects_execution_pair` literal | `trusted: false` 除去 |
| `route/resolve.rs` | :101-104 `RouteResolution.suppressed` field doc | 「trust 不在で負けた project route + displaced 下位層 route」→「同一 `match.command` で下位層に負けた project route(#168 で displaced 記録は project shadow 廃止とともに除去)」へ追従 |

`route/run.rs`(:366-370 target.path 層起点・:633 record
suppressed 配線)・`route/supply.rs`(:136-152
`load_or_none`)・`route_cmd.rs`(:105-134)・`usage_log.rs`
(:359/:418-419)・`policy/exec.rs`(:38-45/:114-116)・
`policy/load.rs`(:239-251)は**不変** — 触れない
(`run.rs:752` の「trusted authoring」test comment は trusted
層概念の記述で不変 — 触れない)。

### 3. `route/routes.rs` unit test(TSPEC Rules に対応)

- **rewrite + rename** `user_shadows_builtin_and_project_needs_
  trusted`(:800-857)→ `user_shadows_builtin_and_project_
  never_shadows` — trusted shadow arm(:822-843 の
  `trusted: true` project YAML + project が勝つ assert)を
  撤去する。維持する pin: user が builtin を無条件 shadow・
  untrusted project route が衝突で負けて `suppressed` に
  記録・非衝突 project route は通常有効化(builtin/user
  silent shadow は project suppressed 記録なし — spec R4)。
- **rewrite + rename** `merge_records_displaced_lower_layer`
  (:1366-1405)— #167 の displaced 記録 semantics は消えた
  ため rewrite(displaced 経路は存在しない): project-lose
  記録(非宣言 project route が衝突で負けて自身が
  `suppressed` へ載る)のみを assert する形へ。
- **comment 追従** `project_layer_grants_are_rejected`
  (:1025-1033)・`project_layer_substitute_is_rejected`
  (:1252-1260)— `trusted: true` 併記 arm は gate 順で
  grants/substitute が先に fire するため assert 自体は不変。
  「`trusted: true` の自己申告は判別に効かない」comment を
  「project 層の `trusted` key 自体も層 gate で拒否される
  — gate 順は grants/substitute が先」旨へ追従。
- **新規** `project_layer_trusted_is_rejected` —
  `project_layer_grants_are_rejected`(:1006-1055)と同型で、
  project 層 routes.yaml の `trusted: true`・`trusted: false`・
  非 bool 値(例: `trusted: str`)の 3 形が
  `InvalidRuntimePolicy` route-load error で拒否され、
  `details.{path, reason:"schema", violation}` が
  `routes[0].trusted: not declarable in the project layer`
  を含むこと(presence-first — 形状不正でも層 gate が先)。
  builtin/user 層の `trusted: true`/`false` 受理と非 bool
  値の `{label}.trusted: not a boolean` schema_error も同
  test で pin(spec R2 両面)。

### 4. integration test rewrite(新 code で red になる旧 pin)

- `tests/integration/route.rs` —
  `resolve_project_trusted_displacement_reports_suppressed`
  (:285-336)を rewrite して `resolve_project_trusted_rejected`
  へ(`resolve_project_substitute_rejected` :266-283 と同型):
  - project 層 `.cheaper/routes.yaml` の entry に `trusted:
    true` → `route resolve` が `INVALID_RUNTIME_POLICY`・
    exit 3・`env["ok"] == false`(`trusted` presence が
    file 全体 load を失敗させる — issue AC2 の最強形 pin)。
  - `trusted` 非宣言の project route が同一 `match.command`
    で user 層 route に負け、`result["route"]` が下位層
    route 名・`result["suppressed"]` が負けた project route
    名を報告する untrusted 衝突 arm(回帰固定 — spec R4)。
- `tests/integration/route_run.rs` —
  `suppressed_project_provenance_reaches_route_record`
  (:1386-1455)の `for trusted in [false, true]` loop を
  撤去し untrusted arm のみ残す: `trusted` 非宣言の project
  route が builtin(`routed` builtin workflow route)に負け、
  resolve 出力と `kind:"route"` record の `suppressed` が
  負けた project route 名(`project-shadow`)を pin する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/trusted-route-flag-issuer-redesign.md`
  (PLAN-034L59uXskRD4ink3h2a13 — T1)
- `docs/designs/trusted-route-flag-issuer-redesign.md`
  (DESIGN-034L4tqYwtc8q88WEIXPh9 — §(a) verbatim rust・§(b)
  merge 差分・§(e) signature 契約・§(g) error 契約・Churn
  一覧が AUTHORITATIVE)
- `docs/specs/trusted-route-flag-issuer-redesign.md`
  (SPEC-034LwdoInh3ie1KPlSjkJl — R1/R2/R3)
- `docs/test-specs/trusted-route-flag-issuer-redesign.md`
  (TSPEC-034L59Ghpuk8XWUN8qgXJu — merge 無条件形・presence
  拒否・受理/inert・field 削除・suppressed provenance Rules)

## Dependencies

- 前提 task なし(chain の head)。
- 後続: T2 は層 gate の存在を検証対象とするため本 task 前提。
  T3 は本 task の確定 semantics を docs へ転記するため前提。
- 既知の transient: `Route.trusted` field 削除は全 `Route`
  literal(routes.rs:643/:1350・resolve.rs:434・
  routing_policy.rs:2090)の同 commit 追随が必須 —
  本 task で cheaper-cli 全体を compile 可能に保つこと。
  加えて `trusted` を YAML で宣言する旧 semantics test 4 件
  (§3/§4)は compile 可能でも実行で red になるため同 commit
  で rewrite する(#167 先例 — semantics を反転させる test は
  semantic 変更 commit と同じ task で処理する)。

## Affected Files

- `crates/cheaper-cli/src/route/routes.rs`(modify — code・
  comment・unit test)
- `crates/cheaper-cli/src/route/resolve.rs`(modify — literal
  除去 + field doc 追従のみ)
- `crates/cheaper-cli/src/hook/routing_policy.rs`(modify —
  literal 除去のみ)
- `crates/cheaper-cli/tests/integration/route.rs`(modify —
  rewrite)
- `crates/cheaper-cli/tests/integration/route_run.rs`(modify —
  arm 撤去)

## Done When

- [x] `Route.trusted` field(doc comment ごと)が削除され、
      `Route.layer` doc が trust・診断・target.path 層起点解決
      の単一 provenance source 旨を記している
- [x] merge の `RouteLayer::Project` 分岐が
      `effective.contains_key(&route.command)` のみで分岐する
      無条件形(衝突 → `suppressed` 記録・非衝突 →
      `order.push` + `insert`)になり、`route.trusted` 読取り・
      displaced 記録 arm が除去されている
- [x] `validate_route` が `passthroughEnv` parse 後・`trusted`
      bool 検査前に `layer == RouteLayer::Project &&
      entry.contains_key("trusted")` → `schema_error(
      "{label}.trusted: not declarable in the project layer")`
      を返し(presence-first)、builtin/user 層では `trusted`
      を bool 形状検査のみして受理する(値は保持しない)
- [x] `KNOWN_KEYS` の `"trusted"` 登録が維持され、typo が
      unknown key で拒否される規約が不変
- [x] 全 `Route` literal(routes.rs:643/:1350・resolve.rs:434・
      routing_policy.rs:2090)が追随し cheaper-cli が compile
      する
- [x] comment 追随 6 site(routes.rs :12-16/:88/:121-124/
      :476-480/:518-519・resolve.rs :101-104)が全件更新され
      `trusted:true` shadow 規約の stale 記述が残っていない
- [x] 新規 unit test `project_layer_trusted_is_rejected` が
      `true`/`false`/非 bool の presence 拒否 3 形と
      builtin/user 受理・非 bool 型検査を pin する
- [x] `user_shadows_builtin_and_project_never_shadows`・
      `merge_records_displaced_lower_layer` rewrite・
      `resolve_project_trusted_rejected`・route_run untrusted
      arm が新 semantics を pin し、`cargo test -p
      cheaper-cli` が全緑

## Out of Scope

- hook 経路 fail-open の project 層違反 test(T2 —
  `tests/integration/hook.rs` 新規 case)
- docs/guides・`resources/routes.yaml` comment・pretooluse
  spec の content update(T3)
- `route/supply.rs`・`route/run.rs`・`route_cmd.rs`・
  `usage_log.rs`・`policy/*` の変更(全て不変 — churn 境界外)
- builtin/user 層 `trusted` key の schema 完全除去(routes.yaml
  `version` bump — Deferred)
- operator/user 側 consent registry(O1b — YAGNI)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::routes` が pass(routes.rs unit test — 新規含む) — 実測: 17 passed / 0 failed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route::` が pass(rewrite 後 integration) — 実測: 10 passed / 0 failed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` が pass(untrusted arm) — 実測: 35 passed / 0 failed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が compile + pass(literal churn の workspace 内 compile 回復 + 全 test 緑確認) — 実測: lib 115・integration 301・observability 17・observability_dev_seam 3・regression 1・run_dev_seam 39・workflow 18・workflow_dev_seam 5、全て 0 failed
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 — 実測: exit 0
- [x] `grep -rn "trusted" crates/cheaper-cli/src/route/routes.rs` で `route.trusted`・`trusted,` literal・stale comment 記述の残存なし(`KNOWN_KEYS`・層 gate・受理 bool 検査・追従済み comment のみ残る) — 実測: `route.trusted`/`.trusted` 参照は `{label}.trusted` error 文言を除き残存 0

deviation: design §(a-3) verbatim の `v.is_bool()` は serde_json
の API 名 `is_boolean()` へ修正して実装(実際の method 名 —
`mise exec -- cargo check` で E0599 を検出し修正)。rename は
`merge_records_displaced_lower_layer` →
`merge_records_losing_project_route` を採用(task は rewrite +
rename を指定、新規 semantics に合わせた命名)。

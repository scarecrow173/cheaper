---
id: "TASK-034L2N4rLJgtw6UXk7EuK3"
type: "task"
status: "done"
title: "route substitute 宣言 schema・project 層拒否・merge suppressed・grant_set/exec_pair"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
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
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
  blocks:
    - "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/substitute-exec-route-mode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/substitute-exec-route-mode.md"
  references:
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# route substitute 宣言 schema・project 層拒否・merge suppressed・grant_set/exec_pair

## Work

plan `docs/plans/substitute-exec-route-mode.md`
(PLAN-034L2MF3X78MvKvcSN8jmV)の T1。routes.yaml route entry に
optional `substitute: {grant, argv}` を追加し、`RouteSubstitute`
struct・`Route.substitute` field・`KNOWN_KEYS`・
`validate_route` の nested object parse + project 層 presence
拒否・`Route::grant_set()` union・`Route::exec_pair()`・merge
の project `trusted:true` 置換で displaced route を
`suppressed` provenance へ記録する拡張を実装する。
design(DESIGN-034L0RxGVVTOqtaUUThowk)§(a)/(g)/(h)/(i) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

### 1. `route/routes.rs` — `RouteSubstitute` + `Route.substitute` + schema

- `RouteSubstitute` struct 新設 — design §(a) verbatim
  (`#[derive(Debug, Clone, PartialEq)]`、`pub grant: String` +
  `pub argv: Vec<String>`、`grant` は既存 `command.grants`
  参照名、`argv` は executable を含まない実行 args 列、
  presence が mode switch — `Some` で substitute mode、
  trusted 層限定の doc comment 付き)。
- `Route` struct(:37-62)の `pub grants`(:52)直後へ
  `pub substitute: Option<RouteSubstitute>` を追加 — design
  verbatim doc comment(firing gate・record attribution
  anchor は `route.grant` のまま)。
- `KNOWN_KEYS`(:334-343)の `"grants"` 直後へ `"substitute"`
  を追加 — fail-loud unknown-key 規約(:344-348)を維持
  (`substitute` の typo は unknown key で route-load error)。
  `KNOWN_KEYS` は全層共通のため層制約は unknown-key ではなく
  別の明示判定で行う(`grants` と同じ構造)。
- `validate_route`(:327 — `layer: RouteLayer` は :329 の既存
  param、新規 param 不要)の `grants` 層制約 + parse
  (:426-454)の直後へ design §(a) verbatim の 2 段処理を挿入:
  1. `layer == RouteLayer::Project && entry.contains_key(
     "substitute")` → `schema_error(file, "{label}.substitute:
     not declarable in the project layer")`
     (`InvalidRuntimePolicy` — **形状検査より先に** presence
     で拒否する fail-closed、O3a。`grants` の :426-431 と同型)。
  2. `substitute` parse — nested object 必須(非 object →
     `"{label}.substitute: not an object"`)、nested 未知 key
     → `"{label}.substitute: unknown key: {key}"`、
     `grant` = non-empty string 必須(欠落・非 string・空 →
     `"{label}.substitute.grant: missing or not a string"`)、
     `argv` = array 必須(欠落・非 array → `"{label}.
     substitute.argv: missing or not an array"`)、各要素は
     non-empty string(→ `"{label}.substitute.argv: element
     must be a non-empty string"`)。`argv: []` は受理。
- `Route` literal 構築(:506-516)へ `substitute` を追加。
- `Route::grant_set()`(:64-75 付近、`(&self) ->
  BTreeSet<String>`)の set へ `substitute.grant` を追加 —
  `{route.grant} ∪ grants ∪ {substitute.grant}` 規則の単一
  owner(design verbatim doc comment 付き)。
- `Route::exec_pair()` 新設 — `<'a>(&'a self, typed_args:
  &'a [String]) -> (&'a str, &'a [String])` で substitute
  mode は `(substitute.grant, substitute.argv)`、
  byte-matched は `(route.grant, typed_args)` を返す実行ペア
  選択規則の単一 owner(viability・run 配線が共有 —
  design verbatim)。

### 2. merge suppressed provenance(routes.rs :154-166)

- `RouteLayer::Project` 置換 branch で `effective.insert` の
  戻り値(displaced route)を `suppressed` へ記録する
  design §(a) verbatim 変更:

```rust
RouteLayer::Project => {
    if effective.contains_key(&route.command) && !route.trusted {
        suppressed.push(SuppressedRoute { name: route.name, command: route.command });
    } else {
        if !effective.contains_key(&route.command) {
            order.push(route.command.clone());
        }
        if let Some(displaced) = effective.insert(route.command.clone(), route) {
            suppressed.push(SuppressedRoute {
                name: displaced.name,
                command: displaced.command,
            });
        }
    }
}
```

- `SuppressedRoute` の semantics は「trust 不在で負けた
  project route」から「merge で負けた route」へ拡張 —
  shape `{name, command}`・consumer(resolve の
  `suppressed()`・`RouteResolution.suppressed`・record 経路)
  は不変。doc comment(:77-78)・file header(:12-14)の文言
  追随は T4 の責務。
- 境界: builtin/user 層間の shadow(:146-150)は silent の
  まま維持 — 記録対象は project 層の `trusted:true` 置換のみ。

### 3. `Route` literal churn(field 追加の compile 追随)

| file | site(現行) | 変更 |
|------|------------|------|
| `route/resolve.rs` | :395-407 `route()` test literal | `substitute: None` 追加(`route_with_grants()` :411-422 は `..route()` spread で明示不要) |
| `hook/routing_policy.rs` | :2080-2091 `route()` test literal | `substitute: None` 追加(評価本体は不変) |

### 4. routes.rs unit test(:574 以降 — TSPEC の Rules に対応)

- `substitute: {grant, argv}` parse → `Route.substitute ==
  Some(RouteSubstitute{…})`・`argv: []` 受理・`grant` 単数
  required 不変。
- malformed: 非 object・`grant`/`argv` 欠落・nested 未知 key・
  `grant` 非 string/空・`argv` 非 array・要素非 string/空 →
  `InvalidRuntimePolicy` + `details.{path, reason:"schema",
  violation}` が file・entry を名指し。`substitute` typo は
  unknown key 拒否不変。
- project 層 `substitute` presence → `InvalidRuntimePolicy`
  (`"not declarable in the project layer"`)、値が malformed
  でも presence が先に拒否されること。builtin/user 層の
  `substitute` 受理・project 層 `grant` 単数受理 —
  `project_layer_grants_are_rejected`(:873-922)と同型。
- `grant_set()` — `{route.grant} ∪ grants ∪ {substitute.
  grant}`、`substitute.grant == route.grant`・`∈ grants[]`
  の dedupe。
- `exec_pair()` — substitute `Some` で宣言ペア、`None` で
  `{route.grant, typed_args}`。
- merge provenance — project `trusted:true` が下位層 route を
  置き換えると `suppressed` が displaced route 名を記録、
  新規追加では記録しない、untrusted project 敗北の既存記録は
  不変。
- **既存 test 更新**: `user_shadows_builtin_and_project_needs_
  trusted`(:710)の `suppressed.is_empty()` assert は新
  semantics で displaced user route 名を返すため失敗する —
  assert を反転して新 semantics を pin する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/substitute-exec-route-mode.md`
  (PLAN-034L2MF3X78MvKvcSN8jmV — T1)
- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk — §(a) verbatim・§(g)
  signature 契約・§(i) error 契約が AUTHORITATIVE)
- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h — R1/R2/R5)
- `docs/test-specs/substitute-exec-route-mode.md`
  (TSPEC-034L2MB0m2NOWZsyuTHsjA — declaration schema・層制約・
  grant_set/exec_pair・merge suppressed Rules)

## Dependencies

- 前提 task なし(chain の head)。
- 後続: T2 は `RouteSubstitute`/`exec_pair`/`grant_set` を
  消費するため本 task 前提。T3 の run 配線・T4 docs も
  本 task の semantics 確定後。
- 既知の transient: `Route` field 追加は全 literal 構築の同
  commit 追随が必須 — 本 task で cli crate 全体を compile
  可能に保つこと。

## Affected Files

- `crates/cheaper-cli/src/route/routes.rs`(modify)
- `crates/cheaper-cli/src/route/resolve.rs`(modify — literal のみ)
- `crates/cheaper-cli/src/hook/routing_policy.rs`(modify — literal のみ)

## Done When

- [ ] `RouteSubstitute`・`Route.substitute`・`KNOWN_KEYS` の
      `"substitute"`・`Route::grant_set()` の union 拡張・
      `Route::exec_pair()` が存在し、`validate_route` が
      `substitute` を nested object として parse する
      (`grant`/`argv` required・nested 未知 key 拒否・
      non-empty 要素・`argv: []` 受理)
- [ ] project 層 entry の `substitute` presence が形状検査
      より先に `InvalidRuntimePolicy`(`"not declarable in the
      project layer"`)で拒否され、builtin/user 層は受理される
- [ ] merge の `RouteLayer::Project` 置換 branch が displaced
      下位層 route を `suppressed` provenance へ記録し、
      builtin/user 層間 shadow は silent のまま
- [ ] 全 `Route` literal(routes.rs・resolve.rs・
      routing_policy.rs)が追随し cheaper-cli が compile する
- [ ] `user_shadows_builtin_and_project_needs_trusted` の
      assert が新 semantics(displaced 記録)へ反転されている
- [ ] routes.rs unit test が schema/層制約/grant_set/exec_pair/
      merge provenance を pin し `cargo test -p cheaper-cli`
      が pass

## Out of Scope

- viability の retarget・`route.grant` member 級降格・
  interactive union(T2)
- `route/run.rs` の exec_pair 配線・route_run.rs e2e(T3)
- docs・comment 追随(`Route.grant`/`Route.grants`/
  `SuppressedRoute` doc・file header — T4)
- `substitute.grant` の存在/eligibility 検証(viability へ
  委譲する規約 — 宣言自体は legal)
- `route resolve` 出力の substitute echo

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::routes` が pass(routes.rs unit test) — 実測: 16 passed / 0 failed(commit 17b7107)。T5 でも再実測 16 passed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が compile + pass(literal churn の workspace 内 compile 回復確認) — 実測: `mise run check` の `cargo test --workspace`(dev-seam feature 付き)が 0 failed で exit 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 — 実測: T5 の clippy run・`mise run check` 内 clippy ともに exit 0

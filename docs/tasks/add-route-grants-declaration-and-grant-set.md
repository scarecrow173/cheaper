---
id: "TASK-034KVxrNr5J7VrsdNaZDAP"
type: "task"
status: "todo"
title: "route grants 宣言 schema・project 層拒否・Route::grant_set"
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
    - "docs/plans/route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
  blocks:
    - "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
    - "docs/tasks/document-route-grant-set-projection.md"
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
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# route grants 宣言 schema・project 層拒否・Route::grant_set

## Work

plan `docs/plans/route-grant-set-projection.md`
(PLAN-034KVx51kVYoMwO8UmJ8VE)の T1。routes.yaml route entry に
optional `grants: [name,…]` を追加し、`Route` struct field・
`KNOWN_KEYS`・`validate_route` の parse + dedupe + project 層
presence 拒否・`Route::grant_set()` helper を実装する。
design(DESIGN-034KUgF5QB3WuzFG6PmVvr)§(a)/(g)/(h)/(i) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

### 1. `route/routes.rs` — `Route` struct + schema

- `Route` struct(:37-55)の `pub grant: String` 直後へ
  `pub grants: Vec<String>` を追加 — design §(a) の verbatim
  doc comment(`grants` — route.grant 以外の追加委譲 grant
  名(宣言順)。parse 時に member 間重複と `route.grant` 自身の
  参照を冪等に除去済み — `[]` ≡ 未指定。有効 set は
  `grant_set()` = {grant} ∪ grants)。
- `KNOWN_KEYS`(:314-322)に `"grants"` を `"grant"` 直後へ追加 —
  fail-loud unknown-key 規約(:323-327)を維持(`grants` の typo
  は unknown key で route-load error)。`KNOWN_KEYS` は全層共通
  で、層制約は unknown-key ではなく別の明示判定で行う。
- `validate_route`(:309 — `layer: RouteLayer` は既存 param、
  新規 param 不要)の `grant` 確定(:399)直後へ design §(a)
  verbatim の 2 段処理を挿入:
  1. `layer == RouteLayer::Project && entry.contains_key("grants")`
     → `schema_error(file, "{label}.grants: not declarable in the
     project layer")`(`InvalidRuntimePolicy` — member 形状検査
     より**先に** presence で拒否する fail-closed、O7a)。
  2. `grants` parse — array 必須(非 array → `"{label}.grants:
     not an array"`)、各 member は non-empty string(→
     `"{label}.grants: member must be a non-empty string"`)、
     `name != grant && !grants.contains(name)` で member 間
     重複と self-ref を冪等除去(宣言順の先勝ちを保持)。
- `Route` literal 構築(:451-460)へ `grants` を追加。
- `Route::grant_set()` 新設 — `(&self) -> BTreeSet<String>` で
  `{route.grant} ∪ grants`(design §(a) verbatim — 「有効 set
  の構築責務は本 helper に置く」契約の単一 owner)。

### 2. `Route` literal churn(field 追加の compile 追随)

| file | site(現行) | 変更 |
|------|------------|------|
| `route/resolve.rs` | :369-380 test literal | `grants: Vec::new()`(または member を持たせる test は値)追加 |
| `hook/routing_policy.rs` | :2080-2091 test literal | `grants: Vec::new()` 追加(評価本体は不変) |

### 3. routes.rs unit test(TSPEC の Rules に対応)

- `grants: [a, b]` parse → `Route.grants == ["a","b"]`・
  member 間重複 `[a,a]` と self-ref `[grant,a]` の冪等除去・
  `grants: []` ≡ 未指定・`grant` required 不変。
- malformed: 非 array・非 string member・空 string member →
  `InvalidRuntimePolicy` + `details.{path, reason:"schema",
  violation}` が file・entry を名指し。unknown key 拒否不変。
- project 層 `grants` presence → `InvalidRuntimePolicy`
  (`"not declarable in the project layer"`)、member 形状が
  不正でも presence が先に拒否されること。builtin/user 層の
  `grants` 宣言受理・project 層 `grant` 単数受理。
- `grant_set()` — `{route.grant} ∪ grants` の BTreeSet、
  `grants: []` で `{route.grant}` のみ。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/route-grant-set-projection.md`
  (PLAN-034KVx51kVYoMwO8UmJ8VE — T1)
- `docs/designs/route-grant-set-projection.md`
  (DESIGN-034KUgF5QB3WuzFG6PmVvr — §(a) verbatim・§(g)
  signature 契約・§(i) error 契約が AUTHORITATIVE)
- `docs/specs/route-grant-set-projection.md`
  (SPEC-034KTMfXYD5ZlGZCRb8Paj — R1・O7a)
- `docs/test-specs/route-grant-set-projection.md`
  (TSPEC-034KVxaBMzQE5YtVOx4j3w — declaration schema・層制約・
  grant_set Rules)

## Dependencies

- 前提 task なし(chain の head)。
- 後続: T2 は `Route.grants`/`grant_set()` を消費するため本
  task 前提。T4 docs も本 task の semantics 確定後に並列可能。
- 既知の transient: `Route` field 追加は全 literal 構築の同
  commit 追随が必須 — 本 task で cli crate 全体を compile 可能
  に保つこと。

## Affected Files

- `crates/cheaper-cli/src/route/routes.rs`(modify)
- `crates/cheaper-cli/src/route/resolve.rs`(modify — literal のみ)
- `crates/cheaper-cli/src/hook/routing_policy.rs`(modify — literal のみ)

## Done When

- [ ] `Route` に `pub grants: Vec<String>`・`KNOWN_KEYS` に
      `"grants"`・`Route::grant_set()` が存在し、`validate_route`
      が `grants` を parse + dedupe する
- [ ] project 層 entry の `grants` presence が member 形状検査
      より先に `InvalidRuntimePolicy`(`"not declarable in the
      project layer"`)で拒否され、builtin/user 層は受理される
- [ ] malformed `grants`(非 array/非 string/空 string)が
      `InvalidRuntimePolicy` + file・entry 名指しの details で
      route-load error になり、unknown key 規約が不変
- [ ] 全 `Route` literal(routes.rs・resolve.rs・
      routing_policy.rs)が追随し cheaper-cli が compile する
- [ ] routes.rs unit test が schema/dedupe/層制約/grant_set を
      pin し `cargo test -p cheaper-cli` が pass

## Out of Scope

- viability の member 検査・`argv_shell_re` 除去・
  `project_grants`(T2)
- record `executed`(T3)・docs(T4)
- member 名の存在/eligibility 検証(viability へ委譲する規約
  — 宣言自体は legal)
- `route resolve` 出力の `grants` echo・宣言数上限

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::routes` が pass(routes.rs unit test)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が compile + pass(literal churn の workspace 内 compile 回復確認)
- [ ] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

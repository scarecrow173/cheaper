---
id: "TASK-034L2N9r4zJdpE3jtoHivQ"
type: "task"
status: "done"
title: "viability 実行ペア retarget・route.grant member 降格・interactive union"
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
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/tasks/add-route-substitute-declaration-and-exec-pair.md"
  blocks:
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
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
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-tools/src/tools/exec/mod.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
  defers: []
  deferred-by: []
---

# viability 実行ペア retarget・route.grant member 降格・interactive union

## Work

plan `docs/plans/substitute-exec-route-mode.md`
(PLAN-034L2MF3X78MvKvcSN8jmV)の T2。`resolve.rs` の
`viability()`(:276-383)の no-grant 段と stdin-unsupported
段を修正し、substitute mode の検査対象を `exec_pair` 由来の
実行ペアへ retarget、`route.grant` を member 級検査へ降格、
substitute ペアへ interactive union 分類を適用する。
design(DESIGN-034L0RxGVVTOqtaUUThowk)§(b)/(g)/(h)/(i) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

分岐順序(unsafe-command → invalid-route → missing-target →
no-grant → member → stdin-unsupported → runnable)・
`Viability` enum(:20-28)・`as_str`(:30-42)・
`RouteResolution` shape(:105-128 — `grant` は `route.grant`
anchor echo のまま、substitute echo は載せない)は不変。

### 1. `route/resolve.rs` — anchor 完全検査の retarget(:305-348)

- 検査対象を `route.exec_pair(&out.exec_args)` 経由の実行ペア
  へ一本化する design §(b-1) verbatim — byte-matched では
  `{route.grant, out.exec_args}`(現行と同値)、substitute
  mode では `{substitute.grant, substitute.argv}`:

```rust
let (exec_name, exec_args) = route.exec_pair(&out.exec_args);
let Some(grant) = input
    .exec_policy
    .policy
    .grants
    .get(exec_name)
    .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
else {
    return Viability::NoGrant;
};
let Ok(pinned) = cheaper_tools::tools::exec::pin_exec_grant(
    exec_name, grant, /* root/symlink_policy/denylist/exec_deny — 現行どおり */
) else {
    return Viability::NoGrant;
};
let options = ExecToolOptions { /* 現行と同一 */ };
if check_exec_execution(&options, exec_name, exec_args).is_err() {
    return Viability::NoGrant;
}
```

- `ExecToolOptions` の構築内容は現行と同一 — 変わるのは
  lookup 名・`pins` map key・`check_exec_execution` の
  `name`/`args` だけ。`check_exec_execution` 内部
  (`resolve_exec_grant` → `authorize_args` argv 照合 → cwd
  prepare → `verify_pinned_executable`)の順序・predicate は
  不変。
- `pinned` は後段 (b-3) の substitute 側 interactive 分類で
  再利用するため `pins` map へは clone を入れる。

### 2. `route.grant` の member 級降格(member loop :353-377)

- substitute 宣言のある route では `route.grant` を既存
  member ループと同じ位置・同じ検査(存在 +
  `is_route_eligible` + `pin_exec_grant`、argv 照合なし)へ
  降格する design §(b-2) verbatim:

```rust
let demoted = route.substitute.is_some().then_some(&route.grant);
for member in demoted.into_iter().chain(&route.grants) {
    // 存在 + is_route_eligible + pin_exec_grant — ループ本体は一切変更しない
}
```

- `then_some` + `Option::into_iter` で substitute mode のみ
  先頭に `route.grant` を連結 — 「anchor 完全検査 ↔ member
  級検査」の 2 段構造は維持し、検査位置(member ループ)を
  そのまま共有する。失敗は全て `NoGrant` へ foldし
  `tracing::debug!` へ member 名を載せる経路を共有
  (viability 値上は区別不能 — O2a fold 規約どおり)。
- 原 argv の `route.grant` argv pattern 照合は行わない —
  実行されない command の形骸化検査(R3/R4)。

### 3. stdin-unsupported 段(:378-381)への substitute 評価追加

- typed 側を維持しつつ substitute ペアへ同じ分類を適用する
  design §(b-3) verbatim:

```rust
if is_interactive(name, &out.exec_args) {
    return Viability::StdinUnsupported;
}
if let Some(substitute) = &route.substitute {
    if is_interactive(&command_name(&grant.executable), &substitute.argv)
        || is_interactive(&command_name(&pinned), &substitute.argv)
    {
        return Viability::StdinUnsupported;
    }
}
```

- 分類対象名は**宣言名と解決名の union** —
  `command_name(&grant.executable)`(grant 宣言の
  `executable` basename)と `command_name(&pinned)`(pin
  解決済み canonical path の basename)の両方。宣言名側は
  canonical 名ずらし(`python3`→`python3.11`・
  `vim`→`vim.basic`・`vi`→`nvi`)の穴を塞ぎ、解決名側は
  shim/symlink 名でも実体で分類する。`grant`・`pinned` は
  (b-1) で解決済みのため追加コストなし。`command_name`
  (`routing_policy.rs:1324`)は :17 の既存 import で足りる。
- `grants[]` member は screening 対象外のまま(known
  semantics — member job は自身 grant bound で動く別経路)。

### 4. test — resolve.rs unit + route.rs integration

- `resolve.rs` test module: `use crate::route::routes::{…}`
  (:387-390)へ `RouteSubstitute` import + substitute
  viability 新規 case:
  - retarget — substitute 宣言 + 健全な substitute ペア +
    原 argv が `route.grant` pattern 不一致でも `Runnable`
    (AC4 の回帰固定)。
  - substitute 失敗 fold — `substitute.argv` が argv pattern
    不一致・`substitute.grant` 未存在・`scope:"project"`・
    `fixed`・pin 不能の各場合に `NoGrant`(AC3)。
  - `route.grant` demote — `route.grant` 未存在・ineligible・
    pin 不能で `NoGrant`(member 級降格 — AC4)。
  - stdin union 両側 — (a) 宣言 executable 名が list 内で
    pin 解決先が canonical 名ずらし(`python3`→`python3.11`
    相当の fixture)でも `StdinUnsupported`、(b) 宣言名が
    list 外でも pin 解決済み実体名が list 内なら
    `StdinUnsupported`、`substitute.argv` に interactive
    flag 含有でも同じ(AC9)。typed 側 interactive は
    substitute 宣言があっても `StdinUnsupported` で不発。
  - match precedence pin — non-runnable best(longer prefix)が
    あるとき viability は best のみ評価し shorter match へ
    fall-through しない(known semantics — review P1-1)。
- `tests/integration/route.rs`: substitute 宣言 fixture +
  viability retarget case + project 層 `substitute` 拒否
  (`load` が `InvalidRuntimePolicy`)+ `route resolve` 出力
  shape 不変の確認(substitute echo が載らず `grant` が
  `route.grant` anchor を指す — R11) + project
  `trusted:true` shadow で `suppressed` field が displaced
  route 名を記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/substitute-exec-route-mode.md`
  (PLAN-034L2MF3X78MvKvcSN8jmV — T2)
- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk — §(b) verbatim・§(i)
  error 契約が AUTHORITATIVE)
- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h — R3/R4/R10/R11)
- `docs/test-specs/substitute-exec-route-mode.md`
  (TSPEC-034L2MB0m2NOWZsyuTHsjA — viability retarget・member
  級降格・stdin union・match precedence Rules)
- `docs/tasks/add-route-substitute-declaration-and-exec-pair.md`
  (TASK-034L2N4rLJgtw6UXk7EuK3 — 前提)

## Dependencies

- `docs/tasks/add-route-substitute-declaration-and-exec-pair.md`
  (T1) — `RouteSubstitute`・`Route.substitute`・`exec_pair`・
  `grant_set`・literal churn(`route()` に `substitute: None`
  済み)が前提。
- 後続: T3 の substitute e2e は本 task の viability retarget
  が無いと substitute route が発火しないため直列。

## Affected Files

- `crates/cheaper-cli/src/route/resolve.rs`(modify)
- `crates/cheaper-cli/tests/integration/route.rs`(modify)

## Done When

- [ ] anchor 完全検査が `route.exec_pair(&out.exec_args)` の
      実行ペアを対象にし、`ExecToolOptions` 構築・
      `check_exec_execution` 内部順序が不変
- [ ] substitute mode の `route.grant` が member ループ位置
      (存在 + `is_route_eligible` + `pin_exec_grant`、argv
      照合なし)で検査され、原 argv の `route.grant` pattern
      照合が行われない
- [ ] substitute 側検査失敗・member 級 `route.grant` 失敗・
      `grants[]` member 失敗が全て `Viability::NoGrant` に
      fold される(新規 viability 値なし)
- [ ] stdin-unsupported が typed 側不変 + substitute ペアの
      宣言名∪解決名 union で分類される
- [ ] `RouteResolution` shape・`route resolve` 出力 shape が
      不変(substitute echo なし、`grant` = `route.grant`)
- [ ] resolve.rs unit test + `route.rs` integration が
      retarget/demote/fold/union/precedence/層拒否を pin し
      `cargo test -p cheaper-cli` が pass

## Out of Scope

- `route/run.rs` の exec_pair 配線・`route_run.rs` e2e(T3)
- docs・comment 追随(`RouteResolution.suppressed` doc —
  T4)
- `grants[]` member の interactive screening・member argv の
  route 解決時照合(known semantics)
- non-runnable best match からの shorter match への
  fall-through(別設計 — precedence pin のみ)
- `RouteExecution::new`・`is_route_eligible`・
  `check_exec_execution`・`pin_exec_grant` の signature 変更

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::resolve` が pass(resolve.rs unit test) — 実測: 7 passed / 0 failed(commit df6dfd8)。T5 でも再実測 7 passed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route::` が pass(route.rs integration) — 実測: 10 passed / 0 failed(commit df6dfd8)。T5 でも再実測 10 passed
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 — 実測: T5 の clippy run・`mise run check` 内 clippy ともに exit 0

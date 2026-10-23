---
id: "TASK-034L59Ty2ObpwGwNFduACU"
type: "task"
status: "todo"
title: "project 層 trusted 拒否の hook fail-open 回帰 test"
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
        path: "crates/cheaper-cli/tests/integration/hook.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  implemented-by: []
  depends-on:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
  blocks:
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
    - "crates/cheaper-cli/tests/integration/hook.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# project 層 trusted 拒否の hook fail-open 回帰 test

## Work

plan `docs/plans/trusted-route-flag-issuer-redesign.md`
(PLAN-034L59uXskRD4ink3h2a13)の T2。design
(DESIGN-034L4tqYwtc8q88WEIXPh9)§(g) error 契約・Verification
Notes の fail-open 項目に従い、`trusted` presence 拒否を含む
load 失敗が hook 経路の `load_or_none`
(`route/supply.rs:138-152`)で route arm 不活性へ畳まれ、
command の直接実行経路が継続することを
`tests/integration/hook.rs` の回帰 test で pin する。
production code は変更しない(test のみ追加)。

### 1. fail-open 回帰 test 新規(`tests/integration/hook.rs`)

route supply 節(:848 以降 — `shell_route_arm_engages_via_
firing_load` の近傍)へ test を追加:

- fixture 構成:
  - **user 層**(`f.cfg` = `CHEAPER_CONFIG_DIR`)に valid な
    `routes.yaml` + grant policy + workflow file を配置 —
    既存 `routed_shell_fixture`(:851-873)と同じ構造で
    `cat` を route する(`cat big.txt` が健全なら
    `Verdict::Rewrite{fallback: Deny}` → exit 2・dedup record
    なし — route arm 活性の対照)。
  - **project 層** `<workspace>/.cheaper/routes.yaml`
    (`f.ws.path().join(".cheaper")`)へ `trusted` key を含む
    route entry(例: `trusted: true`)を書く — **違反 file は
    project 層に置くこと**。既存 `routed_shell_fixture` は
    `f.cfg` = user 層へ routes.yaml を置くため層 gate の
    対象外(user 層の `trusted` は受理・inert で違反になら
    ない) — fixture を再利用して `trusted` を user 層に
    書いてはいけない(design Verification Notes の必須
    条件)。
- 期待挙動(project 層 `trusted` presence → file 全体 load
  失敗 → `load_or_none` が `None` へ fold → **builtin/user/
  project 全層の route arm が不活性化**):
  - `cat big.txt` の hook eval が route arm を通らず unrouted
    eval 経路へ落ちる — `routed_shell_fixture` の control
    arm(routes/policy 無しの同一 firing)と同じ deny 側経路
    で処理され、dedup store に verdict record が publish
    される(route arm 発火時の `Rewrite` verdict は persist
    されない対照 — record の有無が route arm 不活性の観測
    可能な証明)。
  - command 処理自体は継続する(allow/deny の eval 結果は
    返る — crash・error 出力で止まらない)。fail-closed な
    schema 強制と fail-open な hook 合成の責務分離を pin
    する(spec R6)。
- 対照(同一 test 内・または既存 control で担保):
  strict 経路(`route resolve --command …`)は同じ workspace
  で `INVALID_RUNTIME_POLICY`・exit 3 を返し fail-open
  しない — 両経路の非対称を pin する(strict 側は T1 の
  `resolve_project_trusted_rejected` が既に cover するため、
  本 test では hook firing 側の観測に絞ってよい)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/trusted-route-flag-issuer-redesign.md`
  (PLAN-034L59uXskRD4ink3h2a13 — T2)
- `docs/designs/trusted-route-flag-issuer-redesign.md`
  (DESIGN-034L4tqYwtc8q88WEIXPh9 — §(g) error 契約表・
  Verification Notes「hook fail-open」項が AUTHORITATIVE)
- `docs/specs/trusted-route-flag-issuer-redesign.md`
  (SPEC-034LwdoInh3ie1KPlSjkJl — R6 hook 経路 fail-open)
- `docs/test-specs/trusted-route-flag-issuer-redesign.md`
  (TSPEC-034L59Ghpuk8XWUN8qgXJu — hook 経路 fail-open Rule)
- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`
  (TASK-034L59KS9oEjgDzshYZUSn — 前提: 層 gate が存在して
  初めて検証対象になる)

## Dependencies

- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`
  (T1) — project 層 `trusted` presence 拒否 gate・
  `load_or_none` fail-open 経路が T1 で着地していること。
  T1 未着手では project 層 `trusted` が受理され route arm が
  発火して test が成立しない。

## Affected Files

- `crates/cheaper-cli/tests/integration/hook.rs`(modify —
  test 追加のみ。route supply 節 :848 以降)

## Done When

- [ ] `tests/integration/hook.rs` に、project 層
      `<workspace>/.cheaper/routes.yaml` の `trusted` presence
      を含む load 失敗が `load_or_none` 経由で全層 route arm
      を不活性化し、routed だった command が unrouted eval
      経路(dedup record publish 側)で処理され続けることを
      pin する回帰 test が存在する
- [ ] 違反 file が project 層に置かれている(user 層 `f.cfg`
      への `trusted` 記述ではない — user 層受理で違反になら
      ないため)
- [ ] `cargo test -p cheaper-cli --test integration hook` が
      pass する

## Out of Scope

- production code(`route/supply.rs`・`hook/*` 等)の変更 —
  fail-open 規約は不変、pin のみ
- routes.rs・resolve.rs の code/test 変更(T1 済み)
- strict 経路(`route resolve`/`route run`)の presence 拒否
  test(T1 の `resolve_project_trusted_rejected` が担当)
- docs 更新(T3)
- `load_or_none` の fail-open → fail-closed 変更(規約は維持
  — R6)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook` が pass
- [ ] 新規 test が T1 前の code では fail する(層 gate 不在で
      route arm が発火する)ことを確認 — または test が
      fail-open 経路の pin として構造的に正しいことを
      review で確認

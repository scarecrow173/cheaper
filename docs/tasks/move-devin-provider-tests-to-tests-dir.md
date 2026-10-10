---
id: "TASK-034DzYjQDNWcel1Oqxnlia"
type: "task"
status: "todo"
title: "T3: provider test 移動 (F7)"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  blocks:
    - "docs/tasks/rename-provider-crate-and-modularize-devin.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T3: provider test 移動 (F7)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T3 stream(F7)。
`cheaper-provider-devin` の in-src `src/tests/` を crate 標準の統合
test 配置 `tests/` へ移動し、crate 内部 `crate::` 参照を
`cheaper_provider_devin::` public API 経由へ調整する。移動時に必要な
最小限の `pub` 化は intentional surface change として本 task で
記録・実施する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T3: provider test 移動(F7)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F7 stream の
  acceptance criteria(「全テストが `tests/` 配下で pass する」)
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  target layout

## Dependencies

- task 依存なし(plan の Dependencies table で T3 は独立)
- `docs/tasks/rename-provider-crate-and-modularize-devin.md`(T7)を
  block する — rename 前に test 配置を `tests/` へ確定させる必要がある

## Affected Files

- `crates/cheaper-provider-devin/src/tests/*.rs` —
  `crates/cheaper-provider-devin/tests/` へ移動
- `crates/cheaper-provider-devin/src/lib.rs` — `mod tests;` /
  `#[cfg(test)]` 宣言除去
- `crates/cheaper-provider-devin/src/tests/mod.rs` — 分割して各 test
  file へ統合 test 化(shared helper は `tests/common/` または各 file
  内へ)
- 移動先の test は `cheaper_provider_devin::` の public API 経由に調整
  (crate 内部 `crate::` 参照は統合 test では使えない)
- visibility 決定(確定): 移動対象 test が `src/tests/` から参照して
  いる `pub(crate)` item は `jwt_expires_at`(src/jwt.rs:73)・
  `chat_status_message`/`chat_send_error`/`resolve_host`
  (src/rig_provider.rs:365/374/460)。解決方針は move 時に最小限の
  `pub` 化(intentional surface change として task-doc に記録)を
  基本とし、spec AC「全テストが `tests/` 配下で pass する」と整合
  させる。移動時に `pub(crate)` grep sweep を行い、追加の private
  参照が見つかった場合も同方針で処理する

## Done When

- [ ] `src/tests/` が `tests/` へ移動し、`src/lib.rs` の
      `mod tests;`/`#[cfg(test)]` 宣言が除去されている
- [ ] 移動後の全 test が `cheaper_provider_devin::` public API 経由で
      pass する
- [ ] private 参照の解決に実施した `pub` 化が最小限に留まり、
      intentional surface change として本 task-doc(または commit
      message)に記録されている

## Out of Scope

- crate 名の変更・`src/devin/` module 化 — T7 の範囲。本 task では
  `cheaper-provider-devin` 名のまま test 配置のみを変更する

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-provider-devin` exits 0
      (全テストが `tests/` 配下で pass)
- [ ] `test ! -d crates/cheaper-provider-devin/src/tests` exits 0
      (in-src test dir 不在)
- [ ] `git grep -n "mod tests" -- crates/cheaper-provider-devin/src/`
      が 0 件を返す(exit 1)
- [ ] `git grep -n "pub(crate)" -- crates/cheaper-provider-devin/src/`
      の sweep 結果が既知の対象(`jwt_expires_at`・
      `chat_status_message`・`chat_send_error`・`resolve_host`)以外の
      private 参照を残していないことを確認する

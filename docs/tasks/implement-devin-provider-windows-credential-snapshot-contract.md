---
id: "TASK-034Dak5Arnf42bv3QpLooy"
type: "task"
status: "done"
title: "Implement Devin provider Windows credential snapshot contract"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
  implemented-by:
    - "docs/impl/ir/devin-provider-windows-credential-snapshot-sync.md"
  depends-on:
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
    - "docs/tasks/extend-pluginauth-logout-with-authinteraction.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/devin-provider-windows-credential-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement Devin provider Windows credential snapshot contract

## Work

`cheaper-provider-devin` に Windows credential contract（global
toml の atomic copy・key 一致 reuse・logout 非 spawn + notify）
を実装する（issue #80）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-provider-windows-credential-snapshot-sync.md`
- `docs/designs/windows-credential-snapshot-sync-for-devin-provider.md`
- `docs/specs/devin-provider-windows-credential-contract.md`
- `docs/test-specs/devin-provider-windows-credential-contract.md`

## Dependencies

- `docs/tasks/extend-pluginauth-logout-with-authinteraction.md`
  （`PluginAuth::logout` signature が前提）

## Affected Files

- `crates/cheaper-provider-devin/src/credentials.rs`
- `crates/cheaper-provider-devin/src/plugin.rs`
- `crates/cheaper-provider-devin/src/tests/cli_credentials.rs`
- `docs/guides/providers.md`・`docs/guides/configuration.md`

## Done When

- [x] `read_credentials_at(path)` が抽出され
      `resolve_global_credentials_path`（`File:` parse → APPDATA
      fallback）が実装される
- [x] `login_with_devin_cli` の Windows 分岐: `devin_auth_status`
      非呼出し・key 一致 reuse・不一致 notify・exit-0 gate・
      atomic copy・copy 後 parse 検証
- [x] `logout_devin_session` の Windows 分岐: spawn せず toml
      削除 + notify
- [x] `DevinPluginAuth.windows` field・`with_windows` chainable
      builder・login notify の spawn site 移動
- [x] Windows contract test 群（plan sequence 5 の a-h）が追加
      され pass
- [x] guide doc 2 件に Windows carve-out を追記

## Verification

- [x] `mise exec -- cargo test -p cheaper-provider-devin` pass
- [x] `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli`
      pass
- [x] `mise run check` pass

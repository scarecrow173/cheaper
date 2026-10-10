---
id: "TASK-034Dak56U8haKyVTWPnBKv"
type: "task"
status: "done"
title: "Extend PluginAuth::logout with AuthInteraction"
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
  blocks:
    - "docs/tasks/implement-devin-provider-windows-credential-snapshot-contract.md"
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

# Extend PluginAuth::logout with AuthInteraction

## Work

`PluginAuth::logout` の signature に `&mut dyn AuthInteraction` を
追加し、plugin hook が logout 時に利用者へ通知できる channel を
用意する（issue #80 の Windows logout 告知の前提）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-provider-windows-credential-snapshot-sync.md`
- `docs/designs/windows-credential-snapshot-sync-for-devin-provider.md`
- `docs/specs/devin-provider-windows-credential-contract.md`

## Dependencies

- なし（後続 task `implement-devin-provider-windows-credential-snapshot-contract`
  の先行依存）

## Affected Files

- `crates/cheaper-runtime/src/plugin.rs`（trait + default impl）
- `crates/cheaper-runtime/src/auth/service.rs`（`hook.logout` 3
  呼出し :566,580,587 + `logout`/`logout_all` signature）
- `crates/cheaper-cli/src/auth.rs`（`AuthAction::Logout` arm で
  `TerminalInteraction` 生成）
- `crates/cheaper-provider-devin/src/plugin.rs`（
  `DevinPluginAuth::logout` signature）
- `crates/cheaper-runtime/tests/plugin.rs`（`FakeAuth::logout`）
- `crates/cheaper-runtime/tests/auth.rs`・`tests/plugin.rs`・
  `tests/oauth.rs` の logout 呼出し（各 file 既存 stub:
  `ScriptedInteraction`/`NotifyOnly`/`NoopInteraction`）

## Done When

- [x] `PluginAuth::logout` が
  `fn logout(&self, ctx: &ProviderPluginContext, interaction: &mut dyn AuthInteraction) -> Result<(), CheaperError>`
  となり全 impl・呼出しが compile する
- [x] `AuthAction::Logout` が login arm と同様に
  `TerminalInteraction` を生成して `auth.logout` へ渡す
- [x] `cargo test --workspace` が pass する

## Verification

- [x] logout を override しない `FakePluginAuth`（default impl）
      が変更不要であることを compile で確認
- [x] `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli`
      が pass

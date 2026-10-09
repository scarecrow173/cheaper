---
id: "PLAN-034Da73SnpZLHkn4oVZu00"
type: "plan"
status: "approved"
title: "Devin provider Windows credential snapshot sync"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/80"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/devin-provider-windows-credential-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/windows-credential-snapshot-sync-for-devin-provider.md"
  derived-by: []
  verifies:
    - "docs/specs/devin-provider-windows-credential-contract.md"
  verified-by:
    - "docs/test-specs/devin-provider-windows-credential-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Devin provider Windows credential snapshot sync Implementation Plan

## Goal

issue #80: Windows で `devin auth login` が global toml に書く前提に
合わせ、copy-based snapshot・key 一致 reuse・logout 非破壊 + 告知
を provider・runtime・CLI の 3 層に実装する。

## Scope

1 つの implementation stream。`PluginAuth::logout` signature 拡張
（runtime 層）は devin plugin 側の logout notify の前提依存なので
同一 plan 内で順序付けて行う。

## Upstream Documents

- `docs/specs/devin-provider-windows-credential-contract.md`
- `docs/adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md`
- `docs/discovery/devin-provider-windows-credential-isolation-limitation.md`

## Design Inputs

- `docs/designs/windows-credential-snapshot-sync-for-devin-provider.md`
  （approved）

## File Map

- `crates/cheaper-runtime/src/plugin.rs` —
  `PluginAuth::logout` signature に `&mut dyn AuthInteraction` を
  追加（:105 default impl）
- `crates/cheaper-runtime/src/auth/service.rs` —
  `logout`/`logout_all` に interaction 引数を追加し `hook.logout`
  3 呼出し（:566,580,587）へ渡す
- `crates/cheaper-cli/src/auth.rs` —
  `AuthAction::Logout` arm（:587-595）で `TerminalInteraction` を
  生成して `auth.logout` へ渡す
- `crates/cheaper-provider-devin/src/credentials.rs` —
  `read_credentials_at(path)` 抽出・
  `resolve_global_credentials_path`（`File:` parse + APPDATA
  fallback）・`login_with_devin_cli` Windows 分岐（exit-0 gate・
  key 一致 reuse・mismatch notify・atomic copy）・
  `logout_devin_session` Windows 分岐（spawn せず notify）
- `crates/cheaper-provider-devin/src/plugin.rs` —
  `DevinPluginAuth` に `windows: bool` field・
  `DevinProviderPlugin::with_windows` chainable builder（`self`
  消費で `with_runner(r).with_windows(true)` と合成可能、
  fake runner との併用が test (h) の前提）・login notify を
  spawn site へ移動・logout impl の notify
- `crates/cheaper-provider-devin/src/tests/cli_credentials.rs` —
  Windows contract の test 群 + notify 記録用の
  `AuthInteraction` stub（crate 内に既存 stub なし。新規に
  notify 記録型を定義）+ 既存呼出し 5 件の signature 更新
  （`login_with_devin_cli` :213,262,282、
  `logout_devin_session` :301,312）
- `crates/cheaper-runtime/tests/plugin.rs` —
  `FakeAuth::logout` signature 更新
- `crates/cheaper-runtime/tests/auth.rs` `tests/plugin.rs`
  `tests/oauth.rs` — `svc.logout`/`logout_all` 呼出しへ
  interaction 注入（:448,452,467、:252,687、:817。各 test file
  は別 crate root のため `NoopInteraction` は共有不可 —
  auth.rs は `ScriptedInteraction`、oauth.rs は
  `ScriptedInteraction`/`NotifyOnly`、plugin.rs は
  `NoopInteraction` を使用）
- `docs/guides/providers.md`（:217-239 の devin auth/隔離/
  logout 記述に Windows contract の carve-out）・
  `docs/guides/configuration.md`（:145-150 同様）

## Implementation Sequence

- [ ] 1. `PluginAuth::logout` signature を
      `fn logout(&self, ctx: &ProviderPluginContext, interaction: &mut dyn AuthInteraction) -> Result<(), CheaperError>`
      に変更し、service.rs の 3 `hook.logout` 呼出し・CLI
      `AuthAction::Logout` arm（`TerminalInteraction` 生成）・
      `DevinPluginAuth::logout`・`FakeAuth`・全 test 呼出し
      （`NoopInteraction`）を更新して compile を通す
- [ ] 2. `credentials.rs`: `read_devin_credentials` から
      `read_credentials_at(path: &Path)` を抽出して共有化し、
      `resolve_global_credentials_path(cli, appdata)` と
      `File:` 行 parse helper を追加
- [ ] 3. `login_with_devin_cli` に `windows`・`appdata`・
      `interaction` 引数を追加。Windows: `devin_auth_status` は
      呼ばず（global session 判定は使わない。`auth status` の
      spawn は `File:` parse の 1 回のみ）、cheaper+global の
      key 一致で reuse、不一致（global parse 可能時のみ）で
      notify、`devin auth login` exit-0 gate、global_path
      再利用で temp+rename atomic copy
      （`.credentials.toml.tmp-<pid>-<nanos>`、失敗時 temp 削除）、
      copy 後 `read_credentials_at` 検証。
      非 Windows: 現行 contract 不変
- [ ] 4. `logout_devin_session` に `windows`・`interaction` 引数を
      追加。Windows: `devin auth logout` を spawn せず cheaper
      toml のみ削除 + global session 継続を notify。
      `DevinPluginAuth` に `windows` field・`with_windows`
      chainable builder、`ctx.env.env("APPDATA")` は login 経路
      のみで使用（logout には不要）、plugin.rs:98 の login
      notify を spawn site へ移動
- [ ] 5. test 追加: fake runner + `windows: true` 注入で
      （a）exit-0 copy・（b）非 0 不 copy+error・（c）key 一致
      reuse で login 非 spawn かつ `auth status` spawn は 1 回・
      （d）key 不一致で notify+login・（e）global 不在で
      login+copy・（f）logout が argv に `auth logout` を
      含まず toml 削除+notify・（g）`File:` 行 parse・APPDATA
      fallback・（h）`with_runner(..).with_windows(true)` 経由の
      plugin-level logout 検証。非 Windows 既存 test 非
      regression
- [ ] 6. `docs/guides/providers.md`・`configuration.md` の devin
      auth 記述に Windows contract（copy-based snapshot、logout
      非破壊）の carve-out を追加
- [ ] 7. `mise exec -- cargo test -p cheaper-provider-devin` →
      `cargo test -p cheaper-runtime -p cheaper-cli` →
      `mise run check`

## Dependencies and Risks

- sequence 1 の trait signature 変更が先行しないと 4 の notify が
  書けない（順序依存）。
- `devin auth status` 未到達時の APPDATA fallback は known-folder
  と乖離し得るが、reuse 判定が保守的（再 login）に倒れるため安全
  側。copy の atomic 性: `fs::rename` は Windows でも
  `MOVEFILE_REPLACE_EXISTING` で既存 target を置換（`store.rs` の
  remove-first は `persisted.rs:86-91` が複製禁止と明記する旧式）。
- Windows-gated 手動 smoke（`cheaper auth login devin` →
  `auth logout devin` で global session 継続）は CI 対象外。

## Task Breakdown

- `extend-pluginauth-logout-with-authinteraction`（runtime trait
  拡張）: sequence 1。runtime/cli の signature 変更と全呼出し更新。
- `implement-devin-provider-windows-credential-snapshot-contract`
  （Windows credential contract）: sequence 2-6。credentials.rs +
  plugin.rs + Windows test 群 + guide doc。
- 後者は前者に `depends-on`（signature が無いと compile しない）。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-provider-devin`（新規
  Windows contract test 群 pass）
- [ ] `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli`
  （trait signature 変更の非 regression）
- [ ] `mise run check` 全 pass
- [ ] `devin auth status`（実機 Windows）の `File:` 行 regex が
  logged-in 出力で parse できることを本 session で確認済み
  （`File:              C:\Users\...\credentials.toml` 実測）

## Open Questions / Gaps

- logged-out の `devin auth status` が `File:` 行を出すかは未
  検証（fallback が安全側に働くため仕様上は問題なし。実装後の
  手動 smoke で確認）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none（trait 拡張 task → Windows
  contract task の順序依存）
- User approval required before dispatch: no（review-protocol 承認済み
  をもって承認扱い）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- review-protocol で確認: P1 exit-code gate、logout notify channel、
  platform seam、二重 resolution、mismatch notify 条件、atomic
  copy convention（rename は置換可・remove-first 旧式を踏襲しない）。

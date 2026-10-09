---
id: "DESIGN-034D9Xs2f06fbVcxdT08le"
type: "design"
status: "approved"
title: "Windows credential snapshot sync for Devin provider"
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
    - "../specs/devin-provider-windows-credential-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Windows credential snapshot sync for Devin provider

## Context

Devin CLI 3000.x は Windows で credentials.toml を known-folder API
で `%APPDATA%\devin\credentials.toml` に固定解決し、env/flag で
redirect できない（issue #80、実測済み）。spec
（`devin-provider-windows-credential-contract`）と ADR
（copy-based snapshot）に従い、Windows では「global toml →
cheaper data dir への atomic copy」「key 一致による reuse 判定」
「`devin auth logout` 非呼出し + 告知」を実装する。

## Scope

### In Scope

- `crates/cheaper-provider-devin/src/credentials.rs`:
  login reuse 判定・global toml path 解決・atomic copy・logout
  分岐・`File:` 行 parse を Windows contract として実装する。
- `crates/cheaper-runtime/src/plugin.rs`: `PluginAuth::logout` に
  `&mut dyn AuthInteraction` を追加する signature 拡張。
- `crates/cheaper-runtime/src/auth/service.rs`:
  `logout`/`logout_all` が interaction を hook へ渡す。
- `crates/cheaper-cli/src/auth.rs`: `AuthAction::Logout` /
  `logout --all` 経路で `TerminalInteraction` を生成して渡す。
- `crates/cheaper-provider-devin/src/plugin.rs`: platform flag の
  保持と logout notify、login notify の reuse 経路での抑制。

### Out of Scope

- Devin CLI の修正、非 Windows contract の変更、runtime 経路の
  per-request freshness check、新規 crate 依存。

## Components and Boundaries

- **`credentials.rs`（provider 内の contract 実装層）**: 全ての
  Windows 固有判定を platform flag 引数で受ける pure な関数群。
  `cfg!(windows)` はここに書かない。
- **`plugin.rs`（dispatch 層）**: `DevinPluginAuth` が
  `windows: bool` field（default `cfg!(windows)`）を持ち、
  credentials.rs の関数へ渡す唯一の platform dispatch。注入
  seam は `DevinProviderPlugin` に `self` 消費の chainable
  `with_windows(bool)` builder を追加する
  （`with_runner(r).with_windows(true)` と合成可能 —
  plugin-level test は fake runner と併用するため chainable
  が必須。credentials.rs の pure 関数 test は引数で直接注入）。
- **APPDATA の参照**: `resolve_global_credentials_path` の env は
  別途保持せず、`login`/`logout` が受け取る `ctx.env.env("APPDATA")`
  （`ProviderEnv` accessor、`register` が `DEVIN_CLI` に使うのと
  同じ seam）から取得する。`DevinCli` の private `child_env` は
  複製しない。
- **`cheaper-runtime`（trait/service 層）**: `PluginAuth::logout`
  の signature 拡張のみ。provider 固有ロジックは入れない。
- **`auth.rs`（CLI 層）**: logout arm で login arm と同じ
  `TerminalInteraction` 生成を行う。

## Data and Control Flow

### login（Windows contract、plugin → `login_with_devin_cli`）

1. `cheaper = read_devin_credentials(data_dir)`。
2. `global_path = resolve_global_credentials_path(cli, appdata)`:
   - `cli.run(["auth","status"])` の stdout+stderr から
     `^\s*File:\s*(.+)$` を parse（複数行一致は最後を採用）。
     exit code 非 `0`（logged-out）でも出力に `File:` 行があれば
     parse する（logged-out 時に File: が出るかは未検証のため
     fallback が安全側に効く構成に留める）。
   - parse 失敗・CLI 未到達 → `appdata`（`ctx.env.env("APPDATA")`）
     から `devin/credentials.toml` を構成。それも無ければ `None`。
3. `global = global_path` があれば `read_credentials_at(global_path)`
   （`read_devin_credentials` から任意 path 版を抽出した共通 parse）。
4. `cheaper` と `global` が両方 parse でき `api_key` 一致 →
   cheaper を返して終了（`devin auth login` は spawn しない）。
5. `cheaper` が存在し `global` も parse 可能で `api_key` 不一致の
   場合のみ → `interaction.notify` で「cheaper credential は
   global session と異なる account — login で global session が
   更新されるか既存 global session が採用される」旨を告知
   （`global` 不在・parse 不可は「別 account」が確定しないため
   告知しない）。
6. `devin auth login`（inherit stdio）を実行。`code != 0` →
   error（copy しない）。実際に spawn する直前でのみ
   「login を起動する」notify を発火させる（reuse 経路では出さ
   ない。`plugin.rs:98` の既存 notify を spawn site へ移動 —
   非 Windows でも reuse 時に誤誘導の通知が出なくなるのは
   spec Req に基づく意図的な変更で、byte 単位の stderr parity
   ではなく contract parity を優先する）。
7. 手順 2 で解決済みの `global_path` を再利用し、global toml の
   存在を確認（`devin auth status` の 2 度目 spawn はしない）。
   不在・再解決失敗 → actionable error（login は global 側で
   成功しているため、global path と期待 path を error に含める）。
8. `create_dir_all(data_dir/devin)` → global toml を同 dir 内の
   一意 temp file（`.credentials.toml.tmp-<pid>-<nanos>`、
   `catalog/persisted.rs` の convention）へ `fs::copy` →
   `fs::rename` で atomic 配置（Rust の `fs::rename` は
   Windows でも `MOVEFILE_REPLACE_EXISTING` で既存 target を
   置換。`store.rs` の remove-first workaround は
   `persisted.rs:86-91` が複製しないよう明記している旧式であり
   踏襲しない）。失敗時は temp を best-effort 削除 →
   `read_credentials_at` で検証（`api_key` 無ければ error）。

### login（非 Windows）

現行 contract 不変: `devin_auth_status`（CLI authoritative +
toml fallback）で reuse 判定 → `devin auth login` → cheaper
toml 再読。

### logout（Windows contract）

`devin auth logout` を spawn せず、cheaper toml を
`fs::remove_file`（best-effort）し、`interaction.notify` で
「global session は残る。完全 logout は `devin auth logout` を
手動実行」旨を告知する。非 Windows は現行どおり `devin auth
logout` + toml 削除。

### logout 呼出経路の signature

```text
auth.rs Logout arm
  → TerminalInteraction::new(terminal, None, Some(signal), open_url)
  → AuthService::logout(catalog, provider, &mut interaction)
    → PluginAuth::logout(ctx, &mut interaction)
      → DevinPluginAuth::logout → logout_devin_session(...)
```

`logout_all` も同一 interaction を各 hook に渡す。signature 変更
の更新対象:

- `PluginAuth::logout` trait（default impl、`plugin.rs:105`）
- override impl: `DevinPluginAuth`（provider `plugin.rs:155`）、
  `FakeAuth`（`crates/cheaper-runtime/tests/plugin.rs:72`）
  （`FakePluginAuth` は logout を override していないため変更不要）
- `hook.logout` 呼出し 3 箇所（`service.rs:566,580,587`）
- `AuthService::logout`/`logout_all` の signature
- 呼出側: CLI は `auth.rs:589` の `AuthAction::Logout` arm 1 箇所
  のみ（`logout --all` の CLI 経路は存在せず、`logout_all` は
  test-only API）。test 呼出し `tests/auth.rs:448,452,467`、
  `tests/plugin.rs:252,687`、`tests/oauth.rs:817` には各 file の
  既存 stub を渡す（`NoopInteraction` は plugin.rs 内のみ有効 —
  auth.rs は `ScriptedInteraction`、oauth.rs は
  `ScriptedInteraction`/`NotifyOnly`、または
  `fixture::ScriptedAuthInteraction`）。

## Non-Functional Constraints

- 新規 crate 依存なし（global path は `devin auth status` の
  `File:` 行 + `%APPDATA%` fallback で解決）。
- copy は temp + rename で atomic。`fs::copy` は Windows で source
  security descriptor を引き継ぎ permission を保持する。
- Windows contract の全単体 test は platform flag 注入で Linux CI
  から実行可能にする。
- `env`（APPDATA 参照用）も関数引数で注入し、test が process env
  を変更しない形にする。

## Risks and Trade-offs

- `devin auth status` 未到達時は `%APPDATA%` env 経由の fallback
  で known-folder と乖離し得る → その場合 reuse 判定は保守的に
  不一致側（再 login）に倒れるため fail 方向は安全。
- `devin auth login` exit `0` だが global toml が更新されなかった
  場合（既 login の short-circuit）→ copy は「現在の global
  account」の snapshot を取る。別 account 採用は 5 の告知で
  カバー済み（一致時は新規 login 自体が走らない）。
- 非 Windows の既知 latent 問題（login abort 後の stale toml
  reuse）は本 scope では変更しない（spec で contract 不変と明記）。
- `AuthService::logout`/`logout_all` の signature 変更は public
  API の breaking change に見えるが runtime 内部の plugin hook
  経路であり、consumer 面の contract（CLI envelope
  `{provider, loggedOut}`）は不変。同様に `login_with_devin_cli`・
  `logout_devin_session` 等の `lib.rs` re-export 関数も引数追加で
  crate 内 public API が変わるが、workspace 内 consumer は
  `plugin.rs` のみで影響は閉じる。

## Deferred Design Concerns

- upstream が Windows で env redirect をサポートした場合、
  `devin_child_env` の注入は残っているため copy path が自然に
  dormant 化する。再検討 trigger: Devin CLI release note での
  Windows credential dir override。
- `api_server_url` 等の key 以外 field の一致比較は行わない
  （key が同じ account で endpoint だけ変わる実害ケースが稀）。

## Verification Notes

- [ ] fake runner + platform flag 注入で: exit `0` copy / 非 `0`
  不 copy / key 一致 reuse / 不一致 notify+再 login / global 不在
  再 login / logout 非 spawn+notify / `File:` 行 parse・
  `%APPDATA%` fallback が検証できる
- [ ] `cargo test --workspace` で trait signature 変更の非
  regression（FakeAuth 等の impl 更新含む）

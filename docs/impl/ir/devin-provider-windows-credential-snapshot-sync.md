---
id: "IMPL-034DaJCOheEsDZAq8GWBXw"
type: "impl"
status: "completed"
title: "Devin provider Windows credential snapshot sync"
created: "2026-10-09"
updated: "2026-10-17"
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
    - "docs/tasks/implement-devin-provider-windows-credential-snapshot-contract.md"
    - "docs/tasks/extend-pluginauth-logout-with-authinteraction.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
    - "docs/designs/windows-credential-snapshot-sync-for-devin-provider.md"
    - "docs/specs/devin-provider-windows-credential-contract.md"
    - "docs/adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Devin provider Windows credential snapshot sync

## Summary

issue #80 の Windows credential contract を実装した。Devin CLI
3000.x が Windows で env による credential path 変更を無視して
global `%APPDATA%\devin\credentials.toml` に書く制約の下、
login は global toml を atomic copy で cheaper data dir へ
snapshot し、logout は `devin auth logout` を spawn せず
cheaper snapshot のみ削除して global session を保持する。
`PluginAuth::logout` に `AuthInteraction` を追加して logout 時の
告知 channel を用意した。

## Context

task `implement-devin-provider-windows-credential-snapshot-contract`
と先行 task `extend-pluginauth-logout-with-authinteraction`、plan
`devin-provider-windows-credential-snapshot-sync`、approved design
`windows-credential-snapshot-sync-for-devin-provider` に従う。

## Implementation

### Changes

- `cheaper-runtime`: `PluginAuth::logout` に
  `&mut dyn AuthInteraction` を追加。`AuthService::logout`/
  `logout_all` が interaction を hook 3 呼出しへ渡す。
- `cheaper-cli`: `AuthAction::Logout` arm で login arm と同じ
  `TerminalInteraction` を生成して `auth.logout` へ渡す。
- `cheaper-provider-devin/credentials.rs`:
  - `read_devin_credentials` から `read_credentials_at(path)` を
    抽出（global toml の parse に共用）。
  - `resolve_global_credentials_path` — `devin auth status` の
    `File:` 行（最後の一致、`(?m)`）→ APPDATA fallback。
  - `copy_credentials_atomic` — 同 dir 一意 temp
    （`.credentials.toml.tmp-<pid>-<nanos>`）+ `fs::rename`。
  - `login_with_devin_cli` に `windows`/`appdata`/`interaction`
    引数。Windows 分岐 `login_windows_snapshot` — key 一致
    reuse・不一致のみ notify・exit-0 gate・global_path 再利用
    （`auth status` spawn は 1 回のみ）・copy 後 parse 検証。
  - `logout_devin_session` に `windows`/`interaction` 引数。
    Windows は spawn せず snapshot 削除 + notify。
- `cheaper-provider-devin/plugin.rs`: `DevinPluginAuth.windows`
  field・`with_windows(bool)` chainable builder
  （`with_runner(r).with_windows(true)` で合成可能）・
  `ctx.env.env("APPDATA")` を login 経路へ・login notify を
  spawn site へ移動。
- 両 guide doc に Windows carve-out を追記。

### Decisions During Implementation

- login 起動 notify を plugin.rs から credentials.rs の spawn
  直前へ移動 — reuse 経路で誤告知しない spec contract の実現に
  必要（非 Windows でも同じ動作）。
- `File:` 行 regex は `(?m)^\s*File:\s*(.+?)\s*$` —
  `captures_iter().last()` で最後の一致を採用（design どおり）。
- review 指摘を反映し、`File:` 行の値が空・basename が
  `credentials.toml` 以外の場合は採用せず APPDATA fallback へ回す
  （bogus な `File:` が fallback を抑止したり別 file を snapshot
  するのを防ぐ）。APPDATA も空白のみなら未設定扱い。
- `read_credentials_at` で UTF-8 BOM を strip — Windows で手編集
  された toml が parse 不能になるのを防ぐ。
- snapshot copy の io error に src/dest path を含めて actionable
  にした。
- 採用しなかった指摘: cheaper credential 不在 + global parse 可能
  時の「global session を採用し得る」notify — approved design が
  notify 条件を「両方 parse + key 不一致」に限定しており、
  cheaper 不在は Windows での通常 login 系で毎回警告は noise
  になるため（review で reject 判定を記録）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-provider-devin` — 74 pass
  （Windows contract test 12 件 + plugin-level test 1 件含む）。
- `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli` —
  全 suite 0 failed。
- `mise run check` — biome / fmt / clippy `-D warnings` /
  version / doc-id / `cargo test --workspace` / node --test /
  bundle 再生成 / payload / freshness 全 pass（Windows host、
  review 修正後に再実行）。
- review-protocol: 独立 reviewer 2 名（協調 + adversarial）。
  P0 なし。CONFIRMED 反映: bogus `File:` 値の fallback 抑止
  （P1）・BOM parse（P2）・reuse 回帰 test gap（P2）・
  appdata 空白検査・copy error path 文脈・reuse notify 非検証・
  doc drift（P3 群）。REJECTED: cheaper 不在時の global adopt
  notify（design の notify 条件と矛盾・通常経路で noise）。
- Windows 実機 smoke（devin 3000.11.3、実装 binary）:
  - `cheaper auth login devin` → cheaper/global key 一致で reuse、
    非対話 `authenticated: true`。
  - `cheaper auth logout devin` → 「global session は残る」notify
    出力、snapshot 削除、`devin auth logout` 非 spawn。
  - logout 後の `devin auth status` → `Logged in (via Devin)` の
    まま global `credentials.toml` 温存 — issue #80 の global
    session 破壊が解消されたことを実機確認。

## Risks

- `File:` 行の出現・形式は logged-in `devin auth status` で実測
  済み。logged-out 時に出ない場合は APPDATA fallback が安全側に
  働く（reuse 判定は保守的に再 login 側へ倒れる）。
- upstream が将来 Windows env redirect をサポートした場合、copy
  path は dormant 化する設計（design Deferred Concerns どおり）。

## Follow-ups

- issue #80 へ検証証跡を comment して close。

## 2026-10-17 Follow-up: resolver fixture の Linux 可搬性

- 既知の `resolve_global_path_*` 3件を Linux で限定再実行し、3 failed を再現した。固定 Windows path の backslash が Linux の `file_name` では区切りにならず、`join` も native `/` を生成するため、fixture と期待値が契約に合っていなかった。
- 3件の fixture を、既存の Windows workflow 注入テストと同じ TempDir + native `PathBuf::join` に揃えた。複数 File 行・CRLF・空白・File 優先度・bogus 行拒否・APPDATA None/空白・CLI 未到達の assertion は維持した。
- `resolve_global_path_parses_stderr_when_logged_out` を追加し、code `1` の stderr にある有効 File 行を APPDATA 無しでも採用する既存契約を pin した。実 Devin CLI・実 credential にはアクセスしていない。
- 変更された Rust file は `crates/cheaper-provider/tests/cli_credentials.rs` のみ。production source・依存・lockfile・CI 設定は不変。
- Windows host と Linux 隔離環境で、`cargo test --locked --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test cli_credentials` はともに24 passed /0 failed /0 ignored。Linux では同じ command に `--offline` も付けた。
- 両 OS で同 feature/target 選択の `cargo clippy -- -D warnings` と `cargo fmt --all -- --check` が成功し、host `git diff --check` も成功した。今回の follow-up では workspace 全テスト・`mise run check`・実認証 smoke・CI は再実行していない。
- Review: native/foreign path の責任境界、元の3件の意味、追加例、cfg/ignore/serial を増やしていないことを確認し、未解決 blocker なし。過去の spec/status・crate 名などの一括整理は別扱いとした。

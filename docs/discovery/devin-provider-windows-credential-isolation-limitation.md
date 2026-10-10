---
id: "DISC-034D9AL1up7opdx1NuFeir"
type: "discovery"
status: "draft"
title: "Devin provider Windows credential isolation limitation"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/devin-provider-windows-credential-contract.md"
    - "../adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Devin provider Windows credential isolation limitation

## Exploration Goal

GitHub issue #80。`cheaper-provider-devin` の Windows isolated env
（`devin_child_env` が `XDG_DATA_HOME` + `LOCALAPPDATA` を child
process に注入）が Devin CLI 3000.x で機能せず、credential が global
`%APPDATA%\devin\credentials.toml` に読み書きされる。Windows での
正しい auth contract（login・status・logout・catalog）を決める。

Entry Decision: A-4 Direct Start — issue に問題が明記されており、
実測で原因を確定済み。残る判断は「Windows での credential 取り回し
contract をどう設計するか」のみ。

## Key Issues and Assumptions

- **confirmed（実測、devin 3000.11.3 / Windows）**:
  `XDG_DATA_HOME`・`LOCALAPPDATA`・`APPDATA` を全て override しても
  `devin auth status` は
  `C:\Users\<user>\AppData\Roaming\devin\credentials.toml` を報告する。
  credentials 解決は OS known-folder API（RoamingAppData）で env を
  見ない。
- **confirmed（実測）**: `USERPROFILE` override は
  `Error: Failed to determine config directory` で CLI 自体が起動
  失敗する。config dir 解決は USERPROFILE 系を使い、credentials 解決
  は known-folder API という混合解決。
- **confirmed（実測）**: `--config <PATH>` flag は config.json
  （`~/.config/devin/config.json`）のみ override し、
  credentials.toml は redirect しない。
- **confirmed（binary 文字列調査）**: `DEVIN_HOME` /
  `DEVIN_DATA_DIR` / `DEVIN_CONFIG_DIR` / `DEVIN_CREDENTIAL*` 系の
  redirect env は binary に存在しない。存在する env は
  `DEVIN_API_URL`・`DEVIN_MODEL`・`DEVIN_PERMISSION_MODE` 等の
  動作系のみ。
- **confirmed（code 調査）**: `logout_devin_session`
  （`crates/cheaper-provider-devin/src/credentials.rs:170`）は
  `devin auth logout` を無条件実行 → Windows では global session
  を破壊する副作用。
- **confirmed（code 調査）**: cheaper の Devin API 利用は CLI 経由
  ではなく credentials.toml の `api_key` を `rig_provider` で使う
  HTTP 経路。CLI の用途は `auth login/status/logout`・
  `models list`・`version` のみ。
- **assumption**: Linux/macOS では `XDG_DATA_HOME` が依然機能する
  （binary に `XDG_DATA_HOME`/`XDG_CONFIG_HOME` 文字列あり、
  変更しない前提。CI が Ubuntu のため非 Windows 経路の regression
  test で担保）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: login 時に global toml → cheaper dir へ copy + logout は cheaper copy のみ削除 | cheaper-owned credential snapshot を維持。global session を破壊しない。`read_devin_credentials`・`resolve_auth_material`・`refresh` の既存経路がそのまま使える | copy が stale 化しうる（後述の key 一致 check で緩和）。Windows で「真の隔離」は依然不可能 | yes |
| B: Windows では global toml を直接読む | 実装が最小 | 「global credential を暗黙利用しない」contract（ADR-034r05iHrZUE6WqcIaH1CU 系の設計意図）を破る。global session の account 切替にそのまま追随してしまう | no |
| C: Windows は login 不可とし manual copy のみ案内（現状維持 + logout 修正のみ） | 変更最小。安全側 | UX が悪い（毎回手動 copy）。`auth status` の login-reuse 判定が global session を見る問題は残る | no |
| D: Devin CLI 側の修正を待つ | 根本解決 | upstream の timeline 不明。issue は現行 3000.x で発生中 | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: Windows では env/flag による credential dir 隔離は
  原理的に不可能（実測 3 手法全て失敗 + redirect env 不存在）。
- **hypothesis（→ Option A として採用方向）**: Windows の contract
  を「login は `devin auth login`（global 書込）→ global toml を
  cheaper data dir へ copy」「reuse は cheaper toml 存在 +
  global toml の `api_key` 一致で判定（account 切替検出）」
  「logout は cheaper copy のみ削除し `devin auth logout` を
  呼ばない + 利用者へ告知」と定義すれば、隔離の実効部分
  （cheaper が使う key は cheaper 管理領域の snapshot）と安全
  （global session 非破壊）を両立できる。
- **hypothesis**: `devin_auth_status`（login-reuse の stale 判定）
  は Windows では CLI status が global session を見るため、cheaper
  toml の有効性判定としては global toml との key 比較が正しい。
- **hypothesis**: `devin models list` は global creds で動くが、
  model catalog 取得は account 非依存で実害なし（API call 自体は
  cheaper toml の key を使う）。

## Open Questions

- global credential path をどう解決するか: `%APPDATA%` env 参照か
  `dirs`/`known-folders` 系 crate か（`cheaper-provider-devin` の
  既存依存を確認して決める）。
- login 後 copy が失敗した場合（global toml 不在）の error
  message をどうするか。
- Windows で `devin auth logout` を呼ばない場合、利用者への
  global session 継続の告知方法（`interaction.notify` 等）。

## Promotion Candidates

- [x] spec-doc needed: Windows での auth contract（login copy・
  reuse 判定・logout 非破壊・告知）を受入基準として明文化
- [x] adr-doc needed: 「Windows 隔離不可能」の決定と Option A
  （copy-based snapshot + global 非破壊）の採用根拠を記録

## Sources and Relations

- GitHub issue #80
- 実測ログ: `devin auth status`（3000.11.3）が env override
  3 種・`--config` 全てで
  `C:\Users\akiyoshi\AppData\Roaming\devin\credentials.toml` を
  報告、`USERPROFILE` override で `Failed to determine config
  directory`
- `crates/cheaper-provider-devin/src/cli.rs:168`
  （`devin_child_env`）、`credentials.rs:95,140,170`、
  `plugin.rs:92-159`
- ADR-034r05iHrZUE6WqcIaH1CU（devin bundled plugin + data dir
  隔離の原設計）

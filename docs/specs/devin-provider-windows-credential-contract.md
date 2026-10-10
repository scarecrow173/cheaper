---
id: "SPEC-034D9BFJZdSSYBEuXVzBbl"
type: "spec"
status: "proposed"
title: "Devin provider Windows credential contract"
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
  related:
    - "../adr/windows-credential-isolation-fallback-via-copy-based-snapshot.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/devin-provider-windows-credential-isolation-limitation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Devin provider Windows credential contract

## What Should Be Built

`cheaper-provider-devin` の Windows 向け credential contract。Devin
CLI 3000.x が known-folder API で global
`%APPDATA%\devin\credentials.toml` に固定解決するため、env 注入に
よる真の隔離が不可能であることを前提に、login・reuse 判定・
logout・catalog 各経路の Windows 固有の振る舞いを定義する。

## Why Now

GitHub issue #80。現行実装は (1) `devin auth login` が global に
書くため cheaper 側 copy が生成されず login がエラー終了、(2)
`logout_devin_session` が `devin auth logout` を無条件実行し利用者
の global session を破壊、(3) `devin auth status` が global
session を見るため login-reuse 判定が cheaper state を反映しない、
の 3 問題を抱える。

## Users and Value

Windows 上で `cheaper auth login devin` を使う利用者。login が完走
し、`cheaper auth logout devin` が自分の Devin CLI global session
を破壊しないことを保証される。

## Scope

### In Scope

- Windows での login flow: `devin auth login` が exit `0` で
  終了した場合のみ、global credentials.toml を cheaper data dir
  （`<userConfigDir>/providers/devin/data/devin/credentials.toml`）
  へ atomic copy（temp + rename）する。copy 後は
  `read_devin_credentials` で parse 検証する。
- Windows での login-reuse 判定: cheaper toml が存在し、かつ
  global toml が parse 可能で `api_key` が一致する場合のみ既存
  credential を再利用する（account 切替・global logout 後の
  stale copy を検出）。key 不一致で再 login に進む場合、global
  session が別 account のものを置き換えることを
  `interaction.notify` で事前告知する。
- Windows での logout: cheaper-owned credentials.toml のみ削除し
  `devin auth logout` を実行しない。global session が残りうる
  ことを `AuthInteraction` 経由で利用者へ告知する
  （`PluginAuth::logout` の signature に interaction を追加する
  contract 変更を伴う）。
- Windows での global toml path 解決: `devin auth status` 出力の
  `File:` 行を parse して CLI 自身の known-folder 解決に追随し、
  CLI 未到達・parse 失敗時のみ `%APPDATA%\devin\credentials.toml`
  に fallback する。
- 非 Windows 経路の不変性（`XDG_DATA_HOME` 隔離は維持）。

### Out of Scope

- Devin CLI 自体の修正（upstream の責務）。
- Linux/macOS での挙動変更。
- global credential への「暗黙利用」（参照するのは global toml
  path の特定と freshness 判定としての key 一致確認のみ。API key
  供給源は常に cheaper toml）。
- `devin models list`・`devin version` の env 調整（global creds
  で動作するが model catalog は account 非依存、version は
  credential 不要のため実害なし）。
- runtime 経路（`resolve_auth_material`・`to_auth`）での per-request
  freshness check: snapshot semantics として cheaper toml の key を
  pin するのは意図的な分離であり、stale key は API auth failure と
  して顕在化し次回 `cheaper auth login` で再同期する。

## Requirements

- Windows/non-Windows 判定は `cfg!(windows)` を 1 箇所の dispatch
  に限定し、分岐先の関数は platform を引数/seam として受け取る
  形にする。Windows contract の全 AC は Linux CI の unit test で
  検証可能でなければならない。
- copy は `devin auth login` の exit code が `0` の場合のみ実行
  する。exit 非 `0`・global toml 不在・copy 失敗・copy 後 parse
  失敗はいずれも actionable error を返し silent success しない
  （abort した login が既存 global toml を snapshot することを
  防ぐ）。
- copy は destination dir の `create_dir_all` + temp file + rename
  で atomic に行い、source の permission を保持する
  （`std::fs::copy` は Windows で source security descriptor を
  引き継ぐ）。
- `api_key` 一致判定は `read_devin_credentials` と同じ parse 経路
  を通し、両 file が parse 可能な場合のみ比較する。
- Windows では `devin auth status` を global toml path 特定の
  ために呼ぶ（`File:` 行 parse）。logged-in 判定の authoritative
  には使わず、reuse 判定は file 上の key 一致のみで行う。
- Windows で `devin auth logout` を呼ぶコード経路を残さない
  （global session 保護を構造で担保）。
- `PluginAuth::logout` は `AuthInteraction` を受け取れるよう
  signature を拡張し、Windows での global session 継続を
  `interaction.notify` で告知する。
- 「`devin auth login` を起動する」旨の `interaction.notify` は実際に
  login を spawn する経路でのみ発火させ、reuse 経路では誤誘導の
  通知を出さない。
- 非 Windows では既存 contract を一切変更しない。

## Acceptance Criteria

- [ ] Windows contract（platform seam 経由）: `devin auth login`
  が exit `0` で終了した場合、
  `<userConfigDir>/providers/devin/data/devin/credentials.toml`
  が生成され、その `api_key` が global toml と一致する。
- [ ] `devin auth login` が非 `0` で終了した場合、既存 global
  toml は copy されず error が返る。
- [ ] cheaper toml が存在し global toml の `api_key` と一致する
  場合、login は `devin auth login` を再実行せず既存 credential
  を返す。
- [ ] global toml が不存在・`api_key` 不一致の場合、login は
  `devin auth login` を実行し新しい global toml を copy する。
  key 不一致の場合は再 login 前に global session 置換の告知が
  `interaction.notify` で行われる。
- [ ] Windows contract の logout は cheaper toml を削除し、
  `devin auth logout` を spawn しない（fake runner の argv 記録
  で検証）。global session 継続が `interaction.notify` で告知
  される。
- [ ] global toml path は `devin auth status` の `File:` 行から
  解決し、CLI 未到達時は `%APPDATA%` に fallback する。
- [ ] 非 Windows では従来どおり `devin auth logout` を実行し、
  `XDG_DATA_HOME` 隔離 env を child に注入する（既存 test 非
  regression）。
- [ ] copy は atomic（temp + rename）で、copy 後に parse 検証が
  行われる。

## Deferred Design Concerns

- Devin CLI が将来 Windows でも env redirect をサポートした場合の
  復帰: cheaper 側は `devin_child_env` の注入を残すため、upstream
  が env を尊重し始めれば copy path が自然に dormant になる。
  再検討 trigger: Devin CLI の release note で Windows credential
  dir override が announce されたとき。
- global session との「双方向同期」（global 側 login を cheaper
  が検出して自動 refresh）: 現 contract は key 一致を reuse 条件に
  留め、自動追従は行わない。

## Sources and Relations

- `docs/discovery/devin-provider-windows-credential-isolation-limitation.md`
  （実測根拠: env override 3 種・`--config` 全て無効、
  `USERPROFILE` override で CLI 起動失敗、redirect env 不存在）
- GitHub issue #80
- ADR-034r05iHrZUE6WqcIaH1CU（devin bundled plugin + data dir
  隔離の原設計 — 本 spec は Windows 限定の fallback contract と
  して補完する）

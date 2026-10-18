---
id: "TSPEC-034BgubID9xC2jMbCQ0zEG"
type: "test-spec"
status: "proposed"
title: "rust-migration-auth-catalog"
created: "2026-10-06"
updated: "2026-10-06"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# rust-migration-auth-catalog

## Purpose

`auth login/status/logout`・model catalog・credential 永続化が、Pi SDK の
`ModelRuntime` 機構から切り離された cheaper 自前 store として Rust 側で
成立することを保証する意図を記録する。移行では `<userConfigDir>/pi/`
の既存 credential を移行せず再 login を要求する判断があるため、
「参照しない」こと自体が検証対象の contract になる。credential store
形式や初期 provider 範囲が再設計された場合に見直す。

## Feature

新 credential store と model catalog による `auth`/`models` コマンドの
contract parity

## Rules

### credential store と auth コマンド

- credential は `<userConfigDir>` 直下の新規 file(Pi `auth.json` 形式と
  非互換の独自 schema)へ atomic write で永続化する
- `<userConfigDir>/pi/` の `auth.json`/`models.json`/`models-store.json`
  は一切読まず、`PI_CODING_AGENT_DIR` も参照しない。旧 credential 不在の
  状態は `auth status`/error で「再 login が必要」であることが判別可能
  にする
- `auth login/status/logout` の CLI contract を維持する: status は
  `result.providers[]`/`credentials[]`(providerId・name・methods・
  subscription・configured)の形状、login 成功は `{provider,
  authenticated,method,subscription}`、logout は `{provider,
  loggedOut:true}` を再現する
- secret は stdout・envelope・diagnostics に出さない(永続 store のみ)
- api_key login と環境変数 credential 解決を Anthropic/OpenAI/
  OpenAI-compatible/Ollama 系で成立させる
- OAuth(browser auth_url + device code)は github-copilot を必須対象と
  して login → run まで成立させる。非 TTY 等の対話不能環境は
  INVALID_REQUEST とする

### model catalog(G1)

- catalog は bundled 静的 snapshot + user `<userConfigDir>/models.json`
  を統合する。user file は Pi `models.json` 互換 schema(同 schema で
  読み、unknown key は無視)で、OpenAI-compat custom provider・
  `samplingParams` merge・`reasoning`/`maxTokens` metadata を扱える
- `get_available` 相当(認証済み model のみ候補化)を担い、tier 解決・
  `--model <provider>/<id>` explicit 解決はこの catalog 上で動く
- 未知・初期スコープ外の provider api 種別は `INVALID_MODEL_CONFIG` 系の
  明示 error(code・exitCode・retryable 付き error envelope)で失敗させ、
  silent fallback しない
- credential file・catalog file が存在しない場合でも `models` は exit 0
  を維持し、`run` は認証不在を明示 error で返す(TS 版の観測挙動を
  fixture で固定した規則)
- models.dev 系の online refresh は v1 では実装しない(offline 既定 +
  bundled/user file のみ)

## Examples

- Given: `<userConfigDir>/pi/auth.json` に有効な credential があるが新
  store が空。Then: `auth status` は未認証相当を返し再 login 案内が判別
  できる(旧 file を読んで認証済みにしない)
- Given: api_key で `auth login <provider>`。Then: 新 credential store
  に永続化され、`auth status` の `providers[]`/`credentials[]` に反映
  され、stdout に secret が現れない
- Given: `auth login github-copilot`(OAuth、device または browser
  flow)。Then: 完了後に credential が新 store に永続化され、続く run が
  その credential で成立する
- Given: user `<userConfigDir>/models.json` に OpenAI-compatible custom
  provider 定義 + unknown key。Then: unknown key を無視して merge され、
  認証済みなら `models`/`run` の候補に載る
- Given: 未知・スコープ外の provider api 種別を `--model` で指定。Then:
  `INVALID_MODEL_CONFIG` 系の error envelope(code・exitCode・retryable)
  で失敗し、別 provider への silent fallback が起きない
- Given: credential/catalog file が一切存在しない環境。Then: `models`
  は exit 0(bundled のみの結果)を返し、`run` は認証不在の明示 error を
  返す
- Given: `auth logout <provider>`。Then: store から credential が除去
  され `loggedOut:true` が返り、続く run が未認証 error になる

## Guarantees

- credential の唯一の永続先は `<userConfigDir>` 直下の新 store であり、
  Pi 由来の state 読取経路が Rust binary に残らない
- `auth status`/`models` の出力形状が TS 版と互換で、認証有無・再 login
  要求が機械判別できる
- catalog の解決源(bundled snapshot + user file)が固定され、scope 外
  api 種別が必ず明示 error になる
- secret が stdout に漏れない

## Non-goals

- `<userConfigDir>/pi/` 既存 credential の import・移行(再 login 方針で
  割り切る)
- github-copilot 以外の OAuth provider 対応(Deferred scope 決定時に追加
  sub-task 化)
- devin provider の CLI 委譲 auth・Connect wire 互換(別境界、実 endpoint
  依存の手動 smoke)
- models.dev online refresh・`PI_OFFLINE` 相当 policy の移植
- terminal interaction 自体(select prompt・raw-mode secret・URL opener)
  の UX 改善 — 既存 contract の再現のみ

## Risk

- この保証が落ちると、旧 Pi credential を誤って読む実装は移行不能な
  credential 状態を作り、逆に「読まないのに未認証と表示しない」実装は
  再 login 案内を失わせる。silent fallback のある catalog は指定と異なる
  provider/model で run を実行し cost・資格の想定を静かに裏切る。secret
  の stdout 漏洩は即座に credential 侵害となる

---
id: "DISC-034CWF93eq8KVgv2S9sQPd"
type: "discovery"
status: "draft"
title: "OpenAI Codex subscription provider support"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "docs/tasks/phase-3-port-runtime-adapter-providers.md"
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
    - "docs/tasks/phase-3-port-runtime-adapter-providers.md"
    - "../designs/openai-codex-subscription-provider.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/openai-codex-subscription-provider.md"
    - "../adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md"
    - "../designs/openai-codex-subscription-provider.md"
    - "../plans/implement-openai-codex-subscription-provider.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OpenAI Codex subscription provider support

## Exploration Goal

builtin `openai-codex` provider(ChatGPT subscription backend)を Rust
runtime で実際に利用可能にすることが目的。bundled catalog は
`openai-codex-responses` api 種別の model を 8 件収録しているが、実
endpoint への到達は複数の未実装層で阻まれている。

実 credential(`~/.codex/auth.json` の Codex CLI session、ChatGPT
subscription)による smoke で段階的に切り分け済み:

| 段階 | 結果 |
|---|---|
| credential 解決(env-var 参照の `api_key` custom provider) | ✅ `configured: true` |
| HTTP 到達 + 認証(`Bearer` + `chatgpt-account-id`) | ✅ 401 ではなく 400 — 認証通過 |
| model 受理 | ✅ `gpt-6-sol`/`gpt-6-luna` 等は受理。bundled の `gpt-5.4`/`gpt-5.3-codex-spark` は `The '<id>' model is not supported when using Codex with a ChatGPT account` で拒否 |
| request body | ❌(修正前)`{"detail":"Store must be set to false"}` — 汎用 `openai-responses` wire では Codex backend の必須 field を送らない |

## Key Issues and Assumptions

- **Issue 1(wire adapter)**: `SUPPORTED_APIS` が `anthropic-messages` /
  `openai-completions` / `openai-responses` の 3 種のみで、
  `openai-codex-responses` は `INVALID_MODEL_CONFIG`。汎用 responses
  wire は `store:false` 等の Codex 必須 contract を送らない。
- **Issue 2(OAuth flow)**: `builtin_oauth_flows` に `openai-codex` が未
  登録(github-copilot のみ、ISS-23 deferred)。`auth login openai-codex`
  で oauth method を advertise できず、oauth credential の `toAuth` も
  無い。Pi は `auth/oauth/openai-codex.js` で専用 PKCE + localhost:1455
  callback flow を実装していた(`toAuth` は `{apiKey: access}`、
  `accountId` は JWT claim `https://api.openai.com/auth.chatgpt_account_id`
  から抽出)。
- **Issue 3(catalog staleness)**: bundled `openai-codex.json` の model
  list(gpt-5.3-codex-spark, gpt-5.4, gpt-5.4-mini, gpt-5.5, gpt-5.6-*,
  gpt-6-astra)は実アカウントの受理 list(gpt-6.1-sol, gpt-6-astra,
  gpt-6-sol, gpt-6-luna, gpt-reserve, gpt-5.6-*, gpt-5.5,
  codex-auto-review — `~/.codex/models_cache.json`)と乖離。
- **確認済み仮定**: HTTP transport 自体に Codex CLI の身分は不要。wire
  contract(body fields + headers)を満たせば backend は受理する —
  `originator: codex_cli_rs` でなく `rig`/`pi` 等でも動作。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. rig `providers::chatgpt` dialect に配線 | rig 0.43 が `ResponsesContract::Codex`(store:false・stream 強制・sampling 除去・encrypted reasoning include・originator/session_id/account_id header)を内蔵。実装は `builtin.rs` の 1 arm のみ | rig 版が送らない `OpenAI-Beta`/`tool_choice`/`prompt_cache_key` 等は追加 param・header 経由で補う必要 | **yes**(wire) |
| B. 自前の Codex wire を実装 | Pi parity を完全制御 | SSE decode・tool 変換・retry を自作 — rig が既に同等を提供 | no |
| C. OAuth flow を汎用 PKCE seam で実装 | `OAuthFlow` trait + `HttpClient` seam は既存。github-copilot device flow と同じ骨格で browser+callback 版を追加 | localhost callback server が必要(現状の flow は device-code のみ) | **yes** |
| D. oauth せず api_key 経路のみ | 実装最小 | subscription credential(refresh 必須の短命 token)は api_key では運用不能 | no |
| E. catalog を実アカウント model list に更新 | `gpt-6-*` が catalog 解決可能になる | upstream の bundled catalog 更新頻度・互換性の判断が要る | **yes** |

## Tentative Conclusions and Hypotheses

- **confirmed**: wire adapter は `SUPPORTED_APIS` 追加 + `builtin.rs` に
  `chatgpt::DIALECT` 配線 1 arm で成立(実 endpoint で `gpt-6-luna` の
  bulk-reader 完走を確認済み — 作業 tree に未 commit の実装あり)。
- **confirmed**: `chatgpt-account-id` は configured header または JWT
  claim から抽出して `with_account_id` へ渡せばよい(Pi と同じ起源)。
- **confirmed**: bundled catalog の model list は実 backend が拒否する
  ため更新必須。実アカウント受理 list を基準にする。
- **hypothesis**: OAuth flow は `OAuthFlow` trait の browser PKCE
  実装(Pi の `loginOpenAICodex` 相当: auth.openai.com、client_id
  `app_EMoamEEZ...`、localhost:1455 callback、device-code fallback)を
  `builtin_oauth_flows` に登録すれば既存の auth machinery に載る。
- **hypothesis**: rig `chatgpt` dialect が付与しない header/body 差分
  (`OpenAI-Beta` 等)は `auth.headers`/`additional_params` 経路で補完可能。
  未配線の `sampling_params`/`compat`/`model.headers` は本 chain の
  スコープ判断が要る。

## Open Questions

- rig `chatgpt` dialect と Pi adapter の wire 差分(`OpenAI-Beta` header、
  `tool_choice`, `prompt_cache_key`, `instructions` default, websocket
  transport, retry-after-ms 処理, `service_tier` cost 対応)のうち、どれを
  本 chain で再現するか — SSE のみを最小構成とし、websocket/zstd 圧縮/
  session cache は out of scope とするか。
- OAuth credential の保存形式(`access`/`refresh`/`expires`/`account_id`)と
  `toAuth` の `ResolvedAuth` 写像 — header 注入面は既存か。
- bundled catalog 更新の基準 — 実アカウント受理 list に全面置換するか、
  upstream(pi-ai)の更新を待つか。`gpt-6-luna` 等の pricing/contextWindow は
  何をソースにするか。

## Promotion Candidates

- [x] spec-doc needed: Codex subscription provider の利用要件(auth login、
  model 解決、run 完走)と受入基準
- [x] adr-doc needed: wire adapter 選定(rig chatgpt dialect)、OAuth flow
  方式(browser PKCE vs device)、catalog 更新方針

## Sources and Relations

- Pi 参照実装: `@earendil-works/pi-ai@0.85.1`
  - `dist/api/openai-codex-responses.js`(dedicated adapter、SSE + WS)
  - `dist/auth/oauth/openai-codex.js`(PKCE browser flow + device fallback)
  - `dist/providers/openai-codex.js` / `providers/data/openai-codex.json`
- rig 0.43: `rig-core-0.43.0/src/providers/chatgpt/`(DIALECT +
  `ResponsesContract::Codex` + `auth::Authenticator` OAuth 対応)
- smoke 記録: 本 thread 内の `codex-sub` custom provider 実 endpoint 試行
- 関連: `docs/tasks/phase-3-port-runtime-adapter-providers.md`(ISS-23 で
  oauth flow 登録が deferred と記録)

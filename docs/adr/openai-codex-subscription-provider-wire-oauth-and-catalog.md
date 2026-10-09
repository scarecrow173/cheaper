---
id: "ADR-034CWHw0NjYQjdE5o4kErV"
type: "adr"
status: "proposed"
title: "OpenAI Codex subscription provider wire, OAuth, and catalog"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "docs/discovery/openai-codex-subscription-provider.md"
    - "docs/specs/openai-codex-subscription-provider.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/openai-codex-subscription-provider.md"
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
    - "../designs/openai-codex-subscription-provider.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# OpenAI Codex subscription provider wire, OAuth, and catalog

## Context and Problem Statement

builtin `openai-codex` provider(ChatGPT subscription backend)を実 endpoint
で動かすには 3 層の実装が要る: (1) `openai-codex-responses` wire adapter、
(2) `openai-codex` OAuth flow、(3) 受理 model list を反映した bundled
catalog。それぞれの方式を決定する。

実 endpoint smoke で判明した事実:

- ChatGPT Codex backend は標準 Responses API と異なる contract を要求する
  — `store:false`・常時 SSE・`reasoning.encrypted_content` include・
  `originator`/`session_id`/`chatgpt-account-id` header。汎用
  `openai-responses` wire では `{"detail":"Store must be set to false"}`
  で拒否される。
- HTTP client の身分(Codex CLI 等)は不要 — contract を満たせば任意の
  `originator` で受理される(実測済み)。
- Pi は `openai-codex-responses.js` という 3.5k 行の専用 adapter と
  専用 OAuth flow(PKCE browser + localhost:1455 callback + device-code
  fallback)を持っていた。
- rig 0.43 に `providers::chatgpt` dialect(`ResponsesContract::Codex`)が
  内蔵されており、Codex contract の必須部分を既に実装している。

## Decision Drivers

* Pi parity(外部 observable 契約の維持)と実装量のバランス
* rig の内蔵 dialect を最大限利用し、wire 自作を避ける
* OAuth credential の寿命(access token は短命)に refresh が必須
* catalog は実 backend が受理する model のみを露出する
* 新規依存は最小限(loopback server・PKCE は既存 deps で構成可能か)

## Considered Options

### Decision 1: wire adapter

* **A. rig `chatgpt` dialect に配線** — `builtin.rs` に
  `OpenAIConfig::with_key(&chatgpt::DIALECT, key)` の arm を追加
* **B. Codex wire を自作** — Pi adapter を port
* **C. 汎用 `openai-responses` + additional_params で補完** — dialect 無しで
  `store:false` 等を注入

### Decision 2: OAuth flow

* **A. `OAuthFlow` trait に browser PKCE + device-code 両経路を実装** —
  Pi parity(`AuthEvent::AuthUrl` + `AuthPrompt::ManualCode` + loopback
  listener、`Select` で method 選択)
* **B. device-code のみ** — headless 要件のみ満たす最小実装
* **C. browser のみ** — loopback listener のみで device flow 省略

### Decision 3: bundled catalog

* **A. pi-ai 1.1.0 の `openai-codex.json` に同期** — 9 model
  (gpt-5.3-codex-spark, gpt-5.5, gpt-5.6-*, gpt-6-astra, gpt-6-luna,
  gpt-6-sol, gpt-6.1-sol)・pricing/thinking map 付き
* **B. 実アカウント受理 list に全面置換** — `models_cache.json` 基準で
  pricing 不明 model は cost 0
* **C. 現行のまま** — stale のまま user `models[]` で補う運用

## Decision Outcome

### Decision 1: wire adapter — **Option A(rig `chatgpt` dialect)**

`crates/cheaper-rig-adapter/src/builtin.rs` に `"openai-codex-responses"`
arm を追加し、`chatgpt::DIALECT` の `OpenAIConfig` + `.completion()`
(`completion_route: Route::Responses` 経由で Codex contract の Responses
wire)を構築する。`SUPPORTED_APIS` に同 api kind を追加する。

rig dialect が担うもの: `store:false` 強制・`stream:true`(常時 SSE)・
sampling/storage/metadata/structured-output 除去・
`reasoning.encrypted_content` include・`originator`/`User-Agent`/
`session_id`/`ChatGPT-Account-Id` header・Codex SSE decode(envelope 省略
frame の repair)。

`chatgpt-account-id` の解決順は configured/toAuth header >
access token JWT の nested claim(`{"https://api.openai.com/auth":
{"chatgpt_account_id": ...}}`)— Pi `extractAccountId` 相当。header 由来
の値は transport default へは残さず `with_account_id` へ移し、二重送信を
防ぐ。

**baseUrl 正規化(Pi `resolveCodexUrl` の port)**: catalog/upstream の
`baseUrl` は `https://chatgpt.com/backend-api`(`/codex` 無し)で、Pi
adapter が `{base}/codex/responses` を組み立てる前提の値。rig dialect は
`{base}/responses` しか組まないため、素通しすると `/backend-api/responses`
(404)になる。adapter arm 内で `base` を正規化する: 末尾 `/` を除去 →
`/responses` で終われば strip → `/codex` で終わらなければ `/codex` を追加。
これにより catalog 値・credential `auth.baseUrl`・user custom baseUrl
(relay)全てが Pi と同じ endpoint 解決規則に従う。

Pi が送るが rig dialect が送らない差分(`OpenAI-Beta:
responses=experimental`、`tool_choice`、`prompt_cache_key`)は実 endpoint
検証で要否を確認し、必要なものだけを adapter 既定 header として追加する
(不要なら送らない — 実測で SSE 完走を確認済みの構成が正)。
`originator`/`User-Agent` は rig が request 単位で `rig/{ver}` を付与し、
`OpenAIConfig` に identity override setter が無いため `rig` のまま送る
(transport default header は request 側に負けて上書き不能)。`CHATGPT_*`
env var(`CHATGPT_API_BASE`/`CHATGPT_ACCOUNT_ID`/`CHATGPT_ORIGINATOR` 等)
は `from_env_with` 経路でのみ解釈され、`with_key` 構築では効かない —
本実装では env override を提供しない。

### Decision 2: OAuth flow — **Option A(browser PKCE + device-code)**

Pi `openai-codex.js` parity で `OpenAiCodexFlow` を
`crates/cheaper-runtime/src/auth/oauth/openai_codex.rs` に実装し、
`builtin_oauth_flows` に登録する。`GitHubCopilotFlow` と同じく
`Arc<dyn HttpClient>` + `Arc<dyn FlowSleeper>` を構造体に保持し、
`with_client` で test 注入する(`ProviderPluginContext` はこれらを持た
ない — plugin.rs:37-43)。

- **method 選択**: `AuthPrompt::Select` で browser(既定)/ device_code を
  選ぶ(`--method` の非対話 auto-answer は既存機構が担う)。
- **browser flow(既定)**: PKCE(S256)→
  `https://auth.openai.com/oauth/authorize`(client_id
  `app_EMoamEEZ73f0CkXaXp7hrann`、redirect_uri
  `http://localhost:1455/auth/callback`、scope `openid profile email
  offline_access`、`id_token_add_organizations=true`、
  `codex_cli_simplified_flow=true`、`originator=cheaper`)→ `AuthUrl`
  event + `127.0.0.1:1455` loopback。
  - **sequential fallback(race ではない)**: `AuthInteraction::prompt` は
    同 thread blocking(stdin `read_line`)で `Send` bound も無く、
    callback と prompt の真の race は現在の seam では表現不能。
    `TcpListener::bind` 成功時は listener を `set_nonblocking` +
    poll loop(`is_cancelled` を定期確認)で待ち、bind 失敗(port 占有
    等)時のみ `ManualCode` prompt で code 貼り付けへ切り替える。
    bind/listen は injectable seam にして test は ephemeral port で駆動する。
    callback 応答 HTML は Pi `oauth-page` 相当の静的 page。
- **device-code flow(headless)**: `POST /api/accounts/deviceauth/usercode`
  → `DeviceCode` event(verification uri `/codex/device`)→
  `/api/accounts/deviceauth/token` poll(既存 `poll_device_code_flow`
  経由、deadline は response の `expires_in`、無ければ 15 分固定)→
  返却 `authorization_code`+`code_verifier` を `redirect_uri=
  https://auth.openai.com/deviceauth/callback` で exchange。
- **refresh**: `POST /oauth/token` `{grant_type:refresh_token, ...}`。
  `invalid_grant` 等の恒久的失敗は「re-login が必要」を示す message を
  持つ error に写像する。
- **credential**: `{access, refresh, expires(epoch ms), account_id}` —
  `expires` は `expires_in` 優先・無ければ JWT `exp` claim 由来。
  `account_id` は access token の nested JWT claim から抽出(Pi
  `credentialsFromToken` 相当、抽出不能は login 失敗)。
- **`toAuth`**: `{api_key: access, headers: {"chatgpt-account-id":
  account_id}}` — header 経路が wire の `with_account_id` に繋がる。

**rig `chatgpt::auth::Authenticator` を使わない理由**: rig 同梱の OAuth
実装は自前の `auth_file` cache を持ち `CredentialStore` の単一権威と衝突、
browser PKCE を持たず、`HttpClientExt`(async)は本 flow の sync
`HttpClient` seam と不一致 — seam 二重化を避けて自前 port とする。

### Decision 3: bundled catalog — **Option A(pi-ai 1.1.0 同期)**

`crates/cheaper-runtime/src/catalog/data/openai-codex.json` を pi-ai
1.1.0 の同名 file に同期する(upstream format の `chat:` key prefix と
`type` field は loader 仕様に合わせて除去/変換)。これにより ChatGPT
account が拒否する `gpt-5.4`/`gpt-5.4-mini` の stale entry が除かれ、
`gpt-6-luna`/`gpt-6-sol`/`gpt-6.1-sol` が追加される(`gpt-5.5` は upstream
更新でも残存)。変換は `chat:` key prefix の除去と `type` field の除去
(loader は key を見ず `id` field が authoritative、unknown field も寛容
だが upstream byte 差を最小化するため機械的に正規化する)。変換の安全網は
`tests/catalog.rs` の `bundled_snapshot_models_have_supported_fields` が
全 bundled file を parse する既存 test。`data/.manifest.json` の
`files["openai-codex.json"]` sha256 も同時に更新する(現状参照されないが
整合を保つ)。per-account の受理差異(subscription tier 依存)は bundled
list では予測せず、runtime の PROVIDER_FAILURE に委ねる。

### Consequences

* Good: subscription credential で `gpt-6-*` 系が `auth login` →
  `run` まで一通り動く。wire は rig 保守面に乗る。catalog は upstream
  追従で pricing/thinking map が正確。
* Bad: rig dialect が送らない Pi 固有 field(`prompt_cache_key` 等)は
  cache 効率で劣後しうる。wire 上の `originator`/`User-Agent` は
  `rig/{ver}` 固定(`with_key` 構築では identity override setter が無く、
  request header が transport default に勝つ)— backend は受理済みだが
  「cheaper」とは名乗れない。rig に `with_identity`/`with_originator`
  が入れば追随可能。
* Neutral: browser flow の loopback listener は in-process thread —
  windows firewall prompt が出うる(127.0.0.1 のみなので実害小)。
  bind 失敗時は ManualCode path に落ちるため完全遮断にはならない。

### Confirmation

- `cargo test -p cheaper-rig-adapter -p cheaper-runtime` で wire 構築・
  JWT 抽出・OAuth flow(mock HTTP)の unit test 全 pass
- 実 credential smoke: `auth login openai-codex` → `models` →
  `run --model openai-codex/gpt-6-luna` が `ok:true`
- `mise run check` 全緑

## Implementation Plan

* Affected paths: `crates/cheaper-rig-adapter/src/builtin.rs`、
  `crates/cheaper-runtime/src/catalog/mod.rs`(`SUPPORTED_APIS`)、
  `crates/cheaper-runtime/src/auth/oauth/{mod.rs,openai_codex.rs}`(新規)、
  `crates/cheaper-runtime/src/catalog/data/openai-codex.json`、
  `crates/cheaper-cli/src/auth.rs`(必要なら login method 選択の wiring)
* Patterns to follow: `auth/oauth/github_copilot.rs`(flow 実装・mock
  HTTP test・`to_auth`)、`builtin.rs` の dialect arm、Pi
  `openai-codex.js` の flow 定数
* Patterns to avoid: wire を reqwest で自作しない、Pi の websocket/zstd
  経路を持ち込まない、catalog を手編集で独自 schema にしない(upstream
  file の機械的変換)
* Dependencies or configuration: **新規 dep を追加する** —
  `cheaper-runtime` に `sha2`(PKCE S256 challenge)、`getrandom`
  (verifier/state の CSPRNG — `fastrand` は non-crypto で不適)、
  `base64`(challenge encode — workspace に 0.22 実績あり)。全て
  transitive lock 済みの mature crate で、repo rule「依存の追加は事前
  確認」に従い本 ADR で事前に宣言する。`CHATGPT_*` env override は提供
  しない(`with_key` 経路では解釈されないため)

## Verification

* [ ] `cargo test -p cheaper-rig-adapter`(codex arm・JWT 抽出・header
  二重送信防止)
* [ ] `cargo test -p cheaper-runtime`(oauth flow mock: browser/device/
  refresh/state mismatch/to_auth)
* [ ] 実 endpoint smoke `openai-codex/gpt-6-luna` で bulk-reader `ok:true`
* [ ] `mise run check`

## Pros and Cons of the Options

### Decision 1-A: rig chatgpt dialect

* Good: Codex contract の困難な部分(SSE decode、envelope repair、reasoning
  replay、field 除去規則)が rig 保守済み。実 endpoint で完走実績あり。
* Bad: Pi の cache/session 最適化(`prompt_cache_key`・WS session reuse)
  は持てない。rig 版に無い field の追加は rig upstream 次第。

### Decision 1-B: wire 自作

* Good: Pi parity 完全制御。
* Bad: SSE decode・tool 変換・retry を自作して rig の成果を捨てる。

### Decision 1-C: 汎用 responses + additional_params

* Good: 差分だけ注入で dialect 不要に見える。
* Bad: `tool_choice`/`store` 以外にも input item 変換・envelope repair が
  必要で、additional_params は body 差分しか補えない — wire の decode 側
  までは届かず Codex frame を正しく読めない可能性。実質 B の劣化版。

### Decision 2-A: browser + device 両経路

* Good: Pi parity。desktop は browser 1 発、headless/SSH は device で完結。
* Bad: 実装量最大。loopback listener が要る。

### Decision 2-B/2-C: 片方のみ

* Good: 実装半分。
* Bad: もう一方の環境で login 不能 — subscription の主要ユースケースを
  欠く。

### Decision 3-A: upstream 同期

* Good: pricing/thinking map が upstream 保守の正確な値。差分 review が
  機械的。
* Bad: upstream 更新遅延に追随される。

### Decision 3-B/3-C

* B: 実アカウント基準だが pricing 出所なし・account 依存で他 account と
  乖離しうる。
* C: stale entry が backend 400 を呼ぶ — 運用不能。

## More Information

- Pi 参照: `@earendil-works/pi-ai@0.85.1` `auth/oauth/openai-codex.js`
  (flow 定数・credential 組立)および `api/openai-codex-responses.js`
- rig: `rig-core-0.43.0/src/providers/chatgpt/mod.rs`・
  `providers/openai/responses_api/wire.rs` の `ResponsesContract::Codex`
- Discovery: `docs/discovery/openai-codex-subscription-provider.md`
- Spec: `docs/specs/openai-codex-subscription-provider.md`

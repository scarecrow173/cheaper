---
id: "DESIGN-034CXyXncBigEivCETJLBD"
type: "design"
status: "approved"
title: "OpenAI Codex subscription provider design"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "docs/specs/openai-codex-subscription-provider.md"
    - "docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md"
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
  related:
    - "docs/discovery/openai-codex-subscription-provider.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md"
    - "docs/discovery/openai-codex-subscription-provider.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OpenAI Codex subscription provider design

## Context

builtin `openai-codex` provider(ChatGPT subscription backend
`https://chatgpt.com/backend-api/codex/responses`)を 3 層で実用化する:

1. **wire**: `openai-codex-responses` api kind を rig `chatgpt` dialect
   (`ResponsesContract::Codex`)に配線(arm 本体は作業 tree に未 commit で
   実装済み。ただし `normalize_codex_base_url` は未実装 — 現状は catalog
   baseUrl を無正規化で渡すため `/backend-api/responses` になり 404、
   本設計で新規実装する)
2. **auth**: `OpenAiCodexFlow`(browser PKCE + device-code)を
   `builtin_oauth_flows` に登録
3. **catalog**: bundled `openai-codex.json` を pi-ai 1.1.0 に同期

ADR: `docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md`

## Scope

### In Scope

- `builtin.rs` codex arm: `chatgpt::DIALECT` config + baseUrl 正規化 +
  account id 解決(header > JWT claim)(`SUPPORTED_APIS` への
  `openai-codex-responses` 追加は作業 tree で実施済み)
- `auth/oauth/openai_codex.rs`: `OAuthFlow` impl(login/refresh/to_auth)
  + loopback listener + PKCE + device-code poll
- bundled `openai-codex.json` の upstream 同期 + `.manifest.json` sha256
- unit/integration test + 実 credential smoke

### Out of Scope

- Codex WebSocket/zstd/session cache、`service_tier`・reasoning effort の
  CLI 露出、`sampling_params`/`compat`/`model.headers` の request 配線
  一般化(別 chain)、rig `originator` override(`rig/{ver}` 固定を受容)

## Components and Boundaries

### `builtin.rs` codex arm(既存作業 tree、正式化)

```rust
"openai-codex-responses" => {
    let mut config = OpenAIConfig::with_key(&rig::providers::chatgpt::DIALECT, key);
    if api_key.is_empty() { config = config.with_auth(Auth::OptionalBearer); }
    if let Some(url) = base_url { config = config.with_base_url(&normalize_codex_base_url(url)); }
    let mut headers = auth.headers.clone();
    let account_id = take_account_id_header(&mut headers)
        .or_else(|| extract_account_id(&api_key));
    if let Some(id) = account_id { config = config.with_account_id(id); }
    let client = config.connect(http_transport(&headers)?);
    CompletionModel::from_rig(client.completion(spec.model_id))
        .with_max_tokens(spec.max_tokens)
}
```

- `normalize_codex_base_url`(新規実装): 末尾 `/` 除去 → `/responses` で
  終われば strip → `/codex` で終わらなければ追加。Pi `resolveCodexUrl`
  parity で catalog 値(`backend-api`)・credential `auth.baseUrl`・user
  custom baseUrl を同一規則で `.../codex` に揃える(rig は `+ /responses`)。
  唯一の意図的差異: `/codex` を含まない `.../v1/responses` のような入力は
  Pi が `.../v1/responses/codex/responses` を生成するのに対し、本実装は
  `.../v1/codex/responses` を生成する(不正 URL の生成を避ける改善)。
- `take_account_id_header`: `chatgpt-account-id`(大小無視)を headers
  から取出して除去 → `with_account_id`(request 側 header)で送る。
  transport default に残すと二重送信になる。
- `extract_account_id`: JWT payload の nested claim
  `{"https://api.openai.com/auth": {"chatgpt_account_id": ...}}`。
  base64url decode は inline 実装(adapter に dep を増やさない判断 —
  既に作業 tree に存在)。

### `auth/oauth/openai_codex.rs`(新規)

`GitHubCopilotFlow` と同型: `http: Arc<dyn HttpClient>` +
`sleeper: Arc<dyn FlowSleeper>` + `with_client()` 注入 seam。
loopback bind は **factory 注入**(`Fn() -> io::Result<TcpListener>`)とし、
既定は `127.0.0.1:1455` を bind する closure。test は `127.0.0.1:0` に
pre-bind した listener を渡して `local_addr()` で実 port を得る。
`bind_addr` の代わりに listener factory にすることで「bind 成功だが
port 不明」問題と「bind 失敗 inject」を両方 seam だけで表現できる。
この factory は Pi の `PI_OAUTH_CALLBACK_HOST` 相当の override 点も
兼ねる(env/config 経由の host 変更は現 scope では未配線、seam のみ)。

定数(Pi parity):

```text
CLIENT_ID   = "app_EMoamEEZ73f0CkXaXp7hrann"
AUTH_BASE   = "https://auth.openai.com"
AUTHORIZE   = AUTH_BASE + "/oauth/authorize"
TOKEN       = AUTH_BASE + "/oauth/token"
REDIRECT    = "http://localhost:1455/auth/callback"
DEVICE_USERCODE = AUTH_BASE + "/api/accounts/deviceauth/usercode"
DEVICE_TOKEN    = AUTH_BASE + "/api/accounts/deviceauth/token"
DEVICE_VERIFY   = AUTH_BASE + "/codex/device"
DEVICE_REDIRECT = AUTH_BASE + "/deviceauth/callback"
LOGIN_TIMEOUT   = 15 * 60 (seconds)  // browser-wait + device poll 共通
SCOPE       = "openid profile email offline_access"
```

`login`:

1. `AuthPrompt::Select` で `browser`(default)/`device_code` を選択。
   未知の返却 id は error("Unknown login method"、Pi parity —
   scripted/test interaction が任意値を返しうるため)。
2. **browser**: `generate_pkce()`(32 random bytes → base64url-no-pad
   verifier、challenge = base64url(SHA-256(verifier)))→ `state`(16 random
   bytes → 32-char hex)→ authorize URL(`response_type=code`, `client_id`,
   `redirect_uri`, `scope`, `code_challenge`/`S256`, `state`,
   `id_token_add_organizations=true`, `codex_cli_simplified_flow=true`,
   `originator=cheaper`)→ `AuthEvent::AuthUrl` emit(instructions 付き、
   CLI が `open_url` best-effort)。
   - listener bind 成功 → `set_nonblocking(true)` + poll loop
     (50–100ms、`interaction.is_cancelled()` を毎回確認、
     **deadline は `LOGIN_TIMEOUT`(15 分、device flow と同値)で、到達時は
     `Timeout` error + `--method device_code` 案内**)。`accept()` した
     socket には即座に `set_read_timeout(5s)` を掛け(接続のみで何も
     送らない stray client があっても最悪 5s の遅延で poll へ戻り、
     cancel check が詰まらない)、最大 8KiB だけ読んで request line +
     query を parse。request 評価順序は Pi `callback-server.js` parity:
     - method/path が `GET /auth/callback` でない → `404` HTML + close、
       **poll 継続**
     - `state` 不一致(または `state` 欠落で `code` 以外を要求する
       request)→ `400` HTML + close、**poll 継続**(state が routing
       guard — stray hit/別 session の遅延 redirect で login が死なない)
     - 既に settle 済み → `409` HTML + close、**poll 継続**
     - `?error=` param 存在(`error_description` は message 補助のみ、
       単独では fail しない)→ error HTML + close、**flow を失敗終了**
     - `code` 欠落 → `400` HTML + close、**poll 継続**
     - `code` + state 一致 → success HTML + close、**settle**
   - settle 後も listener は token exchange 完了まで保持し、その間の
     request に `409` を返す(Pi `finally { close() }` parity)。
   - bind 失敗(port 占有等)→ `AuthEvent::Info` で loopback 不可を通知し
     (`--method device_code` 選択肢を案内)`AuthPrompt::ManualCode` で
     貼り付けを受ける。non-TTY で ManualCode が INVALID_REQUEST になる
     環境では device_code method が脱出経路。
   - `parse_authorization_input`(Pi parity、順序: 絶対 URL parse →
     `#` split → `code=` query → bare code):
     | input | result |
     |---|---|
     | 空/whitespace | error("Missing authorization code") |
     | `http(s)://...?code=X&state=Y` | `{code:X,state:Y}`、code 無し URL は error(他 branch へ fallthrough しない) |
     | `a#b` | `{code:a,state:b}`(`a#b#c` は `split("#",2)` で `{a,b}`) |
     | `code=X&state=Y` 等の query 文字列 | `{code:X,state:Y}` |
     | それ以外 | `{code: input}`(state 無し → 照合 skip) |
     - parsed.state 存在時のみ expected state と照合、不一致は error
       (ManualCode path では abort する — listener path の継続規則とは別)。
   - code 取得後 `POST /oauth/token` **form**
     (`application/x-www-form-urlencoded`)
     `{grant_type:authorization_code, client_id, code, code_verifier,
     redirect_uri}`。
3. **device_code**: `POST /api/accounts/deviceauth/usercode` を
   **JSON** `{client_id}`(404 時は "device code login is not enabled
   for this server" メッセージ)→ `{device_auth_id, user_code, interval}`
   (全て必須、`interval` は number **または** numeric string、
   `trim` 後 parse、非 finite または <0 は invalid response)→
   `AuthEvent::DeviceCode`(verify uri `/codex/device`)→
   `poll_device_code_flow` で `/api/accounts/deviceauth/token` を
   **JSON** `{device_auth_id, user_code}` で poll。
   `wait_before_first_poll: false`(Pi parity — 最初の poll は即時)。
   deadline は常に `LOGIN_TIMEOUT`(15 分、Pi は usercode response の
   `expires_in` を参照しない — spec Req 4 の `expires_in` 優先は
   harmless な superset として容認するが、実装は Pi parity に揃える)。
   poll response → `DevicePoll` 写像(endpoint は RFC-8628 形でないため
   `fetch_json` ではなく `http.send` を直接使い status を見る):
   - `200` + `{authorization_code, code_verifier}` 両方 → `Complete`
     (片方欠落は `Failed`、"invalid device token response")
   - `403`/`404` → `Pending`
   - それ以外は body の `error` を見る: `error` は string または
     `{code: string}` object の両形があり、`deviceauth_authorization_pending`
     → `Pending`、`slow_down` → `SlowDown`、その他 → `Failed`
   - `poll_device_code_flow` は `Result<String>` しか返せないため、
     poll closure(`FnMut`、Send 不要)が `code_verifier` を `&mut
     Option<String>` として mutably capture して保持し、Complete 値
     として `authorization_code` のみ返す設計とする(generic 化は
     しない — この flow 固有の pair 返却のため。`move` + `RefCell`
     では外側から読めなくなる点に注意)。
   - 得た `{authorization_code, code_verifier}` を
     `redirect_uri=DEVICE_REDIRECT` で `/oauth/token` exchange(form)。
4. token response `{access_token, refresh_token, expires_in}` →
   `credentials_from_token`: `access_token`/`refresh_token` は必須
   (欠落は error、Pi parity — refresh response で `refresh_token` 欠落も
   同様に失敗し rotate しない)。`account_id` = JWT nested claim
   `https://api.openai.com/auth` → `chatgpt_account_id`(抽出不能は
   login/refresh 失敗 — refresh path で失敗した場合も re-login 案内の
   message を含める)。`expires` = `now_ms + expires_in*1000`(`expires_in`
   欠落時は JWT `exp` claim ×1000 — Pi は `expires_in` 必須だが本実装は
   寛容な superset として fallback を許容。`expires_in` も JWT `exp` も
   取れない場合は error とする — 期限不明の credential を残さない)→
   `CredentialData::Oauth`。

`refresh`: `POST /oauth/token` form `{grant_type:refresh_token,
refresh_token, client_id}` → 同じく `credentials_from_token`。
`invalid_grant`/`400` 系、および refresh response での
`account_id` 抽出失敗・`refresh_token` 欠落は、message に「re-login が
必要」を含める(service は credential を消さないため、run 毎に同じ
失敗を繰り返す — 案内 message が唯一の mitigations)。

`to_auth`: `ResolvedAuth { api_key: Some(access), headers:
{"chatgpt-account-id": account_id}, base_url: None, source: OAuth }`。
`account_id` は credential field 優先、空なら JWT claim から再抽出。
(Pi の `toAuth` は `{apiKey}` のみで account-id は adapter 側が JWT から
再抽出する — 本実装は header 経由 + adapter fallback の二段構えで同等の
wire 結果に到達する。JWT nested-claim decode は adapter の inline
base64url 版と runtime の `base64` dep 版に 2 箇所存在する — 小さな
重複として容認)

### bundled catalog 同期

`data/openai-codex.json` を pi-ai 1.1.0 `providers/data/openai-codex.json`
に同期。変換規則(loader 要件ではなく正規化 — `CatalogModel` は
`deny_unknown_fields` を持たず group/model key も無視されるため、
変換無しでも parse は可能): `type == "chat"` 以外の entry を filter
(upstream が将来 non-chat model を足した場合の安全網)した上で `type`
field と upstream 固有の group key prefix `chat:` を除去し、`id` を
key にする(`id` field が authoritative)。`inputLimits` 等の unknown
field はそのまま残す(`compat` は `CatalogModel` の declared
passthrough field)。`.manifest.json` の
`files["openai-codex.json"]` sha256 を更新。変換は script でなく一回の
機械的編集(既存 test `bundled_snapshot_models_have_supported_fields`
が全 file parse を検証)。

### 依存追加(ADR で事前宣言済み)

`cheaper-runtime` に `sha2`(S256)、`getrandom`(verifier/state の
CSPRNG、`fastrand` は non-cryptographic で不可)、`base64`(challenge
encode、`cheaper-provider-devin` が crate-level で 0.22 使用実績)を
追加。いずれも `Cargo.lock` に transitive として既出。adapter 側には
dep を増やさない(inline base64url decoder を維持)。
loopback listener は `std::net::TcpListener` で新規 dep 無し。

## Data and Control Flow

### login(browser)

```text
cli auth login openai-codex
  → AuthService::login → OpenAiCodexFlow::login
    → Select prompt(browser/device_code)
    → PKCE verifier+challenge、state 生成
    → AuthUrl event → CLI が表示 + open_url best-effort
    → TcpListener bind 1455:
        ok → nonblocking poll → GET /auth/callback?code&state
             → HTML 応答 → code
        fail → ManualCode prompt → parse code(+state 照合)
    → POST /oauth/token → credential {access, refresh, expires, account_id}
    → CredentialStore.set("openai-codex")
    → refresh_plugin_catalog(F13、失敗は catalog errors に記録のみ)
```

### run

```text
run --model openai-codex/gpt-6-luna
  → catalog resolve(api: openai-codex-responses)
  → AuthService::get_auth → oauth credential → expiry window 内なら
    OpenAiCodexFlow::refresh → to_auth → ResolvedAuth{api_key, headers}
  → builtin_completion_model(codex arm)
    → normalize baseUrl → with_account_id → rig chatgpt dialect
  → POST https://chatgpt.com/backend-api/codex/responses (SSE,
    store:false, originator/session_id/ChatGPT-Account-Id)
  → run_stream → envelope
```

## Non-Functional Constraints

- credential/token/account_id は stdout・log・commit に出さない
  (`describe()` は secret を含まない表現のみ)。
- loopback は `127.0.0.1` のみ bind(外部露出しない)。state 照合で
  CSRF を防ぐ。
- login の blocking 区間は `is_cancelled` を ≤100ms 周期で確認し、
  Ctrl+C relay(signal relay thread)で中断可能にする。
- `expires` の単位は epoch **ms**(pi-ai parity)。service の refresh
  window(5 分)は既存のまま。

## Risks and Trade-offs

- **bind 成功時は ManualCode 脱出経路が無い**(Pi は listener 成功時も
  manual paste を race させるが、同期 `AuthInteraction` では実現不能 —
  ADR で承認済みの sequential fallback)。SSH 越し等で callback が
  届かない環境は cancel → `--method device_code` を選び直す。
- **browser-wait の脱出は deadline(15 分)または cancel のみ**:
  `AuthInteraction::is_cancelled` は CLI 以外の呼び出し元では常に
  false を返しうるため、deadline を持たない設計は無限待ちになる。
  `LOGIN_TIMEOUT` を設ける。
- **`localhost` redirect vs IPv4-only bind**: `redirect_uri` は
  `localhost:1455`、listener は `127.0.0.1` のみ bind。`localhost` が
  `::1` にのみ解決される環境では callback が listener に届かず
  deadline まで待つ(Pi も同じ default。listener factory seam が
  `PI_OAUTH_CALLBACK_HOST` 相当の将来の override 点)。
- **rig dialect が送らない field**(`OpenAI-Beta`/`tool_choice`/
  `prompt_cache_key`)は smoke で実害なしを確認済み。将来 backend が
  要求を強めたら adapter 既定 header/`additional_params` 配線で追加。
- **originator 不整合**: authorize URL には `originator=cheaper`、wire
  request には `originator: rig`(`with_key` 経路では override 不能、
  backend は受理済み)。別 knob であり、同一と誤認しないよう明記。
- **catalog は upstream 基準**: per-account tier で拒否される model
  (`gpt-5.3-codex-spark` 等)は残りうる — runtime error で表面化する
  設計。
- **oauth path は provider-config `headers` を merge しない**(既存
  copilot と同じ制限)。`openai-codex` は `api_key_name` を持たない
  oauth-only provider のため、`models.json` の `apiKey`/`headers` 設定は
  request auth に一切到達しない(設定自体は受理されるが silent に無効)。
  account-id は credential の `account_id`(または JWT claim)のみが
  有効。
- **listener test の flaky 回避**: 固定 port 1455 を unit test で bind
  しない(pre-bind listener factory 注入 + ephemeral port)。callback
  parse/exchange の本体は mock HTTP + scripted `AuthInteraction` で
  検証する。

## Failure Modes

- login: bind 失敗 → ManualCode fallback / browser-wait deadline →
  `Timeout` / `?error=` callback → `ProviderFailure` / state 不一致
  (ManualCode のみ) → error / exchange 失敗 → `ProviderFailure` /
  account_id 抽出不能 → error(login credential を残さない)。
- refresh: `invalid_grant` → re-login 案内 / `refresh_token` 欠落 →
  error(rotate しない)/ 新 access token に nested claim 無し →
  re-login 案内 / 5 分 window 内の失敗 → run 毎 retry(service 挙動、
  credential は保持)。
- store: `CredentialStore.set` 失敗 → 上流へ伝搬(token は失われ、
  再 login が必要)。

## Deferred Design Concerns

- WebSocket transport・zstd request 圧縮・prompt_cache_key session reuse
  | SSE で要件成立 | backend が SSE を制限した場合 | 帯域・接続数の劣化
- `OpenAI-Beta`/`tool_choice` 等の残差分 | 実 endpoint で不要求を確認 |
  backend が必須化した場合 | その時点で adapter 既定 header に追加
- `AuthInteraction` の Send 化による真の race | 現 sequential fallback
  で成立 | login UX 改善要求が来た場合 | 変更面が trait 境界に広い

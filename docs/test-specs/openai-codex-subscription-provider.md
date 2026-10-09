---
id: "TSPEC-034CYp5Xbp8sJ8ZV199qV3"
type: "test-spec"
status: "draft"
title: "OpenAI Codex subscription provider guarantees"
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
    - "docs/designs/openai-codex-subscription-provider.md"
  derived-by: []
  verifies:
    - "docs/specs/openai-codex-subscription-provider.md"
    - "docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OpenAI Codex subscription provider guarantees

## Purpose

SPEC-034CWHveVPBJp33jPRTCen / ADR-034CWHw0NjYQjdE5o4kErV
(`openai-codex` provider の wire・OAuth・catalog 3 層)が守るべき振る舞いを
固定する: (1) catalog/credential/user custom baseUrl が単一規則で
`.../codex` に正規化されること、(2) browser PKCE + device-code の両
login 経路が Pi parity の request ordering・fallback・deadline 規則に
従うこと、(3) credential が `{access, refresh, expires(epoch ms),
account_id}` を必須 field として保持し refresh/toAuth が規定の失敗
semantics を持つこと、(4) bundled catalog が upstream pi-ai 1.1.0 基準で
`.manifest.json` sha256 と整合すること。

引退条件: wire adapter(rig `chatgpt` dialect 配線)・OAuth flow 方式
(browser PKCE + device-code)・catalog upstream 同期方針のいずれかが
再設計された場合。`parse_authorization_input`・listener request
ordering の規則が変わる場合は本 spec の対応する Rules/Examples を
見直す。

## Feature

`openai-codex` provider(Codex subscription backend)の baseUrl 正規化、
OAuth login(browser loopback PKCE + device-code)・refresh・toAuth、
および bundled catalog の upstream 同期。

## Rules

### baseUrl 正規化(spec Req 6a / design codex arm)

- `normalize_codex_base_url` は末尾 `/` を除去し、末尾 `/responses`
  を strip し、末尾 `/codex` で終わらなければ `/codex` を append する。
  catalog 値(`https://chatgpt.com/backend-api`)・credential
  `auth.baseUrl`・user custom baseUrl(relay)を同一規則で `.../codex`
  に揃える(rig dialect が `base + "/responses"` を組み立てる前提)。
- normalize の**出力は最終 wire URL ではなく base**(rig が後段で
  `/responses` を append する)。例えば `https://relay/v1/responses`
  の正規化結果は `https://relay/v1/codex` であり、wire 上の最終
  URL は `https://relay/v1/codex/responses` になる。
- `/codex` を含まない `.../v1/responses` のような入力は
  `.../v1/codex/responses` を生成する(Pi parity からの意図的差異 —
  不正 URL の生成を避ける改善)。

### `chatgpt-account-id` 解決(spec Req 2・5 / ADR Decision 1)

- `toAuth` は `{api_key: Some(access), headers: {"chatgpt-account-id":
  account_id}, base_url: None, source: OAuth}` を返す。`account_id` は
  credential field 優先、空なら access token JWT の nested claim
  `{"https://api.openai.com/auth": {"chatgpt_account_id": ...}}` から
  再抽出する。
- adapter arm は configured/toAuth header の `chatgpt-account-id`
  (大小無視)を transport default から除去して `with_account_id` へ
  移す(二重送信防止)。header に無ければ JWT claim から拾う。
  解決不能でも構築は失敗しない(backend が 400 を返す運用可視性)。

### authorize URL / PKCE(spec Req 3 / design 定数)

- browser flow の authorize URL は `https://auth.openai.com/oauth/
  authorize` に `response_type=code`・`client_id=
  app_EMoamEEZ73f0CkXaXp7hrann`・`redirect_uri=
  http://localhost:1455/auth/callback`・`scope=openid profile email
  offline_access`・`code_challenge`(base64url の SHA-256(verifier)、
  S256)・`code_challenge_method=S256`・`state`(32-char hex)・
  `id_token_add_organizations=true`・`codex_cli_simplified_flow=true`・
  `originator=cheaper` を含む。
- PKCE verifier は 32 random bytes の base64url-no-pad、state は
  16 random bytes の hex — いずれも CSPRNG(`getrandom`)由来。

### loopback listener request ordering(spec Req 3 / design login browser)

- listener は `127.0.0.1`(既定 port 1455、factory seam で注入可)のみ
  bind し、`set_nonblocking` + poll loop(50–100ms、`is_cancelled` を
  毎回確認、deadline `LOGIN_TIMEOUT`=15 分)で callback を待つ。
  accept した socket は `set_read_timeout(5s)` + 最大 8KiB read。
- login は `AuthPrompt::Select`(browser 既定 / `device_code`)で
  始まり、scripted/test interaction が未知の id を返した場合は
  error("Unknown login method")(Pi parity)。
- request 評価順序は Pi `callback-server.js` parity で固定:
  1. method/path が `GET /auth/callback` でない → `404` + close、poll 継続
  2. `state` param が expected と一致しない(欠落含む) →
     `400` + close、poll 継続
  3. 既に settle 済み → `409` + close、poll 継続
  4. `?error=` param 存在(`error_description` は message 補助のみ) →
     error + close、flow を失敗終了
  5. `code` 欠落 → `400` + close、poll 継続
  6. `code` + state 一致 → success HTML + close、settle
- state 照合(2)は `?error=`(4)・settle 済み(3)を含む全 request で
  先行する。したがって `?error=` による fail も settle 後の `409` も
  正しい `state` を伴う request に限る — wrong/missing state は
  settle 後でも `400` + poll 継続。
- settle 後も listener は token exchange 完了まで保持し、その間の
  request には `409` を返す。login() は single-threaded で exchange
  が accept loop を block するため、「409 window」は post-exchange
  drain で実装する: exchange 中に届いた接続は OS backlog に積まれ、
  exchange return 後の non-blocking accept drain pass が各接続に
  `409` を返してから listener を drop する(Pi は async で live に
  `409` を返す — 本実装は exchange 完了後に返す accepted
  divergence)。test では mock `/oauth/token` 応答を channel/barrier
  で block して exchange 中に 2 本目の `TcpStream` を backlog へ
  積み、barrier 解放後に `409` が返ることを検証する。
- bind 失敗時は `AuthEvent::Info`(loopback 不可 + `--method
  device_code` 案内)の後に `AuthPrompt::ManualCode` で code 貼り付け
  経路へ切り替える(sequential fallback — callback と prompt の真の
  race ではない)。
- browser-wait deadline 到達は `Timeout` error で `--method
  device_code` 案内を message に含める。`is_cancelled` は cancel で
  login を中断する。

### `parse_authorization_input`(ManualCode / design edge-case table)

- 解釈順序: 空 → error / 絶対 URL parse → `#` split → `code=` query →
  bare code。code 無し URL は error(他 branch へ fallthrough しない)。
- parsed.state が存在するときのみ expected state と照合し、不一致は
  error(ManualCode path では abort — listener path の continue 規則
  とは別規則)。

### device-code flow(spec Req 3 / design login device_code)

- `POST /api/accounts/deviceauth/usercode` は **JSON** `{client_id}`。
  404 は "device code login is not enabled for this server"。
  response `{device_auth_id, user_code, interval}` は全て必須、
  `interval` は number または numeric string(`trim` 後 parse、
  非 finite・<0 は invalid response)。
- `AuthEvent::DeviceCode` は `verification_uri: /codex/device`、
  `interval_seconds: Some(<parse 済み interval>)`、
  `expires_in_seconds: Some(900)`(`LOGIN_TIMEOUT` 固定、Pi parity)を
  emit する。
- `/api/accounts/deviceauth/token` への poll は `http.send` を直接
  使い(`fetch_json` でない — endpoint は RFC-8628 形でない)、status
  を見る: `200` + `{authorization_code, code_verifier}` 両方 →
  `Complete`(片方欠落 → `Failed`)、`403`/`404` → `Pending`、その他は
  body `error`(string または `{code}` object)で
  `deviceauth_authorization_pending` → `Pending`、`slow_down` →
  `SlowDown`、その他 → `Failed`。
- `wait_before_first_poll: false`、deadline は常に `LOGIN_TIMEOUT`
  (usercode `expires_in` は参照しない)。poll closure は
  `code_verifier` を `&mut Option<String>` で capture し、Complete 値は
  `authorization_code` のみ返す。
- 得た `{authorization_code, code_verifier}` を
  `redirect_uri=https://auth.openai.com/deviceauth/callback` で
  `/oauth/token` exchange する(form POST)。

### credential / refresh(spec Req 4・8 / design credentials_from_token)

- token response `{access_token, refresh_token, expires_in}` →
  `CredentialData::Oauth`。`access_token`・`refresh_token` は必須
  (refresh response で `refresh_token` 欠落も失敗 — rotate しない)。
- `account_id` = access token JWT の nested claim
  `https://api.openai.com/auth.chatgpt_account_id`。抽出不能は login/
  refresh 失敗。`expires` = `now_ms + expires_in*1000`、fallback は
  JWT `exp` claim ×1000、両方取れなければ error。
- refresh は `POST /oauth/token` form `{grant_type:refresh_token,
  refresh_token, client_id}`。`invalid_grant`/400 系・`refresh_token`
  欠落・`account_id` 抽出失敗は message に re-login 案内を含める。
- login の `/oauth/token` exchange が失敗(`400
  {"error":"invalid_grant"}` 等)した場合は `ProviderFailure` で、
  credential は永続化されない — browser・device 両 path 共通
  (design Failure Modes)。
- credential/token/account_id を stdout・log・commit に出力しない。

### catalog 同期(spec Req 6 / ADR Decision 3)

- bundled `openai-codex.json` は pi-ai 1.1.0 `providers/data/
  openai-codex.json` を基準に同期する: `type=="chat"` 以外を filter →
  `type` field 除去 → model key の `chat:` prefix 除去(`id` field が
  authoritative)。`inputLimits` 等 unknown field と `compat` は残す。
- 同期後の model list は 9 件(gpt-5.3-codex-spark・gpt-5.5・
  gpt-5.6-luna・gpt-5.6-sol・gpt-5.6-terra・gpt-6-astra・gpt-6-luna・
  gpt-6-sol・gpt-6.1-sol)。この set は `bundled.rs` `#[cfg(test)]`
  の unit test で `bundled_models("openai-codex")` がちょうど 9 id
  を返し `gpt-6-luna` を含み stale の `gpt-5.4`/`gpt-5.4-mini` を
  含まないことを assert する。
- `.manifest.json` の `files["openai-codex.json"]` sha256 が file
  bytes と一致することは、同じく `bundled.rs` の Rust unit test で
  `include_str!` の埋め込み bytes に対して検証する(`sha2` dep を
  利用 — `python` 等の外部 script による一回限りの検証ではなく
  継続的な自動検証とする)。
- per-account tier による受理差異(`gpt-5.3-codex-spark` 等)は bundled
  list で予測せず、runtime の PROVIDER_FAILURE として表面化する。

## Examples

- Given: catalog `baseUrl` `https://chatgpt.com/backend-api`。When:
  codex arm が `with_base_url` へ渡す値を正規化する。Then:
  `https://chatgpt.com/backend-api/codex`。
- Given: `https://chatgpt.com/backend-api/codex`(または末尾 `/` 付き)。
  Then: 不変。Given: `https://chatgpt.com/backend-api/codex/responses`。
  Then: `.../backend-api/codex`。Given: `https://relay/v1`。Then:
  `https://relay/v1/codex`。Given: `https://relay/v1/responses`。
  Then: `https://relay/v1/codex`(Pi parity からの意図的差異)。
- Given: `auth.headers` に `ChatGPT-Account-Id`(大文字混在)+ 別 header。
  When: codex arm を構築する。Then: account-id は `with_account_id` へ
  移り transport headers から除去され、他 header は残る。
- Given: header 無し・access token JWT payload が
  `{"https://api.openai.com/auth":{"chatgpt_account_id":"acc-1"}}`。
  Then: `with_account_id("acc-1")`(JWT claim fallback)。
- Given: authorize URL を emit した scripted `AuthInteraction` と
  pre-bound listener(`127.0.0.1:0`)。When: 実 `TcpStream` で
  `GET /auth/callback?code=X&state=<emit state>` を送る。Then:
  success HTML が返り、`POST /oauth/token` の form body が
  `grant_type=authorization_code`・`code=X`・`code_verifier`・
  `redirect_uri=http://localhost:1455/auth/callback` を含み、
  credential が `Oauth` になる。emit された authorize URL の query
  は全 param を assert する: `response_type=code`・`client_id`・
  `redirect_uri`・`scope`・`code_challenge` +
  `code_challenge_method=S256`・`state`・
  `id_token_add_organizations=true`・`codex_cli_simplified_flow=true`・
  `originator=cheaper`。
- Given: 同上の setup で mock `/oauth/token` が
  `400 {"error":"invalid_grant"}` を返す。Then: login は
  `ProviderFailure` で終わり credential は永続化されない
  (browser path の exchange failure)。device path の exchange 失敗も
  同じ assertion shape で検証する。
- Given: scripted `AuthPrompt::Select` が未知の id を返す。Then:
  login は error("Unknown login method")で終わる。
- Given: listener factory が `Err`(port 占有想定)を返す。Then:
  `AuthEvent::Info` の後 `ManualCode` prompt が走り、貼り付け
  `code=X&state=Y` が exchange される。state 不一致の貼り付けは error
  で abort。
- Given: listener が settle 前。When: `GET /other`・`GET
  /auth/callback?code=X&state=wrong`・`GET /auth/callback?error=
  access_denied&state=<emit state>`・`GET /auth/callback?state=<emit
  state>`(code 無し)を逐次送る。Then: 404/400/400 で close され
  poll は継続するが、`?error=`(+ 正しい state)は flow を失敗終了
  させる。**`?error=` request にも正しい `state` が必須** — state
  欠落・不一致の `?error=` request は fail ではなく `400` + poll
  継続になる(state 照合が error 評価に先行する)。
- Given: settle 済み(state 一致の `code` を受理済み)で mock
  `/oauth/token` の応答が channel/barrier で block されている。
  When: exchange 完了前に 2 本目の `TcpStream` を接続して
  `GET /auth/callback?code=X&state=<emit state>` を送り、barrier を
  解放する。Then: exchange 完了後の post-exchange drain pass が
  その接続へ `409` を返す(sync 実装は exchange 中に live で `409`
  を返せない — Pi との accepted divergence)。settle 後でも
  wrong/missing state の request は `409` ではなく `400`。
- Given: `is_cancelled` false・deadline 注入 seam で時間経過を偽装。
  Then: browser-wait が `Timeout` error(`--method device_code` 案内
  付き)で終わる(実時間を待たない)。
- Given: `AuthPrompt::Select` が `device_code` を返す scripted
  interaction と usercode 応答 `{device_auth_id, user_code,
  interval:"5"}`(numeric string)。When: login を駆動する。Then:
  `DeviceCode` event emit → poll が `403` Pending → `200`
  `{authorization_code, code_verifier}` Complete の列を踏み、
  exchange が `redirect_uri=.../deviceauth/callback` で行われる。
- Given: device token poll が `{error:"deviceauth_authorization_pending"}`
  ・`{error:{code:"slow_down"}}`・`{error:"access_denied"}` を順に返す。
  Then: Pending → SlowDown → Failed に写像される。200 で
  `code_verifier` 欠落は Failed("invalid device token response")。
- Given: `parse_authorization_input` の入力列。Then: `""`/`"  "` →
  error、`http://localhost:1455/auth/callback?code=X&state=Y` →
  `{X,Y}`、`http://x/path`(code 無し)→ error、`a#b` → `{a,b}`、
  `a#b#c` → `{a,b}`、`code=X&state=Y` → `{X,Y}`、`rawcode` →
  `{code:"rawcode"}`(state 照合 skip)。
- Given: token response に `refresh_token` 欠落、または access token
  JWT に nested claim 無し。Then: credentials 生成は error(login で
  credential を残さない)。`expires_in` 無し + JWT `exp` 有り →
  `exp*1000`。両方無し → error。
- Given: refresh response が `400 {"error":"invalid_grant"}`。
  Then: error message に re-login 案内を含む。
- Given: `Oauth{access, account_id:"acc-9"}` credential。When:
  `to_auth`。Then: `ResolvedAuth{api_key:Some(access), headers:
  {chatgpt-account-id: acc-9}, source:OAuth}`。`account_id` 空なら JWT
  claim から再抽出。
- Given: pi-ai 1.1.0 `openai-codex.json` の `chat:` prefix key・
  `type:"chat"` field・unknown field(`inputLimits` 等)。When:
  bundled file を同期する。Then: key は `gpt-*` のみ、`type` は除去、
  unknown field は残り、`bundled_models("openai-codex")` が 9 件を
  parse で返し `.manifest.json` sha256 が一致する。
- Given: 実 credential(環境にある場合)。When: `auth login
  openai-codex` → `models --provider openai-codex` → `run bulk-reader
  --model openai-codex/gpt-6-luna`。Then: `gpt-6-*` 系列挙 +
  `ok:true`。credential 無し環境では skip(理由を impl record に記録)。

## Guarantees

- `openai-codex-responses` model が `INVALID_MODEL_CONFIG` にならず、
  catalog baseUrl が単一規則で `.../codex` に正規化されて wire へ渡る
  (rig dialect が `+ /responses` を組み立てる前提と整合)。
- browser login が stray hit/別 session の遅延 redirect・state 不一致・
  code 欠落 request で死なず、正しい state を伴う `?error=` と
  deadline と cancel のみが flow を終わらせる。
- bind 失敗時に `ManualCode`、非 TTY では `--method device_code` の
  脱出経路が残る。
- device-code poll が 200/403/404/error の写像規則を守り、
  `code_verifier` が exchange まで到達する。
- credential の `access`/`refresh`/`expires`/`account_id` が欠落した
  response は永続化されず、refresh の恒久的失敗は re-login 案内を
  持つ error に写像される。
- `chatgpt-account-id` が toAuth header 経路と JWT fallback 経路の
  二段構えで wire に到達し、transport default と request header の
  二重送信が起きない。
- bundled catalog が upstream pi-ai 1.1.0 と整合し、`.manifest.json`
  hash が file bytes と一致する。
- credential・token・account id が stdout・log・commit に出ない。

## Non-goals

- Codex WebSocket transport・zstd request 圧縮・`prompt_cache_key`
  session reuse の検証(SSE で要件成立、out of scope)。
- `OpenAI-Beta`/`tool_choice`/`instructions` 等 rig dialect が送らない
  field の wire 検証(実 endpoint smoke で実害なし確認済み、必要時は
  別 chain)。
- `service_tier`/reasoning effort の CLI 露出、oauth 経路での
  provider-config `headers` merge(github-copilot と同じ既存制限)。
- `chatgpt-account-id` 未解決時の構築失敗(解決不能は構築 error に
  しない設計 — backend の 400 に委ねる)。
- upstream `expires_in`(usercode response)に基づく device deadline
  (Pi parity で常に `LOGIN_TIMEOUT`)。
- rig `originator`/`User-Agent` の `rig/{ver}` 固定の検証
  (`with_key` 経路では override 不能、backend 受理済み)。
- windows firewall・IPv6 `localhost` 解決など環境依存挙動の網羅検証
  (listener factory seam が将来の override 点)。

## Risk

- この保証が落ちると: baseUrl 正規化の欠落で catalog 値が
  `/backend-api/responses`(404)になり provider 全体が到達不能に戻る;
  request ordering の drift は stray hit で login が死ぬ; ManualCode
  fallback の欠落は port 占有/headless 環境で login 不能になる;
  `account_id` 抽出失敗の silent 化は backend 400 を診断不能にする;
  expires 欠落 credential の永続化は毎 run の refresh 失敗ループを
  生む; catalog drift は実 backend が拒否する model を露出する
  (`gpt-5.4` 系の stale entry)。

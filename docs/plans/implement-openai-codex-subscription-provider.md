---
id: "PLAN-034CYiZ4kkOoa3oKL7PnLj"
type: "plan"
status: "approved"
title: "Implement OpenAI Codex subscription provider"
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
    - "docs/specs/openai-codex-subscription-provider.md"
    - "docs/designs/openai-codex-subscription-provider.md"
  implemented-by:
    - "docs/tasks/implement-openai-codex-subscription-provider.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/openai-codex-subscription-provider.md"
    - "docs/designs/openai-codex-subscription-provider.md"
    - "docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md"
    - "docs/discovery/openai-codex-subscription-provider.md"
  derived-by:
    - "docs/tasks/implement-openai-codex-subscription-provider.md"
  verifies: []
  verified-by:
    - "docs/test-specs/openai-codex-subscription-provider.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement OpenAI Codex subscription provider Implementation Plan

## Goal

approved design `DESIGN-034CXyXncBigEivCETJLBD` をそのままコードへ落とす:
builtin `openai-codex` provider(ChatGPT subscription backend
`https://chatgpt.com/backend-api/codex/responses`)を 3 層で実用化する。

1. **wire**: `openai-codex-responses` api kind を rig `chatgpt` dialect に
   配線した既存 arm(作業 tree 未 commit)に `normalize_codex_base_url` を
   追加して catalog baseUrl を `.../codex` に正規化する
2. **auth**: `OpenAiCodexFlow`(browser PKCE + device-code)を新設し
   `builtin_oauth_flows` へ登録する
3. **catalog**: bundled `openai-codex.json` を pi-ai 1.1.0 upstream へ同期し
   `.manifest.json` の sha256 を更新する

## Scope

1 つの実装ストリーム(adapter・oauth・catalog が同一 provider の
end-to-end 化という single critical path + single release intent を共有)。
分割しない。境界は design の Scope に一致する。

- **In scope**: `normalize_codex_base_url` + codex arm の baseUrl 正規化
  配線、`auth/oauth/openai_codex.rs` の `OAuthFlow` impl(login browser/
  device・refresh・to_auth)+ loopback listener + PKCE、 bundled
  `openai-codex.json` の pi-ai 1.1.0 同期 + `.manifest.json` sha256、
  unit/integration test + 実 credential smoke、impl record。
- **Out of scope**: Codex WebSocket/zstd/session cache、`service_tier`・
  reasoning effort の CLI 露出、`sampling_params`/`compat`/`model.headers`
  の request 配線一般化(別 chain)、rig `originator` override、
  `CHATGPT_*` env override、oauth 経路での provider-config `headers`
  merge(github-copilot と同じ既存制限)、catalog 未収録 model の追加。

## Upstream Documents

- `docs/specs/openai-codex-subscription-provider.md`
  (SPEC-034CWHveVPBJp33jPRTCen — Req 1-8・Acceptance Criteria)
- `docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md`
  (ADR-034CWHw0NjYQjdE5o4kErV — Option A 確定決定 3 件 + 依存事前宣言)
- `docs/discovery/openai-codex-subscription-provider.md`
  (DISC-034CWF93eq8KVgv2S9sQPd — 実 endpoint smoke の切り分け結果)

## Design Inputs

- `docs/designs/openai-codex-subscription-provider.md`(approved)— 本 plan
  の唯一の構造源。`Components and Boundaries` の module 契約、request
  ordering、`parse_authorization_input` edge-case table、`DevicePoll`
  写像、`Failure Modes` をそのまま実装・検証へ写像する。
- 設計上の normative 注意:
  - `normalize_codex_base_url`: 末尾 `/` 除去 → `/responses` strip →
    `/codex` 追加。`.../v1/responses` 入力は Pi と異なり
    `.../v1/codex/responses` を生成する意図的差異(design 記載)を
    維持する。
  - `chatgpt-account-id` は transport default header に残さず
    `with_account_id` へ移す(二重送信防止 — 実装済み、回帰維持)。
  - request 評価順序は Pi `callback-server.js` parity: path/method 不一致
    → 404 + poll 継続 → `state` param が expected と一致しない(欠落
    含む)→ 400 + poll 継続 → settle 済み → 409 + poll 継続 →
    `?error=` → fail → `code` 欠落 → 400 + poll 継続 → `code`+state
    一致 → settle。**この順序を並べ替えない。** state 照合は
    `?error=`・409 を含む全 request で先行するため、`error=` や
    settle 後の 409 assertion も必ず正しい `state` を伴わせる
    (wrong/missing state → settle 後でも 400)。
  - listener は settle 後も token exchange 完了まで保持し、その間の
    request に `409` を返す(Pi `finally { close() }` parity)。
    login() は single-threaded で exchange が accept loop を block
    するため、「409 window」は **post-exchange drain** で実装する:
    exchange 中に届いた接続は OS backlog に積まれ、exchange return
    後に non-blocking accept loop が backlog を drain して各
    connection に `409` を返してから listener を drop する(drain も
    通常 request と同じ 5s read timeout・8KiB cap を適用し、byte を
    送らない接続が teardown を stall させないよう iteration に上限を
    設ける)。Pi は async で exchange 中に 409 を live 返却するが、
    本実装は exchange 完了後に返す — accepted divergence として記録する。
  - browser-wait deadline は `LOGIN_TIMEOUT`(15 分)固定。`is_cancelled`
    を ≤100ms 周期で確認。accept socket へ `set_read_timeout(5s)` +
    最大 8KiB 読み取り cap。
  - device-code: `wait_before_first_poll: false`(最初の poll は即時)、
    deadline は常に `LOGIN_TIMEOUT`(usercode response の `expires_in`
    は参照しない — Pi parity)。poll closure が `code_verifier` を
    `&mut Option<String>` で capture する設計を維持する。
    **ADR Decision-2 の「deadline は response の `expires_in`、
    無ければ 15 分固定」bullet は本 design の固定 `LOGIN_TIMEOUT`
    (Pi parity)に supersede される** — 実装は `expires_in` を
    参照しない。
  - `expires` = `now_ms + expires_in*1000`、fallback で JWT `exp`*1000、
    両方取れなければ error(期限不明 credential を残さない)。
    `account_id` 抽出不能は login/refresh 失敗。
  - credential/token/account_id を stdout・log・commit に出さない。

## File Map

### `cheaper-rig-adapter`

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-rig-adapter/src/builtin.rs` | `normalize_codex_base_url` 新規実装 + codex arm の `with_base_url` への配線 | codex arm・`take_account_id_header`・`extract_account_id`・`base64url_decode`・既存 test は作業 tree で未 commit — 最初の commit 境界に含める |

### `cheaper-runtime`

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-runtime/Cargo.toml` | `sha2`・`getrandom = "0.3"`・`base64` の新規 dep | ADR で事前宣言済み、AGENTS.md「Ask first: 依存の追加」に従い user confirmation を実施してから編集。`getrandom` は `"0.3"` に pin(Cargo.lock に 0.2/0.3/0.4 が transitive 混在し API が version 間で drift するため)。adapter 側には dep を増やさない |
| `crates/cheaper-runtime/src/auth/oauth/openai_codex.rs` | 新設。`OpenAiCodexFlow`(`OAuthFlow` impl)+ PKCE/state 生成 + authorize URL + listener factory seam(`Fn() -> io::Result<TcpListener>`、既定 `127.0.0.1:1455`)+ nonblocking poll loop + `parse_authorization_input` + device-code path + token exchange + `credentials_from_token` + `refresh` + `to_auth` | `GitHubCopilotFlow` と同型(`http: Arc<dyn HttpClient>` + `sleeper: Arc<dyn FlowSleeper>` + `with_client()` 注入 seam)。JWT nested-claim decode は runtime の `base64` dep 版(adapter の inline 版との小さな重複は design で受容済み) |
| `crates/cheaper-runtime/src/auth/oauth/mod.rs` | `pub mod openai_codex` + `pub use openai_codex::OpenAiCodexFlow` + `builtin_oauth_flows` への `openai-codex` 登録 | registry は `Runtime::create` の seed — 既存 override 規則不変 |
| `crates/cheaper-runtime/src/auth/mod.rs` | `pub use` への `OpenAiCodexFlow` 追加 | `GitHubCopilotFlow` parity の re-export(1 行) |
| `crates/cheaper-runtime/src/lib.rs` | `pub use` への `OpenAiCodexFlow` 追加 | `GitHubCopilotFlow` parity の re-export(1 行) |
| `crates/cheaper-runtime/src/catalog/mod.rs` | `SUPPORTED_APIS` の `openai-codex-responses` entry | 作業 tree で追加済み(未 commit)— T1 の commit に含める |
| `crates/cheaper-runtime/src/catalog/data/openai-codex.json` | pi-ai 1.1.0 `providers/data/openai-codex.json` への同期 | `type=="chat"` 以外を filter → `type` field 除去 → key の `chat:` prefix 除去(`id` field が authoritative)。`inputLimits` 等 unknown field は残す。script でなく一回の機械的編集 |
| `crates/cheaper-runtime/src/catalog/data/.manifest.json` | `files["openai-codex.json"]` の sha256 再計算 | manifest format: `{"schemaVersion":3,"generatedAt","structureHash","files":{"<name>.json":"<sha256hex>"}}`。現状 loader は `generatedAt` のみ読み hash を検証しないが整合を保つ(ADR) |

### test / docs

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-rig-adapter/src/builtin.rs` `#[cfg(test)]` | normalize 写像 4 case + codex arm 構築 test | `codex_model_builds_with_jwt_derived_account_id`・`account_id_header_is_consumed_not_duplicated`・`base64url_decodes_*` は作業 tree に既存 |
| `crates/cheaper-runtime/src/auth/oauth/openai_codex.rs` `#[cfg(test)]` | mock `HttpClient` + fake `FlowSleeper` + scripted `AuthInteraction` + pre-bound listener(`127.0.0.1:0`、実 `TcpStream` で callback を駆動)による flow test | 固定 port 1455 を unit test で bind しない(design Risk) |
| `crates/cheaper-runtime/tests/catalog.rs` | `bundled_snapshot_models_have_supported_fields` が全 bundled file を parse — 更新後 catalog の安全網 | 変更なしで既存 test が通ることを確認 |
| `crates/cheaper-runtime/src/catalog/bundled.rs` `#[cfg(test)]` | `bundled_models("openai-codex")` の 9-model set assertion + `.manifest.json` `files["openai-codex.json"]` sha256 と埋め込み bytes の一致 test | sha2 dep(T2)を利用した Rust unit test — 外部 script(`python` 等)に頼らず継続検証する |
| `docs/impl/ir/` impl record | 実装記録(`impl-doc` skill の `new_impl_record.js` で scaffold、`--task` で task doc と link) | 完了時に作成 |

## Implementation Sequence

### Phase A — adapter normalization(作業 tree 既存分の正式化 + normalize)

- [ ] 1. `builtin.rs` に `normalize_codex_base_url` を実装する
      (`trim_end_matches('/')` → 末尾 `/responses` なら strip → 末尾
      `/codex` でなければ `/codex` を append)
- [ ] 2. codex arm の `config.with_base_url(url)` を
      `config.with_base_url(&normalize_codex_base_url(url))` へ変更する
- [ ] 3. normalize の unit test を追加する:
      `https://chatgpt.com/backend-api` → `.../backend-api/codex`、
      `.../backend-api/codex` → 不変、`.../backend-api/codex/` → 不変、
      `.../backend-api/codex/responses` → `.../backend-api/codex`、
      `https://relay/v1` → `https://relay/v1/codex`、
      `https://chatgpt.com/backend-api/responses` →
      `.../backend-api/codex`(`/responses` strip 後 `/codex` append)
- [ ] 4. `mise exec -- cargo test -p cheaper-rig-adapter` で緑を確認する
- [ ] 5. 作業 tree の未 commit 変更(codex arm・`SUPPORTED_APIS` entry・
      account-id helpers・既存 test)と本 Phase をまとめて commit する
      (`feat: wire openai-codex-responses models to rig chatgpt dialect`)

### Phase B — oauth 依存追加(user confirmation gate)

- [ ] 6. **user に確認する**(AGENTS.md「Ask first: 依存の追加」):
      `cheaper-runtime` への `sha2`・`getrandom`・`base64` 追加。
      ADR-034CWHw0NjYQjdE5o4kErV で事前宣言済みであることを示す
- [ ] 7. 承認後 `crates/cheaper-runtime/Cargo.toml` に `sha2`・
      `getrandom = "0.3"`(pin — Cargo.lock に 0.2/0.3/0.4 が
      transitive 混在し API が drift するため)・`base64`
      (workspace 実績は 0.22)を追加し `mise exec -- cargo check -p
      cheaper-runtime` で compile を確認する

### Phase C — `openai_codex.rs` scaffolding + credential/token plumbing

- [ ] 8. `auth/oauth/openai_codex.rs` を新設し、`OpenAiCodexFlow` 構造体
      (`http: Arc<dyn HttpClient>` + `sleeper: Arc<dyn FlowSleeper>` +
      `listener_factory: Arc<dyn Fn() -> io::Result<TcpListener> + Send + Sync>`)
      と `new()`(既定 factory は `127.0.0.1:1455` bind)・
      `with_client()`(test 注入、factory override 可)を実装する。
      design 定数(`CLIENT_ID`・`AUTH_BASE`・`AUTHORIZE`・`TOKEN`・
      `REDIRECT`・`DEVICE_USERCODE`・`DEVICE_TOKEN`・`DEVICE_VERIFY`・
      `DEVICE_REDIRECT`・`LOGIN_TIMEOUT=900s`・`SCOPE`)をモジュール
      定数として置く
- [ ] 9. `generate_pkce()`(32 random bytes → base64url-no-pad verifier、
      challenge = base64url(SHA-256(verifier)))と `state` 生成(16 random
      bytes → 32-char hex)を実装する(CSPRNG は `getrandom`)
- [ ] 10. `parse_authorization_input` を design の edge-case table どおり
      実装する(順序: 空 → error / 絶対 URL parse → `#` split →
      `code=` query → bare code。code 無し URL は error で他 branch へ
      fallthrough しない。parsed.state 存在時のみ照合、不一致は error)
      実装注意: `a#b#c` は `{code:a, state:b}` を yield しなければ
      ならない — `split('#')` で先頭 2 要素を取る。`splitn(2, '#')`
      は remainder `b#c` を state に残すため不可。`code=` query
      branch は `contains("code=")` の substring 判定(Pi の
      `value.includes("code=")` parity)で starts-with ではない
- [ ] 11. `credentials_from_token` を実装する(`access_token`・
      `refresh_token` 必須、`account_id` = JWT nested claim
      `https://api.openai.com/auth` → `chatgpt_account_id` — 抽出不能は
      error。`expires` = `now_ms + expires_in*1000`、fallback JWT `exp`
      ×1000、両方無しは error)
- [ ] 12. `refresh`(`POST /oauth/token` form `grant_type=refresh_token`)
      と `to_auth`(`ResolvedAuth { api_key: Some(access), headers:
      {"chatgpt-account-id": account_id}, base_url: None, source: OAuth }`
      — `account_id` は credential field 優先、空なら JWT 再抽出)を
      実装する。`invalid_grant`/400 系・`refresh_token` 欠落・
      `account_id` 抽出失敗は message に re-login 案内を含める
- [ ] 13. `oauth/mod.rs` に `pub mod openai_codex` + `pub use
      openai_codex::OpenAiCodexFlow` を追加し、`builtin_oauth_flows` に
      `("openai-codex", Arc::new(OpenAiCodexFlow::new()))` を登録する。
      `GitHubCopilotFlow` parity で `auth/mod.rs` と `src/lib.rs` の
      `pub use` にも `OpenAiCodexFlow` を追加する(各 1 行)。
      **この Phase では `login` は deliberate stub として
      `ProviderFailure("not yet implemented")` を返す**(T3 時点では
      browser/device の login 実装は未着手のため — T4/T5 で実装して
      置き換える。登録自体は `builtin_oauth_flows` の shape 検証の
      ために先行させる)
- [ ] 14. `mise exec -- cargo test -p cheaper-runtime` で compile +
      既存 test の回帰無しを確認し Phase B+C を commit する
      (`feat: add openai-codex oauth flow scaffolding and credential
      plumbing`)

### Phase D — browser login path(listener + poll loop + ManualCode)

- [ ] 15. `login` の先頭に `AuthPrompt::Select`(`browser` 既定 /
      `device_code`)を実装する。未知の返却 id は error("Unknown login
      method"、Pi parity)
- [ ] 16. browser path: authorize URL(`response_type=code`・`client_id`・
      `redirect_uri`・`scope`・`code_challenge`/`S256`・`state`・
      `id_token_add_organizations=true`・`codex_cli_simplified_flow=true`
      ・`originator=cheaper`)を組み立て、`AuthEvent::AuthUrl`
      (instructions 付き)を emit する
- [ ] 17. listener bind 成功経路: `set_nonblocking(true)` + poll
      loop(50–100ms、`is_cancelled` を毎回確認、deadline `LOGIN_TIMEOUT`
      到達で `Timeout` error + `--method device_code` 案内)。accept した
      socket に `set_read_timeout(5s)` + 最大 8KiB read。request 評価は
      design の順序どおり(path/method → 404 continue → `state` param
      が expected と一致しない(欠落含む)→ 400 continue → settled →
      409 continue → `?error=` → fail → code 欠落 → 400 continue →
      code+state → settle + success HTML)。settle 後も listener は
      exchange 完了まで保持する — login() は single-threaded で
      exchange が accept loop を block するため、exchange 中に届いた
      接続は OS backlog に積まれ、**exchange return 後に non-blocking
      accept loop が backlog を drain して各接続に `409` を返してから
      listener を drop する**(post-exchange drain — Pi は async で
      live に 409 を返すが、本実装は exchange 後に返す accepted
      divergence)
- [ ] 18. listener bind 失敗経路: `AuthEvent::Info` で loopback 不可を
      通知(`--method device_code` 案内を含む)→ `AuthPrompt::ManualCode`
      で貼り付けを受け `parse_authorization_input` で解釈
      (ManualCode path の state 不一致は abort — listener path の
      continue 規則とは別)
- [ ] 19. code 取得後 `POST /oauth/token` form
      (`application/x-www-form-urlencoded`)`{grant_type:
      authorization_code, client_id, code, code_verifier, redirect_uri}`
      で exchange し `credentials_from_token` へ渡す
- [ ] 20. `mise exec -- cargo test -p cheaper-runtime openai_codex` で
      compile を確認し commit する(`feat: add browser pkce login path
      to openai-codex oauth flow`)

### Phase E — device-code login path

- [ ] 21. device_code path: `POST /api/accounts/deviceauth/usercode` を
      **JSON** `{client_id}` で送り(404 → "device code login is not
      enabled for this server")、response `{device_auth_id, user_code,
      interval}` を検証する(全て必須、`interval` は number または
      numeric string、`trim` 後 parse、非 finite または <0 は invalid
      response)
- [ ] 22. `AuthEvent::DeviceCode` を emit する。field は Pi parity で
      `verification_uri: DEVICE_VERIFY`(`/codex/device`)、`user_code`
      は usercode response の値、`interval_seconds:
      Some(<parse 済み interval>)`、`expires_in_seconds: Some(900)`
      (`LOGIN_TIMEOUT` 固定 — usercode response の `expires_in` を
      参照しない、Pi は両 field を emit する)
- [ ] 23. `poll_device_code_flow` で `/api/accounts/deviceauth/token` を
      **JSON** `{device_auth_id, user_code}` で poll する
      (`wait_before_first_poll: false`、deadline `LOGIN_TIMEOUT`)。
      poll closure は `http.send` を直接使い status 写像:
      `200` + `{authorization_code, code_verifier}` 両方 → `Complete`
      (片方欠落 → `Failed` "invalid device token response")/
      `403`・`404` → `Pending` / その他は body `error`(string または
      `{code}` object)で `deviceauth_authorization_pending` → `Pending`、
      `slow_down` → `SlowDown`、その他 → `Failed`。`code_verifier` は
      `&mut Option<String>` を closure が mutably capture する
      (`move` + `RefCell` で外側から読めなくしない)
- [ ] 24. 得た `{authorization_code, code_verifier}` を
      `redirect_uri=DEVICE_REDIRECT` で `/oauth/token` exchange(form)し
      `credentials_from_token` へ渡す
- [ ] 25. `mise exec -- cargo test -p cheaper-runtime openai_codex` で
      compile を確認し commit する(`feat: add device-code login path
      to openai-codex oauth flow`)

### Phase F — catalog 同期(T1 と独立、並行可)

- [ ] 26. upstream pi-ai 1.1.0 `providers/data/openai-codex.json` を取得する
      (canonical source は unpkg の published tarball:
      `https://unpkg.com/@earendil-works/pi-ai@1.1.0/dist/providers/data/
      openai-codex.json`。local extraction
      `C:\Users\akiyoshi\AppData\Local\Temp\pi-latest\package\dist\
      providers\data\openai-codex.json` は当該 machine 固有の一時
      cache であり他環境には存在しない — 存在する場合は unpkg 取得物
      との byte/内容一致 cross-check にのみ使う)
- [ ] 27. 変換を適用して `data/openai-codex.json` を置き換える:
      `type=="chat"` 以外の entry を filter → 各 entry の `type` field を
      除去 → model key の `chat:` prefix を除去(`id` field は既に
      unprefixed で authoritative)。9 model(gpt-5.3-codex-spark・gpt-5.5・
      gpt-5.6-luna・gpt-5.6-sol・gpt-5.6-terra・gpt-6-astra・gpt-6-luna・
      gpt-6-sol・gpt-6.1-sol)になることを確認。`inputLimits` 等 unknown
      field と `compat` はそのまま残す
- [ ] 28. `.manifest.json` の `files["openai-codex.json"]` を新 file の
      sha256 hex で更新する(`structureHash`・`generatedAt` は upstream
      manifest の運用に倣い、今回の機械的編集では `files` entry のみを
      更新する — `generatedAt` を pi-ai 1.1.0 manifest の値に揃えるかは
      upstream `data/.manifest.json` 同梱の有無を確認して揃える)
- [ ] 29. `bundled.rs` の `#[cfg(test)]` に同期 assertion を追加する:
      `bundled_models("openai-codex")` が期待 9 id(gpt-5.3-codex-spark・
      gpt-5.5・gpt-5.6-luna・gpt-5.6-sol・gpt-5.6-terra・gpt-6-astra・
      gpt-6-luna・gpt-6-sol・gpt-6.1-sol)をちょうど返し `gpt-6-luna`
      を含み stale の `gpt-5.4`/`gpt-5.4-mini` を含まないこと、および
      `.manifest.json` の `files["openai-codex.json"]` sha256 が
      埋め込み file bytes(`include_str!` の実体)と一致すること
      (`sha2` dep は T2 で追加済み — `python` 等の外部 script では
      なく Rust unit test で継続検証する)。`mise exec -- cargo test
      -p cheaper-runtime --test catalog` で
      `bundled_snapshot_models_have_supported_fields` が緑、
      `mise exec -- cargo test -p cheaper-runtime` で新規 unit test
      を含む全 test が緑であることを確認し commit する(`feat: sync
      bundled openai-codex catalog with pi-ai 1.1.0`)

### Phase G — oauth flow unit test

- [ ] 30. test scaffolding を `openai_codex.rs` `#[cfg(test)]` に実装する:
      scripted `AuthInteraction`(Select/ManualCode の応答列 + `is_cancelled`
      制御 + event capture)、mock `HttpClient`(URL/method/body assert +
      応答列。`/oauth/token` 応答を channel/barrier で block できる
      仕組みを持たせ、settle 後 409 drain の検証で exchange 完了前に
      2 本目の `TcpStream` を backlog へ積めるようにする)、fake
      `FlowSleeper`(no-op + 記録)、pre-bound
      `TcpListener`(`127.0.0.1:0`)を返す factory + `local_addr()` で
      実 port 取得
- [ ] 31. browser happy path: pre-bound listener へ実 `TcpStream` で
      `GET /auth/callback?code=X&state=<emit された state>` を送り、
      success HTML が返ること・mock HTTP が `/oauth/token` form
      (`code_verifier`・`redirect_uri=REDIRECT` 含む)を受けること・
      返却 credential が `Oauth {access, refresh, expires, account_id}`
      になることを検証する(authorize URL から `state` と
      `code_challenge` を parse して callback に使う)。authorize URL
      の query は**全 param を assert する**: `response_type=code`・
      `client_id`・`redirect_uri`・`scope`・`code_challenge` +
      `code_challenge_method=S256`・`state`・
      `id_token_add_organizations=true`・
      `codex_cli_simplified_flow=true`・`originator=cheaper`。また
      mock `/oauth/token` が `400 {"error":"invalid_grant"}` を返す
      経路で login が `ProviderFailure` となり credential が永続化
      されないことを検証する(exchange-failure case)
- [ ] 32. bind 失敗 → `ManualCode` fallback: factory が `Err` を返す経路で
      `AuthEvent::Info` emit + ManualCode prompt が走り、貼り付け
      `code=X&state=Y` で exchange されることを検証する。ManualCode の
      state 不一致は error で abort することを別 case で検証する。
      また scripted `Select` が未知の id を返す経路は
      error("Unknown login method")で終わることを検証する(Pi parity)
- [ ] 33. listener request matrix: 異なる path/method → 404 + poll 継続、
      `state` param が expected と一致しない(欠落含む)→ 400 + poll
      継続、`GET /auth/callback?error=access_denied&state=<emit された
      state>` → flow 全体 fail、code 欠落(正しい state 付き)→ 400 +
      poll 継続、code+state 一致 → settle を、逐次実 `TcpStream` で
      駆動して検証する。**state 照合は `?error=`・409 を含む全
      request で先行するため、`error=` や settle 後の 409 assertion
      は必ず正しい `state` を伴わせる**(wrong/missing state →
      settle 後でも 400)。settle 後の 409 は post-exchange drain で
      検証する: mock `/oauth/token` の応答を channel/barrier で
      block させ、その間に 2 本目の `TcpStream` を接続して backlog
      に積ませてから barrier を解放する — exchange 完了後の drain
      pass がその接続へ `409` を返すことを assert する(sync 実装は
      exchange 中に live で 409 を返せない — Pi との accepted
      divergence)
- [ ] 34. deadline timeout: scripted `is_cancelled`・fake sleeper と
      deadline 注入 seam で browser-wait が `Timeout` error(`--method
      device_code` 案内を含む)で終わることを検証する(実時間 15 分を
      待たない — sleeper/instant 注入で時間を偽装する。deadline が
      `Instant::now()` 直書きで注入不能なら、flow 側に
      `Duration` override seam(`with_login_timeout` 等)を設ける —
      design の factory seam 思想と同じく内部 seam のみ追加)
- [ ] 35. device-code path: usercode 応答 → `AuthEvent::DeviceCode` emit →
      poll が `403 Pending` → `200 Complete` の列を踏むこと、
      `code_verifier` capture と `redirect_uri=DEVICE_REDIRECT` の
      exchange form、`interval` numeric string 受理・欠落 field →
      invalid response・`slow_down` → `SlowDown` 写像を検証する。
      device path の `/oauth/token` exchange 失敗(`400
      {"error":"invalid_grant"}`)も同じ assertion shape
      (`ProviderFailure` + credential 非永続化)で検証する
- [ ] 36. `parse_authorization_input` table test(空 → error・URL
      `?code&state`・code 無し URL → error・`a#b` → `{a,b}`・
      `a#b#c` → `{a,b}`・query 文字列・bare code)と
      `credentials_from_token` test(access/refresh 欠落 → error・
      account_id 抽出不能 → error・`expires_in` → now+in*1000・JWT
      `exp` fallback・両方無し → error)を書く
- [ ] 37. `refresh` test(form body・`invalid_grant`/400 → re-login
      案内を含む message・refresh response の `refresh_token` 欠落 →
      error・新 access token に nested claim 無し → re-login 案内)と
      `to_auth` test(header `chatgpt-account-id`・credential 空のとき
      JWT 再抽出)を書く
- [ ] 38. `mise exec -- cargo test -p cheaper-runtime` が全緑であることを
      確認し commit する(`test: cover openai-codex oauth flow paths`)

### Phase H — 全体検証 + 実 credential smoke

- [ ] 39. `mise exec -- cargo test -p cheaper-rig-adapter` と
      `mise exec -- cargo test -p cheaper-runtime` を実行して全緑を
      確認する
- [ ] 40. `mise run check` を実行して全緑を確認する
- [ ] 41. 実 credential smoke(credential が入手可能な環境でのみ):
      `auth login openai-codex`(browser または `--method device_code`、
      既存 `~/.codex` credential の活用が可能ならそれも試す)→
      `models --provider openai-codex` が `gpt-6-luna` を含む
      `gpt-6-*` 系を列挙 → `run bulk-reader --model
      openai-codex/gpt-6-luna` が `ok:true`。credential が入手不能な
      環境では skip 理由を impl record に記録する

### Phase I — 記録・監査

- [ ] 42. `new_impl_record.js --task docs/tasks/implement-openai-codex-
      subscription-provider.md` で impl record を scaffold し、実施内容・
      smoke 結果・発覚した差分を記録する
- [ ] 43. `doc_status.js audit --type plan` / `--type task` /
      `--type test-spec` で本 chain の file に対する findings が無い
      ことを確認し、必要なら doc status/relation を更新する

## Dependencies and Risks

- **依存**: 上流 doc は全て確定済み(design approved)。新規外部 dep
  は `sha2`・`getrandom`・`base64` のみ(全て `Cargo.lock` に transitive
  として既出、ADR で事前宣言)で **user confirmation gate(Phase B
  step 6)が必須**。upstream catalog は pi-ai 1.1.0 から取得。
  canonical source は unpkg の published tarball —
  `Temp\pi-latest` の local extraction は当該 machine 固有の一時
  cache で他環境には存在しないため、存在する場合は内容一致の
  cross-check にのみ使う。
- **Critical path**: Phase A(T1)→ 独立。Phase B(T2)→ C(T3)→ D(T4)→
  E(T5)→ G(T7)。Phase F(T6)は T1 と並行可(ただし manifest sha256
  assertion が `sha2` を使うため T2 に依存)。
  Phase H(T8)→ I(T9)。
- **Risk 1 — dep 追加の user 未承認**: Phase B step 6 で承認が得られない
  場合、CSPRNG/SHA-256 を std で実装する手段が無く OAuth flow が構築不能。
  承認を得てから Phase C 以降へ進む(adapter・catalog のみ先行可)。
- **Risk 2 — listener test の flaky**(design Risk): 固定 port 1455 を
  unit test で bind しない。pre-bind listener factory 注入 + ephemeral
  port で回避する規則を plan 段階で固定済み。
- **Risk 3 — poll loop の実時間依存**: `LOGIN_TIMEOUT`=15 分の実待機は
  test 不能。step 34 の deadline/sleeper 注入 seam で回避する
  (seam が `Instant::now()` 直書きに潰される場合は flow 側に
  `Duration` override を追加する — design の seam 思想の範囲内)。
- **Risk 4 — 実 credential smoke の環境依存**: `~/.codex` credential や
  browser が無い環境では smoke を実行できない。skip 理由を impl record に
  記録し、unit/integration test を AC の代替証跡とする。
- **Risk 5 — catalog upstream drift**: local extraction(machine 固有
  cache)と unpkg の内容が異なる場合、pi-ai 1.1.0 の published
  tarball(unpkg)側を authoritative とする(design: upstream 基準)。
- **Risk 6 — `localhost` redirect vs IPv4-only bind**: `::1` のみ解決の
  環境で callback が届かず deadline まで待つ(design で受容済み。
  listener factory seam が将来の override 点)。
- **Risk 7 — request ordering drift**: 404/400/409/fail/settle の順序を
  間違えると stray hit で login が死ぬ。Pi parity 順を step 17 に固定し
  step 33 の matrix test で pin する。

## Task Breakdown

実行単位は `docs/tasks/implement-openai-codex-subscription-provider.md`
(単一 task doc、内部で T1-T9 に順序付け)。Phase との対応:

| ID | 範囲 | 内容 | 依存 |
| --- | --- | --- | --- |
| T1 | adapter normalize | step 1-5 | — |
| T2 | dep 追加 gate | step 6-7 | —(user confirmation) |
| T3 | oauth scaffolding + credential/token plumbing | step 8-14 | T2 |
| T4 | browser login path | step 15-20 | T3 |
| T5 | device-code login path | step 21-25 | T3 |
| T6 | catalog 同期 | step 26-29 | T2(manifest sha256 test が `sha2` に依存。T1 と並行可) |
| T7 | oauth flow unit tests | step 30-38 | T4 + T5 |
| T8 | 全体検証 + smoke | step 39-41 | T1, T6, T7 |
| T9 | impl record + audit | step 42-43 | T8 |

## Verification

- [ ] `mise exec -- cargo test -p cheaper-rig-adapter` で codex arm・
      normalize 4+case・JWT 抽出・header 二重送信防止が緑
- [ ] `mise exec -- cargo test -p cheaper-runtime` で oauth flow mock
      test(browser happy + authorize URL 全 param assert/bind 失敗
      ManualCode/Select 未知 id → "Unknown login method"/state
      mismatch continue/`?error`+state fail/settle 後 409(post-
      exchange drain)/exchange 失敗 → ProviderFailure + 非永続化/
      deadline timeout/device Pending→Complete/
      parse_authorization_input table/credentials_from_token/refresh
      invalid_grant/to_auth header)が緑
- [ ] `bundled_snapshot_models_have_supported_fields` が更新後
      `openai-codex.json` で緑(`mise exec -- cargo test -p
      cheaper-runtime --test catalog`)、bundled.rs unit test で
      `bundled_models("openai-codex")` が 9 id + `gpt-6-luna` 含有・
      `gpt-5.4`/`gpt-5.4-mini` 非含有、`files["openai-codex.json"]`
      sha256 が埋め込み file bytes と一致
- [ ] `builtin_oauth_flows()` が `openai-codex` を含み、`auth login
      openai-codex --method device_code` 経路で `Select` auto-answer が
      動く(既存 CLI `--method` 機構経由)
- [ ] `mise run check` 全緑
- [ ] 実 credential smoke: `models --provider openai-codex` が
      `gpt-6-luna` を列挙し `run bulk-reader --model
      openai-codex/gpt-6-luna` が `ok:true`(credential 入手不能時は
      impl record に skip 理由)

## Rollback Strategy

- 全変更は `crates/` 内の Rust コード + bundled JSON data + doc に留まり、
  外部 contract(envelope・exit code・`models.json` schema)は不変。
  `git revert` で撤去可能。
- `openai-codex.json` の同期を戻す場合は前 commit の file に戻すだけ —
  bundled snapshot は code 埋め込み(`include_str!`)で runtime 状態を
  持たない。`.manifest.json` の hash も一緒に戻す。
- oauth credential は `CredentialStore` の既存 format(`CredentialData::
  Oauth`)で破壊的 schema 変更無し — flow 自体を revert しても既存
  credential は他 provider と同じく残り、読み出し側は provider drop で
  `unknown provider` に戻るだけ。
- `builtin_oauth_flows` の登録行を外せば `auth login openai-codex` は
  oauth method を advertise しなくなり login 経路が閉じる — catalog は
  残るが credential 無しでは到達不能になるだけで破壊的ではない。

## Open Questions / Gaps

- `generatedAt`/`structureHash` の同期方針: `.manifest.json` は pi-ai
  dist の bundled manifest を写したもので、`files` hash のみ必須更新
  (step 28)。upstream manifest の `generatedAt` を揃えるかは upstream
  file 同梱を確認して実装時に確定する。
- `with_login_timeout` 等の deadline 注入 seam の命名・形状は実装時に
  `openai_codex.rs` の既存構造に合わせる(design は seam の存在のみ
  規定)。
- `--method` の CLI wiring は既存機構(`AuthPrompt::Select` auto-answer、
  `crates/cheaper-cli/src/auth.rs`)で成立するため CLI 側の変更は
  現時点で想定しない — flow が `Select` option id `device_code` を
  出せば自動で効く。option id の不一致が発覚した場合のみ `auth.rs` に
  触れる。
- 実 credential smoke は環境依存 — 実施可否は実行時に判断し、skip 時は
  impl record に理由を記録する。

## Self-Review

- [x] Every spec requirement maps to a task(Req 1/6a → T1、Req 2/5 →
      T1+T3、Req 3 → T3-T5、Req 4 → T3+T5+T7、Req 6 → T6、Req 7 →
      Verification の既存 warning 経路確認、Req 8 → review-time
      invariant — credential/token/account_id の非出力は自動 test
      ではなく実装 review で担保する規則のため test case としては
      列挙しない)
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: T1 は独立・小規模、T3-T5 は同一 file の直列
  critical path であるため、同一 agent の inline execution を基本とする。
  T6(catalog 同期)は content 確定済みの機械的変換だが manifest sha256
  assertion が `sha2` に依存するため T2 後に dispatch する。
- Independent work streams: T6(catalog sync)は T1/T3-T5 と独立(T2
  のみ必要)。T5(device-code)は T3 の plumbing 確定後であれば T4 と
  並行可だが、同一 file のため同一 agent で直列に進める方が衝突が少ない。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 本 plan は review gate 待ち(`status: draft`)。task doc は plan 承認を
  前提条件として先に作成済み(TASK-DOC-GATE-001 は chain 指示により
  同時生成 — plan の status が `approved` になってから task を開始する)。
- **前提ゲート**: Phase B step 6 の依存追加 user confirmation は
  AGENTS.md「Ask first」に基づく必須 gate。ADR で事前宣言済みでも
  確認を省略しない。
- レビュー重点 1(normative 遵守): request 評価順序(404→400→409→
  error fail→400→settle)・listener の settle 後保持・
  `wait_before_first_poll: false`・`code_verifier` の `&mut
  Option<String>` capture・expires fallback 規則が design と一致して
  いるか。
- レビュー重点 2(baseUrl 正規化): `normalize_codex_base_url` の
  strip/append 順序と `/codex` を含まない `.../v1/responses` 入力に
  対する意図的差異(`.../v1/codex/responses`)が design 記述どおりか。
- レビュー重点 3(credential): `access`/`refresh`/`expires`/`account_id`
  の必須性と `invalid_grant` re-login 案内・`account_id` 抽出不能時の
  失敗規則が design と一致しているか。
- レビュー重点 4(catalog): `chat:` prefix 除去・`type` field 除去・
  `type=="chat"` filter の機械的変換が upstream file に正しく適用
  されているか(9 model)。

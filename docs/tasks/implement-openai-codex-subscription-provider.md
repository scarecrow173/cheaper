---
id: "TASK-034CYsDxlTD7NNAPMkB4mb"
type: "task"
status: "todo"
title: "Implement OpenAI Codex subscription provider"
created: "2026-10-09"
updated: "2026-10-10"
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
    - "docs/plans/implement-openai-codex-subscription-provider.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-openai-codex-subscription-provider.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-openai-codex-subscription-provider.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/openai-codex-subscription-provider.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement OpenAI Codex subscription provider

## Work

plan `docs/plans/implement-openai-codex-subscription-provider.md`
(PLAN-034CYiZ4kkOoa3oKL7PnLj)の実行単位。builtin `openai-codex`
provider(ChatGPT subscription backend)を 3 層で実用化する:
`normalize_codex_base_url` を含む wire adapter の正式化、
`OpenAiCodexFlow`(browser PKCE + device-code)の `builtin_oauth_flows`
登録、bundled `openai-codex.json` の pi-ai 1.1.0 同期。

plan の Implementation Sequence Phase A-I が本 task の作業単位。順序と
依存関係は以下のとおり(各項目は semantic commit の単位):

- **T1 — adapter normalize**(plan step 1-5、依存なし):
  `builtin.rs` に `normalize_codex_base_url`(末尾 `/` 除去 →
  `/responses` strip → `/codex` append)を実装し codex arm の
  `with_base_url` へ配線。normalize 写像 test(catalog baseUrl・
  既に `/codex`・`/codex/responses`・custom relay)を追加。
  作業 tree の未 commit 変更(codex arm・`SUPPORTED_APIS` entry・
  account-id helpers・既存 test)をこの commit 境界に含める。
- **T2 — dep 追加 gate**(plan step 6-7、依存なし):
  `cheaper-runtime` への `sha2`・`getrandom = "0.3"`・`base64` 追加
  について **user confirmation を取得してから** `Cargo.toml` を
  編集する(AGENTS.md「Ask first: 依存の追加」、ADR で事前宣言
  済み)。`getrandom` は `"0.3"` に pin — Cargo.lock に
  0.2/0.3/0.4 が transitive 混在し API が version 間で drift する
  ため。`mise exec -- cargo check -p cheaper-runtime` で compile
  確認。
- **T3 — oauth scaffolding + credential/token plumbing**
  (plan step 8-14、T2 に依存): `auth/oauth/openai_codex.rs` 新設 —
  `OpenAiCodexFlow` 構造体(`http`/`sleeper`/listener factory seam、
  既定 `127.0.0.1:1455`)・design 定数群・`generate_pkce`/`state` 生成・
  `parse_authorization_input`(edge-case table 順序)・
  `credentials_from_token`(access/refresh 必須・JWT nested claim
  `account_id` 必須・`expires` = `now_ms+expires_in*1000` → JWT `exp`
  fallback → error)・`refresh`(form `grant_type=refresh_token`、
  `invalid_grant` → re-login 案内)・`to_auth`
  (`{api_key: Some(access), headers: {chatgpt-account-id}}`)。
  `oauth/mod.rs` に `pub mod` + `builtin_oauth_flows` 登録 +
  `auth/mod.rs`・`src/lib.rs` へ `OpenAiCodexFlow` の re-export
  (`GitHubCopilotFlow` parity、各 1 行)。T3 時点の `login` は
  deliberate stub として `ProviderFailure("not yet implemented")`
  を返し、T4/T5 で実装して置き換える。
- **T4 — browser login path**(plan step 15-20、T3 に依存):
  `AuthPrompt::Select`(browser 既定/device_code、未知 id は error)→
  authorize URL(`code_challenge`/S256/`state`/
  `id_token_add_organizations`/`codex_cli_simplified_flow`/
  `originator=cheaper`)→ `AuthEvent::AuthUrl` → listener bind 成功は
  nonblocking poll loop(`is_cancelled` ≤100ms、deadline
  `LOGIN_TIMEOUT`=15min + device_code 案内、accept socket に
  `set_read_timeout(5s)` + 8KiB cap、Pi parity の request ordering:
  404/`state` param が expected と一致しない(欠落含む)→
  400-continue/409/error-fail/400-continue/settle、settle 後も
  exchange 完了まで listener 保持 — 409 は post-exchange drain
  (exchange 完了後の non-blocking accept で backlog の接続へ 409
  を返してから drop。Pi の live 409 との accepted divergence))→
  bind 失敗は `AuthEvent::Info` +
  `AuthPrompt::ManualCode`(`parse_authorization_input`、state 不一致は
  abort)→ `POST /oauth/token` form exchange。
- **T5 — device-code login path**(plan step 21-25、T3 に依存):
  `POST /api/accounts/deviceauth/usercode` JSON `{client_id}`(404 →
  "not enabled" message、`{device_auth_id, user_code, interval}` 全必須、
  interval は number または numeric string)→ `AuthEvent::DeviceCode`
  (`/codex/device`、`interval_seconds: Some(interval)`、
  `expires_in_seconds: Some(900)`=LOGIN_TIMEOUT、Pi parity)→
  `poll_device_code_flow`(`http.send` で status
  写像: 200-both-fields→Complete / 403・404→Pending / error string or
  `{code}` object→`deviceauth_authorization_pending` Pending・
  `slow_down` SlowDown・その他 Failed;`wait_before_first_poll: false`、
  deadline `LOGIN_TIMEOUT`、`code_verifier` は `&mut Option<String>`
  capture)→ `redirect_uri=DEVICE_REDIRECT` で exchange。
- **T6 — catalog 同期**(done — plan step 26-29、T2 に依存 — manifest sha256
  assertion が `sha2` を使用。T1/T3-T5 とは並行可):
  pi-ai 1.1.0 `providers/data/openai-codex.json` 取得(canonical は
  unpkg。local `Temp\pi-latest` extraction は machine 固有 cache で
  内容一致 cross-check のみに使う)→ `type=="chat"` filter + `type`
  field 除去 + `chat:` key prefix 除去で `data/openai-codex.json` を
  置き換え(9 model)→ `.manifest.json` の
  `files["openai-codex.json"]` sha256 更新 → `bundled.rs`
  `#[cfg(test)]` に同期 assertion を追加(`bundled_models
  ("openai-codex")` がちょうど 9 id で `gpt-6-luna` 含み
  `gpt-5.4`/`gpt-5.4-mini` を含まない + manifest sha256 が埋め込み
  bytes と一致)→ `mise exec -- cargo test -p cheaper-runtime
  --test catalog` と `mise exec -- cargo test -p cheaper-runtime`
  が緑。
- **T7 — oauth flow unit tests**(plan step 30-38、T4+T5 に依存):
  scripted `AuthInteraction` + mock `HttpClient`(`/oauth/token` 応答を
  channel/barrier で block 可)+ fake `FlowSleeper` + pre-bound
  listener(`127.0.0.1:0`)で browser happy path(実 `TcpStream`
  callback + authorize URL 全 param assert)/bind 失敗→ManualCode/
  Select 未知 id → "Unknown login method"/state 不一致 continue/
  `?error`+state fail/settle 後 409(post-exchange drain —
  barrier-blocked exchange 中に接続を積み exchange 後に 409)/
  browser・device の exchange 失敗 → `ProviderFailure` + 非永続化/
  deadline timeout(実時間を待たない seam)/device Pending→Complete/
  `parse_authorization_input` table/`credentials_from_token`/
  `refresh` invalid_grant/`to_auth` header を緑にする。
- **T8 — 全体検証 + 実 credential smoke**(plan step 39-41、T1・T6・
  T7 に依存): `mise exec -- cargo test -p cheaper-rig-adapter`・
  `mise exec -- cargo test -p cheaper-runtime`・`mise run check`
  全緑。実 credential
  smoke(credential 入手可能な環境のみ): `auth login openai-codex` →
  `models --provider openai-codex` が `gpt-6-luna` 含む `gpt-6-*` を
  列挙 → `run bulk-reader --model openai-codex/gpt-6-luna` が
  `ok:true`。skip 時は理由を impl record に記録。
- **T9 — impl record + audit**(plan step 42-43、T8 に依存):
  `new_impl_record.js --task <本 doc>` で impl record を作成し実施
  内容・smoke 結果を記録。`doc_status.js audit --type plan/task/
  test-spec` の findings を解消する。

依存関係: T2 → T3 → T4・T5 → T7 → T8 → T9、T1 は独立、T6 は T2 に
依存(T8 は T1・T6・T7 に依存)。T4/T5 は同一 file のため同一 agent で
直列が安全。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-openai-codex-subscription-provider.md`
  (本 task が implement する plan — File Map・Implementation Sequence・
  Verification の正)
- `docs/specs/openai-codex-subscription-provider.md`
  (SPEC-034CWHveVPBJp33jPRTCen — Req 1-8・Acceptance Criteria)
- `docs/adr/openai-codex-subscription-provider-wire-oauth-and-catalog.md`
  (ADR-034CWHw0NjYQjdE5o4kErV — Option A 確定決定 + dep 事前宣言)
- `docs/designs/openai-codex-subscription-provider.md`
  (DESIGN-034CXyXncBigEivCETJLBD、approved — module 契約・request
  ordering・edge-case table・Failure Modes)
- `docs/test-specs/openai-codex-subscription-provider.md`
  (TSPEC-034CYp5Xbp8sJ8ZV199qV3 — 本 task が satisfy する保証)

## Dependencies

- plan `PLAN-034CYiZ4kkOoa3oKL7PnLj` の `status` が `approved` 以上に
  なることが本 task の着手条件(TASK-DOC-GATE-001。plan は review
  gate 待ちの `draft` で作成済み — gate 通過後に開始する)。
- **T2 の依存追加は AGENTS.md「Ask first」に基づく user confirmation
  が必須**。承認なしに `Cargo.toml` を編集しない。
- upstream catalog は pi-ai 1.1.0。canonical source は unpkg の
  published tarball(`https://unpkg.com/@earendil-works/pi-ai@1.1.0/
  dist/providers/data/openai-codex.json`)。
  `C:\Users\akiyoshi\AppData\Local\Temp\pi-latest\package\dist\providers\
  data\openai-codex.json` の local extraction は当該 machine 固有の
  一時 cache で他環境には存在しない — 存在する場合は内容一致の
  cross-check にのみ使う。
- 実 credential smoke は credential 入手可能な環境が前提(`~/.codex`
  session または対話 login)。入手不能時は skip 理由を impl record に
  記録する。

## Affected Files

- `crates/cheaper-rig-adapter/src/builtin.rs` — `normalize_codex_base_url`
  + codex arm 配線(既存未 commit 変更を含む)
- `crates/cheaper-runtime/Cargo.toml` — `sha2`/`getrandom`/`base64`
- `crates/cheaper-runtime/src/auth/oauth/openai_codex.rs` — 新設
- `crates/cheaper-runtime/src/auth/oauth/mod.rs` — module・registry 登録
- `crates/cheaper-runtime/src/auth/mod.rs` — `OpenAiCodexFlow` re-export
- `crates/cheaper-runtime/src/lib.rs` — `OpenAiCodexFlow` re-export
- `crates/cheaper-runtime/src/catalog/bundled.rs` — `#[cfg(test)]` に
  openai-codex 同期 assertion + manifest sha256 test 追加
- `crates/cheaper-runtime/src/catalog/mod.rs` — `SUPPORTED_APIS`(未
  commit 変更を T1 commit に含める)
- `crates/cheaper-runtime/src/catalog/data/openai-codex.json` —
  upstream 同期
- `crates/cheaper-runtime/src/catalog/data/.manifest.json` — sha256 更新
- `docs/impl/ir/` — impl record(T9)

## Done When

- [ ] plan の Implementation Sequence step 1-43 が全て完了し、各 Phase
      の semantic commit が積まれている
- [ ] `openai-codex-responses` model が `INVALID_MODEL_CONFIG` にならず、
      catalog `https://chatgpt.com/backend-api` が `.../codex` に正規化
      されて wire に渡る
- [ ] `auth login openai-codex` の browser PKCE / device-code / bind
      失敗 ManualCode fallback が mock HTTP + scripted interaction で
      検証済み(TSPEC-034CYp5Xbp8sJ8ZV199qV3 の Rules/Examples を満たす)
- [x] bundled `openai-codex.json` が 9 model で
      `.manifest.json` sha256 と整合する
- [ ] credential・token・account id が stdout・log・commit に出ない
      (spec Req 8)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-rig-adapter` exits 0
      (normalize 写像・codex arm 構築・JWT 抽出・header 二重送信防止)
- [ ] `mise exec -- cargo test -p cheaper-runtime` exits 0
      (oauth flow mock test 一式・catalog bundled parse を含む)
- [ ] `mise run check` exits 0
- [ ] `grep -n "openai-codex" crates/cheaper-runtime/src/auth/oauth/mod.rs`
      が `builtin_oauth_flows` 登録行を返す
- [x] `bundled.rs` `#[cfg(test)]` の unit test が `.manifest.json`
      `files["openai-codex.json"]` sha256 と埋め込み bytes の一致、
      および `bundled_models("openai-codex")` の 9-model set
      (`gpt-6-luna` 含有・`gpt-5.4`/`gpt-5.4-mini` 非含有)を検証して
      緑(`mise exec -- cargo test -p cheaper-runtime` に含まれる。
      `python` 等の外部 script による ad-hoc 検証は使わない)
- [ ] Manual(環境依存): `cheaper models --provider openai-codex` が
      `gpt-6-luna` を列挙し `cheaper run bulk-reader --model
      openai-codex/gpt-6-luna` が `ok:true`(credential 入手不能時は
      impl record に skip 理由を記録)

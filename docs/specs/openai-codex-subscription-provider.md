---
id: "SPEC-034CWHveVPBJp33jPRTCen"
type: "spec"
status: "implemented"
title: "OpenAI Codex subscription provider"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "docs/discovery/openai-codex-subscription-provider.md"
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
    - "../discovery/openai-codex-subscription-provider.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OpenAI Codex subscription provider

## What Should Be Built

builtin `openai-codex` provider(ChatGPT subscription backend、
`https://chatgpt.com/backend-api/codex/responses`)を Rust runtime で
end-to-end 利用可能にする。

- `auth login openai-codex` で OAuth login(browser PKCE + device-code
  fallback)し、credential を永続化・refresh する
- `cheaper models --provider openai-codex` が login 後に利用可能 model を
  返す
- `cheaper run --model openai-codex/<id>` が Codex wire contract
  (`store:false` 等)で実 endpoint と streaming 完走する
- bundled `openai-codex` catalog を実 backend が受理する model list へ
  更新する

## Why Now

- catalog は `openai-codex-responses` model を 8 件収録するが、adapter・
  OAuth flow ともに未実装で選択すれば `INVALID_MODEL_CONFIG` — 実質 dead
  entry。
- 実 endpoint smoke で subscription credential が有効であること、wire
  contract さえ満たせば `gpt-6-luna` で run が完走することが確認済み。
- bundled catalog の model list は実 backend の受理 list と乖離
  (`gpt-5.4` 系は ChatGPT account で拒否)し、最新 upstream
  (pi-ai 1.1.0)は受理 list と一致する更新済み catalog を持つ。

## Users and Value

- cheaper CLI ユーザー: ChatGPT Plus/Pro subscription で frontier 級
  model(gpt-6 系)を `costUsd` 計量なしの subscription 枠で worker に
  使える。

## Scope

### In Scope

- `openai-codex-responses` api kind の supported 化 + Rig `chatgpt`
  dialect への配線(実装済みの作業 tree 変更を正式化)
- `openai-codex` OAuth flow(browser PKCE `localhost:1455` loopback +
  bind 失敗時 `ManualCode` fallback + `Select` 選択の device-code
  経路)の
  `builtin_oauth_flows` 登録、`refresh`、`toAuth`
  (`{apiKey: access}` + `account_id` → `chatgpt-account-id` header)
- bundled `openai-codex` catalog の model list 更新(pi-ai 1.1.0 基準)
- 上記の単体テスト + 実 credential smoke(`gpt-6-luna`)

### Out of Scope

- Codex WebSocket transport / zstd request 圧縮 / prompt-cache session
  reuse(Pi が持つ高度経路)— SSE streaming で十分に成立する
- `sampling_params`/`compat`/`model.headers` の request 配線の一般化
  (adapter 全体の既知 gap、別 chain)
- `service_tier` / reasoning effort の CLI 露出
- `gpt-reserve`・`codex-auto-review` 等、catalog 未収録 model の追加 —
  user `models.json` で追加可能(非 plugin provider は additive merge で
  ないため従来規則に従う)

## Requirements

- Req 1: `api: "openai-codex-responses"` の model は
  `INVALID_MODEL_CONFIG` にならず、Codex contract(`store:false`、
  SSE streaming、`reasoning.encrypted_content` include、sampling/storage
  field 除去、`originator`/`session_id`/`ChatGPT-Account-Id` header)を
  満たす request を発行する。
- Req 2: `chatgpt-account-id` は `toAuth` の header(credential
  `account_id` 由来)を優先し、無ければ access token JWT の nested claim
  `{"https://api.openai.com/auth": {"chatgpt_account_id": ...}}` から抽出
  する。解決不能でも構築は失敗させない(backend が 400 を返す運用可視性
  を残す)。oauth 経路では provider configured `headers` の merge は行わ
  ない(github-copilot と同じ既存挙動)。
- Req 3: `auth login openai-codex` は browser PKCE flow を既定とする。
  loopback listener(`127.0.0.1:1455`)の bind に成功した場合は callback を
  poll して待ち、bind 失敗時は `ManualCode` prompt で code 貼り付けに
  切り替える(sequential fallback — 現在の同期 `AuthInteraction` seam で
  は callback と stdin prompt の真の race は表現不能なため)。headless
  向けに device-code flow を `Select` prompt で選択可能にする。
- Req 4: token exchange/refresh は `https://auth.openai.com/oauth/token`
  の form POST で、credential は `{access, refresh, expires(epoch ms),
  account_id}` で永続化する。`expires` は token response の `expires_in`
  から算出(無ければ access JWT の `exp` claim)。expiry window 内では
  refresh が走り、`invalid_grant` 系の refresh 失敗は re-login を促す
  message を含む error に写像する。
- Req 5: `toAuth` は `{api_key: access, headers: {"chatgpt-account-id":
  account_id}}` を返し、wire 側と JWT fallback のどちらでも account id が
  到達する。
- Req 6: bundled `openai-codex` catalog は upstream pi-ai 1.1.0 の
  `openai-codex.json` を基準に同期する。upstream list を authoritative
  とし、個別 account/subscription tier による受理差異(例:
  `gpt-5.3-codex-spark` が一部 account で拒否される)は runtime の
  PROVIDER_FAILURE として表面化する設計とする — bundled list 側で
  account 固有の制約を予測しない。
- Req 6a: catalog model の `baseUrl`(`https://chatgpt.com/backend-api`)は
  Pi `resolveCodexUrl` 相当の正規化で `.../codex` segment を補ってから
  wire へ渡す(rig dialect は `base + "/responses"` を組み立てるため、
  catalog 値を素通しすると `/backend-api/responses` になり 404 する)。
- Req 7: catalog/provider error は既存の warning 経路(stderr、
  `--quiet` 抑制)を踏襲し、stdout envelope・exit code は不変。
- Req 8: credential・token・account id を stdout/log/commit に出力しない。

## Acceptance Criteria

- [ ] `openai-codex-responses` model の unit test が wire 構築まで通る
- [ ] `auth login openai-codex` の OAuth flow が mock HTTP で
  browser/device 両経路・state mismatch・token refresh を検証済み
- [ ] 実 credential で `cheaper run bulk-reader --model` が
  `gpt-6-luna` で `ok:true` を返す(subscription backend、usage 記録)
- [ ] `cheaper models --provider openai-codex` が更新後 catalog の
  `gpt-6-*` 系を列挙する
- [ ] `mise run check` 全緑

## Deferred Design Concerns

- Codex websocket/zstd/session-cache transport | SSE で要件成立 |
  backend が SSE を制限した場合 | 高負荷時の帯域・接続数
- `reasoning`/`service_tier` の user 露出 | 現行 CLI option に無い |
  tier 選択要求が来た場合 | 料金・速度の最適化機会損失

## Sources and Relations

- Discovery: `docs/discovery/openai-codex-subscription-provider.md`
  (実 endpoint smoke の切り分け結果)
- Pi 参照実装: `@earendil-works/pi-ai@0.85.1/1.1.0`
  `api/openai-codex-responses.js`、`auth/oauth/openai-codex.js`、
  `providers/data/openai-codex.json`
- rig 0.43: `providers/chatgpt`(DIALECT + `ResponsesContract::Codex`)
- ISS-23(deferred oauth flow 登録)、task
  `docs/tasks/phase-3-port-runtime-adapter-providers.md`

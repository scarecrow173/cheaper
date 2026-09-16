---
id: "SPEC-0011"
type: "spec"
status: "implemented"
title: "cheaper auth command for provider login without a pi installation"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0009-apm-self-contained-cli-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0016-auth-command.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# SPEC-0011: cheaper auth command for provider login without a pi installation

## 背景

production provider E2E は OpenAI を target にするが、ユーザーは API key を
持たず ChatGPT サブスクリプションのみ持つ。Pi は `openai-codex` provider
（ChatGPT OAuth）を built-in で持つが、login は `pi` CLI の TUI 経由であり、
`pi` 本体のインストールを前提にしたくない。`ModelRuntime.login(providerId,
"oauth", interaction)` は public API で credential を `auth.json` へ永続化
するため、cheaper CLI がこの flow を直接吸収できる。

## 要求

1. `cheaper auth login <provider>`: provider の OAuth flow を実行し
   credential を Pi auth store（`~/.pi/agent/auth.json`）へ保存する。
   `--method browser|device` で select prompt を自動回答できる。
2. `cheaper auth status`: 保存済み credential と provider 別の readiness を
   表示する（secret 値は出さない）。
3. `cheaper auth logout <provider>`: credential を削除する。
4. interaction は terminal readline で実装し、`auth_url`/`device_code`/
   `progress`/`info` event を stderr へ表示する。browser method では
   default browser の起動を best-effort で試行する（失敗しても URL 表示で
   進行可能）。
5. bundled CLI（`.apm/hooks/cli`）でも動作すること — `pi` install 不要。
6. 非対話環境（stdin 非 TTY）では browser method の manual_code prompt が
   読み取れないため device method を案内する等、分かりやすく失敗させる。

## 非要求

- ~~api_key 型 provider の login~~（実装済み: `--type api_key` で
  `runtime.login(providerId, "api_key", interaction)` を呼ぶ。secret prompt
  は共通の interaction bridge が処理）
- OAuth flow の再実装（pi-ai の provider-owned flow を使う）
- TUI/リッチ UI

## Acceptance Criteria

- [x] `cheaper auth status` が credential 一覧を envelope/text で返す
- [x] `cheaper auth login openai-codex --method device` が device code を
      表示して待機する（実 OAuth 完了はユーザー操作）
- [x] `cheaper auth logout openai-codex` で credential が消える
- [x] 未知 provider / 未対応 type で適切な error + exit code
- [x] bundle 再生成後も `auth` コマンドが deploy 済み bundle で動く

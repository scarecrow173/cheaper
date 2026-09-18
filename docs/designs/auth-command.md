---
id: "DESIGN-034r05iHy2gdnoJ5j3v9Nk"
type: "design"
status: "approved"
title: "cheaper auth command design (provider OAuth absorbed by the CLI)"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/auth-login-command.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/auth-login-command.md"
  implemented-by:
    - "../plans/auth-command.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/cli-bundle-layout.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-034r05iHy2gdnoJ5j3v9Nk: cheaper auth command design

## コマンド面

```
cheaper auth status
cheaper auth login <provider> [--method browser|device]
cheaper auth logout <provider>
```

- `--format`/`--quiet` は既存共通 option に従う。`auth status` は envelope
  result に `{credentials: [{providerId, type}], providers: [{id, ready,
  reason?}]}` を返す
- `login`/`logout` は interactive/状態変化コマンドで envelope result は
  `{provider, authenticated: true}` / `{provider, loggedOut: true}`
- `auth login` は worker run ではないため `worker: null`

## 実装

`packages/cli/src/auth.ts`:

- `createModelRuntime()`（runtime-pi 経由、既定 `~/.pi/agent/`）を生成
- `status`: `runtime.listCredentials()` + `runtime.checkAuth(id)` /
  `getProviderAuthStatus` で readiness を集める。secret は出力しない
- `login`: `runtime.login(providerId, "oauth", interaction)`。
  provider 不在 → `INVALID_REQUEST` exit 2。oauth 非対応 provider →
  同 error
- `logout`: `runtime.logout(providerId)`。credential 無しでも冪等成功
  （Pi 側が no-op）か、未保存なら message 付き成功でよい

### AuthInteraction（terminal 実装）

`AuthInteraction` = `{signal, prompt, notify}`:

- `prompt({type:"select"})`: `--method` が options.id に一致すれば即答。
  なければ option を番号付きで stderr 表示して readline で選択。
  stdin 非 TTY かつ `--method` 未指定 → 使える method 一覧付きで error
- `prompt({type:"manual_code"|"text"})`: readline question
- `prompt({type:"secret"})`: raw-mode no-echo 入力（readline は入力を
  echo するため API key が画面に残るのを防ぐ）。`\x03`/AbortSignal で
  中断。credential 値は stdout/stderr/envelope に出さない
- `notify("auth_url")`: URL を stderr へ大きく表示 + `start`/`open`/
  `xdg-open` で best-effort 起動（失敗 ignore）
- `notify("device_code")`: userCode + verificationUri + 有効秒数を表示
- `notify("info"|"progress")`: message を stderr へ

`signal` は CLI SIGINT handler に接続し Ctrl+C で flow を中断する。

## 依存関係

新規 dependency なし。`ModelRuntime.login` と pi-ai の provider-owned
OAuth flow（`openaiCodexOAuth` は browser callback server + device code の
両方を内蔵）を使う。bundled CLI に自動的に含まれる。

## 検証

- unit: interaction adapter（select 自動回答・非 TTY error）と
  args parsing、status envelope shape
- 手動/ユーザー操作: `auth login openai-codex` → auth.json 保存 →
  `bulk-read` が `openai-codex` provider で走ることを providers.md に記載

---
id: "SPEC-034r05iHK8bA7gmLlq6ikw"
type: "spec"
status: "implemented"
title: "Auth tests do not launch the OS default browser"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/8"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/auth-url-opener-seam.md"
  depends-on:
    - "./auth-login-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
    - "../designs/auth-command.md"
    - "./repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../adr/inject-url-opener-seam-for-auth-interaction.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# Auth tests do not launch the OS default browser

## What Should Be Built

`pnpm test` を含む通常のテスト実行が、OS の既定 browser（`rundll32` /
`open` / `xdg-open` 等の外部プロセス）を一切起動しないことを保証する。
同時に、本番の `cheaper auth login` が `auth_url` event で既定 browser を
best-effort 起動する既存動作は維持する。browser 起動要求は、実 OS
プロセスを spawn せずに検証できる seam を通じて観測可能にする。

## Why Now

現在 `packages/cli/test/auth.test.ts` の `--method device` テストは fake
runtime が `auth_url` event を emit するため、`createTerminalInteraction`
の `notify` 経由で `openUrlBestEffort()` が OS の URL opener を spawn する。
テストが検証したいのは method 選択・URL/device code の stderr 表示・
envelope・secret 秘匿であり、browser 起動は検証対象ではない。外部 OS
副作用を持つテストは hermetic でなく、CI や sandbox 環境で余計な window /
process を発生させる。

SPEC-034r05iHK8bA7g9KcsCqKj が定めた test hermeticity の方向と矛盾する状態を解消する。

## Users and Value

- 開発者・CI: テスト実行で外部 browser が開かず、環境非依存の結果を得られる。
- 本番利用者: OAuth browser flow の自動起動という既存 UX を維持できる。
- テスト作成者: browser 起動要求を spy/no-op seam で検証できる。

## Scope

### In Scope

- `auth_url` event 処理における browser opener の注入可能化。
- `packages/cli/test/auth.test.ts` が実 OS opener を呼ばないこと。
- browser 起動要求の有無と対象 URL を spy で検証できること。
- device flow テストが無関係な browser 副作用を持たないよう fake event
  を整理すること。
- 本番既定動作（best-effort で OS opener を起動し、失敗しても flow を
  中断しない）の維持。

### Out of Scope

- `device_code` event の表示・flow 自体の変更。
- 実 browser / 実 OAuth provider を使う検証。
- `auth` 以外の command の外部プロセス呼び出し。
- `NODE_ENV === "test"` による本番コードの分岐（採用しない方針）。

## Requirements

1. **テストの hermetic 化**: 通常のテスト実行が OS の URL opener
   プロセスを spawn しない。
2. **注入可能な seam**: browser 起動は本番コードが直接 OS opener を
   呼ぶのではなく、注入可能な依存として扱う。本番既定は現行の
   OS 別 opener と同じ挙動とする。
3. **観測可能性**: テストは起動要求の有無と要求 URL を spy/no-op
   実装で assert できる。
4. **本番動作の維持**: 実 browser-based OAuth flow の `auth_url` で既定
   browser 起動を試行し続け、URL も stderr へ表示し続ける。起動失敗は
   best-effort のまま auth flow を中断しない。
5. **fake event の分離**: `--method device` のテストは、検証対象でない
   browser-auth event を emit しない形へ整理する。browser URL handling と
   device-code handling は実用上独立して検証できること。
6. **既存カバレッジの維持**: auth URL 表示、device-code 表示、method
   選択、login envelope、secret 秘匿の既存検証を維持する。

## Acceptance Criteria

- [ ] `pnpm test` が system default browser を起動しない（OS opener
      プロセスが spawn されないことを spy またはプロセス非発生で示せる）。
- [ ] `packages/cli/test/auth.test.ts` が auth URL 表示、device-code
      表示、method 選択、login envelope、secret 秘匿を引き続きカバーする。
- [ ] browser 起動要求が spy/no-op seam を通じて検証され、`open` /
      `xdg-open` / `rundll32` の実 spawn を伴わない。
- [ ] 本番の `auth_url` 処理が既定で OS opener を best-effort 起動する
      コード経路を維持している（既定 opener が OS 別コマンドへ分解される
      実装が残る）。
- [ ] browser 起動失敗が auth flow を中断しない契約が維持される。
- [ ] `--method device` のテストが browser 起動要求を発生させない。
- [ ] 既存 auth テストと全テストスイートが通る。

## Sources and Relations

- Issue #8: 問題の報告と実装方針（injectable seam、fake event 分離）の起点。
- SPEC-034r05iHK8bA7ghSWRbNU8 / ADR-034r05iHrZUE6W71rbZ6aR / DESIGN-034r05iHy2gdnoJ5j3v9Nk: `cheaper auth` の既存契約。本仕様は
  browser 起動の UX を維持したままテスト seam を追加する。
- SPEC-034r05iHK8bA7g9KcsCqKj: test hermeticity の既存方針。本仕様はその延長の gap 修正。

seam の具体的な形（callback、hooks 拡張等）は後続設計で確定する。
本仕様では実装方式を選定しない。

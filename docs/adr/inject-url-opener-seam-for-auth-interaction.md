---
id: "ADR-034r05iHrZUE6WcElqgQIr"
type: "adr"
status: "accepted"
title: "auth interaction の browser 起動を注入可能な URL opener seam にする"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/auth-tests-do-not-launch-os-browser.md"
    - "https://github.com/scarecrow173/cheaper/issues/8"
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
    - "./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/auth-command.md"
    - "../specs/repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/auth-tests-do-not-launch-os-browser.md"
  derived-by:
    - "../designs/auth-url-opener-seam.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 18. auth interaction の browser 起動を注入可能な URL opener seam にする

## Context and Problem Statement

`createTerminalInteraction()` の `notify` は `auth_url` event を受けるたびに
`openUrlBestEffort()` を呼び、OS の URL opener（`rundll32` / `open` /
`xdg-open`）を spawn する。テストでは fake runtime が `auth_url` を emit
するため、`pnpm test` が実際に system default browser を開いてしまう。

確認済みの現状は次のとおりである。

- `packages/cli/src/auth.ts` の `notify` が `auth_url` で
  `openUrlBestEffort(event.url)` を直接呼ぶ。
- `openUrlBestEffort` は platform 別に `spawn` する module-local 関数であり、
  呼び出し側から差し替える経路がない。
- `packages/cli/test/auth.test.ts` の `--method device` テストは、検証対象と
  無関係な `auth_url` を emit するため browser 起動を引き起こす。
- `executeAuth` は既に `hooks` 引数で `createModelRuntime` と `signal` を
  受け、`CliHooks` には `runWorker` / `builtinDir` / `now` 等の test seam が
  集約されている。

## Decision Drivers

- 通常のテスト実行が外部 OS プロセスを spawn しない（SPEC-034r05iHK8bA7g9KcsCqKj の
  test hermeticity と整合）。
- 本番の browser 起動 UX と best-effort 失敗許容を維持する。
- 既存の `CliHooks` seam パターンと一貫した最小変更にする。
- `NODE_ENV === "test"` のような環境分岐を本番コードへ持ち込まない。

## Considered Options

- **Option 1**: `CliHooks` に URL opener callback を追加し、
  `executeAuth` から `createTerminalInteraction` へ渡す。既定は現行の
  `openUrlBestEffort`。
- **Option 2**: 本番コードに `NODE_ENV === "test"` 分岐を入れて opener を
  no-op にする。
- **Option 3**: テスト側で `node:child_process` の `spawn` を module mock
  して抑止する。
- **Option 4**: `notify` から browser 起動を切り離し、`auth_url` の
  表示だけを interaction が担い、起動判断を `executeAuth` 側の別層へ移す。

## Decision Outcome

**Option 1** を採用する。`CliHooks` に `openUrl?: (url: string) => void`
相当の seam を追加し、`executeAuth` が `createTerminalInteraction` へ渡す。
注入がない場合は現行の `openUrlBestEffort` を既定にする。

理由は次のとおり。

- 既存の `createModelRuntime` / `runWorker` / `signal` と同じ注入面に乗るため、
  CLI の test seam が一箇所に集約され表現ドリフトを起こさない。
- 本番既定動作が変わらず、テストだけが spy/no-op を注入できる。
- Option 2 は環境変数による暗黙分岐であり、issue の明示的な却下条件に該当する。
- Option 3 は `spawn` の module mock で実装詳細に結合し、child_process を
  使う他の経路へ誤って影響しうる。
- Option 4 は責務分離としては妥当だが、`auth_url` 表示と起動を分ける変更面が
  本題（test seam）より大きく、YAGNI に反する。

`createTerminalInteraction` の `opts` は `openUrl` を受け取れる形にし、
テストでは hooks 経由の注入と direct 注入の両方を可能にする。
また `--method device` テストの fake event から無関係な `auth_url` を
取り除き、browser URL handling の検証は spy を伴う専用テストへ分離する。

## Consequences

- テストは実 OS opener を spawn せず、起動要求 URL を spy で assert できる。
- 本番 CLI は hooks 未指定時に現行どおり OS opener を best-effort 起動する。
- `CliHooks` の公開 seam が 1 つ増える。bundled CLI の外部 contract
  （stdin/stdout envelope）は変わらない。
- `auth_url` の表示と起動要求は引き続き `notify` の同一箇所で行われるため、
  URL 表示だけの検証と起動要求の検証は spy の有無で制御する。

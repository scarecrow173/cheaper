---
id: "TASK-034r05iHQeQpDqovKmKaZC"
type: "task"
status: "done"
title: "Auth URL opener seam and test isolation"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
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
    - "docs/plans/auth-url-opener-seam.md"
  implemented-by: []
  depends-on:
    - "docs/plans/auth-url-opener-seam.md"
  blocks:
    - "docs/tasks/auth-url-opener-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/auth-url-opener-seam.md"
    - "docs/specs/auth-tests-do-not-launch-os-browser.md"
    - "docs/adr/inject-url-opener-seam-for-auth-interaction.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Auth URL opener seam and test isolation

## Work

PLAN-034r05iHE1Fbkp4xxlyMdV / DESIGN-034r05iHy2gdnoOvi1vrou の実装 task。browser 起動を `CliHooks.openUrl`
経由の注入 seam にし、テストを実 OS プロセスから分離する。

### 1. `packages/cli/src/cli.ts`

`CliHooks` に `openUrl?: (url: string) => void` を追加する。既存 seam
（`createModelRuntime` / `runWorker`）と同じ doc comment 形式で
「browser 起動差し替え seam（単体テスト用）」旨を記す。

### 2. `packages/cli/src/auth.ts`

- `createTerminalInteraction` の `opts` に `openUrl?: (url: string) => void`
  を追加する。`notify` の `auth_url` 分岐では
  `(opts.openUrl ?? openUrlBestEffort)(event.url)` を呼ぶ。
  既定値はこの一箇所のみで定義する。
- `executeAuth` は `hooks.openUrl` を `createTerminalInteraction` へ中継する。
  `hooks.openUrl` 未指定時は `undefined` を渡し、interaction 側の既定に
  委ねる（`executeAuth` 側に既定値を持たせない）。

### 3. `packages/cli/test/auth.test.ts`

- `--method device` テストの fake `login` から `auth_url` emit を除去する。
  このテストは method 選択・device code 表示・envelope・secret 秘匿を
  検証するものであり、browser 起動要求を発生させない。
- browser 起動要求の専用テストを追加する。`hooksFor(runtime, { openUrl: spy })`
  で注入し、fake が `auth_url` を emit したとき spy が
  `https://example.test/auth` で 1 回呼ばれること、URL が stderr に
  表示されることを assert する。`open` / `xdg-open` / `rundll32` の
  spawn は発生しない（注入 seam が既定 opener を置き換えるため）。
- spy 未注入でも fake が `auth_url` を emit しない限り起動要求は
  発生しないことを、device テスト側で `openUrl` spy 未呼出として
  示してもよい。

## Acceptance

- `pnpm test` が OS opener を spawn しない（`--method device` が起動
  要求を発生させず、`auth_url` を扱うテストは spy 経由）。
- spy が要求 URL で呼ばれる専用テストがある。
- `auth_url` の stderr 表示（URL・instructions）は変更しない。
- `createTerminalInteraction` 未注入時の既定が `openUrlBestEffort`
  であること。

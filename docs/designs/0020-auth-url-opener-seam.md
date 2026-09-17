---
id: "DESIGN-0020"
type: "design"
status: "approved"
title: "Auth URL opener seam (injectable browser launch)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
    - "../adr/0018-inject-url-opener-seam-for-auth-interaction.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
  implemented-by:
    - "../plans/0015-auth-url-opener-seam.md"
  depends-on:
    - "./0016-auth-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/0018-inject-url-opener-seam-for-auth-interaction.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
    - "../adr/0018-inject-url-opener-seam-for-auth-interaction.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-0020: Auth URL opener seam

## Context

`createTerminalInteraction()` の `notify` が `auth_url` event で
`openUrlBestEffort()` を直接呼び、OS の URL opener を spawn する。
SPEC-0015 の要求に従い、browser 起動を `CliHooks` 経由の注入可能な依存に
変え、テストが実プロセスを spawn せず起動要求を観測できるようにする
（ADR-0018 Option 1）。

## Scope

- `CliHooks` に `openUrl?: (url: string) => void` を追加する。
- `executeAuth` が `hooks.openUrl` を `createTerminalInteraction` へ渡す。
- `createTerminalInteraction` の `opts` に `openUrl` を追加し、未指定時は
  現行の OS 別 opener を既定にする。
- `auth.test.ts` の fake event を整理し、browser 起動要求を spy で検証する。

以下は変更しない: `auth_url` / `device_code` の stderr 表示内容、OAuth /
device flow 自体、envelope 形、起動失敗時の best-effort 継続。

## Components and Boundaries

```text
runCli(..., hooks)
  └─ executeAuth(args, io, hooks, startedAt)
       └─ createTerminalInteraction(io, { method, signal, openUrl })
            └─ notify("auth_url")
                 ├─ stderr へ URL / instructions を表示（変更なし）
                 └─ openUrl(event.url)   ← 注入 seam
                      既定: openUrlBestEffort（OS 別 spawn, best-effort）
```

- `CliHooks.openUrl` は既存の `createModelRuntime` / `runWorker` /
  `signal` と同じ注入面に置く。bundled CLI の外部 contract
  （argv / stdin / stdout envelope / exit code）は変わらない。
- `createTerminalInteraction` の `opts.openUrl` は hooks 経由でも
  direct でも注入できる optional 引数。`executeAuth` は
  `hooks.openUrl` をそのまま中継し、未指定なら `undefined` を渡して
  interaction 側の既定に委ねる（二重の既定値を持たせない）。
- 既定 opener は `auth.ts` 内の現行実装を維持する。`rundll32` /
  `open` / `xdg-open` の platform 分岐と、spawn 失敗を握りつぶす
  best-effort 契約は変更しない。

## File Map

- `packages/cli/src/cli.ts` — `CliHooks` に `openUrl` を追加（doc comment
  付き）。
- `packages/cli/src/auth.ts` — `createTerminalInteraction` の `opts` に
  `openUrl` を追加。`notify` の `auth_url` 分岐が注入 opener（既定
  `openUrlBestEffort`）を呼ぶ。`executeAuth` が `hooks.openUrl` を中継。
- `packages/cli/test/auth.test.ts` — `--method device` テストから無関係な
  `auth_url` emit を除去し、browser 起動要求は `openUrl` spy で検証する
  専用テストへ分離。

## Test Plan

- `--method device` テスト: `openUrl` spy が呼ばれないこと、または注入せず
  fake が `auth_url` を emit しないことで browser 副作用がないことを示す。
- 専用テスト: fake が `auth_url` を emit したとき spy が要求 URL で
  1 回呼ばれること。`rundll32` / `open` / `xdg-open` の spawn は発生しない。
- 既存カバレッジ（URL 表示、device-code 表示、method 選択、envelope、
  secret 秘匿）は維持する。

## Risks and Trade-offs

- hooks 未指定の本番経路は既定 opener を使うため動作不変。seam を公開
  interface に足す点だけが API 面の差分であり、後方互換を壊さない。
- `openUrlBestEffort` 自体のユニットテストは追加しない（OS 依存・外部
  プロセス）。起動要求が seam に届くことと既定値の存在を検証する。

## Verification Notes

- `pnpm test` で外部 opener プロセスが spawn されないことを spy で確認。
- `pnpm lint` / `pnpm typecheck` / `pnpm test` を通す。

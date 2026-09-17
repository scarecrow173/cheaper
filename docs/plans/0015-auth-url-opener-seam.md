---
id: "PLAN-0015"
type: "plan"
status: "approved"
test-spec-skip: "Verified by package unit test suite (auth.test.ts spy/no-op seam); no dedicated test-spec doc authored."
title: "Injectable auth URL opener seam for hermetic tests"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0020-auth-url-opener-seam.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0020-auth-url-opener-seam.md"
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
  implemented-by:
    - "../tasks/0093-auth-url-opener-seam-and-test-isolation.md"
    - "../tasks/0094-auth-url-opener-final-verification.md"
  depends-on:
    - "../plans/0011-auth-command.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
    - "../adr/0018-inject-url-opener-seam-for-auth-interaction.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0020-auth-url-opener-seam.md"
  derived-by: []
  verifies:
    - "../specs/0015-auth-tests-do-not-launch-os-browser.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-0015: Injectable auth URL opener seam for hermetic tests

## Goal

SPEC-0015 / ADR-0018 / DESIGN-0020 に従い、auth の browser 起動を
`CliHooks.openUrl` 経由の注入可能な seam にし、テストが実 OS プロセスを
spawn せず起動要求を検証できるようにする。本番既定動作は不変とする。

## Scope

単一 implementation stream。`packages/cli/src/{cli,auth}.ts` と
`packages/cli/test/auth.test.ts` に限定される小規模変更のため plan 分割は
しない。

### In Scope

- `CliHooks` への `openUrl?: (url: string) => void` 追加。
- `executeAuth` から `createTerminalInteraction` への `openUrl` 中継。
- `createTerminalInteraction` `opts.openUrl` と既定 `openUrlBestEffort`。
- `auth.test.ts` の fake event 整理と spy 検証への分離。

### Out of Scope

- OAuth / device flow、stderr 表示、envelope、起動失敗許容の変更。
- `openUrlBestEffort` 自体の OS 別分岐の変更やユニットテスト追加。
- `auth` 以外の外部プロセス呼び出しの seam 化。

## File Map

| File | Change |
| --- | --- |
| `packages/cli/src/cli.ts` | `CliHooks.openUrl` 追加（test seam の doc comment）。 |
| `packages/cli/src/auth.ts` | `opts.openUrl` 追加、`notify` が注入 opener を使用、`executeAuth` が中継。 |
| `packages/cli/test/auth.test.ts` | `--method device` テストから `auth_url` emit を除去。spy 検証テストを追加。 |

## Task Breakdown

1. **TASK-0093**: seam 実装と test 分離。`CliHooks.openUrl` →
   `executeAuth` → `createTerminalInteraction` の注入経路を作り、
   `--method device` テストから `auth_url` を外し、browser 起動要求は
   spy テストへ移す。
2. **TASK-0094**: final verification。`pnpm lint` / `pnpm typecheck` /
   `pnpm test` を通し、外部 opener が spawn されないことを確認し、
   SPEC-0015 の status を `implemented` へ同期する。

## Dependency Graph

```text
TASK-0093 (seam + test 分離)
   └─ TASK-0094 (verification)  ※ TASK-0093 完了後のみ着手可
```

## Risk Register

| Risk | Mitigation |
| --- | --- |
| seam 追加で本番既定動作が変わる | 既定値は interaction 側のみで持ち、`executeAuth` は未指定時 `undefined` を中継する。 |
| `--method device` テストの event 削除で URL 表示カバレッジが落ちる | spy テストで `auth_url` の表示と起動要求を独立して検証する。 |
| spawn 非発生の検証が環境依存になる | プロセス監視ではなく、注入 opener が OS opener に到達しない seam 構造で示す。 |

## Rollback Strategy

`CliHooks.openUrl` は optional 追加のため revert しても外部 contract は
変わらない。問題時は本 plan のコミットを revert すれば従来の直接
`openUrlBestEffort` 呼び出しへ戻る。

## Verification Matrix

| Acceptance Criterion (SPEC-0015) | Verification |
| --- | --- |
| `pnpm test` が browser を起動しない | spy 注入下で OS opener 関数が呼ばれないこと、かつ `--method device` が起動要求を発生させないテスト。 |
| auth.test.ts の既存カバレッジ維持 | URL/device-code 表示・method 選択・envelope・secret 秘匿テストが残る。 |
| spy/no-op seam で検証 | `openUrl` spy が要求 URL で 1 回呼ばれる専用テスト。 |
| 本番 OAuth の best-effort 起動維持 | `createTerminalInteraction` 未注入時の既定が `openUrlBestEffort` である実装確認。 |
| 起動失敗で flow 中断しない | 既定 opener の try/catch + `child.on("error")` 契約の維持。 |
| device テストが browser 副作用を持たない | fake event から `auth_url` を除去し、spy 未呼出または emit なしを assert。 |
| 全テスト通過 | `pnpm lint` / `pnpm typecheck` / `pnpm test`。 |

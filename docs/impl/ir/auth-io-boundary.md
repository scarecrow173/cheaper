---
id: "IMPL-2OYQijUxRdsfa8Cig2DuQe"
type: "impl"
status: "completed"
title: "Auth text/select prompt I/O boundary routing (PLAN-zDQIdChPXY1lrGJyzZTiQc)"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/36"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/cli/src/auth.ts"
      - type: "file"
        path: "packages/cli/test/auth.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/auth-io-boundary-prompt-routing-and-tests.md"
    - "docs/tasks/auth-io-boundary-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/auth-io-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/auth-io-boundary.md"
    - "docs/specs/auth-io-boundary.md"
    - "docs/adr/auth-terminal-prompt-io-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/auth-io-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Auth text/select prompt I/O boundary routing (PLAN-zDQIdChPXY1lrGJyzZTiQc)

## Summary

`createTerminalInteraction` の text/select prompt が使う
`createInterface` の `output` を `process.stderr` から injected
`io.stderr` へ変更した。`io.stderr` が `Writable` のときはそのまま
pass-through し、plain object のときだけ write forward の `Writable`
で wrap する conditional pass-through（`instanceof` 1 箇所）。auth
interaction の通常 prompt 出力がすべて injected `CliIo` boundary 内に
入り、TTY select/text prompt が injected `io.stderr` に出ることを
検証する 2 テストを追加した。Issue #36。

## Context

TASK-FP5YNv1aQHfB69NlhCQJbG〜TASK-AZSy7cWtCdKaL5D2ro8TWy
（PLAN-zDQIdChPXY1lrGJyzZTiQc / SPEC-WdXXEoaAmbOafvuIpM6FFT /
ADR-pINoxFu3eduWaJCNBtOT1M / DESIGN-Hz0wDP9E6GjkHNLwOEOGiw）に従う。
Issue #36。

## Implementation

### Changes

- `packages/cli/src/auth.ts`: `createTerminalInteraction` 内に
  `io.stderr instanceof Writable ? io.stderr : forwardWritable` の
  prompt output を 1 箇所追加し、`ask()` の `createInterface` の
  `output` を `process.stderr` からその pass-through へ変更。
  `createInterface` の `output` 型は `WritableStream` を要求するため
  `{ write(s) }` の直接代入は typecheck で通らず、常時 `Writable`
  wrap は `isTTY` を失い本番 echo を消す回帰になるため conditional
  にした（いずれも Node 26 で実測）。本番は
  `io.stderr === process.stderr`（`main.ts` で注入）をそのまま通す
  ため、terminal mode・入力 echo・AbortSignal 連携が現行と同一。
- `packages/cli/test/auth.test.ts`: 2 テスト追加。
  - `routes an interactive TTY select prompt through the injected io.stderr`:
    TTY stdin に `"2\n"` を feed し、prompt message・option labels・
    `select [1-2]:` が injected stderr に出ること、選択 id `device` が
    runtime へ返り login が成功することを検証。
  - `routes a text prompt through the injected io.stderr and returns the
    answer`: `type: "text"` prompt の response が runtime へ渡り、prompt
    message が injected stderr に出ることを検証。

### Alternatives rejected

- `io.stderr` の直接代入（`output: io.stderr`）: `createInterface` の
  `output` 型が `WritableStream` を要求し `pnpm typecheck` で reject。
- `io.stderr` の常時 `Writable` adapter wrap: adapter は TTY 性を
  失わせ、本番 interactive prompt の入力 echo が消える回帰になる
  ため不採用（ADR-pINoxFu3eduWaJCNBtOT1M で比較記録）。
- `CliIo.stderr` の型を `Writable` へ変更: 公開 contract と全 test
  helper を壊すため不採用。

### Notes

- secret prompt は raw-mode 独自 reader（readline 非経由）で既に
  boundary 内にあり、no-echo 動作は不変。
- `openUrlBestEffort` の `spawn` 等、prompt 出力と無関係な経路は
  対象外（issue の方針どおり）。

## Verification

- `pnpm test`（packages/cli）: 157 tests すべて pass。新規 2 テストを
  含む auth.test.ts 17 tests pass。
- 回帰: secret no-echo・`--method` auto-answer・non-TTY select fail・
  auth_url opener seam・device_code・cancellation の既存テストが
  変わらず pass。
- `mise run check`: pass（lint・version・typecheck・test・bundle
  freshness）。

## Follow-ups

- なし。

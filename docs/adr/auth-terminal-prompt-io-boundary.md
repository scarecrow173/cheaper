---
id: "ADR-pINoxFu3eduWaJCNBtOT1M"
type: "adr"
status: "accepted"
title: "auth text/select prompt を injected CliIo の stderr writer へ bridge する"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/auth-io-boundary.md"
    - "https://github.com/scarecrow173/cheaper/issues/36"
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
    - "./inject-url-opener-seam-for-auth-interaction.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/auth-command.md"
    - "../designs/auth-url-opener-seam.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/auth-io-boundary.md"
  derived-by:
    - "../designs/auth-io-boundary.md"
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

# auth text/select prompt を injected CliIo の stderr writer へ bridge する

## Context and Problem Statement

`createTerminalInteraction()` の通常 text/select prompt は
`readline/promises` の `createInterface({ input: io.stdin, output: process.stderr })`
を使い、process-global stderr を直接参照している。`CliIo` の
`io.stderr` は `{ write(s: string): unknown }` の minimal contract で
`stream.Writable` ではないため、そのまま `createInterface` の `output`
には渡せない。注入された I/O boundary 内で readline を使う方法を
決める必要がある。

確認済みの現状は次のとおりである。

- secret prompt は `io.stderr.write()` + raw-mode stdin で独自実装済みで、
  すでに injected boundary 内にある。
- `ask()` 内の `createInterface` だけが `process.stderr` を直接参照する。
- `CliIo` type は `stdout` / `stderr` を `write()` メソッドのみの
  minimal interface として定義し、本番は `process.stdout` /
  `process.stderr`、テストは plain object を注入する。
- issue #36 は「必要以上に terminal abstraction を増やさない」最小構成を
  要求する。

## Decision Drivers

- auth interaction の prompt output をすべて injected CLI I/O boundary 内に
  収める（SPEC-WdXXEoaAmbOafvuIpM6FFT）。
- `CliIo` の公開 contract を壊さない。test seam の型を広げない。
- secret no-echo・`--method` auto-answer・AbortSignal・opener seam の
  既存動作を変えない。
- KISS/YAGNI: 新しい terminal abstraction を持ち込まない。

## Considered Options

- **Option 1**: `io.stderr` を `stream.Writable` で薄く wrap し、
  `createInterface` の `output` へ渡す。write は `io.stderr.write()` へ
  forward するだけの adapter。
- **Option 2**: `CliIo.stderr` の型を `stream.Writable` に変更し、
  `io.stderr` をそのまま `output` へ渡す。
- **Option 3**: readline をやめて select/text prompt も `io.stderr.write()`
  + 独自入力 reader で実装する（secret prompt と同じ方式）。

## Decision Outcome

**Option 1** を採用する。`createTerminalInteraction` 内で `io.stderr` を
forward する最小の `Writable` adapter を 1 箇所だけ生成し、
`createInterface({ input: io.stdin, output: <adapter> })` とする。

理由は次のとおり。

- `CliIo` の公開型は一切変わらず、本番 (`process.stderr`) とテスト
  (plain object) の双方の注入がそのまま動く。表現ドリフトを起こさない。
- Option 2 は `CliIo.stderr` を `Writable` 必須にし、既存の minimal
  contract と全 test helper の型を壊す。issue の「既存 abstraction を
  壊さない最小構成」に反する。
- Option 3 は readline が提供する echo・編集・AbortSignal 連携を自前で
  再実装することになり、変更面が本題より大きく回帰リスクも高い。

## Consequences

- text/select prompt の表示（prompt message・option labels・echo）が
  すべて injected `io.stderr` を経由し、テストで完全に観測できる。
- `process.stderr` への直接参照は `openUrlBestEffort` の OS opener 等、
  auth interaction の通常 prompt 出力とは無関係な経路を除き消える。
- adapter は write forward のみで state を持たず、secret no-echo
  実装には関与しない。

---
id: "DESIGN-Hz0wDP9E6GjkHNLwOEOGiw"
type: "design"
status: "approved"
title: "Auth terminal prompt I/O boundary (injected stderr for text/select prompts)"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/auth-io-boundary.md"
    - "../adr/auth-terminal-prompt-io-boundary.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/auth-io-boundary.md"
  implemented-by:
    - "../plans/auth-io-boundary.md"
  depends-on:
    - "./auth-command.md"
    - "./auth-url-opener-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/inject-url-opener-seam-for-auth-interaction.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/auth-io-boundary.md"
    - "../adr/auth-terminal-prompt-io-boundary.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-Hz0wDP9E6GjkHNLwOEOGiw: Auth terminal prompt I/O boundary

## Context

`createTerminalInteraction()` の通常 text/select prompt は
`readline/promises` の `createInterface({ input: io.stdin, output: process.stderr })`
を使い、`CliIo` boundary の外へ出力している。secret prompt は独自の
raw-mode reader（`io.stderr.write()` + stdin）で既に boundary 内にある。
ADR-pINoxFu3eduWaJCNBtOT1M の Option 1 に従い、`io.stderr` が
`Writable` のときはそのまま、plain object の injected writer のとき
だけ write forward の `Writable` で wrap して `createInterface` の
`output` へ渡す conditional pass-through にする。

## Design

### Prompt output の pass-through

`createTerminalInteraction` 内に prompt 用 output を 1 箇所だけ生成し、
`ask()` の `createInterface` がそれを使う。

```ts
const promptOutput =
  io.stderr instanceof Writable
    ? io.stderr
    : new Writable({
        write(chunk, _encoding, callback) {
          io.stderr.write(chunk.toString());
          callback();
        },
      });

rl ??= createInterface({ input: io.stdin, output: promptOutput });
```

- `createInterface` の `output` 型は `WritableStream` を要求するため、
  `{ write(s) }` の `io.stderr` を直接渡すと `pnpm typecheck` で
  reject される（runtime は受け付けるが型が通らない）。
- 常時 `Writable` wrap は adapter が `isTTY` を持たず readline の
  terminal mode が off になり、本番 interactive prompt の入力 echo が
  消える回帰になる（Node 26 で実測）。`instanceof` pass-through なら
  本番は `process.stderr` をそのまま通し terminal mode・echo・
  AbortSignal 連携が現行と同一。
- `CliIo` type・`io.stderr` contract・本番の `process.stderr` 注入は
  すべて不変。
- `process.stderr` への直接参照は auth interaction の通常 prompt 出力
  経路から消える。

### 変わらないもの

- secret prompt: `io.stderr.write()` + raw-mode stdin の no-echo reader を
  そのまま使う（readline を経由しない）。
- `notify`: `io.stderr.write()` で auth_url / device_code / info /
  progress をそのまま出す。
- `--method` auto-answer・non-TTY fail・AbortSignal/SIGINT/SIGTERM・
  `openUrl` seam・stdout envelope contract: すべて不変。

## Test design

`packages/cli/test/auth.test.ts` の `makeIo` は `io.stdin` に
`Readable` + `isTTY` flag、`io.stderr` に write capture object を注入する。
readline は `input.isTTY === true` でも非 TTY の Readable を受け付けるため、
TTY select/text prompt の動作を injected stream で再現できる。

1. TTY select prompt: fake runtime が `interaction.prompt({ type: "select" })`
   を呼び、stdin へ `"2\n"` を feed。prompt message・option labels が
   injected `io.stderr` に出て、選択した option id が runtime へ返り
   login が成功することを確認する。
2. text prompt: fake runtime が `interaction.prompt({ type: "text" })` を
   呼び、stdin へ入力を feed。response が runtime へ渡り、prompt が
   injected `io.stderr` に出ることを確認する。
3. regression: 既存テスト（secret no-echo・`--method` auto-answer・
   non-TTY select fail・auth_url opener seam・device_code・cancellation）
   が変わらず pass する。

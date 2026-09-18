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
ADR-pINoxFu3eduWaJCNBtOT1M の Option 1 に従い、`io.stderr` を
`stream.Writable` で薄く wrap して `createInterface` の `output` へ渡す。

## Design

### Prompt output adapter

`createTerminalInteraction` 内に、injected writer を `Writable` へ変換する
最小の adapter を 1 箇所だけ置く。

```ts
const promptOutput = new Writable({
  write(chunk, _encoding, callback) {
    io.stderr.write(chunk.toString());
    callback();
  },
});
```

`ask()` の `createInterface` はこの adapter を `output` に使う。

```ts
rl ??= createInterface({ input: io.stdin, output: promptOutput });
```

- adapter は write forward のみで state を持たない。
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

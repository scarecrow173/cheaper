---
id: "SPEC-WdXXEoaAmbOafvuIpM6FFT"
type: "spec"
status: "approved"
title: "Keep interactive auth prompts inside the injected CLI I/O boundary"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/36"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/auth-io-boundary.md"
  depends-on:
    - "./auth-login-command.md"
    - "./auth-tests-do-not-launch-os-browser.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
    - "../adr/inject-url-opener-seam-for-auth-interaction.md"
    - "../designs/auth-command.md"
    - "../designs/auth-url-opener-seam.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../adr/auth-terminal-prompt-io-boundary.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Keep interactive auth prompts inside the injected CLI I/O boundary

## What Should Be Built

`packages/cli/src/auth.ts` の `createTerminalInteraction` が生成する
text/select prompt の terminal 出力を、注入された `CliIo` boundary
（`io.stderr`）内に統一する。現状 `readline/promises` の
`createInterface({ input: io.stdin, output: process.stderr })` が
process-global stderr を直接参照しており、`CliIo` abstraction と test
seam がこの経路だけ破れている。

## Why Now

`runCli(..., io, hooks)` は stdout/stderr/stdin を注入できる構造で、
auth code も基本的にその contract に従っている。text/select prompt のみ
`process.stderr` へ直接出るため:

- test が prompt output を `io.stderr` で完全には観測できない
- embedded / wrapped CLI で injected I/O と実 process stderr が分離する
- auth I/O contract の一部だけ hermetic でない

加えて `packages/cli/test/auth.test.ts` は interactive TTY の通常
select/text prompt が injected stderr に出ることを検証しておらず、
coverage gap がある（GitHub issue #36）。

## Users and Value

- cheaper CLI の利用者: embedded / wrapped 実行でも auth prompt が正しい
  出力先へ届く。
- CLI をテスト・統合する開発者: `io.stderr` 注入だけで auth interaction
  を完全に観測・検証できる。

## Scope

### In Scope

- `createTerminalInteraction` の通常 text/select prompt 出力先を
  `io.stderr` に変更する
- `auth interaction` の通常 prompt 出力先として `process.stderr` を
  直接参照しない
- interactive TTY select/text prompt が injected `io.stderr` に出る
  ことを検証する test の追加

### Out of Scope

- public auth behavior（login method・flow・表示内容）の変更
- stdout envelope contract の変更
- `CliIo` type の構造的変更（必要最小限の場合を除く）
- 新しい terminal abstraction の追加

## Requirements

- text/select prompt の表示先を `io.stderr` とする
- secret prompt の no-echo raw-mode 入力を維持する
- `--method` による select auto-answer を維持する
- AbortSignal / SIGINT / SIGTERM の中断動作を維持する
- non-TTY stdin での対話 prompt 失敗動作を維持する
- auth_url / device_code / progress notify の出力を維持する
- browser opener seam（`openUrl` 注入）を維持する

## Acceptance Criteria

- [ ] auth interaction の prompt output がすべて injected CLI I/O boundary 内にある
- [ ] 通常の text/select prompt で `process.stderr` を直接使わない
- [ ] interactive select/text prompt の test coverage が追加される
- [ ] secret no-echo contract が維持される
- [ ] auth URL opener seam / device flow / `--method` / cancellation が回帰しない
- [ ] `mise run check` が pass する

## Risks

- `CliIo.stderr` は `{ write(s: string): unknown }` の minimal contract で
  `stream.Writable` を要求しない。`readline.createInterface` の `output`
  に直接渡せないため、injected writer を Writable へ bridge する最小の
  変換が必要になる可能性がある。
- readline の `output` 差し替えで terminal/echo 動作が変わるリスク。
  secret prompt は readline を通さないため影響しないが、text/select の
  echo 動作は既存と同等であることを test で確認する。

## Open Questions

- なし（issue #36 で要件・方針ともに承認済み）。

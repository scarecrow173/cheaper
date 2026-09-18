---
id: "TASK-FP5YNv1aQHfB69NlhCQJbG"
type: "task"
status: "todo"
title: "Auth prompt I/O boundary routing and TTY prompt tests"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/auth-io-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/auth-io-boundary.md"
  blocks:
    - "docs/tasks/auth-io-boundary-final-verification.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Auth prompt I/O boundary routing and TTY prompt tests

## Work

PLAN-zDQIdChPXY1lrGJyzZTiQc / DESIGN-Hz0wDP9E6GjkHNLwOEOGiw の実装 task。
text/select prompt の readline 出力を injected `io.stderr` へ bridge
し、interactive TTY prompt の coverage を追加する。

### 1. `packages/cli/src/auth.ts`

- `io.stderr`（`{ write(s: string): unknown }`）を `stream.Writable` へ
  forward する最小の adapter を `createTerminalInteraction` 内に追加する。
- `ask()` の `createInterface` を
  `createInterface({ input: io.stdin, output: <adapter> })` に変更し、
  `process.stderr` 直接参照を除去する。
- secret prompt（raw-mode reader）・`notify`・`--method` auto-answer・
  signal 処理は変更しない。

### 2. `packages/cli/test/auth.test.ts`

- TTY select prompt テスト: fake runtime が
  `interaction.prompt({ type: "select", ... })` を呼ぶ。stdin へ有効な
  選択値（`"2\n"` 等）を feed し、prompt message と option labels が
  injected `io.stderr` に出ること、選択 option id が runtime へ返り
  login が成功することを assert する。
- text prompt テスト: fake runtime が `interaction.prompt({ type: "text" })`
  を呼ぶ。stdin へ入力を feed し、response が runtime へ渡ること、
  prompt が injected `io.stderr` に出ることを assert する。
- regression: 既存の secret no-echo・`--method` auto-answer・non-TTY
  select fail・auth_url opener seam・cancellation テストが変わらず
  pass すること。

## Acceptance

- select/text prompt の表示がすべて injected `io.stderr` を経由する。
- `auth.ts` の prompt 経路に `process.stderr` 直接参照が残らない。
- 新規 2 テストが追加され、`pnpm test` が pass する。
- 既存テストが回帰しない。

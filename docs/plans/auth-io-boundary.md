---
id: "PLAN-zDQIdChPXY1lrGJyzZTiQc"
type: "plan"
status: "in-progress"
test-spec-skip: "Verified by package unit test suite (auth.test.ts injected-io assertions); no dedicated test-spec doc authored."
title: "Route auth text/select prompts through the injected CliIo stderr"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/auth-io-boundary.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/auth-io-boundary.md"
    - "../specs/auth-io-boundary.md"
  implemented-by:
    - "docs/tasks/auth-io-boundary-prompt-routing-and-tests.md"
    - "docs/tasks/auth-io-boundary-final-verification.md"
  depends-on:
    - "../plans/auth-url-opener-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/auth-io-boundary.md"
    - "../adr/auth-terminal-prompt-io-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/auth-io-boundary.md"
  derived-by: []
  verifies:
    - "../specs/auth-io-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# PLAN-zDQIdChPXY1lrGJyzZTiQc: Route auth text/select prompts through the injected CliIo stderr

## Goal

SPEC-WdXXEoaAmbOafvuIpM6FFT / ADR-pINoxFu3eduWaJCNBtOT1M /
DESIGN-Hz0wDP9E6GjkHNLwOEOGiw に従い、`createTerminalInteraction` の通常
text/select prompt 出力を injected `io.stderr` へ統一し、interactive
TTY prompt の test coverage を追加する。本番動作・envelope・secret
no-echo・`--method`・cancellation は不変とする。

## Scope

単一 implementation stream。`packages/cli/src/auth.ts` と
`packages/cli/test/auth.test.ts` に限定される小規模変更のため plan 分割は
しない。

### In Scope

- `io.stderr` を forward する `Writable` adapter の追加（`auth.ts` 内 1 箇所）。
- `createInterface` の `output` を adapter へ差し替え。
- TTY select prompt・text prompt が injected `io.stderr` に出る test の追加。

### Out of Scope

- `CliIo` type の変更（contract を維持するため adapter 側で吸収する）。
- public auth behavior・表示文言・envelope の変更。
- secret prompt 実装・`notify`・`executeAuth` の変更。

## File Map

| File | Change |
| --- | --- |
| `packages/cli/src/auth.ts` | `io.stderr` forward の `Writable` adapter を追加し、`createInterface` の `output` へ渡す。`process.stderr` 直接参照を除去。 |
| `packages/cli/test/auth.test.ts` | TTY select prompt・text prompt の injected-stderr 検証テストを追加。 |

## Task Breakdown

1. **TASK-FP5YNv1aQHfB69NlhCQJbG**: adapter 実装と TTY prompt test 追加。
   `auth.ts` の `createInterface` output を injected `io.stderr` へ
   bridge し、select/text prompt の injected-stderr テストを追加する。
2. **TASK-AZSy7cWtCdKaL5D2ro8TWy**: final verification。`mise run check`
   を通し、回帰（secret no-echo・`--method`・non-TTY・opener seam・
   cancellation）がないことを確認し、SPEC status を `implemented` へ
   同期し impl record を作成する。

## Dependency Graph

```text
TASK-FP5YNv1aQHfB69NlhCQJbG (adapter + tests)
   └─ TASK-AZSy7cWtCdKaL5D2ro8TWy (verification)  ※ TASK-FP5YNv1aQHfB69NlhCQJbG 完了後のみ着手可
```

## Risk Register

| Risk | Mitigation |
| --- | --- |
| `io.stderr` が `Writable` でなく `createInterface` が受け付けない | write forward の `Writable` adapter を噛ませる（ADR Option 1）。 |
| adapter 経由で readline の echo/終端動作が変わる | TTY select/text テストで prompt 表示と応答を end-to-end に検証する。 |
| 既存テスト（non-TTY・`--method`・secret）が壊れる | 変更点は `output` 1 行相当に限定し、全既存テストを回帰確認する。 |

## Rollback Strategy

変更は `auth.ts` 内の adapter 追加と `output` 差し替えのみ。問題時は
本 plan のコミットを revert すれば `process.stderr` 直接参照の従来
実装へ戻る。

## Verification Matrix

| Acceptance Criterion (SPEC-WdXXEoaAmbOafvuIpM6FFT) | Verification |
| --- | --- |
| prompt output が injected boundary 内にある | TTY select/text テストで prompt・option labels・echo が `io.stderr` capture に現れる。 |
| `process.stderr` を直接使わない | `auth.ts` に `process.stderr` 参照が残らないこと（`openUrlBestEffort` 等 prompt 無関係経路を除く）。 |
| interactive select/text coverage 追加 | 新規 2 テストの追加と pass。 |
| secret no-echo 維持 | 既存 secret テストが変わらず pass。 |
| opener seam / device flow / `--method` / cancellation 回帰なし | 既存テスト群の pass。 |
| `mise run check` pass | canonical check の exit 0。 |

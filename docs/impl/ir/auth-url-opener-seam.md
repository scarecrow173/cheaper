---
id: "IMPL-034r05iHy2gdnpt2vgHZ55"
type: "impl"
status: "completed"
title: "Auth URL opener seam (PLAN-034r05iHE1Fbkp4xxlyMdV)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/cli/src/cli.ts"
      - type: "file"
        path: "packages/cli/src/auth.ts"
      - type: "file"
        path: "packages/cli/test/auth.test.ts"
      - type: "file"
        path: ".apm/hooks/cli/bin/cheaper.mjs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: ".apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/tasks/auth-url-opener-seam-and-test-isolation.md"
    - "docs/tasks/auth-url-opener-final-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/auth-tests-do-not-launch-os-browser.md"
    - "docs/adr/inject-url-opener-seam-for-auth-interaction.md"
    - "docs/designs/auth-url-opener-seam.md"
    - "docs/plans/auth-url-opener-seam.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Auth URL opener seam (PLAN-034r05iHE1Fbkp4xxlyMdV)

## Summary

Issue #8 対応。auth の browser 起動を `CliHooks.openUrl` 経由の注入 seam に
変更し、`pnpm test` が OS の URL opener（`rundll32` / `open` /
`xdg-open`）を spawn しなくなった。本番既定動作は不変。

## Context

TASK-034r05iHQeQpDqovKmKaZC〜0094（PLAN-034r05iHE1Fbkp4xxlyMdV / SPEC-034r05iHK8bA7gmLlq6ikw / ADR-034r05iHrZUE6WcElqgQIr / DESIGN-034r05iHy2gdnoOvi1vrou）に従う。

## Implementation

### Changes

- `cli.ts`: `CliHooks.openUrl?: (url: string) => void` を追加。既存
  `createModelRuntime` / `runWorker` / `signal` と同じ test seam の注入面
- `auth.ts`: `createTerminalInteraction` の `opts` に `openUrl` を追加し、
  `notify` の `auth_url` 分岐が `(opts.openUrl ?? openUrlBestEffort)` を
  呼ぶ。既定値は interaction 側の一箇所のみで定義。`executeAuth` は
  `hooks.openUrl` を中継し、未指定時は `undefined` を渡す
- `auth.test.ts`: `--method device` テストの fake `login` から無関係な
  `auth_url` emit を除去し、`openUrl` spy 未呼出を assert。browser 起動
  要求は spy が要求 URL で 1 回呼ばれる専用テストへ分離
- `.apm/hooks/cli/bin/cheaper.mjs`: bundled CLI を再生成（freshness check
  対象のため source と同期）

### Decisions During Implementation

- **既定値の所在**: `executeAuth` 側に既定を持たせず interaction 側の
  `opts.openUrl ?? openUrlBestEffort` のみとし、既定の二重定義を避けた
- **`--method device` の assert 方法**: fake から `auth_url` を除いた上で
  `openUrl` spy の未呼出を確認し、「device flow が browser 起動要求を
  発生させない」ことを構造と挙動の両面で示した
- **実 OS opener のテスト**: `openUrlBestEffort` 自体のユニットテストは
  追加しない（OS 依存・外部プロセス）。seam 未到達の構造で保証する

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- pnpm lint`: biome check 131 files、問題なし
- `mise exec -- pnpm typecheck`: `tsc -b` 成功
- `mise exec -- pnpm test`（`packages/cli`）: 13 files / 127 tests green
  （auth.test.ts 15 件、新規 spy テスト含む）
- テスト実行で OS opener が spawn されないこと: `--method device` が
  起動要求を発生させず（spy 未呼出）、`auth_url` を扱うテストは注入
  spy 経由で既定 opener に到達しない

## Risks

- なし。

## Follow-ups

- なし

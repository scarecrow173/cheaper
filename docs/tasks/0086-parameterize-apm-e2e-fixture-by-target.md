---
id: "TASK-0086"
type: "task"
status: "done"
title: "Parameterize apm e2e fixture by target"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/0013-cross-agent-distribution-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0013-cross-agent-distribution-verification.md"
  blocks:
    - "docs/tasks/0087-per-target-apm-install-verification-e2e.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0018-cross-agent-distribution-verification.md"
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

# Parameterize apm e2e fixture by target

## Work

`packages/cli/test/e2e/apm-fixture.ts` を target パラメータ化する
（DESIGN-0018 §1 option (a)）。全パラメータに `target = "windsurf"`
default を付けて既存 call site（`apm-install.test.ts`）を無変更で
維持する。

- `makeConsumerLayout(root, target = "windsurf")`: consumer `apm.yml`
  の `targets:` を `- <target>` で生成（pkg 側 `apm.yml` は不変）
- `apmInstall(consumer, home, target = "windsurf")`: `apm install
  --target <target>` を spawn
- `bundleRoot(consumer, target = "windsurf")`: 下記 target map で
  deploy dir を返す。`agent-skills` は hook deploy 対象外のため
  `null` を返す（return type を `string | null` に変更）
- `bundledCli(consumer, target = "windsurf")`: `bundleRoot` に追従
  （`agent-skills` では呼ばれない前提、または `bundleRoot` null 時は
  throw でよい）
- `generatedConfigPaths(consumer, target): string[]`（新規）: target
  → 生成 config path 一覧を返す。`agent-skills` は空配列。copilot
  は file 名が dep 名由来のため `.github/hooks` ディレクトリ path
  を返し、`*-hooks.json` の一意解決は呼び出し側（TASK-0087）が
  `readdirSync` で行う旨を JSDoc コメントで明記する

target map（consumer fixture の dep dir 名 = `pkg`）:

| target | bundleRoot | generatedConfigPaths |
| --- | --- | --- |
| `windsurf` | `.windsurf/hooks/pkg` | `.windsurf/hooks.json` |
| `claude` | `.claude/hooks/pkg` | `.claude/apm-hooks.json`, `.claude/settings.json` |
| `codex` | `.codex/hooks/pkg` | `.codex/hooks.json` |
| `cursor` | `.cursor/hooks/pkg` | `.cursor/hooks.json` |
| `copilot` | `.github/hooks/scripts/pkg` | `.github/hooks/`（dir を返す。`*-hooks.json` は readdirSync 解決） |
| `agent-skills` | `null`（hook deploy なし） | `[]` |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0013-cross-agent-distribution-verification.md`
- `docs/designs/0018-cross-agent-distribution-verification.md`

## Dependencies

- PLAN-0013

## Affected Files

- `packages/cli/test/e2e/apm-fixture.ts`

変更しない file（明示）: `apm-install.test.ts`、
`pre-read-hook.test.ts`、`delegation-loop.test.ts`（既存 call site は
default 引数で後方互換のため無変更）。

## Done When

- [ ] 4 helper が `target` パラメータ（default `"windsurf"`）を持ち、
      `generatedConfigPaths` が追加されている
- [ ] 既存 `apm-install.test.ts` が無変更のまま compile・実行される
      （apm 不在環境では skip）

## Verification

- [ ] `mise exec -- pnpm -F cheaper build` が exit 0（型エラーなし）
- [ ] `mise exec -- pnpm -F cheaper test` が exit 0（既存 windsurf e2e
      含む全 suite が pass または apm 不在で skip）

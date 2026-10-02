---
id: "IMPL-034z1STqpGaGE7SWSnCLo3"
type: "impl"
status: "completed"
title: "Unify packageVersion on builtinDir"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
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
    - "docs/tasks/unify-packageversion-resolution-on-builtindir.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Unify packageVersion on builtinDir

## Summary

`packageVersion()` を `builtinDir` 引数基準の解決に変更し、
`version.json` → `package.json` の順で builtinDir 直下を探索する。
dev / APM bundle / SEA VFS の 3 形態で同一規則となった。

## Context

docs/tasks/unify-packageversion-resolution-on-builtindir.md
（PLAN-034z1M6zCAdKb12eiCWZRm task 2）。TSPEC-034z1P09dg86lO9uB31cUi
の builtinDir 統一規則を実装。

## Implementation

### Changes

- `packages/cli/src/version.ts`: `packageVersion(builtinDir)` に変更。
  `import.meta.url` 相対解決を廃止し `path.join(builtinDir, name)` で
  `version.json` / `package.json` を探索
- `packages/cli/src/cli.ts`: `--version` 分岐で
  `packageVersion(hooks.builtinDir ?? defaultBuiltinDir())` を呼ぶ
- `packages/cli/test/version.test.ts`（新規）: builtinDir 解決 4 件の
  unit test（TDD: red → green 確認済み）

### Decisions During Implementation

- SEA VFS では `import.meta.url` の `../version.json` が VFS root の外へ
  逃げるため、bundle 位置ではなく builtinDir 起点に揃えた
- fallback 順は `version.json` 優先（APM/SEA は version.json のみ、
  dev は package.json のみ存在する配置と一致）

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `pnpm exec vitest run test/version.test.ts` — 4/4 pass
- `node packages/apm/.apm/hooks/cli/bin/cheaper.mjs --version` → `1.0.0`
- `node packages/cli/bin/cheaper.js --version` → `1.0.0`

## Risks

- `packageVersion` の引数なし呼び出しは型エラーになるため破壊的変更だが
  call site は cli.ts の 1 件のみで済む

## Follow-ups

- SEA entrypoint（sea-main.ts）が `builtinDir: import.meta.dirname` を
  渡すことで本変更が VFS 上で効く（TASK-034z1POhgJ49oVv2arDj6X）

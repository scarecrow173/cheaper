---
id: "TASK-034r05iHK8bA7gUBy3ygQD"
type: "task"
status: "done"
title: "Implement workspace path resolution with symlink policy"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/implement-core-error-model-and-exit-code-table.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement workspace path resolution with symlink policy

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 16。`resolveWorkspacePath(root, input, symlinkPolicy)` を DESIGN-034r05iHrZUE6WsgJOy6Ny のアルゴリズム 1–5 どおり実装（NUL 拒否・path.resolve・relative で ESCAPE 判定・win32 大小文字非依存・component 毎 lstat/symlink 判定・不存在最深祖先 realpath が root 配下）。テストは tmp dir に実 symlink を作成。Windows で権限が無い場合は `it.skipIf` で理由をログし、CI/Unix では必ず実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-core-error-model-and-exit-code-table.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/workspace/paths.ts`
- `packages/core/test/workspace/paths.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] ../・絶対・~・symlink in/out・create 許容の各テストが green（Windows は権限次第で skipIf）

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/workspace/paths.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- operations 注入での利用は runtime-pi（TASK-034r05iHK8bA7h4BqSHmqd）。

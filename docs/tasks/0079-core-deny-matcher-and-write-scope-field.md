---
id: "TASK-0079"
type: "task"
status: "done"
title: "Core deny-path matcher, writeScope frontmatter, policy denyPaths"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/0010-write-scope-and-privacy-deny.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/0010-write-scope-and-privacy-deny.md"
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

# Core deny-path matcher, writeScope frontmatter, policy denyPaths

## Work

- `packages/core/src/workspace/deny-paths.ts`: glob-lite matcher
  （`*`/`?`/`**`、unanchored-by-default、`dir/**` は dir 自体にも match）と
  `DEFAULT_DENY_PATHS`、`createPathDenylist` を実装し `index.ts` から export
- `worker/validate.ts`: `cheaper.writeScope?: string` を受理し
  `WorkerDefinition` に載せる
- `policy/schema.ts`: `filesystem.denyPaths?: string[]` を受理、
  `RuntimePolicy.filesystem.denyPaths` に正規化
- `packages/cli/policy.yaml` に `denyPaths: []` を追加
- unit test: matcher 表裏、frontmatter 受理/拒否、policy merge

## Acceptance Criteria

- [x] matcher が `.env`/`*.pem`/`.ssh/**` 等を任意深さで match
- [x] `writeScope` 未宣言 worker は従来どおり受理される
- [x] policy `denyPaths` が effective policy に反映される

## Done

core matcher + policy/worker schema 拡張と unit tests。

---
id: "TASK-034rBt3LLoIUcFHjk4gRlI"
type: "task"
status: "done"
title: "Shell-read blocking contract tests"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
  implemented-by:
    - "docs/impl/ir/shell-read-candidate-extraction-implementation.md"
  depends-on:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
    - "docs/tasks/shell-read-candidate-extractor-in-routing-policy.md"
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
  references:
    - "docs/test-specs/shell-read-blocking-contract-tests.md"
  defers: []
  deferred-by: []
---

# Shell-read blocking contract tests

## Work

`packages/cli/test/e2e/cross-agent-routing-adapters.test.ts` を拡張し、
TSPEC-034rBrs4K5W7pe0ixjegZk の Rules/Examples を stdin 注入 contract
test として固定する。issue #42 の test リスト（各 command の deny、
small file allow、複数 file、option、quoted path、pipe、ambiguous
syntax fail-open、file not found、workspace 外、malformed payload、
CLI failure、malformed count-tokens output、FILE_TOO_LARGE、threshold
override、file-read との verdict 共有）を網羅する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/shell-read-candidate-extraction-implementation.md`
- `docs/test-specs/shell-read-blocking-contract-tests.md`
- `docs/specs/shell-read-large-file-blocking.md`

## Dependencies

- `docs/tasks/shell-read-candidate-extractor-in-routing-policy.md`
  （policy 実装が先）

## Affected Files

- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`

## Done When

- [x] issue #42 の test リストに対応する case が全て存在する
- [x] file-read と shell-read が同一 file に同一 verdict を返す
      regression test がある

## Verification

- [x] `mise exec -- pnpm --filter cheaper exec vitest run
      test/e2e/cross-agent-routing-adapters.test.ts` が全件 pass
      （31 tests、2026-09-20 実測）

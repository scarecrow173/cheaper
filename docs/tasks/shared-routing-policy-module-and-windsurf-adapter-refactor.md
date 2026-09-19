---
id: "TASK-034rAcEwRUaAkZ0KOCmc9J"
type: "task"
status: "todo"
title: "Shared routing policy module and Windsurf adapter refactor"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cross-agent-adapter-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Shared routing policy module and Windsurf adapter refactor

## Work

`packages/apm/.apm/hooks/routing-policy.js` を新規作成し、canonical
routing request（`{kind:"file-read"|"shell", path|command, cwd,
sourceTool?}`）と shared policy `evaluate()` を実装する。既存
`pre-read-code.js` の判定ロジック（workspace 内判定、stat、fast path、
`count-tokens` spawn、`FILE_TOO_LARGE` fail-closed、token threshold
deny、deny guidance 文言）をこの module へ移し、`pre-read-code.js`
は Windsurf payload の parse と verdict → exit 2 / stderr 変換だけを
持つ adapter にする。`shell` request は常に allow を返す seam とする
（#42 の接続点）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cross-agent-native-routing-adapters-implementation.md`
- `docs/designs/cross-agent-native-routing-adapter-architecture.md`
- `docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md`
- `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`

## Dependencies

- なし（chain の先頭 task）

## Affected Files

- `packages/apm/.apm/hooks/routing-policy.js`（新規）
- `packages/apm/.apm/hooks/pre-read-code.js`（policy 呼出しへ移譲）

## Done When

- [ ] `routing-policy.js` が canonical request を受けて
      allow / deny verdict を返す
- [ ] `pre-read-code.js` が判定ロジックを持たず policy へ委譲する
- [ ] deny guidance の文言（`tokens` / `bytes` / `measurement cap` /
      `workspace-relative path` / `bulk-read`）が既存と同一

## Verification

- [ ] `pnpm --filter @cheaper/cli test -- pre-read-hook` が全件 pass
      （Windsurf 既存 test が regression しない）

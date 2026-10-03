---
id: "TASK-034zBJ64O6cfOluLvLxYIO"
type: "task"
status: "done"
title: "Append sibling tool-call parallelism section to execution-engine design doc"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/designs/pi-execution-engine.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-pi-sibling-tool-call-parallelism.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-pi-sibling-tool-call-parallelism.md"
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
---

# Append sibling tool-call parallelism section to execution-engine design doc

## Work

plan `docs/plans/implement-pi-sibling-tool-call-parallelism.md`（Phase C、
T14）。`docs/designs/pi-execution-engine.md` 本文末尾に
`## Sibling tool-call parallelism` セクションを追記し、front matter の
`updated` を当日日付に更新する。approved doc への後追記である旨と、
DESIGN-034zAKcWuB8SuFkOePsubg / SPEC-034zA513QKarQrr5SQtgp7 /
issue #55 への参照を attribution として明記する。既存構造
（Context→…→Verification Notes）は維持する。

design の文書化 subsection に従い、追記セクションには以下を含める:

- pinned `pi-agent-core@0.85.1` の `toolExecution: "parallel"` 既定を
  cheaper はそのまま利用する（`toolExecution`/`executionMode` は一切
  設定しない）
- parallel path semantics: `Promise.all` overlap 実行、`tool_result`
  は call 順 emit、`stopReason: "length"` で sibling 全員未実行
  error、同一ファイル write/edit は `withFileMutationQueue` で call
  順直列化
- 節約源が **Worker turn 数と増大 input context の反復送信の削減**で
  あり、並列化は token 単価を変えない旨
- `output.maxTokens` cap との相互作用（cap 超過 turn は全 sibling
  未実行になるため batch 粒度を cap に収める）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-pi-sibling-tool-call-parallelism.md`
- `docs/designs/pi-sibling-tool-call-parallelism-inside-workers.md`
  （DESIGN-034zAKcWuB8SuFkOePsubg、文書化内容が定義済み）
- `docs/specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md`
  （SPEC-034zA513QKarQrr5SQtgp7）
- issue #55: https://github.com/scarecrow173/cheaper/issues/55

## Dependencies

- なし（doc 追記のみ。Phase A / Phase B の task と並行可能）

## Affected Files

- `docs/designs/pi-execution-engine.md` — 末尾への 1 セクション追記 +
  `updated` frontmatter 更新

## Done When

- [x] `docs/designs/pi-execution-engine.md` 末尾に
      `## Sibling tool-call parallelism` セクションが存在し、上記の
      必須内容（`toolExecution: "parallel"` 既定のそのまま利用 /
      parallel path semantics / 節約源 / `output.maxTokens` cap
      相互作用）が全て記述されている
- [x] approved doc への後追記である旨と、
      DESIGN-034zAKcWuB8SuFkOePsubg / SPEC-034zA513QKarQrr5SQtgp7 /
      issue #55 への attribution が明記されている
- [x] front matter の `updated` が追記当日日付に更新されている

## Verification

- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] Manual: `docs/designs/pi-execution-engine.md` の
      `## Sibling tool-call parallelism` セクションが plan T14 の
      content list（既定利用・parallel path semantics・節約源・cap
      相互作用・attribution・`updated` 更新）を全て含むことを確認

---
id: "DESIGN-034r05iHrZUE6Wy5X0Hc88"
type: "design"
status: "superseded"
title: "cheap-writer contract"
created: "2026-09-15"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by:
    - "writer-worker-contracts.md"
  related:
    - "workspace-filesystem-boundary-sandboxed-tools.md"
    - "pi-execution-engine.md"
    - "bulk-reader-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheap-writer-built-in-worker.md"
    - "../adr/wrapper-measured-deterministic-metadata.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  derived-by:
    - "../plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheap-writer contract

## Context

SPEC-034r05iHK8bA7g7YRKGyQr の built-in Worker「cheap-writer」の Worker file と input/output schema を確定する。workspace を直接変更し、changed files/diff 統計は LLM 報告ではなく wrapper が実測する。

## Scope

### In Scope

- `packages/cli/workers/cheap-writer.md`（frontmatter + Markdown 指示）
- `schemas/cheap-writer.input.schema.json` / `cheap-writer.output.schema.json`
- targets 外書き込みの扱いと abort 時の `PARTIAL_MUTATION` 規則

### Out of Scope

- rollback 機構（abort 時 rollback しない、§3.2・SPEC-034r05iHK8bA7g7YRKGyQr）
- Worker 固有ロジックの Runtime への移譲（置かない、§0）

## Components and Boundaries

- Worker file: `tools: [read, grep, find, ls, write, edit]`、`tier: low`、input/output schema は `./schemas/` のファイル参照

### input schema（additionalProperties false）

| field | 型 | 制約 |
| --- | --- | --- |
| `task` | string | required、1–6000 |
| `targets` | array | required、1–20 × `{ path, intent: "create"|"modify"|"create-or-modify", notes?: ≤500 }` |
| `references` | string[] | 任意、≤30。書く前に読むファイル |
| `constraints` | string[] | 任意、≤20 × ≤300 |

### output schema（全 required、additionalProperties false）

- `summary`: string、1–1500
- `notes`: string[] ≤20 × ≤500
- `unresolved`: string[] ≤20 × ≤500

**changed files は LLM に報告させない**。`meta.changes` が唯一の情報源（§2.6、ADR-034r05iHlWosDpMXiQ38yf）。

## Data and Control Flow

- Runtime は targets 外への書き込みを機械的には禁止しない（Worker 固有ロジックを Runtime に置かない §0）。指示で targets 外を書かないこと・必要なら `unresolved` に挙げることを命じ、実際の変更は `meta.changes` で親 Agent が検証できる
- partial writes / abort: DESIGN-034r05iHrZUE6WsgJOy6Ny の journal と DESIGN-034r05iHrZUE6WvaQsRRsM の abort 分類で `PARTIAL_MUTATION`（exit 9）。rollback なし

## Non-Functional Constraints

- 変更の事実確定は journal 実測のみ。LLM 出力の `summary`/`notes` は説明であって変更一覧の信頼源ではない

## Risks and Trade-offs

- targets 外書き込みを許容する設計は、親 Agent が `meta.changes` を見て検証することを前提とする。検証しない運用では意図外変更が残る
- rollback 非対応は abort/timeout 時の中途半端な状態を許容する。`PARTIAL_MUTATION` と `meta.changes` で親が判断材料を得る

## Deferred Design Concerns

- targets 外書き込みの機械的拒否（write allowlist） | Worker 固有ロジックを Runtime に入れない方針と衝突 | 親 Agent 側の検証が不十分と判明した時点 | targets 外変更を見逃すリスク
- rollback / dry-run | 引継ぎで V1 非対応 | 要求が出た時点 | 破壊的な失敗で復旧手段が journal + 手動のみ

## Verification Notes

- [ ] input/output schema が draft 2020-12 で Ajv `validateSchema` を通る
- [ ] `meta.changes` が journal のみから生成され、LLM 報告と独立
- [ ] abort/timeout 時に journal 非空なら `PARTIAL_MUTATION` exit 9 と `meta.changes` が出力される
- [ ] workspace 外 path への write/edit は tool error として拒否される

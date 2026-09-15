---
id: "DESIGN-0006"
type: "design"
status: "draft"
title: "bulk-reader contract"
created: "2026-09-15"
updated: "2026-09-15"
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
  superseded-by: []
  related:
    - "0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "0007-cheap-writer-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0002-bulk-reader-built-in-worker.md"
    - "../adr/0005-model-tier-model-resolver.md"
    - "../adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# bulk-reader contract

## Context

SPEC-0002 の built-in Worker「bulk-reader」の Worker file と input/output schema を確定する。read-only で巨大コンテキストを短い structured findings に圧縮する。

## Scope

### In Scope

- `packages/cli/workers/bulk-reader.md`（frontmatter + Markdown 指示）
- `schemas/bulk-reader.input.schema.json` / `bulk-reader.output.schema.json`（draft 2020-12）
- excerpt policy と禁止事項

### Out of Scope

- Worker 実行機構（DESIGN-0002/0004）
- 結果の親 Agent への提示フォーマット（親側の責務）

## Components and Boundaries

- Worker file: `tools: [read, grep, find, ls]`、`tier: low`、input schema `./schemas/bulk-reader.input.schema.json`、output schema `./schemas/bulk-reader.output.schema.json`
- path 共通 pattern（両 schema で `$defs` に定義）: workspace 相対 POSIX、`^(?!/)(?![A-Za-z]:)(?!\.\.(/|$))(?!.*/\.\.(/|$)).+$`

### input schema（additionalProperties false）

| field | 型 | 制約 |
| --- | --- | --- |
| `task` | string | required、1–4000 文字 |
| `paths` | string[] | 任意、≤50。相対 path または glob。ディレクトリは再帰 |
| `hints` | string[] | 任意、≤20。symbol 名・キーワード |
| `maxFindings` | int | 任意、1–30 |

### output schema（全 required、additionalProperties false）

- `summary`: string、1–1500
- `findings`: ≤30 × `{ title ≤120, detail ≤1200, confidence: "high"|"medium"|"low", locations: ≤10 × { path, lines?: {start ≥1, end ≥ start}, excerpt?: string ≤600 } }`
- `relevantFiles`: ≤30 × `{ path, reason ≤300 }`
- `suggestedReads`: ≤10 × `{ path, lines?, reason ≤300 }`
- `openQuestions`: ≤10 × string ≤300

## Data and Control Flow

- excerpt policy: verbatim、≤600 文字・≤20 行、根拠提示に必要な箇所のみ、ファイル全文や長大ダンプ禁止
- Markdown 指示の骨子（英語で記述）: role 定義 → 手順（paths/hints から find/grep で当たりを付け、read は必要範囲のみ offset/limit）→ 出力規則 → 禁止事項（推測を事実として書かない、ファイルを変更しない）

## Non-Functional Constraints

- read-only: filesystem 変更 0 件を wrapper（journal）が実測で保証（SPEC-0002 受け入れ条件）
- 出力は宣言 schema に適合する JSON のみ（DESIGN-0004 の Output contract）

## Risks and Trade-offs

- read-only 制約は tool vocabulary（write/edit 不所持）で構造的に担保し、指示文にも明記する二重化
- `paths` の glob は sandboxed walker の結果に依存するため、親 Agent の意図した対象外は結果品質で劣化する。`relevantFiles`/`suggestedReads` で後続 read を提案する設計で緩和

## Deferred Design Concerns

- findings の信頼度集計・evidence 必須化 | V1 は `confidence` field で自己申告 | 精度評価データが揃った時点 | 根拠なき finding を見分けられない

## Verification Notes

- [ ] input/output schema が draft 2020-12 で Ajv `validateSchema` を通る
- [ ] Worker が read/grep/find/ls のみを持ち write/edit を持たない
- [ ] read-only 実行で `meta.changes.files` が空
- [ ] workspace 外 path 入力は tool error として拒否される

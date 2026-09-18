---
id: "DESIGN-034r05iHy2gdnoDFAeW1DN"
type: "design"
status: "approved"
title: "code-writer and doc-writer worker contracts"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/code-writer-and-doc-writer-built-in-workers.md"
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
  supersedes:
    - "cheap-writer-contract.md"
  superseded-by: []
  related:
    - "workspace-filesystem-boundary-sandboxed-tools.md"
    - "pi-execution-engine.md"
    - "cli-contract-envelope-exit-codes.md"
    - "bulk-reader-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/code-writer-and-doc-writer-built-in-workers.md"
    - "../adr/wrapper-measured-deterministic-metadata.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# code-writer and doc-writer worker contracts

## Context

SPEC-034r05iHK8bA7gaTnVhXWI の built-in Writer 2 種の Worker file・input/output schema・CLI alias を
確定する。両 Worker は同一の generic Runtime 上で動き、違いは Worker
Definition・schema・instructions のみ。Runtime に Worker 固有分岐は追加しない。
mutation の source of truth は `meta.changes`（ADR-034r05iHlWosDpMXiQ38yf）で、changed files を
LLM に自己申告させない点は両 Worker で共通。

## Scope

### In Scope

- `packages/cli/workers/code-writer.md`・`doc-writer.md`（frontmatter + 指示）
- `schemas/code-writer.{input,output}.schema.json`・`doc-writer.{input,output}.schema.json`
- CLI alias `code-write` / `doc-write`（`ALIAS_WORKERS`・HELP・test 更新）
- `cheap-writer.md`・`cheap-writer.*.schema.json`・`cheap-write` alias の削除

### Out of Scope

- write-scope の機械的強制（別 design。本 contract では指示 + `meta.changes` 検証）
- schema の共有化・継承機構（2 schema が似ていても contract は各 Worker 独立に宣言。
  過度な共通化はしない）

## Components and Boundaries

両 Worker の frontmatter 共通部:

- `tools: [read, grep, find, ls, write, edit]`
- `cheaper.model.tier: low`
- input/output schema は `./schemas/` のファイル参照

### code-writer input schema（additionalProperties false）

| field | 型 | 制約 |
| --- | --- | --- |
| `task` | string | required、1–6000 |
| `targets` | array | required、1–20 × `{ path, intent: "create"\|"modify"\|"create-or-modify", notes?: ≤500 }` |
| `references` | string[] | 任意、≤30。書く前に読むファイル |
| `constraints` | string[] | 任意、≤20 × ≤300 |

旧 `cheap-writer` と同一 contract。名称のみ変更で schema 内容は変えない。

### doc-writer input schema（additionalProperties false）

| field | 型 | 制約 |
| --- | --- | --- |
| `task` | string | required、1–6000。文書の目的と期待する内容 |
| `documentType` | string enum | required。`adr`/`spec`/`design`/`plan`/`task`/`readme`/`guide`/`reference`/`release-notes`/`other` |
| `targets` | array | required、1–20 × `{ path, intent, notes? }`（code-writer と同形） |
| `decisions` | string[] | 任意、≤30 × ≤1000。親が確定済みの判断。writer は変更・創作しない |
| `facts` | string[] | 任意、≤30 × ≤1000。文書に使う確定済みの事実 |
| `references` | string[] | 任意、≤30。根拠資料として読むファイル |
| `constraints` | string[] | 任意、≤20 × ≤300。形式・構造・スタイルの制約 |

`decisions`/`facts` は「親が決め、writer が構造化・展開・整形・記述する」分担を
contract で表す。判断材料が欠けているとき writer は推測で埋めず `unresolved` へ返す。

### output schema（両 Worker 共通構造、全 required・additionalProperties false）

- `summary`: string、1–1500
- `notes`: string[] ≤20 × ≤500
- `unresolved`: string[] ≤20 × ≤500。doc-writer では未確定の判断・不足情報を
  ここへ返すことが主要用途

changed files/diff は output に含めない。`meta.changes` が唯一の情報源。

### CLI alias

`ALIAS_WORKERS` を `bulk-read`/`code-write`/`doc-write` へ更新し、
`cheap-write` は削除（UNKNOWN command として `INVALID_REQUEST`）。

## Data and Control Flow

- 入力: stdin/`--input` の JSON → `validateWorkerInput` → generic `runWorker`
- targets 外書き込みは機械的には禁止しない（現行どおり指示 + `meta.changes` で
  親が検証。機械的制約は別 design）
- partial writes/abort: journal + abort 分類で `PARTIAL_MUTATION`（exit 9）。
  rollback なし

## Non-Functional Constraints

- 変更の事実確定は journal 実測のみ
- doc-writer は `decisions`/`facts`/`references` に無い重要判断を追加しない
  （指示で禁止し、品質は benchmark phase で実測）

## Risks and Trade-offs

- targets 外書き込みを許容する設計は、親 Agent が `meta.changes` を見て検証する
  ことが前提（DESIGN-034r05iHrZUE6Wy5X0Hc88 から継続。write-scope 機構は後続 phase）
- doc-writer の判断創作防止は現時点では contract + 指示レベル。 hallucination
  は doc-writer benchmark の検証項目とする

## Verification Notes

- [ ] 両 schema が draft 2020-12 で Ajv `validateSchema` を通る
- [ ] `code-write`/`doc-write` alias が解決し、`cheap-write` が拒否される
- [ ] `meta.changes` が journal のみから生成される
- [ ] worker test で built-in worker 解決が code-writer/doc-writer を返す

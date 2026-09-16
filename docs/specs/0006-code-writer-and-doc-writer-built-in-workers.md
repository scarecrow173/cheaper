---
id: "SPEC-0006"
type: "spec"
status: "implemented"
title: "code-writer and doc-writer built-in Workers"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "0001-cheaper-v1-worker-runtime-cli.md"
  blocks: []
  supersedes:
    - "0003-cheap-writer-built-in-worker.md"
  superseded-by: []
  related:
    - "0002-bulk-reader-built-in-worker.md"
    - "../adr/0005-model-tier-model-resolver.md"
    - "../adr/0006-wrapper-measured-deterministic-metadata.md"
    - "../adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  refines:
    - "0001-cheaper-v1-worker-runtime-cli.md"
  refined-by: []
  derives-from:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# code-writer and doc-writer built-in Workers

## What Should Be Built

built-in Writer Worker を `cheap-writer` から再編成し、責務別に2つへ分ける。

- `code-writer`（`cheaper code-write`）: 機械的なコード生成・変更を workspace へ直接適用する
- `doc-writer`（`cheaper doc-write`）: 親 Agent が既に決定・整理した内容を構造化文書として workspace へ書く

`cheap-writer` / `cheap-write` の名称は廃止し、deprecated alias・互換 Worker・
migration shim は作らない（実運用前のため直接移行）。

## Why Now

`cheap-writer` はコード生成責務のみを想定していたが、最終目的では文書生成も
frontier model から逃がす対象である。コードと文書では委譲してよい判断の性質が
異なり（機械的変換 vs 既決内容の構造化）、1 Worker のままでは入力 contract が
曖昧になる。実運用前の今のうちに最終形へ分離する。

## Users and Value

- 親 Coding Agent: tests/DTO/config/boilerplate 等の機械的コードと、ADR/spec/plan
  等の構造化文書の生成を cheap model へ委譲し、token cost を削減できる
- 開発者: 生成物は workspace の実ファイルとして得られ、変更事実は `meta.changes`
  で検証できる

## Scope

### In Scope

- `code-writer`: tests・DTOs・types・configs・boilerplate・repetitive adapters・
  simple mechanical refactoring・pattern-based code generation
- `doc-writer`: ADR・specification・design document・implementation plan・task
  document・README section・guide・reference documentation・change summary・
  release notes・structured project documentation
- 両 Worker とも workspace へ直接 write/edit する
- 両 Worker とも変更事実の source of truth は Runtime 実測の `meta.changes`。
  changed files を LLM に自己申告させない
- CLI alias `code-write` / `doc-write`、built-in worker・schema・test・doc の
  新名称への直接移行

### Out of Scope

- Worker 固有ロジックの Runtime への実装（違いは Worker Definition・
  input/output schema・instructions で表現する。新しい Writer 追加に Runtime
  変更が要らない構造を維持）
- 親 frontier model 側へ残す判断: architecture decisions・security-sensitive・
  concurrency-sensitive・subtle debugging・ambiguous business rules・
  unclear requirements・novel algorithm design
- targets 外書き込みの機械的強制（invocation-scoped writable paths 等の
  write-scope 機構は別 spec/design で扱う）
- Hook による hidden Writer 自動実行（行わない）
- APM Worker skill の内容（別 spec）

## Requirements

### code-writer

- tools: `read` / `grep` / `find` / `ls` / `write` / `edit`、model tier `low`
- 入力で `targets`（書込み対象 path と intent）を必須とし、targets 外への
  書き込みを指示で禁止する。必要になった場合は `unresolved` で親へ返す

### doc-writer

- tools: `read` / `grep` / `find` / `ls` / `write` / `edit`、model tier `low`
- 重要な設計判断を doc-writer 自身へ委譲しない。decision・facts・constraints・
  document purpose・target paths・references は入力または既存資料から与える
- 不足している重大な意思決定を勝手に創作してはならない。不明点は output の
  `unresolved`（相当）で親へ返す
- ADR 等では decision / context / alternatives / consequences の主要判断材料が
  入力から与えられることを contract で表現する

### 共通

- 入力は JSON task input、宣言した input schema に従う
- 出力は宣言した output schema に適合する（schema validation は Runtime Invariant）
- `meta.changes` が mutation の唯一の事実源であり、output に changed files の
  一覧を含めない
- 旧名称（`cheap-writer` / `cheap-write`）は code・test・有効な spec/design・
  CLI・README から除去する。歴史記録（完了 task・IR・EXP）は書き換えない

## Acceptance Criteria

- [ ] `cheaper run code-writer` / `cheaper code-write` で有効入力が実行できる
- [ ] `cheaper run doc-writer` / `cheaper doc-write` で有効入力が実行できる
- [ ] `cheap-writer` / `cheap-write` を指定した実行は未知の worker/command として
  拒否される（alias・fallback なし）
- [ ] 両 Worker の実行後、filesystem 差分が `meta.changes` に実測で記録される
- [ ] doc-writer の input schema が判断材料（decisions/facts 相当）を入力として
  受け取れる構造である
- [ ] Runtime（core/runtime-pi）に Worker 固有分岐が追加されていない

## Deferred Design Concerns

- input/output JSON Schema の具体的内容・instructions 文言 | design で確定
- targets 外書き込みの機械的制約（generic write-scope）| 別 spec（Writer
  hardening phase）で扱う | 当面は指示 + `meta.changes` 検証
- doc-writer の出力品質・hallucination 検証 | benchmark phase で実測

## Sources and Relations

- 一次ソース: DISC-0004（V2 productization briefing）
- supersedes: SPEC-0003（cheap-writer）
- 関連: SPEC-0002（bulk-reader）、ADR-0005/0006/0007

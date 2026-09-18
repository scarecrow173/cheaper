---
id: "SPEC-034r05iHK8bA7g7YRKGyQr"
type: "spec"
status: "superseded"
title: "cheap-writer built-in Worker"
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
  depends-on:
    - "cheaper-v1-worker-runtime-cli.md"
  blocks: []
  supersedes: []
  superseded-by:
    - "code-writer-and-doc-writer-built-in-workers.md"
  related:
    - "../adr/model-tier-model-resolver.md"
    - "../adr/wrapper-measured-deterministic-metadata.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  refines:
    - "cheaper-v1-worker-runtime-cli.md"
  refined-by: []
  derives-from:
    - "../discovery/cheaper-v1-pi-sdk.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheap-writer built-in Worker

## What Should Be Built

機械的・パターンベースのコード生成を cheap model へ委譲し、workspace を直接変更する built-in Worker `cheap-writer`（引継ぎ §3.2）。`cheaper cheap-write`（= `cheaper run cheap-writer`）として公開する。

## Why Now

tests / DTO / types / configs / boilerplate / repetitive adapters / simple mechanical refactoring などの機械的生成を frontier model から逃がすため（§1, §3.2）。

## Users and Value

- 親 Coding Agent: 生成物全文を受け取らず、workspace への適用と diff 統計だけ得られる
- 開発者: 定型的コード生成のコストを削減できる

## Scope

### In Scope

- tools: `read` / `grep` / `find` / `ls` / `write` / `edit`
- model tier: `low`
- workspace を直接変更する（生成コード全文を親 Agent へ返すことを基本としない）
- changed files / diff 統計を wrapper が実測して result に含める（§2.6）
- abort しても automatic rollback しない。mutation 済みの可能性は structured result/error で `PARTIAL_MUTATION` を明示（§34）

### Out of Scope

- architecture decisions / security-sensitive logic / concurrency / subtle debugging / ambiguous business rules（§3.2 対象外）
- automatic Git rollback（§39 non-goal）
- shell/network tool
- input/output JSON Schema の具体的内容（§40 未確定事項、design で確定）

## Requirements

- workspace 内への write/edit のみ許可し、workspace 外への変更は拒否すること
- changed files・diff 統計（added/deleted lines 等）を LLM 自己申告ではなく wrapper が filesystem から実測すること
- abort 後に rollback せず、部分的変更が残った場合は `PARTIAL_MUTATION`（exit 9）で明示すること

## Acceptance Criteria

- [ ] 有効な JSON 入力を与えたとき、workspace が直接変更され、宣言 schema に適合する structured result が返る
- [ ] result の changed files / diff 統計が wrapper 実測値と実際の filesystem 差分で一致する
- [ ] workspace 外への write/edit tool call が拒否される
- [ ] mutation 後に abort されたとき、automatic rollback されず `PARTIAL_MUTATION` と exit 9 が返る
- [ ] mutation が発生しなかった abort では `ABORTED` exit 8 が返る

## Deferred Design Concerns

- input/output JSON Schema、reference files、target/multiple targets、partial writes、changed files 算出、diff 統計の詳細 | 理由: 引継ぎ §40 未確定 | 再着手: design-doc | 無視時リスク: partial mutation の表現不統一

## Sources and Relations

- 一次ソース: `docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md` §3.2
- 上流: DISC-034r05iHy2gdnp377SlYbj / refines: SPEC-034r05iHK8bA7g59V25ltC
- 関連 ADR: ADR-034r05iHlWosDpLI5iJRbx（model tier）, ADR-034r05iHlWosDpMXiQ38yf（wrapper 実測）, ADR-034r05iHlWosDpO699OwYF（filesystem boundary）

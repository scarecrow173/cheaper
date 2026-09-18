---
id: "SPEC-034r05iHK8bA7g6PcCiDBp"
type: "spec"
status: "implemented"
title: "bulk-reader built-in Worker"
created: "2026-09-15"
updated: "2026-09-17"
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
  superseded-by: []
  related:
    - "../adr/model-tier-model-resolver.md"
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

# bulk-reader built-in Worker

## What Should Be Built

大量・巨大なコードを cheap model へ読ませ、圧縮された findings を親 Agent へ返す built-in Worker `bulk-reader`（引継ぎ §3.1）。`cheaper bulk-read`（= `cheaper run bulk-reader`）として公開する。

## Why Now

frontier model へ巨大コンテキストを渡さず、repository investigation / call flow / dependency tracing / large-context summarization を安価なモデルで済ませるため（§1, §3.1）。V1 の最初の built-in Worker のひとつ。

## Users and Value

- 親 Coding Agent: 巨大コンテキストを frontier model に載せず、短い structured findings のみ受け取れる
- 開発者: 大規模コード探索のコストを削減できる

## Scope

### In Scope

- tools: `read` / `grep` / `find` / `ls`（portable tool vocabulary、§10）
- model tier: `low`（§13）
- ファイル変更を行わない read-only 実行
- 結果は短い structured findings を宣言した output schema に適合させて返す
- 入力は JSON task input

### Out of Scope

- workspace への書き込み（write/edit/shell/network は付与しない）
- APM Hook / Skill 側の redirect ロジック（§36–37、別 spec へ deferred）
- input/output JSON Schema の具体的内容（§40 未確定事項、design で確定）

## Requirements

- read-only: 実行による filesystem 変更が 0 件であることを wrapper が実測して検証すること
- 入力は JSON、宣言した input contract に従う
- 出力は宣言した output schema に適合しなければならない（schema validation は Runtime Invariant）
- workspace 外のファイルは読めないこと（filesystem boundary、§26, §31）

## Acceptance Criteria

- [ ] 有効な JSON 入力を与えたとき、宣言 schema に適合する structured findings が stdout に返る
- [ ] 実行後の filesystem 差分が wrapper 実測で 0 件であるとき `ok: true` が返る
- [ ] workspace 外 path を読もうとする tool call が拒否される
- [ ] 入力が不正 JSON / contract 違反のとき non-zero exit と structured error が返る
- [ ] 結果が frontier model へ返すに足る短い findings に限定される（生成コンテンツ全文を含まない）

## Deferred Design Concerns

- input/output JSON Schema、path semantics、directory/glob handling、relevant symbol / suggested reads / source excerpt 表現 | 理由: 引継ぎ §40 未確定 | 再着手: design-doc で contract 確定 | 無視時リスク: 親 Agent との contract が曖昧なまま実装

## Sources and Relations

- 一次ソース: `docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md` §3.1
- 上流: DISC-034r05iHy2gdnp377SlYbj / refines: SPEC-034r05iHK8bA7g59V25ltC
- 関連 ADR: ADR-034r05iHlWosDpLI5iJRbx（model tier）, ADR-034r05iHlWosDpO699OwYF（filesystem boundary）

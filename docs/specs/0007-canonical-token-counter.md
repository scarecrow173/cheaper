---
id: "SPEC-0007"
type: "spec"
status: "proposed"
title: "Canonical Token Counter, count_tokens tool, and count-tokens CLI"
created: "2026-09-16"
updated: "2026-09-16"
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
  supersedes: []
  superseded-by: []
  related:
    - "../adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "0002-bulk-reader-built-in-worker.md"
  refines: []
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

# Canonical Token Counter, count_tokens tool, and count-tokens CLI

## What Should Be Built

`cheaper` 本体に単一の canonical Token Counter を実装し、2 つの入口から共有する。

- `count_tokens` tool（portable tool vocabulary 追加）: Worker が workspace 内 file の
  token 数を測定できる
- `cheaper count-tokens <path>` CLI command: LLM/model/provider を起動せずに
  token 数だけを machine-readable JSON で返す。APM Hook が large-file 判定に使う

## Why Now

large file の判定を file size・行数ではなく token 数で行うことが large-context
delegation の前提である。測定ロジックを APM 側へ複製すると drift するため、
`cheaper` 本体に 1 実装を置き、Worker tool と CLI の双方が同じ canonical
count を返す必要がある。

## Users and Value

- 親 Coding Agent / APM Hook: Read 前に `cheaper count-tokens` で file を測定し、
  threshold 超過なら bulk-reader へ delegate できる
- Worker: 巨大 file を読む前に `count_tokens` で規模を確認できる

## Scope

### In Scope

- workspace-relative file → filesystem boundary validation → file read →
  tokenization → token count の一連処理を `@cheaper/core` に 1 実装として置く
- tokenizer 名/version を測定結果に含める（観測可能）
- `count_tokens` tool を portable tool vocabulary へ追加し runtime-pi registry で提供
- `count-tokens` CLI command（`--cwd`・`--format`・`--quiet`）。envelope/exit code・
  stdout=structured output・stderr=diagnostics の既存設計に従う
- 決定的で再現可能な tokenizer（文字数 heuristic は使わない）

### Out of Scope

- 全 provider・全 model の tokenizer 対応抽象化（canonical tokenizer を 1 つ決めて
  一貫して使う。provider billing の再現が目的ではない）
- large-file threshold そのもの（delegation 判断は APM 側設定。別 spec）
- 複数 file / glob の cumulative count（必要性が benchmark で確認できた場合のみ）
- APM Hook 実装（別 spec）

## Requirements

- token 数は決定的・再現可能な tokenizer で算出する。文字数÷4 等の heuristic を
  主要 metric に使わない
- `count_tokens` tool と `count-tokens` CLI は同一の canonical 実装を呼び、
  同一 file に同一の count を返す
- filesystem boundary / symlink policy を必ず通す。workspace 外 file の count を
  許可しない
- 返却値に file 本文を含めない。少なくとも `path`・`tokens`・`tokenizer`、
  必要に応じ `bytes` を含める
- CLI は model/provider/LLM を起動しない（discovery・auth に触れない）
- tokenizer の実装名・encoding 名・version が結果から分かる

## Acceptance Criteria

- [ ] 同一 file について CLI と `count_tokens` tool が同一 token 数を返す
- [ ] `cheaper count-tokens <path>` が LLM 起動なしに envelope JSON を stdout へ返す
- [ ] workspace 外 path・境界外 symlink が拒否される
- [ ] 同一入力で count が再現する（決定性 test）
- [ ] tokenizer 名/version が result に含まれる
- [ ] 存在しない file・binary file の振る舞いが定義され test される

## Deferred Design Concerns

- tokenizer library の選定 | ADR で確定（js-tiktoken をユーザー承認済み）
- binary/不存在 file の扱い・返却 schema 詳細 | design で確定
- threshold の値・配置 | benchmark phase で実測して別途確定

## Sources and Relations

- 一次ソース: DISC-0004（V2 productization briefing）
- 関連: SPEC-0002（bulk-reader）、ADR-0007（filesystem boundary）

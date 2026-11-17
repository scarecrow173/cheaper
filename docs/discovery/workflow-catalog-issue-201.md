---
id: "DISC-034ZivKUnqfir09E2bIRBc"
type: "discovery"
status: "active"
title: "Workflow catalog and read-only list for issue 201"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["cheaper maintainers"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/201"]
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
    - "../specs/workflow-file-interface.md"
    - "../specs/workflow-contract-compiler.md"
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

# Workflow catalog and read-only list for issue 201

## Exploration Goal

親 Agent が保存済み Workflow 候補を、private DAG や全 file 走査なしに軽量に
列挙できるかを決める。Epic #200 の先行基盤として、catalog は発見用索引で
あり、定義・policy・grant・実行の正本ではない。実行経路は既存の明示 path
(`workflow interface|run <path>`)のまま維持する。

## Key Issues and Assumptions

- Confirmed: 現行 CLI の `workflow` arm は `interface|run <path>` のみ受理
  (`args.rs`)。named lookup 経路は存在しない(#69 で延期済み)。
- Confirmed: `resolve_workspace_layout` は canonical workspace root と
  `dirs.project_dir = <root>/.cheaper` を返す。catalog 配置は
  `<root>/.cheaper/workflows/catalog.json` が自然(保存場所規約と一致)。
- Confirmed: workflow file の boundary は loader.rs の
  `resolve_workspace_path` + denylist + symlink policy。catalog の
  entry path 検証と manifest read は同じ policy 入力を再利用する。
- Confirmed: utility command 規約は `utility_success_envelope` +
  `write_output` + `record_usage` 非記録(`history`/`policy`/`route check`
  と同型)。`workflow list` はこの規約に従う。
- Assumption: manifest の構造的破損(JSON parse 不可・schemaVersion
  非対応・上限超過)は「一覧可能な情報なし」として error envelope で失敗
  させ、entry 単位の異常(missing path・path escape・duplicate id・
  stale)は `ok:true` + diagnostics + per-entry status で診断する。
  「壊れた manifest を空カタログと誤認しない」が境界。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| catalog read/validate を `cheaper-workflow` crate に置く(loader.rs sibling) | workflow file contract の owner が一元。将来の書込み側(#204)も同一 schema/validation を再利用できる | workflow crate に compiler/engine 以外の module が増える | Yes |
| catalog を `cheaper-cli` crate に置く | cli 側だけで完結 | workflow file contract が 2 crate に分散し schema drift の温床 | No |
| manifest なしに `.cheaper/workflows/*.json` を glob 探索する | 登録不要で見つかる | private DAG を開くか怪しい file を一覧するかの二択になり省トークン目的に反する。issue が manifest 方式を明示 | No |
| `workflow list` の代わりに `workflow catalog` 等の別名 | 用途が明確 | `history list` と同型の `list` 動詞が既存規約 | No |

## Tentative Conclusions and Hypotheses

Confirmed: catalog は bounded JSON manifest(schemaVersion 1、entries は
`{id, path, summary, tags?}`)。`workflow list` は read-only utility
command として entries + diagnostics を utility envelope で返し、
private jobs/prompt/binding を一切含めない。

Hypothesis: entry 検証は「resolve 可能・workspace 内・deny 非該当・
file 存在」を stat までに留め、compiler 実行・DAG/prompt 読込は行わない。
候補選定後の正式 validation は既存 `workflow interface <path>` に委譲。

Confirmed: `--limit` は bounded 出力制御として提供するが cursor は不要
(単一 bounded file)。manifest 欠如は空一覧として成功扱いし、明示 path
経路を妨げない。

## Open Questions

- malformed manifest を error envelope で閉じるか `ok:true` + diagnostics
  に留めるか → design で確定(entry 異常との二分が素直)。
- manifest 上限(entries 数・file bytes・id/summary/tags 長)の具体値 →
  design で既存 bounded 規約(history limit 等)に合わせて確定。

## Promotion Candidates

- [x] spec-doc needed: catalog manifest 契約・`workflow list` 出力/exit
  契約・diagnostics・非責務(named lookup 非導入)の固定。
- [x] adr-doc needed: 「catalog は発見用 index であって権限/実行の正本で
  はない」ことと、manifest 破損時の fail-closed 境界を記録。

## Sources and Relations

Issue #201(親 Epic #200)が product source。参照済み spec:
`workflow-file-interface`(#69/#70)、`workflow-contract-compiler`(#64)。
code: `crates/cheaper-cli/src/args.rs`・`cli.rs`・`workflow_run.rs`・
`workflow_context.rs`・`compose.rs`、`crates/cheaper-workflow/src/loader.rs`、
`crates/cheaper-core/src/config/dirs.rs`。

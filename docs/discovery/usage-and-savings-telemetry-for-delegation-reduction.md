---
id: "DISC-034rzEbmKarhQzw41fhumg"
type: "discovery"
status: "resolved"
title: "Usage and savings telemetry for delegation reduction"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/44"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/delegation-usage-measurement-log.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../adr/usage-jsonl-contract-canonical.md"
    - "../designs/delegation-usage-measurement-log.md"
  defers: []
  deferred-by: []
---

# Usage and savings telemetry for delegation reduction

## Exploration Goal

Issue #44: cheaper による delegation で親 Coding Agent 側の
context/token/cost をどの程度削減できたかを、実測可能な指標と
仮定に基づく推定を分離して記録・集計できるようにする。
既存の `meta.usage` と `<userConfigDir>/usage.jsonl`
（SPEC-034r05iHK8bA7gveeDA5HC / ADR-034r05iHrZUE6WljM1ZMiX）の上に、
「親 context に投入せずに済んだ token」の実測と
`cheaper usage` 集計コマンドを追加する方向を確定する。

## Key Issues and Assumptions

- actual / measured / estimated の 3 区分を schema・UI 双方で
  混同させないことが中核要件（issue の設計原則）。
- "source tokens" を何で測るかが最大の設計論点。
  `meta.usage.input` は Pi の message.usage の累積で、
  turn ごとに context 全体を再計上するため「source 内容量」
  としては過大。tool output（read/grep/find/ls が worker に返した
  text）の token 和のほうが「親の代わりに ingest した内容量」
  として素直に解釈できる。
- returned tokens は `result` JSON の canonical token count で
  決定的に測れる（core の o200k_base tokenizer を再利用）。
- estimated savings は #45 の pricing metadata layer に依存するが
  並行開発中のため、具体的実装名に依存しない疎結合 seam にする。
- code-writer / doc-writer は source↔returned の直接比較が
  成立しにくいため savings percentage を自動生成しない
  （issue 明記）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: tool output の token 和を sourceTokens とする | worker が実際に ingest した内容量を deterministic に実測できる。worker 種別を問わず通用する | tool 1 回の output 単位なので file 単位の内訳は持たない。system prompt 分は含まれない | yes |
| B: `meta.usage.input` を sourceTokens とする | 追加計測なしで既存値を転用できる | turn ごとの context 再送を累積するため内容量を大幅に過大計上する。system prompt も混入 | no |
| C: 入力 `paths` 解決 file 群の token 和を事前計測 | 「対象 source」の解釈として直感的 | worker が実際に読んだ範囲と乖離する（部分 read・grep 絞り込み）。事前に全 file を走査するコストが増える | no |
| D: record への追加は usage.jsonl のみで envelope `meta` は変えない | envelope contract を一切触らない | 親 agent が per-run で即座に avoided context を参照できない | maybe |
| E: envelope `meta` にも offload section を追加 | 親 agent が run 直後に実測削減量を見られる。usage.jsonl と同一値を 1 箇所で生成して転写できる | envelope schema に optional field 追加（後方互換） | yes |

## Tentative Conclusions and Hypotheses

- (hypothesis→confirmed 相当) 計測点は sandboxed tool の
  execute 結果。`wrapToolBudget` と同じ wrap 層に meter を追加し、
  tool output text の canonical token 和を `meta` に載せる。
- (confirmed) record schema は `schemaVersion` を据え置き、
  optional `savings` section を追加する（既存 consumer は
  未知 field を無視でき、per-line parse で破損行も skip 可能）。
- (confirmed) `cheaper usage` は usage.jsonl を読み `--since` /
  `--worker` / `--provider` / `--model` で filter・集計する
  read-only utility command。記録経路（best-effort append）には
  触れない。
- (hypothesis) estimated savings は comparison model の pricing
  metadata が存在する場合のみ算出し、record/envelope では
  `estimated*` prefix で分離する。pricing の source・checkedAt・
  stale を出力に残し、価格不明なら推定額を出さない。

## Open Questions

- pricing metadata の提供形式（#45 側の実装）— 本 issue では
  「`<userConfigDir>` 配下の pricing file を読む seam」として
  抽象化し、#45 の具体名に依存しない。manageable downstream。
- stale の判定閾値（例: checkedAt から 30 日）— spec/ADR で
  確定する。pre-implementation blocker ではない。

## Promotion Candidates

- [ ] spec-doc needed: usage telemetry の要件（actual/measured/
  estimated 区分・`cheaper usage` 集計・filter・後方互換）
- [ ] adr-doc needed: sourceTokens の計測点（tool output meter）
  と record schema 拡張・pricing seam の疎結合方針

## Sources and Relations

- Issue #44: 要件の起点。
- SPEC-034r05iHK8bA7gveeDA5HC / ADR-034r05iHrZUE6WljM1ZMiX /
  ADR-034r2eId2hXJGWDjBylKVE / DESIGN-034r05iHy2gdnoXu6h71XL:
  既存 usage.jsonl contract と記録経路。
- `packages/cli/src/usage-log.ts`: `buildUsageRecord()` /
  `recordUsage()` の現行実装。
- `packages/runtime-pi/src/tools/budget.ts`: tool wrap 層
  （meter 追加候補位置）。
- `packages/runtime-pi/src/usage.ts` / `run.ts`: `WorkerUsage` /
  `WorkerRunMeta` の現行 shape。

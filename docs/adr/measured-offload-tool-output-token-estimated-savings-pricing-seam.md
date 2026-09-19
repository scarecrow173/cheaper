---
id: "ADR-034rzGnXqru47VgHNL16cS"
type: "adr"
status: "accepted"
title: "measured offload は tool output token 和・estimated savings は pricing seam 経由"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
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
  derives-from:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../discovery/usage-and-savings-telemetry-for-delegation-reduction.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "./usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "./usage-jsonl-contract-canonical.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# measured offload は tool output token 和・estimated savings は pricing seam 経由

## Context and Problem Statement

SPEC-034rzGnCaRQc5ceI95lf5i（Issue #44）は actual / measured /
estimated の 3 区分で delegation 削減効果を記録・集計する。
決めるべきこと: (a) measured offload の `sourceTokens` を何で
測るか、(b) usage.jsonl record schema をどう拡張するか、
(c) estimated savings が依存する pricing metadata を
並行開発中の #45 とどう疎結合にするか。

## Decision Drivers

* Issue #44: 「実測可能な削減指標と仮定を置いた推定金額を明確に
  分離」「価格不明 model について推定額を捏造しない」「
  pricing metadata が利用可能なら参照する疎結合」を要求する。
* `meta.usage.input` は Pi の message.usage.input の累積であり、
  turn ごとに context 全体（system prompt + 履歴）を再計上する
  — 「source の内容量」としては意味が違う。
* ADR-034r05iHrZUE6WljM1ZMiX / ADR-034r2eId2hXJGWDjBylKVE:
  usage.jsonl の記録対象 contract は確定済みで、本 issue は
  record の拡張であって記録条件の変更ではない。
* sandboxed tool は `wrapToolBudget` ですでに全 tool を wrap
  しており、output text を観測する choke point が既にある。

## Considered Options

### sourceTokens の計測点

* A: tool output text の canonical token 和（tool wrap 層に meter
  を追加）。
* B: `meta.usage.input` をそのまま sourceTokens に使う。
* C: worker input `paths` が指す file 群の token 和を事前計測。

### record schema の拡張

* i: `savings` optional section を既存 record に追加（
  schemaVersion 据え置き）。
* ii: `schemaVersion` を 2 に上げて新 record 型を分離。

### estimated savings の pricing 接続

* α: cheaper 側で pricing metadata の minimal consumer contract
  （file 配置 + entry shape）だけを定義し、provider は #45 側に
  委ねる。
* β: #45 の実装完了を待ってから接続する。

## Decision Outcome

Chosen option: "A + i + α"。

- **sourceTokens = tool output token 和**: run 中に sandboxed
  tool が返した text output を canonical tokenizer
  （o200k_base / js-tiktoken）で逐次 encode して加算する。
  「親の代わりに worker が ingest した内容量」として実測でき、
  worker 種別を問わず同じ意味を持つ。`meta.usage.input`（B）は
  context 再送を含み過大、paths 事前計測（C）は worker が実際に
  読んだ範囲と乖離するため採用しない。
- **record schema**: `savings: { sourceTokens, returnedTokens,
  avoidedContextTokens }` を run record の optional field として
  追加し `schemaVersion: 1` を維持する（option i）。未知 field を
  無視する consumer を壊さず、既存 record との混在を許容する。
  envelope 側は success `meta` に同値の `savings`（実測 3 field
  のみ）を optional で載せる。
- **estimated**: comparison model は `cheaper usage
  --compare-model <provider/id>` で指定する。pricing は
  `<userConfigDir>/pricing.json`（`models` map:
  `"<provider>/<id>" → { inputPerMillionUsd, outputPerMillionUsd,
  checkedAt, source }`）を読む minimal contract とし、file 不在・
  model 不明・stale（checkedAt が 30 日超または欠損）なら
  estimated を出力しない。#45 の実装名には依存しない。

### Consequences

* Good, because tool wrap 層の meter は worker 非依存で
  code-writer / doc-writer にも同じ計測が適用される。
* Good, because `schemaVersion` 据え置きで既存 consumer・
  集計の後方互換が保たれる。
* Good, because pricing seam を file contract に限定することで
  #45 の完了を待たずに本 issue を完遂できる。
* Bad, because tool output には grep/find の listing や error
  message も含まれ、「source file の内容量」と厳密には一致
  しない — 「親 context に投入されなかった token 数」という
  定義で文書化して吸収する。
* Bad, because pricing.json を書く側が無い間は estimated が常に
  非表示になる（機能として存在するが発動しない）。

### Confirmation

* fake-stream e2e で `meta.savings` と record の `savings` が
  tool output から実測されることを assert。
* `cheaper usage` が旧 record（savings なし）を含む log を
  壊さず集計することを unit/e2e test で確認。
* pricing.json 不在・対象 model 欠損・stale の各条件で
  estimated が非出力になることを unit test で確認。

## Implementation Plan

* Affected paths: `packages/runtime-pi/src/tools/`（meter wrap）、
  `packages/runtime-pi/src/run.ts`・`usage.ts`（meta への載せ）、
  `packages/cli/src/usage-log.ts`（record 拡張）、
  `packages/cli/src/args.ts`・`cli.ts`（`usage` command）、
  `packages/cli/src/text-format.ts`（text 出力）、
  `packages/core/src/token/count.ts`（string 用 count 公開）、
  `docs/guides/usage.md`。
* Patterns to follow: `wrapToolBudget` の tool wrap 方式、
  `recordUsage` の best-effort 記録、`parseDuration` の duration
  表記、envelope の optional field 追加。
* Patterns to avoid: `meta.usage.input` の sourceTokens 流用、
  価格不明時の推定出力、code-writer/doc-writer 向けの
  savings percentage 自動生成。
* Dependencies or configuration: 新規依存なし。
  `<userConfigDir>/pricing.json` は optional 入力。

## Verification

* [ ] fake-stream e2e: bulk-reader run で `meta.savings` の 3 field
      が正の実測値として記録される
* [ ] unit test: tool output meter が text の token 和を返す
* [ ] unit test: `cheaper usage` の filter・集計・0 件・旧 record
      混在
* [ ] unit test: pricing 不在/不明/stale で estimated 非出力
* [ ] `mise run check` が pass

## Pros and Cons of the Options

### Option A（採用）: tool output token 和

* Good, because worker が実際に ingest した内容量を
  deterministic に実測でき、全 worker に通用する。
* Bad, because file 単位の内訳を持たず、listing や error 出力も
  含む。

### Option B: `meta.usage.input` 流用

* Good, because 追加計測が不要。
* Bad, because turn ごとの context 再送を累積して内容量を大幅に
  過大計上し、「削減量」の指標として誤解を招く。

### Option C: 入力 paths の事前計測

* Good, because 「対象 source」の解釈として直感的。
* Bad, because worker の実際の read 範囲と乖離し、事前に全
  file を走査するコストが増える。

### Option ii（不採用）: schemaVersion 2

* Good, because schema を厳密に版管理できる。
* Bad, because optional field 追加で済む変更に対して新旧 2 系統の
  record 処理が必要になり、consumer 側の分岐が増える。

### Option β（不採用）: #45 完了待ち

* Good, because pricing contract を #45 の実装に正確に合わせ
  られる。
* Bad, because 本 issue の完了が別 worktree の進行にブロック
  される。minimal contract で先に seam を決めるほうが両者の
  結合を緩くできる。

## More Information

- Issue #44 / SPEC-034rzGnCaRQc5ceI95lf5i / DISC-034rzEbmKarhQzw41fhumg。
- ADR-034r05iHrZUE6WljM1ZMiX: usage.jsonl の置き場と
  best-effort append。
- ADR-034r2eId2hXJGWDjBylKVE: 記録対象 contract（変更しない）。
- `packages/runtime-pi/src/tools/budget.ts`: meter 追加位置の
  既存 wrap 層。


---
id: "ADR-0012"
type: "adr"
status: "accepted"
title: "Keep 8192 tokens as the default large-read threshold for the pre_read_code hook"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0007-bulk-read-delegation-benchmark.md"
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
    - "./0011-canonical-tokenizer-js-tiktoken.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0007-bulk-read-delegation-benchmark.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
---

# 12. Keep 8192 tokens as the default large-read threshold for the pre_read_code hook

## Context and Problem Statement

`pre_read_code` hook は `cheaper count-tokens` の結果が
`CHEAPER_READ_TOKEN_THRESHOLD` を超えた file の read を block し、
`bulk-read` への委譲を促す。8192 は実測前の provisional 値であり、
DISC-0007 の benchmark で実測根拠を取った上で確定する必要があった。

## Decision Drivers

- hook は safety net: 明確に大きい file だけを block し、通常の read を
  阻害しないこと
- 委譲の latency（実測 ~15s）が小 file では直接 read を大きく上回る
- parent が file の正確な内容を要する場面（編集対象の精読）での
  false positive block は製品の writer ワークフローを傷つける
- 積極的な委譲は hook ではなく parent の skill 自発利用が担う設計

## Considered Options

- 2048 tokens — corpus の 9.2% が対象、cost break-even (~2k) 付近
- 4096 tokens — corpus の 2.0% が対象
- 8192 tokens — corpus の 0.7%（lockfile 級）のみ対象

## Decision Outcome

Chosen: "8192 tokens"。

- 8k 超は ~32KB 以上で「明確に大きい」と見なせ、block の false positive
  リスクが最小
- 実測で 6267-token file の委譲は parent context を 96% 削減した。
  8k+ では同等以上の削減が見込める
- 4-8k 帯は編集対象 source file として普通に出現し、ここを block すると
  code-writer 用途の parent フローを阻害する
- 純コスト損益分岐 (~2k) より高いが、latency と false positive の非コスト
  要因を優先した意図的な保守値
- `CHEAPER_READ_TOKEN_THRESHOLD` env で deployment ごとに下げられる。
  本番運用の telemetry で再評価する余地を残す

### Consequences

- 本リポジトリ規模では hook が発火するのは lockfile 級のみで、
  日常フローでは不可視
- 4-8k file の不要な read は hook では防がない。skill の自発利用か
  env 閾値引き下げで対処する
- threshold の変更は bundle 再生成（`scripts/bundle-cli.mjs`）が要らない
  単一変数の変更で済む

## Implementation Plan

- `pre-read-code.js` の `DEFAULT_TOKEN_THRESHOLD = 8192` を維持（変更なし）
- DESIGN-0013 の provisional 注記を本 ADR への参照に更新
- benchmark 結果は EXP-0003 に記録済み

## Verification

- DISC-0007 の分布・委譲実測が本値の根拠
- `pre-read-hook.test.ts` が default threshold で pass し続けること
- env 上書きが e2e で検証済み（`CHEAPER_READ_TOKEN_THRESHOLD` 経路）

## Pros and Cons of the Options

### 2048

- Good: corpus の過半 token（50.7%）を委譲対象にできる。context 保護が最大
- Bad: p75-90 の通常 file まで block し、~15s latency の friction が大きい。
  false positive 多発

### 4096

- Good: 重い file（corpus token の 36.7%）を捕捉しつつ file 数は 2%
- Bad: 4-8k 帯の編集対象 file を block する false positive が残る

### 8192 (chosen)

- Good: 明確に大きい file のみ。false positive 最小。96% 削減の実績あり
- Bad: 中規模 file（2-8k）の累積 context 汚染は skill 任せになる

## More Information

- `docs/discovery/0007-bulk-read-delegation-benchmark.md` — 計測データ
- `docs/impl/exp/0003-bulk-read-delegation-benchmark.jsonl` — 実験記録

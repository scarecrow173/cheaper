---
id: "DISC-034r05iHy2gdnpcXzmUKaV"
type: "discovery"
status: "resolved"
title: "Huge-file token count guard and fail-closed hook behavior"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-16-v2-productization-final-evaluation.md"
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
    - "../adr/canonical-tokenizer-js-tiktoken.md"
    - "../adr/default-large-read-threshold.md"
    - "./bulk-read-delegation-benchmark.md"
    - "./shell-read-large-file-detection-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Huge-file token count guard and fail-closed hook behavior

## Exploration Goal

`count-tokens` が巨大 file（数百MB〜GB級のログ・minified bundle・バイナリ
等）に対して OOM / timeout した場合の挙動を確定する。v2 productization
の review 後続課題として、fail-open が最悪ケース（巨大 file の直接
read）を招く構造的問題を解消する。

## Key Issues and Assumptions

- `countFileTokens`（`packages/core/src/token/count.ts`）は file 全体を
  `readFile` → `toString("utf8")` → `encode` とメモリに展開する。
  file size の ~10-20 倍のメモリを消費し、巨大 file で OOM しうる
  （confirmed: 実装を直接確認）
- `encode` は同期 CPU-bound で、大 file では長時間ブロックする
  （confirmed: js-tiktoken の encode 契約）
- `pre-read-code.js` hook は `spawnSync timeout: 30_000` で、error・
  非0 exit・JSON parse 失敗をすべて `exit(0)` の fail-open で扱う
  （confirmed: hook 実装を直接確認）
- 結果として「最も block すべき巨大 file ほど測定に失敗し、fail-open
  で parent が直接 read する」という構造的な穴がある（confirmed）
- `count_tokens` tool（runtime-pi）と `count-tokens` CLI は同じ
  `countFileTokens` を呼ぶため、guard は core に置けば全経路をカバー
  できる（confirmed: tools/count-tokens.ts を直接確認）
- `tokens ≤ bytes` が常に成立する（1 token は最低 1 byte を表す）ため、
  bytes ≤ threshold の file は encode なしに安全判定できる
  （hypothesis → 実装で検証可能な決定的性質）

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| fail-open 維持（現状） | 測定不能でも read を邪魔しない | 巨大 file が parent context を破壊する最悪ケースを放置 | no |
| fail-closed: 測定不能 = 閾値超過扱い | hook の目的（context 保護）と整合。byte size は stat で決定的に取れ count 自体をスキップできる | 高圧縮率 file で false positive ありうるが、bulk-read が正常処理するため被害小 | yes |
| 先頭 N bytes sampling で推定 count | 真の token 数に近い推定が可能 | canonical deterministic count の契約（ADR-034r05iHrZUE6W3U1rbJq4）を壊す。実装複雑 | no |
| hook 側のみ byte pre-check | 実装が小さい | CLI / tool 単体経路は無防備のまま OOM する | no |

## Tentative Conclusions and Hypotheses

- confirmed: 巨大 file は fail-closed で扱う。byte cap を core
  `countFileTokens` に導入し、cap 超過時は typed error を返す。CLI は
  dedicated error code に map し、hook はそれを閾値超過と同じ block
  （exit 2 + 委譲 guidance）として扱う
- confirmed: cap default は 16MB、env `CHEAPER_COUNT_MAX_BYTES` で
  deployment ごとに上書き可（ADR-034r05iHrZUE6W4XOqUBih と同じ「保守的 default + env
  調整余地」の方針）。16MB ならメモリ peak ~300MB・encode 数秒で
  hook の 30s timeout に十分収まる
- confirmed: ユーザー承認済み（fail-closed 方針・16MB + env 上書きを
  deep-dive で確認）

## Open Questions

- bulk-reader 側が巨大 file を chunked read する際の同等 OOM リスクは
  本 chain の scope 外。別 chain として扱うかは後続で判断
- 巨大 file を block した際の guidance メッセージに byte size を
  含めるか（観測性の観点では含める方向）

## Promotion Candidates

- [x] spec-doc needed: huge-file guard の要求・fail-closed 契約・
      acceptance criteria
- [x] adr-doc needed: 「byte cap による fail-closed 測定ガード」の
      architecture 決定記録（canonical count 契約との関係）

## Sources and Relations

- `docs/handoff/2026-09-16-v2-productization-final-evaluation.md` —
  v2 review の後続課題として特定された本 item
- ADR-034r05iHrZUE6W3U1rbJq4: canonical tokenizer 契約（cap は count 契約に影響しない）
- ADR-034r05iHrZUE6W4XOqUBih: read threshold の env 上書きパターン（cap も同型にする）
- DISC-034r05iHy2gdnpb598KIca: bulk-read 委譲の効果実測（fail-closed 時の委譲先の実績）

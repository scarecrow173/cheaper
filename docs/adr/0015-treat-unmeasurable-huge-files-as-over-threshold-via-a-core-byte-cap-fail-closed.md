---
id: "ADR-0015"
type: "adr"
status: "accepted"
title: "Treat unmeasurable huge files as over-threshold via a core byte cap (fail-closed)"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0008-huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
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
    - "./0012-default-large-read-threshold.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0012-huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0008-huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
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
    informed: []
---

# 15. Treat unmeasurable huge files as over-threshold via a core byte cap (fail-closed)

## Context and Problem Statement

`countFileTokens` は file 全体を `readFile` → `toString("utf8")` →
`encode` とメモリに展開する。巨大 file（数百MB〜GB級のログ・minified
bundle・バイナリ）では OOM または長時間の同期 CPU ブロックが起きる。
`pre-read-code.js` hook は `spawnSync timeout: 30_000` で、測定の
error・非0 exit・JSON parse 失敗をすべて `exit(0)` の fail-open で
扱うため、**最も block すべき巨大 file ほど測定に失敗し parent が
直接 read する** 構造的な穴がある。

## Decision Drivers

- hook の存在意義は parent context 保護であり、「大きすぎて測れない
  file を通す」は目的の否定
- byte size は `stat` で決定的・低コストに取得でき、count 自体を
  スキップできるため OOM/timeout の原因を除去できる
- guard は core `countFileTokens` に置けば CLI `count-tokens`・
  `count_tokens` tool・hook の全経路を同一実装でカバーできる
- canonical deterministic count 契約（ADR-0011）は cap 未満 file に
  限って維持される

## Considered Options

- fail-open 維持 — 測定不能でも read を許可（現状、却下）
- fail-closed byte cap — `maxBytes` 超過は read/encode せず typed error
  → CLI は専用 error code → hook は閾値超過同等の block（採用）
- 先頭 N bytes sampling で近似 count — ADR-0011 の決定的 count 契約を
  壊すため却下
- hook 側のみ byte pre-check — CLI / tool 単体経路が無防備のまま残る
  ため却下（core cap が正規の置き場所）

## Decision Outcome

Chosen option: "fail-closed byte cap"。

- core `countFileTokens` に `maxBytes` option（default 16MB）を追加し、
  `stat` で cap 超過を検出したら read/encode せず typed error を投げる
- CLI `count-tokens` は cap error を専用 error code の envelope に map
  する。`CHEAPER_COUNT_MAX_BYTES` env で deployment ごとに cap を
  上書き可能（ADR-0012 の threshold env と同型）。この env 上書きは
  CLI `count-tokens`（= hook 経路）のみに効き、runtime `count_tokens`
  tool は core default 16MB のままとする
- hook は cap error code を閾値超過と同等に扱い、exit 2 + bulk-read
  委譲 guidance を返す。fail-closed は「`FILE_TOO_LARGE` または正常
  count で threshold 超過」に限定し、file 不在・workspace 外・input
  異常・CLI spawn error / timeout / parse 不能などの infra 障害は
  従来どおり fail-open（exit 0）を維持する
- `tokens ≤ bytes` が常に成立するため、bytes ≤ threshold の file は
  encode なしに「閾値以下」と判定できる（実装上の最適化として許容）

### Consequences

* Good, because 巨大 file が parent context を破壊する最悪ケースが
  構造的に消え、OOM/timeout の原因である count 自体を実行しない。
* Good, because guard が core にあるため CLI・tool・hook の全経路が
  同じ契約で守られる。
* Bad, because 高圧縮率の巨大 file（bytes 大・tokens 小）は false
  positive で block される。parent context は保護されるが、委譲先の
  bulk-reader は現状 file 全体を Buffer に読む `readFile` と 1MB 超を
  skip する walker で構成されており、cap 超過 file を安全に処理できる
  保証はまだない。bounded/chunked な巨大 file 対応は follow-up work
  であり、現状では委譲自体が非常に大きな file で失敗しうる。
* Bad, because cap 超過 file の正確な token 数は取得できない（envelope
  の error code で「測定不能」を表現する）。

### Confirmation

SPEC-0012 の acceptance criteria（cap error の envelope・env 上書き・
hook fail-closed e2e・cap 未満の parity）で検証する。

## Implementation Plan

* Affected paths: `packages/core/src/token/count.ts`,
  `packages/cli/src/cli.ts`, `packages/cli/src/args.ts`（env 解釈位置）、
  `.apm/hooks/pre-read-code.js`, 関連 test
* Patterns to follow: typed error + dedicated envelope error code（既存
  `WorkspaceEscapeError` → `INVALID_REQUEST` map と同型）、env 上書きは
  `CHEAPER_READ_TOKEN_THRESHOLD` の parse/fallback と同型
* Patterns to avoid: sampling による近似 count、hook 側でのみの guard、
  fail-open の対象拡大
* Dependencies or configuration: `CHEAPER_COUNT_MAX_BYTES` env（正の
  整数、不正値は default 16MB にフォールバック。CLI `count-tokens`
  経路のみ有効で、`count_tokens` tool は core default を使う）

## Verification

* [ ] cap 超過 file で `count-tokens` が専用 error code・非0 exit し、
      OOM/timeout しない
* [ ] `CHEAPER_COUNT_MAX_BYTES` の上書き・不正値フォールバックを test
      で検証
* [ ] hook が cap error で exit 2 + 委譲 guidance を返す e2e
* [ ] cap 未満 file の count 結果が従来と一致（parity）
* [ ] `count_tokens` tool でも同一 guard が効く

## Pros and Cons of the Options

### fail-open 維持

* Good, because 計測不能な環境でも通常 read を阻害しない。
* Bad, because 巨大 file の直接 read による context 破壊という、hook
  の目的そのものを裏切る最悪ケースを放置する。

### fail-closed byte cap (chosen)

* Good, because 安全側に倒れ、OOM/timeout の原因を根本から除去する。
  byte size は決定的で実装が単純。
* Bad, because 高圧縮率 file で false positive block が起きうる
  （委譲先 bulk-reader の巨大 file 対応は follow-up work であり、
  現状は委譲自体が失敗しうる）。

### 先頭 N bytes sampling

* Good, because 真の token 数に近い推定が得られる可能性がある。
* Bad, because canonical deterministic count 契約（ADR-0011）を壊し、
  「約何 tokens」という不確かな値で block 判断をすることになる。

### hook 側のみ byte pre-check

* Good, because 変更面が最小。
* Bad, because `count-tokens` CLI・`count_tokens` tool の単体経路が
  依然 OOM する。guard の正規の置き場所は core。

## More Information

* `docs/discovery/0008-huge-file-token-count-guard-and-fail-closed-hook-behavior.md`
  — failure mode・alternatives の調査
* `docs/specs/0012-huge-file-token-count-guard-with-fail-closed-hook-blocking.md`
  — 要求と acceptance criteria
* ADR-0011: canonical tokenizer 契約（cap 未満 file の count 決定性は
  維持）
* ADR-0012: threshold env 上書きパターン

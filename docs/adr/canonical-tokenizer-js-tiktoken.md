---
id: "ADR-034r05iHrZUE6W3U1rbJq4"
type: "adr"
status: "superseded"
title: "Use js-tiktoken o200k_base as the single canonical tokenizer"
created: "2026-09-16"
updated: "2026-10-08"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/v2-productization-initiative-briefing.md"
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
  superseded-by:
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  related:
    - "../specs/canonical-token-counter.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/v2-productization-initiative-briefing.md"
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

# 11. Use js-tiktoken o200k_base as the single canonical tokenizer

## Context and Problem Statement

large-file delegation 判定には file bytes・行数ではなく token 数を使う（SPEC-034r05iHK8bA7gdbCbWupr）。
これには決定的・再現可能な tokenizer が必要だが、`@earendil-works/pi-ai` /
`pi-coding-agent` / `pi-agent-core` を含む依存ツリー全体に再利用可能な LLM
tokenizer が存在しないことを全走査で確認した（一致する `tokenize` は markdown
lexer・JSON parser・AWS auth token provider のみ）。よって新規 dependency の
追加が必要であり、どれを canonical として採用するかの決定が要る。

## Decision Drivers

* 目的は provider billing token 数の完全再現ではなく、large-context delegation
  判定に使える一貫した再現可能な metric
* pure JS で Node 全環境（Linux/Windows/macOS）に追加依存なく動くこと
* 全 provider/model の tokenizer 抽象化は不要（YAGNI）。canonical tokenizer を
  1 つに固定して drift を防ぐ
* 文字数 heuristic への退避は不可（指示 §10）

## Considered Options

* Option A: `js-tiktoken`（pure JS port of tiktoken、依存ゼロ、MIT、
  cl100k_base/o200k_base 対応）
* Option B: `gpt-tokenizer`（pure TS/JS、同等機能、API surface がやや大きい）
* Option C: `@dqbd/tiktoken`（OpenAI tiktoken の WASM バインド、本家に近いが
  WASM 初期化・配布が重い）
* Option D: 文字数÷4 heuristic（決定的だが token metric として不十分）

## Decision Outcome

Chosen option: "Option A", because pure JS・依存ゼロで全対象環境へ追加負担なく
導入でき、BPE ベースの決定的 token count を提供するため。ユーザー承認済み。

併せて以下を決定する。

- canonical encoding は **`o200k_base`** とする（現行 OpenAI 系 encoding。
  cheaper の production provider 検証対象は OpenAI であり、大域的に広い語彙の
  encoding は routing metric として保守的）
- tokenizer 実装・encoding 名・version は count 結果へ必ず含め、計測値の由来を
  観測可能にする
- dependency は `js-tiktoken` を `@cheaper/core` へ pin して追加する。
  model/provider 別の encoding 切替機構は作らない

### Consequences

- count は provider 実 billing と一致しない場合があり得るが、threshold は同じ
  metric で benchmark して決めるため一貫性は保たれる
- 将来別 encoding が必要になった場合は新しい ADR で canonical の切替を検討する
  （並存・抽象化は行わない）

## Implementation Plan

- `js-tiktoken` を `@cheaper/core` の dependency へ pin して追加する
- `packages/core/src/token/count.ts` に `countFileTokens` と
  `TOKENIZER_IDENTITY`/`TOKENIZER_VERSION` を実装し、`src/index.ts` から export
- `count_tokens` を portable tool vocabulary に追加し、runtime-pi tool と
  `count-tokens` CLI の双方から同一関数を呼ぶ（TASK-034r05iHQeQpDpQjb4veqT〜0069）

## Verification

- `TOKENIZER_VERSION` が install 済み `js-tiktoken` の package.json version と
  一致することを test で検証する
- deterministic count・multibyte/binary・workspace escape・symlink policy・
  CLI/tool parity を test する

## Pros and Cons of the Options

- Option A (js-tiktoken): pure JS・依存ゼロで軽い。maintainer 数が少ない点は
  受け入れる
- Option B (gpt-tokenizer): 同等機能だが API surface が大きく、cheaper の
  必要最小限を超える
- Option C (@dqbd/tiktoken): 本家 tiktoken に最も近いが WASM 初期化と配布が
  重く、Node-only 用途では過剰
- Option D (chars/4): 依存ゼロだが token metric として信頼できず、
  delegation 判定の根拠に使えない

## More Information

- 決定経緯: DISC-034r05iHy2gdnp7GkO7QDK（V2 productization initiative briefing）でユーザーが
  `js-tiktoken` を選択
- 契約詳細: DESIGN-034r05iHy2gdnoEfk3jERu、SPEC-034r05iHK8bA7gdbCbWupr

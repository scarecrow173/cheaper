---
id: "SPEC-034r05iHK8bA7gveeDA5HC"
type: "spec"
status: "implemented"
title: "Continuous usage measurement log for delegations"
created: "2026-09-18"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/13"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/delegation-usage-measurement-log.md"
  depends-on:
    - "./ollama-real-provider-worker-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/cli-contract-envelope-exit-codes.md"
    - "../impl/ir/run-real-provider-smoke-test-with-ollama.md"
    - "./usage-and-savings-telemetry-aggregation.md"
  refines: []
  refined-by:
    - "./usage-log-recording-scope-docs.md"
  derives-from:
    - "../discovery/v2-productization-initiative-briefing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Continuous usage measurement log for delegations

## What Should Be Built

worker run（delegation）ごとの実測 usage（token 数・cost・
duration・model）を、CLI が JSONL file に best-effort append で
継続記録する仕組み。`count-tokens` の事前見積も同じ file に記録
し、parent agent が delegation 単位の削減効果を参照・集計できる
形にする。

## Why Now

cheaper の存在意義は parent agent の context・cost 削減だが、
現状は `meta.usage` が per-run の envelope 出力に留まり、蓄積・
集計の仕組みがない（Issue #13）。Issue #12 で実 provider e2e が
整い、実測 usage が deterministic に assert できる経路ができた
ため、記録基盤を最小構成で導入するタイミング。

## Users and Value

- parent agent（consumer）: delegation ごとの token/cost 実績を
  JSONL から読み、`count-tokens` の事前見積と比較して削減効果を
  数値化できる。
- 開発者: real-provider e2e・実運用の両方で usage の実測値が
  追跡可能になり、削減効果の回帰を検出できる。

## Scope

### In Scope

- `run` 系コマンド（`run <worker>`・worker alias）のうち、
  success envelope と `meta.model`・`meta.usage` を持つ error
  envelope（usage-bearing failure）から usage 記録を 1 行
  JSONL で append する（`<userConfigDir>/usage.jsonl`）。
- `count-tokens` の測定結果（path・tokens・bytes）を同じ file に
  記録する（事前見積として実測と突き合わせ可能にする）。記録は
  token count が成功し `tokens` を持つ実行に限る。
- 記録は best-effort: 書き込み失敗が exit code・stdout envelope
  contract を変えない。
- 記録経路の unit test・fake-stream e2e・real-provider e2e での
  実測 assert。

### Out of Scope

- JSONL の集計・summary コマンド（JSONL 自体が parent の読める
  形。集計 UI は follow-up）。
- auth・help・version 等の計測対象外コマンドの記録。
- `meta.model`・`meta.usage` を持たない run failure（worker・
  model 解決前の input/config error 等）や、失敗した
  count-tokens（`FILE_TOO_LARGE`・file not found・workspace
  escape 等）の記録 — いずれも usage/測定結果を持たず記録対象外。
- log rotation・retention（YAGNI。file は 1 行/ run で増え方は
  緩やか）。
- parent agent 側の skill への参照手順組み込み（docs 記載まで）。

## Requirements

1. **記録場所**: `resolveConfigDirs` の `userConfigDir` 配下の
   `usage.jsonl`。`CHEAPER_CONFIG_DIR` で隔離可能とし、consumer
   の workspace・`.cheaper/` は汚さない。
2. **record shape**: `schemaVersion`・`ts`（ISO 8601）・`kind`
   （`"run"` / `"count"`）・`worker`（count は null）・`root`
   （workspace 絶対 path）・`ok`・`durationMs` を必須とし、
   run は `model`（provider/id/tier）・`usage`（
   input/output/cacheRead/cacheWrite/totalTokens/costUsd）・
   error 時は `error.code`、count は `result`（path/tokens/
   bytes）を含む。
3. **best-effort**: log 書き込み失敗（dir 作成不可・disk full
   等）は exit code・stdout を変えず、`--quiet` でなければ
   stderr に 1 行 diagnostic を出すのみとする。
4. **隔離**: test では `CHEAPER_CONFIG_DIR` を tmpdir に向け、
   実 user config に書き込まない。
5. **実測連携**: real-provider e2e（SPEC-034r05iHK8bA7guAnTdpdi）で log file に
   `kind:"run"`・provider `ollama`・正の `usage.input` を持つ
   record が append されることを assert する。
6. **回帰安全**: envelope schema・stdout contract・既存 exit
   code を変えない。Ollama 不在環境でも `mise run check` が
   green のまま。

## Acceptance Criteria

- [ ] `run` のうち success と `meta.model`・`meta.usage` を持つ
      error の実行に限り、`usage.jsonl` へ 1 行の run record が
      append される（usage を持たない failure は記録されない）。
- [ ] `count-tokens` のうち token count が成功し `tokens` を
      持つ実行に限り、count record が append される（
      `FILE_TOO_LARGE` 等の失敗は記録されない）。
- [ ] log 書き込み不可の環境でも CLI の exit code・stdout が
      変わらない。
- [ ] real-provider e2e が log record を assert する。
- [ ] `pnpm typecheck`・`pnpm lint`・`pnpm test`・audit が pass。

## Sources and Relations

- Issue #13: 要件の起点（置き場・記録・実測連携・count_tokens
  整合の 4 指示）。
- SPEC-034r05iHK8bA7guAnTdpdi / ADR-034r05iHrZUE6WjfsVTWjO: real-provider e2e — 実測 assert の足場。
- DESIGN-034r05iHrZUE6WvDuYjZB4: envelope schema（`meta.usage` の canonical 形）。
- `packages/core/src/token/count.ts`: count-tokens の事前見積。

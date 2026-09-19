---
id: "SPEC-034rzGnCaRQc5ceI95lf5i"
type: "spec"
status: "implemented"
title: "Usage and savings telemetry aggregation"
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
  related:
    - "./delegation-usage-measurement-log.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/usage-and-savings-telemetry-for-delegation-reduction.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../adr/usage-jsonl-contract-canonical.md"
    - "../designs/delegation-usage-measurement-log.md"
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Usage and savings telemetry aggregation

## What Should Be Built

delegation の削減効果を 3 区分で記録・集計する仕組み。

- **actual**: worker が実際に消費した token/cost（既存の
  `meta.usage` / run record をそのまま actual として扱う）。
- **measured offload**: worker の tool output（read/grep/find/ls/
  count_tokens が返した text）の canonical token 和
  `sourceTokens`、親へ返した `result` JSON の token 数
  `returnedTokens`、その差 `avoidedContextTokens` を run ごとに
  実測して記録する。
- **estimated**: comparison model（親 model 相当）と pricing
  metadata が利用可能な場合のみ、equivalent cost / savings を
  `estimated*` 名で別枠算出する。

加えて `cheaper usage` 集計コマンドを追加し、期間・worker・
provider・model で filter した集計を出力する。

## Why Now

cheaper の価値は親 agent の context/cost 削減にあるが、現状は
per-run の `meta.usage` と usage.jsonl の記録があるだけで、
「親 context に投入せずに済んだ量」を直接確認する手段がない
（Issue #44）。継続的に価値を検証するため、実測と推定を明確に
分離した telemetry を導入する。

## Users and Value

- parent agent / 利用者: `cheaper usage` で delegation の実績と
  削減量を期間・worker 単位で確認できる。
- 開発者: measured offload の実測値により、worker の圧縮効率や
  回帰を追跡できる。推定値と実測値の混同を schema レベルで防ぐ。

## Scope

### In Scope

- run record（`usage.jsonl`）への `savings` section 追加:
  `sourceTokens`・`returnedTokens`・`avoidedContextTokens`
  （measured）を含む。record は optional field 追加のみで
  `schemaVersion` は据え置き。
- success envelope の `meta` にも同じ measured offload 値を載せ、
  親 agent が run 直後に参照できるようにする。
- tool output の token 計測: sandboxed tool の実行結果 text を
  canonical tokenizer（o200k_base / js-tiktoken）で数えて加算する
  meter を runtime-pi の tool wrap 層に追加する。
- `cheaper usage [--since <duration>] [--worker <name>]
  [--provider <name>] [--model <provider/model>]` 集計コマンド。
  usage.jsonl を read-only で読み、runs・worker token・cost・
  measured offload の合計を出力する。`--format json|text`。
- estimated savings: comparison model の指定・pricing metadata・
  直接比較可能な run のすべてが存在する場合のみ
  `estimatedParentEquivalentCostUsd` / `estimatedSavingsUsd` /
  `estimatedSavingsPercent` を算出・表示する。pricing の
  `source`・`updatedAt`・`stale` を出力に含める。
- `docs/guides/usage.md` に指標の定義・限界（各指標が何を意味し
  何を意味しないか）を明記する。

### Out of Scope

- 親 Coding Agent の全 token 消費の完全な観測。
- 「cheaper が無ければ親が何 turn 使ったか」の再現。
- 推定値を実測値として表示すること。
- pricing metadata の取得・更新機構そのもの（#45 の範囲。
  本 spec は読み取り seam のみ定義する）。
- usage.jsonl の rotation / retention。
- log に記録されない execution path（worker/model 解決前の
  error 等）の計測拡大 — 記録対象 contract（ADR-034r2eId2hXJGWDjBylKVE）は
  変更しない。

## Requirements

1. **measured offload の定義**: `sourceTokens` は run 中に
   sandboxed tool が返した text output の canonical token 和、
   `returnedTokens` は `result` object を JSON 直列化した text の
   canonical token 数、`avoidedContextTokens` は
   `max(0, sourceTokens - returnedTokens)` とする。
   `avoidedContextTokens` は「親が確実に同量を消費した」意味では
   なく「親 context に直接投入されなかった token 数」と定義し、
   文書に明記する。
2. **worker 非依存**: 計測は tool wrap 層で行い全 worker に
   適用する。各 record の `savings.directlyComparable` が
   「親 model との直接比較が成立する workload か」を明示する
   （`write`/`edit` tool または `writeScope` を持たない
   worker のみ true。bulk-reader は true、code-writer /
   doc-writer は false）。estimated percentage は
   `directlyComparable: true` の record のみを母数に算出し、
   該当 run が 0 件なら estimated を出力しない。
3. **後方互換**: usage.jsonl の既存 record field・kind・
   `schemaVersion`・記録対象条件は不変。`savings` は optional
   field の追加のみ。古い record（savings なし）を読む集計は
   欠損を 0 ではなく「計測なし」として扱う。
4. **estimated の分離**: estimated 系 field・出力行は必ず
   `estimated` / `Estimated` を明示する。comparison model 未指定、
   価格不明（runtime catalog・`pricing.json`・bundled snapshot
   のいずれにも無い）、stale、または直接比較可能な run が
   0 件の場合は estimated 値を一切出力しない（捏造しない）。
   pricing 解決は core の `resolvePricing`（#45）を使い、
   `<userConfigDir>/pricing.json` は同じ `PricingSnapshot`
   contract の bridge として読む。
5. **集計 command**: `cheaper usage` は usage.jsonl を読んで
   filter・集計する read-only command。`--since` は `7d` のような
   duration 表記（既存 `parseDuration` 再利用）で `ts` を filter。
   `--worker`・`--provider`・`--model`（`provider/id`）で record
   を絞る。log 不在・0 件でも error にせず 0 集計を表示する。
6. **失敗・部分 run**: `ok:false` の usage-bearing record も
   actual 集計には含める（実際に消費した cost であるため）が、
   runs 数・savings 集計では成功 run と区別して扱う。measured
   offload 値を持たない record は savings 集計の母数に含めない。
7. **回帰安全**: envelope schema は optional field 追加のみで
   既存 consumer を壊さない。記録失敗は従来通り best-effort。
   `mise run check` が Ollama 不在でも green。

## Acceptance Criteria

- [ ] bulk-reader の fake-stream e2e で `meta` または record の
      `savings.sourceTokens` / `returnedTokens` /
      `avoidedContextTokens` が実測値として記録される
      （`avoided = max(0, source - returned)`）。
- [ ] run record に `savings` section が含まれ、既存 record
      field・`schemaVersion`・記録対象条件が不変である。
- [ ] `cheaper usage` が runs・worker token・cost・measured
      offload を集計して出力し、`--since`・`--worker`・
      `--provider`・`--model` で filter できる。
- [ ] comparison model + pricing metadata がある場合のみ
      `Estimated` 行が表示され、価格不明・未指定では表示されない。
- [ ] 出力上、actual / measured / estimated が明確に区別される
      （estimated は `Estimated` 表記を伴う）。
- [ ] `docs/guides/usage.md` に 3 区分の定義と限界が記載される。
- [ ] `pnpm typecheck`・`pnpm lint`・`pnpm test` が pass。

## Deferred Design Concerns

- usage.jsonl の rotation / retention | 現状 1 行/run で増加は
  緩やか | log が運用上肥大化した時点 | 長期運用で file 肥大
- pricing metadata の取得・鮮度管理 | #45 の scope | #45 実装後に
  seam 接続 | seam 不一致で再調整
- per-tool / per-file の offload 内訳 | 集計のみで十分 | 詳細
  分析需要が出た時点 | 現状は合計値のみで詳細な内訳は不明

## Sources and Relations

- Issue #44: 要件の起点（actual/measured/estimated 区分・指標
  候補・CLI 候補・完了条件）。
- DISC-034rzEbmKarhQzw41fhumg: sourceTokens 計測点・record
  schema・estimated 分離の探索結果。
- SPEC-034r05iHK8bA7gveeDA5HC / ADR-034r05iHrZUE6WljM1ZMiX /
  ADR-034r2eId2hXJGWDjBylKVE: usage.jsonl の既存 contract（関連
  chain。本 spec の記録拡張の前提）。
- `packages/cli/src/usage-log.ts`・`packages/runtime-pi/src/tools/`:
  実装対象の現行コード。

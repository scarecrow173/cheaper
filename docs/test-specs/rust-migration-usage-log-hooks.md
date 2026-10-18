---
id: "TSPEC-034BguaDXc4T8pJgjVNbXv"
type: "test-spec"
status: "proposed"
title: "rust-migration-usage-log-hooks"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
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
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# rust-migration-usage-log-hooks

## Purpose

移行前後をまたぐ永続 state(`<userConfigDir>/usage.jsonl`)と、envelope
外の別 contract である `cheaper hook` 経路が、Rust binary でも既存
contract を維持することを保証する意図を記録する。usage.jsonl は新旧
実装が同一 file を読み書きする唯一の交差領域であり、hook は fail-open
破壊が親 agent の全操作を即時 block する高危険経路である。両者の
書式・失敗意味が別 contract に再設計された場合にのみ見直す。

## Feature

usage.jsonl の新旧 reader/writer 互換と、`cheaper hook` 4 target の
byte 一致・fail-open 維持

## Rules

### usage.jsonl 互換

- 記録先は `<userConfigDir>/usage.jsonl` への 1 行 JSONL、best-effort
  append。書き込み失敗は exit code・stdout を変えず、workspace・
  `.cheaper/` には書かない
- record 形は `kind:"run"`(worker・model・usage・durationMs)と
  `kind:"count"`(path・tokens・bytes)を維持し、拡張は additive-only
  (旧 reader が読める key のみ追加。新 reader は `savings`/`turns`/
  `toolCalls` 等の欠落を Option として許容)
- 記録対象は run の成功と `meta.model`・`meta.usage` を持つ error
  (usage-bearing error)、および `tokens` を返した count-tokens の成功に
  限定する。worker/model 解決前の error・失敗した count-tokens・
  auth/help/version 等の utility command は記録しない
- `cheaper usage` は新旧混在 log を読み込み集計できる。破損行は skip
  し、欠落 field を持つ旧 record も除外せず読み込む
- `--since`/`--worker`/`--provider`/`--model` filter と
  `--compare-model` の pricing 解決順(runtime catalog → user
  `<userConfigDir>/pricing.json` → bundled snapshot)・stale 30 日判定・
  `directlyComparable` 母数の意味は TS 版と一致させる
- `meta.usage.costUsd` は Rust 側では pricing 層算出であり、新旧 record
  で口径が混在しうることが `docs/guides/usage.md` に記載される

### hook 経路

- `cheaper hook <event> --target <agent>` の 4 target(windsurf/
  copilot/pretooluse/cursor)について、allow/deny/malformed 入力の
  stdout/stderr/exit code が TS 版と byte 一致する
- hook 経路は runCli の envelope JSON・`recordUsage`・汎用 catch に到達
  しない早期分岐構造を維持し、全 error(args parse 失敗・未知 target・
  policy 破損・cwd 不存在・内部例外・panic)を target 適合の allow +
  exit 0 に変換する(target 判別不能なら無言 exit 0)
- copilot target は絶対に非 0 を返さない。cursor target は常に
  schema-valid JSON(malformed stdin・内部 error でも
  `{"permission":"allow"}`)を emit し、無言 exit 0 を禁止する
- deny 判定の文言形式と routing policy(threshold・byte fast path・
  shell tokenizer + read candidate 抽出・30s 計測 timeout)は TS 版と
  同一 fixture で一致する
- hook 計測は usage.jsonl に記録されない

## Examples

- Given: Rust writer が書いた新 record と `savings`/`turns`/`toolCalls`
  欠落の旧 record が混在する usage.jsonl。Then: Rust `cheaper usage` が
  全行を読み集計し、TS `cheaper usage` が同じ file を読んで同じ集計を
  返す(双方向互換)
- Given: meta を持たない error(worker 名不正・INVALID_REQUEST 等)。
  Then: usage.jsonl に record が追加されない
- Given: usage-bearing error(provider failure 等)。Then: `kind:"run"` の
  record が 1 行 append され、書き込みに失敗しても stdout/exit code は
  変わらない
- Given: 破損した行(非 JSON・途中切断)を含む usage.jsonl。Then: 破損
  行は skip され集計が継続する
- Given: deny 対象 file の windsurf `pre-read-code` payload。Then:
  stderr guidance + exit 2 が TS 版と byte 一致する
- Given: malformed JSON stdin。Then: copilot は allow 方向で exit 0
  (非 0 不返却)、cursor は `{"permission":"allow"}` を emit、未知
  target や panic 注入でも非 0 exit にならない

## Guarantees

- 並走期間中に TS/Rust 両 binary が同一 usage.jsonl を壊さず読み書き
  できる(additive-only 拡張 + Option field 読み)
- 記録範囲(usage-bearing error と成功 run/count-tokens のみ)が維持され、
  utility command や解決前 error で log が汚染されない
- hook 経路は panic を含む全 error を allow/exit 0 へ落とし、deny は
  routing policy の measured deny/FILE_TOO_LARGE のみから生じる
- byte 一致検証は stdout・stderr・exit code の 3 面で行われる

## Non-goals

- `cheaper usage` の新 filter・新集計軸の追加
- usage.jsonl のローテーション・サイズ管理などの運用機能
- routing 閾値・文言・policy ルール自体の再設計(挙動不変が前提)
- 実 agent(copilot/claude/codex/cursor)での発火検証(upstream 依存)
- hook event 種・target adapter の追加

## Risk

- この保証が落ちると、(a) usage.jsonl の破壊的 schema 変更で新旧どちら
  かの `cheaper usage` が読めなくなり移行期の計量が失われる、(b) hook
  の fail-open 欠落(特に copilot の非 0 exit)が親 agent の全 tool 呼出
  しを deny 相当に倒し consumer 環境を即座に停止させる。どちらも配布
  切替後に初めて実害となる regression である

---
id: "TSPEC-034BgtZIn0KAN8vslhM1WI"
type: "test-spec"
status: "proposed"
title: "rust-migration-differential-golden"
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

# rust-migration-differential-golden

## Purpose

TS 実装と Rust 実装が「外部から観測可能な挙動」で一致することを、同一
fixture の replay 比較で機械的に保証する意図を記録する。全面 rewrite
では abort 分類・usage 集計・error 写像のように文書化しきれない corner
の silent regression が生じるため、binary-to-binary の golden 比較が
配布切替(Phase 5)の必須 gate となる。TS 実装が撤去され比較元が消えた
時点で本 spec は retire し、fixture は Rust 側の回帰基盤として再編する。

## Feature

scripted fake provider fixture による TS/Rust binary 間 differential
golden 比較

## Rules

### fake provider seam(3 層注入)

- fake provider は catalog 登録(`cheaper-fake/fake-model` を
  `get_available` 相当に露出)・keyless credential 解決・scripted stream
  の 3 層セットで構成し、いずれか 1 層のみの注入では run が成立しない
  ことを確認する
- seam は dev/test 限定の注入経路であり、production の provider 解決・
  credential 解決の contract を変えない

### fixture DSL

- fixture は「worker 入力 + 応答 item 列 + env」を JSON で記述し、TS 側
  `CHEAPER_TEST_FAKE_STREAM` の応答配列と同じ意味を Rust 側 seam が
  replay できる形に揃える。時刻・乱数は注入で固定し deterministic にする
- DSL は少なくとも応答 item 列に加えて `delay_ms`・`error{status,
  retryable}`・`hang_until_abort`・`abort_response`・`usage_delta` の各
  primitive を表現できなければならない
- case 集合は 20-40 件とし、代表 worker × 正常系 / invalid output→
  repair / abort(timeout・maxTurns・maxToolCalls・SIGINT)/
  PARTIAL_MUTATION / error envelope をカバーする

### 比較規則

- 比較対象は stdout envelope と exit code。両 binary を同一 input/env/
  cwd で実行し、normalize 適用後に厳密一致させる
- normalize 規則は harness 内の単一実装とし、テスト個所や fixture 側へ
  複製しない
- 比較除外(系統的・非決定な差を吸収する範囲): `meta.durationMs`、
  `meta.execution` の時刻系 field(`operationMs`/`activeMs`/`overlapMs`/
  `queueMs`)、`usage.costUsd`(TS=provider 応答値 vs Rust=pricing 層算出
  の系統的不一致)、tokenizer identity(`result.tokenizer.*`・
  `details.tokenizer.*`)、`error.message` 本文(Node `err.message` と
  Rust `io::Error` Display の差異)、stderr 診断文言、JSON キー順、
  schema error の message 本文・件数・code(schema error の契約は
  `instancePath` のみ)
- 厳密比較対象: `usage` の token field(`input`/`output`/cache 系)、
  `turns`・`toolCalls`・`rejectedToolCalls`・`repairs`・
  `transientRetries`・`changes`・`savings`・`result`・`meta.execution`
  の決定的 field(`operations`/`byState`/`byTool`/`maxConcurrency`)・
  `error.code`/`exitCode`・終了 exit code
- 「`meta` 全 field の比較」はこの normalize 規則適用後の意味と解釈する

## Examples

- Given: `error{status:429}` を 1 回挟んで成功する応答列。Then: 両
  binary で `transientRetries` が 1 計上され、usage・`result`・exit
  code が一致する
- Given: `delay_ms` が policy timeout を超える fixture。Then: 両 binary
  が TIMEOUT 分類の error envelope(`code`/`exitCode` 一致)を返す
- Given: `hang_until_abort` + SIGINT 注入。Then: 両 binary が ABORTED 系
  の同一 code を返し、accumulator 由来の usage snapshot が meta に残る
- Given: mutation journal 非空の状態での abort。Then: 両 binary が
  PARTIAL_MUTATION で包んだ error を返す(cause の `code` は一致、
  `error.message` 本文は除外)
- Given: `usage_delta` を含む応答列。Then: `input`/`output`/cache 系
  token field が両 binary で厳密一致する
- Given: schema 非適合の最終出力 → repair 経路。Then: `repairs` 計数と
  `details.errors[]` の `instancePath` が一致する(message 文言・件数は
  比較しない)

## Guarantees

- abort 分類・PARTIAL_MUTATION・`transientRetries`・hook 逐次集計など
  文書化しきれない意味論の silent regression が、normalize 後の `meta`
  厳密比較で検出される
- fixture DSL・normalize 規則が実装より先行して定義され、比較ロジック
  が単一実装に集約される
- TS binary が供給できる間、Rust binary の挙動差分が CI で fail する
  (配布切替の前提条件)

## Non-goals

- 実 provider との差分検証(429 分類・totalTokens 未報告 fallback・
  cache token 差等は実 provider 依存のため CI 外の手動 smoke で扱う)
- performance・実行時間の比較(wall-clock 由来 field は除外対象)
- stderr 文言・`error.message` 本文・schema validator(Ajv vs
  `jsonschema`)の文言互換
- workflow engine 内部の比較(CLI envelope 経由の観測面のみ)

## Risk

- この保証が落ちると「外部 contract 不変の全面 rewrite」という移行前提
  が検証不能になり、abort/usage/会計の silent な乖離が配布切替後に
  初めて表面化する。fixture/規則の後付けは差分を「normalize で吸収する
  側」へ倒す誘因を生み、比較の意味自体が空洞化する


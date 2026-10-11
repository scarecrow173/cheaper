---
id: "SPEC-034DUFyMl47o924O1KW8Ya"
type: "spec"
status: "approved"
title: "Observability foundation scope"
created: "2026-10-11"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/observability-foundation-logging-and-profiling.md"
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
    - "../adr/consolidate-observability-behind-a-dedicated-crate.md"
    - "../adr/logging-output-and-activation-contract.md"
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "../designs/cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  derived-by:
    - "../adr/consolidate-observability-behind-a-dedicated-crate.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Observability foundation scope

## What Should Be Built

observability foundation(logging + profiling)の導入。
DISC-034DUx01YRjOAf4gTaftO7 で確定した決定軸 D1–D9 に従い、
以下を 1 フェーズで実施する:

- 新規 `cheaper-observability` crate の新設 — subscriber init、
  sink 構築(file roller + stderr writer)、format 選択、
  filter/level 配線、activation surface を所有する(ADR:
  consolidate-observability-behind-a-dedicated-crate)。
- 全 crate への tracing instrumentation — cli dispatch・hook・
  tool internals・run pipeline stages・auth/catalog I/O boundary。
  leaf crate は `tracing` crate へ直接依存して span/event を emit
  する(subscriber 未設定時はほぼ zero-cost)。
- dual sink: file(`<cwd>/.cheaper/logs/`、rolling daily/per-startup +
  世代数管理)+ stderr。stdout は envelope 専用のまま
  (hard invariant)。
- format: human-readable text + structured JSON、切替可能。
- activation: env var(`CHEAPER_LOG` = level、`CHEAPER_LOG_FORMAT`)+
  CLI flag(`--log <level>`・`--log-format <text|json>`)の併用、
  flag が env var を override。既定 OFF で ~zero overhead。
- profiling は tracing span による stage timing — run pipeline の
  stage 分解を span tree から得る(CPU sampling ではない)。
- safety: 既定は metadata-only(len/hash/id)。credential・prompt・
  tool payload 本体は記録しない。

## Why Now

Rust + Rig 移行・構造整理で外部 contract が固定された直後であり、
性能・バイナリサイズ最適化を始める前に証拠収集基盤を敷く時期。
現状は構造化 logging が皆無で、run pipeline 内部はブラックボックス、
不具合調査は推測頼み、`benchmarks/run.mjs` は外部 wall-clock のみで
stage 分解の証拠を持たない。instrumentation は全 crate に波及するため、
機能追加再開前に境界と contract を固定する。

## Users and Value

- cheaper のメンテナ / coding agent: run pipeline の stage 別 timing と
  内部状態の証拠が得られ、最適化の対象選定と不具合調査が計測ベースに
  なる。
- consumer: 既定 OFF で無効時のコストはほぼゼロ。envelope・exit code・
  usage.jsonl・hook contract は不変。

## Scope

### In Scope

- `cheaper-observability` crate 新設(subscriber init・sink 構築・
  format 選択・filter/level 配線・activation surface)。
- workspace 全 crate への `tracing` span/event instrumentation:
  cli dispatch・hook・tool internals・run pipeline stages・
  auth/catalog I/O boundary。
- file sink: `<cwd>/.cheaper/logs/` 配下、rolling
  (daily/per-startup)+ 世代数 retention(tracing-appender 型)。
- stderr sink: 既存 `io.stderr` diagnostic 経路(CliIo boundary の
  error line・model line・usage-log line・auth prompt)と共存する
  行単位の出力。
- format 切替: human-readable text / structured JSON。
- activation: `CHEAPER_LOG`(level)+ `CHEAPER_LOG_FORMAT` env var +
  CLI flag `--log`/`--log-format`(flag 優先)。既定 OFF。
- span による stage timing(run pipeline 分解)のための span 配置・
  field 規約(stage 名・id・len/hash の canonical naming と JSON
  field schema)。
- metadata-only redaction 規約と、その regression test。
- 不変条件の維持確認(envelope byte-parity・exit code・usage.jsonl・
  hook contract・`.cheaper/` gitignore 整合・`panic="unwind"`)。

### Out of Scope

- 最適化の実施自体(本フェーズは証拠収集基盤のみ)。
- CPU sampling / flamegraph(pprof 等)、memory profiling。
- stdout への logging 出力(envelope contract 違反のため恒久的に
  対象外)。
- 外部 backend への送出(OTLP exporter 等)。
- raw payload capture の opt-in 機構の実装 — **本フェーズでは構築
  しない(deferred)**。metadata-only が既定かつ唯一の挙動であり、
  opt-in 機構の形(別 env var / file 限定 / stderr 除外等の制約)は
  後続の design で確定する。

## Requirements

- **stdout は envelope 専用(hard invariant)**: `envelope_json` の
  「envelope 1 JSON + 末尾改行 1 つのみ」・`schemaVersion: 1` を
  一切変更しない。logging が stdout に混入する経路を作らない。
- **stderr sink は既存 diagnostics と共存する**: stderr は既に
  `CliIo` boundary(`io.stderr`)経由で diagnostics を運んでいる —
  error line(`cli.rs` の `error {code}: {message}` 書き出し)、
  run 成功時の `model: <prov>/<id> (tier <t>)` line、`usage-log:`
  line、auth prompt、`emit_catalog_warnings` の `warning:` line、
  `--quiet` による suppression(error line は cli.rs:186-188 の
  `suppress` gate、model line は cli.rs:330-332、warning line は
  cli.rs:112-115、`usage-log:` line は `quiet` 引数経由で
  usage_log.rs:196-197 が抑制)。tracing stderr sink はこの
  channel と行単位で interleave する設計
  制約であり、以下を満たす:
  - diagnostics 行の形式・内容・`--quiet` semantics は不変
    (hook contract・consumer 契約の維持)。
  - `--quiet` は diagnostics を抑制するが、flag/env var による
    **明示的な log 有効化は抑制しない**(確定済みルール — test で
    固定する)。
  - hook 経路で stderr log が親 agent との stdin/stdout protocol に
    干渉する場合、hook 経路では stderr sink を無効化する fallback
    を用意する(実装時検証の open item)。
- **依存方向は一方向**: `cli → {workflow, provider} → runtime →
  {rig-adapter, tools} → core` を壊さない。observability は全 crate
  から見て下流(`everything → cheaper-observability →
  tracing-subscriber`)に置き、`cheaper-observability` は
  run/business crate に依存しない。`cheaper-core` は
  rig-free/tokio-free のまま(observability 経由で tokio/rig を
  引き込まない)。
- **既定 OFF で ~zero overhead**: subscriber 未設定時の span コストが
  無視できること。無効化時 overhead の計測方法を定義する
  (benchmarks/run.mjs の off/on 比較等)。
- **metadata-only が不変条件**: credential(api_key/oauth)、
  `system_prompt`/`prompt`、tool payload 本体を field 値として記録
  しない。記録するのは len/hash/id 等の metadata。redaction 規約の
  網羅 test を持つ。
- **`.cheaper/` 規約との整合**: log 出力先は `<cwd>/.cheaper/logs/`。
  `.gitignore` の bare `logs` pattern が cover することを確認し、
  不足があれば追従する。
- **`panic = "unwind"` を維持**: observability の導入で panic 戦略・
  panic hook 経路を変えない。
- 実施は意味のある境界で分割コミット(Conventional Commits)。

## Acceptance Criteria

- [ ] log 無効(既定)で `cheaper-cli` regression suite(55 cases)が
  無変更で pass し、envelope byte-parity・exit code・hook 挙動が
  維持される。
- [ ] `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` または
  `--log`/`--log-format` で有効化した run が、
  file(`<cwd>/.cheaper/logs/`)と stderr の両 sink に run pipeline
  の stage timing を含む event を出力し、JSON format が機械
  parse 可能である。
- [ ] CLI flag(`--log`/`--log-format`)が env var
  (`CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`)を level・format の双方で
  override することを示す test がある。
- [ ] human-readable text format が両 sink で実際に出力されることを
  示す test がある(JSON parse 可能性だけでなく text 経路も検証)。
- [ ] run pipeline 以外の計測点 — hook 経路・tool internals・
  auth/catalog I/O boundary — が、log 有効時にそれぞれ観測可能な
  event を出力する。
- [ ] credential・prompt・tool payload 本体が log(file/stderr 双方)
  に出現しないことを示す regression test がある。
- [ ] log 無効時 overhead の計測方法が定義され、計測結果が許容範囲
  として記録される。
- [ ] `--quiet` + log 有効化の組合せで diagnostics は抑制され、
  flag/env var による明示的な log は出力される(確定済みルールを
  test で固定)。
- [ ] `.cheaper/logs/` が `.gitignore` の `logs` pattern で cover
  されることを確認済み。
- [ ] hook 経路で stderr sink が hook contract に干渉しないことを
  検証済み(干渉する場合は stderr sink 無効化 fallback が動作)。
- [ ] `mise run check` が全工程 green。

## Deferred Design Concerns

- raw payload capture の opt-in 機構 | 既定の metadata-only で調査力が
  足りないケースが観測されたとき再検討 | 設計前に credential 流出経路を
  作るリスクを冒さないため意図的に deferred。
- CPU sampling / flamegraph・memory profiling | 最適化フェーズで
  関数粒度の証拠が必要になった時点 | stage timing で bottleneck が
  十分絞れる間は不要。
- 外部 backend(OTLP 等)への送出 | 集約・長期保管の要求が生じた時点 |
  `cheaper-observability` 境界に吸収できる設計のため後付け可能。
- rolling の既定値(daily vs per-startup・保持世代数・file 命名)、
  span/field の canonical naming、JSON field schema — 実装 design で
  確定する open item。

## Sources and Relations

- 上流: DISC-034DUx01YRjOAf4gTaftO7(決定軸 D1–D9 の確定記録、
  inventory findings、open questions)。
- crate topology の決定: ADR consolidate-observability-behind-a-dedicated-crate。
- 出力・activation contract の決定: ADR
  logging-output-and-activation-contract。
- 関連不変条件: DESIGN cli-contract-envelope-exit-codes、ADR
  gate-dev-test-seams-behind-a-dev-seam-cargo-feature(activation
  機構の feature-gate 前例)。

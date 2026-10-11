---
id: "ADR-034DUFzrXJxOwfl1qveXWR"
type: "adr"
status: "accepted"
title: "Consolidate observability behind a dedicated crate"
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
    - "../specs/observability-foundation-scope.md"
    - "./logging-output-and-activation-contract.md"
    - "./gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/observability-foundation-scope.md"
  derived-by:
    - "../designs/observability-foundation-logging-and-instrumentation-wiring.md"
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

# Consolidate observability behind a dedicated crate

## Context and Problem Statement

observability foundation(logging + profiling)の導入で、tracing
infrastructure の所有境界を決める必要がある(DISCO の D8)。

- 現状 workspace の全 crate に `tracing`/`log` 系依存は 0 件で、
  観測のための出力経路は production 経路に存在しない。
- sink 構築(file roller + stderr writer)・format 選択
  (text/JSON)・filter/level 配線・activation surface(env var +
  CLI flag の解釈)は横断関心事であり、置き場が未定義だと各 crate
  に分散する。
- 依存方向は一方向(`cli → {workflow, provider} → runtime →
  {rig-adapter, tools} → core`)であり、新 crate はこれを破壊しない
  位置に置く必要がある。
- `cheaper-core` は同期純ロジックで rig-free/tokio-free — sink 所有の
  ような I/O 責務を持ち込めない。

## Decision Drivers

* sink init/format/routing の単一の所有境界 — backend 切替・sink 追加を
  1 箇所に吸収する
* leaf crate は標準 instrumentation API(`tracing` の span/event
  macro)を直接使い、依存を薄く保つ — subscriber 未設定時はほぼ
  zero-cost
* 依存方向の一方向性を維持: 全 crate → observability →
  tracing-subscriber。observability が run/business crate に依存
  してはいけない
* `cheaper-core` の rig-free/tokio-free 不変条件を壊さない
  (observability 経由で tokio/rig が core に流入しないこと)
* activation surface(env var/flag の解釈・優先順位)を cli が
  再発明しないこと

## Considered Options

option label は discovery(D8)から振り直している — discovery T-B
(`cheaper-core` module 案)は棄却済みで本 ADR の options には含めず
(More Information 参照)、本 ADR の T-B = discovery T-C、T-C は ADR
で追加した facade 案。

* T-A: 各 crate が `tracing` を直接依存 + init・sink 構築は
  `cheaper-cli` が行う(専用 crate なし)
* T-B: 新規 `cheaper-observability` crate が subscriber init・sink
  構築・format 選択・filter/level 配線・activation surface を所有。
  leaf crate は `tracing` crate へ直接依存して span/event を emit
* T-C: `cheaper-observability` が span macro 含む全 instrumentation
  を wrap する facade を提供し、leaf crate は tracing を直接触らない

## Decision Outcome

Chosen option: "T-B: 新規 `cheaper-observability` crate"、because
sink init/format/routing の方針を 1 境界に集約でき、将来の backend
切替・sink 追加を leaf crate へ波及させずに吸収できる唯一の構成だから。
leaf crate が `tracing` を直接使うのは標準 API であり、subscriber 未
設定時はほぼ zero-cost のため facade(T-C)で隠す実益がない。

### Consequences

* Good, because sink 追加・backend 切替・format 規約・redaction 規則
  の変更が `cheaper-observability` の 1 crate に閉じる。
* Good, because leaf crate の instrumentation は標準 `tracing` API
  のままで、ecosystem の知識・tooling がそのまま使える。
* Good, because cli・runtime・tools 等は emit 側の薄い依存
  (`tracing` のみ)を持つだけで、subscriber の存在を知らなくてよい。
* Bad, because workspace に crate が 1 つ増え、全 crate の
  Cargo.toml への `tracing` 依存追加と cli の init 配線という初期
  コストが発生する。
* Bad, because format・field 規約の二重検証(text/JSON)と redaction
  規則の網羅 test を `cheaper-observability` 側で持つ必要がある。

### Confirmation

`SPEC-034DUFyMl47o924O1KW8Ya` の acceptance criteria により検証される
(log 有効時に両 sink へ stage timing event・secrets 非記録 test・
`mise run check` green)。依存方向は `cargo tree` で構造的に確認する。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-observability/` — 新規 crate(subscriber init・
    file roller + stderr writer の sink 構築・format 選択・
    filter/level 配線・activation surface)
  - root `Cargo.toml` — workspace members へ追加
  - `crates/cheaper-cli/Cargo.toml` — `cheaper-observability` 依存
    追加、dispatch 入口で init 呼出し
  - 全 leaf crate の `Cargo.toml` — `tracing` 依存追加
    (cli・runtime・tools・workflow・provider・rig-adapter・core)
* Patterns to follow:
  - leaf crate は `tracing` crate へ直接依存し、`tracing::instrument`
    / `span!` / `event!` で emit する。observability crate への依存は
    leaf crate 側に不要 — emit は subscriber の有無を意識しない。
  - `cheaper-observability` の公開面は init 関数(activation 解釈済
    config を受け取り subscriber + guard を返す)程度の薄さに留める。
  - activation surface(env var 名・flag 名・優先順位の解釈)は
    `cheaper-observability` が所有し、cli は flag 値を渡すだけ。
    activation 機構の設計参照として dev-seam cargo feature の
    gate 前例(ADR-034DyGyITd0YaATg8fkld7)を使う。
* Patterns to avoid:
  - `cheaper-observability` が run/business crate(runtime・tools 等)
    に依存すること — 依存は `everything → observability →
    tracing-subscriber` の一方向。
  - observability facade が span macro まで wrap すること(T-C)—
    全 crate が facade に結合し標準 API を隠すだけで実益がない。
  - `cheaper-core` が sink 構築や tokio/rig を含む依存を持つこと —
    core は同期純ロジックのまま(`tracing` emit のみ可)。
  - init/routing 方針を leaf crate や cli command 個所へ分散する
    こと(T-A の失敗形)。
* Dependencies or configuration:
  - `tracing`・`tracing-subscriber`・`tracing-appender`
    (rolling file)を `cheaper-observability` が保持。
  - env var `CHEAPER_LOG`(level)・`CHEAPER_LOG_FORMAT` + CLI flag
    の contract は ADR logging-output-and-activation-contract 側で
    確定。

## Verification

* [ ] `cargo tree -p cheaper-cli` で `cheaper-observability` が
  sink/init 側の依存として現れ、`cheaper-observability` から
  run/business crate への依存が存在しない。
* [ ] `cargo tree -p cheaper-core` に `tokio`/`rig` 系依存が現れない
  (core の rig-free/tokio-free 維持)。
* [ ] `cargo build --workspace`・`mise run check` が green。
* [ ] log 有効時に `.cheaper/logs/` file sink と stderr sink の双方へ
  event が出力される。

## Pros and Cons

### T-A: per-crate tracing + init in cheaper-cli

* Good, because 新 crate を作らず最小の構成変更で始められる。
* Bad, because sink 構築・format・routing 方針が cli に集中し、
  backend 切替・sink 追加のたびに cli の init 箇所と依存配線を
  触る。規約の単一の正がない。

### T-B: 新規 `cheaper-observability` crate

* Good, because init/sink/format/activation を 1 境界に集約でき、
  将来の変更を leaf crate へ波及させない。
* Bad, because crate が 1 つ増え、全 crate への `tracing` 依存追加と
  init 配線の初期コストがかかる。

### T-C: instrumentation 全体を wrap する facade

* Good, because instrumentation 記法まで一元化できる。
* Bad, because 全 crate が facade に結合して標準 `tracing` API が
  隠れ、span 配置の自由度と ecosystem 互換性を失う。ゼロコスト
  性質上 facade の実益が薄い。

## More Information

* 上流: DISC-034DUx01YRjOAf4gTaftO7(D8 の確定記録)。
* 関連 spec: SPEC-034DUFyMl47o924O1KW8Ya(実施スコープ)。
* `cheaper-core` に observability module を置く案(DISCO の T-B)は、
  core が同期純ロジック(tokio 境界の内側)で sink 所有が責務違反
  のため棄却済み。
* activation の feature-gate 前例: ADR-034DyGyITd0YaATg8fkld7
  (dev-seam cargo feature)。

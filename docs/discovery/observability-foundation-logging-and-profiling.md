---
id: "DISC-034DUx01YRjOAf4gTaftO7"
type: "discovery"
status: "draft"
title: "Observability foundation for logging and profiling"
created: "2026-10-11"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "crates/cheaper-cli/src/envelope.rs"
    - "crates/cheaper-cli/src/main.rs"
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-runtime/src/auth/store.rs"
    - "benchmarks/run.mjs"
    - "docs/guides/configuration.md"
    - ".gitignore"
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
    - "./post-migration-structural-inventory-and-cleanup-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "../designs/cli-contract-envelope-exit-codes.md"
    - "../designs/delegation-usage-measurement-log.md"
    - "./usage-and-savings-telemetry-for-delegation-reduction.md"
    - "./rust-rig-native-runtime-migration-for-cheaper-cli.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/observability-foundation-scope.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Observability foundation for logging and profiling

## Exploration Goal

Rust + Rig 移行および構造整理(PR #93、上流は
post-migration-structural-inventory-and-cleanup-policy)完了後の次フェーズ
として、observability foundation(logging + profiling)の導入方針を確定する
ための探索。前フェーズの discovery で「ロギング/プロファイリング環境は
別文書で後続」と明示的に defer されていた項目の受け皿。

課題フレーミング: observability が皆無であることは 2 つの構造的問題を
生んでいる — (1) 最適化(性能・バイナリサイズ)に使う証拠チャネルが存在せず、
bottleneck が計測不能。(2) 不具合調査が推測頼みで、run pipeline 内部は
呼び出し側からブラックボックス。

deep-dive で確定済みの決定軸(再審議しない):

- logging + profiling を **1 フェーズ**で導入する
- consumer は **human-readable + structured JSON の両対応** — format 切替は
  設計要件
- profiling は **tracing span による内部 stage timing**(CPU sampling /
  flamegraph は本フェーズ外、後続候補)
- **全 crate 網羅的カバレッジ**(cli dispatch・hook・tool internals・
  run pipeline stages・auth/catalog I/O boundary)
- 有効化は **env var(`CHEAPER_LOG` level + `CHEAPER_LOG_FORMAT` style)+
  CLI flag の併用、flag が優先**
- sink は **file(`<cwd>/.cheaper/logs/`)+ stderr の dual routing**。
  stdout は envelope 専用のまま(hard contract)
- retention は **rolling(daily/per-startup)+ 世代数管理**
  (tracing-appender 型)
- **新規 `cheaper-observability` crate** が sink init/format/routing を
  所有し、他 crate は thin span の emit のみ
- 既定は **metadata-only**(len/hash/id)。credential・prompt・tool payload
  本体は記録しない。raw payload capture は別の明示 opt-in 機構

なぜ今か: 移行・構造整理で外部 contract が固定された直後であり、
最適化を始める前に証拠収集基盤を敷く時期。instrumentation は全 crate に
波及するため、機能追加が再開する前に境界と contract を固定する。

## Context and Constraints

- **stdout は envelope 専用(hard contract)**: `envelope.rs` の
  `envelope_json` は「envelope 1 JSON + 末尾改行 1 つのみ」の stdout 契約で、
  `schemaVersion: 1` 固定・byte-parity regression
  (`crates/cheaper-cli/tests/envelope.rs`)が存在。logging 出力の stdout
  混入は contract 違反であり、全 sink 設計の前提条件。
- **`.cheaper/` は project-level config dir**: `<cwd>/.cheaper/`(`--cwd` で
  変更可)に workers/・policy.yaml・models.yaml を置く既存規約
  (docs/guides/configuration.md)。log の置き場もこの配下が自然で、
  `.gitignore` の bare `logs` pattern が `.cheaper/logs/` を既に cover。
- **secret は run 経路を流れる**: auth credential(`api_key`/`oauth`、
  `cheaper-runtime/src/auth/store.rs`)と user prompt
  (`system_prompt`/`prompt`、`cheaper-runtime/src/run/mod.rs`)が
  instrumentation 対象経路を通る。既定で本体を記録しない redaction 規則が
  不変条件。
- **依存方向は一方向**: `cli → {workflow, provider} → runtime →
  {rig-adapter, tools} → core`。新 crate はこの一方向を破壊しない位置に
  置く。
- **内部不変条件**: Rig 型は `cheaper-rig-adapter` と `cheaper-provider`
  のみ。core は同期純ロジック(tokio 境界の内側)。`panic = "unwind"`。
- **dev-seam の feature-gate 前例**: `cheaper-runtime`/`cheaper-cli` に
  `dev-seam` cargo feature の gate 実績があり、有効化機構の設計参照になる。
- **behavior-preserving でない点に注意**: observability の追加自体は
  外部 contract を変えないが、instrumentation の overhead は benchmark
  値に影響し得る — off 時のコストを測定可能にしておく。

## Inventory Findings

### observability の欠如(confirmed)

- **構造化 logging が皆無**: 全 crate の Cargo.toml に `tracing`/`log` 系の
  依存宣言 0 件。production 経路の stderr 出力は panic hook
  (`cheaper-cli/src/main.rs` の `eprintln!("cheaper internal panic: ...")`)
  1 箇所のみ。残りの `println!`/`eprintln!` は test code
  (`core::workspace::paths`、`tools::walker` の symlink skip 通知)と
  test fixture binary(`cheaper-tools/src/bin/diff_fixture_child.rs`)に
  限定される。すなわち「観測のための出力経路」は production 経路に存在しない。
- **debug/verbose 機構なし**: 既存 env var は config/test 用途のみ
  (`CHEAPER_CONFIG_DIR`、`CHEAPER_TEST_FAKE_STREAM` 等)。
- **stdout は result envelope のみ**: 上記 Constraints の通り。
- **`usage.jsonl` は observability ではない**: `<userConfigDir>/usage.jsonl`
  は delegation accounting(run の worker/model/usage/durationMs、
  count-tokens の tokens/bytes)であり、工程内訳・内部状態・debug 情報を
  持たない。目的が異なり、本フェーズの代替にはならない
  (usage-and-savings-telemetry-for-delegation-reduction 参照)。
- **`benchmarks/run.mjs` は外部 wall-clock 計測のみ**: process 起動から
  終了までの外側時間で、run pipeline 内部の stage 分解を持たない。
  最適化の証拠としては内部計測が必要。

### instrumentation 対象の地形(confirmed)

- workspace は 7 crate: `cheaper-cli`(dispatch・hook・envelope)、
  `cheaper-runtime`(run pipeline・auth・catalog)、`cheaper-tools`
  (tool 実装 + runtime spine)、`cheaper-workflow`、`cheaper-provider`、
  `cheaper-rig-adapter`、`cheaper-core`(同期純ロジック)。
- hook 経路は stdin/stdout が親 agent と接続されるため、stderr 出力が
  hook contract と干渉しないか検証が必要(Open Questions 参照)。

## Alternatives and Comparison

### D1: フェーズスコープ — logging + profiling を同時か分割か

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| L-A: logging のみ先行 | 差分が小さい | profiling 追加時に span 配線を二度実施。最適化の証拠確保が後れる | no |
| **L-B: logging + profiling を 1 フェーズ** | tracing span が logging と profiling を兼ね、1 回の instrumentation で両方を得られる | 単一フェーズの差分は大きめ | **yes(採用)** |
| L-C: profiling のみ | 最小差分 | 調査用の event 記録が残らず debugging は改善しない | no |

### D2: consumer / output format

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| F-A: human-readable のみ | 実装最小 | CI・機械処理での parse が不安定。benchmark 連携に不向き | no |
| F-B: structured JSON のみ | 機械処理に最適 | 人間の ad-hoc 調査で読みにくい | no |
| **F-C: 両対応 + format 切替** | 用途別に選択可。切替機構自体が確定済の設計要件 | formatter の分岐と field 規約の二重検証が必要 | **yes(採用)** |

### D3: profiling 形態

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| P-A: CPU sampling / flamegraph(pprof 等) | 関数粒度の正確な hotspot | 別 backend・別出力経路が必要。envelope 経路との統合が遠い | no(後続候補) |
| **P-B: tracing span による内部 stage timing** | logging と同一基盤。span tree で run pipeline の stage 分解が自然に得られる | 手動 instrumentation で網羅性が配置品質に依存 | **yes(採用)** |
| P-C: 外部 wall-clock のみ(現状維持) | 変更ゼロ | 内部 stage が分解できず最適化の証拠にならない | no |

### D4: カバレッジ

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| C-A: run pipeline のみ | 差分最小 | cli dispatch・hook・tool internals が盲点のまま | no |
| **C-B: 全 crate 網羅**(cli dispatch・hook・tool internals・run pipeline stages・auth/catalog I/O boundary) | end-to-end の stage 分解と調査が成立 | instrumentation 差分が広い | **yes(採用)** |
| C-C: 既知ホットスポットのみ | 局所的 | 「どこが盲点か」自体が未計測で選定根拠がない | no |

### D5: 有効化経路

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A-A: env var のみ | hook・子プロセス経路でも伝播する | 1 回限りの有効化が冗長。shell 環境に依存 | no |
| A-B: CLI flag のみ | 呼び出しごとに明示的 | hook 経由等 flag を通せない経路で有効化不能 | no |
| **A-C: env var + CLI flag(flag 優先)** | 両経路を cover。優先順位が明確で dev-seam 前例と整合 | 優先ルールと名前方針の文書化が必要 | **yes(採用)** |

### D6: sink 構成

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| S-A: stderr のみ | 実装最小 | 永続化されず事後調査・CI での証跡確保が不可 | no |
| S-B: file のみ | 永続化される | 実行中のリアルタイム確認ができない | no |
| **S-C: file(`<cwd>/.cheaper/logs/`)+ stderr dual routing** | 即時確認と永続化を両立。stdout は envelope 専用のまま維持 | sink ごとの init・format 切替配線が必要 | **yes(採用)** |

### D7: file retention

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| R-A: 単一 file 無制限追記 | 単純 | rotate なしで肥大。disk 消費が予測不能 | no |
| **R-B: rolling(daily/per-startup)+ 世代数管理(tracing-appender 型)** | サイズ・世代上限が予測可能。実績ある crate 機構に寄せられる | 間隔・世代数の既定値決定が必要 | **yes(採用)** |
| R-C: 起動ごとに truncate | 単純 | 直前 run の証拠を失い、調査用途に耐えない | no |

### D8: crate topology — observability の所有境界

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| T-A: 各 crate が tracing を直接依存 + cli が init | 新 crate 不要 | init/format/routing 方針が各 crate に分散。backend 切替・sink 追加時に全 crate へ波及 | no |
| T-B: `cheaper-core` に observability module | 新 crate 不要。依存の最下流に置ける | core は同期純ロジック(tokio 境界の内側) — file/stderr sink 所有は責務違反 | no |
| **T-C: 新規 `cheaper-observability` crate が sink init/format/routing を所有。他 crate は thin span の emit のみ** | backend 切替・sink 追加を 1 境界に吸収。format 規約・redaction 規則の単一の正にできる | crate が 1 つ増える。依存配線の初期コスト | **yes(採用)** |

### D9: safety / redaction

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| SEC-A: payload 本体をそのまま記録 | 調査力最大 | credential・prompt が disk に残り secrets 不変条件違反 | no |
| **SEC-B: metadata-only 既定(len/hash/id)。credential・prompt・tool payload 本体は不記録。raw payload capture は別の明示 opt-in 機構** | 既定で安全 + 必要時のみ深堀り可能。opt-in の責任境界が明確 | redaction 規則の網羅 test と opt-in 機構の設計が必要 | **yes(採用)** |
| SEC-C: field allowlist のみ | 最も保守的 | 「何を記録してよいか」の判断が instrumentation ごとに必要で debug 力が不安定 | no |

## Tentative Conclusions and Hypotheses

- **H1(confirmed)**: 決定軸 D1–D9 は deep-dive で全て確定済み。残る論点は
  実装詳細(命名・既定値・opt-in 機構の形)のみで、spec/ADR 化時に詰める。
- **H2(confirmed)**: envelope stdout contract が全設計の hard constraint。
  「envelope 1 JSON + 末尾改行 1 つのみ」を壊さないことが sink 設計・
  format 設計・panic 経路設計すべての前提。
- **H3(hypothesis)**: backend は tracing ecosystem(tracing +
  tracing-subscriber + tracing-appender)が事実上一択。span による
  stage timing・dual sink・format 切替・rolling file の要件を 1 系統で
  満たせる。`log`/`slog` 系を選ぶ理由は薄い。
- **H4(hypothesis)**: 実施順序は「`cheaper-observability` crate 新設
  (sink init・format・routing)→ cli init 配線 → span instrumentation を
  依存方向に沿って波及(cli → runtime → tools/workflow)→ redaction
  規則と opt-in 機構」が妥当。redaction は instrumentation と同時進行で
  よいが、acceptance test は必須。

## Open Questions

- CLI flag の正確な名前(`--log` / `--log-level` / `--verbose` 系)と
  env var 名の最終形(`CHEAPER_LOG`・`CHEAPER_LOG_FORMAT` は仮称) — spec で確定。
- rolling の既定値(daily vs per-startup、保持世代数、file 命名規則) — spec で確定。
- raw payload capture の opt-in 機構の形(別 env var / 別 flag /
  file 限定にするか、opt-in 時も stderr を除外するか等の制約) — ADR/spec で確定。
- span 命名・field 規則(stage 名・id・len/hash の canonical naming)と
  JSON format の field schema — spec で確定。
- leaf crate が `tracing` を直接依存して span を emit するか、
  `cheaper-observability` 経由の薄い facade とするか — spec/ADR で確定。
- hook 経路で stderr 出力が親 agent との hook contract(stdin/stdout
  protocol)に干渉しないか — 実装時に検証。干渉する場合は hook 経路の
  stderr sink を無効化する fallback が要る。
- instrumentation off 時の overhead(無効化 span のコスト)を benchmark
  で許容範囲とできるか — 実装時に計測。

## Promotion Candidates

- [x] spec-doc needed: observability foundation の実施スコープ — D1–D9 の
  採用方針、対象 crate 一覧、span/field 規則、activation 経路、
  acceptance criteria(envelope byte-parity 維持・`mise run check` green・
  credential/prompt 非記録の regression test・off 時 overhead 計測)。
- [x] adr-doc needed: (1) `cheaper-observability` crate topology(D8:
  新 crate 所有 vs core module vs per-crate 直接依存)、(2) logging
  output contract(D6: file + stderr dual sink と stdout 不変の保証)、
  (3) secret/payload redaction policy(D9: metadata-only 既定 +
  raw capture opt-in の境界)。
- [ ] design-doc: span 配線・sink 構成の実装設計が必要な粒度になった場合のみ。

## Sources and Relations

- 現状確認: `grep -rn "tracing\|log" crates/*/Cargo.toml`(依存 0 件)、
  `grep -rn "eprintln!\|println!" crates/*/src`(panic hook + test/fixture のみ)、
  `crates/cheaper-cli/src/envelope.rs`(envelope_json の stdout 契約)、
  `benchmarks/run.mjs`(wall-clock のみ)。2026-10-11 実施。
- `.cheaper/` 規約: `docs/guides/configuration.md`、`.gitignore` の
  bare `logs` pattern。
- secret の流路: `crates/cheaper-runtime/src/auth/store.rs`
  (api_key/oauth credential)、`crates/cheaper-runtime/src/run/mod.rs`
  (system_prompt/prompt)。
- 上流: post-migration-structural-inventory-and-cleanup-policy
  (「ロギング/プロファイリング環境は別文書で後続」の defer 元)、
  rust-rig-native-runtime-migration-for-cheaper-cli(移行本体)。
- 参照実績: gate-dev-test-seams-behind-a-dev-seam-cargo-feature
  (feature-gate 前例)、cli-contract-envelope-exit-codes(envelope
  contract)、delegation-usage-measurement-log(usage.jsonl の責務範囲)。
- 決定軸の確定経緯: 本セッションの deep-dive(D1–D9 全て確定、
  再審議しない)。

---
id: "ADR-034DUFA70cQdAyG7Lbuuay"
type: "adr"
status: "accepted"
title: "Logging output and activation contract"
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
    - "./consolidate-observability-behind-a-dedicated-crate.md"
    - "../designs/cli-contract-envelope-exit-codes.md"
    - "../designs/observability-foundation-logging-and-instrumentation-wiring.md"
    - "./auth-terminal-prompt-io-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
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

# Logging output and activation contract

## Context and Problem Statement

observability foundation の導入で、log の出力先(sink)・format・
有効化経路の contract を確定する必要がある(DISCO の D2・D5・D6・
D7・D9)。

前提となる制約:

- **stdout は envelope 専用(hard contract)**: `envelope_json` は
  「envelope 1 JSON + 末尾改行 1 つのみ」で `schemaVersion: 1` 固定、
  byte-parity regression test 済み。logging の stdout 混入は contract
  違反。
- **stderr は既に diagnostics channel**: `CliIo` boundary の
  `io.stderr` が error line(`error {code}: {message}`)、run 成功時の
  `model: <prov>/<id> (tier <t>)` line、`usage-log:` line、auth
  prompt、`warning:` line を運び、`--quiet` で suppression される
  (error line は cli.rs:186-188 の `suppress` gate、model line は
  cli.rs:330-332、warning line は cli.rs:112-115、`usage-log:`
  line は `quiet` 引数経由で usage_log.rs:196-197)。tracing
  stderr sink はこの channel と共存する必要がある。
- **secret は run 経路を流れる**: auth credential(api_key/oauth)と
  user prompt(system_prompt/prompt)、tool payload が
  instrumentation 対象経路を通る。
- **hook 経路**: stdin/stdout が親 agent と接続されるため、stderr
  出力が hook contract と干渉しないか実装時検証が要る。

## Decision Drivers

* stdout envelope contract(`schemaVersion:1` byte-parity)を
  絶対に壊さない
* 実行中のリアルタイム確認と事後調査・CI での証跡確保の両立
* project-scope の診断(`.cheaper/` は既存の project-level config
  dir で `.gitignore` の `logs` pattern が `.cheaper/logs/` を
  cover)に適した置き場
* hook・子プロセス経路でも伝播する有効化経路 + 呼び出しごとの明示的
  な有効化経路の両立
* 既定 OFF で ~zero overhead
* credential・prompt・tool payload 本体が disk/stderr に残らない
  既定の安全性

## Considered Options

* sink 構成: S-A stderr のみ / S-B file のみ / S-C file +
  stderr dual routing
* file 置き場: `.cheaper/logs/` / userConfigDir / 明示 path 指定
* retention: R-A 単一 file 無制限追記 / R-B rolling(daily/per-startup)
  + 世代数管理 / R-C 起動ごと truncate
* activation: A-A env var のみ / A-B CLI flag のみ / A-C env var +
  CLI flag(flag 優先)
* format: F-A human-readable のみ / F-B structured JSON のみ /
  F-C 両対応 + 切替
* safety: SEC-A payload 本体記録 / SEC-B metadata-only 既定 +
  raw capture は別 opt-in / SEC-C field allowlist のみ
* stdout logging(stdout へ log を流す) — contract 違反のため検討対象外

## Decision Outcome

Chosen options:

- **S-C: file(`<cwd>/.cheaper/logs/`)+ stderr dual routing** — 即時確認と
  永続化を両立し、stdout は envelope 専用のまま維持する。
- **file 置き場は `<cwd>/.cheaper/logs/`** — project-scope diagnosis の
  ため project config dir 配下が自然で、`.gitignore` の `logs`
  pattern が既に cover。userConfigDir ではなく project 側に置く。
- **R-B: rolling(daily/per-startup)+ 世代数管理(tracing-appender
  型)** — サイズ・世代上限が予測可能。
- **A-C: env var(`CHEAPER_LOG` = level、`CHEAPER_LOG_FORMAT`)+ CLI
  flag(flag 優先)** — hook 経路等 flag を通せない経路を env var が
  cover し、flag が per-invocation の明示 override になる。既定 OFF。
- **F-C: human-readable text + structured JSON の両対応・切替** —
  用途別に選択可能。
- **SEC-B: metadata-only 既定(len/hash/id)、credential・prompt・
  tool payload 本体は不記録。raw payload capture は別の明示 opt-in
  機構** — opt-in 機構は本フェーズでは構築せず deferred(spec の
  Out of Scope 参照)。

stderr 共存ルール: tracing stderr sink は既存 `io.stderr`
diagnostics と行単位で interleave する。diagnostics 行の形式・内容・
`--quiet` semantics は不変。`--quiet` は diagnostics を抑制するが、
flag/env var による明示的な log 有効化は抑制しない(確定済み
ルール — test で固定)。hook 経路で stderr log が hook contract
に干渉する場合は hook 経路の stderr sink を無効化する fallback を
用意する(実装時検証の open item)。

### Consequences

* Good, because 実行中に stderr で stage timing を追え、事後調査・CI
  では `.cheaper/logs/` の file 証跡を使える。
* Good, because stdout には一切触れず、envelope byte-parity・
  hook contract・usage.jsonl が構造的に守られる。
* Good, because `.cheaper/` 既存規約に乗るため gitignore・consumer
  向け説明が増えない。
* Bad, because stderr が diagnostics と log の 2 系統を運び、行
  interleave・`--quiet` と log level の関係・hook 経路の扱いという
  設計項目が増える。
* Bad, because sink ごとの init・format 切替配線と、rolling の既定値
  (daily vs per-startup・世代数・命名)の決定が必要になる。

### Confirmation

`SPEC-034DUFyMl47o924O1KW8Ya` の acceptance criteria により検証される
(無効時 regression 55 pass・両 sink への stage timing event・
secrets 非記録 test・`--quiet` と log 有効化の組合せ挙動・
`.gitignore` cover 確認)。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-observability/` — dual sink 構築(file roller +
    stderr writer)・format 選択・filter/level 配線・activation 解釈
  - `crates/cheaper-cli/src/cli.rs` — flag parse・`io.stderr` との
    共存ポイント・`--quiet` 適用範囲
  - `.gitignore` — `.cheaper/logs/` の cover 確認・必要時追従
* Patterns to follow:
  - stderr sink writer は `CliIo`/`CliWrite` boundary と同じ行単位
    書き出し経路に乗せ、diagnostics と混線しない 1 行 = 1 event に
    する(JSON format でも 1 line JSON)。
  - activation 解釈(env var 値の parse・flag 優先・OFF 判定)は
    `cheaper-observability` 側に集約し、cli は flag 値を渡すだけ。
  - `--quiet` は diagnostics(`io.stderr` 経由の error/model/
    usage-log/warning line)のみ抑制し、明示的 log 有効化は抑制
    しない(確定済みルール — test 化する)。
* Patterns to avoid:
  - stdout への log 出力 — envelope contract 違反。
  - diagnostics 行の形式変更や `--quiet` semantics の拡大解釈 —
    hook contract・consumer 契約を壊す。
  - file 置き場を userConfigDir や明示 path にすること —
    project-scope diagnosis の置き場として `.cheaper/` が既定。
  - 無制限追記・起動ごと truncate — retention が予測不能または
    直前 run の証拠を失う。
* Dependencies or configuration:
  - env var: `CHEAPER_LOG`(level)、`CHEAPER_LOG_FORMAT`(text|json)。
  - CLI flag: `--log <level>`・`--log-format <text|json>`(env var
    名 `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` に揃え、既存 kebab-case
    flag の `--worker-file`/`--compare-model` 系に倣う)。flag >
    env var の優先順位。
  - `tracing-appender` 系 rolling file(daily/per-startup + 世代数)。
    既定値は実装 design で確定。

## Verification

* [ ] log 無効で regression suite(55 cases)pass・envelope
  byte-parity 不変。
* [ ] `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` または flag で有効化時に
  file + stderr 両 sink へ stage timing event が出力され、JSON
  format が parse 可能。
* [ ] flag が env var を override する(level・format 双方)。
* [ ] `--quiet` 下で diagnostics が抑制され、明示的 log 有効化は
  出力される(確定済みルールを test 化)。
* [ ] credential・prompt・tool payload 本体が file/stderr の log に
  出現しない regression test がある。
* [ ] `.cheaper/logs/` が `.gitignore` の `logs` pattern で cover
  される。
* [ ] hook 経路で stderr sink が hook contract に干渉しない(干渉時は
  無効化 fallback が動作)。

## Pros and Cons

### S-C: file + stderr dual routing

* Good, because リアルタイム確認と永続化証跡を両立し、stdout を
  触らない。
* Bad, because stderr が diagnostics と log の 2 系統を運び共存設計
  が必要で、sink ごとの format/init 配線が増える。

### S-A: stderr のみ / S-B: file のみ

* Good, because 実装最小(S-A)または永続化のみ確実(S-B)。
* Bad, because S-A は証跡が残らず事後調査・CI で使えず、S-B は
  実行中の確認ができない。

### `.cheaper/logs/` vs userConfigDir / 明示 path

* Good, because `.cheaper/` は既存の project-level config dir で
  `.gitignore` の `logs` pattern が cover、project-scope diagnosis
  の置き場として自然。
* Bad, because consumer が project 外の集約場所を欲する用途
  (userConfigDir 的集約)は別途考慮が必要になる可能性がある。

### R-B: rolling(daily/per-startup)+ 世代数管理

* Good, because サイズ・世代上限が予測可能で、tracing-appender 型の
  実績ある機構に寄せられる。
* Bad, because rolling 間隔・保持世代数・file 命名の既定値を別途
  決める必要がある。

### R-A: 単一 file 無制限追記 / R-C: 起動ごと truncate

* Good, because どちらも実装が単純。
* Bad, because R-A は肥大して disk 消費が予測不能、R-C は直前 run
  の証拠を失い調査用途に耐えない。

### A-C: env var + CLI flag(flag 優先)

* Good, because hook/子プロセス経路(env var)と per-invocation の
  明示(flag)の両方を cover し優先順位が明確。
* Bad, because env var 名・flag 名・優先ルールの文書化と、
  `--quiet` との関係整理が必要。

### A-A: env var のみ / A-B: CLI flag のみ

* Good, because 経路が 1 系統で単純(A-A は hook・子プロセス経路
  へ伝播、A-B は呼び出しごとの明示性)。
* Bad, because A-A は 1 回限りの有効化が冗長で shell 環境に依存
  し、A-B は hook 経由等 flag を通せない経路で有効化不能。

### F-C: human-readable text + structured JSON 両対応・切替

* Good, because 実行中の ad-hoc 調査(text)と CI・機械処理(JSON)
  の両用途を 1 基盤で cover する。
* Bad, because formatter の分岐と field 規約の二重検証が必要。
  F-A 単独では CI・機械処理での parse が不安定、F-B 単独では
  人間の ad-hoc 調査で読みにくい。

### SEC-B: metadata-only 既定 + 別 opt-in

* Good, because 既定で credential・prompt・tool payload が disk/
  stderr に残らず、深堀りが必要な場合のみ opt-in で責任境界が
  明確。
* Bad, because redaction 規則の網羅 test と、opt-in 機構の形(別 env
  var/file 限定/stderr 除外等の制約)の後続設計が必要。

## More Information

* 上流: DISC-034DUx01YRjOAf4gTaftO7(D2・D5・D6・D7・D9 の確定
  記録、stderr `CliIo` 共存の design constraint 記録)。
* 関連 spec: SPEC-034DUFyMl47o924O1KW8Ya(実施スコープ・raw
  payload opt-in の deferred 記録)。
* 関連決定: crate topology は ADR
  consolidate-observability-behind-a-dedicated-crate、envelope
  contract は DESIGN cli-contract-envelope-exit-codes、stderr
  prompt の CliIo bridge は ADR auth-terminal-prompt-io-boundary。

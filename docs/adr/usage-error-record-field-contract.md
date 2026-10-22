---
id: "ADR-RrljT4yH9woiGWbIny9ifG"
type: "adr"
status: "accepted"
title: "usage.jsonl の run error record は固定 allowlist の診断 field で拡充する (issue #141)"
created: "2026-10-23"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
    - "crates/cheaper-cli/src/usage_log.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/run-error-record-observability.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/run-error-record-observability.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../designs/run-error-record-observability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
  derived-by:
    - "../designs/run-error-record-observability.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# usage.jsonl の run error record は固定 allowlist の診断 field で拡充する (issue #141)

## Context and Problem Statement

Issue #141 の調査(DISC-034Kqi8LcVZaxgz7wxHfg4)で、
`usage.jsonl` の `kind:"run"` error record が `error: {"code"}` のみを
書く設計(`crates/cheaper-cli/src/usage_log.rs:151-158`)が、
失敗率 30.6% の原因分析を telemetry から完結できない根本原因であると
確定した。envelope(stdout)には `error.message`・`error.details`・
`meta.repairs` が残るが、record が落とすため永続 telemetry としては
機能しない。record を拡充する上での field contract を決める必要がある。

## Decision Drivers

* usage.jsonl は `<userConfigDir>` 下の per-run durable telemetry で、
  envelope や `.cheaper/logs` とは別の保存面(tracing は `CHEAPER_LOG`
  opt-in で rotated)。
* `error.details` には model の生出力(`rawOutput`、workspace file の
  excerpt を含み得る)や user home 直下の absolute path
  (`rawOutputPath`/`checkpoint.path`)が混在する — そのまま写すと
  解析用に export される可能性のある JSONL へ生データが残る。
* code だけでなく「code の直下の原因分類」が要る: abort の reason、
  schema 失格の instancePath、provider 失敗の errorMessage。
* 成功・失敗両方の run で「どの version・どの実効 budget が走ったか」が
  TIMEOUT/MAX_TURNS の出所判別に必須(discovery F-1: record には
  version も policy echo も無く、旧版説と policy 固定説を識別不能)。

## Considered Options

* **Option A(採用)**: `error` object を固定 allowlist で拡充する。
  `code` 既存 + `reason`(abort 語彙のみ)・`message`(≤300字)・
  `schemaErrors`(`{instancePath,message}` ≤5件)・`errorMessage`(≤300字)・
  `checkpoint`(`{path,observationsCount}` をそのまま)の条件付き field。
  併せて全 `kind:"run"` record に `version`・実効
  `execution: {timeoutMs,maxTurns,maxToolCalls}`・`repairs` を記録する。
* **Option B**: `error.details` を verbatim で(サイズ cap 付き)写す。
  却下: `rawOutput`(最大2000字で model 生出力=workspace file excerpt
  を含む)・absolute path を含むまま durable log へ残るため、
  「参照(path)は記すが中身は記さない」という privacy/容量の
  境界を保てない。
* **Option C**: record は据え置き、`.cheaper/logs` の tracing や
  `<userConfigDir>/invalid-outputs/` の sidecar に委ねる。
  却下: tracing は `CHEAPER_LOG` opt-in で file rotation されるため
  「後から集計して原因分類する」用途に使えない。invalid-outputs
  sidecar は INVALID_OUTPUT のみをカバーし、TIMEOUT/PROVIDER_FAILURE
  の分類には寄与しない。

## Decision Outcome

Option A を採用する。

1. **分類に必要な最小集合を固定で名付ける** — `reason`・
   `schemaErrors[].instancePath`・`errorMessage`・`checkpoint`・
   `message`・`version`・`execution`・`repairs` はいずれも
   「error.code の直下の原因分類」を record から答えるための
   bounded な値であり、生の model/workspace 内容(`rawOutput` 等)は
   含まない。ただし echo が完全にゼロではない点に注意:
   `schemaErrors[].message` は jsonschema の `err.to_string()` 由来で
   失格した instance 値や `instancePath` の property 名を写し得る。
   したがって privacy の保証は「内容を一切含まない」ではなく
   「≤5 件 × message ≤120 字(`instancePath` ≤200 字)の bounded
   echo」に限定される。
2. **写す元は envelope の既存 contract** — `error.details.*` と
   `meta.repairs` を選んで写すだけで、runtime 側の error 生成・
   details 内容は変えない(envelope 公開 contract 不変)。
3. **`version`/`execution` echo は成功 record にも付ける** — 「どの
   版・どの budget が走ったか」は成否に依らず必要で、追加コストが
   定数級で小さい。

## Consequences

* 良い結果: `cheaper usage` の JSONL だけで TIMEOUT の出所(timeoutMs
  echo + version)・INVALID_OUTPUT の失格箇所(instancePath+repairs)・
  PROVIDER_FAILURE の下位分類(errorMessage)・budget 枯渇の resume 先
  (checkpoint)が判別できるようになり、#141 型の調査が record から
  完結する。version echo は今後の「この error はどの版で出たか」の
  切り分けを常に可能にする。
* 負の結果: record 行が肥大化する(数十〜数百バイト/行)が、
  best-effort の append-only JSONL としては許容範囲。allowlist は
  新しい `error.details` key が増えたとき record 側の追随を要求する
  (drift 検知機構は無い — 既知リスク、invalid-outputs の
  `details.checkpoint` 同様)。
* `schemaVersion` は 1 のまま維持する予定 — additive field 追加であり、
  既存 reader が未知 field を無視する限り互換(最終判断は design で)。

## References

* Discovery: `docs/discovery/worker-run-failure-rate-31pct-investigation.md`
* Spec: `docs/specs/run-error-record-observability.md`
* 関連 ADR: `docs/adr/invalid-output-raw-output-persistence.md`
  (生 output を file へ退避し参照のみ返す同型の先例)

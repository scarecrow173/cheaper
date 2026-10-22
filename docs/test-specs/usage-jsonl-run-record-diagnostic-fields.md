---
id: "TSPEC-034Kr7KMznzk97mL1fbS1b"
type: "test-spec"
status: "draft"
title: "usage.jsonl run record diagnostic field contract (issue #141)"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
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
    - "docs/plans/implement-run-error-record-observability.md"
    - "docs/discovery/worker-run-failure-rate-31pct-investigation.md"
    - "docs/adr/usage-error-record-field-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/run-error-record-observability.md"
    - "docs/designs/run-error-record-observability.md"
    - "docs/adr/usage-error-record-field-contract.md"
  derived-by: []
  verifies:
    - "docs/specs/run-error-record-observability.md"
    - "docs/designs/run-error-record-observability.md"
  verified-by:
    - "docs/tasks/extend-usage-log-error-record-diagnostic-fields.md"
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
  references:
    - "docs/discovery/worker-run-failure-rate-31pct-investigation.md"
  defers: []
  deferred-by: []
---

# usage.jsonl run record diagnostic field contract (issue #141)

## Purpose

issue #141 の調査(DISC-034Kqi8LcVZaxgz7wxHfg4)で確定した構造的
observability gap — `kind:"run"` error record が `error: {"code": ...}`
のみを書き、失敗の原因分類に必要な情報が永続 telemetry に残らない —
を解消する record 拡充が、SPEC-G1ALq0d5lyFIi0h43tZsZG と
DESIGN-034KqI29dOTRXERRPTp5gV の field contract を満たし続けることを
pin する。

保つべき意図は 2 つ。(1) **分類可能性**: `version`・実効 `execution`
echo・`repairs`・error allowlist field だけで TIMEOUT の出所・
INVALID_OUTPUT の失格箇所・PROVIDER_FAILURE の下位分類・budget 枯渇の
resume 先が record 単体から答えられること。(2) **境界の固定**:
allowlist は固定であり、model 生出力(`rawOutput`)や payload 参照
(`rawOutputPath`)・その他 details key は durable log に漏らさないこと
(ADR Option A/B の privacy/容量境界)。

record field contract(allowlist・cap・条件)や echo 供給経路の再設計が
起きた場合、あるいは `cheaper usage` 側に error 内訳集計が入って本
spec の守る「record だけで分類可能」の責務が移る場合にのみ退役する。

## Feature

`crates/cheaper-cli/src/usage_log.rs` の `build_usage_record` が
envelope `Value` から構築する `kind:"run"` record(success 分岐と
usage-bearing error 分岐の両方)の field contract — `version`・
`repairs`・`execution` echo・`error` object の allowlist 拡張と、
それらを供給する `ExecutionEcho`(`From<&ExecutionPolicy>` 射影)→
`RunCtx` → `record_usage` の配線。

## Rules

- Rule: `version` は全 `kind:"run"` record に binary 自身の
  `env!("CARGO_PKG_VERSION")` を記録する — success・error の両分岐で
  `ok` 直後に位置する。builtin_dir override や resource version.json
  ではなく「どの binary が走ったか」の値。
- Rule: `execution` echo は `restrict_policy` 解決後の
  `ExecutionPolicy` から `{timeoutMs, maxTurns, maxToolCalls}` の
  3 field だけを写す固定射影。`maxTurns` は `Option<u64>` —
  policy 上の上限なし(`None`)は record で `null` として記録し、
  「key 無し(=echo 未供給)」と区別する。`concurrency` 等他 field は
  record shape に漏らさない。
- Rule: `repairs` は `env.meta.repairs` が non-null のときだけ
  `savings` の後に number として記録する(既存 `!is_null` 規約と同一)。
- Rule: `error` object の拡張は固定 allowlist と固定順 —
  `reason`(abort 語彙 `{"timeout","signal","maxTurns","maxToolCalls"}`
  に一致する `error.details.reason` のみ)・`message`(`error.message`
  先頭 300 文字)・`schemaErrors`(`error.details.errors` が array の
  とき先頭 5 件を `{instancePath ≤200 文字, message ≤120 文字}` —
  object entry のみ、欠落 key はその key を省略)・
  `errorMessage`(`error.details.errorMessage` 先頭 300 文字)・
  `checkpoint`(`error.details.checkpoint` が object のとき
  `{path, observationsCount}` のみ — 両 key 欠落なら `checkpoint`
  自体を省略)。
- Rule: allowlist 外は記録しない — `rawOutput`・`rawOutputPath`・
  `exitCode`・`retryable`・`cause`・`details` の残余 key
  (`changes`/`missing`/`grant`/`layer` 等)は record に載らない。
- Rule: 切り詰めは char(Unicode scalar)基準で、byte 境界の中間で
  文字を壊さない。
- Rule: fail-soft — 対応値が無い・型が合わない field は key 自体を
  省略し、`build_usage_record` が `None` や panic にならない。
  `echo` が `None` の run record では `execution` key を省略する
  だけで書き込みは継続する。
- Rule: additive-only — `schemaVersion: 1` 維持、既存 field の形・
  記録条件は不変、key 挿入位置は固定(`version` は `ok` 直後、
  `repairs` は `savings` の後、`execution` は `repairs` の後)。
- Rule: `cheaper usage` 非依存 — 新 field を含む record が混在する
  file を従来どおり集計できる(unknown field 無視の reader 構造)。
- Rule: 構造的 edge — `PARTIAL_MUTATION` では `reason` は inner
  `cause` 側に畳まれるため record の `reason` は載らない
  (`checkpoint` のみ記録)。`details.checkpoint` は best-effort で
  abort record でも `checkpoint` key が載らない経路がある — test は
  `checkpoint` の無条件の存在を要求しない。

## Examples

- Example: TIMEOUT の usage-bearing error envelope
  (`error.details.reason: "timeout"`)→ record の `error.reason` が
  `"timeout"` で、`execution.timeoutMs`・`version` が記録される —
  TIMEOUT の出所(絶対 vs stall、値の固定元)が record から判別可能。
- Example: INVALID_OUTPUT(`error.details.errors` が 5 件超の array・
  `meta.repairs: 2`)→ `schemaErrors` は先頭 5 件の
  `{instancePath, message}`、`repairs` が記録される — 失格箇所と
  repair 消費の有無が判別可能。
- Example: PROVIDER_FAILURE(`error.details.errorMessage:
  "GetChatMessage HTTP 500: ..."`)→ `errorMessage` ≤300 文字で記録
  — HTTP status・transport 失敗・EOS 未到達の下位分類が判別可能。
- Example: MAX_TURNS_EXCEEDED / MAX_TOOL_CALLS_EXCEEDED →
  `error.reason` が `"maxTurns"`/`"maxToolCalls"`、`checkpoint` が
  あれば `{path, observationsCount}` で記録される。
- Example: `error.details.reason` が abort 語彙外の文字列
  (例: `"user"` や独自 reason)の envelope → `reason` は記録されない。
- Example: `error.details.errors` の entry に `message` が無い object
  → `instancePath` のみの entry として記録される。
- Example: 300 文字超の `error.message`・`errorMessage`、200 文字超の
  `instancePath`、120 文字超の `schemaErrors[].message` → 各 cap で
  char 基準に切り詰められる。
- Example: success run envelope → `version`・`execution`(echo 供給時)
  が記録され `error` object は無い。
- Example: `echo` を `None` で構築した run record → `execution` key
  自体が無い(その他 field は通常どおり記録)。
- Example: `maxTurns: null` の policy → `execution.maxTurns` は
  `null` として記録される(欠落ではない)。
- Example: `run::execute` を dev-seam fake-stream fixture で実行した
  error run → 実 `ctx.execution_echo` 配線経由で record の
  `version`・`execution`・`error.reason` が pin される(fixture-level
  の `build_usage_record` test だけに留めない)。

## Guarantees

- `ok:false` の `kind:"run"` record が `version`・`execution`・
  `repairs` と `code` に加えて条件該当する診断 field を持ち、
  spec の全 acceptance criteria を record 単体で答えられる。
- error `details` の新 key 追加があっても allowlist 外は record に
  漏れない(privacy/容量境界が構造的に保たれる)。
- `cheaper usage` の既存集計(`--since`/`--worker`/`--provider`/
  `--model`/`--compare-model`)は新 field の存在下で同じ結果を返す。
- 旧 writer の record(`version`/`execution`/`repairs`/`error.*` なし)
  と新 writer の record は同一 usage.jsonl に混在可能。
- echo 未供給・details 欠落・型不一致でも record の書き込み自体は
  落ちない(best-effort append 規約の維持)。

## Non-goals

- envelope(stdout)公開 contract の検証 — `meta`/`error` の構造・
  details 内容は runtime/envelope 側の test-spec 領域であり、
  本 spec は record 側の選択・切り詰めのみを固定する。
- `kind:"count"`/`"route"`/`"route-refetch"` record の contract。
- `cheaper usage` の error 内訳集計・`--error` filter(別 issue —
  spec の Open Item で deferred)。
- `rawOutput`/`rawOutputPath` の record 追加(ADR Option A が除外
  した payload 参照 — Deferred Design Concerns 側)。
- `error.cause.code`(PARTIAL_MUTATION wrap 時の abort code)の
  record 追加(allowlist 固定外 — Deferred Design Concerns 側)。
- `checkpoint` の存在そのもの — best-effort 保存であり、
  本 spec は「あれば `{path,observationsCount}` で写す」射影のみ。
- 実行結果の記録 — pass/fail・回数は impl-doc experiment log の領域。

## Risk

- allowlist が守られないと `rawOutput`(model 生出力・workspace
  file excerpt)や absolute path が durable log に残留し、ADR
  Option B を却下した privacy/容量境界が破れる。
- `version`/`execution` echo が欠落すると、TIMEOUT の出所(旧版の
  絶対期限説 vs policy 固定説)が再び record から識別不能になる —
  issue #141 で実際に分析を詰まらせた gap の回帰。
- fail-soft が壊れる(欠落 key・型不一致で `None`・panic)と、
  best-effort append 規約が破れて record 書き込み自体が失われ、
  telemetry の存在前提が揺らぐ。
- 切り詰めが char 基準でないと multi-byte 文字で JSON が壊れ、
  `cheaper usage` が行全体を skip する破損行を生む。
- e2e pin が無いと `ctx.execution_echo` の設定忘れ(restrict_policy
  後の暗黙の手順依存)が compile で検出されず、`execution` が静かに
  欠落したまま出荷される。

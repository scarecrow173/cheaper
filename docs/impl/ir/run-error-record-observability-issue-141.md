---
id: "IMPL-034KrGAahAOH1wp7c8Wcob"
type: "impl"
status: "completed"
title: "Implement run error record observability issue 141"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/extend-usage-log-error-record-diagnostic-fields.md"
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
    - "docs/tasks/update-usage-jsonl-record-field-docs.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-run-error-record-observability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/usage-jsonl-run-record-diagnostic-fields.md"
    - "docs/discovery/worker-run-failure-rate-31pct-investigation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-run-error-record-observability.md"
    - "docs/designs/run-error-record-observability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/usage-error-record-field-contract.md"
    - "docs/specs/run-error-record-observability.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement run error record observability issue 141

## Summary

issue #141 の調査(DISC-034Kqi8LcVZaxgz7wxHfg4)で確定した observability
defect — usage.jsonl の `kind:"run"` error record が `{"code"}` しか
持たず「なぜ失敗したか」を record から判別できない構造的 gap — を、
design(DESIGN-034KqI29dOTRXERRPTp5gV)/ ADR 固定 allowlist 契約に
従い解消した。`kind:"run"` record(success・error 両分岐)へ
`version`・`repairs`・`execution` echo を追加し、error 分岐の
`error` object を allowlist 5 field へ拡張した。

1. `version`: `env!("CARGO_PKG_VERSION")` を `ok` 直後に常時挿入 —
   record を emit した binary の product version で新旧版を識別する。
2. `repairs`: `env.meta.repairs` を non-null のみ記録(既存
   `turns`/`toolCalls`/`savings` の `!is_null` 規約と同一)。
3. `execution`: `restrict_policy` 解決後の `ExecutionPolicy` から
   `ExecutionEcho`(timeoutMs/maxTurns/maxToolCalls の 3 field 固定
   射影)を `RunCtx` 経由で供給。`maxTurns` は `None` → `null` で
   「上限なし」を記録し key 欠落(echo 未供給)と区別する。
4. `error` object: `code` に加え `reason`(abort 語彙一致のみ)・
   `message`/`errorMessage`(char 基準 300 字 cap)・`schemaErrors`
   (object entry 先頭 5 件・instancePath ≤200/message ≤120)・
   `checkpoint`(`{path, observationsCount}` のみ)を固定順に射影。
   allowlist 外(rawOutput/rawOutputPath/exitCode/retryable/cause
   等)は一切写さない。

## Context

TASK-034Kr8hGPsSchbau1iTWjt(T1)・TASK-034Kr8ooGb34ib95k3TT6i(T2)・
TASK-034Kr8oUgcvTuprTQJwuyp(T3)・TASK-034Kr8pszGdyClSDbgMU3Q(T4)。
PLAN-034Kr7PAaZlxMx7LjG34Ls / DESIGN-034KqI29dOTRXERRPTp5gV /
SPEC-G1ALq0d5lyFIi0h43tZsZG / ADR-RrljT4yH9woiGWbIny9ifG に従う。
issue #141 自体は worker run 失敗率 31% の要因調査であり、発火機構
(TIMEOUT/MAX_TURNS/INVALID_OUTPUT/PROVIDER_FAILURE)は
DISC-034Kqi8LcVZaxgz7wxHfg4 で機構確定済み。本 chain が修復するのは
同 discovery の C5(確定)「record が `error.code` しか持たない」という
構造的 observability gap。

## Implementation

### Changes

- `crates/cheaper-cli/src/usage_log.rs`:
  `truncate_chars`(char / Unicode scalar 基準、`repair.rs` 同一規約)と
  `ABORT_REASONS` 語彙 const、`ExecutionEcho` 型 +
  `From<&ExecutionPolicy>`(3 field 固定射影)、
  `insert_execution_echo`・`error_record_object` helper を追加。
  `build_usage_record`/`record_usage` に `echo: Option<&ExecutionEcho>`
  引数(`now` 手前)を追加し、success・error 両 `kind:"run"` 分岐で
  `ok` 直後に `version`、`savings` 後に `repairs`(non-null のみ)、
  その直後に `execution`(`Some` のみ)を挿入。error object は
  `code` 既存に続き allowlist 5 field を条件付き・固定順・cap 付きで
  射影。`kind:"count"`/`"route"`/`"route-refetch"` record の shape
  は不変。
- `crates/cheaper-cli/src/cli.rs`: `RunCtx` に
  `execution_echo: Option<ExecutionEcho>` field を追加。run success
  arm・汎用 catch は `ctx.execution_echo.as_ref()`、count-tokens は
  `None` を渡す。
- `crates/cheaper-cli/src/run.rs`: `restrict_policy` 直後に
  `ctx.execution_echo = Some((&effective_policy.execution).into())`
  — 解決後の実効値であり declared 値ではない。
- `crates/cheaper-cli/tests/integration/usage_log.rs`: 既存
  `build_usage_record` 呼出し 12 箇所に `None` を追加し signature
  追随した上で(T2)、diagnostic field section に 10 test を新規追加
  (計 20 test) — `version`/`repairs`/`execution` の位置・条件・
  `From` 射影、error allowlist 固定順・全 field 記録・allowlist 外
  非記録・multi-byte cap・条件付き省略・fail-soft、`cheaper usage`
  非回帰。
- `crates/cheaper-cli/tests/run_dev_seam.rs`: `FIXTURE_ABORT_POLICY_ECHO`
  (timeout_ms:45000/max_turns:9/max_tool_calls:5)と
  `run_error_record_pins_version_execution_and_reason_e2e` を追加 —
  実配線経由で `version`・`execution`(解決後の実効値)・
  `error.reason:"signal"` を pin。
- `docs/guides/usage.md`・`AGENTS.md`: run record の新 field 群を
  文書化。record の `execution`(policy echo)と envelope
  `meta.execution`(ExecutionMetrics、record 非搭載)の同名別概念を
  明記。

### Decisions During Implementation

- `version` は `env!("CARGO_PKG_VERSION")` を **cheaper-cli で評価**
  する — runtime crate 側では `version = "0.1.0"` pin のため値が
  違う(design §Risks の review 指摘を反映)。「record を emit した
  binary 自身の version」という semantic を保持するため cli 側の
  compile-time 定数のままとした。
- `checkpoint` は best-effort 射影 — `{path, observationsCount}` の
  み写し、両 key 欠落なら `checkpoint` key 自体を省略する。将来
  details 側に key が追加されても record に漏れない(allowlist)。
- `PARTIAL_MUTATION` 相当の error(details.cause 畳み込みで reason
  語彙を持たない)は reason 無し・checkpoint のみの構造的 edge と
  して扱い、追加 field の拡張は Deferred Design Concerns に送った。
- `UnknownToolCall` 由来の失敗は details を持たないため error object
  は `code` のみになる — record 契約として意図どおり(詳細なし
  error を無理に補完しない)。
- `echo` 引数追加に伴う signature 変更・`RunCtx` field・3 呼出し
  箇所・`run.rs` 設定を 1 つの原子単位として実施 — compile error
  強制で配線漏れを検出する design そのもの。

## Related Experiments

### Adopted

なし(known-solution task。design が field 契約・cap・挿入位置・配線
形まで確定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::`: 20 test 全 pass
- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test run_dev_seam`: 38 test 全 pass
- `cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings`: pass
- `cargo fmt` / `cargo check --all-targets`: pass
- `mise run check`: 全 step exit 0(biome・`cargo fmt --check`・
  clippy・version/doc-id check・`cargo test --workspace`・
  `node --test`・lockfile/apm-payload check)
- `node scripts/check-doc-ids.mjs`: pass(T4 Verification)

証跡 commit: `11c70d1`(T1 — diagnostic fields)→ `ab3c2db`
(T2 — execution echo 配線)→ `7cea563`(T3 — contract test + e2e
pin)→ `db692c1`(T4 — usage.md/AGENTS.md 文書化)。各 task の完了
記録は `781e159`・`5fb7759`・`ed83253` の task-doc commit に付随。

## Risks

- `execution` echo は `restrict_policy` 以降の値であるため、
  `restrict_policy` 以前の失敗(worker/model 解決 error 等、
  usage-bearing でない failure)には `execution` が載らない —
  record contract 上の意図どおりで、記録対象外 failure は引き続き
  record 自体が生成されない。
- `error.reason` は abort 語彙への一致時のみ記録されるため、
  PROVIDER_FAILURE/INVALID_OUTPUT 系 error では `reason` が欠落し
  `code` + `errorMessage`/`schemaErrors`/`checkpoint` で判別する
  構造になる。`cheaper usage` の error 内訳集計・`--error` filter は
  spec Open Item として別 issue に送り済み。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation chain
(issue #141)由来の未分類・未処理の follow-up はない。

- issue #141 は investigation issue であり、依頼された deliverable は
  「失敗原因の診断」。本 chain は調査で確定した observability defect
  (C5)の修復 + 機構仮説の canonical 記録
  (`docs/discovery/worker-run-failure-rate-31pct-investigation.md`、
  status: resolved)をもって完了する。
- root-cause 仮説(C1: pre-#111 default `execution.timeout:120s` +
  旧 `maxTurns:8` 由来の ~120s TIMEOUT、C2: MAX_TURNS 機構、C3:
  INVALID_OUTPUT → `output.maxTokens:2000` 打ち切り疑い、C4:
  PROVIDER_FAILURE 写像面)は discovery doc の Tentative
  Conclusions/Open Questions に記録済みだが、確定には issue reporter
  の環境データ(install version・policy.yaml・checkpoints/
  invalid-outputs sidecar・観測期間と upgrade 時期)が必要で、repo 内
  で実行可能な follow-up 作業ではない。データが提供された場合は
  新規 chain/issue として扱う。
- `cheaper usage` の error 内訳集計・`--error` filter は spec Open
  Item/Deferred として既に分類済み(別 issue 送り)のため新規
  follow-up artifact は起こさない。

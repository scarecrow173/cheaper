---
id: "TASK-034Kr8oUgcvTuprTQJwuyp"
type: "task"
status: "done"
title: "Add usage.jsonl diagnostic field contract tests and end-to-end echo pin"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-run-error-record-observability.md"
  implemented-by:
    - "docs/impl/ir/run-error-record-observability-issue-141.md"
  depends-on:
    - "docs/plans/implement-run-error-record-observability.md"
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
  blocks:
    - "docs/tasks/update-usage-jsonl-record-field-docs.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-run-error-record-observability.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/usage-jsonl-run-record-diagnostic-fields.md"
  references:
    - "docs/designs/run-error-record-observability.md"
    - "docs/specs/run-error-record-observability.md"
  defers: []
  deferred-by: []
---

# Add usage.jsonl diagnostic field contract tests and end-to-end echo pin

## Work

plan `docs/plans/implement-run-error-record-observability.md`
(PLAN-034Kr7PAaZlxMx7LjG34Ls)の T3。TSPEC-034Kr7KMznzk97mL1fbS1b
の Rules・Examples を fixture-level test と 1 件の e2e pin に落とす。
design §Verification Notes の全項目を網羅する。

### 1. `tests/integration/usage_log.rs` — fixture-level record contract test

既存の envelope fixture パターン(`json!` + `build_usage_record` +
`fixed(0.0)`)に倣って追加する。対象:

- **error allowlist の網羅**: `error.details.reason`(abort 語彙)・
  `error.message`・`error.details.errors`・`error.details.errorMessage`・
  `error.details.checkpoint` を持つ usage-bearing error envelope から、
  `error.reason`/`message`/`schemaErrors`/`errorMessage`/`checkpoint`
  が記録されること。
- **allowlist 外の非記録**: `details.rawOutput`/`rawOutputPath`/
  `exitCode`/`retryable`/`cause`/`details` の残余 key
  (`changes`/`missing`/`grant`/`layer` 等)を含む envelope で、
  record の `error` object にそれらが載らないこと。
- **cap と件数制限**: 300 文字超 `message`/`errorMessage`・
  200 文字超 `instancePath`・120 文字超 `schemaErrors[].message`・
  5 件超 `errors` array がそれぞれ切り詰められること(char 基準 —
  multi-byte 文字を含む入力で byte 壊れがないこと)。
- **条件付き記録**: abort 語彙外の `details.reason` は非記録、
  `errors` entry の欠落 key はその key を省略、`checkpoint` の両 key
  欠落は `checkpoint` 自体を省略。
- **fail-soft**: 新 field の欠落・型不一致でも `build_usage_record`
  が `Some` を返し panic しないこと。
- **`version`**: record の `version` が `env!("CARGO_PKG_VERSION")`
  と一致すること(success・error 両分岐)。
- **`execution` echo**: `ExecutionEcho` を渡した record が
  `{"timeoutMs": <u64>, "maxTurns": <u64|null>, "maxToolCalls": <u64>}`
  を `repairs` の後に持つこと。`max_turns: None` → `null` で key 有り、
  `echo: None` → `execution` key 無し。
- **`repairs`**: `meta.repairs` non-null で記録、null で省略
  (success・error 両分岐)。
- **`cheaper usage` 非回帰**: 新 field を含む record 行を混在させた
  file で `read_usage_records`/`build_usage_report` が従来どおり
  集計できること(既存の新旧互換 test と同じ構造で追加)。

### 2. `tests/run_dev_seam.rs` — end-to-end echo pin

`run::execute` → `restrict_policy` → `ctx.execution_echo` →
`record_usage` の実配線経由を通し、`f.usage_log()` の record で
`version`・`execution`・`error` field を pin する(design §Verification
Notes が fixture-level のみに留めないことを要求 — `ctx.execution_echo`
の設定順序は compile で検出されない暗黙の依存)。

- fake-stream fixture(既存 `FIXTURE_ABORT` 相当 — `usage_delta` +
  `hang_until_abort` で usage-bearing ABORTED error を出すもの)で
  `Fixture::run(&["run", "fixture-worker"])` を実行し、usage.jsonl
  の `kind:"run"` error record を検査する。
- assert: record の `version` が `env!("CARGO_PKG_VERSION")`、
  `execution` が `{timeoutMs, maxTurns, maxToolCalls}` で
  fixture/既定 policy の解決後値と一致、`error.reason` が
  `"signal"`(fixture abort の `RunAbortReason::Signal` 由来)。
- `checkpoint` 等の best-effort field は存在を無条件には assert
  しない(test-spec §Rules の構造的 edge)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-run-error-record-observability.md` — 本 task
  が implement する plan。File Map・Dependency Graph の T3
- `docs/test-specs/usage-jsonl-run-record-diagnostic-fields.md`
  (TSPEC-034Kr7KMznzk97mL1fbS1b)— Rules・Examples・Guarantees が
  固定すべき契約
- `docs/designs/run-error-record-observability.md`
  (DESIGN-034KqI29dOTRXERRPTp5gV)— §Verification Notes の項目網羅
- `docs/specs/run-error-record-observability.md`
  (SPEC-G1ALq0d5lyFIi0h43tZsZG)— Acceptance Criteria

## Dependencies

- `docs/tasks/extend-usage-log-error-record-diagnostic-fields.md`
  (TASK-034Kr8hGPsSchbau1iTWjt)— envelope 由来 field の射影が前提
- `docs/tasks/wire-execution-echo-into-run-usage-records.md`
  (TASK-034Kr8ooGb34ib95k3TT6i)— `echo` 引数・`ctx.execution_echo`
  配線が前提

## Affected Files

- `crates/cheaper-cli/tests/integration/usage_log.rs`(modify —
  contract test 追加)
- `crates/cheaper-cli/tests/run_dev_seam.rs`(modify — e2e pin 追加)

## Done When

- [x] test-spec の全 Rules に対応する assert が存在する
- [x] `version`/`execution`/`error.reason` が実配線経由で pin される
      e2e test が 1 件存在する
- [x] `checkpoint` の無条件の存在を assert する test がない
- [x] `cheaper usage` が新 field 混在 file を従来どおり集計する
      非回帰 test が存在する

## Out of Scope

- `cheaper usage` の error 内訳集計・`--error` filter(別 issue)
- `rawOutput`/`rawOutputPath`・`error.cause.code` の record 追加
  (Deferred Design Concerns)
- docs 更新(T4)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test run_dev_seam` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

## Done

`tests/integration/usage_log.rs` に diagnostic field section を追加
(10 test 新規・計 20 test)。`error_env`/`ok_env` envelope helper +
`top_keys` を追加。網羅した内容:
- `version` — success・error 両分岐で `env!("CARGO_PKG_VERSION")` +
  `ok` 直後の位置 pin
- `repairs` — non-null で記録・null で省略(両分岐)
- `execution` echo — `{timeoutMs,maxTurns,maxToolCalls}` 3 key・
  `repairs` 直後の位置・`max_turns: None` → `null` 記録・
  `echo: None` → key 省略・`From<&ExecutionPolicy>` が 3 field のみ
  写すこと
- error allowlist — `code`/`reason`/`message`/`schemaErrors`/
  `errorMessage`/`checkpoint` の固定順 + 全 field 記録
- allowlist 外非記録 — `rawOutput`/`rawOutputPath`/`exitCode`/
  `retryable`/`cause`/`changes`/`missing`/`grant`/`layer`
- cap — 300/300/200/120 文字 + errors 5 件を multi-byte 入力で
  char 基準に切り詰め
- 条件付き省略 — abort 語彙外 reason・entry 欠落 key・両 key 欠落
  checkpoint・PARTIAL_MUTATION 相当(cause 畳み込みで reason 無し・
  checkpoint のみ)の構造的 edge
- fail-soft — details 欠落・型不一致で `Some` + panic 無し
- `cheaper usage` 非回帰 — 新 field 混在 file で
  `read_usage_records`/`build_usage_report` が従来どおり集計
`tests/run_dev_seam.rs` に `FIXTURE_ABORT_POLICY_ECHO`
(fixture `policy` 節で `timeout_ms:45000`/`max_turns:9`/
`max_tool_calls:5` を pin)と
`run_error_record_pins_version_execution_and_reason_e2e` を追加 —
usage-bearing ABORTED run の record で `version`・`execution`
(解決後の実効値)・`error.reason:"signal"` を実配線経由で pin。
`checkpoint` は存在時のみ key 集合を検査。`usage_log::` 20 test・
`run_dev_seam` 38 test・`clippy -D warnings`・`cargo fmt` 全て pass。

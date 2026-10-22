---
id: "PLAN-034Kr7PAaZlxMx7LjG34Ls"
type: "plan"
status: "approved"
title: "usage.jsonl run error record 診断 field 拡充の実装計画 (issue #141)"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
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
    - "docs/specs/run-error-record-observability.md"
  implemented-by:
    - "docs/tasks/extend-usage-log-error-record-diagnostic-fields.md"
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
    - "docs/tasks/update-usage-jsonl-record-field-docs.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/run-error-record-observability.md"
    - "docs/designs/run-error-record-observability.md"
    - "docs/adr/usage-error-record-field-contract.md"
  derived-by:
    - "docs/tasks/extend-usage-log-error-record-diagnostic-fields.md"
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
    - "docs/tasks/update-usage-jsonl-record-field-docs.md"
  verifies: []
  verified-by:
    - "docs/test-specs/usage-jsonl-run-record-diagnostic-fields.md"
  references:
    - "docs/discovery/worker-run-failure-rate-31pct-investigation.md"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/cli.rs"
    - "crates/cheaper-cli/src/run.rs"
  defers: []
  deferred-by: []
---

# usage.jsonl run error record 診断 field 拡充の実装計画 (issue #141)

## Goal

`usage.jsonl` の `kind:"run"` record(success + usage-bearing error)に
`version`・実効 `execution` echo・`repairs` を additive に追加し、
error record の `error` object を固定 allowlist の診断 field
(`reason`/`message`/`schemaErrors`/`errorMessage`/`checkpoint`)で拡充する。
issue #141 の失敗率 31% 調査(DISC-034Kqi8LcVZaxgz7wxHfg4)で確定した
「`error.code` の直下の原因分類が record に存在しない」構造的 gap を解消し、
同種の失敗分析が永続 telemetry から完結できるようにする。

## Scope

単一の implementation stream。spec・ADR・design が既に field contract
(allowlist・cap・条件付き key・additive-only)と配線方式(record 専用経路:
`env!("CARGO_PKG_VERSION")` + `RunCtx` 経由の `ExecutionEcho`)を確定して
おり、本 plan はその実装順序・task 分割・検証のみを担う。

| In | Out |
|---|---|
| `crates/cheaper-cli` の `usage_log.rs`(envelope `Value` → record `Value` の射影の単一責務境界) | envelope(stdout)の `meta`/`error` 構造・内容(公開 contract 不変 — `schemaVersion: 1` 維持) |
| `ExecutionEcho` + `From<&ExecutionPolicy>` + `RunCtx` 経由の echo 供給 | runtime(`cheaper-runtime`)・core(`cheaper-core`)・adapter の error 生成・details 内容 |
| `error.details` 由来の固定 allowlist projection と切り詰め | `kind:"count"`/`"route"`/`"route-refetch"` record |
| `tests/integration/usage_log.rs`・`tests/run_dev_seam.rs` の test | `cheaper usage` の集計・filter(新 field の集計利用は別 issue) |
| `docs/guides/usage.md`・`AGENTS.md` の record schema 記述 | `schemaVersion` bump・`<userConfigDir>/invalid-outputs/`・`checkpoints/` sidecar 機構 |

## Upstream Documents

- `docs/specs/run-error-record-observability.md`
  (SPEC-G1ALq0d5lyFIi0h43tZsZG — requirements・acceptance criteria)
- `docs/adr/usage-error-record-field-contract.md`
  (ADR-RrljT4yH9woiGWbIny9ifG — Option A 固定 allowlist の決定)
- `docs/discovery/worker-run-failure-rate-31pct-investigation.md`
  (DISC-034Kqi8LcVZaxgz7wxHfg4 — gap の構造的事実)

## Design Inputs

- `docs/designs/run-error-record-observability.md`
  (DESIGN-034KqI29dOTRXERRPTp5gV — AUTHORITATIVE な HOW:
  `ExecutionEcho` 形・key 挿入位置・allowlist 表・fail-soft 規約・
  既知の構造的 edge)

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/usage_log.rs` | record schema の単一責務境界 — `truncate_chars` helper・error `details` の allowlist projection・`version`(`env!("CARGO_PKG_VERSION")`)・`repairs` 転写・`ExecutionEcho` 型 + `From<&ExecutionPolicy>`・`echo` 引数 | T1・T2 が変更 |
| `crates/cheaper-cli/src/cli.rs` | `RunCtx.execution_echo` field(:302-308 付近)と `record_usage` 3 呼出し箇所の echo 受け渡し(run success :464・汎用 catch :266・count-tokens :367 は `None`) | T2 が変更 |
| `crates/cheaper-cli/src/run.rs` | `restrict_policy`(:223-229)直後の `ctx.execution_echo` 設定 — 実効値の唯一の供給源 | T2 が変更 |
| `crates/cheaper-cli/tests/integration/usage_log.rs` | record contract の fixture-level test(allowlist・cap・fail-soft・両分岐・reader 非回帰) | T2 で signature 追随・T3 で case 追加 |
| `crates/cheaper-cli/tests/run_dev_seam.rs` | `run::execute` → `record_usage` の end-to-end pin(`ctx.execution_echo` 実配線経由) | T3 が変更 |
| `docs/guides/usage.md` | run record field の記述・`execution` 命名衝突の注記 | T4 が変更 |
| `AGENTS.md` | delegation 計測ログ節の record-field 記述 | T4 が変更 |

## Implementation Sequence

- [ ] TASK-034Kr8hGPsSchbau1iTWjt: `usage_log.rs` に envelope 由来の
      診断 field(`version`・`repairs`・`error.reason`/`message`/
      `schemaErrors`/`errorMessage`/`checkpoint` + `truncate_chars`)を
      追加する — signature 不変で compile・既存 test pass を維持
- [ ] TASK-034Kr8ooGb34ib95k3TT6i: `ExecutionEcho` + `echo` 引数 +
      `execution` key 挿入 + `RunCtx.execution_echo` + cli.rs 3 呼出し
      + run.rs `restrict_policy` 後の設定 + 既存 test call site の
      signature 追随 — compile error 強制で配線漏れを検出
- [ ] TASK-034Kr8oUgcvTuprTQJwuyp: fixture-level record contract test
      (`tests/integration/usage_log.rs`)と `run::execute` →
      `record_usage` の e2e pin(`tests/run_dev_seam.rs`)を追加
- [ ] TASK-034Kr8pszGdyClSDbgMU3Q: `docs/guides/usage.md`・`AGENTS.md`
      の record field 記述を追随し `mise run check` で閉じる

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 envelope 由来 field | — | T2, T3 |
| T2 execution echo 配線 | T1 | T3, T4 |
| T3 contract test + e2e pin | T1, T2 | T4 |
| T4 docs + 最終検証 | T3 | — |

critical path は T1 → T2 → T3 → T4 の直列。T2 が T1 に依存するのは
`execution` key の挿入位置が `repairs`(T1 で追加)の直後と固定されている
ため(同一 file の順序依存)。T3 は両方の field 群を assert するため T2
までの成果を必要とし、T4 は verify 済みの contract を文書化する位置づけ。

## Dependencies and Risks

- T2 は signature 変更(`build_usage_record`/`record_usage` に
  `echo: Option<&ExecutionEcho>` 追加)を含むため、呼出し箇所(cli.rs
  3 箇所)と既存 test call site を同じ task 内で更新しないと compile
  error になる — 同一 commit で原子的に扱う。
- `ctx.execution_echo` の設定忘れは compile では検出されない
  (`Option` 初期値 `None` → `execution` key が静かに欠落する)。
  緩和は T3 の e2e pin — design が fixture-level test だけに留めない
  ことを明示している暗黙の手順依存。
- allowlist drift(ADR 既知リスク): `error.details` に新 key が増えても
  record は追従しない。今回の対応表(design §record schema)を更新する
  運用ルールのみで drift 検知機構は作らない。
- data-loss risk: なし(append-only JSONL への additive 変更。既存行は
  一切変わらず、新 field 欠落でも record 書き込みは落とさない)。
- breaking-change risk: なし(reader `is_record` は `kind` のみ検査し
  未知 field を無視 — `usage_report.rs:219-242`。新旧 writer の
  record は同一 file に混在可能)。
- external dependency risk: なし(新規 crate 依存ゼロ。
  `env!("CARGO_PKG_VERSION")` は compile-time 定数)。

## Task Breakdown

- T1 `docs/tasks/extend-usage-log-error-record-diagnostic-fields.md`
  — envelope/compile-time 由来 field の射影(signature 不変)
- T2 `docs/tasks/wire-execution-echo-into-run-usage-records.md`
  — `ExecutionEcho` 型と echo 供給経路(signature 変更の原子単位)
- T3 `docs/tasks/add-usage-log-diagnostic-field-tests.md`
  — fixture-level contract test + e2e pin
- T4 `docs/tasks/update-usage-jsonl-record-field-docs.md`
  — user-facing record schema 記述の追随

## Verification Matrix

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| T1 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` | exit 0(新 field 追加後も既存 assert 不変) |
| T2 | `mise exec -- cargo check -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | exit 0(全 call site・test が compile) |
| T3 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` + `--test run_dev_seam` | exit 0、新 assert が design の verification notes を網羅 |
| T4 | `mise run check` | exit 0(biome・fmt・clippy・doc-id 等の CI 同等 pipeline) |

## Rollback

全変更は additive(optional key のみ追加)かつ単一ブランチ内の
論理コミット単位で `git revert` 可能。不可逆 step(migration・公開
API・データ変換)は存在しない — usage.jsonl の旧行は不変で、新
field を消しても reader 集計に影響しない。

## Open Questions / Gaps

- `record_usage`/`build_usage_record` の `echo` 引数の位置は design が
  未指定 — `now` 引数の手前に追加するのが自然(既存引数列との整合)。
  T2 で確定する。
- e2e pin は `tests/run_dev_seam.rs` の fake-stream fixture 経路を使う
  (abort fixture で usage-bearing error record を生成可能)。
  INVALID_OUTPUT 経路の `schemaErrors`・`repairs` e2e は fixture 構成
  (repair 尽きまでの session 数)に依存するため、e2e では
  `version`/`execution`/`error.reason` の pin に留め、allowlist の
  網羅は fixture-level test に委ねる。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution — task は直列依存で並列化の利益
  がなく、T2 の signature 原子性が分割実行を阻む。
- Independent work streams: なし(T1 → T2 → T3 → T4 の直列)。
- User approval required before dispatch: yes(delegated execution を
  選ぶ場合のみ)。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: spec Req 2 の「`maxTurns` は `null` 許容」は `Option<u64>` の
  `None` → record `null` で表現し「key 無し(echo 未供給)」と区別する。
- 構造的 edge(design §既知の構造的 edge を再掲): `PARTIAL_MUTATION`
  では `reason` が inner `cause` 側に畳まれ record の `reason` は
  載らない(`checkpoint` のみ)。`details.checkpoint` は best-effort
  で MAX_TURNS/MAX_TOOL_CALLS でも欠落経路がある — test は
  `checkpoint` の無条件の存在を assert しない。
- 最も重要な upstream: `docs/designs/run-error-record-observability.md`
  の record schema 表(field・位置・型・出所・条件)が実装の
  authoritative 定義。

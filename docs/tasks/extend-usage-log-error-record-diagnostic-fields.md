---
id: "TASK-034Kr8hGPsSchbau1iTWjt"
type: "task"
status: "done"
title: "Extend build_usage_record error records with envelope-derived diagnostic fields"
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
        path: "crates/cheaper-cli/src/usage_log.rs"
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
  blocks:
    - "docs/tasks/wire-execution-echo-into-run-usage-records.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
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
    - "docs/adr/usage-error-record-field-contract.md"
  defers: []
  deferred-by: []
---

# Extend build_usage_record error records with envelope-derived diagnostic fields

## Work

plan `docs/plans/implement-run-error-record-observability.md`
(PLAN-034Kr7PAaZlxMx7LjG34Ls)の T1。`usage_log.rs` の
`build_usage_record`(:80-160)に、envelope/compile-time 由来の診断
field を **signature 不変のまま**追加する — record はこの時点で
`version`・`repairs`・拡張 `error` object を emit する(`execution`
echo の供給経路は T2)。

### 1. `truncate_chars` private helper

- char(Unicode scalar)基準の切り詰め helper を `usage_log.rs` に追加
  — `cheaper-runtime` の `repair.rs:223-228` と同一規約(runtime 側は
  private で再利用不可。5 行の local helper で十分、KISS)。byte
  境界で multi-byte 文字を壊さない。

### 2. `version` field(全 `kind:"run"` record)

- `ok` 直後に `env!("CARGO_PKG_VERSION")` を挿入 — success・error
  両分岐で常時。compile-time 定数のため配線・失敗モードを持たない
  (`cheaper-cli` で評価される点に注意 — runtime 側では
  `version = "0.1.0"` pin のため値が違う。design §Risks)。

### 3. `repairs` field(全 `kind:"run"` record)

- `env.meta.repairs` を `savings` の後に挿入 — non-null のみ
  (既存 `turns`/`toolCalls`/`savings` の `!is_null` 規約と同一、
  `usage_log.rs:146-150`)。success・error 両分岐で。

### 4. `error` object の allowlist projection(error 分岐のみ)

既存 `code`(:151-158)に加えて、**固定順**で以下を挿入する。

| field | 出所 | 条件・cap |
| --- | --- | --- |
| `reason` | `env.error.details.reason` | string かつ abort 語彙 `{"timeout","signal","maxTurns","maxToolCalls"}`(`RunAbortReason::as_str`、`abort.rs:36-43`)に一致するときのみ |
| `message` | `env.error.message` | string → `truncate_chars` 先頭 300 文字 |
| `schemaErrors` | `env.error.details.errors` | array のとき先頭 5 件を `{instancePath, message}` で写す — `instancePath` ≤200 文字・`message` ≤120 文字。object entry のみ、欠落 key はその key を省略 |
| `errorMessage` | `env.error.details.errorMessage` | string → 先頭 300 文字 |
| `checkpoint` | `env.error.details.checkpoint` | object のとき `{path, observationsCount}` のみ写す — それ以外の key を将来追加されても漏らさない。両 key 欠落なら `checkpoint` 自体を省略 |

fail-soft: 対応値が無い・型が合わない場合は key 自体を省略し、
`build_usage_record` が `None` や panic にならないこと。
allowlist 外(`rawOutput`/`rawOutputPath`/`exitCode`/`retryable`/
`cause`/`details` 残余 key)は一切写さない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-run-error-record-observability.md` — 本 task
  が implement する plan。File Map・Dependency Graph の T1
- `docs/designs/run-error-record-observability.md`
  (DESIGN-034KqI29dOTRXERRPTp5gV)— record schema 表・挿入位置・
  cap の AUTHORITATIVE 定義
- `docs/specs/run-error-record-observability.md`
  (SPEC-G1ALq0d5lyFIi0h43tZsZG)— Req 1・3・4・6
- `docs/adr/usage-error-record-field-contract.md`
  (ADR-RrljT4yH9woiGWbIny9ifG)— Option A 固定 allowlist の根拠

## Dependencies

- なし(後続 task は本 task に依存 — `blocks` 参照)。T2 による
  `execution` key の挿入位置は本 task が追加する `repairs` の直後。

## Affected Files

- `crates/cheaper-cli/src/usage_log.rs`(modify — 既存 signature・
  call site 不変)

## Done When

- [x] `truncate_chars` が char 基準で存在する
- [x] `kind:"run"` の success・error 両分岐に `version`
      (`env!("CARGO_PKG_VERSION")`・`ok` 直後)と `repairs`
      (non-null のみ)が挿入される
- [x] `error` object が `code` に加えて allowlist 5 field を
      条件付き・固定順・cap 付きで持つ
- [x] allowlist 外 key(`rawOutput`/`rawOutputPath`/`exitCode`/
      `retryable`/`cause`/その他 details key)が record に載らない
- [x] signature・呼出し箇所・既存 test が無変更で compile・pass する
- [x] `kind:"count"` record の shape が不変

## Out of Scope

- `ExecutionEcho` 型・`echo` 引数・`execution` key(T2)
- `cli.rs`/`run.rs` の変更(T2)
- test 追加(T3 — 本 task は既存 test の pass で検証する)

## Verification

- [x] `mise exec -- cargo build -p cheaper-cli` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

## Done

`crates/cheaper-cli/src/usage_log.rs` のみ変更(signature・call site
不変)。`truncate_chars`(char / Unicode scalar 基準・`repair.rs`
同一規約)と `ABORT_REASONS` 語彙 const を追加。success・error 両
`kind:"run"` 分岐で `ok` 直後に `version`
(`env!("CARGO_PKG_VERSION")`)、`savings` の後に `repairs`
(non-null のみ — error 分岐は既存 `!is_null` loop に `"repairs"`
を畳み込み、success 分岐は loop 後の条件 insert)。error object は
`code` 既存に続き `reason`(abort 語彙一致のみ)・`message`(≤300
文字)・`schemaErrors`(object entry 先頭 5 件・`instancePath`
≤200/`message` ≤120・欠落 key 省略・写せる entry 無しなら key
自体省略)・`errorMessage`(≤300 文字)・`checkpoint`(`{path,
observationsCount}` のみ・両欠落なら key 省略)を
`error_record_object` helper で固定順に射影。`cargo build`・
`usage_log::` filter 10 test・`clippy -D warnings`・`cargo fmt` 全て
pass。`kind:"count"` shape 不変。

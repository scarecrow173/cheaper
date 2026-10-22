---
id: "TASK-034Kr8ooGb34ib95k3TT6i"
type: "task"
status: "todo"
title: "Wire execution echo through RunCtx into run usage records"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-run-error-record-observability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-run-error-record-observability.md"
    - "docs/tasks/extend-usage-log-error-record-diagnostic-fields.md"
  blocks:
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
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

# Wire execution echo through RunCtx into run usage records

## Work

plan `docs/plans/implement-run-error-record-observability.md`
(PLAN-034Kr7PAaZlxMx7LjG34Ls)の T2。`kind:"run"` record の
`execution` echo を、`ExecutionEcho` 型 → `RunCtx` → `record_usage`
の record 専用経路で供給する。signature 変更・型・全 call site・
`run.rs` の echo 設定は **1 つの原子単位** — compile error 強制で
配線漏れを検出する設計そのもの(design §Risks)。

### 1. `usage_log.rs` — `ExecutionEcho` 型と `From` impl

```rust
/// `kind:"run"` record の `execution` echo — `restrict_policy` 解決後の
/// `ExecutionPolicy` から 3 field のみ写す固定 allowlist projection。
pub struct ExecutionEcho {
    pub timeout_ms: u64,
    /// `None` は policy 上の turn 上限なし — record では `null` として
    /// 記録し「key 無し(=echo 未供給)」と区別する(spec Req 2)。
    pub max_turns: Option<u64>,
    pub max_tool_calls: u64,
}

impl From<&cheaper_core::policy::schema::ExecutionPolicy> for ExecutionEcho { /* 3 field 写像 */ }
```

`ExecutionPolicy`(`schema.rs:112-124`)は `concurrency` 等他 field を
持つが record contract は 3 field 固定 — `From` impl に射影を集約し、
policy struct の成長が record shape に漏れないようにする。

### 2. `usage_log.rs` — `echo` 引数と `execution` key 挿入

- `build_usage_record`・`record_usage` に `echo: Option<&ExecutionEcho>`
  引数を追加(位置は `now` 引数の手前 — plan §Open Questions で
  確定)。
- `Some(echo)` のとき、`repairs`(T1)の直後に
  `execution: {"timeoutMs": <u64>, "maxTurns": <u64|null>,
  "maxToolCalls": <u64>}` を挿入 — 3 key 常時、`maxTurns` のみ
  `None` → `null`。`echo` が `None` なら `execution` key 自体を
  省略するだけ(fail-soft、record 書き込みは落とさない)。
- `kind:"count"` record・`kind:"route"`/`"route-refetch"` record は
  対象外 — `echo` は run 分岐でのみ参照する。

### 3. `cli.rs` — `RunCtx` field と 3 呼出し箇所

- `RunCtx`(:302-308)に `execution_echo:
  Option<crate::usage_log::ExecutionEcho>` field を追加 —
  `run_root`/`run_user_config_dir`/`worker_name` と同じ carrier に
  載せ、success arm と汎用 catch の両方が同じ値を参照できる
  既存境界に揃える。
- `record_usage` 呼出し 3 箇所の echo 実引数:
  - run success arm(:464 付近): `ctx.execution_echo.as_ref()`
  - 汎用 catch(:266 付近): `ctx.execution_echo.as_ref()`
  - count-tokens(:367 付近): `None`

### 4. `run.rs` — `restrict_policy` 後の echo 設定

- `restrict_policy`(:223-229)で得た `effective_policy` の直後に
  `ctx.execution_echo = Some((&effective_policy.execution).into());`
  を設定する。`effective_policy.execution` は `--timeout` narrow・
  policy 層 merge を通った**解決後**の値であり declared 値ではない
  (spec Req 2)。usage-bearing error は全て `restrict_policy` 以降の
  `run_worker` 内で発生するため、record 対象 run で echo が `None`
  になる経路は構造的に存在しないが、`Option` の fail-soft 規約は
  維持する。

### 5. 既存 test の signature 追随

- `tests/integration/usage_log.rs` の既存 `build_usage_record`
  呼出しに `None` を追加して compile を維持する(挙動の assert は
  T3 が担当 — 本 task では新 assert を追加しない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-run-error-record-observability.md` — 本 task
  が implement する plan。File Map・Dependency Graph の T2
- `docs/designs/run-error-record-observability.md`
  (DESIGN-034KqI29dOTRXERRPTp5gV)— 配線決定・`ExecutionEcho` 形・
  `execution` の値形・fail-soft 規約の AUTHORITATIVE 定義
- `docs/specs/run-error-record-observability.md`
  (SPEC-G1ALq0d5lyFIi0h43tZsZG)— Req 2(実効 execution echo)

## Dependencies

- `docs/tasks/extend-usage-log-error-record-diagnostic-fields.md`
  (TASK-034Kr8hGPsSchbau1iTWjt)— `execution` key の挿入位置は T1 の
  `repairs` の直後に固定されており、同一 file の順序依存がある。

## Affected Files

- `crates/cheaper-cli/src/usage_log.rs`(modify — `ExecutionEcho`・
  `From`・`echo` 引数・`execution` 挿入)
- `crates/cheaper-cli/src/cli.rs`(modify — `RunCtx` field・3 呼出し)
- `crates/cheaper-cli/src/run.rs`(modify — echo 設定 1 行)
- `crates/cheaper-cli/tests/integration/usage_log.rs`(modify —
  call site の signature 追随のみ)

## Done When

- [ ] `ExecutionEcho` + `From<&ExecutionPolicy>` が `usage_log.rs` に
      存在し 3 field のみ写す
- [ ] `build_usage_record`/`record_usage` が `echo` 引数を受け、
      `Some` のとき `execution` を `repairs` 直後に、
      `None` なら省略する
- [ ] `RunCtx.execution_echo` field が存在する
- [ ] run 経路の 2 呼出しが `ctx.execution_echo.as_ref()`、
      count-tokens が `None` を渡す
- [ ] `run.rs` で `restrict_policy` 直後に `ctx.execution_echo` が
      設定される
- [ ] workspace が `--all-targets` で compile し既存 test が pass
      する(signature 変更の全呼出し箇所が追随済み)

## Out of Scope

- 新しい test case・assert の追加(T3)
- `kind:"count"`/`"route"`/`"route-refetch"` record への echo
- docs(`usage.md`/`AGENTS.md`)の更新(T4)

## Verification

- [ ] `mise exec -- cargo check -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0
- [ ] `git grep -n "execution_echo" -- crates/cheaper-cli/src` が cli.rs・run.rs の両方に match する

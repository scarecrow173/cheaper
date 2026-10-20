---
id: "IMPL-034JhCuYprFfcFcaGX6qsA"
type: "impl"
status: "completed"
title: "Add workflow job when condition evaluation"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-workflow/src/schema.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/types.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/bindings.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/compiler.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/engine.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/monitor.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/compile_contracts/compiler.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/engine.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/resource_inference.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-workflow-when.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
  experiments:
    adopted: []
    rejected: []
---

# Add workflow job when condition evaluation

## Summary

job `when: {left, op, right}` predicate を schema・型・compiler・
engine へ実装。`$from`/`$input`/`$literal` binding をそのまま
operand に使い、`when` 内 `$from` は compile 時 dep edge 推論に
参加する。when-false は `Skipped` + `CONDITION_SKIPPED` で
dependent へ `dependency_failed` を伝播せず、condition-skipped
producer への `$from` は `null` に materialize する。

## Context

Task `add-workflow-when`(plan Phase A4、design §workflow `when`
R21-R25/D4)。routed-exec builtin workflow が
`{left: {$from: exec, path: /details/outputTokens}, op: gt}` で
analyze job を条件実行するための前提機構。

## Implementation

### Changes

- `schema.rs`: `when_predicate()` — `oneOf` で `exists` は
  `right` 禁止・`eq|ne|gt|lt` は `right` 必須を表現(if/then/not
  は strict-mode compiler が拒否するため `oneOf` 2 分岐)。
  `job_schema` へ `when` 追加。`validate_definition` の
  `references()` へ `job.when` を渡し dep edge 推論・pointer
  schema 検証・unknown `$from`/`$input` 拒否に参加。
- `types.rs`: `WorkflowJob.when: Option<Value>`(raw JSON、
  operand binding は未展開)+ `parse_job` で parse。
  `CompiledJob.when` + Serialize 対応(`when?` は省略)。
  `WorkflowJobState.condition_skipped: bool`。
- `bindings.rs`: `MaterializeOptions.skipped_from:
  Option<&HashSet<String>>` 追加。`Binding::From` で
  `results.get` 不在かつ集合に含まれる producer は binding 式
  全体を `Value::Null` に materialize(path 解決を行わない)。
  集合外の result 不在は従来どおり `missing job result` error。
- `compiler.rs`: `materialize_job_input` に `skipped` 集合引数を
  追加。`skipped_of(states)` を `materialize_output`/
  `materialize_outputs` から `output_value` へ配線。
- `engine.rs`: `evaluate_when()` — `exists` は非 null
  materialization(producer skipped/result 不在・pointer 未解決
  は `false`)、`eq`/`ne` は JSON 等価、`gt`/`lt` は両 operand が
  number のときのみ数値比較(非数値は `false` へ安全側に倒す)。
  `pump()` で `dependency_failed == false` かつ `when` ある job を
  admission 前に評価。when-false → `condition_skipped` 集合へ登録
  + `finish_job(Skipped, CONDITION_SKIPPED)`。binding 解決失敗は
  silent skip でなく job failure。`finish_job` の
  `dependency_failed` 伝播を `CONDITION_SKIPPED` で抑止。
  states 構築時に `condition_skipped` flag を populate。workflow
  error 集約から condition-skipped job を除外。

### Decisions During Implementation

- `when` は raw `Value` として保持 — operand binding は
  materialize 時に解決するため専用型を導入せず既存 binding 語彙に
  載せた(表現ドリフト回避)。
- `CONDITION_SKIPPED` の判定は outcome diagnostic code で行う
  (`JobStatus::Skipped` は `DEPENDENCY_FAILED` との共有のため
  status だけでは区別不能)。`dependency_failed` 非伝播・error
  集約除外・null materialization の 3 箇所で同一 signal を使う。
- `exists` は `$from` operand の pointer 未解決を「存在しない」=
  `false` と解釈する一方、`eq`/`ne`/`gt`/`lt` の operand
  materialize 失敗は job failure — compile 時検査を通過した参照が
  runtime で失敗するのは engine 不変条件違反のため。
- runtime binding failure の test は `details`(optional・open
  schema)配下の pointer を使用 — closed schema の不可能 path は
  compile 時 `validate_pointer_schema` が先に拒否するため、
  runtime 失敗経路は schema 上不可能と証明できない pointer でのみ
  到達可能。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-workflow --test engine when` — 5 pass
  (新規: `when_false_skips_without_failure_and_materializes_null`・
  `when_true_runs_job`・`when_from_creates_dependency_edge`・
  `when_numeric_operators_safely_evaluate`・
  `when_binding_failure_is_job_failure`)。
- `cargo test -p cheaper-workflow` — 全 suite pass(lib 2・
  compile_contracts 45・engine 27・filesystem_contracts 43・
  process 1)。新規 compile test `when_schema_and_dependency_
  inference` を含む。
- `cargo clippy -p cheaper-workflow -p cheaper-tools --tests` —
  warning なし(A1 由来の `map_or` warning も `is_none_or` へ修正)。

## Risks

- `WorkflowJobState` に `condition_skipped` field を追加 —
  外部 consumer が struct literal で構築している場合は更新が必要
  (workspace 内は全て更新済み)。
- `materialize_job_input` の signature に `skipped` 集合が追加
  された — crate 外 caller は存在しない。

## Follow-ups

- D2 `add-builtin-route-resources` で routed-exec.workflow.json の
  `when: {left: {$from: exec, path: /details/outputTokens}, op: gt,
  right: {$input: tokenThreshold}}` を接続。

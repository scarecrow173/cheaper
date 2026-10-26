---
id: "IMPL-034MCFKrUMUf5W5lyv1eWt"
type: "impl"
status: "completed"
title: "Remove ambient Node from workflow fixture"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/remove-ambient-node-from-workflow-fixture-issue-188.md"
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

# Remove ambient Node from workflow fixture

## Summary

`cheaper-workflow` の producer fixture から ambient Node 探索と Node 不在時の
silent skip を取り除いた。workflow integration test はテストバイナリを fixture
child として再実行し、Node 不在の明示 PATH 下でも全契約検証を通過する。

## Context

Task `TASK-034MCBOZ7xsFWR2eROLiV5` に従い、issue #188 の承認済み plan / design /
spec の範囲で workflow producer・consumer・resource 契約を保った。

## Implementation

### Changes

`crates/cheaper-workflow/tests/process.rs` で Node resolver と silent skip を削除し、
`current_exe` を `--exact workflow_fixture_child --nocapture` で呼び出す process
fixture を追加した。fixture は env marker `CHEAPER_WORKFLOW_PROCESS_FIXTURE=generate`
がある場合に `out.txt` へ `generated` を書く。process policy は `inputs: []` と
`outputs: ["out.txt"]` を宣言する。producer/consumer の compile・materialize・
execute・読取アサーションと resource inference の既存検証は維持した。

### Decisions During Implementation

fixture child は同一 integration test executable を再利用した。通常の test harness
実行時は marker が無いため fixture 関数は何もせず、process task からの起動時は
`generate` marker を検証してから宣言出力を書き込む。script を実行しないため
policy の `inputs` は空にした。

## Related Experiments

### Adopted

該当なし。

### Rejected

該当なし。

## Validation

成功: `mise exec -- cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact`
(Windows、1 passed)。

成功: `mise exec -- cargo fmt --check -p cheaper-workflow`。

成功: WSL で `rustup which cargo` / `rustup which rustc` が返した
`/home/akiyoshi/.rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/bin` と標準
system directories のみを process-local `PATH` に設定し、`cargo` と `rustc` の
解決先を確認した。`command -v node` は見つからず、その環境で同じ exact-filter
test を実行して 1 passed。

## Risks

なし。fixture は shell/Node に依存せず、workflow contract のテスト範囲に限定される。

## Follow-ups

なし。本IRはこのTask 2の実装・検証結果を記録する。

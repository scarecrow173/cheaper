---
id: "TASK-gkjJbivCKNLqicYwSJj3x1"
type: "task"
status: "done"
title: "execution.maxTurns の Option<u64> 化と built-in 既定の null 化"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/111"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  blocks:
    - "docs/tasks/budget-exhaustion-checkpoint-persistence.md"
    - "docs/tasks/run-resume-cli.md"
    - "docs/tasks/issue-111-docs-and-final-verification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "docs/adr/nullable-execution-maxturns-unbounded-by-default.md"
  defers: []
  deferred-by: []
---

# execution.maxTurns の Option<u64> 化と built-in 既定の null 化

## Work

1. `crates/cheaper-core/src/policy/schema.rs` — `ExecutionPolicy.max_turns` を
   `Option<u64>` 化。schema を `{"type": ["integer","null"], "minimum": 1}` へ。
   正規化: `null`/欠落 → `None`、非 null 不正値 → `Some(0)` で残留
   （schema 検証が本来弾く経路）
2. `crates/cheaper-cli/resources/policy.yaml` — `maxTurns: 8` → `null`、
   `timeoutMs: 120000` → `300000`
3. `Option<u64>` 伝播: `SessionSpec.policy_max_turns`、repair options、
   `fake_provider` の recorded spec、`fixture.rs` の `default_policy()` を
   `None` 基準に。bounded が必要な既存 test は明示 `Some(N)`
4. 内部ガード計算は
   `policy_max_turns.map(|p| usize::try_from(p).unwrap_or(usize::MAX)).unwrap_or(internal)`
   を維持（`p as usize` 禁止）。`DEFAULT_MAX_TURNS=1024` 不変、
   内部 breach は `INTERNAL_ERROR` のまま。`MaxTurnsError` メッセージを
   「policy abort should have fired first」と誤解しない文言へ修正
5. `crates/cheaper-core` に built-in `policy.yaml` を load して
   `max_turns == None` / `timeout_ms == 300_000` を pin する drift test
   （`common::builtin_dir()` 経路）
6. golden fixture（`crates/cheaper-core/tests/golden/fixtures/policy/load.json`）
   と既存 test builder の `max_turns` を新契約へ追随

## Done criteria

- T1: `maxTurns: null`（既定）で複数 turn を要する fixture run が完走
- T2: 整数 `maxTurns` 指定で従来どおり `MAX_TURNS_EXCEEDED` +
  `details.reason="maxTurns"`
- T5: schema が integer/null を受理し 0・負・文字列を拒否
- built-in policy drift test が pass
- `cargo test -p cheaper-core -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` pass

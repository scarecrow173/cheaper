---
id: "IMPL-UAM3loJyQptDIIrWiEJ3me"
type: "impl"
status: "completed"
title: "issue-111: nullable maxTurns・stall timeout・budget 枯渇 checkpoint/resume の実装"
created: "2026-10-17"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/111"
  changes:
    added:
      - "crates/cheaper-runtime/src/run/checkpoint.rs"
      - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
      - "docs/adr/execution-timeout-as-progress-reset-stall-window.md"
      - "docs/adr/nullable-execution-maxturns-unbounded-by-default.md"
      - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
      - "docs/discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
      - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
      - "docs/specs/worker-run-nullable-maxturns-and-stall-timeout.md"
      - "docs/tasks/budget-exhaustion-checkpoint-persistence.md"
      - "docs/tasks/issue-111-docs-and-final-verification.md"
      - "docs/tasks/nullable-maxturns-policy-model-and-propagation.md"
      - "docs/tasks/run-resume-cli.md"
      - "docs/tasks/run-stall-timeout-progress-window.md"
    modified:
      - "crates/cheaper-cli/resources/policy.yaml"
      - "crates/cheaper-cli/src/args.rs"
      - "crates/cheaper-cli/src/cli.rs"
      - "crates/cheaper-cli/src/run.rs"
      - "crates/cheaper-cli/src/workflow_run.rs"
      - "crates/cheaper-cli/tests/integration/args.rs"
      - "crates/cheaper-cli/tests/integration/policy.rs"
      - "crates/cheaper-cli/tests/integration/run.rs"
      - "crates/cheaper-cli/tests/regression/expected/run-max-tool-calls.json"
      - "crates/cheaper-cli/tests/regression/expected/run-max-turns.json"
      - "crates/cheaper-cli/tests/regression/normalize.rs"
      - "crates/cheaper-cli/tests/run_dev_seam.rs"
      - "crates/cheaper-core/src/policy/capability.rs"
      - "crates/cheaper-core/src/policy/restrict.rs"
      - "crates/cheaper-core/src/policy/schema.rs"
      - "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - "crates/cheaper-rig-adapter/src/error.rs"
      - "crates/cheaper-runtime/src/dev/fake_provider.rs"
      - "crates/cheaper-runtime/src/dev/fixture.rs"
      - "crates/cheaper-runtime/src/run/abort.rs"
      - "crates/cheaper-runtime/src/run/boundary.rs"
      - "crates/cheaper-runtime/src/run/mod.rs"
      - "crates/cheaper-runtime/src/run/repair.rs"
      - "crates/cheaper-runtime/src/run/session.rs"
      - "crates/cheaper-runtime/tests/fixtures/05-timeout-delay.json"
      - "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - "crates/cheaper-runtime/tests/run_fixture_test.rs"
      - "docs/guides/configuration.md"
      - "docs/guides/usage.md"
      - "docs/specs/tool-operator-built-in-worker.md"
      - "packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md"
      - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "docs/adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "docs/adr/execution-timeout-as-progress-reset-stall-window.md"
    - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
    - "docs/discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
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

# issue-111: nullable maxTurns・stall timeout・budget 枯渇 checkpoint/resume の実装

## Summary

Issue #111(bulk-reader が low-tier model で MAX_TURNS_EXCEEDED/TIMEOUT
で結果を返さない)の対応。`execution.maxTurns` を `Option<u64>` 化して
既定を `null`(policy レベルの turn 上限なし、内部 runaway guard のみ)
とし、run 経路の `timeout` を進捗 RunEvent でリセットされる stall
window(既定 300s)へ変更した。明示的な bounded budget(maxTurns/
maxToolCalls)枯渇時には tool 観測を `<userConfigDir>/checkpoints/
run-<id>.json` へ atomic 保存し、envelope 最外層の
`error.details.checkpoint={path,observationsCount}` で参照を返す。
`cheaper run --resume <path>` で新 session/budget の continuation run
として再開でき、観測は連鎖 resume で累積する。workflow 経路の timeout
は従来どおり absolute deadline。

## What was implemented

- **nullable maxTurns** — `ExecutionPolicy.max_turns: Option<u64>`、
  schema `"type": ["integer","null"]` + minimum 1、builtin policy
  `maxTurns: null`。session/repair/fake/restrict/capability の全使用
  箇所を Option 対応。内部 guard は None→1024・bounded は最低 1024 に
  正規化。
- **stall timeout** — `boundary.rs` の deadline loop が
  `Progress.last() + timeout` で起床→最終進捗で再判定→無言なら
  `TIMEOUT` abort。`Progress` は `tokio::time::Instant` ベースで、
  on_event funnel と repair event sink の両方が `note` する。
- **checkpoint 保存** — `run/checkpoint.rs` 新設。`ToolCommitted` と
  `ToolResult` を call_id で join して観測化(admission 拒否・
  rolled-back call は除外)、result 抜粋 2000 文字/entry・合計
  ~100KB cap。`MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED`(直・
  PARTIAL_MUTATION wrap 両方)で temp+rename atomic write + mode
  0o600。save 失敗は best-effort で code/exit を変えない。timeout・
  signal abort・workflow job は対象外。
- **`--resume`** — `run`/`bulk-read` 等の alias で受理、非 run
  command は明示 reject。checkpoint は schemaVersion==1・基本型・
  worker 一致を検証(不一致は INVALID_INPUT/exit 2)。`--input` 明示
  (`-` 含む)は stored input を上書き、未指定は stored input を使い
  stdin を読まない。user prompt 末尾に continuation section
  (abort reason・counters・観測・継続指示)を注入し、新しい
  session/policy/budget で実行。grant は checkpoint に非永続。
- **fake provider** — scripted `ToolCalls` が Rig 実順序
  (`ToolCall`→`ToolCommitted`(admitted のみ)→実行→`ToolResult`)を再現。
  admission 判定は `ToolBudget::tool_calls` 差分。
- **regression 対応** — `normalize.rs` に checkpoint path の
  run-id scrub(`run-<id>.json`)。`run-max-turns` /
  `run-max-tool-calls` の expected に `details.checkpoint` を追加。
  `05-timeout-delay` fixture は setup-boundary 発火の非対称(details・
  meta.model 欠落)を許容する期待へ調整(unit pin で契約は担保)。
- **docs** — configuration.md(nullable/stall/run vs workflow/
  transientAttempts 範囲/checkpoint・--resume・grant 非永続・
  input 変更時の stale context 警告)、usage.md(option 表)、
  cheaper-usage / cheaper-bulk-read SKILL.md、tool-operator spec の
  旧既定値(8/120s)表記を更新。

## Verification

- `cargo test -p cheaper-runtime --features dev-seam --test
  run_dev_seam`: 28 pass(nullable/stall/checkpoint/resume/連鎖累積/
  timeout 非 checkpoint 化を含む)
- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,
  cheaper-cli/dev-seam --test integration`: 185 pass(args parse・
  builtin policy pin・resume parse/alias/非 run reject 含む)
- `--test run_dev_seam`(CLI 側): resume checkpoint load/input
  precedence/worker mismatch/不正 file reject 4 件 pass
- `--test regression`: 61 case pass(checkpoint path scrub + golden
  fixture 更新済み)
- `cargo test -p cheaper-core --test golden`: policy load/schema
  corpus pass(builtin 埋め込みを新 policy に再同期、maxTurns null
  accept・0/string reject case 追加)
- `mise run check`: 全ステージ pass(biome/fmt/clippy/version/
  doc-id/workspace test/node test/lockfile/payload)

### 途中で潰した flake

- `timeout_does_not_reclassify_existing_signal_reason` — signal spawn
  の sleep 起点と stall window 起点のずれで高負荷時に Timeout が
  先勝ちし得たため signal を即 abort 化して確定的に先勝ちにした。
- `05-timeout-delay` — setup boundary 中の abort は code のみ復元
  (details/meta.model 欠落)する設計のため、fixture 期待を同範囲に
  合わせた(実装側の対称性強制は行わない — TS parity 契約)。

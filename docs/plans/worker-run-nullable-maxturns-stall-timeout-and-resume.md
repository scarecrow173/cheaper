---
id: "PLAN-enjTBUdUwFPP2aCo1QGmER"
type: "plan"
status: "approved"
test-spec-skip: "Verified by cheaper-runtime dev-seam suite (28 tests incl. nullable maxTurns/stall window/checkpoint/resume/chained accumulation), CLI integration+regression corpus (61 golden cases), golden policy corpus, and mise run check; no dedicated test-spec doc authored."
title: "execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume の実装計画"
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
    - "docs/specs/worker-run-nullable-maxturns-and-stall-timeout.md"
  implemented-by: []
  depends-on:
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "docs/adr/execution-timeout-as-progress-reset-stall-window.md"
    - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
    - "docs/discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "docs/designs/worker-run-nullable-maxturns-and-stall-timeout.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "docs/adr/execution-timeout-as-progress-reset-stall-window.md"
    - "docs/adr/budget-exhaustion-checkpoint-and-resume.md"
  defers: []
  deferred-by: []
---

# execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume の実装計画

## Goal

issue #111 の本筋: low-tier model の `bulk-reader` が `MAX_TURNS_EXCEEDED` /
`TIMEOUT` で結果を返さない問題を、3 つの独立した機構で解消する。

1. `execution.maxTurns` を `Option<u64>` 化し、`null` = policy レベルの
   turn 上限なし。built-in 既定を `maxTurns: null` に変更
2. `cheaper run` の `execution.timeoutMs` を「最後の進捗 `RunEvent` からの
   stall window」へ変更（workflow の absolute deadline は維持）
3. 明示的な budget 枯渇（`MaxTurns` / `MaxToolCalls`）時に checkpoint を
   保存し `--resume <path>` で再開可能にする

## Scope

| In | Out |
|---|---|
| `crates/cheaper-core` policy schema / `ExecutionPolicy` 型 | workflow 経路の timeout 意味論（absolute 維持） |
| `crates/cheaper-runtime` run loop / boundary / session / repair / dev fixture | Rig 内部の `DEFAULT_MAX_TURNS` 内部ガード値（1024 維持） |
| `crates/cheaper-cli` args / run / envelope / normalize / policy.yaml | workflow job abort の checkpoint 化（run 経路のみ対象） |
| 回帰 fixture・golden・unit/integration test | checkpoint の retention/TTL・清掃機構 |
| `docs/guides/configuration.md`、`cheaper-usage`/`cheaper-bulk-read` SKILL.md | `maxDuration` 型の絶対時間上限 knob（別途検討） |

## File Map

- `crates/cheaper-core/src/policy/schema.rs` — `max_turns: Option<u64>` 化、
  schema `{"type": ["integer","null"], "minimum": 1}`、null/欠落 → `None`、
  非 null 不正値は `Some(0)` で残留（schema 検証が本来弾く）
- `crates/cheaper-core/src/policy/load.rs` — built-in `policy.yaml` を
  load して `max_turns == None` を pin する drift test（`common::builtin_dir()`）
- `crates/cheaper-cli/resources/policy.yaml` — `maxTurns: 8` → `maxTurns: null`、
  `timeoutMs: 120000` → `300000`
- `crates/cheaper-runtime/src/run/boundary.rs` — 単発 `sleep(timeout)` を
  `last_progress + timeout` の deadline loop へ（`on_event` で進捗通知、
  repair event も通知）
- `crates/cheaper-runtime/src/run/mod.rs` — budget abort 時の checkpoint
  保存 + envelope 最外層 error への `details.checkpoint` 付与
  （`PARTIAL_MUTATION` wrap 時は wrapper 側）
- `crates/cheaper-runtime/src/run/session.rs` / `repair.rs` /
  `dev/fake_provider.rs` / `dev/fixture.rs` — `Option<u64>` 伝播、
  fixture `default_policy()` `max_turns: None`
- `crates/cheaper-cli/src/args.rs` / `run.rs` — `--resume <path>`、
  非 run コマンドでの拒否、worker 一致検証、input 優先順位
  （明示 `--input` > checkpoint input）、stdin スキップ
- `crates/cheaper-cli/tests/regression/normalize.rs` — checkpoint path
  の run-id 部分を scrub
- `tests/regression/cases/run-max-turns.json` / `run-max-tool-calls.json`
  および `expected/` — `details.checkpoint` を含む期待値へ更新

## Verification Matrix

| 変更 | 検証 |
|---|---|
| `maxTurns: null` で multi-turn 完走 | T1: run_dev_seam fixture test |
| 整数 `maxTurns` で `MAX_TURNS_EXCEEDED` 維持 | T2: 既存回帰 fixture + unit |
| 進捗 event で stall timer reset | T3: 500ms window / 50ms 間隔 emit |
| 無進捗で `TIMEOUT` | T4: fixture test |
| schema の integer/null 受理・拒否 | T5: cheaper-core golden + unit |
| checkpoint 保存 + 最外層 details | T6: dev-seam test + 回帰 fixture |
| resume で観測注入・input 優先・worker 一致 | T7/T9: CLI integration test |
| chained resume の観測累積 | T8: dev-seam test |
| 全体回帰 | `mise run check`（dev-seam feature 付き） |

## Risks

- `Option<u64>` 伝播で `usize` 変換が 32bit truncate →
  `usize::try_from(p).unwrap_or(usize::MAX)` を維持（design 記載済み）
- stall timeout が発火経路（session 中 vs setup boundary）で details 形状が
  異なる既往の経路非対称 → fixture は `errorDetails` を期待しない
  （PR #109 で確定済みの方針）
- `--resume` 時の checkpoint worker 名と `--worker-file` の解決名不一致 →
  resolved `def.name` で比較
- checkpoint の write 失敗は best-effort — abort 分類を変えず
  `details.checkpoint` を付けないだけ

## Rollback

単一ブランチ内の論理コミット単位で revert 可能。`maxTurns` nullable 化は
schema/型変更を含むため単独 revert では既存 checkpoint/resume コードと
整合しなくなる点に注意（resume 部分のみ revert する場合は checkpoint 保存を
無効化しても既存挙動は維持される）。

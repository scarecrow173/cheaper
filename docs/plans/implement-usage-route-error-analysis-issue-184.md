---
id: "PLAN-034M8k2N0x4RKepPhz3J4M"
type: "plan"
status: "approved"
title: "Implement issue 184 usage route and error analysis"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
  changes:
    added:
      - type: "file"
        path: "docs/plans/implement-usage-route-error-analysis-issue-184.md"
      - type: "file"
        path: "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
      - type: "file"
        path: "docs/tasks/pin-usage-route-error-contract-tests.md"
      - type: "file"
        path: "docs/tasks/implement-usage-route-window-and-by-route.md"
      - type: "file"
        path: "docs/tasks/implement-usage-by-error-aggregation.md"
      - type: "file"
        path: "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
      - type: "file"
        path: "docs/tasks/measure-usage-jsonl-peak-memory.md"
      - type: "file"
        path: "docs/tasks/verify-usage-analysis-chain-and-audit.md"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
  implements:
    - "docs/specs/usage-analysis-scope-issue-184.md"
  implemented-by:
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
  derived-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  verifies: []
  verified-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  references:
    - "docs/discovery/usage-analysis-audit-issue-184.md"
    - "docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "docs/guides/usage.md"
    - "docs/impl/exp/README.md"
  defers: []
  deferred-by: []
---

# Implement issue 184 usage route and error analysis Implementation Plan

## Goal

2026-10-26 改訂: #192 境界を追加した amendment は独立 review と Judge 判定を
経て承認済み（下記 approval checkpoint）。先行 spec/design/plan の承認と
ADR の O2 採択は維持する。T1-T6 の実装・検証と文書 closeout が完了した。

Implement issue #184's route-window filtering, per-route and per-error usage
reports, CLI flags, and text/JSON output while preserving existing run filters,
usage record schema, privacy allowlists, and no-flag report behavior.

## Scope

One implementation stream: parser flags, report aggregation, text formatting,
integration coverage, and user documentation share one CLI contract and one
release intent. Keep writer/schema changes, new dependencies, and streaming
aggregation out of scope.

## Upstream Documents

- `docs/specs/usage-analysis-scope-issue-184.md` (amendment approved)
- `docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md` (accepted)

## Design Inputs

- `docs/designs/usage-grouped-route-error-analysis-issue-184.md` (amendment approved)

## File Map

- `crates/cheaper-cli/src/args.rs` — `--route`/`--group-by` parse, usage-only
  option guard, exact error contracts.
- `crates/cheaper-cli/src/usage_report.rs` — route window and route filter,
  owner join/orphan logic, unkeyed counting, per-route and by-error aggregates,
  existing run aggregation and report callers.
- `crates/cheaper-cli/src/cli.rs` — usage help text and parsed option handoff.
- `crates/cheaper-cli/src/text_format.rs` — route section and flag-gated
  grouped sections with JSON parity.
- `crates/cheaper-cli/tests/integration/args.rs` — CLI option and exact guard
  message tests.
- `crates/cheaper-cli/tests/integration/usage_report.rs` — report JSONL
  fixtures, contract edges, privacy, compatibility, and report formatting.
- `crates/cheaper-cli/tests/integration/usage_log.rs` — existing report
  builder callers and output contract updates.
- `crates/cheaper-cli/tests/integration/main.rs` — module declarations only
  if new integration test modules are needed; prefer existing modules.
- `docs/guides/usage.md` — flags, route/error sections, windows, denominator
  meanings, and measured versus estimated values.
- `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl` — bounded
  representative large JSONL fixture dimensions and peak-memory measurement
  evidence as an `experiment_event.v1` note (`benchmarks/results/` is
  reserved for `cheaper-bench` harness `schemaVersion: 2` run artifacts and
  does not fit ad-hoc measurement notes — convention confirmed via
  `benchmarks/results/README.md` and `docs/impl/exp/README.md`).

## Implementation Sequence

1. Add failing focused tests for route window/scoping/join, error grouping,
   CLI guards, privacy, legacy behavior, nonfinite values, and text parity;
   include the omitted edge cases listed in the Verification Matrix below.
2. Implement route window filtering and grouped route aggregation in
   `usage_report.rs`, including string-route owner requirements, window-wide
   orphan/unkeyed diagnostics, and the documented `format_number` key path.
3. Implement error-code aggregation and canonical execution value maps while
   preserving empty error codes and existing failure denominator semantics.
4. Wire CLI flags/help and exact validation messages; update all report
   builder callers found in `usage_report.rs`, `usage_report.rs` tests, and
   `usage_log.rs` tests.
5. Add text sections and user guide documentation; verify flag-gated JSON/text
   parity and no-flag backward compatibility.
6. Generate a bounded, representative large JSONL input, measure peak memory
   of current full-file parsing with a documented reproducible command, and
   record fixture size, record mix, environment, and result in the
   `docs/impl/exp` measurement note before deciding whether streaming
   remains deferred.
7. Run focused tests, then canonical `mise run check`; review the final diff
   for scope, privacy, doc-index, and contract consistency.

## Dependencies and Risks

- 本 #192 境界改訂の review/approval は完了。実装は T1 から別 dispatch とし、
  この文書 closeout では開始しない。
- No dependency changes are planned. Existing `usage.jsonl` writer schema and
  ordering remain untouched.
- Route totals change under `--since` and `--route`; tests and the guide must
  pin this observable behavior.
- Fake records can reuse a `logRef`; first window record wins and must not
  double-attribute a refetch.
- Error detail and command/path fields are sensitive; aggregation keys and
  output must remain restricted to the design allowlist.
- Large JSONL memory results may show full-file parsing is not viable; preserve
  the recorded evidence and route any streaming redesign back through design.
- External verification dependency risk: `mise`-managed tools must be
  available; no live provider or network access is required for this feature.
- #184/#192/#185 に hard implementation dependency はなく別 PR とし、
  提案順は #184 → #192 → #185。#184 は Usage consumer のみを扱い、
  #192 の必須 new Usage ID 連携・ID 発行/伝搬・History store は実装しない。
  `usage --execution-id` は #192 の optional candidate に留める。

## Task Breakdown

- T1: Pin CLI args and report contract tests (args, report, writer callers).
  → docs/tasks/pin-usage-route-error-contract-tests.md
- T2: Implement route window, route scope, join, orphan/unkeyed, and per-route
  aggregates.
  → docs/tasks/implement-usage-route-window-and-by-route.md
- T3: Implement by-error aggregates and canonical execution value keys.
  → docs/tasks/implement-usage-by-error-aggregation.md
- T4: Implement CLI handoff, help, and text formatting; update guide and
  compatibility snapshots.
  → docs/tasks/wire-usage-group-by-flags-text-and-guide.md
- T5: Measure representative large JSONL peak memory and record evidence;
  decide whether streaming remains deferred.
  → docs/tasks/measure-usage-jsonl-peak-memory.md
- T6: Run focused and canonical verification, doc-status checks, and final
  scope/privacy audit.
  → docs/tasks/verify-usage-analysis-chain-and-audit.md
All implementation tasks depend on T1's regression expectations, but the
chain is strictly serialized: T1 → T2 → T3 → T4 → T5 → T6. T2 and T3 both
edit the same `build_usage_report` tail and the same
`tests/integration/usage_report.rs` file, so T3 depends on T2 (parallel
execution would guarantee a merge conflict). T4 depends on T2/T3. T5's
verification (`cargo test … usage_report::`) cannot go green until T4 lands
the last pin tests, so T5 depends on T4. T6 depends on all implementation
and evidence tasks as the chain gate. Landing model: all six tasks land on
`feature/usage-analysis-issue-184` before merge — `mise run check` is
expected-red between T1 and T4 by TDD pin-test design (new pins fail while
their implementation is still pending), and the canonical check is T6's
gate, not a per-task gate. The behavior test spec is
docs/test-specs/usage-route-error-analysis-contract-issue-184.md, linked from
this plan's `relations.verified-by` and from each task's `verified-by`.

## Verification

### #192 consumer tolerance acceptance（T1 pin → T6 gate）

test-spec の同名 5 test を既存 Usage harness で追加・確認する。

| test | acceptance |
| --- | --- |
| `usage_optional_ids_preserve_report` | 同一 Usage records の optional executionId/parentExecutionId/traceId の有無で aggregate/group/filter/refetch 出力が同一 |
| `usage_same_trace_distinct_runs_counted` | 同一 trace の異なる run 2 件を両方計数し、recorded-run 失敗率の母数を維持 |
| `usage_refetch_distinct_execution_id_joins_log_ref` | 元 Route と独立 ID の refetch も window 内 logRef owner に join |
| `usage_window_orphan_not_backfilled_from_history` | window 外 owner は、対応 History 相当の外部情報があっても backfill せず orphan |
| `usage_report_history_independent` | History 不在で report が成立し、無関係な外部ファイルがあっても不変。History records は入力でない |

consumer tolerance のみであり #192 の ID 妥当性/発行/連携 test ではない。
History schema/file path を仮定せず、必要なら既存 helper で無関係な inert
sentinel を置く（実 History writer を作らない）。詳細な fixture と filter
組合せは test-spec に従う。6 task の直列依存・T1〜T4 の expected-red window
は維持し、新規依存は追加しない。T5 メモリ計測は Usage JSONL のみで、
History store の容量/性能測定へ拡張しない。

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` exits 0 (65 passed).
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` exits 0 (47 passed).
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0 (21 passed).
- [x] `mise run check` exits 0.
- [x] A bounded representative large JSONL measurement records input dimensions and peak memory using a reproducible command; evidence is checked in as `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl` (`experiment_event.v1` note convention, per `docs/impl/exp/README.md`).
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` reports no new blocking findings for the issue #184 artifacts.
- [x] 上記 consumer tolerance 5 test が green で、writer/schema 無変更、
      History 非参照、recorded-run 母数、window-bound logRef join を確認。

## Open Questions / Gaps

- The representative dataset size and acceptable peak-memory threshold are
  not specified by the issue. Establish a bounded input anchored to the
  observed scale of a real `usage.jsonl` (naming the actual file used, or
  the recorded fallback assumption when none is available) and report
  measurement without inventing a pass threshold; if evidence indicates
  unacceptable memory use, return to design before deferring streaming.
- The measurement artifact destination is fixed: an `experiment_event.v1`
  note at `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl`
  (`benchmarks/results/` is the `cheaper-bench` harness's append-only
  `schemaVersion: 2` store and does not accept ad-hoc notes).

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation with review
  checkpoints after tests, implementation, and memory evidence.
- Work streams are serialized, not parallel: route (T2) and error (T3)
  aggregation share the `build_usage_report` tail and one test file, so T3
  follows T2; formatting/docs (T4) follows both; measurement (T5) is a
  bounded slice sequenced after T4; verification (T6) gates the chain.
- T1-T6 は承認済み plan の直列順で実行済み。T6 focused/canonical verification と
  doc-status audit は成功し、implementation-verified へ routing する。

## Review Handoff

- Review focus: confirm the two window-wide diagnostics, unkeyed route owner
  behavior, canonical number formatter path, all CLI/report callers, privacy
  allowlist, backward compatibility, and the bounded memory measurement
  evidence. 先行版承認後に test-spec/T1-T6 は作成済み。#192 境界と
  consumer tolerance fixture の追加差分も独立 review/approval 済み。

### 2026-10-26 graph repair checkpoint

文書 status を戻す前に、同一 focus
`docs/plans/implement-usage-route-error-analysis-issue-184.md` で以下を実行した
（共通 prefix: `mise exec -- node
.agents/skills/doc-driven-dev-graph/scripts/route_graph.js`）。

1. `--current planning --signal design-gap --focus <上記 path> --explain --json`
   → exit 0、`status: edge`、`edgeId: planning-to-design`、
   `condition: design-gap`、`next: design`、`blockers: []`。
   priority 10 の `planning-retry` は matched false。
2. `--current design --signal spec-gap --focus <上記 path> --explain --json`
   → exit 0、`status: edge`、`edgeId: design-to-briefing`、
   `condition: spec-gap`、`next: briefing`、`blockers: []`。
   priority 10 の `design-retry` は matched false。
3. draft 反映後、`--current briefing --focus <上記 path> --explain --json`
   （追加 signal なし）→ exit 0、`status: edge`、
   `edgeId: briefing-retry`、`condition: briefing-not-pass`、
   `next: briefing`、`blockers: []`。priority 10 の retry が matched true
   となり、未承認改訂の briefing 待機を確認した。

この時点の checkpoint は **briefing / pending amendment review** だった。
spec/design/plan を一旦 draft とし、test-spec は従来規約どおり draft、
T1-T6 は todo を維持した。`implementation-verified` は送信していない。

### 2026-10-26 amendment approval checkpoint

- 独立 review `A360297a4` / `B33c64c68`: ともに **NO candidates**。
- 別 Judge `24a337ec`: **ACCEPT WITH NON-BLOCKING NOTES**。
  改訂 spec/design/plan を approved に戻す明示的許可を受領した。
  唯一の note は本改訂に起因しない既存 ADR reciprocal warning
  （下記 audit 記録）であり、この改訂で修正する対象ではない。
- この許可に基づき spec/design/plan と対応 index を approved へ復帰。
  test-spec は確立済みの draft 規約を維持し、T1-T6 は todo のまま。
  これは amendment の承認記録であり、実装の検証結果ではない。

### 2026-10-26 approved amendment routing / closeout

上記承認の反映後、共通 prefix
`mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js`
に `--focus docs/plans/implement-usage-route-error-analysis-issue-184.md
--explain --json` を付け、追加 signal なしで **1 edge ずつ** 実行した。

| current | edgeId | condition | next | requiredAudits |
| --- | --- | --- | --- | --- |
| briefing | `briefing-to-design` | briefing | design | design |
| design | `design-to-planning` | design | planning | design |
| planning | `planning-to-task-graph` | planning | task-graph | plan, task, test-spec |

全 3 回が exit 0、`status: edge`、`blockers: []`。
実行済み design/plan/task/test-spec audit と承認済み既存文書を照合し、
新たな文書作成や domain 変更は不要と確認した。
task-graph 到達後に `mise exec -- node
.agents/skills/doc-driven-dev-graph/scripts/build_task_graph.js
--plan docs/plans/implement-usage-route-error-analysis-issue-184.md --json`
を実行し exit 0。6 nodes / 9 dependency edges、`issues: []`、
`active/resumableActive/completed: []`、
`runnable: ["TASK-034MbSQgdIQfx2nMRZ0RCz"]`（T1）のみ。
実際の dependency edge は T1→T2/T3/T5、T2→T3/T4、T3→T4、
T4→T5/T6、T5→T6 であり、6 task の直列順と一致する。
T2-T6 は先行 task 待ちで、task status は全て todo を維持した。

`mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/inspect_graph.js
--focus docs/plans/implement-usage-route-error-analysis-issue-184.md --format json`
も exit 0。definition issues / state blockers / hardBlockers は全て空。
bootstrap/briefing/design/planning は pass、implementation は
`implementation-verified`・`tasks-incomplete` により blocked。
followup-triage と exit-audit も未完了のまま。現在 checkpoint は
**task-graph / T1 runnable・未 dispatch** とし、ここで停止する。
implementation edge は選択せず、`implementation-verified` も送信していない。

文書検証（product code/test は変更・実行していない）:

- `mise exec -- node scripts/check-doc-ids.mjs`: exit 0、
  `doc id check OK: 1238 markdown files scanned`。
- `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js
  --type <type> --json`: spec/design/plan/test-spec/task/discovery と
  補足 adr を各実行し全て exit 0、type 全体でも blocking findings は 0。
  active #184 の spec/design/plan/test-spec/T1-T6/discovery の findings は 0。
  ADR `grouped-route-and-error-analysis-in-cheaper-usage.md` には既存の
  non-blocking `inconsistent-reciprocal-relation` が 1 件残る
  （implemented-by → design の implements に reciprocal link が無い）。
  他領域にも既存 warning はあるが、本改訂による新規 finding はない。
- `git diff --check`: exit 0。
- Rust test・`mise run check`・T5 計測は未実行であり、上記 Verification
  checkbox は未完了のまま。consumer tolerance の green もまだ主張しない。

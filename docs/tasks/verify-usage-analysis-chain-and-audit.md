---
id: "TASK-034MbTCpfWxoiFfeKTaOMs"
type: "task"
status: "todo"
title: "Run focused and canonical verification, doc-status audit, and final scope/privacy review (issue #184 T6)"
created: "2026-10-25"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  references:
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  defers: []
  deferred-by: []
---

# Run focused and canonical verification, doc-status audit, and final scope/privacy review (issue #184 T6)

## Work

T1-T5 の着地を canonical verification で締める — focused test 群・
`mise run check`・doc-status audit・最終 diff の scope/privacy/
doc-index 一貫性 review。plan Verification の全 checkbox をここで
検証する。

1. focused test:`usage_report::`・`args::`・`usage_log::` の
   module filter で、T1 の pin test 全件と既存 test が green。
2. `mise run check` 全段(biome・cargo fmt・clippy・version/
   doc-id check・cargo test --workspace・node --test・lockfile・
   slim payload check)。
3. doc-status audit(`node .agents/skills/doc-status/scripts/
   audit_docs.js --type plan|test-spec|task --json` を各 type で
   実行)で issue #184 artifacts(plan・test-spec・task ×6)に
   new blocking findings がないこと。
4. 最終 diff review:
   - spec Out of Scope の不侵犯 — usage.jsonl writer/record schema
     無変更・streaming 化なし・`d`/`w` duration suffix なし・
     `--worker`/`--provider`/`--model` の route record 非適用・
     workflow 由来の usage 観測なし。
   - privacy allowlist — group key・出力に `executed.argv`・
     `error.message`/`schemaErrors`/`checkpoint`/`errorMessage`・
     path 系・平文 command が無い(`argv_and_error_detail_not_
     leaked` の green と diff 目視の両面で確認)。
   - flag-gated additive — flag 無し JSON が regression golden
     `cli-usage-*.json` と byte 不変。
   - doc-index — docs/plans・docs/test-specs・docs/tasks の 3 README
     の index 行が canonical(ID・title・status・file が front
     matter と一致)。
   - guide — design §8 の doc-sync 一覧の反映漏れなし。
   - plan「Dependencies and Risks」の各項目が着地状態と矛盾しない
     こと(window-wide diagnostics・first-wins・canonical formatter・
     caller 追従・allowlist・golden 無変更・memory evidence)。

## #192 consumer tolerance gate（2026-10-26、改訂レビュー・承認済み）

- `usage_optional_ids_preserve_report`・
  `usage_same_trace_distinct_runs_counted`・
  `usage_refetch_distinct_execution_id_joins_log_ref`・
  `usage_window_orphan_not_backfilled_from_history`・
  `usage_report_history_independent` の 5 test が全て green。
  test-spec の定義に沿って、ID 有無の aggregate/group/filter/refetch parity、
  同一 trace の distinct run 計数、独立 refetch ID の logRef join、
  window 外 owner の orphan 維持、History 不在/非入力を確認する。
- fixture は既存 Usage harness と必要なら inert な無関係の sentinel のみ。
  将来の History schema/store/filename を仮定しない。#192 が所有する ID
  妥当性/発行/伝搬/必須 Usage ID 連携の検証をこの task へ持ち込まない。
- 最終 diff で writer/schema 無変更、History 非参照、recorded-run 母数を
  確認し、T5 は Usage-only メモリ計測のまま、新規依存なし、
  T1 → T2 → T3 → T4 → T5 → T6 の直列依存と T1〜T4 expected-red の
  履歴を維持していることを確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T6 slice・Verification matrix)
- docs/test-specs/usage-route-error-analysis-contract-issue-184.md (Guarantees の最終確認)

## Dependencies

- docs/tasks/wire-usage-group-by-flags-text-and-guide.md (T4 — 全実装)
- docs/tasks/measure-usage-jsonl-peak-memory.md (T5 — 計測証跡)

## Affected Files

- 変更なし(検証 task — 発見された問題は該当 task の file scope へ
  戻して修正する)

## Done When

- [ ] 上記 consumer tolerance 5 test と責務境界の audit 結果を記録

- [ ] plan Verification の全 checkbox が検証済み
- [ ] focused test 3 モジュールと `mise run check` が green
- [ ] doc-status audit が issue #184 artifacts に新規 blocking
      finding を報告しない
- [ ] 最終 diff で scope 逸脱・privacy 漏出・golden 変化・
      doc-index 不整合がないことを確認し、結果を本 task に記録

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0
- [ ] `mise run check` exits 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が issue #184 artifacts に新規 blocking finding を報告しない

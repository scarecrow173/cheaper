---
id: "TASK-034MbTvAS8BfQZNoBH12OQ"
type: "task"
status: "done"
title: "Measure representative large usage.jsonl peak memory and record evidence (issue #184 T5)"
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
    generated:
      - type: "file"
        path: "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
  implements:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
  blocks:
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
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
    - "docs/impl/exp/README.md"
  defers: []
  deferred-by: []
---

# Measure representative large usage.jsonl peak memory and record evidence (issue #184 T5)

## Work

spec/design の「Deferred Design Concerns」にある streaming/bounded
aggregation 移行判断の前段として、現行の全量 `Vec<Value>` 展開
(`read_usage_records` → `build_usage_report`)のピークメモリを、
bounded で代表的な大規模 usage.jsonl fixture で実測し証跡を残す。
本 task は計測と記録のみ — streaming 移行自体は行わない(移行の
要否は実測で判断し、要と出た場合も本 issue では開始せず design
へ差し戻す)。

1. fixture 生成 — 実運用で観測される usage.jsonl の規模を根拠に
   bounded な代表 JSONL を生成する(run/route/route-refetch の
   現実的 mix)。規模の anchor は実際の `usage.jsonl`
   (`<userConfigDir>/usage.jsonl`、既定 `~/.config/cheaper/
   usage.jsonl`)の観測値(件数・byte)を取り、計測 note 内で
   **使用した実 file を特定して記録**する — 実 log が環境に
   存在しない・参照できない場合は、採用した規模の fallback
   前提(何を根拠に件数・byte・mix 比を決めたか)を note に
   明記する。捏造はしない(plan Open Questions の
   「representative dataset size は issue 未指定」の扱いに従う)。
2. 計測 — 再現可能な command(`cheaper usage` 実行時の peak RSS
   計測、または `build_usage_report` 経路の同等計測)で peak
   memory を計測し、command・fixture 件数/byte・record mix・
   計測環境(OS/toolchain)・結果を記録する。pass/fail 閾値は
   捏造しない — 実測 fact のみを記録する(plan Open Questions)。
3. artifact 保存 — 計測証跡は
   `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl` へ
   `experiment_event.v1` schema の event log として記録する
   (`docs/impl/exp/README.md` の慣行 — `rig-poc-go-no-go-
   conditions-measurement.jsonl` 等の計測系先例と同じ形)。
   `benchmarks/results/` は `cheaper-bench` harness の
   `schemaVersion: 2` run artifact 専用の append-only store で
   あり、手動計測 note の置き場ではない(`benchmarks/results/
   README.md`・`benchmarks/README.md` で確認済み)。fixture
   生成 recipe(件数・byte・mix 比・生成手順)は note 本文へ
   inline 記述し、commit 対象の generator script は作らない —
   専用 script を commit する判断を取る場合のみ、その file を
   本 task の `relations.changes.generated` へ追記する。
4. 判定記録 — 実測結果に基づき「本 issue では全量 `Vec<Value>`
   展開を維持する」判断、または「運用閾値超過を示唆するため
   streaming 移行の revisit を design へ差し戻す」判断を、実測
   根拠つきで文書化する(Deferred Design Concerns の revisit
   trigger の発火条件確認)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T5 slice・Open Questions)
- docs/designs/usage-grouped-route-error-analysis-issue-184.md (Non-Functional Constraints・Deferred Design Concerns)
- docs/specs/usage-analysis-scope-issue-184.md (Deferred Design Concerns — streaming 移行の revisit trigger)

## Dependencies

- docs/tasks/pin-usage-route-error-contract-tests.md (T1 の fixture
  builder が record 形状を確定 — 計測 fixture は実 writer 互換の
  形状で作る)
- docs/tasks/wire-usage-group-by-flags-text-and-guide.md (T4 — 本
  task の Verification が `cargo test … usage_report::` の全件
  green を要求するため、最後の pin test が着地する T4 完了後に
  しか開始できない — plan の serialized chain 参照)

## Affected Files

- `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl`
  (`experiment_event.v1` の checked-in measurement note —
  `benchmarks/results/` は `cheaper-bench` harness 専用のため
  採用しない。fixture 生成 recipe は note 本文へ inline 記述)

対象外: `usage_report.rs` の streaming 化(実測で要否を判断する
前段であり、本 task では実装しない)。

## Done When

- [x] bounded 代表 JSONL(件数・byte・record mix を記録)が生成
      され、規模の anchor となった実 `usage.jsonl` の特定
      (または fallback 前提)と生成手順が再現可能な形で note
      本文へ inline 記録されている
- [x] 再現可能 command で peak memory が計測され、command・環境・
      fixture 寸法・結果が
      `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl`
      (`experiment_event.v1`)に記録されている
- [x] 保存先が `benchmarks/results/` でなく `docs/impl/exp/` の
      measurement note である理由(harness 専用 store との
      区別)が note または本 task に記録されている
- [x] 「streaming は deferred のまま維持」または「design 差し戻し」
      の判定が実測根拠つきで記録されている

## Verification

- [x] artifact に記録した再現 command を再実行すると同一 fixture
      で peak memory が再計測できる(再現性チェック — command は
      artifact 本文に記載)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` exits 0(65 passed)
- [x] `git status --short` で artifact が check-in 対象として tracked
      され、計測用一時 fixture(非 commit 対象の生成物)が diff に
      残っていないこと(supplement)

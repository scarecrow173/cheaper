---
id: "IMPL-034Aca4oJotR844AUJf6QR"
type: "impl"
status: "completed"
title: "Resource manager observation の実装記録"
created: "2026-10-05"
updated: "2026-10-05"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/wire-resource-manager-observation.md"
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
# Resource manager observation の実装記録
## Context
A の coordinator 承認後に slice B を開始。inline/TDD、nested dispatch は common brief により省略。
## Implementation
read/count_tokens/walker/sniff/grep open と Pi write/edit/ls の境界を ResourceAccess に接続。runWorker は raw writeScope のみ抽出し、session creation 前に ready を待つ。mkdir 祖先認可と legacy standalone boundary も同じ controller に委譲、実 settle 後の ALS observation は拒否する。grep.ts の2箇所は actual open の観測に必要な scope 内の追記。
## Validation and Evidence
RED: 新規 suite で observation 欠落、strict read 未拒否、モデル待機中に foreign write が開始することを確認。journal API の fixture 誤りは finalize().files/kind に訂正した。
GREEN: runtime-pi 全35 files/379 tests、tool-runtime 全6 files/105 tests pass（02:47 JST）。pnpm lint/typecheck/build exit 0。
新規8 tests: actual read/count、record/strict、I/O前 writeScope拒否+mkdir/journal、privacy秘匿、bounded snapshot と cap後拒否、operation scope外/settle後観測拒否、alias認可、fake Pi read→model wait→edit の job保護。
## Follow-ups
canonical check deferred to coordinator merge run。

## Final Review and Validation
coordinator msg_2ce1608b3f28 は APPROVED: implementation slice B。TSPEC 承認と relation metadata 補完は coordinator が実施。
P2 指示に従い、symlink の実 path が privacy-deny 対象なら observer/stat/yield 前に entry を skip し、許可 entry の走査を継続する。logical deny と canonical deny の存在秘匿を揃え、他の scope/strictness エラーは従来どおり拒否する。
回帰 RED は alias→protected 内の denied nested file による走査全体 abort。tool-runtime 再 build 後の同じ test は GREEN。
最終検証（03:58 JST）: tool-runtime 6 files/105 tests、runtime-pi 35 files/380 tests pass。pnpm lint exit 0（既存33 warnings）、pnpm typecheck exit 0、pnpm build exit 0。
新規 suite は9 tests。Worker job 保護 test は journal modified a と read/edit operation completed も確認する。

## Deferred Exposure
AccessSnapshot の WorkerRunMeta への露出は coordinator 決定で defer。runtime API の accesses() で取得でき、AC7 のアクセス記録要件は実装・検証済み。canonical check deferred to coordinator merge run。

## Related Experiments
独立 experiment log は作成していない。採用した方針を temporary real filesystem と fake Pi の契約テストで検証した。

## Summary
担当 task の実装・契約検証を完了し、coordinator の実装 review 承認を得た。

## Risks
本 lease は invocation 内の協調実行を対象とする。外部 process による filesystem mutation は保護対象外で、cross-process/OS lock は導入していない。

## Exit Audit and Graph Terminal
doc-maintenance plan/apply で spec/plan/task/test-spec の own status rows を再生成。impl/ir README は承認済み例外による own row の機械的 status 更新。IR audit が要求する見出しを A/B 両 record に補完し、担当 record の error は0。
doc-status all audit: own chain + shared ADR の blocking finding 0、global blocking 48 は既存・担当外。shared ADR の reciprocal warning 2 は未変更。
run-until-yield の継続 trace: implementation-to-followup-triage → followup-triage-terminal → exit-audit-to-complete → complete（status=terminal、blockers=[]）。taskBudgetCount=2/maxHops=11 は A から継承。両 task done、runnable/active/blocked は空。
task audit / impl-record audit / scoped all audit を完了として評価し、followup-terminal と exit-audit-pass はこの dispatch の own chain + shared ADR に限る。全 GraphRoute の receipt は worker report に保存する。

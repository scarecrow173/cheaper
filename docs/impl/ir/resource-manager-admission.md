---
id: "IMPL-034AaY3USHTmzeYrZ7pa7J"
type: "impl"
status: "completed"
title: "Resource manager admission の実装記録"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
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
    - "docs/tasks/implement-resource-manager-admission.md"
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
# Resource manager admission の実装記録
## Context
slice A を開始。plan msg_b40bf1f1ee70 承認、inline/TDD、coordinator review を使用する。
## Implementation
ResourceManager の mixed FIFO/all-or-none、opaque parent/closing token、borrow 全包含/foreign-job fail-fast と parent-blocked waiter bypass を実装。ToolScheduler は capacity と claim を atomic admission し preparation error を queued→cancelled に記録する。ResourceAccess は writeScope/manifest の単一認可と snapshot、registry は ready latch/ALS と入力 snapshot を所有する。
meta profile/nested dispatch は明示 common brief により省略し prescribed chain を inline 実行する。
## Validation and Evidence
baseline: tool-runtime 87、runtime-pi focused 70、core 93 pass/2 skip。
RED: resources suite は ready/acquireJobLease 未実装と independent mutation 直列化で失敗。GREEN: tool-runtime 全 6 files/98 tests pass（2026-10-05 02:06 JST）、pnpm lint exit 0（既存 warning 33）、pnpm typecheck/build exit 0。
新規 11 tests: same-file/独立 claim、親保持/bypass/FIFO、record-only ready 直列化、元 abort reason、owner/strictness、alias、reverse multi-claim、late detach、snapshot、process lifetime、foreign-job fail-fast、pending ready capacity を検証。coordinator slice A review を依頼する。
## Follow-ups
review msg_d925990b327a は BLOCKED（raw root の canonical claim conflict 見落とし）。symlink workspace root で job が running global mutation を追い越す RED を確認後、ResourceManager の realpathSync snapshot と shared controller root を修正。実在しない fake root は raw のまま保持する。P2 の nested pump、preparation cancellation reason、root exclusive fallback、workspace-denied accounting、stale test 名も修正/検証。tool-runtime 105 tests と lint/typecheck/build が pass（02:31 JST）。再 review を依頼する。
自己レビューで queued sibling writer を後続 shared sibling が追い越す RED を確認し、bypass を同じ親を借りていない要求に限定した。追加の unknown manifest root-exclusive test と合わせ tool-runtime 100 tests、lint/typecheck/build が pass（02:21 JST）。mixed queue の明示順序を維持する。Graph taskBudgetCount=2、maxHops=11。初回 implementation-review msg_d925990b327a は pending、同じ質問を再開中。
B が実 I/O observer を所有する。canonical check deferred to coordinator merge run。

## Review
coordinator 再 review: APPROVED: implementation slice A（初回 msg_d925990b327a の全 P1/P2 対応を検証、2026-10-05）。tool-runtime 105 tests、lint/typecheck/build pass。slice B への移行を許可。


## Related Experiments
独立 experiment log は作成していない。採用した方針を temporary real filesystem と fake Pi の契約テストで検証した。

## Summary
担当 task の実装・契約検証を完了し、coordinator の実装 review 承認を得た。

## Risks
本 lease は invocation 内の協調実行を対象とする。外部 process による filesystem mutation は保護対象外で、cross-process/OS lock は導入していない。

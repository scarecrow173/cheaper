---
id: "TASK-034zXB21CtI0XLnu6QAHPe"
type: "task"
status: "done"
title: "Migrate native workflows and release guidance"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/implement-payload-untracking-migration.md"
  implemented-by: ["IMPL-034A6i3cxbZXoHqDxhuyjB"]
  depends-on:
    - "docs/plans/implement-payload-untracking-migration.md"
    - "docs/tasks/untrack-payload-and-validate-source-assets.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/payload-free-source-acquisition-and-native-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# migrate-native-workflows-and-release-guidance

## Work
workflow freshness を除去し native smoke と拒否/1 dispatch guide を整合する。

## Upstream Documents
PLAN-034zXjR2oE7USB76eUPCAq、SPEC-034zWqZR8Dmu9TXTR2qcsg、DESIGN-034zWr0ZeceJCJaL3ZRK3H、TSPEC-034zXB13nuorLP81dn8ro8。

## Scope
.github/workflows/**、scripts/test/release-workflow.test.mjs、AGENTS.md、release guide、scripts/lib/apm-bundle.mjs の comment。

## Dependencies
前 task TASK-034zXB1vK84hUCtRlEyiqG の完了が必要。 子 run は作らない。

## Done Criteria
- 指定する focused verification が exit 0。
- 変更と RED/GREEN evidence を task の IR に記録し、coordinator の review を受ける。
- 依存変更、履歴 rewrite、実 publish、cheaper.mjs refresh を行わない。

## Verification
- [x] mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs — exit 0

## Notes
canonical check deferred to coordinator merge run。global の他 chain findings は ownership 外として report する。

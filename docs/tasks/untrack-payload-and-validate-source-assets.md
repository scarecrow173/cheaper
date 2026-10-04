---
id: "TASK-034zXB1vK84hUCtRlEyiqG"
type: "task"
status: "in-progress"
title: "Untrack payload and validate source assets"
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
  implemented-by: []
  depends-on:
    - "docs/plans/implement-payload-untracking-migration.md"
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

# untrack-payload-and-validate-source-assets

## Work
source checker/test、attributes/ignore/index を変更する。

## Upstream Documents
PLAN-034zXjR2oE7USB76eUPCAq、SPEC-034zWqZR8Dmu9TXTR2qcsg、DESIGN-034zWr0ZeceJCJaL3ZRK3H、TSPEC-034zXB13nuorLP81dn8ro8。

## Scope
scripts/check-apm-payload.mjs、scripts/test/apm-payload.test.mjs、.gitattributes、.gitignore、3 payload executable。

## Dependencies
fresh setup と承認済み upstream が前提。 子 run は作らない。

## Done Criteria
- 指定する focused verification が exit 0。
- 変更と RED/GREEN evidence を task の IR に記録し、coordinator の review を受ける。
- 依存変更、履歴 rewrite、実 publish、cheaper.mjs refresh を行わない。

## Verification
- [ ] mise exec -- node --test scripts/test/apm-payload.test.mjs — exit 0
- [ ] mise exec -- node scripts/check-apm-payload.mjs — exit 0

## Notes
canonical check deferred to coordinator merge run。global の他 chain findings は ownership 外として report する。

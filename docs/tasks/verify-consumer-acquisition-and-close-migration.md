---
id: "TASK-034zXB2z3uWw5lq9IyKOqe"
type: "task"
status: "in-progress"
title: "Verify consumer acquisition and close migration"
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
    - "docs/tasks/migrate-native-workflows-and-release-guidance.md"
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

# verify-consumer-acquisition-and-close-migration

## Work
fresh native fixture と既存 consumer tests を移行し、distribution、supersede/index、review/audit evidence を完了する。

## Upstream Documents
PLAN-034zXjR2oE7USB76eUPCAq、SPEC-034zWqZR8Dmu9TXTR2qcsg、DESIGN-034zWr0ZeceJCJaL3ZRK3H、TSPEC-034zXB13nuorLP81dn8ro8。

## Scope
承認済み 3 CLI test files と native-executable-fixture.ts、distribution guide、旧 ADR/SPEC/TSPEC、generated index、IR。

## Dependencies
前 task TASK-034zXB21CtI0XLnu6QAHPe の完了が必要。 子 run は作らない。

## Done Criteria
- 指定する focused verification が exit 0。
- 変更と RED/GREEN evidence を task の IR に記録し、coordinator の review を受ける。
- 依存変更、履歴 rewrite、実 publish、cheaper.mjs refresh を行わない。

## Verification
- [ ] mise exec -- pnpm --filter ./packages/cli exec vitest run test/e2e/launcher-acquisition.test.ts test/e2e/deployed-executable.test.ts test/e2e/apm-install-targets.test.ts test/e2e/apm-install.test.ts test/e2e/apm-package-layout.test.ts — exit 0
- [ ] mise exec -- pnpm lint — exit 0
- [ ] mise exec -- pnpm typecheck — exit 0
- [ ] mise exec -- pnpm build — exit 0

## Notes
canonical check deferred to coordinator merge run。global の他 chain findings は ownership 外として report する。

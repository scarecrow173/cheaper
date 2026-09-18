---
id: "TASK-034r05iHK8bA7hf8fHPk7j"
type: "task"
status: "done"
title: "Implement final output extraction and schema validation"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement final output extraction and schema validation

## Work

PLAN-034r05iHy2gdnpIrX0mTIb step 12。`session.messages` 最後の assistant message の text block 連結 → ```` ``` ```` フェンス除去 → `JSON.parse` → Ajv validate。失敗時は生出力と Ajv errors を返す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/pi-execution-engine.md`

## Dependencies

- `docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/output.ts`
- `packages/runtime-pi/test/output.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 正常 JSON・フェンス付き・非 JSON・schema 不適合の各テストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/output.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- repair ループは TASK-034r05iHK8bA7hgqNrs11u。

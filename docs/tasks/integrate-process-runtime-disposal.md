---
id: "TASK-034AGiKxX1qSMznOGzJrLb"
type: "task"
status: "done"
title: "Integrate process runtime disposal"
created: "2026-10-05"
updated: "2026-10-06"
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
    - "../plans/implement-managed-process-lifecycle-hardening.md"
  implemented-by:
    - "../impl/ir/integrate-process-runtime-disposal.md"
  depends-on:
    - "../plans/implement-managed-process-lifecycle-hardening.md"
    - "./implement-process-tree-lifecycle.md"
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
    - "../test-specs/managed-process-bounded-lifecycle-guarantees.md"
  references: []
  defers: []
  deferred-by: []
---

# Integrate process runtime disposal

## Work

registry の scoped timeout cancel seam と managed invocation promise tracking を実装し、owner/runtime dispose が process actual settle を待つようにする。既存 generic detach、scheduler/resource release の唯一の finally を維持し、新 pool を作らない。

- [x] task の in-progress IR を作成する。
- [x] TIMEOUT/timed_out、queue wait 除外、outer first-cause、owner isolation、dispose/capacity の回帰 test を記述する。
- [x] focused test の意味のある RED を確認する。
- [x] registry の onTimeout/drain を実装する。
- [x] focused test の GREEN を確認し logical change を commit する（ae1f081）。
- [x] guide に defaults/platform/no-polling/diagnostic/abortGraceMs を超え得る bounded dispose を記載する。
- [x] affected focused suites、lint/typecheck/build と CLI related test を実行する。
- [x] coordinator の implementation review と test-spec acceptance を取得する。
- [x] chain terminal status/index、scoped exit-audit と report を完成し commit する。

focused 329 passed /1 POSIX-only skip と lint/typecheck/build は pass。CLI は15 passed /1 stale bundled parity failed /1 skipped。coordinator verdict（question msg_b41a1970f2fd）は implementation review/test-spec acceptance APPROVED、P0/P1 なし。bundle freshness の failure は coordinator merge 時の pnpm bundle + mise run check に defer することが承認された。worker では禁止された bundle 再生成を行わず source と生成物の evidence を分ける。

## Classification

- `normal-plan-task`

## Upstream Documents

- approved plan `../plans/implement-managed-process-lifecycle-hardening.md`、spec AC1/AC5/AC6/AC9、shared ADR D1/D2/D3/D5/D7。
- test-spec `../test-specs/managed-process-bounded-lifecycle-guarantees.md`。

## Dependencies

- E `./implement-process-tree-lifecycle.md` done が前提。最終 review は coordinator ask（新 active Dispatch のみ）。

## Affected Files

- `packages/tool-runtime/src/registry.ts`, `packages/tool-runtime/test/process.test.ts`、Pi process test と `docs/guides/managed-process.md`。
- Pi focused scope は `process.test.ts`, `tool-runtime-contract.test.ts`, `run.test.ts`, `session.test.ts` を含む。
- own chain と generated indexes の status/evidence のみ。新依存/push/PR/nested dispatch/canonical full check は行わない。

## Done When

- [x] first-cause と operation state、queue timeout 除外、scope/invocation/outer abort、owner isolation と actual settle までの capacity を検証した。
- [x] focused packages と lint/typecheck/build は pass、CLI bundle freshness は承認済み coordinator defer、review の P0/P1 未解決なし。
- [x] own chain + shared ADR scoped audit blocking 0。
- [x] canonical check deferred to coordinator merge run と非実行 platform/既存 global findings を report に明記した。

## Verification

- [x] `mise exec -- pnpm exec vitest run packages/core/test/policy packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/run.test.ts packages/runtime-pi/test/session.test.ts` exits 0。
- [x] CLI hook-contract を単独 --testTimeout 30000 で再実行し15 passed /1 failed /1 skipped。stale bundle の1 failure は coordinator merge 時の pnpm bundle + mise run check へ defer APPROVED（msg_b41a1970f2fd）。CLI suite GREEN は主張しない。
- [x] `mise exec -- pnpm lint` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
- [x] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` の own chain + shared ADR に blocking 0。
- [x] `git diff --check` exits 0。

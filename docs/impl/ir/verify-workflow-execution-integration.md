---
id: "IMPL-034APqdMTW2hYck2FYoKv1"
type: "impl"
status: "completed"
title: "Verify workflow execution integration"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: docs/impl/ir/workflow-run-cli-public-envelope.md }
      - { type: file, path: docs/impl/ir/README.md }
      - { type: file, path: docs/tasks/integrate-workflow-run-cli.md }
      - { type: file, path: docs/tasks/verify-workflow-execution-integration.md }
      - { type: file, path: docs/tasks/README.md }
      - { type: file, path: docs/plans/implement-one-shot-workflow-execution-and-cli-integration.md }
      - { type: file, path: packages/cli/src/cli.ts }
      - { type: file, path: packages/cli/src/usage-log.ts }
      - { type: file, path: packages/cli/src/workflow-run.ts }
      - { type: file, path: packages/cli/test/workflow-run.test.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: ["../../tasks/verify-workflow-execution-integration.md"]
  implemented-by: []
  depends-on: ["../../tasks/integrate-workflow-run-cli.md"]
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../../plans/implement-one-shot-workflow-execution-and-cli-integration.md"]
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

# Verify workflow execution integration

## Summary

S5 は commit `c7c0e6f`、S6 follow-up は commit `f6a2bb6` で coordinator review approved。S6 review findings の P3 3点を解消し、focused suite と build/lint/typecheck、scoped exit-audit を確認した。

## Context

承認済み workflow execution plan の S6 verification matrix に従う。canonical `mise run check` と merge verification は coordinator が実行する。

## Implementation

### Changes

- S5 の task status を done とし、S6 verification task を in-progress にした。task index は doc-maintenance plan/apply で再生成した。
- coordinator P3 の未使用 observer option を削除し、tool failure にも選択済み toolCalls/changes metadata を保持するよう修正した。`--quiet` の stderr 抑制は workflow-run pre-engine error のみに限定した。
- usage logger は workflow envelope type guard で除外し、通常 error envelope の usage-bearing failure 記録を保持する。
- plan の CLI package filter を実在する `cheaper` に直し、IR へ実測と repository baseline を記録する。

### Decisions During Implementation

- default full CLI suite の2失敗は範囲外の Copilot hook timeout と launcher concurrent cache rename 競合。両方を単独再実行し pass を確認した。長時間の one-worker full-suite retry は約8分後に停止したため、成功とは扱わない。
- focused V1–V5 command と S5 CLI files の単独実行結果を full suite の失敗と別に記録する。
- 既存 global audit blocking findings 48件は ownership 外で変更しない。

## Related Experiments

### Adopted

該当なし。

### Rejected

該当なし。

## Validation

- V1 core policy/write-scope: 6 files / 59 tests passed。
- V1 workflow compiler: 1 file / 48 tests passed。
- V2 workflow engine/resource inference/no-Pi: 3 files / 64 tests passed。
- V3 tool-runtime artifact/read/no-Pi: 3 files / 30 tests passed; workflow artifact store: 1 file / 15 tests passed。
- V4 tool-runtime invocation/resources: 2 files / 34 tests passed; runtime-pi lease/run/read/registry: 4 files / 102 tests passed。
- S5 V5a: 8 files / 116 tests passed; V5b built CLI E2E: 2 files / 11 tests passed; every named S5 CLI file passed individually (workflow-run 7, interface 25, args 31, input 7, envelope 9, text 3, usage 12, main 22, E2E 2+9).
- Coordinator approved implementation review for `c7c0e6f`: 14 requirements were verified, including parsed-token input detection, timeout restriction, workflow discriminator and text/usage dispatch, lazy shared runtime/store/model paths, tool-only no-model path, streamFn placement, public Pi helper handler seam, artifact projection, lease/manifest ownership, single stdout and exit behavior. P3 findings were unused observer option, quiet scope, duplicate usage guard, and asymmetric tool failure metadata; each was addressed in this S6 change and relevant tests rerun.
- `mise exec -- pnpm lint`: exit 0, 271 files checked, 36 existing warnings.
- `mise exec -- pnpm typecheck`: exit 0. `mise exec -- pnpm build`: exit 0.
- `mise exec -- pnpm --filter cheaper test`: exit 1, 363 passed / 10 skipped / 2 failed of 375; Copilot hook timeout and launcher cache race both passed in isolated reruns. `vitest run --maxWorkers=1` retry was stopped after about 8 minutes without a final report.
- `audit_impl_record.js --json`: 99 IR files, findings 0. Full doc-status audit: 550 files / 462 findings; selected workflow chain findings 0, 48 repository-wide blocking findings outside this chain. Two plan-referenced shared ADRs have two nonblocking reciprocal warnings and zero blocking findings.
- Canonical `mise run check` and merge verification are deferred to coordinator.

## Risks

The complete default CLI package suite did not finish green as a single invocation. The two failures pass in isolation; the serialized retry remained inconclusive. Global blocking document findings remain outside this chain.

## Follow-ups

Complete final status/index synchronization, run final doc/task graph exit-audit, commit S6 closure, and report the full-suite limitation to the coordinator.

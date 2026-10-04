---
id: "IMPL-034zXB37yW53DP9xUVf57j"
type: "impl"
status: "completed"
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
    - "docs/tasks/untrack-payload-and-validate-source-assets.md"
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

# Untrack payload and validate source assets

## Summary
承認済み plan の Unit A を開始する。binary 不同梱 source の invariant を checker/index の境界で修復する。

## Context
PLAN-034zXjR2oE7USB76eUPCAq の Unit A。承認済み SPEC/ADR/DESIGN と source asset contract に従う。

## Implementation
実装前に IR を開き、binary-free source/checker と Git index/ignore contract の tests を先行追加した。
3 target executable を git rm --cached で追跡解除し、worktree 内の当該 file を削除した。LFS 履歴は保持し、
attributes の payload filter を除去、cli/bin/*/ の local output を ignore した。
checkApmPayload(root) は必須 launcher/hooks/small assets の nonempty regular file、pointer 拒否、JSON と
canonical version metadata を検証する。platform binary の存在・byte 比較・実行は行わない。

## Related Experiments
なし（承認済み設計の source tracking/checker 移行）。

## Validation
- mise run setup: exit 0。
- baseline launcher acquisition: 45 passed / 5 skipped。
- baseline release script suites: 26 passed / 1 skipped、exit 0。
- APPROVED: spec-approval / adr-acceptance / design-approval、APPROVED: plan-approval を coordinator ask で受領。
- graph: probe-to-briefing → briefing-to-design → design-to-planning。focus は本 spec。子 run は作成していない。
- graph: planning-to-task-graph → task-graph-to-implementation (commitGate)。approved PLAN を inline 実行。
- RED: scripts/test/apm-payload.test.mjs exit 1（旧 checker に checkApmPayload(root) がなく、Git index に binary が残る）。dist/73c-evidence/source-red.log。
- GREEN: 同 script suite は 16 tests pass、exit 0。source-green.log。checker CLI も exit 0。
- fixture は missing/pointer/directory/empty/malformed JSON/version drift と foreign target ignore を検証する。

## Risks
source checker は native executable の version/実行を実証しない。native build と consumer fixture が担う。

## Follow-ups
APPROVED: implementation-review / test-spec-approval を受領。canonical check deferred to coordinator merge run。

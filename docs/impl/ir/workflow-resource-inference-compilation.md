---
id: "IMPL-034zUfCug3Fz5ZzcrI1yhg"
type: "impl"
status: "in-progress"
title: "Workflow resource inference compilation"
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
    - "docs/tasks/integrate-workflow-resource-inference.md"
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

# Workflow resource inference compilation

## Summary
compiler の inference gate を resources port 注入へ変更し、全 job の manifest を検査して保持する。conflicts/diagnostics を deep clone で公開し、inferred edges は既存 compileDag に union する。

## Context
承認済み plan slice 2 / TASK-034zTTjWYMNKiiVb4ZZ6LV。共有 ADR D4/D5 の shape と未注入 compiler の既存挙動を保持する。

## Implementation
### Changes
compiler.ts、compiler.test.ts。IR は編集前に in-progress で作成した。
### Decisions During Implementation
未注入かつ非空 resources では INVALID_REQUEST を維持する。注入された port の失敗・manifest 欠落は省略-only workflow でも伝播する。optional field は exactOptionalPropertyTypes を守り undefined property を組み立てない。

## Related Experiments
なし。

## Validation
- RED: workflow suite 106 tests 中 3 failure。all-omitted port calls=0、missing manifest が resolve、diagnostics=undefined を確認。
- GREEN: workflow suite 106 tests / 5 files pass（2026-10-04 14:31 JST）。
- needs + $from + producer edge dedupe、inferred cycle rejection、既存 needs があっても duplicate producer rejection、metadata known consumer read、result deep clone を test で固定。
- pnpm lint: exit 0。
- pnpm typecheck: optional property construction の TS2375 を修正後 exit 0。
- pnpm build: 同修正後 exit 0。
- tool-runtime suite 18 tests / 3 files pass。
- canonical check deferred to coordinator merge run。

## Risks
fallback safety claim による conservative conflicts、Worker ancestor mkdir の root exclusive safety、core literal symlink/hard-link residual risk を継承する。engine/scheduler/ResourceManager/process executor は実装しない。

## Follow-ups
coordinator implementation review、chain status/index maintenance、scoped exit-audit を実施する。

## 自己点検での source boundary 修正
- Worker target src/*.ts が一般 write normalization で root subtree に拡大し、無関係 output ownership を受理していた。新規 test は promise resolved の RED。authority は core ownership normalization（produces bucket）で exact literal を検査し、root mkdir safety は normalizeResourceClaim で別に作る。
- 非 process job の process 宣言が tool/Worker の実 source を置き換えていた。新規 test は promise resolved の RED。process declaration は kind:tool/tool:process のみ受理する。
- process config 自体が explicit source なので resources-omitted fallback diagnostic を付けない。
- 修正後 workflow suite 108 tests / 5 files GREEN、pnpm lint/typecheck/build exit 0（2026-10-04 14:47 JST）。固定 core 型と approved design の authoritative exact target / process seam に合わせる source-layer correction。レビュー対象 commit に追加する。

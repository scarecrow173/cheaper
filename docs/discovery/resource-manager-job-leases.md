---
id: "DISC-034A8jvNOxFXeSIc8tWQmE"
type: "discovery"
status: "resolved"
title: "Job lifetime lease の ownership と admission"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/61"]
  derives-from: ["../ideas/resource-manager-job-leases.md"]
  derived-by: ["../specs/resource-manager-job-leases.md", "../adr/resource-manager-job-leases.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---
# Job lifetime lease の ownership と admission

## Exploration Goal
writeScope の stale-read→stale-edit を防ぎ、親 lease の borrow と子 operation 間競合を両立する ownership 境界を調べる。

## Constraints
共有 ADR D2/D4/D7 と issue Triage の borrow 決定に従う。型・normalization・predicate は core を再利用。#62 engine と #59 executor は対象外。

## Evidence
- `packages/tool-runtime/src/invocation.ts` の ResourceManager/Lease は never placeholder。parentLease は拒否される。
- `packages/tool-runtime/src/scheduler.ts` は per-resource 指定でも global exclusive。record は admission 前 queued、実 settle まで capacity を保持。
- `packages/core/src/resources/` に canonical real path、未知 bucket の root fallback、containment/conflict がある。
- `packages/runtime-pi/src/run.ts` は writeScope を input から導出するが session lifetime lock はない。
- `packages/runtime-pi/src/tools/operations.ts` は actual filesystem boundary と MutationJournal を所有。

## Alternatives
1. per-call lock のみ: 独立 file overlap は可能だが session 間 stale-read を防げない。
2. job lease + child operation lease: stale-read を防止し同一親内の conflict も別管理。token の所属・lifetime 検証が必要。
3. 全 invocation global exclusive: 安全だが独立 resource 並行性を提供しない。

## Tentative Conclusion
案2が共有 ADR に整合する。ResourceManager は invocation 単位、親 job lease を子へ渡す。子は親の覆う claim を借用しつつ子同士の conflict を保持する。
job ready は session 開始前に await。operation は capacity と全 claims が揃った時だけ start、claims 待ちは queued。待機 claim は capacity を占有せず別 job を進める。resource conflict は semantic edge を作らない。
実 I/O boundary で observed access を記録し manifest 外 write を拒否、read は default record / opt-in reject とする案。injected handler/process は宣言を保守的に信頼し OS tracer を主張しない。

## Open Questions
pre-implementation blocker: coordinator の ADR intent / spec-design / plan 承認。directory access は subtree を用いる。

## Promotion Candidates
spec と slice ADR と design。hot file・依存変更は不要の見込み。

## Entry Decision
A-4: AC は brief が指定、ownership/admission を repository evidence で確認。dispatch 指示に従い prescribed chain を inline 実行し profile/nested dispatch は作らない。

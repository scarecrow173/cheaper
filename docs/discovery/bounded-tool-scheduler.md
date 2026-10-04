---
id: "DISC-034zWckRcYKm7QivslioOd"
type: "discovery"
status: "resolved"
title: "ToolScheduler の admission と ownership 境界"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../ideas/bounded-tool-scheduler.md"]
  derived-by: ["../specs/bounded-tool-scheduler.md", "../adr/bounded-tool-scheduler.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# ToolScheduler の admission と ownership 境界
## Exploration Goal
admission から settle までを adapter 非依存で制限する最小境界を調べる。
## Constraints
共有 ADR D1/D2/D3/D7、#56 の first-cause/late-settle、core isMutatingTool を再利用する。nested dispatch は行わない。
## Evidence
registry.execute は validation → 同期 budget reservation → manager.run → meter → handler。manager.run は即 start し queue 待機を持たない。
invocation は単一 manager を共有し Pi adapter は runtime.execute を呼ぶ。Workflow compiler は metadata port のみで実行 engine は #62。
公開 definitions.execute は現状 raw handler を公開し facade bypass の余地がある。公開 descriptor も facade を通す必要がある。
## Alternatives
1. adapter ごとの semaphore: shared upper bound と mutation 順序を保証できない。
2. tool-runtime の invocation-owned scheduler: manager record を queue 前に作り、settle まで capacity を保持する案。共有 ADR に整合する。
## Tentative Conclusion
案2が有力。read を shared、mutation を exclusive として、FIFO で writer starvation を防ぐ。unsettled detach で capacity を返すと実際の並行数が上限を超えるため、実 settle まで保持する。
## Open Questions
pre-implementation blocker: spec/design と core policy 変更の承認。#59 pool と #61 per-resource lease は後続事項。
## Promotion Candidates
spec/design へ昇格。共有 ADR D1/D2/D7 を参照し、FIFO と capacity 所有だけを slice ADR に記録する。
## Entry Decision
A-4: brief と issue Triage に AC が明示されている。dispatch 指示で briefing meta skill の profile/subagent 設定を省略し prescribed chain を inline 実行する。

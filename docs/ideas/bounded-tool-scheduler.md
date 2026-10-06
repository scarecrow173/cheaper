---
id: "IDEA-034zWckq6QerjvcSiFzdmE"
type: "idea"
status: "promoted"
title: "操作並行度を canonical runtime で制限する"
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
  derives-from: []
  derived-by: ["../discovery/bounded-tool-scheduler.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 操作並行度を canonical runtime で制限する
## Idea
Pi と Workflow の tool 呼出しを、invocation 全体の bounded scheduling に通す。
## Problem / Opportunity
同時 read の上限と adapter 横断の mutation 順序を Worker/Workflow 利用者に提供する必要がある。
## Open Questions
capacity 待機と OperationManager の queued 状態をどこで接続するか。
## Next Action
discovery で入口・lifecycle を追跡し、spec/design を coordinator に提示する。

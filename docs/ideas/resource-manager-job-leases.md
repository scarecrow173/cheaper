---
id: "IDEA-034A8hNxIG1ERfsXQ9172S"
type: "idea"
status: "promoted"
title: "Job lifetime resource leases"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/61"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
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
  derived-by: ["../discovery/resource-manager-job-leases.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Job lifetime resource leases

## Summary

Worker の writeScope を job 全体で保護し、独立 resource の operation を安全に並行化する。

## Problem and Motivation

read と edit の間に別 Worker が変更すると stale contents に基づく編集になる。

## Expected Value

Worker と Workflow の呼出し元が mutation の整合性と並行性を得る。

## Open Questions

job と子 operation の lease ownership と admission 順序をどこで管理するか。

## Next Action

discovery に昇格し、spec/design を coordinator の承認に提示する。

---
id: "TSPEC-034AaXPxCwdUwH1XaYc6Jn"
type: "test-spec"
status: "draft"
title: "Resource lease の寿命とアクセス境界"
created: "2026-10-05"
updated: "2026-10-05"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/resource-manager-job-leases.md"
  derived-by: []
  verifies:
    - "docs/specs/resource-manager-job-leases.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# Resource lease の寿命とアクセス境界
## Purpose
stale-read→stale-edit と ownership 消失を防ぐ契約を保持する。job/operation lease の仕組みを廃止した時のみ再評価する。
## Feature
ResourceManager による job と operation の寿命保護。
## Rules
- Rule: 全 claim は atomic に取得し、実 settle まで保持する。
- Rule: 全包含 parent borrow のみ許可し、foreign job conflict は INVALID_REQUEST。
- Rule: 親に塞がれた waiter は子の完了を塞がず、親解放後は提出順へ戻る。
- Rule: writeScope/manifest/operation scope を I/O 前に検証する。readStrictness は親から継承する。
## Examples
- Example: 独立 file mutation は重なり、同一 file の read/write は待つ。
- Example: read→model待機→edit の間に別 job は同じ file を変更できない。
- Example: reverse multi-claim と queue abort は部分取得も reason 置換も生まない。
- Example: symlink alias と ancestor は canonical identity で conflict する。
- Example: undeclared read は record/reject、privacy deny は記録されない。
- Example: process declaration double の promise 未 settle は claim を保持する。
## Guarantees
owner/manager を偽装できない。unknown scope は保守的に fallback し、cap を超えた access も検証する。予算、六状態遷移、journal を維持する。
## Non-goals
cross-process lock、OS tracer、実 provider の品質、Workflow DAG inference。
## Risk
契約を失うと stale edit・scope 外 mutation・親子 deadlock・早期解放による競合が再発する。

---
id: "SPEC-034A8kLznlxOOeccFx9SNM"
type: "spec"
status: "implemented"
title: "ResourceManager job lifetime leases"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/61"]
  derives-from: ["../discovery/resource-manager-job-leases.md"]
  derived-by: ["../designs/resource-manager-job-leases.md"]
  related: ["../adr/resource-manager-job-leases.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---
# ResourceManager job lifetime leases

## Problem
per-call mutation serialization は Worker が read と edit の間に持つ前提を保護できない。独立 resource にも global exclusive が適用される。

## Audience and Value
Worker 呼出し元と Workflow engine は job の mutation scope を保護し競合しない operation を並行実行できる。

## Scope
共有 ADR D2/D4/D7 の canonical resource lease、job lifetime writeScope、borrow、abortable all-or-none wait、observed filesystem access validation。

## Non-Goals
cross-process/OS tracer、hard-link alias、外部 symlink 差替え、process executor、Workflow engine/DAG inference、model-visible lock tool は含まない。

## Behavior
validated manifest がある場合のみ per-resource mutation を許す。manifest なし・未知 mutation は root exclusive。serialized ceiling は維持。子 scope は親 lease を解放せず同じ親の子 operation 間 conflict は維持。
observed write は authoritative writeScope と明示 manifest の両方に包含されなければ拒否。undeclared read は既定 record-only、明示 strict mode では拒否。MutationJournal と privacy deny は維持。

## Acceptance Criteria
- AC1: validated manifest で独立 file mutation が overlap、同一 file read/write/edit は非 overlap。
- AC2: Worker writeScope が session 全体で保持され read→edit 間に他 job が書けない。
- AC3: declared process output scope が process promise の実 settle まで保持（#59 seam double）。
- AC4: 複数 claim を部分保持せず逆順 request が deadlock せず queued abort は original reason。
- AC5: symlink alias と directory subtree overlap を検出、未知/broad mutation は root exclusive。
- AC6: 親 borrow は self-deadlock せず、親を待つ conflicting waiter（queued job parent 取得を含む）が先に queue にいても子が進行する。二つの record-only job の ready は直列化し、foreign-token conflict の不整合は child 待機へ入れない。子同士は競合制御、子 dispose と late detach は親範囲を早期解放しない。
- AC7: actual access を記録、scope 外 write は I/O 前拒否、undeclared read は documented policy。
- AC8: MutationJournal/writeScope と budget/OperationManager の既存 focused suite に回帰なし。

## Verification
tool-runtime/runtime-pi focused suite、pnpm lint/typecheck/build。canonical check deferred to coordinator merge run。

## Approval
coordinator msg_84fd50c7a75a: APPROVED: spec-design。

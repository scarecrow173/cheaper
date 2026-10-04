---
id: "ADR-034zT6bqTxOvK1nP0ELSyR"
type: "adr"
status: "accepted"
title: "OperationManager の cancel 要求と実 settle を分離する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/56"]
  derives-from: ["../discovery/operation-manager-lifecycle.md"]
  references: ["fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# OperationManager の cancel 要求と実 settle を分離する

2026-10-04 coordinator msg_58cbe5b57993: local ADR の作成・accepted への変更を承認。

## Context and Problem Statement

#63 の bounded drain は無協力 handler が止まらなくても返る。#56 Triage と共有 ADR D3 は実 settle の観測前に terminal を確定することを禁じる。
本 ADR は D1/D3/D7 を変更せず、未 settle の record と runtime ownership の表現を細化する。
coordinator msg_df657964221d の許可により、graph の briefing が ADR artifact を要求する場合の scoped decision を記録する。

## Decision Drivers

- finite abortGraceMs と finite disposal return。
- cancel 要求を実停止の証明として扱わない。
- owner isolation と first-cause の観測。

## Considered Options

- cancel 直後に cancelled/timed_out へ移す: 停止を偽装するため却下。
- 実 settle を無期限に待つ: #63 bounded return に反するため却下。
- running のまま unsettled を明示し runtime ownership を終了する: 採用候補。late settle を追跡し、停止未確認の診断を保持する。

## Decision Outcome

第三案。queued cancel は未開始なので即 terminal。running cancel は signal と first-cause の記録だけを行う。
grace 超過は settlement=unsettled、owned=false、detachedAt を付けるが finishedAt は付けない。
fulfilled/rejected の観測時だけ completed/failed または requested cancelled/timed_out を確定する。
scope disposal はその owner だけ、root disposal は全 owner を閉じる。

### Consequences

- Good: running 区間を grace 時刻で偽って終了しない。bounded return と ownership isolation を同時に満たす。
- Bad: 無協力 external 処理の副作用は継続し得る。process executor の kill/cleanup 設計は #59/#60 に委ねる。

## Implementation Plan

- packages/tool-runtime/src/operation.ts に manager と state 検証を置く。core ResourceClaim 型を参照し lease/scheduler は作らない。
- registry.ts は既存 accounting の admitted callback から tracked run を行い、実 handler Promise の settle を通知する。
- invocation.ts は 1 manager を owner scope に共有し、signal.reason を保持する。
- runtime-pi/src/run.ts は requestAbort の reason を CheaperError として signal.reason に渡す最小変更のみ。
- manager/registry/invocation の tests を追加し、既存 canonical result/journal/meter を維持する。依存・hot file は変更しない。

## Verification

- [ ] transition matrix・first-cause・queued/running cancel・late fulfill/reject が pass。
- [ ] 非協力 handler の finite drain、owned active=0、finishedAt absent、sibling 生存が pass。
- [ ] tool-runtime focused suite・runtime-pi abort tests・lint/typecheck/build が pass。

## Revisit Conditions

後続 process executor が実停止保証を導入する場合も、terminal は実 settle 観測を根拠とする。

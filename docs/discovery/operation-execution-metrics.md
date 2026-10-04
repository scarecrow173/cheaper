---
id: "DISC-034A8tYcU3vJfWHLa6qGQ2"
type: "discovery"
status: "resolved"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  source: ["https://github.com/scarecrow173/cheaper/issues/58"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../ideas/operation-execution-metrics.md"]
  derived-by: ["../specs/operation-execution-metrics.md","../adr/operation-execution-metrics.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Goal

#58 の runtime 計測を共有 ADR D1/D3/D7 の固定契約で実現する。

## Evidence

- tool-runtime/src/operation.ts: createdAt/start/finishedAt と六状態があり、cancel 要求だけでは running を終了しない。
- tool-runtime/src/invocation.ts: ownerId で records を絞れる。invocation が clock と operation lifetime を所有する。
- runtime-pi/src/run.ts: buildMeta が usage/savings を集約し、WorkerRunError が partial runMeta を運ぶ。前段の model 解決/session 構築エラーには snapshot が無い。
- cli/src/envelope.ts: errorMeta は field を列挙するため execution の明示追加が必要。

## Alternatives

イベント callback collector は lifecycle と二重状態を持つ。既存 timestamp の純粋 reducer なら scheduling を変更せず同じ計測を得られるため採用する。

## Conclusion

OperationManager の単一 clock で snapshot を作り、owner scoped Worker と invocation 全体の Workflow で同じ reducer を使用する。byTool は toolName、toolName がない宣言操作は kind を用い、operations=sum(byState)=sum(byTool) を保つ。

## Open Questions

仕様上の未決事項なし。hot file と spec/ADR/design は coordinator approval 待ち。

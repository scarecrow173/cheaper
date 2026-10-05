---
id: "IMPL-034ANyyPxu6Pz67UTx3yR5"
type: "impl"
status: "completed"
title: "Workflow Worker permission と lease lifetime を配線する"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: packages/tool-runtime/src/access.ts }
      - { type: file, path: packages/tool-runtime/src/invocation.ts }
      - { type: file, path: packages/tool-runtime/test/resources.test.ts }
      - { type: file, path: packages/runtime-pi/src/run.ts }
      - { type: file, path: packages/runtime-pi/test/resource-leases.test.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/wire-workflow-worker-leases.md"
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

# Workflow Worker permission と lease lifetime を配線する

## Summary

S4 の Worker child permission と caller-owned parent lease lifetime を正本に接続した。

## Context

[承認済み plan](../../plans/implement-one-shot-workflow-execution-and-cli-integration.md) S4 と [task](../../tasks/wire-workflow-worker-leases.md) に従う。S3 は coordinator review で APPROVED 済み。

## Implementation

### Changes

ResourceAccess.prepareWorkerManifest は、既存 prepare が canonicalize した literal exact writeClaims を safety writes/produces の各 bucket に contained な対象へ narrow する。safety reads を保持し、writeScope 無しでは宣言を保持、manifest 不明では既存 conservative fallback を保持する。

WorkerExecutionBinding に resourceManifest を追加し、runWorker は同じ narrowed manifest を toolOptions、Pi helper handler、canonical scope へ渡す。helper の observeAccess/journal 配線は既存実装を再利用する。

InvocationToolRuntime.dispose の無条件 resources.releaseJobs を除去した。acquireJobLease の caller が parent release を所有し、dispose 後の late finally も releaseJobLease を使える。scope.ready で自己取得した lease の cleanup と ResourceManager の child-token retention は変更しない。

### Decisions During Implementation

ResourceManager のアルゴリズム、core helper、Pi adapter、CLI は変更しない。権限 narrowing と job safety admission を混同せず、ancestor mkdir を sibling file の permission として残さない。existing parent containment/readStrictness/observed I/O/journal の実装を唯一の正本として使う。

coordinator 承認の `.sdp` profile 再生成省略を継続し、approved plan/TDD/impl-doc/incremental implementation/coordinator review を使用する。nested worker は使用しない。

## Related Experiments

### Adopted

なし。承認済み設計の実装。

### Rejected

なし。

## Validation

tool-runtime RED: V4 の invocation/resources は 4 failed / 28 passed。child token 有/無の双方で dispose 後の parent が closing/released になり、prepareWorkerManifest が未実装だった。修正後 34 passed。contender は caller release と child actual settle まで待ち、自己取得 lease は dispose で release されることも確認した。literal `[1]` を glob 化せず exact identity に保持し、bucket ごとの intersection、explicit empty、unknown manifest fallback を検証した。

Pi RED: binding manifest の配線を省略した baseline は、strict parent でも宣言外 `b` read が completed となり、新 regression が失敗した。manifest を配線すると actual I/O boundary で拒否される。test fixture の既存拒否 semantics（read は failed、writeScope admission 拒否は cancelled）を source に合わせて訂正した。

V4 の最終 GREEN は tool-runtime 34 passed / runtime-pi 102 passed。全 `pnpm build`、変更 5 files の Biome check は exit 0、IR audit は 98 files / findings []。

追加 package 全体検証を二つ同時に実行した際、tool-runtime は process-tree cleanup 1 failure（152 passed）、runtime-pi は stale read fixture 2 failures と access-row timeout 1 failure（384 passed）だった。fixture は coordinator が別 commit `2e230cd` で限定修正を承認し、単独 19 passed。process test は source 変更なしの単独 13 passed、access-row case は source 変更なしの単独 1 passed / filter 対象外 9。並列実行時の transient/timing の可能性があるが原因は確定していない。

`mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run --maxWorkers=1` は 153 passed、runtime-pi の同じ serialized package check は 387 passed。初回並列失敗を package 全体の無条件 GREEN と読み替えない。

source commit `5ba49c0` に対する independent coordinator review（review-protocol）は APPROVED、P0/P1 なし。literal-exact bucket narrowing、reads/unknown fallback、deny、scope/Pi helper の同一 manifest、dispose 後の caller release、parent retention、engine finally/stop の解放責任、fixture 修正範囲を確認した。P3 の既存 releaseJobs dead helper は scope 外の任意 cleanup として残す。

## Risks

fake provider integration を使用し、real model/provider の品質は検証していない。CLI composition は S5。canonical check と main merge の結果は coordinator 所有。

## Follow-ups

S5 workflow run CLI integration へ進む。canonical check/main merge は coordinator 所有。

---
id: "IMPL-034AMOOcav5x6lqiVb9awU"
type: "impl"
status: "completed"
title: "Canonical artifact read と publisher を配線する"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - { type: file, path: packages/tool-runtime/test/artifact-read.test.ts }
    modified:
      - { type: file, path: packages/tool-runtime/src/artifact.ts }
      - { type: file, path: packages/tool-runtime/src/catalog.ts }
      - { type: file, path: packages/tool-runtime/src/invocation.ts }
      - { type: file, path: packages/tool-runtime/src/read.ts }
      - { type: file, path: packages/tool-runtime/src/registry.ts }
      - { type: file, path: packages/workflow/src/artifacts.ts }
      - { type: file, path: packages/workflow/test/artifacts.test.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/wire-workflow-artifact-tools.md"
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

# Canonical artifact read と publisher を配線する

## Summary

S3 の明示 bounded artifact publication/consumption を canonical read に配線した。既存 RunArtifactStore を再利用し、scope は publisher/reader を借用する。

## Context

[承認済み plan](../../plans/implement-one-shot-workflow-execution-and-cli-integration.md) S3 と [task](../../tasks/wire-workflow-artifact-tools.md) に従う。

## Implementation

### Changes

ArtifactPut を tool-runtime 正本へ移し、workflow は互換 re-export する。ArtifactPublisher と借用 ArtifactPorts、store の immutable cap getter を追加した。

read schema/handler は workspace 行 read、workspace publication、artifact byte read の排他的 union を共有する。publication は content を返さず producer.jobId を internal scope owner に固定する。artifact byte mode は workspace resolver/deny/catalog filesystem claim に触れず明示空 resources を宣言する。catalog output schema も三つの結果を受け付ける。

publication は chunk read を cap+1 source bytes に限定し、stat 後の成長、NUL/image/non-regular/deny、abort を put 前に拒否する。FileHandle は finally close する。

### Decisions During Implementation

coordinator は `.sdp` profile 全再生成の省略を承認した。承認済み plan、TDD、impl-doc、incremental implementation、slice ごとの coordinator review を明示 stack とした（user override）。nested worker は使用していない。

既存 store の identity/caps/copy/lifetime は再実装しない。scope dispose は borrowed store を閉じない。workspace read の既存 abort error message は維持する。

## Related Experiments

### Adopted

なし。承認済み設計に沿う既知解の実装。

### Rejected

なし。

## Validation

RED: `mise exec -- pnpm --filter @cheaper/tool-runtime test test/artifact-read.test.ts test/runtime.test.ts test/no-pi-deps.test.ts` は 8 failed / 19 passed。publication が通常 read として本文を返す、mixed input の受理、overflow の未拒否、reader 未配線を確認した。

追加 RED: artifact URI を workspace path として渡すテストが ENOENT で失敗し、input schema で workspace namespace から拒否した。stat 後に成長する source は cap+1 (7 bytes) まで読み拒否し、abort 後は put しないことをテストした。

GREEN: focused V3 tool-runtime は 30 passed、workflow artifacts は 15 passed。tool-runtime 全体は 147 passed、workflow 全体は 171 passed。`mise exec -- pnpm build` は全 package で exit 0、変更した 8 source/test files の Biome check は exit 0。

追加互換検証: Pi read/registry の 50 件中 1 件が既存 abort message の変化で RED。新規 modes のみに入口 abort check を移し、50 passed を確認した。修正後の V3 tool-runtime 30 passed、tool-runtime/workflow build exit 0。IR audit の changes shape 指摘を file-object へ訂正し、findings [] を確認した。

logical commit `ca2bc0f`。coordinator は durable question `msg_fe0640e49c87` の independent review（review-protocol）で APPROVED、P0/P1 なしと回答した。owner 固定、cap+1/pre-put 拒否、store atomic recheck、single-owned ArtifactPut、空 artifact resources、scope の ports override 不可、borrowed store cleanup、S3 範囲を確認した。P3 の明示 implements ArtifactPublisher は任意の nit として記録し、追加変更しない。

## Risks

real provider/CLI workflow composition は S3 の検証対象外。canonical `mise run check` と main merge の結果は coordinator 所有で、ここでは成功を宣言しない。

## Follow-ups

S4 worker binding、S5 workflow run CLI integration を plan 順で進める。

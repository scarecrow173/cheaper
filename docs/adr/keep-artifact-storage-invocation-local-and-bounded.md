---
id: "ADR-034zTe2Gt1o8oAkX43ilEm"
type: "adr"
status: "accepted"
title: "Keep artifact storage invocation-local and bounded"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
  derives-from: ["DISC-034zSQKiU63uXPcMSjjodn"]
  derived-by: ["DESIGN-034zSR05NE5hXXhkTRLMgW"]
  references: ["ADR-034zGZpjoQlppwkkOs5gur"]
metadata:
  adr:
    decision-makers: ["coordinator"]
    consulted: []
    informed: ["Workflow Engine implementer"]
---

# Keep artifact storage invocation-local and bounded

## Context and Problem Statement

大きな workflow 中間出力を全文 prompt に流さず保持する。共有 ADR D4/D5/D7 の artifact namespace、data provenance、invocation 所有を互換的に細化する。coordinator msg_a8d4489914f3 で本 local ADR の作成と accepted を承認済み。共有 ADR は変更しない。

## Decision Drivers

- filesystem から独立した run identity と lookup。
- storage/read/preview の明示 cap と failure/abort cleanup。
- tool-runtime が workflow を import しない依存方向。

## Considered Options

- in-memory run-local store。容量を注入 cap で管理でき filesystem lifetime を増やさない。
- temporary file store。大容量では有利だが host path、permissions、unlink 失敗を新たに扱う必要がある。

## Decision Outcome

in-memory store を workflow が invocation ごとに所有する。opaque run UUID と artifact UUID の URI を reference に保持し producer.jobId/name は別 metadata。tool-runtime に独立 ArtifactReader port を定義する。必須 cap は engine が供給し RuntimePolicy artifact schema は後続統合が所有する。getter/toJSON を持たない frozen plain JSON reference を返す。

### Consequences

- Good: run 間分離と filesystem 非アクセスを store 境界で検証できる。
- Bad: 大容量は cap で拒否される。永続化と無制限出力は扱えない。
- Bad: abort clear は tool drain と順序づけないので consumer は次 read の failure を扱う必要がある。

## Implementation Plan

- packages/tool-runtime/src/artifact.ts と barrel に read source/metadata/reader contract のみ追加。既存 tool 実装は変更しない。
- packages/workflow/src/artifacts.ts と barrel に bounded store、finally helper、unavailable final projection を追加。
- packages/workflow/test/artifacts.test.ts で string/Uint8Array byte計測、overflow 原子性、namespace、不正 id、copy 隔離、JSON metadata、abort と借用失効を検証。
- 新依存・migration・core/CLI/runtime-pi 配線は不要。engine の future InvocationToolRuntimeOptions.artifacts seam と #65-B inference は後続。

## Confirmation

- [x] focused workflow/tool-runtime suites が pass。
- [x] pnpm lint/typecheck/build が pass。
- [x] shared ADR D4/D5/D7 と矛盾しない scoped audit が pass。

## More Information

cap・identity・borrowed lifetime の詳細は run-scoped-artifact-store design に記録。canonical check deferred to coordinator merge run。

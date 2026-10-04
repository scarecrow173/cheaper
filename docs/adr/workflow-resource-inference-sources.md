---
id: "ADR-034zTpAnvDWILMV1LH66Wd"
type: "adr"
status: "accepted"
title: "Workflow の既知 resource と safety fallback を分離する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/65"

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
    - "../discovery/workflow-resource-inference.md"

  derived-by:
    - "../plans/implement-workflow-resource-inference.md"
    - "../designs/workflow-resource-inference.md"
  verifies: []
  verified-by: []
  references:
    - "fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"

  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Workflow の既知 resource と safety fallback を分離する

## Context and Problem Statement
#64 は ResourceInference port のみで、tool catalog は schema しか公開しない。fallback の root read を意味的 dependency とすると出力を持つ複数 job に false cycle が生まれる。D4/D5 を保持し access の正本と依存の根拠を明示する必要がある。
coordinator は msg_448756273b23 で local ADR intent を承認した。共有 ADR の D1/D4/D5 は references として維持し変更しない。

## Decision Drivers
- tool semantics と workflow rule の責務を重複させない。
- 未知 access に安全な safety manifest と、意味が分かる consumer read を区別する。
- optional な追加だけで既存 fake catalog/port caller を維持する。

## Considered Options
- workflow が tool 名と input path semantics を直接 switch する（却下: handler/catalog と知識が重複）。
- ToolMetadata に optional pure resource descriptor を公開する（採用: access semantics を catalog の正本から取得）。
- broad fallback reads の overlap も dependency にする（却下: unknown access から semantic order を発明する）。

## Decision Outcome
Chosen option: optional catalog descriptor と known-read/safety-fallback 分離。

### Additive metadata surface
ToolMetadata.resources?: (input:JsonValue)=>ResourceManifestDeclaration を catalog.ts に追加する。これは共有 ADR D1 surface への additive refinement であり、coordinator が共有 ADR へ追記する対象。既存 required field と dependency direction は不変。descriptor は input だけを受け副作用を持たず、core に委ねる declaration を返す。

### Dependency and safety knowledge
D5 resources 省略/{} の safety manifest は fallback を保持し、known metadata reads または明示 reads のみを produces overlap の根拠にする。unknown read/root safety claim は conflict に参加するが依存には参加しない。宣言と既知 descriptor が矛盾する underclaim は拒否する。

### Consequences
- Good: workflow 内の tool semantics 再実装を防ぎ、unknown fallback の false dependency を防ぐ。
- Bad: catalog callback の保守と source reason の記録が必要。省略 object の fallback と known dependency read は二つの役割として扱う。
- Worker の ancestor mkdir は exact target だけでは safety underclaim になるため root exclusive safety を保守的に保持する。ownership は広げない。
- #59 process/task config と #66 artifact namespace の実装は含めない。

## Implementation Plan
catalog.ts に optional descriptor と built-in read/write/edit/subtree semantics を追加する。workflow/resource-inference.ts は core normalization/predicates、source extraction と dependency/conflict 検査を所有する。types.ts/compiler.ts/index.ts を互換追加し、workflow/test/resource-inference.test.ts で known/fallback 分離と canonical identity を検証する。core/runtime-pi/CLI/hot files は変更しない。

## Confirmation
- [ ] catalog descriptor を持つ job は input から scope を得る。
- [ ] unknown/root fallback read は dependency edge を作らず conflict に参加する。
- [ ] produces 重複は compile error、same-state writers は conflict のみ。
- [ ] workflow focused tests と dependency guard、lint/typecheck/build が通る。

## More Information
SPEC-034zSWvgiB9gHQpjguNJ3M、共有 ADR-034zGZpjoQlppwkkOs5gur D1/D4/D5。後続 #59/#66 または catalog で新 resource semantics を公開する時に再検討する。

## 2026-10-04 acceptance
coordinator: APPROVED: ADR gate。D1 surface の ToolMetadata.resources 追記は coordinator が merge 時に実行する。

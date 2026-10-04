---
id: "DESIGN-034zSWw1LYrRvcB471oQm2"
type: "design"
status: "approved"
title: "Workflow resource inference port 実装"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "../adr/workflow-resource-inference-sources.md"
    - "../specs/workflow-resource-inference.md"
  derived-by:
    - "../plans/implement-workflow-resource-inference.md"
  verifies: []
  verified-by:
    - "../test-specs/workflow-resource-inference.md"
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../discovery/workflow-resource-inference.md"

  defers: []
  deferred-by: []
---

# Workflow resource inference port 実装

## Context
overview → 本 design の順で読む。共有 ADR D1/D4/D5 が責務と固定型の正本。#64 ResourceInference を注入する方式を維持し、WorkspaceResourceInference({workspaceRoot,symlinkPolicy,tools}) を公開する。

## Scope
### In Scope
manifest 導出・ownership/依存/conflict 検査・理由の保存。
### Out of Scope
execution、policy schema、capability 拡張、artifact namespace、dynamic discovery。

## Components and Boundaries
- tool-runtime/catalog.ts に ToolMetadata.resources?: (input:JsonValue)=>ResourceManifestDeclaration を追加。副作用なしの descriptor。built-ins の path exact/subtree を catalog 内で宣言し workflow は tool 名の access semantics を再実装しない。
- workflow/resource-inference.ts が core normalizeResourceManifest/normalizeResourceClaim/claimContains/resourcesOverlap/claimsConflict を利用する。独自 canonicalization/glob/path joining は作らない。
- workflow/types.ts の ResourceInferenceJob に process?:ProcessResourceDeclaration を追加。task:string、inputs/outputs は core ScopeDeclaration 配列（binding 解決済み）。既存 WorkflowJob の JSON は不変。
- ResourceInferenceResult に optional conflicts と diagnostics を追加し fake port の互換を維持。CompiledWorkflow に同名 readonly 配列を optional に追加し compiler が clone して保存する。edges は既存 {from,to} のまま。
- ResourceConflict={jobs:[string,string],claims:[ResourceClaim,ResourceClaim]}、ResourceInferenceDiagnostic は kind（dependency/conflict/fallback）、job IDs、canonical claim pair または fallback reason を持つ discriminated union。

## Data and Control Flow
1. job ID の sort 順で、明示 resources path は string として core に渡す。未 materialize binding は拒否。catalog descriptor は input を clone して受け取る。
2. descriptor の known bucket と resources array の coverage を core claimContains で確認。非空 resources object では明示 bucket を優先、欠落 bucket を descriptor または fallback で補完。resources 省略/{} は safety を core fallback にするが known dependency reads は別に保持する。
3. Worker writeScope field を literal string[] / {path:string}[] として検査し core exact normalization。未知 $from が targets に含まれる場合は authority 未確定で conservative root writes。親 mkdir は root subtree safety、explicit writes の coverage 検査は exact target に対して行う。unknown authority で明示 writes/produces を確証できない場合 reject。
4. Process declaration は inputs/outputs 必須。outputs を exclusive produces/writes とし、通常の explicit declarations と coverage/ownership を検査。undeclared process は fallback 成功にしない。
5. job pair と claim pair の安定順で duplicate produces を最初に reject。mode は ownership overlap に影響しない。
6. produces×dependencyReads を core resourcesOverlap で調べ、自己 job は skip、edge は job pair で dedupe。fallback read は semantic knowledge がないため edge source から外す。
7. 全 safety claim pair を core claimsConflict で調べ conflict を annotate。conflict に edge は追加しない。diagnostic sort/dedupe は resourceKey を使う。
8. compiler は resources port 注入時は省略-only jobs でも呼ぶ。未注入なら従来の explicit resources rejection を維持。全 manifest と diagnostics/conflicts を clone し、既存 compileDag で union edges を検査する。

## Non-Functional Constraints
deterministic JSON、入力 snapshot、モデル/handler 初期化なし。complexity は job pair と claim pair の bounded compile-time comparison。core/tool metadata の公開 import のみ。

## Risks and Trade-offs
- mkdir safety を root exclusive にする Worker は over-serialize し得る。現在の runtime の ancestor mkdir access を漏らさないための明示 fallback。
- tool の absolute path 入力は既存 runtime が許可するが D5 declaration は relative。descriptor は absolute path を unknown bucket にし broad fallback（独自 path canonicalization はしない）。明示 D5 resource は core の rejection を保持。
- catalog の known access と宣言を矛盾させる no-access は reject。metadata 不明 tool に対する reads:[] は declared no access として受理。
- 未解決 $from による path scope は dynamic discovery を追加せず fallback。known literal leaf は可能な範囲で保持。

## Deferred Design Concerns
#59 executor、#66 artifact provenance、ResourceManager/engine は別 owner。core #65-A literal subtree symlink residual risk は継承する。

## Verification
focused workflow suite と catalog tests、dependency guard、pnpm lint/typecheck/build。canonical check deferred to coordinator merge run。approval 後に test-spec/tasks を作成し TDD で contract regression を先行する。

## Coordinator review amendments（2026-10-04）
msg_b28e978f1490: APPROVED: spec/design gate（P0/P1 なし）。
- ToolMetadata.resources の optional callback は D1 surface への additive refinement として local ADR の追記対象。共有 ADR 本体は coordinator 所有のため変更しない。
- ResourceInferenceJob.process は将来 #59 compiler/task-config 経路、または port 直接 caller が populate。undeclared tool:process の INVALID_REQUEST は port level で行使する（現行 JSON kind/schema は不変）。
- writeScope 無し + mutating capability tools は broad writes fallback。read-only + 明示 resources は writes=[]。
- port 注入時は全 job に manifest が付く observable change を plan/task と contract tests に含める。

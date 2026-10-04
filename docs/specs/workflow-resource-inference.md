---
id: "SPEC-034zSWvgiB9gHQpjguNJ3M"
type: "spec"
status: "approved"
title: "Workflow resource inference 契約"
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
  implemented-by:
    - "../plans/implement-workflow-resource-inference.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/workflow-resource-inference.md"

  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-resource-inference.md"

  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/workflow-resource-inference.md"
  references:
    - "../adr/workflow-resource-inference-sources.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"

  defers: []
  deferred-by: []
---

# Workflow resource inference 契約

## What Should Be Built
全 job の canonical ResourceManifest、producer→consumer edges、resource conflicts、推論理由を返す ResourceInference port 実装。

## Why Now
#64 の port が未実装で、宣言 output に依存する consumer が手書き needs を要求する。複数 output owner を曖昧に選ぶと workflow の意味が変わる。

## Users and Value
親 Coding Agent と後続 engine/scheduler が compile 前に依存と排他要求を確認できる。

## Scope
### In Scope
- workspace resource 宣言、catalog の既知 tool access semantics、Worker writeScope、process named-task declaration seam。
- #64 compiler の既存 port/CompiledJob.manifest に結果を接続し conflict と理由を公開する。
### Out of Scope
- ResourceManager、scheduler、engine、CLI、process executor、artifact namespace の実装、動的 graph refinement、filesystem の現存一覧から output を推測すること。

## Requirements
1. ADR D4 の canonical absolute real-path identity、mode、exact/subtree、Windows case、glob ownership を core の単一境界から得る。
2. ADR D5 の resources 省略/{} は安全性 manifest の conservative fallback。非空 resources object の明示 array は宣言として扱い、空 array は no-access。known tool descriptor がある場合、非空 object の未指定 bucket は descriptor から補完可能。全 resources 省略でも port を注入した compiler は port を呼ぶ。未注入時の従来契約は維持する。
3. catalog が公開する known read/write を導出し、明示宣言による既知 access の underclaim は拒否する。unknown descriptor や deferred $from path は root fallback。安全性 fallback だけを根拠に edge を作らない。known metadata read と明示 reads は dependency の根拠になる。
4. Worker writeScope は input targets の authoritative exact mutation scope。未解決 targets は broad fallback とし、狭い writes/produces の確証には使わない。明示 writes/produces が authoritative target 外、または既知 targets を覆わない writes 宣言なら拒否。writeScope 無しの mutation tool Worker は broad writes、read-only capability Worker は明示 resources がある場合 writes=[]。writeScope input の不正値は拒否。
5. writeScope の親 mkdir も runtime が許可するため、safety writes に workspace root subtree を追加する（exact targets の ownership や dependency を広げない）。この overclaim と制約を diagnostics へ明示する。
6. produces と known/declared reads の overlap は producer→consumer edges。needs/$from との union は compiler の既存 DAG 検査へ通す。自己の reads/produces は edge にしない。
7. 異なる job の produces 同士の exact/subtree overlap は needs による順序があっても INVALID_REQUEST。決定的な job pair と claim pair を error details に含める。
8. reads/writes/produces の exclusive overlap を conflict として注釈し semantic order を発明しない。edge/conflict の理由に job id と canonical claims を載せる。
9. process は現行 JSON kind を増やさず tool:process の将来 port source で表す。named task の inputs/outputs は明示必須（[] は可）。undeclared process は推論不可として INVALID_REQUEST。outputs は produces と writes。実 executor/config loader は #59。
10. caller の jobs/input/metadata を変更せず、同じ job 集合の順序を変えても manifest/edges/conflicts/diagnostics の JSON が一致する。

## Acceptance Criteria
- [x] AC1: built-in read/write/edit/ls/grep/find/count_tokens の metadata と入力から scope が得られ、metadata 未公開 tool は fallback。
- [x] AC2: Worker 明示 reads、writeScope targets、既知 scope 矛盾拒否、親 mkdir safety の挙動が検証できる。
- [x] AC3: future output の produces∩known/declared reads の edge と compiler の needs/from dedupe、cycle 拒否。
- [x] AC4: writes∩writes は conflict のみ、fallback read は invented dependency を作らない。
- [x] AC5: duplicate exact/subtree/canonical alias output は stable INVALID_REQUEST。
- [x] AC6: resources 省略/{} と reads:[] の差を検証する。
- [x] AC7: job/claim 順序と caller mutation によらず byte 一致し全 job manifest を返す。
- [x] AC8: core normalization/predicates を再利用し boundary escape と glob producer 拒否が伝播する。
- [x] AC9: process declaration seam は typed、inputs/outputs 欠落を拒否し outputs→inputs の edge を返す。

## Deferred Design Concerns
process 実行は #59、artifact provenance は #66、dynamic discovery は issue #65 triage の既存 exclusion。既存 literal subtree symlink/hard-link/外部差替えの residual risk は core #65-A と同じ境界。

## Sources and Relations
accepted ADR-034zGZpjoQlppwkkOs5gur の D1/D4/D5 と issue #65 Triage 2026-10-04 に従う。

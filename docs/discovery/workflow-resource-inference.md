---
id: "DISC-034zSWuKOprLRHraOiRn31"
type: "discovery"
status: "resolved"
title: "Workflow inference の責務と情報源"
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
    - "../ideas/workflow-resource-inference.md"

  derived-by:
    - "../adr/workflow-resource-inference-sources.md"
    - "../specs/workflow-resource-inference.md"

  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"

  defers: []
  deferred-by: []
---

# Workflow inference の責務と情報源

## Exploration Goal
#64 port に #65-A の canonical normalization を接続し、未知の safety scope を semantic dependency と混同せず output ownership を扱う。

## Constraints
共有 ADR D1/D4/D5 を維持する。core/runtime-pi/CLI/hot files は変更禁止。dispatch が briefing-flow の SUBAGENT-STOP に該当するため、profile 再生成をせず明示 stack（idea/discovery/spec/design/plan/test-spec/task/impl-doc）を使用する。Entry Decision A-2: metadata 表現と unknown safety scope の扱いを比較する。

## Findings
- core normalization.ts は omitted bucket を root subtree、空配列を no access、produces を ownership として処理する。
- workflow compiler.ts は explicit resources がある時に全 job を port へ渡す。inferred edges は compileDag で unknown/self/cycle/dedupe 検査される。
- tool catalog.ts は input/output schema のみで access semantics がない。tool 名を workflow で switch すると handler の責務と重複する。
- WorkerDefinition.cheaper.writeScope は input field 名。runtime は targets の exact display membership を検査し、親 directory の mkdir を許可する。

## Alternatives
| Option | 利点 | 不利点 |
| --- | --- | --- |
| workflow で tool 名ごとの path 規則を持つ | catalog 変更が不要 | semantics が重複し catalog contract の source にならない |
| catalog に optional pure resource descriptor を公開する | semantics の正本を catalog に保つ | optional surface の追加が必要 |
| fallback root overlap を全て DAG edge にする | 単純 | unknown read を依存と誤認し false cycle を作る |

## Tentative Conclusions and Hypotheses
仮説: catalog の optional input-to-declaration descriptor を core normalization へ渡し、known reads と safety fallback を分離する方法が責務に合う。ownership 重複は core resourcesOverlap で検出し、conflict は annotation のみとする。新 ADR は不要（D4/D5 の実装細化）。

## Open Questions
設計の precedence と process seam を coordinator に review してもらう。追加の先行実装 blocker はない。

## Promotion Candidates
spec を作成し acceptance checks と conservative fallback の範囲を固定する。

## Sources and Relations
issue #65 dispatch brief、共有 ADR D4/D5、上記 source を確認した。

## 2026-10-04 review update
msg_448756273b23 により optional metadata と knowledge/safety 分離の local ADR 起草を承認。共有 ADR の契約変更は行わない。

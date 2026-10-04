---
id: "TSPEC-034zTsgOgsTzuFTcEztxzd"
type: "test-spec"
status: "proposed"
title: "workflow-public-interface"
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
    - "docs/designs/workflow-file-interface.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-file-interface.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow 公開 interface の utility 契約

## Purpose
親 agent が private implementation を読む必要なく invocation を組めるようにする。interface command を廃止する時は同じ保証を別 surface に移管する。

## Feature
workflow interface の compact JSON/text と fail-closed error envelope。

## Rules
- Rule: result は #64 WorkflowInterface の schemaVersion/inputs/outputs のみ。
- Rule: metadata 検証は行い、jobs/prompt/tool parameters/resources/bindings を正常出力へ含めない。
- Rule: model/Worker/tool/process を実行しない。
- Rule: 許可 option は cwd/format/quiet、input - を含む他 command option は拒否。
- Rule: structural/schema/default error が Worker metadata error に先行する。

## Examples
- Example: required target と default limit を持つ workflow は値なしで interface を返す。
- Example: 不在/malformed/schema violation は一つの error envelope と exit 2、text error は既存 formatText。
- Example: invalid schemaVersion と壊れた Worker metadata を同時に与えて canonical definition error が先に出る。
- Example: interface --input - / workflow run / bare workflow は INVALID_REQUEST。

## Guarantees
AC1/2/4/5/8 と utility worker:null、canonical --cwd、正常時 usage/model artifact 不生成を保護する。

## Non-goals
workflow run、bundle refresh、real provider smoke、generic inspect。

## Risk
private dump は親 context を浪費し、execution side effect は discovery utility の責務を破る。

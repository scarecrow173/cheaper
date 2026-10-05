---
id: "TSPEC-034AJ8RBszSVnXlZydfnyU"
type: "test-spec"
status: "proposed"
title: "Workflow verification evidence と dependency direction"
created: "2026-10-05"
updated: "2026-10-06"
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
    - "../designs/one-shot-workflow-execution-and-cli-composition.md"
  derived-by: []
  verifies:
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
  verified-by: []
  references: ["../test-specs/operation-execution-metrics.md"]
  defers: []
  deferred-by: []
---

# Workflow verification evidence と dependency direction

## Purpose

部分的なfake/structural evidenceを製品全体や実モデル成功へ拡大しないため、verificationの範囲を保持する。別のcanonical verification契約に移行する場合に再評価する。

## Feature

Workflow verification evidence と dependency direction。

## Rules

- Rule: workflow/tool-runtimeからruntime-pi/Pi SDKをimportしない。
- Rule: per-job/global metricsはowner/invocation正本でWorker snapshotsを足さない。
- Rule: focused tests、型/build/lint、CLI単独tests、scoped doc auditの範囲を区別して記録。

## Examples

- Example: source import/export/dynamic-importguardがforbiddendependencyを検出する。
- Example: tool-onlyrunのmodel呼出し0はprovider品質の証拠として扱わない。
- Example: fake deterministic E2Eがpassしてもcanonical merge checkとrealprovider smokeを実施済みと記録しない。

## Guarantees

実行したcommands/outcomesと未実施項目をIR/reportへ記録し、coordinatorreview前に完了を自己承認しない。

## Non-goals

canonical mise run checkのworker実行、実モデル費用/速度/token savings。

## Risk

architecture drift、誤った完了/費用主張、review不能な記録。

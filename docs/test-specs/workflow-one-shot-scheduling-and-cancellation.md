---
id: "TSPEC-034AJ8xcmAsWo69hx3u1vW"
type: "test-spec"
status: "proposed"
title: "One-shot DAG の依存・capacity・terminal 保証"
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
  references: ["../test-specs/workflow-resource-inference.md"]
  defers: []
  deferred-by: []
---

# One-shot DAG の依存・capacity・terminal 保証

## Purpose

one-shot collect-all と論理終了/物理settleの区別を future scheduler 改変から守る。polling 方式への変更時には同等保証を確認してから退役する。

## Feature

One-shot DAG の依存・capacity・terminal 保証。

## Rules

- Rule: compiled semantic edges だけを使い、全 predecessor 成功後に input を materialize。
- Rule: admitting/ready は executor slot を消費せず active executor を maxJobs 以内に保つ。
- Rule: producer 非成功は affected descendants だけを skip、独立 branch は継続。
- Rule: first outer abort は一回だけ伝播し、logical detach 後の結果は immutable。

## Examples

- Example: maxJobs2 で独立A/Bが重なり、fan-in Cは両方成功後に start。
- Example: Aのresource待機中も非conflict Bは start、compiled edges は変わらない。
- Example: A failure でC skip、B公開outputを false outcome に残す。
- Example: preabort は admission/executor0、outer timeout は engine start 基準、grace超過 parent は actual finally まで保持。
- Example: late success/error/admission grant が terminal/result を書き換えず rejection を観測する。

## Guarantees

jobs と outputs は code-unit 昇順、diagnostics は2048 UTF-8 bytes、private payload/details/cause を公開しない。

## Non-goals

実行時刻の完全決定性、persistent polling、cross-invocation scheduling。

## Risk

deadlock、無期限待機、unrelated branch喪失、late-result leak。

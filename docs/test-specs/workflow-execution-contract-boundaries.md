---
id: "TSPEC-034AJ8stqUyJWIgj6oaOzt"
type: "test-spec"
status: "proposed"
title: "Workflow の policy・schema・literal scope 境界"
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
  references: ["../test-specs/workflow-compiler-contract-guarantees.md"]
  defers: []
  deferred-by: []
---

# Workflow の policy・schema・literal scope 境界

## Purpose

policy/compiler の正本を adapter に複製せず、project が権限・容量を拡大できない契約を保持する。Workflow 契約を廃止する際に再評価する。

## Feature

Workflow の policy・schema・literal scope 境界。

## Rules

- Rule: trusted merge 後に defaults を補完し project の maxJobs/artifact caps は min restriction。
- Rule: maxJobs/maxReadBytes は正、他 artifact caps は非負の safe integer。
- Rule: job output schema は compiler が所有し JSON snapshot を返す。
- Rule: literal writeScope 抽出は core が一箇所で所有する。

## Examples

- Example: user maxJobs4 と project8 の effective は4、project2 は2、trusted 未指定/project16 は1。
- Example: cap0 は保存/preview 無効、read0/fraction/unsafe integer は拒否。
- Example: schema が number の job が string を返すと INVALID_OUTPUT、valid object は caller と共有しない。

## Guarantees

operation/process の既定を維持し、timeout は既存 policy 上限だけを狭める。

## Non-goals

既存 compiler 全契約・process scheduling 本体・実 provider 品質。

## Risk

正本の分散、policy widening、invalid output による downstream corruption。

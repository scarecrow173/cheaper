---
id: "TSPEC-034AJ8MX15GFDduILAiVko"
type: "test-spec"
status: "proposed"
title: "workflow run の入力・public envelope・互換性"
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
  references: ["../test-specs/workflow-public-interface.md", "../test-specs/workflow-loader-contract.md"]
  defers: []
  deferred-by: []
---

# workflow run の入力・public envelope・互換性

## Purpose

親agentが一つの正しい結果を消費できるCLI境界を保つ。workflow run surfaceを廃止する際のみ退役を検討する。

## Feature

workflow run の入力・public envelope・互換性。

## Rules

- Rule: --input omissionは{}、明示'-'はstdin、parsed.tokensで存在を判別。
- Rule: full allowlistだけを受け付け、definition pathのloader/deny境界を維持。
- Rule: runtime/storeは初回admission、modelは初回Workerの共有lazy promise、tool-onlyのmodel初期化は0。
- Rule: success/failureともpublic result/jobs/metaを一つのstdoutenvelopeに保持。
- Rule: workflow variantはgeneric text errorより先にdispatchしusage.jsonlの対象から外す。

## Examples

- Example: public output tokens があってもcount recordを作らず、private sentinelはJSON/textに出ない。
- Example: failed Aと独立成功BはnonzeroとB outputを返し、pre-engineerrorも--format/--quietを保持。
- Example: tool+Worker+artifact bounded readがsameinvocationを借り、owner snapshotsを合算しない。
- Example: interfaceはexecutor/model0、single-worker run/aliasと既存read/input/envelopeの動作を維持。

## Guarantees

stdout一回、diagnostics stderr、jobs/outputErrors昇順、cleanup後metadataは取得可能性を宣言しない。

## Non-goals

実providersmoke、SEA/APM配布の再生成、費用比較。

## Risk

private payload漏洩、workflowをsingle-model usageへ誤分類、既存CLI回帰。

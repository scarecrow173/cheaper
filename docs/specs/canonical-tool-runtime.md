---
id: "SPEC-034zILh8aauVSReoD33mtV"
type: "spec"
status: "approved"
title: "Canonical Tool Runtime 抽出"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/63"]
  derives-from: ["../discovery/canonical-tool-runtime.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Canonical Tool Runtime 抽出

## What Should Be Built

Pi Worker と direct caller が一つの Tool Runtime を通じて deterministic tool を実行する。
accepted ADR D1 の catalog/result と D7 の invocation ownership seam を公開する。

## Why Now

Workflow の tool job が Pi session/model を作らずに既存 tool を利用する必要がある。

## Users and Value

Worker/Workflow 実装者が schema、安全性、結果、計測を共有できる。

## Scope

### In Scope

- read/grep/find/count_tokens の canonical handler と input/output schema catalog。
- write/edit/ls の helper-backed handler 注入、未登録 handler の明示拒否。
- MutationJournal の正本移動と旧 import/export の class identity 維持。
- Pi thin adapter、budget/meter の一回適用、single-Worker invocation 所有と dispose。
- Worker Runtime / Tool Runtime / Workflow Engine の責務を区別する architecture 文書の更新（自分の design doc）。

### Out of Scope

- scheduler / OperationManager / ResourceManager / lease の実装（#56/#57/#61）。
- Workflow executor、CLI、core、dependency/hot file の変更。
- Pi agent loop と成熟した mutation helper semantics の再実装。

## Requirements

- direct と Pi は同じ canonical result と throw を保持する。
- canonical 入口が malformed input を schema 検証し、Pi validation を前提としない。
- input validation、admission/budget/meter の責務を一箇所に持つ。
- D7 optional binding 型を公開し、未実装 service を成功する no-op として公開しない。

## Acceptance Criteria

- [x] 同一実 fixture に対し read/grep/find/count_tokens が Pi 無しで実行でき、Pi adapter と content/details、truncation/count、boundary/deny が一致する。注入 write/edit は writeScope 拒否も一致する。
- [x] direct path で未注入の write/edit/ls は明示拒否される。ToolCatalog.get/list は Pi 初期化なし・副作用なしに schema を返す。
- [x] malformed input を canonical schema で拒否し、symlink escape/huge-file cap/grep batchMaxBytes の既存 test を維持する。
- [x] toolCalls/rejectedToolCalls/sourceTokens が正確で、sibling admission と abort の既存 pin を維持する。
- [x] journal class identity、same-file write→edit ordering、partial mutation 報告を保存する。
- [x] @earendil-works/* を含む Pi 依存/runtime-pi/workflow を import しない dependency guard が通る。
- [x] runWorker が single-Worker runtime を所有し finally で abort/drain/dispose する。
- [x] architecture 文書が Worker Runtime / Tool Runtime / Workflow Engine の責務を区別して記述する。

## Sources and Relations

GitHub issue #63 本文と Triage 2026-10-04、共有 ADR D1/D7 を契約とする。


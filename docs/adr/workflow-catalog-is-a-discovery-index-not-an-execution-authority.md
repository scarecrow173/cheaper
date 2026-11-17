---
id: "ADR-034ZixZvVK6PLmS1LMRJqu"
type: "adr"
status: "accepted"
title: "Workflow catalog is a discovery index, not an execution authority"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/201"
    - "https://github.com/scarecrow173/cheaper/issues/200"
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
  related:
    - "../specs/workflow-catalog-issue-201.md"
    - "../specs/workflow-file-interface.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-catalog-issue-201.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Workflow catalog is a discovery index, not an execution authority

## Context and Problem Statement

Epic #200 の discover/create 導線が参照する既存 Workflow 候補の軽量一覧が
必要になった。catalog を「どこまで信頼できる情報源」とするかで設計が
分かれる: 実行・権限の信頼源にすると named lookup・自動 routing の領域に
踏み込み、#69 で延期した責務と衝突する。もう一つの論点は、manifest が
構造的に破損したとき `workflow list` をどう振る舞わせるかである。

## Considered Options

* catalog を発見用 index に限定し、実行は明示 path +
  既存 compiler/policy に委ねる。manifest 破損は error envelope で
  fail closed、entry 単位の異常は一覧内 diagnostics で報告する。
* catalog を権限・実行の正本とし、`workflow run <name>` の named
  lookup や grant 導出の根拠にする。
* manifest 破損を含むあらゆる異常を `ok:true` + diagnostics に畳み、
  list 自体は常に成功扱いにする。

## Decision Outcome

Chosen option: "catalog を発見用 index に限定し、manifest 破損は fail
closed、entry 異常は diagnostics"。catalog の正本は Workflow 定義 file
自体であり、catalog entry はその写しに過ぎない。正本を2つにすると
stale 時にどちらが権限かの規則が必要になり、#69 が延期した named
lookup の設計を事実上先取りする。manifest が parse 不能・非対応
schemaVersion・上限超過のときは「一覧可能な情報が無い」のであり、空
カタログとして成功を返すと破損を見逃すため error envelope で閉じる。
対照的に entry 単位の異常(missing path・path escape・duplicate id)
は「一覧できたが内容に問題がある」状態なので `ok:true` + diagnostics
で報告する。

## Implementation Plan

* Affected paths: `crates/cheaper-workflow/src/catalog.rs`(新規)、
  `crates/cheaper-cli/src/args.rs`・`cli.rs`・workflow command module、
  `docs/guides/` 配下。
* Constraints: catalog から grant/policy 変更・workflow 実行・named
  lookup を一切導出しない。entry path の検証は loader と同一の
  `resolve_workspace_path` + deny/symlink policy 入力を使う。private
  jobs/prompt/binding を一覧に含めない。

## Verification

* [ ] `workflow list` が catalog なし workspace で空一覧を返し、明示
  path の既存 command を妨げない。
* [ ] malformed manifest が error envelope + 非 0 exit になる。
* [ ] duplicate id・missing path・path escape が diagnostics で報告
  され、黙った last-wins や別 entry 選択が起きない。

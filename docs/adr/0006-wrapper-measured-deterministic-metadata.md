---
id: "ADR-0006"
type: "adr"
status: "accepted"
title: "決定論的メタデータは wrapper が実測する"
created: "2026-09-15"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "../specs/0003-cheap-writer-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0001-cheaper-v1-pi-sdk.md"
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

# 6. 決定論的メタデータは wrapper が実測する

## Context and Problem Statement

changed files・diff・added/deleted lines・exit status・duration・provider/model・token usage・tool count・turn count をどこから取得するか。§2.6 は「deterministic な情報を LLM に自己申告させない」と明記する。

## Decision Drivers

* §2.6: deterministic な情報は filesystem/Git 等から機械的に取得
* §35: usage/telemetry は最低限 provider/model/input/output/cached tokens/duration/turn count/tool count
* §38: changed files/diff 統計を wrapper が実測できることが V1 成功条件

## Considered Options

* (1) LLM に result 内で自己申告させる
* (2) git diff に依存して変更を計測する
* (3) wrapper が filesystem snapshot + Pi events から実測する

## Decision Outcome

Chosen option: "(3) wrapper が filesystem snapshot + Pi events から実測する", because LLM 申告は信頼できず、git 依存は git repo 非依存という要件を壊すため。wrapper が filesystem の前後差分と Pi の usage イベントから機械的に取得する。詳細アルゴリズム（snapshot 手法・diff 計算）は design で確定する。

### Consequences

* Good, because LLM の誤報・幻覚に依存しない正確なメタデータが得られる。
* Good, because git repo でない workspace でも動く。
* Bad, because snapshot/diff 実装のコストと、大規模 workspace での計測コストがある。

### Confirmation

* changed files / diff 統計が実 filesystem 差分と一致するテスト（§42 Execution / cheap-writer カテゴリ）。

## Implementation Plan

* Affected paths: `packages/runtime-pi/`（予定 path）— filesystem snapshot/diff・Pi event usage 収集、`packages/core/`（予定 path）— result metadata 型
* Patterns to follow: usage は `turn_end` の `message.usage`（input/output/cacheRead/cacheWrite/totalTokens/cost）から集計、tool/turn count はイベントカウント
* Patterns to avoid: LLM にメタデータを書かせること、git コマンドへの依存（git 非依存を基本とする）
* Dependencies or configuration: なし

## Verification

* [ ] bulk-reader で filesystem 変更 0 件が実測されるテスト
* [ ] cheap-writer で changed files・added/deleted lines が実差分と一致するテスト
* [ ] usage（provider/model/tokens/duration/turn/tool count）が result meta に載るテスト

## Pros and Cons

### Option 1: LLM 自己申告

* Good, because 実装不要。
* Bad, because §2.6 に明確に反し、誤報を検証できない。

### Option 2: git diff 依存

* Good, because 実装が楽で正確。
* Bad, because git repo 前提になり非 git workspace で動かない。abort 時の untracked 部分変更も扱いにくい。

### Option 3: wrapper 実測

* Good, because git 非依存で全ケースをカバー。
* Bad, because snapshot・diff・symlink 処理の実装コスト。

## More Information

* 引継ぎ §2.6, §35, §38, §42
* Pi usage イベント: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md

## 更新履歴

### 2026-09-15: 実測手段の具体化

* 「wrapper 実測」の実装は mutation journal に確定: write/edit の `operations` 層でファイルごとの書き込み前内容（存在しなければ null）を記録し、終了時に disk 上の最終内容と比較、行差分は `diff` パッケージ `diffLines` で算出する。
* 全域 snapshot を不要とした理由: cheaper の tool vocabulary に shell が無く、全書き込みが cheaper 提供 operations を通るため、journal が書き込みの全域をカバーする。

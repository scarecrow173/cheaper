---
id: "ADR-0005"
type: "adr"
status: "proposed"
title: "Model tier 抽象と Model Resolver"
created: "2026-09-15"
updated: "2026-09-15"
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
  implemented-by: []
  depends-on:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
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

# 5. Model tier 抽象と Model Resolver

## Context and Problem Statement

Worker が使うモデルをどう指定するか。Worker は環境非依存であるべきで、具体モデル名や provider を持つべきではない（§13–§14, §27）。

## Decision Drivers

* §13: Worker は `cheaper.model.tier`（low/medium/high）のみ持つ。具体モデル名を持たない
* §14: 利用可能モデル一覧から tier 分類・選択を行う
* §27: model policy（`maxTier` / `tierFallback`）で制御

## Considered Options

* (1) Worker が具体モデル名を指定する
* (2) CLI で provider+model を都度指定する
* (3) tier 抽象 + Model Resolver（分類 priority: user override > built-in > Pi metadata heuristic > unknown→medium）

## Decision Outcome

Chosen option: "(3) tier 抽象 + Model Resolver", because Worker を環境非依存に保ちつつ、利用可能モデルからの選択と上限制御を Runtime 側に集約できるから。

### Consequences

* Good, because Worker Definition がモデルカタログに依存しない。
* Good, because user が override 設定で分類を制御できる。
* Bad, because tier 分類ロジック（built-in 分類 + heuristic）の保守が必要。

### Confirmation

* classification priority と unknown→medium、maxTier 超過 `POLICY_VIOLATION`、fallback higher-only がテストで確認されること。

## Implementation Plan

* Affected paths: `packages/core/`（予定 path）— tier 型・分類器・resolver、`packages/runtime-pi/`（予定 path）— `ModelRuntime.getAvailable()` からのモデル一覧取得
* Patterns to follow: classification priority `user override > cheaper built-in classification > Pi metadata heuristic > unknown → medium`（§14）
* Patterns to avoid: 未知 model を `low` に分類すること、Worker Definition への具体 model list、lower fallback
* Dependencies or configuration: Pi `ModelRuntime.getAvailable()`、`Model` メタデータ（`cost`, `contextWindow`, `maxTokens`, `reasoning` 等）

確定内容（§13–14, §27）:

* tier: `low`（cheap/fast/quota-friendly）/ `medium`（general-purpose）/ `high`（expensive/scarce/frontier-class）。order: low < medium < high
* `model.maxTier`: 要求 tier が超えたら `POLICY_VIOLATION`
* `model.tierFallback`: `none`（無ければ `MODEL_TIER_UNAVAILABLE`）/ `higher`（上位のみ探索、maxTier 超過なし）。lower fallback なし
* unknown model は `medium`。`low` にしない
* same-tier の provider/model preference は cheaper config 側で定義
* Worker Definition に concrete model / provider / credentials を書かない（§12）

## Verification

* [ ] 分類 priority（override > built-in > heuristic > unknown→medium）の単体テスト
* [ ] unknown model が `medium` になるテスト
* [ ] `tierFallback: higher` が上位のみ探索し `maxTier` を超えないテスト
* [ ] 要求 tier 超過で `POLICY_VIOLATION` exit 4、利用不可で `MODEL_TIER_UNAVAILABLE` exit 5 のテスト

## Pros and Cons

### Option 1: Worker が具体モデル指定

* Good, because 実装が単純。
* Bad, because 環境差で壊れ、§12/§13 の設計思想に反する。

### Option 2: CLI で provider+model 指定

* Good, because 呼び出し側が制御できる。
* Bad, because 毎呼び出しで指定が必要で、Worker の意図（tier）が伝わらない。

### Option 3: tier 抽象 + resolver

* Good, because Worker の portability と運用者の制御を両立。
* Bad, because 分類ルールの保守コスト。

## More Information

* 引継ぎ §13–§14, §27
* Pi `Model` 型フィールド: https://github.com/earendil-works/pi — `@earendil-works/pi-ai` 0.85.1 `dist/types.d.ts`

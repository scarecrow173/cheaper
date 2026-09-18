---
id: "ADR-034r6mq2Glfp3YlWyQdlIO"
type: "adr"
status: "accepted"
title: "大規模 fixture テストの eval budget は test 側で明示注入して環境非依存にする"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/39"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/runtime-pi/test/tools/grep.test.ts"
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
    - "../specs/grep-eval-budget-test-stability.md"
    - "../specs/grep-batch-max-bytes-line-payload-invariant.md"
    - "../impl/ir/grep-batch-max-bytes-invariant.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 大規模 fixture テストの eval budget は test 側で明示注入して環境非依存にする

## Context and Problem Statement

`grep.test.ts` の 16MB / 100MB fixture を使う構造検証テストが
production 既定の eval budget（per-file 5s / total 60s）を暗黙に
継承しており、高負荷環境で累積 regex 評価時間が予算を超えて file
が `[skipped: timeout]` になり flake する（issue #39）。テストの
目的は巨大入力での正しさと bounded 挙動の検証であり、eval
budget の timeout 自体は `fakeClock()` 注入の決定的テストで既に
カバーされている。

## Considered Options

* A: 大規模 fixture 用 tool factory（`bigTool`）に十分な
  eval budget を明示注入する
* B: production 既定値（`GREP_EVAL_PER_FILE_MS` 等）を引き上げる
* C: fixture を小さくして既定予算内に収める
* D: 当該テストを CI 専用 stress test として分離する

## Decision Outcome

Chosen option: "A", because テストが検証したいのは timeout 挙動
ではなく構造的正しさであり、予算を実時間に依らず十分な値へ
明示注入するのが最小変更で意図も明確。B は production の
safeguard を弱めるため却下。C は 100MB 級の走査経路カバレッジを
失うため却下。D は本修正で flake 原因が除去されるため過剰。

## Implementation Plan

* Affected paths: `packages/runtime-pi/test/tools/grep.test.ts`
  （`bigTool` factory とその利用テスト）
* Constraints: production 側の既定値・`fakeClock()` ベースの
  決定的 budget テスト・fixture 内容・assertion 内容は変更しない

## Verification

* [ ] 大規模 fixture テストが `[skipped: timeout]` を出さない
* [ ] `packages/runtime-pi` の `pnpm test` が pass する
* [ ] `mise run check` が pass する

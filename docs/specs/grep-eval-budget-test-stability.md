---
id: "SPEC-034r6kd649rTDnOi828voj"
type: "spec"
status: "implemented"
title: "Stabilize environment-sensitive grep eval-budget tests"
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
  depends-on:
    - "./grep-batch-max-bytes-line-payload-invariant.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
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
---

# Stabilize environment-sensitive grep eval-budget tests

## What Should Be Built

`packages/runtime-pi/test/tools/grep.test.ts` の大規模 fixture
（16MB / 100MB）を使う構造検証テストが、production 既定の eval
budget（`GREP_EVAL_PER_FILE_MS = 5000` /
`GREP_EVAL_TOTAL_MS = 60000`）を暗黙に継承しているため、負荷の
高い環境で regex 評価の累積実時間が per-file 予算を超え、対象
file が `[skipped: timeout]` 扱いになって flake する。

これらのテストの目的は「巨大入力でも正しい結果・batch
protocol・bounded memory」を検証することであり、eval budget の
timeout 挙動ではない。budget 挙動は `fakeClock()` を注入した
決定的テストで別途カバー済み。大規模 fixture 用の tool factory
に、環境速度に依存しない十分な予算を明示注入して、構造的
assertion を環境非依存にする。

## Why Now

issue #36 の検証中に worker 3 並列稼働下の Windows box で 1-3 件
flake を観測。未変更の main でも同条件で再現し、負荷低下後は
pass、CI では恒常 pass — 実装起因ではなく環境依存の timing
flake。`mise run check` の安定性は canonical verification
contract の信頼性に直結するため、ローカル高負荷環境でも
flake しない状態にする。

## Users and Value

- contributor: 高負荷・低速環境でも `mise run check` / `pnpm test`
  をそのまま信頼できる
- reviewer / CI: flake 由来の再実行や「環境のせい」判定コストが
  消える

## Scope

### In Scope

- `grep.test.ts` の大規模 fixture（`bigRoot` 配下）を走査する
  テストが使用する tool factory に、環境非依存の eval budget を
  明示する
- 対象は構造・正しさを検証するテスト（known match in tail /
  fh.read chunk bound / batch bound / bounded heap）に限定する

### Out of Scope

- production 側の `GREP_EVAL_PER_FILE_MS` / `GREP_EVAL_TOTAL_MS`
  既定値の変更（budget 自体は正しい safeguard）
- eval budget・deadline・abort の挙動テスト（`fakeClock()` で
  既に決定的）
- vitest timeout・fixture サイズの見直し（現行値で十分）
- CI 専用 stress test への分離（本修正で不要になる想定）

## Requirements

- 大規模 fixture テストは machine の速度・負荷に依存せず、
  regex eval が実時間でいくらかかっても構造 assertion を評価
  できること
- eval budget の timeout/skip/truncate 挙動を検証する既存の
  決定的テスト（`fakeClock()` 注入）は変更しない
- fixture 生成・サイズ（16MB / 100MB）とテストの assertion
  内容は変えない — budget 注入のみで flake を解消する

## Acceptance Criteria

- [ ] 大規模 fixture を使う全テストで、file が eval budget 由来の
  `[skipped: timeout]` にならない
- [ ] `packages/runtime-pi` の test suite が高負荷環境を含めて
  安定して pass する
- [ ] `mise run check` がローカル・CI の双方で pass する

## Deferred Design Concerns

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Sources and Relations

- issue #39（#36 検証中に観測、#37 final audit で起票）
- `IMPL-034r05iHy2gdnpwO94elRp`（grep batchMaxBytes invariant —
  eval budget 機構を導入した実装系譜）

---
id: "DESIGN-034r6oM1pW6bz2SSvR3rWT"
type: "design"
status: "approved"
title: "grep eval-budget test stabilization via explicit budget injection"
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
    - "../adr/grep-eval-budget-test-stability-budget-injection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/grep-eval-budget-test-stability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# grep eval-budget test stabilization via explicit budget injection

## Context

`packages/runtime-pi` の grep tool は worker_threads へ batch 送信
して regex 評価を行い、実時間で計測した累積 eval budget
（`evalPerFileMs` 既定 5000ms、`evalTotalMs` 既定 60000ms）を
超えると file を timeout skip / 走査全体を打ち切る。

`grep.test.ts` には `bigRoot` 配下の大規模 fixture（16MB・100MB）
を使う構造検証テストが4件あり、いずれも `bigTool()` factory 経由で
既定 budget を継承している。高負荷環境では累積 eval 時間が 5s を
超え、対象 file が `[skipped: timeout]` になって assertion が
失敗する（issue #39）。

eval budget 自体の挙動（累積・skip・worker 再生成・total
打ち切り）は `fakeClock()` を `now` へ注入した決定的テストで
カバー済みのため、大規模テストは予算計測に依存させない。

## Scope

### In Scope

- `bigTool()` factory に環境非依存の eval budget（実時間で
  到達し得ない十分な値）を既定として注入する
- `tool()` factory（小 fixture・既定 budget を使うテスト群）は
  変更しない — 既定 budget を検証している挙動テストが混在する
  ため、注入は大規模 fixture 用の経路に限定する

### Out of Scope

- production 側の `limits.ts` 既定値変更
- `fakeClock()` ベースの budget 挙動テストの変更
- fixture 生成・サイズ・assertion の変更

## Components and Boundaries

- `bigTool(opts)`: `createGrepTool({ root: bigRoot, symlinkPolicy:
  "deny", evalPerFileMs: <十分な値>, evalTotalMs: <十分な値>,
  ...opts })` とし、個別テストからの override は `opts` 経由で
  引き続き可能にする
- 予算値は vitest の test timeout（120s）を大きく上回る有限値
  （例: 600_000ms）とする — 「budget 機構を無効化」ではなく
  「実時間で到達し得ない予算」として意図を表現する

## Data and Control Flow

- 変更は test factory の option 注入のみ。tool 内部の計測・
  budget 判定・出力 contract には一切触れない

## Non-Functional Constraints

- 大規模テストは負荷・マシン性能に関わらず deterministically に
  構造 assertion を評価できること
- 小 fixture の既存テスト（既定 budget 依存・`fakeClock()`
  注入）への影響がないこと

## Risks and Trade-offs

- 予算を大きくしすぎると、本来 budget で守られる「無限に重い
  regex」系の回帰を大規模テストが検出できなくなる → 当該
  テスト群はその責務を持たない（fakeClock テストが責務）ため
  許容。万一の暴走は vitest の test timeout が最終防御
- `opts` spread を budget 注入の後に置くため、将来 budget を
  絞りたいテストがあっても override で対応可能

## Deferred Design Concerns

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Verification Notes

- [ ] 大規模 fixture 使用の4テストが `[skipped: timeout]` を
  出さず pass する
- [ ] `pnpm test`（runtime-pi）が pass する
- [ ] `mise run check` が pass する

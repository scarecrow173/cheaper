---
id: "ADR-0021"
type: "adr"
status: "accepted"
title: "grep の batchMaxBytes invariant 違反を tool 構築時に reject する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0017-bulk-reader-huge-file-safe-io.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
  derived-by:
    - "../designs/0023-grep-batch-max-bytes-invariant-enforcement.md"
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

# 21. grep の batchMaxBytes invariant 違反を tool 構築時に reject する

## Context and Problem Statement

SPEC-0018 は、sandboxed `grep` の `batchMaxBytes`（B）と 1 行の最大 decoded
payload（`3 * maxLineBytes`）の間の暗黙の invariant `B >= 3 * L` を明文化
することを要求する。この invariant が成立するとき、「1 batch ≤ B」の上限は
単一行を含めて厳密に保たれる。違反する設定では、空 batch に必ず行を積む
forward-progress 保護によって `payloadBytes > B` の単独行 batch が作られ、
上限は形式的に破られる。

現状の既定値（L=256KiB、B=1MiB）では `3 * L = 768KiB < B` で invariant は
成立する。問題は、`batchMaxBytes` / `maxLineBytes` の縮退注入 option が
test-only のままである間は invariant が暗黙に留まり、将来 production-facing
option へ格上げされた際に上限 contract を破る設定が静かに受け入れられる
ことである。

Issue #14 は対処として「invariant を option 定義・doc に明記する」または
「tool 構築時に違反を検出して reject / clamp する」のいずれかを求める。

## Decision Drivers

- 違反設定を使う既存 test が 2 件存在し、reject を選ぶと fixture 修正と
  一部 test の置き換えが必要になる。
- clamp は設定ミスを静かに補正し、注入者の意図（小さい B で batch 締めを
  観測する）を破壊する。
- 記録のみでは、option の production 格上げ時に fail-fast しない誤設定を
  防げない。
- forward-progress 保護のフォールバック経路（単独行が B を超えても送る）
  は invariant を強制しても防御的に残す価値がある。

## Considered Options

- **記録のみ**: invariant を JSDoc・設計文書に明記し、違反時の単独行 over-B
  batch を documented behavior とする。実装・test 変更は不要。
- **構築時 reject**: `createGrepTool` で `B < 3 * L` を検出し error を投げる。
  違反 test は適合 fixture への修正または reject 検証への置き換えが必要。
  over-B 単独行経路は到達不能の防御コードとなる。
- **構築時 clamp**: `batchMaxBytes = max(B, 3 * L)` に補正する。注入者の
  意図を変えてしまい、観測したい挙動が得られない誤設定を生む。

## Decision Outcome

Chosen option: **構築時 reject**。invariant を JSDoc に明記したうえで、
`createGrepTool` が `batchMaxBytes < 3 * maxLineBytes` を検出したら即座に
`Error` を投げる。理由:

- issue が懸念するのは「test-only 注入が production-facing option へ
  格上げされた際に invariant が暗黙のまま残る」ことであり、fail-fast の
  reject はこのリスクを直接解消する。
- clamp は注入者の意図を黙って変え、test が意図した batch 締め条件を
  観測できなくする。記録のみでは誤設定を実行時まで発見できない。
- 既存 test 2 件の修正コストは小さい。over-B 単独行経路は到達不能になるが、
  forward-progress 保護の防御としてコードは維持する。

## Consequences

- `createGrepTool({ batchMaxBytes, maxLineBytes })` で `B < 3 * L` なら
  構築時に `Error` が投げられる。実効値（未指定時の既定値適用後）で判定する。
- `batchMaxBytes` / `maxLineBytes` の JSDoc に invariant と `3 *` の根拠
  （invalid UTF-8 byte → U+FFFD 3 byte 展開）を明記する。
- `grep.test.ts` の違反注入 2 件を整合化する: batch 締め test は
  `maxLineBytes` も小さくして `B >= 3 * L` を満たす fixture へ修正し、
  単独行 over-B test は構築時 reject を検証する test へ置き換える。
- 単独行 over-B batch の送出経路は到達不能となるが、防御コードとして残す。
- production 既定値の組み合わせは invariant を満たすため動作変更はない。

## Confirmation

- 違反 option での `createGrepTool` 呼び出しが error を投げる test が通る。
- 適合 fixture で batch 締めの既存 test が全件通る。
- runtime-pi test suite、`pnpm typecheck`、`pnpm lint` が全件通る。

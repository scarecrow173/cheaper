---
id: "SPEC-0018"
type: "spec"
status: "approved"
title: "grep batchMaxBytes / decoded line payload invariant"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/14"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/0023-grep-batch-max-bytes-invariant-enforcement.md"
  depends-on:
    - "./0014-bulk-reader-huge-file-safe-io.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/0017-bulk-reader-huge-file-safe-io.md"
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../adr/0021-grep-batch-max-bytes-invariant-reject-at-construction.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# grep batchMaxBytes / decoded line payload invariant

## What Should Be Built

sandboxed `grep` tool の batch byte 上限 `batchMaxBytes`（B）と、1 行の
decoded payload 上限（`maxLineBytes`（L）を UTF-8 decode した最悪展開
`3 * L`）の間にある暗黙の invariant を明文化し、違反する設定が「1 batch ≤ B」
の上限を形式的に破って静かに動作しない契約を定める。

## Why Now

PR #6 の review で記録された non-blocker。`grep.ts` は空 batch には必ず
次の行を積む forward-progress 保証を持つため、`payloadBytes > B` の 1 行は
単独 batch として送られる。production の既定値（L=256KiB、B=1MiB、
decode 最悪展開で ~768KiB）では到達しないが、`batchMaxBytes` /
`maxLineBytes` を小さく注入する test では「1 batch ≤ B」を破る batch が
作られうる。test 注入 option が将来 production-facing へ格上げされた場合、
この上限 contract が暗黙のまま残る。

## Users and Value

- ツール設定の変更者: 注入 option を production option へ格上げする際に、
  破りうる上限 contract を明示された形で知ることができる。
- 利用者・運用者: 誤った上限組み合わせによる規定外 batch の送出を防ぐ。

## Scope

### In Scope

- `batchMaxBytes >= 3 * maxLineBytes` の invariant を option 定義に明記する。
- invariant に違反する option 組み合わせに対する扱い（reject / clamp /
  記録のみ）を決定し、実装または明文化する。
- 違反注入を使う既存 test の整合化。
- invariant を前提とする batch 締めロジックの挙動回帰検証。

### Out of Scope

- `maxLineBytes` / `batchMaxBytes` の既定値の変更。
- forward-progress 保証（空 batch は常に次の行を積む）自体の撤廃。
- batch 以外の grep 上限（eval 予算・deadline・出力 cap）の見直し。
- test 注入 option の production-facing option への格上げ自体。

## Requirements

1. **invariant の明文化**: `batchMaxBytes` は少なくとも 1 行の最大 decoded
   payload（`3 * maxLineBytes`）以上でなければならない、という contract を
   option 定義（JSDoc）に明記する。`3 *` の根拠（invalid UTF-8 byte が
   U+FFFD の 3 byte へ展開しうること）も記す。
2. **違反への対処**: 違反する option 組み合わせが渡された場合の扱いを
   ADR で決定する。reject を選ぶ場合は tool 構築時に分かりやすい error で
   拒否する。clamp を選ぶ場合は補正後の値を明示する。
3. **test の整合**: 違反注入に依存する既存 test を invariant 適合な
   fixture へ修正するか、reject を検証する test へ置き換える。
4. **既定値の不変**: production 既定（`GREP_MAX_LINE_BYTES` /
   `GREP_BATCH_MAX_BYTES`）は変更せず、invariant を満たし続ける
   （`3 * 256KiB = 768KiB < 1MiB`）。
5. **既存挙動の維持**: invariant 適合な設定では batch 締め・footer・skip
   報告の既存挙動を変えない。

## Acceptance Criteria

- [ ] `batchMaxBytes` option の定義に `batchMaxBytes >= 3 * maxLineBytes`
      の contract とその根拠が記載されている。
- [ ] ADR で決定した扱い（reject / clamp / 記録のみ）に従い、違反する
      option 組み合わせに対する挙動が実装または文書で定義されている。
      reject の場合、違反設定での tool 構築が error になる test がある。
- [ ] invariant 適合な fixture で batch 締めの既存 test が通り、違反注入を
      使う test は適合 fixture または reject 検証へ整合化されている。
- [ ] `pnpm typecheck`・`pnpm lint`・runtime-pi の既存 test が全件通る。

## Sources and Relations

- Issue #14 / PR #6 review の non-blocker: 本仕様の起点。
- SPEC-0014 / ADR-0017 / DESIGN-0019: streaming grep batch protocol と
  上限値の導入元。本仕様はそこで暗黙だった上限間の関係を明文化する。

違反への対処方式（reject / clamp / 記録のみ）は後続 ADR で確定する。

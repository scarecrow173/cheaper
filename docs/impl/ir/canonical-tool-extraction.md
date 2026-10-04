---
id: "IMPL-034zQtPhC1tLOPNry0QTLE"
type: "impl"
status: "completed"
title: "Canonical tool extraction"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/extract-canonical-tools.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
  experiments:
    adopted: []
    rejected: []
---

# Canonical tool 抽出

## Summary

D1 の Pi-free handler/catalog/journal と canonical input validation/accounting を実装した。

## Context

本 chain の approved spec/design と plan、docs/tasks/extract-canonical-tools.md に従う。共有 ADR D1/D7 を実装する。

## Implementation

### Changes

read/grep/find/count_tokens と bounded I/O utilities を tool-runtime に移動。Pi factories は互換 exports と thin adapter。write/edit/ls は runtime-pi の port から注入。edit access の Pi error wrapping は注入 port で元の boundary 拒否を保存する。

## Related Experiments

known-plan execution のため JSONL experiment は作成していない。metadata.experiments の adopted/rejected は空。

## Validation

新 API 不在で direct/parity tests の red を確認。最終 targeted suite は15 files / 214 tests pass。direct/Pi adapter は同じ実 workspace fixture で content/details と boundary/deny/writeScope の拒否を比較した。canonical schema の malformed input、budget/meter の一重計測、journal class identity、既存 huge-file/symlink/grep byte cap/order/partial mutation の pin を検証した。

## Risks

setup 中 deadline を跨ぐ latent Worker timeout race を regression test で確認し、coordinator 承認後の一行 guard で修正した。詳細は invocation-tool-lifetime.md に記録した。巨大単一文字の tokenizer fixture は区切りテキストに変更し byte cap を同じく超過させた。


lint/typecheck/build は pass。canonical check の再実行では pure runtime 18 tests と runtime-pi 33 files / 365 tests が pass し、CLI は並列 host contention で4件 timeout した。coordinator 方針に従い canonical check deferred to coordinator merge run。

## Follow-ups

#56/#57/#61 の service/lease/scheduling は契約どおり後続へ委ねる。docs/designs/overview.md:49 の旧 runtime-pi 帰属は hot file のため coordinator への非ブロッキング doc-only note とし、本 slice に追加作業を残さない。

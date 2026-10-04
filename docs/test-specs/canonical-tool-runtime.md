---
id: "TSPEC-034zQr5gZxyNe4nWC1uHHg"
type: "test-spec"
status: "proposed"
title: "Canonical Tool Runtime の adapter parity と所有境界"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/canonical-tool-runtime.md"
  derived-by: []
  verifies:
    - "docs/specs/canonical-tool-runtime.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## Purpose

Worker と direct caller の安全性と結果が entry path に依存しないことを保証する。
D1/D7 に代わる approved contract が成立した場合だけ supersede する。

## Feature

Pi session/model 無しの deterministic tools と、同じ runtime を使う Pi adapter。

## Rules

### Rule: canonical schema が admission と副作用の前で入力を検証する

core compileSchema は JSON-schema default を適用しない。handler の fallback で read offset/limit、grep/find maxResults を決める。
unknown と未注入 write/edit/ls は UNKNOWN_TOOL。catalog は root/handler/Pi 初期化なしで全 7 tool の schema を返す。

### Rule: 安全性・結果・accounting が両 adapter で一致する

同一実 workspace を使い text/details、truncation/count、boundary/deny/writeScope の error を比較する。
budget は sibling start より前に予約し、rejected と成功 output token を一回だけ計上する。

### Rule: journal と Worker scope の所有を保存する

旧 journal import と新 import と public exports が同じ class（instanceof）を指す。
same-file write→edit ordering と partial mutation を保持する。
root abort は全 scope、child abort は自身だけを cancel する。owned finally は facade を dispose、borrowed finally は自身 scope のみ。
abort を無視する handler があっても drain は abortGraceMs で有限に終わる。

## Examples

### Example: direct/Pi parity

realpath の同一 root に UTF-8、oversized、deny 対象と mutation target を置く。
read page/byte continuation、grep/find truncation、count_tokens details が一致する。
../ escape、deny path、writeScope 外の write/edit が同じ throw を返す。

### Example: malformed は disk を変更しない

path 型違い、limit 上限超過、edits 不正を direct に渡すと INVALID_INPUT、helper は未開始で journal は空。

### Example: sibling budget と abort

maxToolCalls=1 で二つの call を開始すると handler start は一つ、拒否は一つ。
abort 済み scope は開始しない。成功 result の token 和だけ meter に載り、boundary reject は一回。

### Example: non-cooperative call の cleanup

未 settle handler を開始して dispose すると scope signal を abort、grace 後 return、late settle は無害。
他 Worker scope と borrowed root を終了しない。duplicate owner、root/policy mismatch、parentLease 利用は reject。

## Guarantees

D1 catalog/result、validation 一回、Pi-free imports、bounded I/O（symlink/huge-file/grep batchMaxBytes）、
D7 owned/borrowed cleanup と既存 sibling/no-execution-mode pin を保存する。
既存 runtime-pi/test/tools は compatibility exports 経由でそのまま実行する。

## Non-goals

global scheduler/operation state/resource lease、Workflow/CLI、実 provider の品質検証。

## Risk

Pi helper schema drift は pinned schema equality で検出する。
countFileTokens の途中停止は core 未対応であり、開始前 abort と finite drain の範囲を保証する。

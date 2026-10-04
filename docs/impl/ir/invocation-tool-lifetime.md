---
id: "IMPL-034zQM35ydxC5Mucz5GUQJ"
type: "impl"
status: "completed"
title: "Invocation tool lifetime"
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
    - "docs/tasks/wire-tool-runtime-lifetime.md"
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

# Invocation tool lifetime

## Summary

D7 owned/borrowed scope と finite drain を実装した。

## Context

本 chain の approved spec/design と plan、docs/tasks/wire-tool-runtime-lifetime.md に従う。共有 ADR D1/D7 を実装する。

## Implementation

### Changes

invocation は root/symlink policy を固定し child scope の journal/budget/meter を保持する。runAbort を child に渡し owned finally は facade、borrowed finally は自身 scope だけ dispose。未実装 services/lease は never 型、実体/no-op はない。

## Related Experiments

known-plan execution のため JSONL experiment は作成していない。metadata.experiments の adopted/rejected は空。

## Validation

facade 不在3 tests red、runWorker の所有配線不在3 tests redを確認。facade4 tests、owned/borrowed/mismatch3 testsはpass。単独 run/grep74 tests pass。

## Risks

canonical check で setup deadline race が再現し、coordinator 承認の一行 guard で修正した。grace は finite/nonnegative を強制、開始済み countFileTokens の中断は core 未対応で bounded drain 後 late settlement を許容する。


## Scope clarification evidence

coordinator は scope clarification を承認した。canonical check の既存30ms timeout test が5000ms timeoutしたため、slow catalog80ms/policy20ms/noncooperative stream/grace30ms で setup 後の stream start1（期待0）を redで再現した。run.ts の inner try に prompt 前 abortReason 判定と既存 throwClassified 一行だけを追加した。prompt-grace.ts/model loop は変更していない。

## Follow-ups

#56/#57/#61 の service/lease/scheduling は契約どおり後続へ委ねる。docs/designs/overview.md:49 の旧 runtime-pi 帰属は hot file のため coordinator への非ブロッキング doc-only note とし、本 slice に追加作業を残さない。

## Final verification evidence

D7/facade/run/sibling/no-execution-mode targeted6files56tests pass。canonical check の途中でpure18/runtime-pi365tests pass。lint/typecheck/build pass。coordinator 新policyに従い canonical check deferred to coordinator merge run。CLI の並列20s flake3file は package 設定で単独再実行し、copilot-pre-read-hook 11 pass、hook-contract 16 pass / 1 skip、cross-agent-routing-adapters 32 pass を確認した（root5s config試行とは区別）。global doc-status は旧文書41blockingであり、coordinator承認により本chain＋共有ADRのscoped exit-auditで終端を判断する。

## Graph completion evidence

run-until-yield の plan focus により `implementation-to-followup-triage` → `followup-triage-terminal` → `exit-audit-to-complete` を確認した。impl-record audit は findings=[]、本 chain と共有 ADR の doc-status audit は blocking=0（既存 warning は残る）。global audit の41件は所有範囲外の既存 blocker であり、全体 pass は主張しない。coordinator の承認済み scoped exit-audit を適用した。

briefing gate の `considered-options` は、ルータが unordered list item のみを数えるため失敗した。coordinator の事前承認に従い、共有 ADR の選択肢1〜3の記号だけを変更し、別の docs(adr) commit とした。契約本文は変更していない。router/relations を変更して focus を迂回していない。

follow-up は分類済み。#56/#57/#61 は共有契約の後続 scope、overview.md:49 は coordinator の非ブロッキング doc-only note とし、本 slice に未完了の実装 task はない。実装と Worker 配線を論理単位で commit し、graph の complete(status=terminal)、blockers=[] を確認した。

## Independent review

requesting-code-review に従う独立 read-only review は `356838b..6f83f94` を対象に ACCEPT、confirmed P0/P1 なし。追加 test/full check/編集はしていない。journal/meta snapshot が outer finally の drain 前である点は base にも存在し、今回の差分で partial mutation 報告が壊れる concrete repro は確認できなかったため、非ブロッキングの観察として保持する。

scheduler/resource/lease の実体と、model/session setup が永続停止する場合の await 全面改修は scope 外として評価を留保した。未実装の never 型・明示拒否と、setup 完了後に新 stream を始めない承認済み guard はレビュー対象に含めた。

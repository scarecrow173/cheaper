---
id: "DESIGN-034Au1ncZwENCfsEQyf0cD"
type: "design"
status: "approved"
title: "managed process lifecycle hardening"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/60"]
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
    - "../specs/managed-process-lifecycle-hardening.md"
    - "../adr/managed-process-bounded-lifecycle-policy.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../adr/managed-process-named-task-lifecycle.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Managed process lifecycle hardening

## Overview

shared ADR D1/D2/D3/D5/D7 と #59 の canonical executor を基盤とする。core は policy/default/merge、tool-runtime は process lifecycle、adapter は canonical result を保持する。
固定 cap と即時 force-kill は新 ADR で refine。Artifact Store handoff は coordinator verdict により defer。

## Scope and Non-goals

top-level process.timeout / maxOutputBytes / killGrace / heartbeat.interval、normal/failure/timeout/abort/dispose cleanup を対象とする。
別 manager/pool、LLM wake/context heartbeat、poll/status API、任意 shell、依存追加、native containment と streaming writer は追加しない。

## Policy Representation and Validation

既存 ProcessPolicy に optional lifecycle raw fields を追加し、resolveProcessPolicy が default を満たした ResolvedProcessPolicy を返す。raw duration は文字列のまま policy/schema/load/merge を通し、executor が shared parseDuration で ms に変換する。tasks の構造は変えない。
partial omission は維持、trusted merge 後/default 補完/project restriction 前に同じ resolver を使う。project は timeout/killGrace/heartbeat.interval の ms 比較と maxOutputBytes/maxConcurrent の数値 min を使い元表現を選ぶ。
duration は positive かつ 2147483647ms 以下、maxOutputBytes は 1..16777216 bytes。ceiling を core constant で一元定義し canonical static output schema に利用する。top-level field を各 task に置く input は additionalProperties:false で拒否。

| Field | Default | Project restriction |
| --- | --- | --- |
| timeout | 2m | 最大 root 実行時間を min |
| maxOutputBytes | 65536 | 合計 retained bytes を min |
| killGrace | 2s | 最大 graceful 待ち時間を min |
| heartbeat.interval | 30s | 最大観測間隔を min、無効化不可 |

policy.yaml ×2 はこの defaults を追加する hot-file gate 対象。root manifests/scripts/overview/shared ADR は変更しない。

## Executor State and First Cause

prepare と scheduler admission 完了後に native child を spawn する。spawn 時点で timeout と heartbeat を開始し、queue wait は timer に含めない。
timeout は CheaperError(TIMEOUT) を生成し、registry-owned onTimeout seam が現在の AsyncLocalStorage operation ID に manager.cancel を呼ぶ。local AbortController と operation.signal の合成 signal が tree termination を開始する。operation の first cancellation reason と timed_out をそのまま使い、OperationManager の他 tool failure 分類を変更しない。
onTimeout seam は operation が既に terminal なら何もしない。
外側 signal が先なら timeout callback は何もせず original reason を保つ。cancel cleanup failure は diagnostics に残し original reason を置換しない。
root exit で execution timeout は止める。heartbeat は handler settle まで診断可能。正常/nonzero exit でも tree cleanup を開始し child close と cleanup 完了を待つ。
spawn failure（PID 無し）は pipe/listener/timer cleanup を行い TOOL_EXECUTION_FAILURE にする。

## Bounded Capture and Diagnostics

stdout/stderr 共通 remaining=maxOutputBytes。chunks は残り bytes だけ copy し cap 後も drain。UTF-8 replacement/tail は既存 capture.finish と同じ byte-boundary 処理。details の bytes/capturedBytes/truncated を維持する。
canonical stream schema の maxLength/capturedBytes ceiling は shared constant から取得する。display の64文字折返しは維持する。
heartbeat は Node runtime の process.stderr（cheaper 自身の診断 channel）に task/PID/elapsedMs/output-byte counters/phase の bounded JSON 行を出す。child stderr capture と別経路で、canonical result/details と LLM hook に渡さない。stderr が writableNeedDrain のときは診断を drop する。観測例外が child 結果を変えない。

## Small Platform Backend

新 module が tree の graceful terminate、残存確認、force terminate を所有する。spawn authorization/cwd/env/capture は executor に保つ。

### POSIX

既存 detached group を維持。SIGTERM→最大 killGrace→残存 group だけ SIGKILL。ESRCH は終了済み。normal root exit でも group を確認し残存 descendants を終了する。
group を離れる setsid 等は OS containment の対象外。Linux の zombie を実行中 process と扱わない test helper を使う。

### Windows

SystemRoot の native taskkill.exe を shell:false/windowsHide で呼ぶ。graceful は /PID /T、escalation は /PID /T /F。console app の graceful は best effort。
SystemRoot の WindowsPowerShell に固定 OS query を実行させ、Get-CimInstance Win32_Process の PID/ParentProcessId/CreationDate のみを bounded JSON として取得する。user input を shell expression に挿入しない。
root PID の下の live hierarchy と既知 descendant identities を集め、creation time で stale/reused PID を除外する。root が既に exit しても parent ID が root を指す live child から辿り、残存 branch を taskkill target にする。
snapshot/helper は timeout と output cap を持ち、helper 自身の stream/timer/listener も finally で回収する。query failure は cleanup failure として明示し、tree cleanup の成功を偽装しない。
**Declared platform limitation W1:** 初回 identity snapshot 前に中間親が終了して ancestry を失った Windows 子孫は、この ancestry-based cleanup の保証範囲外とする。adversarial containment escape とは別の platform limitation である。通常 root exit（child/grandchild の ancestry が追える場合）と stubborn tree は実 test で保証する。coordinator verdict msg_6cc7f6088615 により native Job Objects は必須とせず将来 ADR に defer する。

## Settle, Capacity, and Disposal

cleanup は一つの idempotent promise とし abort/timeout/root exit が重なっても二重 kill sequence を開始しない。grace timer は tree が消えれば停止し force-kill しない。
finally で process timeout/heartbeat/grace timers と signal/data/event listeners、stdout/stderr handles を回収する。force failure では root を kill し pipe を destroy して hang を避け diagnostics を残す。
registry は process invocation promise のみを owner scope 内で追跡する。drain は既存 manager.drain を実行した後、managed process promises の actual settle を await する。generic tool の bounded detach behavior は変更せず、managed process は bounded killGrace+backend helper deadline で終える。
scheduler/resource manager の capacity/lease release の finally を唯一の解放点として保ち、新 pool/semaphore を作らない。borrowed scope.dispose は当該 owner だけを abort/await し他 job は維持する。

## Artifact Seam

今回は full output を追加 buffer して store.put に渡さない。bounded result は store 無しで動く。
将来の統合には tool-runtime が依存する streaming writer port が必要であり、workflow store implementation を composition root から注入する。新 public writer 型/API は今回追加しない。

## Verification Matrix

| Behavior | Lowest meaningful verification |
| --- | --- |
| defaults / raw validation / trusted+project merge | core policy.process tests |
| configurable cap >64KiB / UTF-8 / canonical output schema | tool-runtime process tests |
| queue wait exclusion / timeout timed_out / outer first-cause | invocation shared pool + two owners |
| graceful exit / stubborn escalation | native fixture + POSIX-only signal test（Windows skip 明示） |
| abort / normal exit / nonzero exit cleanup | real parent/child/grandchild PID fixture |
| scope dispose / invocation dispose / outer signal | shared invocation owner tests、他 owner の非 cancel |
| capacity held through grace+cleanup | queued second process の marker が settle 前に作られないこと |
| heartbeat isolation and timer cleanup | runtime stderr spy と real child result/metadata/operation-count 比較 |
| adapter sharing | runtime-pi process adapter focused test |

新 tests は RED を確認してから実装する。Windows は本 checkout で native 実 tree、Linux は対応 platform の test を備え、この checkout で未実行なら明記する。
focused package suites + pnpm lint/typecheck/build を pass、CLI policy asset 関連 test を単独実行。canonical check deferred to coordinator merge run。

## Risks and Rollback

Windows query の latency と宣言済み ancestry-loss limitation を記録する。helper output/clock/timer overflow は boundary validation と bounded helper で防ぐ。
変更は lifecycle/default と executor/registry drain の logical commits に分け、既存 named task 認可/pool/resource routing を維持する。rollback は各 commit の通常 revert（破壊的操作は行わない）。

## Approval and Graph Trace

run-until-yield。準備文書 commit: 786e3f5。probe-to-briefing → briefing-to-design（delegate=design-doc、requiredAudit=design）。
ADR intent は coordinator direct verdict（closed msg_678e8b5e7687 の回答）で確認済み。
正式 coordinator verdict msg_6cc7f6088615（thread msg_47732c6b1e9b）で spec approval / ADR acceptance / design approval、policy.yaml ×2 defaults、capture ceiling 16MiB、heartbeat.interval project-min と W1 limitation が APPROVED。required mechanical edits と P3 明確化を反映して status を更新した。

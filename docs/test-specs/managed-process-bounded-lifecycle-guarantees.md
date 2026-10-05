---
id: "TSPEC-034AGg47u5fWQqP0pEJfI9"
type: "test-spec"
status: "proposed"
title: "Managed process bounded lifecycle guarantees"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
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
    - "../designs/managed-process-lifecycle-hardening.md"
  derived-by: []
  verifies:
    - "../specs/managed-process-lifecycle-hardening.md"
    - "../adr/managed-process-bounded-lifecycle-policy.md"
  verified-by: []
  references: ["./managed-process-lifecycle.md", "./managed-process-task-authorization.md"]
  defers: []
  deferred-by: []
---

# Managed process bounded lifecycle guarantees

## Purpose

runtime が process lifetime、memory と cancellation の責任を持ち、Worker/Workflow の polling や LLM wake を必要としないことを保つ。#59 の既定64KiB・認可・adapter共有を維持し、configurable cap、timeout、通常終了後の cleanup を追加する。process tool 廃止または正式な後継契約への移行時にのみ退役可能。

## Feature

Feature: policy-defined process が bounded output と execution deadline を持ち、管理下 tree の cleanup が実 settle するまで invocation に所有される。

## Rules

- Rule: trusted replace → default completion → project-min。lifecycle fields は top-level process.* に固定し、project/per-task input が上限を緩和できない。duration は正で Node timer ceiling 以下、combined output cap は1..16MiB。
- Rule: timeout は native spawn 起点。queue/prepare/admission 待ちは含まない。外側 abort は queue 中も有効で、最初の reason/state を cleanup failure や後続 timer で上書きしない。
- Rule: normal/nonzero/spawn failure/timeout/abort/dispose の全経路で pipes/timers/listeners を回収し、追跡可能な parent/child/grandchild を終了する。POSIX は group、Windows は identity/ancestry を用い、grace 内の tree exit 後には force-kill しない。
- Rule: retained stdout/stderr と decoded UTF-8 の合計 bytes は policy cap 以下。超過後も drain して bytes/capturedBytes/truncated を保持する。
- Rule: heartbeat は Node runtime diagnostic channel のみ。child stderr、canonical result、Worker context と LLM turn を増やさず、terminal 後に timer が残らない。backpressure 時は drop する。
- Rule: scheduler/resource capacity は actual handler settle まで保持する。owned scope dispose はその owner を止め、他 owner を維持する。managed dispose は generic abortGraceMs を超え得るが bounded cleanup を待つ。

## Examples

- Example: legacy omission は timeout=2m、cap=65536、killGrace=2s、heartbeat.interval=30s。trusted timeout=3m/cap=1MiB に対し project timeout=5m/cap=2MiB は trusted 値を維持し、project timeout=1m/cap=1024 は短縮する。
- Example: project の heartbeat.interval=1m は trusted/default の最大観測間隔30sを緩和しない。interval=10s は短縮でき、false/0/unknown fields は拒否する。
- Example: process pool の二番目 task が timeout より長く queue にいても admission 後に完全な execution deadline を得る。queue 中の outer abort は元 reason で settle する。
- Example: stubborn parent/child/grandchild は timeout/abort 後、grace を経て force 終了する。POSIX の graceful fixture は終了 marker を保存して force-kill なしで exit する。Windows console graceful 非対応を明示する。
- Example: root が0または非0で先に exit し、子孫が stdout/stderr を継承して生存していても、tree 回収と pipe close が終わってから handler が settle する。
- Example: cap=1024 と cap>64KiB の mixed/binary output が configured bytes に収まり canonical output schema を満たす。非0 exit も bounded diagnostics を返す。
- Example: cleanup failure と outer abort が重なっても original reason/cancelled を維持する。process timeout は TIMEOUT/timed_out。
- Example: abortGraceMs より長い managed cleanup 中は dispose と queued pool task が完了せず、actual settle 後にのみ capacity を解放する。他 owner の実 tree は scope-local dispose で終了しない。
- Example: runtime heartbeat を観測しても captured child stderr/result/operation数/budget/LLM turn は heartbeat で変わらず、settle 後に新しい heartbeat が出ない。

## Guarantees

- first-cause、execution deadline、bounded memory、real settlement と owner isolation を一つの canonical executor で維持する。
- 実 tree の PID 生存状態を platform-aware に検証する。Linux zombie は稼働 process と数えない。未実行 platform は skip/未検証を明記する。
- Pi adapter の process result と generic tool-runtime lifecycle contract に regression を入れない。

## Non-goals

- Windows で初回 identity snapshot 前に中間親が終了して ancestry を失った子孫は宣言済み platform limitation。adversarial containment escape とは別に記録する。
- POSIX group escape、explicit re-parent、新 native Job Objects、streaming Artifact Store writer、OS sandbox、real provider/model、poll/status API、LLM wake は対象外。

## Risk

- orphan、終了しない pipe、無制限 output、false terminal state、cleanup 前の capacity 解放、別 owner の誤 cancel、Worker context の heartbeat 汚染が起きる。テストは実装手順ではなくこの契約を保護する。

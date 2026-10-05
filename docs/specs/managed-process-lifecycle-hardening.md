---
id: "SPEC-034Aq1jA258sf4IaRxgCrq"
type: "spec"
status: "proposed"
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
  derives-from: ["../discovery/managed-process-lifecycle-hardening.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/managed-process-named-task-lifecycle.md"]
  defers: []
  deferred-by: []
---

# managed process lifecycle hardening

## What Should Be Built

policy-defined process の実行期限、合計 output byte cap、段階的な tree 終了、runtime-only heartbeat を canonical executor が所有する。

## Why Now

#59 の executor は abort の即時 force-kill と固定 cap のみを持つ。stubborn task・通常 root exit・dispose を含む lifetime を bounded にする必要がある。

## Users and Value

Worker、Workflow、親 Coding Agent が process の終了を poll せず、共通 policy と terminal result/reason を利用する。

## Scope

### In Scope

per-process timeout、configurable combined stdout/stderr cap、killGrace、heartbeat.interval、外側 abort propagation、managed tree cleanup、既存 pool の保持と解放、platform の実 tree tests。

### Out of Scope

unrestricted shell、status(jobId)/ps/log polling/model sleep、LLM wake、invocation を超える process、OS containment を逃れた process、新 pool、新依存。
oversized full output の artifact handoff は coordinator の ADR intent verdict により別統合依存（#66）へ defer する。

## Requirements

- process timeout は spawn から計測し、queue 待ちは含めない。外側 run timeout/abort が先ならそれを優先する。
- timeout は TIMEOUT と timed_out、abort は original reason と cancelled。first-cause を後続 cleanup error や timer で上書きしない。
- terminate を要求し bounded killGrace 後に生存 managed tree を force-kill する。全 terminal 経路で child pipes/timers/listeners を回収する。
- stdout/stderr の合計 retained bytes は maxOutputBytes 以下。cap 後も drain し bytes/capturedBytes/truncated を維持する。
- lifecycle field は top-level process.* に置き、per-task override を提供しない。
- heartbeat は runtime diagnostic channel（cheaper 自身の stderr/debug）のみ。child の captured stderr、tool result、Worker context、LLM turn に heartbeat を追加しない。
- scope dispose・invocation abort・Worker abort で tree 終了を要求する。pool/resource capacity は実 settle まで保持する。
- POSIX と Windows の違いを明示し universal graceful termination を主張しない。外部 session/re-parent による containment escape は対象外。
- shared ADR D1 の依存方向、D2 の trusted/restrict 合成、D3 の terminal/settlement、D5 の named argv、D7 の owned/borrowed lifetime を保つ。

## Acceptance Criteria

- [ ] AC1: process timeout で TIMEOUT/timed_out、外側 abort が先なら original reason/cancelled。
- [ ] AC2: configured cap が combined output と UTF-8 text に適用され truncation diagnostics を返す。
- [ ] AC3: timeout/abort の parent/child/grandchild が終了し stubborn tree は grace 後に force-kill。
- [ ] AC4: grace 内に管理 tree が exit した場合 force-kill しない（Windows graceful 非対応ケースは明示）。
- [ ] AC5: normal/nonzero/spawn failure/timeout/abort/dispose の cleanup が settle し capacity を解放する。root が先に exit した残存 descendant も対象。
- [ ] AC6: heartbeat が観測でき result/context/turn を汚染しない。timer が terminal 後に残らない。
- [ ] AC7: policy defaults、legacy omission、trusted overrides、project non-relaxation、invalid duration/cap を検証する。
- [ ] AC8: Linux/Windows の実 tree tests を備え、実行しなかった platform は skip/未検証を明記する。
- [ ] AC9: process pool の queue wait が process.timeout を超えても、admission 後の child 実行期限を消費しない。外側 run abort は queue 待ち中にも有効。

## Sources

GitHub issue #60 本文と Triage 2026-10-04、dispatch brief、および discovery に記した現行 source。

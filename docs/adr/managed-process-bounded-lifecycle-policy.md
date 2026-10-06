---
id: "ADR-034AtSJVcROd9oyMi4ksua"
type: "adr"
status: "accepted"
title: "managed process bounded lifecycle policy"
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
  related: ["../specs/managed-process-lifecycle-hardening.md"]
  refines: ["./managed-process-named-task-lifecycle.md"]
  refined-by: []
  derives-from: ["../discovery/managed-process-lifecycle-hardening.md"]
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/managed-process-bounded-lifecycle-guarantees.md"]
  references: ["./managed-process-named-task-lifecycle.md", "./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Managed process bounded lifecycle policy

## Context and Problem Statement

#59 の accepted ADR `managed-process-named-task-lifecycle.md` は固定 65536-byte capture、POSIX SIGKILL / Windows taskkill /T /F の即時終了、正常終了後の孫残留検査を保証しないと定めた。
#60 はこの三つを明示的に refine し、configurable cap、graceful→force escalation、通常 root exit 後を含む managed tree cleanup に変更する。named-task 認可、shell:false、minimal env、argv、I/O 宣言、pool と adapter の契約は保つ。
共有 ADR の D1/D2/D3/D5/D7 を維持する。

## Decision Drivers

- lifecycle は executor が所有し Worker/Workflow が poll しない。
- first-cause cancellation reason と OperationManager terminal state を一致させる。
- memory、timer、pipe と process pool は bounded、容量は実 settle まで保持する。
- Windows と POSIX の違いを backend に隔離し universal graceful termination を主張しない。

## Considered Options

- Option 1: Worker/Workflow caller に timeout、kill、poll を分散する。
- Option 2: canonical executor と小さい platform backend に統一し既存 OperationManager/pool を再利用する。
- Option 3: native OS containment と streaming Artifact Store writer を同時に追加する。

## Decision Outcome

Chosen option: **2**。依存を追加せず、policy/config は core、spawn/capture/heartbeat/tree cleanup は tool-runtime、Pi/Workflow は共通 executor を呼ぶ。

### Top-level process policy と既定

```yaml
process:
  maxConcurrent: 1
  timeout: 2m
  maxOutputBytes: 65536
  killGrace: 2s
  heartbeat:
    interval: 30s
  tasks: {}
```

全 lifecycle field は `process.*` に固定し、per-task override を schema で拒否する。legacy omission は既定を補完する。
timeout / killGrace / heartbeat.interval は正の duration、Node timer の最大値 2147483647ms 以下。maxOutputBytes は正の整数で上限 16777216 bytes（canonical output schema と capture の共有 ceiling）。
timeout は spawn から child root exit までの期限で、scheduler queue wait は含まない。外側 run abort/timeout は queue・実行の両方に作用し、先に発生した reason を保持する。

### Layering と heartbeat.interval restriction

trusted builtin→user は scalar replace、その後 default 補完、project は `maxConcurrent`、`timeout`、`maxOutputBytes`、`killGrace`、`heartbeat.interval` を **min** 合成する。
duration は ms 比較し選択した元の文字列表現を維持する。partial omission は inherit。
heartbeat.interval は「最大観測間隔」と定義する。project は interval を増やして観測を弱めたり無効化したりできない。短縮による高頻度診断は runtime diagnostic channel の backpressure 時に drop して unbounded buffering を防ぐ。
既存 D2 の restrict-concurrency mode を拡張するが、他の policy key の merge semantics は変えない。

### Terminal state と cleanup

process timeout は TIMEOUT reason で現在の scoped operation を cancel し、既存 OperationManager の timed_out 分類と first-cause を使う。manager の全 tool failure 分類は変更しない。
process を terminate し killGrace 内の exit を待つ。残存 tree を force-kill し pipes/listeners/timers を回収してから settle。通常/nonzero root exit でも残存 tree を回収する。
scope drain/dispose は managed process の cleanup promise を await し、generic OperationManager の bounded detach は他 tool に維持する。scheduler の pool/resource release は実 handler settle の finally だけが所有する。

### Platform behavior

POSIX は detached process group に SIGTERM、grace 後に残存 group へ SIGKILL。group が既に消えていれば force-kill を実行しない。
Windows は native taskkill /PID /T（best effort）→grace→/T /F。root exit 後にも tree を回収するため、OS の PID/parent/creation-time snapshot を root PID と既知 descendant identity で辿り、消えた root では残存 child を kill target にする。
Windows graceful は console app に対して保証せず、許可された native executable の tree containment escape（新 session / explicit re-parent 等）は対象外。
初回 identity snapshot 前に中間親が終了して ancestry を失った Windows 子孫は、ancestry-based cleanup の保証範囲外とする。これは adversarial containment escape とは別の宣言済み platform limitation であり、通常 root exit 時に ancestry を追える残存 child/grandchild は回収する。native Job Objects は今回必須とせず、より強い保証は将来 ADR に defer する（coordinator verdict msg_6cc7f6088615）。

### Heartbeat と output

heartbeat は cheaper 自身の runtime diagnostic channel（stderr/debug）だけ。child captured stderr、canonical result/details、Worker context、LLM turn には含めない。
capture は stdout/stderr 合計 maxOutputBytes、cap 後も drain、UTF-8 text も byte cap 内、bytes/capturedBytes/truncated を保持する。
full oversized output の Artifact Store handoff は **DEFERRED**。RunArtifactStore.put は完成 buffer をコピーする API のため、full output をそのために buffer せず streaming port の別統合へ委ねる。tool-runtime が workflow を import しない。

## Consequences

- Good: policy と lifecycle の正本を共有し、queue wait / first-cause / capacity の既存契約を保てる。
- Cost: Windows snapshot は追加 helper と cleanup latency を伴い、strict containment は別境界になる。
- Follow-up: streaming artifact writer の別統合。no store でも bounded log を維持する。
- Revisit: Windows ancestry-loss を含む保証が必須なら native containment の別 ADR を作る。

## Pros and Cons

| Option | Pros | Cons |
| --- | --- | --- |
| Caller-owned lifecycle | executor 差分が小さい | ownership と cancellation rule が重複し polling を招く |
| Canonical executor + backend | 既存 manager/pool を共有し adapter drift を防ぐ | platform-specific cleanup を実 tree で検証する必要 |
| Native containment + streaming store | 更に強い lifetime/output 契約が可能 | 今回の依存・package seam・配布境界を超える |

## Implementation Plan

- `packages/core/src/policy/process.ts`, `merge.ts`, `load.ts`, `schema.ts`: duration/cap/default/restrict を一元化し raw field は top-level に限定する。
- `packages/tool-runtime/src/process.ts`, 新 backend module、`registry.ts`: process-local timers、bounded capture、scoped timeout cancel seam、actual cleanup drain。
- `packages/cli/policy.yaml`, `packages/apm/.apm/hooks/cli/policy.yaml`: design gate で承認された既定のみ追加する。
- `packages/core/test/policy/process.test.ts`, `packages/tool-runtime/test/process*.test.ts`, child fixture、Pi adapter test: policy layering、queue exclusion、state/reason、real tree、heartbeat isolation を検証。
- `docs/guides/managed-process.md`: platform 差異、no-polling、diagnostic-only heartbeat、artifact defer を記録。
- 依存/manifests/root scripts、pool 機構、shared ADR、overview、bundled CLI は変更しない。

## Verification

- [ ] policy default/override/project restriction/invalid duration/cap/per-task rejection が focused tests で確認できる。
- [ ] queue wait は process timeout を消費せず、outer abort は queued cancellation に伝播する。
- [ ] timeout が TIMEOUT/timed_out、abort が original reason/cancelled となる。
- [ ] success/nonzero/spawn failure/timeout/abort/dispose で managed tree と pipes/timers が終了し pool は実 settle まで保持される。
- [ ] grace 内 exit と stubborn escalation の platform-aware 実 tree tests を実行し非実行 platform を明示する。
- [ ] heartbeat は runtime diagnostics だけで child output/result/context/turn に含まれない。
- [ ] focused package suite、lint/typecheck/build、scoped doc audit を確認。canonical check は coordinator merge run に defer。

## More Information

coordinator direct verdict（2026-10-05）は ADR intent と defaults、top-level fields、artifact defer を確認した。正式 verdict msg_6cc7f6088615 は spec+ADR+design gate、policy.yaml ×2 defaults、16MiB ceiling、heartbeat.interval project-min と W1 limitation を承認した。

## Confirmation

coordinator の正式 ADR acceptance と design gate の結果、ならびに Verification の実行 evidence を implementation record に記録する。

---
id: "DISC-034Aq1bAPGiJ8GsKHAeKAL"
type: "discovery"
status: "resolved"
title: "managed process lifecycle hardening"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/60"
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
  derives-from: ["../ideas/managed-process-lifecycle-hardening.md"]
  derived-by: ["../specs/managed-process-lifecycle-hardening.md", "../adr/managed-process-bounded-lifecycle-policy.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/managed-process-named-task-lifecycle.md"]
  defers: []
  deferred-by: []
---

# managed process lifecycle hardening

## Exploration Goal

#59 の named process を timeout・abort・通常 exit・dispose の全経路で settle させ、Worker/Workflow に polling を要求しない lifecycle を調べる。Entry Decision: A-2（platform cleanup と artifact handoff の trade-off を確認する）。
dispatch は明示した doc skill chain を指定しているため briefing-flow の profile discovery/子 dispatch は使用せず、指定 delegate を inline で実行する。

## Constraints

共有 ADR の D1/D2/D3/D5/D7 を保つ。pool と OperationManager を再利用し、依存追加・shell tool・LLM heartbeat・persistent background process は対象外。

## Evidence

- `packages/tool-runtime/src/process.ts`: stdout/stderr 合計 65536 bytes、abort 時だけ killTree、POSIX SIGKILL / Windows taskkill /T /F。通常 root exit 後の descendants cleanup がない。
- `packages/tool-runtime/src/operation.ts`: first-cause cancel に TIMEOUT があれば timed_out。handler throw だけなら failed。
- `packages/core/src/policy/process.ts`, `load.ts`, `merge.ts`: trusted replacement → default 補完 → project restriction。新 limit はここで一元化できる。
- `packages/workflow/src/artifacts.ts`: RunArtifactStore.put は完成済み string/Uint8Array をコピーして保存する。streaming writer は無い。tool-runtime → workflow の import は D1 が禁止する。

## Alternatives

| Option | Pros | Cons |
| --- | --- | --- |
| caller が timeout/poll/kill を所有 | executor の差分が小さい | ownership が分散し no-polling 契約を破る |
| canonical executor + 小さい platform backend | adapter/pool を共有し first-cause を保てる | Windows graceful は best effort、root exit と tree cleanup の race を扱う必要 |
| OS containment / streaming store を同時導入 | escaped descendant / full output 保存を拡張可能 | Windows Job Object/native launcher と store writer 契約が #60 の依存・境界を広げる |

## Tentative Conclusions

executor 所有の timer/capture/heartbeat と platform backend が妥当と推測する。POSIX SIGTERM→SIGKILL、Windows taskkill /T→/T /F とし universal graceful を主張しない。
artifact の full output handoff は現行 put に繋ぐと unbounded buffering を招くため、別統合依存へ defer する案を design gate で確認する。

## Open Questions

- lifecycle defaults、project の timeout/output/killGrace 制限の追加、policy.yaml ×2 変更を coordinator に確認する。
- TIMEOUT と OperationManager timed_out の整合は manager の outcome classification か scoped cancel seam のどちらで所有するか。
- Windows root が先に exit する場合の descendant cleanup を既存 taskkill backend の範囲で検証し、必要な境界を design で決める。
- OperationManager.drain は abortGraceMs（既定 5000ms）を超える未 settle operation を detach する。process.killGrace と platform helper deadline の合計がこれを超えた場合の dispose/late-settle 境界も design gate で確認する。pool capacity は scheduler が実 settle まで保持する既存契約を維持する。

## Promotion Candidates

spec と ADR と design に lifecycle/default/platform 契約を記録し coordinator の承認を受ける。

## Coordinator Intent Confirmation — 2026-10-05

元質問 msg_678e8b5e7687 は dispatch settle により閉じたが、coordinator が direct message で ADR intent を承認した。
defaults は timeout=2m、maxOutputBytes=65536、killGrace=2s、heartbeat.interval=30s。field は top-level process.* に固定する。
heartbeat は runtime diagnostic channel のみで child の captured stderr に含めない。artifact handoff は buffered-only store のため別統合へ defer。
#59 の accepted ADR の固定 cap と SIGKILL-only decision を明示的に refine する。heartbeat.interval の restriction と queue wait 除外を ADR/design に記録する。

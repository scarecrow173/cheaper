---
id: "ADR-034A8FtvtZFYBJpZ8MNu4N"
type: "adr"
status: "accepted"
title: "親 job lease と子 operation conflict を ResourceManager で管理する"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/61"]
  derives-from: ["../discovery/resource-manager-job-leases.md"]
  derived-by: ["../designs/resource-manager-job-leases.md"]
  related: ["../specs/resource-manager-job-leases.md"]
  references: ["./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
metadata:
  adr:
    decision-makers: ["coordinator"]
    consulted: []
    informed: []
---
# 親 job lease と子 operation conflict を ResourceManager で管理する

## Context and Problem Statement
per-call lock では Worker A の read と edit の間に Worker B が書ける。共有 ADR D4 は core canonical claims と job lifetime borrow を規定、D7 は invocation の単一 service を規定する。

## Decision Drivers
stale-read 防止、独立 resource overlap、all-or-none 取得、first-cause abort、late-settle の早期解放防止、既存 writeScope/MutationJournal の互換性。

## Considered Options
- per-call lock のみ: stale-read を防げず却下。
- invocation global lock: 安全だが独立 resource を並行化できず却下。
- job lease と child lease を分離: 親保護と子 conflict を両立し採用候補。

## Decision Outcome
intent は coordinator が msg_548a23d615bb で APPROVED: adr-intent。spec-design は別ゲートで承認する。
spec-design は coordinator msg_84fd50c7a75a で承認済み。
tool-runtime の ResourceManager が job token と operation token を管理し core の型と正規化/predicate を再利用。job.ready を session 前に await、子 operation は parentLease の claims を borrow。親 lease と同じ token を再取得せず、子 token 同士は通常どおり conflict 判定。包含されない write は fail closed。
ready は Worker session を開始する呼出し元が明示的に await する。通常の direct tool execute は per-operation lease のみ（createWorkerScope は adapter の名前であり direct tool job にも利用される）。writeScope/manifest の authorization は ready の有無によらず実施する。
job は reads/writes を全寿命で all-or-none 取得する。readStrictness=record の mutating job は undeclared dynamic read を許すため safety reads を root shared に拡大し、narrow 並行性を優先する job は reject と明示 reads を用いる。diagnostic declaration と safety claim は同じ canonical claim 型で保持する。
foreign job の root shared と exclusive も通常の conflict 判定に含めるため record-only mutation jobs は並行 admission されない。borrow read/write はいずれも全包含必須、部分借用しない。foreign job-lifetime token に conflict する child は fail-fast、sibling operation のみ待機可能。
root shared の blast radius は invocation 全 workspace の全 foreign exclusive claim。record-only job ではこの並行性制限を選び dynamic read を許す。親に blocking される conflicting waiter（job acquisition 含む）は有効 borrow child の進行を妨げない。lease 解放後は既待機 conflict を優先する。
foreign-token fail-fast は parent/binding contract violation の例外に限定する。通常の independent job/operation の resource conflict は abortable queue で待機する。
readStrictness は tool-runtime ResourceScopeOptions の単一宣言から確定する。parent token の値を子が継承、不一致 override は拒否。writeScope と mkdir ancestor rule は tool-runtime scope preparation helper で一意化し Pi boundary と observer が共有する。
capacity admission と全 claims 取得が同時に可能な場合だけ operation を start。claim wait に capacity を与えない。ResourceManager queue は all-or-none、claims は安定 key 順、待機 job は同期 submission sequence の FIFO で tie-break。async normalization の settle 順で並べ替えず、既 start job と semantic DAG edge を変更しない。
release は冪等で、親 release 要求後も active child が settle するまで親を保護。root/owner abort は queued wait を original reason で拒否、running token は actual settle まで保持。

## Observed Access Policy
実 filesystem boundary に observer port を追加し owner/operation/claim と allowed/declared を記録する。write は writeScope、明示 job manifest、operation claim を満たさなければ I/O 前拒否。read の job manifest 逸脱は default record-only、opt-in reject。operation 逸脱は scheduling safety を満たさないため read も拒否。deny policy は observer より先に適用し privacy を維持。
atomic tmp と認可済み target の parent mkdir は既存実装詳細として扱い target の exclusive scope で保護する。異なる target を共有 parent へ作成する場合の recursive mkdir は既存仕様を維持し directory ownership を推測しない。
process は宣言 seam の lease のみ検証。外部 syscall と observer を使わない injected handler は観測を主張しない。

## Consequences
- 独立 file job の overlap と同一 scope の session 保護を両立する。
- 未協調 handler の token は late settle まで残り他 job を待たせる。bounded dispose を安全な停止と誤認しない。
- scoped read が親 exclusive lease と partial overlap の場合は明示 wider declaration が必要。未知 read は root shared、未知 write は root exclusive。
- serialized policy は global operation exclusion を維持、per-resource は internally validated resource metadata/writeScope/manifest がある時のみ。
- cross-process/OS tracer/Workflow engine は対象外。

## Implementation Plan
`packages/tool-runtime/src/resources.ts` に lease 管理、`registry.ts` に ready/manifest/observed access、`scheduler.ts` に atomic admission、`invocation.ts` に parent token 検証と acquireJobLease facade、`operation.ts` に queued resourceClaims の記録 seam、`index.ts` に exports。
read/count-tokens/walker の I/O 前 observer と runtime-pi tools/operations、tools/registry の observer wiring、run.ts の session 前 ready。依存追加、core 型の再定義、hot file 変更、CLI/workflow compiler 変更は行わない。
既存 serialized fast admission と budget reservation を維持。test doubles は宣言 seam と deferred promise を使い timing に依存しない。

## Verification
- [ ] tool-runtime focused suite で独立/競合/borrow/abort/alias/late settle を確認。
- [ ] runtime-pi focused suite で writeScope lifetime と actual I/O/journal/deny 回帰を確認。
- [ ] pnpm lint/typecheck/build が exit 0。
- [ ] scoped exit-audit に blocking finding がない。

## Confirmation
spec-design と plan と実装 review を coordinator に ask。canonical check deferred to coordinator merge run。hard-link alias と外部 symlink 差替えは現行 workspace boundary の保証範囲外。

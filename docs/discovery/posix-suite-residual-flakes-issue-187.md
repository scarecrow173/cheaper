---
id: "DISC-034MwehOB3CPhXhCebg1G3"
type: "discovery"
status: "resolved"
title: "issue #187 POSIX suite 残存 flake の原因分析"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/187"
    - "https://github.com/scarecrow173/cheaper/issues/187#issuecomment-6394277590"
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
  derives-from: []
  derived-by:
    - "../specs/stabilize-posix-suite-residual-flakes-issue-187.md"
    - "../adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md"
    - "../designs/issue-187-posix-suite-residual-flake-stabilization.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  defers: []
  deferred-by: []
---

# issue #187 POSIX suite 残存 flake の原因分析

## Exploration Goal

briefing の entry decision は **A-3 Combined Discovery**。#187 の複数の症状について、
issue 本文、follow-up comment、承認済み #157/#180 documents、diagnostic probe を照合する。
各症状が test-owned timing / fixture boundary、runtime lifecycle invariant の違反、または
別の原因によるものかを、修正 owner を選ぶ前に個別に特定する。報告は共通 root cause
を立証していない。明示された task dispatch により probe と並行して draft を作成したが、
briefing や因果分析の完了を意味しない。

## Key Issues and Assumptions

### 報告された事実

- `resources::tests::abort_landing_before_listener_registration_still_rejects`
  stalled in Ubuntu workflow run `40149087832`; an orphan `cheaper_tools` test
  binary remained without a result for over 66 minutes. The test holds the
  `inner` mutex and sleeps 50 ms to land an abort in the listener-registration
  window. The issue reports a 10-second receive timeout and a passing 0.05-second
  WSL isolated run. The hang mechanism is explicitly unknown.
- `route_run::exec_infra_failure_returns_error` failed in Ubuntu run
  `39764859083` with `child was spawned through the grant then killed`; the
  source is `crates/cheaper-cli/tests/integration/route_run.rs:628`. Issue
  evidence relates this case to a 1 ms timeout and missing child marker, but
  does not establish whether spawn, marker observation, or kill ordering is at
  fault.
- `hook_config::hook_config_sync_warns_on_legacy_conflict` reported
  `ETXTBSY` while executing a copied `CARGO_BIN_EXE_cheaper` under full-suite
  parallel load in WSL; an isolated rerun passed. The issue does not establish
  which copy/execute lifecycle operation owns the race.
- A later #187 comment reports one Windows failure in
  `proc_command::abort_kills_and_preserves_reason` at `assertion failed:
  !alive(pid)` during a full check; isolated and full reruns passed. The issue
  author calls it a process-kill liveness timing race, but its causal relation
  to the POSIX failures remains unproven.
- The `proc_invocation.rs` Unix compile failure was fixed by PR #186. It is not
  a remaining #187 repair target.

これらは Issue とその comment に記録された観測であり、この briefing で
独立に再現した結果ではない。別 agent の bounded diagnostic probe が次を確認した。

### 現 checkout の bounded probe 結果

- **resources:** 現行 `resources.rs:665-670` は listener 登録後、同じ `inner` lock の下
  で `signal.reason()` を再確認し、abort 済み waiter を除去して rejection する。現行
  test (`resources.rs:1300-1348`) は manager mutex を保持して50 ms後にabortするが、
  worker が pre-check を通過してlock待ちになったことを同期的には保証しない。probe は
  WSL で対象test 1/1、`resources::tests` 15/15 pass と報告した。これは過去の Ubuntu
  stall の原因や修正済みを証明しない。
- **route_run:** 現 checkout の test は child marker を要求せず、`ToolExecutionFailure`
  と `routed command did not complete` を検証している。これは承認済み #157 spec の
  test-side timing fix と一致する。probe の focused test は1/1 pass。過去の #187 観測は
  現 checkout で既に覆われている可能性が高く、新たな修正を正当化する証拠はない。
- **hook_config:** `ETXTBSY` は capability check の `Command::output` が unique temp PATH
  shim を実行する箇所で起きた。copy は同期実行で temp dir も unique だが、failure-time
  executable path/writer/timestamps は保存されておらず、copy/execute overlap は証明されて
  いない。probe の focused test は1/1 pass。原因を選ぶ証拠がない。
- **Windows liveness:** failure は `wait_for(|| !alive(pid))` が true を返した後に最終
  `assert!(!alive(pid))` が失敗している。`alive` は exit status を見ず `tasklist` output の
  substring で PID を探すため、dead→alive の非単調観測であり、cleanup 不履行の証拠では
  ない。probe の focused test は1/1 pass。process handle を用いた安定 oracle の可否と、
  失敗時の tasklist stdout/stderr, child identity/state の capture が必要。

各結果は probe が報告した単回の focused pass であり、full-suite または歴史的 failure
の非再現を示さない。briefing 担当 agent 自身は test を実行していない。

### 制約

- 各 failure site を独立に帰属させる。Issue 内でまとめられていることを理由に
  共通 root cause を推定しない。
- abort outcome、child/process termination、hook behavior、failure reporting
  の assertion を保つ。coverage を skip・disable・弱体化しない。
- failure path は無期限に hang せず、bounded な診断結果を返す。
- #188 の ambient Node cleanup は別 chain とする。#162 の実 host 環境不足は
  この chain の前提にしない。
- issue #157/#180 の承認済み test-boundary・deterministic-test 方針を、適用条件が
  一致する場合に再利用する。本番層の変更は production invariant 違反の証拠が
  得られた場合に限る。

## Alternatives and Comparison

| 選択肢 | 利点 | 欠点 | 現時点の見立て |
| --- | --- | --- | --- |
| 事前に契約違反を再現したうえで、時間窓や fixture を所有する test 側を直す | 無効状態が test synchronization / fixture に属する場合に本番 behavior を変更せずに済み、#157/#180 の承認済み方針に合う | test が実際の runtime lifecycle defect を示している場合は不適切。各原因の証拠が必要 | 原因が test 側と確認された failure site に限って第一候補 |
| abort / spawn / cleanup の本番 invariant 違反が証明された場合に本番 owner を直す | 実際の外部 observable defect を直し、意味のある test を維持できる | behavior risk が広い。Issue の報告だけでは違反を特定できない | production owner の failure が証明された場合のみ |
| sleep/timeout 延長、suite retry、対象 test 抑止を行う | 観測上の失敗を短期的に減らす可能性がある | race を隠し、hang を長引かせ、契約を直さず検証信号を除去し得る | 単独の解決策としては採用しない |

## Tentative Conclusions and Hypotheses

- **Issue evidence で確認済み:** residual POSIX failure は3件として報告され、suite の
  block または間欠 fail と記録された。Unix compile error は修正済み。Windows abort
  liveness assertion は一度 fail し、隔離実行と全体 rerun は pass した。
- **resources の現状:** listener 登録後の abort 再確認は現 checkout だけでなく、歴史的
  stalled workflow run `40149087832` の head `5e28c40d414a0ae8142c7b5bbc992e6d21b497b9`
  と run `40153767045` の head `55741984401e5c84d59fd53bada6b197925f9e2c` にも存在する。
  両方で `signal.reason()` の再確認・waiter 除去・respond/deferred-pump が確認され、
  `865765f6` (Oct 8) から両 head への ancestry も確認済みである。従って旧 recheck の
  欠落はこの historical stall の原因ではなく、stall は guard 存在下の未局在問題である。
  現 test の mutex + 50 ms sleep は worker phase を確定しないため、次工程は同期点と
  bounded watchdog を設計し、stall 時の状態を診断する。
- **route_run の現状:** historical failure head `51f2f4c` は #157 fix `9de23f8` の祖先で
  ある。fix は child marker assertion を除去し、current test は `ToolExecutionFailure`
  と `routed command did not complete` を確認する。歴史的失敗は現行契約より前の
  revision に属するため、この site は既存 #157 correction で覆われる disposition とし、
  route code change は追加しない。
- **仮説 — copied executable ETXTBSY:** fixture の copy と execute が並行する可能性は
  あるが、Issue はその schedule を立証しておらず、別の filesystem/process
  interaction も除外していない。
- **仮説 — Windows liveness:** process termination と test の liveness probe の観測が
  race する可能性がある。POSIX 報告と同じ機構だと仮定しない。
- 修正境界は原因を再現した後、各 test または runtime owner から個別に選ぶ。
  3つの報告を共通 abstraction や単一の本番 fix の根拠にしない。

## Open Questions

- resources の既存 guard 下で起きた stall は、どの worker/listener phase・wait budget・
  lock state で発生するか。test の synchronization phase を確実に固定し、bounded timeout
  時に診断情報を返す方法は何か。
- unique copied PATH shim の capability check が `ETXTBSY` になった時の exact executable
  path、file metadata/timestamps、writer process/operation は何か。bounded reproduction で
  これらを採れない場合の supported disposition は何か。
- Windows の dead→alive `tasklist` 観測は、既存依存で得られる process-handle identity に
  よる待機 oracle に置き換えられるか。置き換えが適切でない場合、どの bounded output と
  evidence で残余不確実性を記録するか。
- 各 failure の test、phase、process/listener state、経過 budget を特定できる、bounded
  で有用な failure diagnostic は何か。
- final verification に使える Linux/POSIX と Windows host は何か。どの command と
  repeat / parallel-load 条件が十分か。

## Promotion Candidates

- [x] spec-doc needed: residual suite failure の deterministic な owner-boundary 修正と
  cross-platform acceptance criteria を定義する。
- [x] adr-doc needed: deterministic phase synchronization、安定した process identity による
  test oracle、bounded diagnostic と supported disposition を選ぶ cross-cutting test policy
  を ADR に記録する。既存 #157/#180 の route-specific decisions は維持し、重複しない。
  ADR は independent review を経て accepted。設計判断であり、実装承認ではない。

## Sources and Relations

- [Issue #187](https://github.com/scarecrow173/cheaper/issues/187) と
  [Windows liveness comment](https://github.com/scarecrow173/cheaper/issues/187#issuecomment-6394277590)。
  Run ID、報告症状、上記 scope exclusion の根拠。原因の証拠ではない。
- [Issue #157 spec](../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md)
  と [issue #180 spec](../specs/deterministic-proc-process-tests-issue-180.md) は既存の
  合意済み context であり、#187 の原因が一致する証拠ではない。
- [Issue #180 test-boundary ADR](../adr/resolve-proc-process-test-flake-at-the-test-boundary.md)
  と [issue #157 test-determinism ADR](../adr/resolve-route-test-flake-via-test-side-determinism.md)
  が既存の修正 class を規定する。新しい #187 cross-cutting decision は
  [accepted ADR](../adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md)
  に記録した。
- 完了した diagnostic probe は現 checkout の bounded focused pass と既存コードの証拠を
  追加したが、歴史的 resources stall、ETXTBSY、Windows liveness failure は再現・帰属
  されていない。resources historical heads の ancestry/recheck evidence は旧 missing-guard
  説を排除するが、stall の機構を特定しない。route chronology は #157 ancestry で閉じた。
  ETXTBSY と Windows liveness は bounded diagnostic capture と supported disposition を
  求める approved design に含まれるが、因果原因は未確定のままである。

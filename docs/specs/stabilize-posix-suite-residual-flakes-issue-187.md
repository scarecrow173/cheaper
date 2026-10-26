---
id: "SPEC-034MwgZyWwDaYNIFU0hnVv"
type: "spec"
status: "approved"
title: "Issue #187: 残存 POSIX test suite failure の安定化"
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
  implemented-by:
    - "../plans/stabilize-issue-187-posix-suite-residual-flakes.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/posix-suite-residual-flakes-issue-187.md"
  derived-by:
    - "../designs/issue-187-posix-suite-residual-flake-stabilization.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md"
    - "deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "deterministic-proc-process-tests-issue-180.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  defers: []
  deferred-by: []
---

# Issue #187: 残存 POSIX test suite failure の安定化

## What Should Be Built

各 test が守る behavior を維持しながら、POSIX test suite の完了を妨げる残存
nondeterministic failure を解消する。`resources`、`route_run`、`hook_config` の
POSIX 3件と、別途報告された Windows process-liveness assertion を診断し、破られた
invariant を所有する層でそれぞれ扱う。共通原因の証拠がない限り、原因と修正境界を
一つにまとめない。

## Why Now

`ubuntu-latest` workflow run `40149087832` は resources abort-window test で66分超
停止し、run `39764859083` は route integration test で失敗した。並列 WSL full run でも
hook configuration test の `ETXTBSY` が報告されたが、隔離 rerun は pass した。後日の
#187 comment には Windows process-liveness failure が1回記録され、rerun は pass した。
これらの結果により POSIX verification が未完了となり、suite の signal が不安定に
なっている。元の Unix-only `proc_invocation` compile error は PR #186 で修正済みで、
残存修正対象ではない。

## Users and Value

maintainer と contributor が必要とするのは、対応 POSIX CI run の完了と、Windows
verification での信頼できる process-lifecycle coverage である。failure は有用な
diagnostic とともに終了し、CI stall が最初の契約違反を隠さないこと。

## Scope

### In Scope

- `cheaper-tools::resources::tests::abort_landing_before_listener_registration_still_rejects`:
  historical stalled heads `5e28c40d414a0ae8142c7b5bbc992e6d21b497b9`
  (run `40149087832`) and `55741984401e5c84d59fd53bada6b197925f9e2c`
  (run `40153767045`) already contain the post-registration `signal.reason()` recheck,
  waiter removal, response, and deferred-pump path; `865765f6` is their ancestor.
  Do not attribute these stalls to a missing recheck. Add a private
  `#[cfg(test)]`, instance-scoped one-shot phase hook immediately after the real
  successful pre-check and before request enqueue. The worker waits on a bounded
  test-owned channel at that point; the test observes the phase, aborts, then
  releases it to traverse the real enqueue/listener-registration/recheck code.
  This proves abort landed after the actual pre-check and before enqueue; it
  does not claim second-lock contention. The hook is shared only by clones of
  that manager and absent from production builds/public API. Preserve rejection,
  original abort reason, and future manager usability.
  Change runtime only if new causal evidence identifies a production invariant violation.
- `cheaper-cli` integration test
  `route_run::exec_infra_failure_returns_error`: 報告 run と current test revision の前後を
  照合済み。historical failing SHA `51f2f4c` は #157 fix `9de23f8` の祖先で、child
  marker assertion は同 fix で除去された。current test は `ToolExecutionFailure` と
  `routed command did not complete` を確認する。この site は supported disposition とし、
  route code change を追加しない。
- `cheaper-cli` integration test
  `hook_config::hook_config_sync_warns_on_legacy_conflict`: `ETXTBSY` が capability check の
  `Command::output` による unique temp PATH shim execution で生じたことを記録する。
  `Command::output()` の `Err(io::Error)` は spawn/exec failure であり、`ErrorKind`、
  `raw_os_error`、message、source/copy executable paths、bounded file metadata/timestamps を記録する。
  path は診断上限内では全文を記録する。上限を超える場合は切り詰めを明示し、先頭・末尾、
  元の byte 長、暗号用途ではない安定識別値を残す。
  この branch に child process、exit status、stdout/stderr は存在しない。`Ok(Output)` は
  child 起動後の outcome で、exit status と bounded stdout/stderr を別 branch で記録する。
  原因未局在の間は retry、skip、assertion weakening を導入しない。
- `cheaper-tools::tests::proc_command::abort_kills_and_preserves_reason`: Windows の
  `!alive(pid)` failure は `wait_for(|| !alive(pid))` が true を返した後の assert で起き、
  `alive` が exit status を無視して `tasklist` output の substring で PID を検出するため、
  dead→alive という非単調 oracle の可能性がある。process termination と abort-reason
  assertions を保つ。PID file 検出後、abort より前に test-local Win32 `OpenProcess`
  (`SYNCHRONIZE | PROCESS_TERMINATE`) handle を開き、abort 前は未 signal と確認する。abort 後は同じ process
  object を `WaitForSingleObject` の bounded wait で観測する。`tasklist` は supplemental
  diagnostic のみで sole oracle にしない。test-local FFI を使い dependency/public API を
  増やさない。共通原因の証拠が出るまで独立症状として扱う。
- 対応 POSIX host と Windows で focused regression coverage および意味のある full-suite
  verification を行う。

### Out of Scope

- PR #186 ですでに修正された Unix `proc_invocation.rs` compile error。
- [Issue #188](https://github.com/scarecrow173/cheaper/issues/188) で別管理する ambient
  Node test-site cleanup。
- issue #162 の host environment setup。この chain の前提ではない。
- CI retry、test skip/disable、assertion の弱体化、timeout 延長だけで済ませる対応。
- 関係のない process、hook、POSIX suite failure。

## Requirements

- 修正を選ぶ前に、各 failure を報告時の trigger を制御して再現するか、原因を因果的に
  切り分ける。最初に test/runtime 契約が外れる event を、test、platform、load 条件、
  revision、bounded wait または process state とともに記録する。focused pass 一回だけを
  歴史的 failure の解消根拠にしない。
- failure ごとに test/fixture または production の境界を独立に選ぶ。本番 behavior は
  production-owned invariant 違反の証拠がある場合だけ変更する。Issue 内の分類や rerun
  結果から共通 root cause を推定しない。
- 各 test が意図する abort、process termination、hook configuration、error reporting
  behavior を保つ。原因を捉える regression test は、記録された defect で fail し、
  owner-boundary 修正後に pass する。
- 影響を受ける wait/failure path は定義済み budget 内に終了し、失敗した operation、
  child/listener、elapsed budget を特定できる状態を出す。test を無期限 pending にしない。
- `Command::output` の `Err(io::Error)` と `Ok(Output)` を分ける。`Err` に対して存在しない
  child/status/output を記録せず、`Ok` を spawn error と誤分類しない。
- retry、sleep、長い timeout、環境 workaround を、診断済み原因の修正または明示的 defer
  の代替にしない。
- Windows liveness 報告は discovery と verification で独立に追跡する。診断によりこの
  修正外と判断する場合は、報告を黙って落とさず、根拠のある disposition を記録する。
- [ADR](../adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md) は
  deterministic test-boundary synchronization/oracles、bounded diagnostics、因果証拠がない
  production behavior changes の defer を記録する。ADR は accepted であり、既存
  #157/#180 decisions を維持する。implementation approval は別 gate とする。

## Acceptance Criteria

- [x] #187 discovery record に failure ごとの reproduction evidence、confirmed cause または
  明示的に未解決の選択肢、選定した owner boundary が記録され、証拠なしに4件を一原因と
  表現していない。
- [ ] resources abort-window の focused regression が listener 登録 window で abort が
  着地したときの rejection と original abort reason、subsequent manager usability を検証する。
  private instance-scoped test hook は real pre-check 直後・enqueue 前に one-shot pause し、
  test-owned bounded channel で abort/release を同期する。request future は Tokio timeout、
  outer watchdog は phase/result wait と `JoinHandle::is_finished()` wait を budget 内に行う。
  `join()` は終了確認後だけ呼び、watchdog expiry 時に unbounded join しない。この test は
  pre-check/enqueue gap を保証し、second-lock contention は主張しない。
- [x] route-run report の CI revision が現 #157 correction より前か後かを照合し、current
  test が child marker timing に依存せず `ToolExecutionFailure` と
  `routed command did not complete` を検証することを確認する。現行 test がこの契約を満たす
  場合は #187 code change を重ねず、historical symptom が既存 correction で覆われる根拠を
  記録する。historical failing SHA `51f2f4c` は fix `9de23f8` の祖先と確認済み。
- [ ] hook-config regression が legacy-conflict warning assertion を維持し、以前
  `ETXTBSY` が報告された concurrency で実行できる。`Command::output` の `Err(io::Error)` は
  `ErrorKind`/`raw_os_error`/message と source/copy executable paths/file metadata/timestamps を記録する。
  path は診断上限内では全文を記録する。上限を超える場合は切り詰めを明示し、先頭・末尾、
  元の byte 長、暗号用途ではない安定識別値を残す。
  存在しない child/status/stdout/stderr は出さない。`Ok(Output)` は status/stdout/stderr を別に
  記録する。原因が再現不能なら条件と証拠を記録した supported disposition を independent
  review で受け、speculative retry を使わない。
- [ ] Windows process-liveness verification が、PID file observation 後・abort 前に開いた
  process handle の identity と bounded termination wait により abort reason と child
  termination を示す。`pending.await` 自体も有限 outer watchdog で囲み、timeout 診断に operation,
  phase, elapsed time, allowed budget, PID を含める。期限切れ時は `pending` を再 await/join せず
  abort handle を non-blocking に abort し、別 budget の fixture cleanup として同じ process
  handle に `TerminateProcess` と bounded wait を適用する。handle は success/error/timeout の
  全経路で RAII により close する。test-local Win32 API を使い、新 dependency は追加しない。
  `tasklist` は supplemental diagnostic のみ。handle oracle が実装できない理由が出た場合は
  evidence 付き alternative/disposition を independent review で承認する。runtime cleanup failure
  は未証明で、retry/skip/assertion weakening は採らない。
- [ ] 影響する test は有効なままで意図した behavior を assertion する。acceptance が
  retry、timeout 延長、coverage skip のみを根拠としない。
- [ ] POSIX full verification は provisioned Ubuntu CI で `mise install` → `mise run setup` →
  `mise run check` を実行し、host、各 command、result を記録する。WSL は Node unavailable の
  ため canonical full-check host とせず、fake pass に置き換えない。
- [ ] Windows は対応 host で `mise install` 後に `mise run check` を連続3回以上実行し、各 run
  の開始/終了、command、result を記録する。単回 focused pass を反復 gate の代替にしない。

## Deferred Design Concerns

<!-- 現時点で deferred design item は確定していない。#188 と #162 は scope boundary
     として分離し、暗黙の依存関係を作らない。 -->

## Review and Approval Record

- Reviewer A: independent pass, no issue candidates.
- Reviewer B: fresh adversarial pass, no issue candidates.
- Validator ledger: `0 confirmed / 0 rejected / 0 unverified`.
- Judge: `ACCEPT WITH NON-BLOCKING NOTES`; no confirmed P0/P1. The route-run
  chronology question is closed by the SHA ancestry evidence above. Resources
  historical stalls remain post-recheck and unlocalized. Implementation/code
  approval requires causal evidence or an independently reviewed supported
  disposition for every symptom.
- Approval: the user delegated approval to the assigned agent once blockers
  were gone. This approves the spec as design input, not implementation or
  product-code changes.
- Design round 1 Judge: confirmed P1 on the resources handshake proof; the
  design and ADR were corrected before independent round-2 review.
- Design round 2: Reviewer A and Reviewer B completed; Validator ledger
  `0 confirmed / 0 rejected / 0 unverified`; final Judge `ACCEPT`, with no
  confirmed P0/P1. The accepted design requires a phase hook after the actual
  successful pre-check and before enqueue; it does not claim second-lock
  contention. Hook `ETXTBSY` and Windows liveness causes remain unresolved.
- Design approval: user-delegated approval authority exercised after blockers
  were cleared. This approves the design and ADR as implementation inputs;
  implementation/code approval remains a separate gate.

## Sources and Relations

報告された事象と rerun 結果の出典は [issue #187](https://github.com/scarecrow173/cheaper/issues/187)
および [Windows liveness comment](https://github.com/scarecrow173/cheaper/issues/187#issuecomment-6394277590)。
原因を示す証拠ではない。[issue #157 spec](deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md)
と [issue #180 spec](deterministic-proc-process-tests-issue-180.md)、および各 test-boundary
ADR は、因果条件が一致する場合に再利用する既存 decision である。完了した bounded
diagnostic probes は current focused pass とコード上の契約を確認したが、歴史的 resources
stall、`ETXTBSY`、Windows liveness failure の root cause は再現・確定していない。この spec
は `approved` である。ADR は accepted、design は approved である。
Implementation/code approval は別 gate である。

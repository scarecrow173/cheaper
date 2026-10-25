---
id: "PLAN-034LIvc25ZGY86L0WGpFmy"
type: "plan"
status: "approved"
title: "Fix proc_process test flake via test-side determinism (issue #180)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/180"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/deterministic-proc-process-tests-issue-180.md"
  implemented-by:
    - "docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md"
    - "docs/tasks/verify-proc-process-determinism-and-mise-check.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/fix-route-test-flake-via-test-side-determinism.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/deterministic-proc-process-tests-issue-180.md"
    - "docs/designs/deterministic-proc-process-tests-issue-180.md"
    - "docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  derived-by:
    - "docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md"
    - "docs/tasks/verify-proc-process-determinism-and-mise-check.md"
  verifies: []
  verified-by:
    - "docs/test-specs/deterministic-proc-process-test-assertions-for-parallel-load-flake.md"
  references:
    - "docs/discovery/proc-process-test-flake-under-parallel-load.md"
    - "crates/cheaper-tools/tests/proc_process.rs"
  defers: []
  deferred-by: []
---

# Fix proc_process test flake via test-side determinism (issue #180) Implementation Plan

## Goal

issue #180 の process-tree 系 flaky test(2026-10-23、Windows local、full-suite
並列負荷で 2 件の transient fail)を、ADR Option A(test 側 determinism +
failure diagnostics)に従って deterministic な構成へ修正する。変更面は
`crates/cheaper-tools/tests/proc_process.rs` 単一 file の test コードのみ
— `src/tools/proc/` の本番実装変更は一切行わない。

## Scope

単一 implementation stream — 同じ violated invariant(test の assertion が
制御下にない wall-clock/ambient 状態を観測する)に対する同一方針の修正で、
critical path と release intent が同一(1 PR で process-tree test を負荷非
依存にする)のため分割しない。

In scope(design In Scope のまま転記):

- `proc_fixture_child` fixture: `pids.d/{parent,child,grandchild}` の
  incremental per-key emit(atomic tmp+rename)・`PID_DIR_ENV`
  (`CHEAPER_PROC_PID_DIR`)が唯一の伝達路・unix `graceful` mode の
  `parent.armed`/`parent.terminated` marker・`SpawnedChild`
  kill-on-drop guard・`spawn_fixture` の stderr `Stdio::inherit()`。
- `cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout`:
  mode 別 task timeout・outcome 分類表・早期 `Timeout` 受理・per-mode
  emit floor・bounded post-resolution emit 回収・emit 済み全 pid の
  消滅検証・`TaskRun` guard。
- sibling test `times_out_native_process_and_cleans_tree`・
  `allows_sigterm_exit_within_grace_before_force_kill`(unix)・
  `abort_kills_managed_parent_and_descendant_and_preserves_reason` への
  同一構造適用。
- `wait_for`/`alive` helper 全体: `WaitCtx` context 付き `wait_until`
  統一(panic 版 wait は作らず全失敗経路を `diag_fail` へ集約)・
  unfiltered `tasklist /NH /FO CSV` snapshot の client 側照合
  (`live_pids` へ統合、`alive` 廃止)・poll cadence 緩和・
  context-carrying `expect` 化。
- panic-path leak containment: `TaskRun`(JoinHandle + drop-guard 同期
  `kill_emitted`)・`abort_and_drain` give-up path。
- failure diagnostics `dump_failure`/`diag_fail`: error `source()` chain・
  `find_cheaper_error` details JSON・mode・elapsed・emit 状態を stderr
  採取し全 failure 経路で必ず呼ぶ(`node_task_resolves_via_path` 含む)。

Out of scope(spec / design と同じ境界):

- `crates/cheaper-tools/src/tools/proc/`(`run`/`drive`/`monitor`/`cleanup`/
  `tree`/`spawn`/`executable`)と `tools/exec` の本番実装変更 — diagnostic
  が tool 側帰属を示した場合の conditional phase-2 は別 issue。
- ambient `node` PATH 解決自体の変更・他 test file・CI 構成変更。
- `executes_literal_argv_...`・`fails_nonzero_exit_...`・
  `caps_combined_output_...`・`touch_with_matching_write_scope_...` の
  assertion 意味の変更(`wait_for`/`live_pids` signature 変更に伴う機械的
  callsite 更新のみ)。
- 新規依存(std・`libc`・`tokio`・`serde_json` の既存依存のみ)。
- test 側 `live_pids`/`kill_emitted` の creation-time guard(spec
  Residual (c) — documented, not fixed)。

## Upstream Documents

- `docs/specs/deterministic-proc-process-tests-issue-180.md`
  (SPEC-034LFaC05pNn26SBFu5KUC, approved) — 要件・Requirements・Acceptance
  Criteria・Residual Race Vectors。拘束力のある contract。
- `docs/adr/resolve-proc-process-test-flake-at-the-test-boundary.md`
  (ADR-034LFaJmc8htAt3pDMjMQ2, accepted) — Option A「test 側 determinism +
  failure diagnostics」の構造決定(B-F 却下の根拠を含む)。
- `docs/discovery/proc-process-test-flake-under-parallel-load.md` —
  R1(構造 race 確定)・R2-R5・R4(未確定)の原因調査。

## Design Inputs

- `docs/designs/deterministic-proc-process-tests-issue-180.md`
  (DESIGN-034LGDPeqMOtv5azFEm4Ei, approved) — D1-D8 の決定事項が本 plan
  の authoritative HOW。本 plan は design を implementation order へ転記
  したもので独自判断を足さない。

design との差分(review 確定の justified deviation — merged task doc にも
転記済み):

- **panic 版 wait helper を作らない** — design :199-200(panic 版
  `wait_for`/`wait_emitted` は panic→Drop 経路の意図箇所に限る)と
  :221-223(wait/verify の failure 出口は panic 版 wait を通さず全て
  `diag_fail` へ)の内部矛盾を、design wording 修正で `wait_until` +
  `diag_fail` の authoritative 形へ解消した。
- **自然解決の emit floor は全 expected key** — design outcome 分類表
  の「自然解決なら全 3 件」 semantics を採用し、§3 の emit ≥1 記述を
  design wording 修正で揃えた。
- **dev-seam feature flag を `-p cheaper-tools` コマンドから落とす** —
  design Verification Notes の `cargo test ... --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` は `cheaper-tools` が
  両 crate に依存しないため `-p cheaper-tools` では feature 解決不能。
  `-p cheaper-tools` の全コマンドは flag 無しで実行し、`mise run
  check` の workspace 経路では design どおり feature 付きで走る。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-tools/tests/proc_process.rs` | modify | fixture emit protocol・deterministic wait/kill/verify helpers・leak containment・failure diagnostics・対象 4 test(+`node_task_resolves_via_path`)の再構成 | 現行 717 行、design File/Line Edit Map の行 anchor 参照 |

## Implementation Sequence

2 task の直列 — emit protocol の producer(fixture 各 arm)と
consumer(test 側 emit 観測 helper)が同じ file に同居し、protocol
変更と consumer 実装を分けると中間状態で compile/test が成立しない
(未接続 helper が dead code で `clippy -D warnings` fail + 旧
`pids.json` 読み側が残る段階では対象 test が確実に fail)ため、
producer/consumer は単一 task で atomic に flip する。詳細な作業項目は
merged task doc の §1-§4 を参照。

### T1(merged) — deterministic proc_process test rewrite

- [ ] §1 fixture emit protocol(D1): `PID_DIR_ENV` + budget/poll 定数・
      `emit_pid`/`emit_marker` atomic emit・fixture 各 arm を `pids.d/`
      per-key emit へ(emit 責務 mode で env 未設定なら mode 名入り
      panic)・`spawn_fixture` → `SpawnedChild` guard・`fixture_task`
      optional `pids_dir`(`Some` caller のみ env+`outputs:["pids.d/**"]`)
      ・`tool_with` definition-seam variant
- [ ] §2 deterministic helper 層(D4-D7): `WaitCtx`/`wait_until` 統一
      (panic 版 wait 無し)・`emitted_pids`/`collect_emitted`・
      `live_pids`(unfiltered tasklist CSV / unix `kill(pid,0)`+zombie、
      `alive` 全 callsite 置換)・`verify_emitted_dead`・`kill_emitted`・
      `TaskRun`(abort/settle/abort_and_drain/disarm/Drop、`handle.abort()`
      非呼出し)・`dump_failure`/`diag_fail`
- [ ] §3 `cleans_live_descendants` 変換(D2・D3): mode 別 timeout
      (`"60s"`/`"2s"`)・outcome 分類表 + 早期 `Timeout` 受理・abort
      mode `wait_until`+`select!`・per-mode emit floor(自然解決 = 全
      3 件、60s-armed Timeout = emit ≥1、deep-stubborn = 0-emit 受理)・
      settle 後 bounded collect → `verify_emitted_dead` → `disarm()`
- [ ] §4 sibling + node 変換: `times_out_native_process_and_cleans_tree`
      (0-emit 受理)・`abort_kills_managed_parent_and_descendant_and_
      preserves_reason`(全 key `wait_until` select! → abort → 2 件消滅)・
      `allows_sigterm`(killGrace `10s` + `.armed` marker 分岐、unix)・
      `node_task_resolves_via_path`(Err 経路 `diag_fail`)
- [ ] `mise exec -- cargo test -p cheaper-tools --test proc_process` が
      exit 0(全 test 含む)+ fmt/clippy clean

### T2 — 全体検証

- [ ] `cargo test --test proc_process` Windows + POSIX(`gh workflow run
      ci` または local POSIX env)で pass
- [ ] `mise run check` ×3 連続 pass(smoke signal)
- [ ] diagnostics 実効確認(一時的 budget 潰し → revert)・emit floor
      分類確認(極小 timeout → mode identity 基点の floor 動作確認 →
      revert)・diff review(`{key} gone`/`/FI`/`alive(`/変換経路の
      context-free unwrap 不残存)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 deterministic test rewrite | — | T2 |
| T2 全体検証 | T1 | — |

critical path は T1 → T2 — producer/consumer atomicity のため実装は
単一 task、検証のみ別 task に分離する。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 早期 `Timeout` 受理が genuine regression を一部 mask する | med | med | 自然解決は全 expected key の emit を要求(不足は `diag_fail`)、60s-armed mode の Timeout 解決は emit ≥1 を要求(0-emit は broken fixture `diag_fail`)。diagnostics が再発時の帰属を採取。spec が明示的に選んだ trade-off |
| 2 | helper signature 変更の callsite 更新漏れ・未接続 helper の残存で compile/clippy fail | med | med | merged task で `alive` → `live_pids` の全 callsite を一括置換し panic 版 wait(`wait_for`/`wait_emitted`)は作らない — task 内 verification で `cargo test --test proc_process` exit 0 + `clippy -D warnings` を確認。dead helper を残さない |
| 3 | `select!` 内で panic する wait を使い stall が unwind して abort 経路を破壊 | low | med | panic 版 wait helper は作らず `wait_until` のみを mandate(deviation 記録済み) — `select!` に panic 経路を持ち込めない構造で排除。stall は `abort_and_drain` → `diag_fail` の正式 give-up 経路 |
| 4 | Windows `tasklist` CSV parse の表現 drift(memory 列の `"12,345 K"` 等)で `live_pids` が誤判定 | low | med | parse 規則を design D4 の確定値(`"` prefix・`","` split・field 1=PID)で実装し、nonconforming output は全員 alive 扱い(conservative — vacuous pass にならない) |
| 5 | `tasklist /FI` filter 複数併記の再導入で `live_pids` が vacuous 化 | low | high | spec/design が禁止を明記 — unfiltered `/NH` snapshot + client 側照合のみ。review で `/FI` 非含有を確認 |
| 6 | panic mid-`drive()` leak の accepted residual(pre-emit panic で parent ≤1 process) | low | low | emit-first 配置で window を構造的に最小化済み。`abort_and_drain`(runtime 生存中の give-up)が主機構として全 window を覆う。documented residual |
| 7 | liveness/kill の PID-reuse 誤判定(spec (c) accepted residual) | low | low | `kill_emitted` の live ∩ emitted ∧ `current_exe` image 照合 + `SpawnedChild` `try_wait()` gate で誤 kill window を狭める。documented, not fixed |
| 8 | timeout mode の壁時計増(60s-armed mode・helper storm 下の settle/drain worst ~90s) | low | low | deep-stubborn は `"2s"`・sibling `"500ms"` 維持。`SETTLE_BUDGET` 180s・`DRAIN_BUDGET` 120s の明示 budget で bound |
| 9 | 外部依存 availability | low | low | 新規 crate・env・CI 構成変更なし。std・`libc`・`tokio`・`serde_json` の既存依存のみ |

### Rollback

- **Trigger**: 修正後も対象 test が不安定、または assertion が意図しない
  失敗を示す。
- **Procedure**: revert unit は本 change の commit — `git revert` で
  `proc_process.rs` を flake 修正前へ戻す。変更面は単一 test file のみ・
  本番 code 非接触・schema/migration/共有 state なしのため完全に戻せる。
- **Verification**: revert 後に `mise exec -- cargo test -p cheaper-tools
  --test proc_process` が(元の非対称 pass/fail 傾向で)実行できること。

## Task Breakdown

task doc へ分割した slice(2 task — producer/consumer atomicity のため
実装は merged 単一 task):

- T1(merged): deterministic proc_process test rewrite(emit protocol +
  helper 層 + `cleans_live_descendants` 変換 + sibling/node 変換)
  — `docs/tasks/implement-pids-d-emit-protocol-and-test-helper-layer.md`
- T2: 全体検証 — `docs/tasks/verify-proc-process-determinism-and-mise-check.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| 対象 test が pass(全 4 mode・sibling・node) | `proc_process` test target 全体 | `mise exec -- cargo test -p cheaper-tools --test proc_process` |
| `cleans_live_descendants` が mode 別 timeout・早期 Timeout 受理・emit 済み全件消滅検証の経路を通る | 対象 test 単体 | `mise exec -- cargo test -p cheaper-tools --test proc_process cleans_live_descendants` |
| sibling test 各件 | per-test filter | `mise exec -- cargo test -p cheaper-tools --test proc_process times_out_native_process_and_cleans_tree` / `... abort_kills_managed` / `... allows_sigterm`(unix のみ) |
| diagnostics 実効確認 | 意図的に budget を潰した一時的変更で `diag_fail` が error chain/details/mode/elapsed を出力 → revert | Manual(一時的変更 + revert — design Verification Notes) |
| emit floor 分類確認 | timeout を極小にした一時的変更で mode identity 基点の emit floor(deep-stubborn 0-emit 受理・60s-armed 0-emit `diag_fail`・emit ≥1 受理 + 消滅検証)を確認 → revert | Manual(同上) |
| `mise run check` 3 連続で proc_process 系が失敗しない(smoke signal — ~1 件/日 rate に対する証明ではない) | canonical verification contract ×3 | `mise run check` |
| 再発時帰属判別 | `diag_fail` 出力が `"process tree helper failed"`/`"managed process cleanup failed"` 系と node 起因を区別できる | 再発時運用 — diagnostics が tool 側帰属を示したら判断記録を docs へ追記 + follow-up issue |

## Open Questions / Gaps

- design が assertion 形状・timeout 値・helper 形・budget 階層・分類表を
  D1-D8 で確定済みのため、実装上の不明点は無い。
- exit 255 の帰属(R4)は第 1 段では diagnostics のみ — 再発時の
  `diag_fail` 出力が `"process tree helper failed"` 由来か node 起因かを
  判別する。tool 側帰属なら conditional phase-2(別 issue)で
  `WinTree::terminate` の helper 連鎖 hardening を判断する(design
  Deferred Design Concerns)。
- 3 連続 green は ~1 件/日 rate に対する smoke signal で証明ではない —
  spec AC と同じ受容。

## Self-Review

- [x] Every spec requirement maps to a task.(Requirements 全量 + Residual
      (a)(b)(d) → merged implementation task、AC の検証列 → 全体検証 task)
- [x] No placeholders remain.(timeout 値・budget 階層・分類表・verification
      command は design の確定値を転記)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution — 実装は merged 単一 task で
  `crates/cheaper-tools/tests/proc_process.rs` 単一 file を編集し、
  T2 が収束検証の review checkpoint。
- Independent work streams: none(T1 → T2 の直列)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: upstream 4 doc(spec approved・design approved・ADR accepted・
  discovery resolved)が timeout 値・budget 階層・分類表・helper 形まで
  確定済み。plan は design を implementation order へ転記したもので、
  Design Inputs 節に記した 3 件の justified deviation(panic 版 wait 不
  採用・自然解決 emit floor = 全 key・dev-seam flag 除去)以外の独自
  判断を足していない。
- 重点確認: (a) mode 別 timeout が `"60s"`/`"2s"`/`"500ms"` の design 確定
  値であること、(b) 60s-armed mode で 0-emit Timeout が `diag_fail` である
  こと(0-emit 受理は 2s/500ms mode のみ)、(c) `fail-tree` 受理が
  `message=="managed process failed"` 照合を含むこと(cleanup-failed と
  同 details を分ける)、(d) panic 版 wait helper が無く wait が
  `wait_until` + `diag_fail` に統一されていること、(e) `tasklist /FI` が
  残っていないこと、(f) `alive` wrapper が残っていないこと
  (clippy -D warnings)。

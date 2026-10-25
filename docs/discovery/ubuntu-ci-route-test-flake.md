---
id: "DISC-Gvis7cxcHxPQwsvoSGmuox"
type: "discovery"
status: "resolved"
title: "Ubuntu CI route test flake (mtime tie-break / 1ms timeout race)"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
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
  related:
    - "proc-process-test-flake-under-parallel-load.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "../designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "route 系テストが ubuntu CI で断続失敗する原因を確定し、決定的な修正で flake を解消できるか"
    scope: "route_run::exec_infra_failure_returns_error (1ms timeout race) と route::log_store::tests::sweep_enforces_ttl_count_size_and_skips_tmp (mtime tie-break) の 2 件"
    outcome: "確定 — 両方とも test 側の timing/enumeration 依存が原因。本番実装は正しい。test 側 determinism 修正で解消可能"
---

# Ubuntu CI route test flake (issue #157)

## Exploration Goal

issue #157 の 2 件の flake について、実コードを読んで原因を確定し、
deterministic な修正方針を決めることが目的。issue の推定(1ms timeout
race・mtime tie-break)を検証した結果、両方とも概ね正しいが、より
正確な機序が判明した。

## Key Issues and Assumptions

### `exec_infra_failure_returns_error`(route_run.rs:599-637)

- **confirmed**: test は grant `timeout: '1ms'` + `CHEAPER_ROUTE_CHILD_MODE:
  sleep` で子 process(テストバイナリ自身)を spawn させ、timeout kill
  後に `marker.exists()` を assert する(line 628-631)。
- **confirmed**: timeout は `proc::monitor::drive`(monitor.rs:225)で
  `sleep(timeout_ms)` として arm され、発火すると
  `timeout_controller.abort(Timeout)` → `wait_aborted` →
  `cleanup::start_cleanup` → `ProcessTree::terminate(kill_grace_ms)`
  で process group を終了する。POSIX は `kill(-pid, SIGTERM)` →
  grace 内 poll → `SIGKILL`(tree.rs:90-116)。
- **confirmed**: race の本質は「`command.spawn()` 成功後、timeout が
  ~1-3ms で SIGTERM を送る」のに対し、子側は exec + Rust test
  binary 起動(libtest init 含む)→ env 読取 → `fs::write(marker)`
  に通常でも数十 ms かかること。ubuntu CI の負荷では SIGTERM が
  marker write に先勝ち、`marker.exists()` が false で fail する。
  CI 観測(run 39763252995/39764859083)の assertion message
  "child was spawned through the grant then killed" と一致する。
- **confirmed**: Windows では `WinTree::terminate` が初回 powershell
  CIM snapshot + collect_snapshot + taskkill を経由するため kill が
  数百 ms 以上遅れ、子の marker write が必ず先に完了する — これが
  「Windows ローカルは pass、ubuntu CI は flake」の非対称性の直接因。
- **confirmed**: `is_err()` 側は deterministic(子は 30s sleep、
  timeout 1ms が必ず先に発火し `Err` を返す)。flake するのは
  `marker.exists()` だけ。
- **confirmed(推定の精緻化)**: timeout error は `proc::drive` 内でしか
  生成されず、`drive` は `command.spawn()` 成功後にしか入らないため、
  `ErrorCode::Timeout` は **post-spawn の決定的証拠**になる。error
  伝播: `CheaperError(Timeout)` → `AbortReason::from`(operation.rs:
  154-157)→ `aborted_err` → `tool_executor_error` が
  `find_cheaper_error` で code 抽出(workflow_run.rs:552-567)→
  `WorkflowExecutorError(Timeout)` → `outcome.error.code` →
  `route::run::finish` が `CheaperError::new(error.code,
  "routed command did not complete")` を返す(run.rs:450-463)。
  test は `err.downcast_ref::<CheaperError>()` で `code ==
  ErrorCode::Timeout` を deterministic に assert できる
  (`cheaper_tools`/`cheaper_core` は cheaper-cli の直接依存)。
- **修正注記(2026-10-22)**: 上記の伝播記述は誤り。
  `WorkflowExecutorError(Timeout)` → `outcome.error.code` の伝播は
  **発生しない** — engine は `outer_error` が無いまま job が失敗
  終了した場合、`outcome.error` を `CheaperError::new(
  ErrorCode::ToolExecutionFailure, "workflow job failed")` に
  collapse する(engine.rs:1176-1179)。`Timeout` code は per-job
  diagnostic にのみ残り、`route::run::finish` は
  `CheaperError::new(error.code /* ToolExecutionFailure */,
  "routed command did not complete")` を返す(run.rs:450-463)。
  したがって `Err` 側で deterministic に assert できるのは
  `code() == ErrorCode::ToolExecutionFailure` + message
  `contains("routed command did not complete")` であり、
  `ErrorCode::Timeout` の assert は全 platform で決定的に fail
  する。本注記は Option A 表・Tentative Conclusions の
  `code == ErrorCode::Timeout` 記述にも同様に適用される。
  spec/ADR/design はこの修正を反映して改訂済み。
- つまり `marker` による spawn 証明は、同じ識別力を持つ timing
  非依存の assertion で置き換え可能。

### `sweep_enforces_ttl_count_size_and_skips_tmp`(log_store.rs:559-606)

- **confirmed**: `sweep_with` は mtime を `duration_since(UNIX_EPOCH)
  .as_secs()` に**秒へ切り捨てて**収集し(log_store.rs:139-144)、
  `files.sort_by_key(mtime)`(line 166)で古い順削除する。test の
  連続 `fs::write` はほぼ必ず同一秒に乗り、ns 精度で異なっていても
  秒では tie になる。
- **confirmed**: Rust の `sort_by_key` は stable のため tie 時の
  順序は `fs::read_dir` の列挙順 = filesystem 依存。(b) では
  {fresh, a, b, c} の先頭 2 件が消え、`b.exists() || c.exists()`
  は削除対象がちょうど {b, c} のときのみ fail。(c) では {big 128B,
  small 8B} に対し max 100B で、small が先に並ぶと small+big の
  2 件削除 → `assert_eq!(removed, 1)` が `left: 2, right: 1` で
  fail — CI 観測(log_store.rs:602)と完全一致。
- **confirmed(推定の精緻化)**: tie-break 依存先は「readdir 列挙順」。
  ubuntu CI の FS(ext4/overlay)では小 directory でも作成順と一致
  しない順序を返し得るのに対し、Windows NTFS は name 順に近い
  列挙になりがちで、(b) は {a,b,c,fresh}、(c) は {big,small} と
  常に test 期待順になる → ローカル pass の非対称を説明できる。
- **confirmed**: (a) TTL block は `old.log` に `set_modified(past)` を
  適用済みで deterministic — 同一 test 内に既に正規形の pattern が
  ある。修正は (b)(c) でも同じく対象 file の mtime を明示的に
  順序付ければよい。
- 実装 semantics(秒精度・stable sort・tie 時任意順)自体は
  production で問題ない: route log file 名は `<unix-ms>-<pid>-
  <hex>.log` で mtime tie はほぼ起きず、起きても削除順は任意で
  許容される。主原因は「test が制御していない外部状態を観測して
  いる」ことで、修正の責務境界は test 側。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. test 側 determinism — sweep は `set_modified` で mtime 明示順序付け、infra test は `marker.exists()` を error `Timeout` code assert に置換 | 完全 deterministic・高速のまま(1ms timeout 維持可)・test の意図(post-spawn 証拠・古い順削除)を直接表現 | marker による「子が実際に走った」行動証拠は失う(ただし Timeout code が同等以上の post-spawn 証明) | yes |
| B. grant timeout を数百 ms〜秒に緩める(issue 対応案) | marker assert を維持できる | race を消さず window を広げるだけで理論上 flake 残存。test が timeout 分だけ遅くなる | no |
| C. 実装側に deterministic tie-break(`sweep_with` を (mtime, path) sort) | 全 caller で順序が安定 | production で mtime tie はほぼ起きず test 構築の問題を直さない。YAGNI — 将来必要なら deferred concern へ | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: 両 flake の主因は本番実装ではなく test の
  timing/enumeration 依存。本番コード変更は不要。
- **confirmed**: infra test は `timeout: '1ms'` のまま、assertion を
  `err` の `CheaperError.code() == ErrorCode::Timeout` 検証に
  置き換えるのが最短の deterministic 修正。`is_err()` と
  no-route-record assert は維持。
- **confirmed**: sweep test は (b)(c) で対象 file へ `set_modified`
  による一意 mtime を付与すれば `read_dir` 順に依存しない。
  合わせて削除・残存の assertion を厳密な file 集合へ絞れる。
- **hypothesis → 記録済み**: `sweep_with` に (mtime, path) の
  secondary sort key を足す hardening は任意。本 spec scope 外で
  deferred design concern として残す。

## Open Questions

- なし(blocker なし)。実装側 tie-break の要否のみ deferred concern
  として下流(spec)に引き継いだ。

## Promotion Candidates

- [x] spec-doc needed: deterministic route test の修正要件 —
  `docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md`
- [x] adr-doc needed: 修正方針選択の記録(アーキテクチャ判断自体は
  不要 — test-only の最小 ADR) —
  `docs/adr/resolve-route-test-flake-via-test-side-determinism.md`

## Sources and Relations

- issue #157: <https://github.com/scarecrow173/cheaper/issues/157>
- 失敗観測: actions run 39763252995(chore/security-toolchain-updates、
  route_run marker 未作成・rerun で sweep `left: 2, right: 1`)、
  39764859083(main、route_run 同所)
- 検証対象コード: `crates/cheaper-cli/tests/integration/route_run.rs`
  (599-637)、`crates/cheaper-cli/src/route/log_store.rs`(115-183,
  559-606)、`crates/cheaper-tools/src/tools/proc/monitor.rs`(205-262)、
  `crates/cheaper-tools/src/tools/proc/tree.rs`(POSIX 90-116 /
  Windows 465-499)、`crates/cheaper-cli/src/route/run.rs`(436-463)、
  `crates/cheaper-cli/src/workflow_run.rs`(552-567)、
  `crates/cheaper-tools/src/operation.rs`(145-157)

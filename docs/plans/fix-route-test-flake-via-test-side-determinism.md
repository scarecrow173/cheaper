---
id: "PLAN-034foupf7ivrjlU7zVsv6M"
type: "plan"
status: "approved"
title: "Fix route test flake via test-side determinism (issue #157)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/157"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  implemented-by:
    - "docs/tasks/make-exec-infra-failure-test-deterministic.md"
    - "docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md"
    - "docs/tasks/verify-route-test-determinism-and-mise-check.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "docs/adr/resolve-route-test-flake-via-test-side-determinism.md"
  derived-by:
    - "docs/tasks/make-exec-infra-failure-test-deterministic.md"
    - "docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md"
    - "docs/tasks/verify-route-test-determinism-and-mise-check.md"
  verifies: []
  verified-by:
    - "docs/test-specs/deterministic-route-test-assertions.md"
  references:
    - "docs/discovery/ubuntu-ci-route-test-flake.md"
    - "crates/cheaper-cli/tests/integration/route_run.rs"
    - "crates/cheaper-cli/src/route/log_store.rs"
  defers: []
  deferred-by: []
---

# Fix route test flake via test-side determinism (issue #157)

## Goal

issue #157 の 2 件の flaky route test を、ADR Option A(test 側 determinism)
に従って timing・`fs::read_dir` 列挙順への暗黙依存を排除した deterministic
な assertion へ修正する。本番実装変更はゼロで、変更面は 2 つの test のみ。

## Scope

単一 implementation stream — 同じ flake 原因分類(test が制御下にない外部
状態を観測する)に対する同一方針の修正で、critical path と release intent
が同一(1 PR で CI を安定化する)のため分割しない。

In scope:

- `crates/cheaper-cli/tests/integration/route_run.rs` —
  `exec_infra_failure_returns_error`(:599-637)の実行到達証明を
  `marker.exists()` から `Err` の `code() == ErrorCode::ToolExecutionFailure`
  + message `contains("routed command did not complete")` へ置き換える
  (design D1)。
- `crates/cheaper-cli/src/route/log_store.rs` —
  `sweep_enforces_ttl_count_size_and_skips_tmp`(:559-606)の (b) count
  sweep・(c) size sweep で対象 file に `File::set_modified` の一意 mtime
  順序を与え、assertion を厳密集合へ絞る(design D2-D4)。同一 test 内の
  `set_mtime` 操作を 1 helper に集約する(design D5・(a) block も含む)。

Out of scope(spec / design と同じ境界):

- `proc::run`/`drive`/`cleanup`/`tree` の timeout・kill 実装変更。
- `RouteLogStore::sweep_with` の semantics 変更(mtime 秒精度・stable
  sort・tie 時任意順は production で問題ない)。
- 他の route 系 test・workflow test への波及、CI 構成変更。
- 新規 crate 依存の追加(`filetime` 等)。std API のみで完結する。

## Upstream Documents

- `docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md`
  (SPEC-034KoUwVbN6YjxD7CjpYkf, approved) — 要件・Acceptance Criteria。
  拘束力のある contract。
- `docs/adr/resolve-route-test-flake-via-test-side-determinism.md`
  (ADR-034KoUAHJ1vI8BbyJoL0Q9, accepted) — Option A「test 側で
  determinism を確保」の構造決定(Option B/C 却下の根拠を含む)。

## Design Inputs

- `docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md`
  (DESIGN-034Kp4aEcwrsOReUSPyxU9, approved) — D1-D5 の決定事項が本
  plan の authoritative HOW。本 plan は design を implementation order へ
  転記したもので独自判断を足さない。
- `docs/discovery/ubuntu-ci-route-test-flake.md`
  (DISC-Gvis7cxcHxPQwsvoSGmuox, resolved) — 原因確定の検証過程(参照)。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-cli/tests/integration/route_run.rs` | modify | `exec_infra_failure_returns_error`(:599-637)の assertion 置換 + `cheaper_core::errors::{CheaperError, ErrorCode}` import 追加 | fixture 配線(`reject_fx`・policy.yaml の `CHEAPER_ROUTE_CHILD_MODE: sleep`・`timeout: '1ms'`・payload token・`marker` 変数・usage log assert)は不変 |
| `crates/cheaper-cli/src/route/log_store.rs` | modify | `sweep_enforces_ttl_count_size_and_skips_tmp`(:559-606)に `set_mtime` helper 導入 + (a)(b)(c) の mtime 明示順序付け + assertion 厳密集合化 | `#[cfg(test)] mod tests`(:384)内のみ。`use super::*` で `fs`/`File`/`Path`/`SystemTime`/`Duration`/`io` は既に import 済み |

## Implementation Sequence

task slice は T1・T2 が並行可能(file 集合が disjoint)で、T3 が両者の完了後に
全体検証を実行する。

### T1 — `exec_infra_failure_returns_error` の assertion 置換(route_run.rs)

- [ ] file 先頭の use 群に
      `use cheaper_core::errors::{CheaperError, ErrorCode};` を追加する
      (先例: `tests/integration/envelope.rs:10` が同一 import)
- [ ] `assert!(run_route(&mut f, &token).await.is_err());` と
      `marker.exists()` assert(:627-631)を design D1 の確定形へ置き換える:
      `run_route(&mut f, &token).await.err().expect("route must fail: grant timeout is 1ms")`
      で `Err` を取り出し(`.err().expect(...)` — `unwrap_err()` は
      `RouteRunOutput: Debug` を要求するため使わない)、
      `assert_eq!(err.downcast_ref::<CheaperError>().map(CheaperError::code), Some(ErrorCode::ToolExecutionFailure), ...)`
      と `assert!(err.to_string().contains("routed command did not complete"), ...)`
      を置く。`marker.exists()` 系の timing 依存 assertion は残さない
- [ ] `timeout: '1ms'`(:603)・policy.yaml 書換え(:604-613)・
      `reject_fx` 呼出し・`marker` 変数(fixture 配線として `reject_fx` が
      `&marker` を要求するため残す)・usage log の `"route"` record 非含有
      assert(:632-636)は全て維持する
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      route_run::exec_infra_failure_returns_error -- --exact` が exit 0

### T2 — `sweep_enforces_ttl_count_size_and_skips_tmp` の mtime 明示順序付け(log_store.rs)

- [ ] `#[cfg(test)] mod tests` 内に
      `fn set_mtime(path: &Path, time: SystemTime) -> io::Result<()>` を追加する
      (実体は `fs::File::options().write(true).open(path)?.set_modified(time)`。
      先例: `tests/integration/hook_eval_dedup.rs:127-132`・
      `src/hook/eval_cache.rs:982`。`use super::*` で必要な型は導入済み)
- [ ] `let now = SystemTime::now();` を test 先頭で 1 回だけ取得し、(a) block
      の inline `fs::File::options()...set_modified(past)`(:575-581)を
      `set_mtime(&old, now - Duration::from_secs(3600)).unwrap()` 系の
      helper 呼出しへ揃える(同一 test 内の同一操作を 1 表現に集約 —
      表現ドリフト防止)。TTL assertion(removed==1・fresh 残存・old 削除)
      と in-flight `.tmp` 生成は不変
- [ ] (b) count sweep: `a.log`/`b.log`/`c.log`/`fresh.log` に
      `a = now-40s`、`fresh = now-30s`、`b = now-20s`、`c = now-10s` の
      `set_mtime` を与え(`as_secs` 単位で 1 秒以上離れた一意 key)、
      `sweep_with(Duration::MAX, 2, u64::MAX)` の assertion を厳密集合へ:
      `assert_eq!(removed, 2)`、`!a.exists()`、`!fresh.exists()`、
      `b.exists()`、`c.exists()`。`b.exists() || c.exists()` の
      条件付き disjunction は廃止(design D3)
- [ ] (c) size sweep: 直前の cleanup loop
      `for path in [&a, &b, &c, &fresh] { let _ = fs::remove_file(path); }`
      (:596-598)を**維持**し(落とすと残留 file が tie を再導入する —
      design D4)、`big = now-20s`(古)、`small = now-10s`(新)の
      `set_mtime` を与えて `sweep_with(Duration::MAX, usize::MAX, 100)` の
      assertion を `assert_eq!(removed, 1)`・`!big.exists()`・
      `small.exists()` へ厳密化(`big.exists() != small.exists()` 廃止。
      逆順にすると small 先行削除でも 128B > 100 で 2 件削除となり意図が
      変わるため順序は必須)
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::log_store` が
      exit 0

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 route_run.rs assertion 置換 | — | T3 |
| T2 log_store.rs mtime 順序付け | — | T3 |
| T3 全体検証(route_run:: 全体 + `mise run check`) | T1, T2 | — |

critical path は T1/T2 → T3。T1 と T2 は file 集合が disjoint で互いの成果物を
参照しないため並行実行可能。T3 は両方の着地済み assertion が `mise run check`
内で green であることを確認する収束 step。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `marker.exists()` 除去で「子が実際に動作を開始した」行動証拠を失う | — | low | `code() == ToolExecutionFailure` + `"routed command did not complete"` message が `captured() == None && outcome.error.is_some()` 分岐固有の post-admission 証拠であり、design で代替十分と確定済み |
| 2 | `ErrorCode::Timeout` を assert したくなる誤読 — engine collapse により `Err` 側には届かず全 platform で決定的 fail | low | med | design D1 で collapse 機序(engine.rs:1172-1188)と `Timeout` assert 不採用理由を確定済み。plan/task でも code は `ToolExecutionFailure` と明記 |
| 3 | (c) block の cleanup loop を落として (b) 残留 file が mtime tie を再導入し flake が戻る | low | med | design D4 が維持必須を明示。task checklist にも「維持」を明記し reviewer が diff で確認できる |
| 4 | mtime の秒境界跨ぎで tie が再発する | low | med | `now` を 1 回取得し 10 秒刻みの整数秒差分を与えるため `as_secs` key は必ず一意(design D2)。FS timestamp granularity が粗くても 1 秒超の差は安全 |
| 5 | test 追加で wall-clock が遅くなる / 他 test への波及 | low | low | `timeout: '1ms'` 維持・`set_modified` は metadata 操作のみで sleep 無し。変更面は 2 test のみ |
| 6 | 外部依存 availability | low | low | 新規 crate・env・CI 構成変更なし。std `File::set_modified` と既存 import のみ |

### Rollback

- **Trigger**: 修正後も test が不安定、または assertion が意図しない失敗を
  示す。
- **Procedure**: revert unit は本 change の commit — `git revert` で 2 test
  file を flake 修正前へ戻す。schema 変更・migration・共有 state は無く、
  本番 code 非接触のため `git revert` で完全に戻せる。
- **Verification**: revert 後に `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration route_run::` と `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::log_store`
  が(元の非対称 pass/fail 傾向で)実行できること。

## Task Breakdown

task doc へ分割した候補(3 task — T1/T2 は並行可能、T3 は収束検証):

- T1: `exec_infra_failure_returns_error` の error code+message assertion 化
  — `docs/tasks/make-exec-infra-failure-test-deterministic.md`
- T2: `sweep_enforces_ttl_count_size_and_skips_tmp` の `set_mtime` 明示順序
  付け + assertion 厳密集合化
  — `docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md`
- T3: route_run:: 全体 + `mise run check` の最終検証
  — `docs/tasks/verify-route-test-determinism-and-mise-check.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| AC: `exec_infra_failure_returns_error` が `ToolExecutionFailure` code + `"routed command did not complete"` message assert を持ち `marker.exists()` 非依存 | 対象 test 単体実行(`--exact`) | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::exec_infra_failure_returns_error -- --exact` |
| AC: sweep test が `set_modified` 明示順序で削除数・残存 file が一意 | `route::log_store` unit test filter | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::log_store` |
| AC: `route_run` integration target 全体が pass | `route_run::` module filter | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` |
| AC: `mise run check` が pass(biome・fmt・clippy・version/doc-id・`cargo test --workspace`・`node --test`・lockfile/payload check) | canonical verification contract | `mise run check` |
| ubuntu CI で marker 未作成・readdir 順不定でも fail しない | assertion が timing/enumeration を参照しないことの review 確認(design Verification Notes 記載の代替確認) | 手動 review(diff で `marker.exists()`・readdir 順依存 assertion が無いこと) |

## Open Questions / Gaps

- design が assertion 形状・mtime 値・helper 形を確定済みのため、実装上の
  不明点は無い。
- ubuntu CI での実検証は本 change の merge 後の CI 実行で確認する —
  assertion が外部状態を参照しないことの review 確認で代替可能(design
  Verification Notes)。
- `sweep_with` の (mtime, path) secondary sort key による実装側 hardening は
  spec/design の Deferred Design Concerns で scope 外確定(YAGNI)。

## Self-Review

- [x] Every spec requirement maps to a task.(2 要件 + 全 AC → T1/T2/T3 と
      Verification matrix に対応付け済み)
- [x] No placeholders remain.(assertion 形状・mtime 値・verification
      command は design D1-D5 の確定値を転記)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation — T1・T2 は
  file 集合が disjoint な独立 slice で、T3 が収束検証の review checkpoint。
- Independent work streams: T1・T2(並行可能)。T3 は両者に依存。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in
  the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and
  verification matrix.

## Review Handoff

- 前提: upstream 3 doc(spec approved・design approved・ADR accepted)が
  assertion 形状・mtime 値・維持条件まで確定済み。plan は design を
  implementation order へ転記したもので独自判断を足していない。
- 重点確認: (a) `Err` の code assert が `ToolExecutionFailure` であり
  `Timeout` ではないこと(engine collapse 済み)、(b) (c) block 直前の
  cleanup loop が維持されていること、(c) `marker` 変数・`reject_fx` 配線が
  assert 対象から外れるだけで残っていること、(d) (b)(c) の assertion が
  厳密 file 集合(disjunction 廃止)になっていること。

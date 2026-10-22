---
id: "TSPEC-034cAvaR7bDF2BjMm2JiAA"
type: "test-spec"
status: "draft"
title: "Deterministic route test assertions (infra failure and sweep ordering)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "docs/adr/resolve-route-test-flake-via-test-side-determinism.md"
  derived-by: []
  verifies:
    - "docs/specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "docs/designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
  verified-by:
    - "docs/tasks/make-exec-infra-failure-test-deterministic.md"
    - "docs/tasks/pin-sweep-test-deletion-order-with-explicit-mtimes.md"
  references:
    - "docs/discovery/ubuntu-ci-route-test-flake.md"
  defers: []
  deferred-by: []
---

# Deterministic route test assertions (infra failure and sweep ordering)

## Purpose

issue #157 で観測された 2 件の flaky route test が、「test の assertion が
制御下にない外部状態(process spawn→子側 write の timing、`fs::read_dir`
列挙順)を観測する」という invariant 違反を二度と持ち込まないことを pin
する。本 test spec は、将来 assertion を書き換える・削除する変更が
determinism の意図を維持しているかを判定するための契約。対象の本番実装
(`proc` timeout/kill・`RouteLogStore::sweep_with` 削除順)が変わって
assertion 形状の再設計が必要になった場合、または flake 再発が別機序で
確認された場合に廃止・supersede を検討する。

## Feature

route 系 test の失敗 assertion は、timing・filesystem 列挙順に関わらず
platform 間で同じ結果を返す — 検証したい invariant(post-admission の
実行到達 error、sweep の古い順削除)を timing 非依存の観測量で直接表す。

## Rules

- `exec_infra_failure_returns_error`
  (`crates/cheaper-cli/tests/integration/route_run.rs`)は、`run_route`
  の戻り `Err` が `CheaperError` で `code() ==
  ErrorCode::ToolExecutionFailure` かつ message が
  `"routed command did not complete"` を含むことを実行到達証明として
  assert する。child 側の side effect(`marker.exists()` 等)を kill 前に
  観測する assertion を持たない。
- `ErrorCode::Timeout` は `Err` の code から検証しない — engine が job
  失敗を `ToolExecutionFailure` に collapse するため全 platform で
  決定的に fail する assertion になる。
- `sweep_enforces_ttl_count_size_and_skips_tmp`
  (`crates/cheaper-cli/src/route/log_store.rs`)は、count sweep・size
  sweep の対象 file 全てに `File::set_modified` で一意な mtime 順序
  (`as_secs` 単位で 1 秒以上離れた値、単一の `now` 起点)を与える。
- 削除・残存の assertion は、決定した順序に対応する厳密な file 集合で
  表す — `a || b` のような条件付き disjunction や existence XOR による
  曖昧な許容を残さない。
- sweep test は `.tmp` 除外と TTL sweep の検証を弱めず、block 間で残留
  file が次の sweep の削除順に影響しないよう後続 block 前に除去する。

## Examples

- grant `timeout: '1ms'` + 30s sleep の child fixture で `run_route` が
  `Err` を返す(決定的)。その `Err` から `downcast_ref::<CheaperError>()`
  で code を取り `ToolExecutionFailure`、message に
  `"routed command did not complete"` を含む → POSIX kill が marker write
  に先勝つ ubuntu CI でも pass。
- count sweep で mtime 順 a < fresh < b < c を付与し max 2 →
  削除は決定的に {a, fresh}、`removed == 2`・b/c 残存。
- size sweep で big(128B)を small(8B)より古く付与し合計上限 100B →
  古い順に 1 件(big)だけ削除、`removed == 1`・small 残存。逆順なら
  small 先行削除でも残量 128B > 100 で 2 件削除になり test 意図が変わる
  ため順序選択は assertion と連動する。

## Guarantees

- 両 test は `fs::read_dir` 列挙順・mtime granularity・spawn→子側 write
  遅延・CI 負荷に関わらず deterministic である。
- infra failure の assertion は workflow が admission を通過して exec job
  を失敗まで実行したこと(post-admission)を timing 非依存に証明する。
- sweep の assertion は「削除は mtime の古い順」という実装 invariant を
  一意な file 集合で直接検証する。

## Non-goals

- 本番実装(`proc` timeout/kill・`sweep_with` の sort・削除 semantics)の
  正しさや変更 — 本 spec は test 側 assertion の契約のみを対象とする。
- `sweep_with` の (mtime, path) tie-break 等の実装側 deterministic
  hardening(upstream の Deferred Design Concerns で scope 外)。
- literal な post-spawn 識別子(error code 経由で `Timeout` を得る等) —
  test-only では到達不能であり production 変更を要するため対象外。
- 他の route 系・workflow 系 test の determinism 監査。

## Risk

- この保証が落ちると、assertion が再び timing・列挙順を観測して ubuntu CI
  で断続失敗し、`mise run check` の verification 信号が毀損される —
  全 PR が「本物の regression か flake か」の判別コストを負う。
- 厳密集合 assert を緩めた場合(disjunction 等への回帰)、sweep の削除順
  regression が test を通り抜ける。

---
id: "SPEC-034KoUwVbN6YjxD7CjpYkf"
type: "spec"
status: "approved"
title: "Deterministic route tests for ubuntu CI flake (issue #157)"
created: "2026-10-22"
updated: "2026-10-23"
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
  implemented-by:
    - "../plans/fix-route-test-flake-via-test-side-determinism.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/ubuntu-ci-route-test-flake.md"
  derived-by:
    - "../designs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../plans/fix-route-test-flake-via-test-side-determinism.md"
    - "../test-specs/deterministic-route-test-assertions.md"
  verifies: []
  verified-by:
    - "../test-specs/deterministic-route-test-assertions.md"
  references: []
  defers: []
  deferred-by: []
---

# Deterministic route tests for ubuntu CI flake (issue #157)

## What Should Be Built

issue #157 の 2 件の flaky test を、timing・filesystem enumeration order に
依存しない deterministic な assertion へ修正する。対象は
`crates/cheaper-cli/tests/integration/route_run.rs` の
`exec_infra_failure_returns_error` と
`crates/cheaper-cli/src/route/log_store.rs` の
`sweep_enforces_ttl_count_size_and_skips_tmp`。本番実装
(`proc::drive` の timeout/kill 機構、`RouteLogStore::sweep_with` の
削除順)は正しく、変更は test 側のみで完結する。

## Why Now

`mise run check`(`cargo test`)が ubuntu CI で断続失敗し、main でも
再現している(issue #157、run 39763252995/39764859083)。ローカル
Windows では再現しない。route 系 test の追加(PR #152 系)以降、
ubuntu runner でしか発火しない前提が残っているため CI が不安定で、
他 PR の verification 信号を毀損している。

## Users and Value

この repo で作業する developer と parent agent が受益者。`mise run
check` を全 platform で deterministic に pass させることで、CI 失敗が
「本物の regression」か「flake」かを都度判別する負担を取り除く。

## Scope

### In Scope

- `exec_infra_failure_returns_error` の実行到達証明 assertion を
  deterministic にする修正。具体的には、戻り値の error が
  `ErrorCode::ToolExecutionFailure` を持つ `CheaperError` で、かつ
  message が `"routed command did not complete"` を含むことを検証する
  (この message は `route::run::finish` の
  `captured() == None && outcome.error.is_some()` 分岐に固有で、
  workflow が exec job を失敗まで実行した post-admission 証拠。
  timing 非依存。engine は job 失敗を `ToolExecutionFailure` に
  collapse するため `ErrorCode::Timeout` は `Err` の code からは
  検証できない)。`timeout: '1ms'` は維持してよい。
- `sweep_enforces_ttl_count_size_and_skips_tmp` の (b) count sweep・
  (c) size sweep で、対象 file の mtime を `set_modified` で明示的に
  順序付ける修正((a) TTL sweep が `old.log` に使っているのと同じ
  pattern)。削除・残存の assertion を、決定した順序に対応する厳密な
  file 集合へ絞る。
- 両 test が `fs::read_dir` の列挙順・mtime granularity・spawn 遅延に
  依存しないこと。

### Out of Scope

- `proc::run`/`drive`/`cleanup`/`tree` の timeout・kill 実装変更。
- `RouteLogStore::sweep_with` の削除順 semantics 変更(mtime 秒精度・
  stable sort・tie 時の任意順は production 上問題ない)。
- 他の route 系 test・workflow test への波及。
- CI 構成(runs-on・retry・OS matrix)の変更。

## Requirements

- `exec_infra_failure_returns_error`:
  - `run_route` が `Err` を返すことは維持する(child は 30s sleep、
    timeout 1ms が必ず先に発火するため deterministic)。
  - 実行到達証明は error の `CheaperError.code() ==
    ErrorCode::ToolExecutionFailure` と、message が `"routed
    command did not complete"` を含むことで行う(post-admission
    分岐証明。literal な post-spawn 識別子は test-only では error
    code 経由で到達不能)。`marker.exists()` のような「child の
    side effect が kill 前に完了した」ことへの timing 依存
    assertion は残さない。
  - usage log に `"route"` record が書かれないことの検証は維持する。
- `sweep_enforces_ttl_count_size_and_skips_tmp`:
  - count sweep・size sweep の両 block で、sweep 対象全 file に
    `File::set_modified` で一意な mtime 順序を与え、削除対象・残存
    file の assertion が `read_dir` 順に依存しないこと。
  - `.tmp` 除外・TTL sweep の既存検証を弱めない。
- 実装はテストの意図(削除は「古い順」、infra failure は
  post-admission の実行到達 error)を維持すること — timing で
  たまたま pass する形ではなく、assertion が検証したい invariant
  を直接表すこと。

## Acceptance Criteria

- [ ] `exec_infra_failure_returns_error` が error の code assertion
  (`ToolExecutionFailure`)+ `"routed command did not complete"` の
  message assertion を持ち、`marker.exists()` への依存が無い。
  ubuntu CI で `marker` 未作成でも fail しない形になっている。
- [ ] `sweep_enforces_ttl_count_size_and_skips_tmp` が count/size
  sweep で `set_modified` による明示的 mtime 順序を使い、削除数・
  残存 file の assertion が一意に決まる。
- [ ] `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  route_run` と `cargo test -p cheaper-cli log_store` が pass する。
- [ ] `mise run check` が pass する。

## Deferred Design Concerns

- `sweep_with` の tie-break を (mtime 秒, path) の secondary sort key で
  deterministic にする実装側 hardening は検討余地として残すが、
  production file 名(`<unix-ms>-<pid>-<hex>.log`)では mtime tie が
  ほぼ発生せず、修正の主原因は test 構築側にあるため本 scope では
  行わない(YAGNI)。将来 readdir 順への暗黙依存が別 test に現れた場合に
  再検討する。

## Sources and Relations

- issue #157: <https://github.com/scarecrow173/cheaper/issues/157>
- 原因確定の検証過程:
  `docs/discovery/ubuntu-ci-route-test-flake.md`
- 修正方針の選択記録:
  `docs/adr/resolve-route-test-flake-via-test-side-determinism.md`

---
id: "TASK-034KVxxS09XbljJlhLD9Ev"
type: "task"
status: "done"
title: "route record の executed:{grant,argv} split"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
    - "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
  blocks:
    - "docs/tasks/verify-route-grant-set-projection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/route-grant-set-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-grant-set-projection.md"
  references:
    - "docs/designs/route-grant-set-projection.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/discovery/route-grant-set-projection-issue-166.md"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
  defers: []
  deferred-by: []
---

# route record の executed:{grant,argv} split

## Work

plan `docs/plans/route-grant-set-projection.md`
(PLAN-034KVx51kVYoMwO8UmJ8VE)の T3。`kind:"route"` usage
record に additive `executed:{grant,argv}` を追加し、「要求された
command」(`command`/`commandHash` — 原 command identity)と
「実際に実行された exec」(captured `details.{grant,argv}` の
実測 echo)の attribution を分離する。
design(DESIGN-034KUgF5QB3WuzFG6PmVvr)§(d)/(g)/(h)/(i) の
verbatim rust・error 契約をそのまま実装する — 再解釈しない。

### 1. `usage_log.rs` — `RouteRecordInput` + `build_route_record`

- `RouteRecordInput`(:332-357)へ 2 field 追加(design §(d)
  verbatim doc comment):
  - `pub executed_grant: String` — 実測 exec grant 名、
    captured `details.grant`。byte-matched mode では
    `route.grant` と一致(O6a)。
  - `pub executed_argv: Vec<String>` — 実測 exec argv、
    captured `details.argv`。byte-matched mode では原
    `exec_args` と一致。
- `build_route_record`(:362-404)が `commandHash` 直後へ
  `executed` を挿入 — `json!({"grant": input.executed_grant.
  clone(), "argv": input.executed_argv.clone()})`。record が
  書かれる全経路で必須(R6)。`schemaVersion: 1`(:372)・
  `command`/`commandHash` の `DefaultHasher` hex 規約
  (:364-365,:377-380)は不変。
- `{grant,argv}` object の組み立ては `build_route_record` に
  留め、caller は typed 値だけを渡す(record 形状の owner は
  usage_log — 組み立てを run.rs へ漏らさない)。
- `executed.argv` は env-prefix 値を含まない(`exec_args` は
  env-prefix strip 済み — route log と同じ露出面。payload.env
  を record field に含めない構造保証は不変)。
- usage_log unit test 追加 — `executed` shape・`commandHash`
  不変・`schemaVersion:1`。

### 2. `route/run.rs` — finish() の details 抽出 + literal

- `finish()`(:440-)の `captured.details`(:465 —
  `command::execute` が `ProcIdentity.details` へ
  `("grant", name)`・`("argv", args)` を書く
  `tools/command/mod.rs:263-272`)から、既存 `detail_i64`
  accessor と同じ style で design §(d) verbatim の 2 値を
  抽出:
  - `executed_grant` — `details.get("grant").as_str()`、
    欠落・非 string は `CheaperError::new(ErrorCode::
    InternalError, "captured exec result has no executed grant")`。
  - `executed_argv` — `details.get("argv").as_array()` で全件
    string、欠落・非 array・非 string 混入は
    `InternalError("captured exec result has no executed
    argv")`。
  - `expected_input` gate が `job_id=="exec" && tool=="command"`
    の command surface details を保証済みのため、欠落は infra
    invariant 違反として `exitCode`(:467-474)と同一 contract
    で `InternalError` に倒す(record に silent absence を
    残さない)。
- `RouteRecordInput` literal(:575-588)へ `executed_grant`・
  `executed_argv` を追加するだけ — record 書込み位置
  (`record_route` :574-594)・best-effort 規約(失敗時は
  `"usage-record"` postprocess failure :595-597 + stderr
  `usage-log:` 行・exit/stdout 不変)は不変。
- finish test の `completed_exec_result`(:899-912 — `details`
  に grant/argv 既存)へ `executed` assertion を追加。

### 3. test(TSPEC の Rules に対応)

- `usage_log.rs` unit — `build_route_record` が
  `executed:{grant,argv}` を `commandHash` 直後に必須挿入・
  `commandHash`/`schemaVersion`/`route` 不変・`suppressed`
  optional field との共存。
- `route/run.rs` finish test — captured `details.{grant,argv}`
  が record `executed` へ写る・欠落 fixture が `InternalError`
  (`exitCode` 欠落と同じ分類・record 未記録)。
- `tests/integration/usage_log.rs`(:299-338 `route_record_shape`)
  — `executed:{grant,argv}` shape + `commandHash`/`schemaVersion`
  不変の assertion。
- `tests/integration/route_run.rs` — routed run の usage.jsonl
  record が `executed` を持ち byte-matched echo(`grant ==
  route.grant`・`argv == 原 exec_args`)である case。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/route-grant-set-projection.md`
  (PLAN-034KVx51kVYoMwO8UmJ8VE — T3)
- `docs/designs/route-grant-set-projection.md`
  (DESIGN-034KUgF5QB3WuzFG6PmVvr — §(d) verbatim・§(e)
  attribution anchor・§(i) error 契約が AUTHORITATIVE)
- `docs/specs/route-grant-set-projection.md`
  (SPEC-034KTMfXYD5ZlGZCRb8Paj — R6)
- `docs/test-specs/route-grant-set-projection.md`
  (TSPEC-034KVxaBMzQE5YtVOx4j3w — record split・anchor
  規約 Rules)
- `docs/tasks/split-route-viability-and-generalize-grant-projection.md`
  (TASK-034KVxxl3lkopkjHoRid7V — 前提)

## Dependencies

- `docs/tasks/split-route-viability-and-generalize-grant-projection.md`
  (T2) — `route/run.rs` の同一 file を T2(:121)と本 task
  (:465 以降・:575-588)で編集するため直列化(plan Dependency
  Graph)。実装上は T2 と compile 独立だが conflict 回避のため
  T2 着地後に着手。
- 消費側は code 変更不要: `usage_report.rs` の
  `route_bucket`(:503-538)は `kind`/`ok`/`originalTokens`/
  `returnedTokens`/`postprocessFailures` のみを読み、additive
  field は無視される(design §(d) — `cheaper usage` 集計は
  無改訂で動く)。

## Affected Files

- `crates/cheaper-cli/src/usage_log.rs`(modify)
- `crates/cheaper-cli/src/route/run.rs`(modify — finish() 抽出
  + literal + test)
- `crates/cheaper-cli/tests/integration/usage_log.rs`(modify)
- `crates/cheaper-cli/tests/integration/route_run.rs`(modify —
  record `executed` case)

## Done When

- [ ] `RouteRecordInput` が `executed_grant: String`・
      `executed_argv: Vec<String>` を持ち、`build_route_record`
      が `commandHash` 直後に `executed:{grant,argv}` を必須
      挿入する(`schemaVersion:1`・`command`/`commandHash`
      不変)
- [ ] `route/run.rs` finish() が captured `details.{grant,argv}`
      を抽出し、欠落・非 string・非 array が `InternalError`
      (`exitCode` 欠落と同一 invariant)になる
- [ ] `executed.argv` が env-prefix 値を含まず、record 書込み
      位置・best-effort 規約が不変
- [ ] `cheaper usage` の route bucket 集計が `executed` を無視
      して従来どおり動き、新旧 record が同一 usage.jsonl に
      混在できる(code 変更不要の回帰確認)
- [ ] usage_log unit・`route_record_shape` integration・finish
      test・route_run record case が存在し `cargo test -p
      cheaper-cli` が pass

## Out of Scope

- viability/projection(T2 — `run.rs:121` は T2 済み)
- docs(T4 — usage.md・parent-agent-integration への `executed`
  記述と平文 arg 露出の明記)
- `schemaVersion` bump・`commandHash` 削除・後方互換 reader・
  captured 前 exec job 失敗経路の record
- member job 自体の usage-bearing record 新設(`kind:"route"`
  record は routed exec の計測)
- `usage_report.rs` code 変更(additive field は既存 reader が
  無視)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib usage_log` exits 0 — 実測: 実装時に pass(commit f5e8d54)、`mise run check` でも 0 failed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log` exits 0(`route_record_shape` の `executed` assertion 含む) — 実測: `route_record_shape` を含む usage_log integration test 全件 pass(`mise run check` 内)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run` exits 0(record `executed` case) — 実測: route_run target 全件 pass(`mise run check` 内、env-prefix 非混入・anchor identity 保持 case 含む)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 — 実測: `mise run check` の clippy `--all-targets` が exit 0

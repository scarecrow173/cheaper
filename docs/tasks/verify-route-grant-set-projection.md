---
id: "TASK-034KVxDMCEUdI5WtH1SJxd"
type: "task"
status: "done"
title: "route grant-set projection 最終検証 — targeted tests・doc-id・audit_docs・mise run check"
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
        path: "docs/plans/route-grant-set-projection.md"
      - type: "file"
        path: "docs/tasks/add-route-grants-declaration-and-grant-set.md"
      - type: "file"
        path: "docs/tasks/split-route-viability-and-generalize-grant-projection.md"
      - type: "file"
        path: "docs/tasks/split-route-record-executed-attribution.md"
      - type: "file"
        path: "docs/tasks/document-route-grant-set-projection.md"
      - type: "file"
        path: "docs/tasks/verify-route-grant-set-projection.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
    - "docs/tasks/split-route-record-executed-attribution.md"
    - "docs/tasks/document-route-grant-set-projection.md"
  blocks: []
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
  defers: []
  deferred-by: []
---

# route grant-set projection 最終検証 — targeted tests・doc-id・audit_docs・mise run check

## Work

plan `docs/plans/route-grant-set-projection.md`
(PLAN-034KVx51kVYoMwO8UmJ8VE)の T5。T1-T4 で完了した実装が
design の Verification Notes・spec の Acceptance Criteria 通りに
成立していることを、targeted test・TSPEC Rule 対応確認・full
verification pipeline で閉じる。product code は変更しない
(検証と doc status 更新のみ — gap 検出時のみ該当 test file
へ pin 追加)。

### 1. spec Acceptance Criteria ↔ TSPEC Rule の対応確認

spec(SPEC-034KTMfXYD5ZlGZCRb8Paj)の Acceptance Criteria 各項目に
対応する test が存在し pass していることを確認する
(TSPEC-034KVxaBMzQE5YtVOx4j3w の Rules に対応 — 欠落があれば
該当 task の責務として pin 追加):

| AC | 対応 test 領域 |
|----|----------------|
| `grants: [a,b]` 宣言 route の routed workflow で member `input.name` が実行可・未宣言名拒否 | `route_run.rs` integration(member 起動・undeclared 拒否 Rule) |
| project 層 `grants` key → fail-loud route-load error・builtin/user 受理・project `grant` 単数受理 | `routes.rs` unit + `route.rs` integration(宣言層制約 Rule) |
| anchor argv 不一致 → `no-grant` 不発・原 command 直接実行 | 既存 resolve/run test(byte-matched 回帰 Rule) |
| member 未存在/project-scope/fixed → `no-grant`(新規値なし) | `resolve.rs`/`route.rs`(member ineligible fold Rule) |
| member argv pattern 外 → 実行時 `check_exec_execution` 拒否・発火は止まらない | `route_run.rs` integration(member argv 遅延検査 Rule) |
| quoted metachar(`git log -G 'a|b'`)→ `runnable` | `resolve.rs` unit + `route.rs`(argv_shell_re 除去 Rule) |
| `argv_shell_re` 定義・import 削除・grep 0 件 | §2 grep sanity |
| record `executed:{grant,argv}`・`command`/`commandHash` 不変・`schemaVersion:1`・`cheaper usage` 集計不変 | `usage_log.rs` unit + `route_record_shape` + `route_run.rs` record case(record split Rule) |
| `expected_input.name == route.grant`・`job_id != "exec"` member job・`job_id=="exec"` member 名 → `exec-input-mismatch` | `workflow_run`/`run.rs`/`route_run.rs`(anchor 規約 Rule) |
| `load_or_none`・`unsafe-command`・`stdin-unsupported` 既存分類の回帰無し | 既存 route/hook/supply test 群 |

### 2. 配管の端到端確認

- viability 分割の構造確認 — anchor 完全検査(:315-351)が
  不変で member ループがその直後にあり、member には
  `check_exec_execution`/`verify_pinned_executable`/cwd
  prepare が呼ばれていないこと(pin は検査のみで破棄)。
- 投影の不変条件 — `Runnable` を返した route で
  `project_grants(&route.grant_set())` が `{route.grant} ∪
  grants ∪ {fixed}` を残し、undeclared 名の member job が
  `resolve_exec_grant` で `PolicyViolation` になること
  (proc_invocation parity の回帰含む)。
- record provenance — `finish()` が captured
  `details.{grant,argv}` から `executed` を構成し、欠落が
  `InternalError`(`exitCode` と同一 invariant)であること。
  `executed.argv` に env-prefix 値が混入しないこと。
- grep sanity — `grep -r argv_shell_re crates/` が 0 件・
  `grep -rn "project_grant\b" crates/` が `project_grants`
  への置換で残件なし・`grep -rn "Route {" crates/` の literal
  site 全てに `grants` field がある・`TODO`・`unimplemented!`・
  stub の残存なし。
- `\0` 含 arg の到達点 — `route resolve --argv` で
  `[\0]` 含 arg が `runnable` 到達可 → `route run` spawn
  `InvalidInput` の exec job fail として pin されている
  (viability 層へ `[\0\r\n]` regex が再導入されていない
  ことをコードで確認 — O5b 不採用の帰結)。

### 3. full verification・status 更新

- `mise run check` が pass すること(biome・fmt・clippy
  `--all-targets`・version・doc-ids・`cargo test --workspace`・
  `node --test`・lockfile・apm-payload — `--features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きの
  canonical pipeline)。
- T1-T5 の task status を `done`・plan status を `completed`
  へ更新する(実測結果を各 task の `## Verification` 欄へ
  記録)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/route-grant-set-projection.md`
  (PLAN-034KVx51kVYoMwO8UmJ8VE — T5)
- `docs/designs/route-grant-set-projection.md`
  (DESIGN-034KUgF5QB3WuzFG6PmVvr — Verification Notes が
  AUTHORITATIVE)
- `docs/specs/route-grant-set-projection.md`
  (SPEC-034KTMfXYD5ZlGZCRb8Paj — Acceptance Criteria)
- `docs/test-specs/route-grant-set-projection.md`
  (TSPEC-034KVxaBMzQE5YtVOx4j3w — 全 Rules の最終確認)

## Dependencies

- `docs/tasks/split-route-record-executed-attribution.md`
  (T3) — record split が完了し workspace が compile している
  こと。
- `docs/tasks/document-route-grant-set-projection.md`(T4) —
  docs contract が完了していること。
- (T1・T2 は T3 経由で推移的に完了前提)

## Affected Files

- `docs/tasks/add-route-grants-declaration-and-grant-set.md`・
  `docs/tasks/split-route-viability-and-generalize-grant-projection.md`・
  `docs/tasks/split-route-record-executed-attribution.md`・
  `docs/tasks/document-route-grant-set-projection.md`・
  `docs/tasks/verify-route-grant-set-projection.md`(status 更新)
- `docs/plans/route-grant-set-projection.md`(status 更新)
- 観測 pin 欠落が見つかった場合のみ、該当 test file へ pin
  追加(本来は T1-T4 の責務 — 本 task で新規 test を書くのは
  gap 検出時に限る)

## Done When

- [x] spec Acceptance Criteria 全項目に対応する test が存在し
      pass(TSPEC Rule との対応表が §1 で確認済み) — `mise run check`
      の `cargo test --workspace` で 0 failed(`route::resolve_member_grants_viability`・
      `route_run::exec_job_named_member_is_input_mismatch`・
      `usage_log::route_record_shape`・
      `proc_invocation::grant_set_projection_preserves_scope_parity` 含む)
- [x] viability 分割・投影境界・record provenance の構造確認・
      `\0` 含 arg の fail 位置 pin が実測されている —
      `proc_command::metachar_args_pass_through_and_nul_degrades_to_spawn_error`
      が spawn 時 InvalidInput への degrade を pin(viability 層へ
      regex は再導入されていない)
- [x] `grep -r argv_shell_re crates/` 0 件・
      `project_grant` 残件なし・`Route` literal 全件 `grants`
      field あり・stub 残存なし — 実測済み(残存は複数形
      `project_grants` 参照 12 件のみ)
- [x] `mise run check` exits 0
- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `audit_docs` が本 issue の新規 doc で error 0
- [x] 5 task + plan の status が更新されている

## Out of Scope

- product code の追加変更(実装漏れが見つかった場合は該当
  task へ差し戻す — T5 で大規模な実装をやり直さない)
- substitute mode(#167)・docs sweep(#169)・後続分解 issue の
  作業
- `mise run check` 以外の新規検証基盤

## Verification

- [x] 上記 §1-§3 の全項目が pass(コマンド実測を記録)
- [x] `node scripts/check-doc-ids.mjs` exits 0 — 実測: "doc id check
      OK: 1156 markdown files scanned"
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で本 issue の新規 doc 7 件(plan・test-spec・task×5)が
      error 0 — 実測: severity "error" 0 件
- [x] `mise run check` exits 0 — 実測: biome → fmt --check →
      clippy `--all-targets`(dev-seam) → version → doc-ids →
      `cargo test --workspace`(dev-seam)0 failed → node --test
      (scripts 43 / benchmarks 12 pass) → lockfile → apm-payload、
      全 stage pass で exit 0

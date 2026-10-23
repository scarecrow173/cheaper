---
id: "TASK-034L2NsdLuC7G3uiTACWd3"
type: "task"
status: "done"
title: "substitute-exec route mode 最終検証 — targeted tests・doc-id・audit_docs・mise run check"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/plans/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/tasks/add-route-substitute-declaration-and-exec-pair.md"
      - type: "file"
        path: "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
      - type: "file"
        path: "docs/tasks/wire-route-run-substitute-exec-pair.md"
      - type: "file"
        path: "docs/tasks/document-substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/tasks/verify-substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/specs/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/adr/substitute-exec-route-mode.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
    - "docs/tasks/document-substitute-exec-route-mode.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/substitute-exec-route-mode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/substitute-exec-route-mode.md"
  references:
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
  defers: []
  deferred-by: []
---

# substitute-exec route mode 最終検証 — targeted tests・doc-id・audit_docs・mise run check

## Work

plan `docs/plans/substitute-exec-route-mode.md`
(PLAN-034L2MF3X78MvKvcSN8jmV)の T5。T1-T4 で完了した実装が
design の Verification Notes・spec の Acceptance Criteria
通りに成立していることを、targeted test・TSPEC Rule 対応
確認・full verification pipeline で閉じる。product code は
変更しない(検証と doc status 更新のみ — gap 検出時のみ該当
test file へ pin 追加)。

### 1. spec Acceptance Criteria ↔ TSPEC Rule の対応確認

spec(SPEC-034KXdoK2bMG3gQtIj8l6h)の Acceptance Criteria
各項目に対応する test が存在し pass していることを確認する
(TSPEC-034L2MB0m2NOWZsyuTHsjA の Rules に対応 — 欠落が
あれば該当 task の責務として pin 追加):

| AC | 対応 test 領域 |
|----|----------------|
| AC1: `substitute:{grant,argv}` schema 受理・`argv:[]` 受理・非 object/key 欠落/nested 未知 key/非 string 要素で route-load error | `routes.rs` unit(declaration schema Rule) |
| AC2: project 層 `substitute` key → fail-loud `InvalidRuntimePolicy`・builtin/user 受理・project `grant:` 単数/byte-matched 有効 | `routes.rs` unit + `route.rs` integration(宣言層制約 Rule) |
| AC3: `substitute.argv` pattern 不一致・`substitute.grant` 未存在/project-scope/fixed/pin 不能 → `no-grant`(新規値なし) | `resolve.rs`/`route.rs`(viability retarget Rule) |
| AC4: 原 argv が `route.grant` pattern 不一致でも `runnable`・`route.grant` 未存在/ineligible/pin 不能で `no-grant` | `resolve.rs` unit(retarget・member 級降格 Rule) |
| AC5: exec job が `{name:substitute.grant, args:substitute.argv}` を実行・`expected_input` 通過・異なる name/args は `exec-input-mismatch` | `route_run.rs` integration(実行 anchor Rule) |
| AC6: substitute exit(0/非 0)が `route run` exit へ mirror | `route_run.rs`(exit mirror Rule) |
| AC7: deadline・exec job lifecycle が `substitute.grant` 宣言値由来 | `run.rs` unit(deadline 選出)+ `route_run.rs`(env 到達) |
| AC8: record が `route`/`commandHash`/`executed:{substitute.grant, substitute.argv}`・`schemaVersion:1`・新規 field 無し | `route_run.rs` record case + `usage_log.rs` shape 回帰(record attribution Rule) |
| AC9: typed 側 interactive 不発維持 + substitute 側 interactive(宣言名 union・解決名 union・argv flag)で `stdin-unsupported` | `resolve.rs` unit(stdin union Rule) |
| AC10: `grants[]` member 併用・`job_id != "exec"` member job 動作・投影外名拒否 | `route_run.rs` integration(member 併用 Rule) |
| AC11: exec job 無し/`outputs.execResult` 無し workflow が substitute 有無に関わらず convention 拒否 | `route_run.rs` integration(Option E 境界 Rule) |
| AC12: `substitute` 無し route の viability/exec-input/exit mirror/record 回帰無し | 既存 `resolve.rs`/`routes.rs`/`run.rs` unit・`route.rs`/`route_run.rs`/`usage_log.rs`/`hook.rs`/`proc_invocation.rs`(cheaper-tools)test 群 — **例外は suppressed 2 assert 更新のみ** |

### 2. 配管の端到端確認

- retarget 構造確認 — `resolve.rs` の完全検査が
  `route.exec_pair(&out.exec_args)` の実行ペアを対象にし、
  `route.grant` の原 argv 照合が残っていないこと(形骸化
  検査の除去 — grep で `route.grant` を対象にした
  `check_exec_execution` 呼出しが exec_pair 経由のみである
  こと)。
- member 級降格 — member ループが `demoted.into_iter().
  chain(&route.grants)` を走査し、`route.grant`・member に
  `check_exec_execution`/`verify_pinned_executable`/cwd
  prepare が呼ばれていないこと。
- interactive union — substitute 分類が宣言名
  `command_name(&grant.executable)` と解決名
  `command_name(&pinned)` の両方へ掛かっていること。
- 投影の不変条件 — `Runnable` を返した substitute route で
  `project_grants(&route.grant_set())` が `{route.grant} ∪
  grants ∪ {substitute.grant} ∪ {fixed}` を残し、undeclared
  名の job が `resolve_exec_grant` で `PolicyViolation` に
  なること(proc_invocation parity の回帰含む)。
- 実行 anchor — `expected_input`/`deadline`/inputs
  `grant`/`execArgs` が `exec_pair` 由来、`argv`/`args`/
  `executable` が原 command identity であること。D11 gate・
  `route_context` 配線・`finish()`・`build_route_record`
  が code 不変であること(diff 確認)。
- merge suppressed — project `trusted:true` 置換で displaced
  route が `suppressed` に記録され、`route resolve` 出力・
  `kind:"route"` record の `suppressed` に乗ること。
  builtin/user 層間 shadow が silent のままであること。
- grep sanity — `grep -rn "Route {" crates/` の literal
  site 全てに `substitute` field がある・`grep -rn
  "substitute" crates/cheaper-cli/src/route/` の配置が
  design §(h) の site に一致・`TODO`・`unimplemented!`・
  stub の残存なし・`RouteResolution` に substitute field が
  追加されていないこと。
- Option E 境界 — `validate_routed_convention`(:902-945)・
  `route_cmd.rs` 出力 shape が無改訂であること(diff 確認)。

### 3. full verification・status 更新

- `mise run check` が pass すること(biome・fmt・clippy
  `--all-targets`・version・doc-ids・`cargo test --workspace`・
  `node --test`・lockfile・apm-payload — `--features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きの
  canonical pipeline)。
- T1-T5 の task status を `done`・plan status を `completed`
  へ更新する(実測結果を各 task の `## Verification` 欄へ
  記録)。spec(SPEC-034KXdoK2bMG3gQtIj8l6h)・ADR
  (ADR-034KXk8h1AtwCkf2cQrb7m)の Acceptance Criteria/
  status 追随は spec chain 側の慣行に従う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/substitute-exec-route-mode.md`
  (PLAN-034L2MF3X78MvKvcSN8jmV — T5)
- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk — Verification Notes が
  AUTHORITATIVE)
- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h — Acceptance Criteria)
- `docs/test-specs/substitute-exec-route-mode.md`
  (TSPEC-034L2MB0m2NOWZsyuTHsjA — 全 Rules の最終確認)

## Dependencies

- `docs/tasks/wire-route-run-substitute-exec-pair.md`
  (T3) — substitute e2e・suppressed provenance が完了し
  workspace が compile していること。
- `docs/tasks/document-substitute-exec-route-mode.md`
  (T4) — docs contract・comment 追随が完了していること。
- (T1・T2 は T3 経由で推移的に完了前提)

## Affected Files

- `docs/tasks/add-route-substitute-declaration-and-exec-pair.md`・
  `docs/tasks/retarget-route-viability-to-substitute-exec-pair.md`・
  `docs/tasks/wire-route-run-substitute-exec-pair.md`・
  `docs/tasks/document-substitute-exec-route-mode.md`・
  `docs/tasks/verify-substitute-exec-route-mode.md`(status 更新)
- `docs/plans/substitute-exec-route-mode.md`(status 更新)
- `docs/specs/substitute-exec-route-mode.md`・
  `docs/adr/substitute-exec-route-mode.md`(status/AC 追随 —
  spec chain 側の慣行)
- 観測 pin 欠落が見つかった場合のみ、該当 test file へ pin
  追加(本来は T1-T3 の責務 — 本 task で新規 test を書くの
  は gap 検出時に限る)

## Done When

- [x] spec Acceptance Criteria AC1-AC12 全項目に対応する
      test が存在し pass(TSPEC Rule との対応表が §1 で確認
      済み) — 実測: AC1/2 = routes.rs unit
      (`substitute_parse_and_empty_argv`・
      `substitute_malformed_is_schema_error`・
      `project_layer_substitute_is_rejected`)、AC3/4/9 =
      resolve.rs unit + `route::` integration
      (`substitute_retargets_viability_and_demotes_anchor`・
      `substitute_interactive_union`・
      `substitute_interactive_union_canonical_names`・
      `resolve_substitute_retargets_viability`・
      `resolve_project_substitute_rejected`)、AC5/6/8/10/11 =
      route_run.rs integration(`substitute_exec_pair_runs_and_mirrors_exit`・
      `substitute_exec_job_input_mismatch`・
      `substitute_grant_env_reaches_child_not_anchor`・
      `substitute_member_jobs_reach_members_and_substitute`・
      `substitute_routed_convention_still_enforced`)、AC7 =
      run.rs unit `exec_deadline_uses_selected_exec_grant_timeout`、
      AC12 = `mise run check` の `cargo test --workspace` で
      全 50 test binary 0 failed(回帰例外は suppressed 2
      assert のみ設計どおり)
- [x] retarget・member 級降格・interactive union・投影境界・
      実行 anchor・merge suppressed・Option E 境界の構造確認
      が実測されている — 実測: `check_exec_execution` が
      `route.exec_pair(&out.exec_args)` 由来ペアのみを対象
      (resolve.rs:319/354)・member ループは `demoted.chain(&route.grants)` で
      存在+eligibility+pin のみ(:366-391)・union は
      `command_name(&grant.executable)`+`command_name(&pinned)`
      (:405-406)・`project_grants(&route.grant_set())`
      (run.rs:121)・`expected_input`/deadline/inputs が
      exec_pair 由来(:377-406)・merge は builtin/user 層間
      silent のまま displaced を `suppressed` へ記録
      (routes.rs:192-218)・`route_cmd.rs` が merge-base から
      無差分(Option E 境界)
- [x] `grep -rn "Route {" crates/` の literal site 全件
      `substitute` field あり・stub 残存なし・
      `RouteResolution` に substitute field なし —
      実測: literal site 全件 substitute あり(`..route(..)`
      spread 経由含む)・`TODO`/`unimplemented!`/`todo!` 0 件・
      `RouteResolution` field 一覧に substitute なし
- [x] `mise run check` exits 0 — 実測: biome → fmt --check →
      clippy `--workspace --all-targets`(dev-seam) → version →
      doc-ids(1168 files) → `cargo test --workspace`(dev-seam)
      50 binary 全件 0 failed → node --test(scripts 43 /
      benchmarks 12 pass) → lockfile → apm-payload、全 stage
      pass で exit 0
- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `audit_docs` が本 issue の新規 doc で error 0 — 実測:
      `--type all --json` で severity "error" 0 件
      (inconsistent-reciprocal-relation warning は repo 全域の
      既存 pattern で blocking:false)
- [x] 5 task + plan(+ spec/ADR の status 追随)が更新されて
      いる — 実測: task×5 = done・plan = completed。spec =
      "approved"・ADR = "accepted" の terminal status 済みで
      追随変更不要(#166 先例どおり README index row・
      spec/ADR frontmatter は本 commit で変更しない)

## Out of Scope

- product code の追加変更(実装漏れが見つかった場合は該当
  task へ差し戻す — T5 で大規模な実装をやり直さない)
- Option E(#167 範囲外)・後続分解 issue(#168/#169)の作業
- `mise run check` 以外の新規検証基盤
- `route resolve` 出力への substitute echo(Deferred)

## Verification

- [x] 上記 §1-§3 の全項目が pass(コマンド実測を記録) — 実測:
      targeted test 全 pass(routes.rs unit 16・resolve.rs unit 7・
      run.rs unit 21・integration route:: 10・route_run 37)、
      §2 構造確認は retarget/member 降格/interactive union/投影/
      実行 anchor/merge suppressed/grep sanity/Option E 境界の
      全項目を file 実測で確認、§3 の status 更新は本 commit
- [x] `node scripts/check-doc-ids.mjs` exits 0 — 実測: "doc id
      check OK: 1168 markdown files scanned"
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json` で本 issue の新規 doc 7 件(plan・test-spec・task×5)が error 0 — 実測: `--type all --json` exit 0、severity "error" 0 件
- [x] `mise run check` exits 0 — 実測: biome → fmt --check →
      clippy `--workspace --all-targets`(dev-seam) → version →
      doc-ids → `cargo test --workspace`(dev-seam)0 failed →
      node --test(scripts 43 / benchmarks 12 pass) → lockfile →
      apm-payload、全 stage pass で exit 0

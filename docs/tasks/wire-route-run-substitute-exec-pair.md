---
id: "TASK-034L2NdmhEj1VYd7riHhtL"
type: "task"
status: "todo"
title: "route run exec_pair 配線・投影・substitute e2e"
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
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/tasks/retarget-route-viability-to-substitute-exec-pair.md"
  blocks:
    - "docs/tasks/document-substitute-exec-route-mode.md"
    - "docs/tasks/verify-substitute-exec-route-mode.md"
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
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
  defers: []
  deferred-by: []
---

# route run exec_pair 配線・投影・substitute e2e

## Work

plan `docs/plans/substitute-exec-route-mode.md`
(PLAN-034L2MF3X78MvKvcSN8jmV)の T3。`route/run.rs` の
`execute()`(:334-424)の 4 箇所を `exec_pair` 由来の実行
ペアへ切替え、`route_run.rs` integration で substitute mode
の端到端(exec job が substitute ペアを実行・exit mirror・
record `executed`・member 併用・convention 拒否)と merge
suppressed provenance の assert 更新を行う。
design(DESIGN-034L0RxGVVTOqtaUUThowk)§(c)/(d)/(e)/(h)/(i)
の verbatim・error 契約をそのまま実装する — 再解釈しない。
`RouteExecution` 本体・`finish()`・`prepare()` の構造・
`project_grants` 呼出し(:121-123 — `grant_set()` が
`substitute.grant` を含むため文面不変)は不変。

### 1. `route/run.rs` — `execute()` の実行ペア配線

design §(d) verbatim:

```rust
// 実行ペア — substitute mode は {substitute.grant, substitute.argv}、
// byte-matched は {route.grant, resolution.exec_args}(O4a)。
let (exec_grant, exec_argv) =
    prepared.route.exec_pair(&prepared.resolution.exec_args);
let grant_timeout = prepared
    .supply
    .policy
    .exec
    .grants
    .get(exec_grant)              // :361 — route.grant → exec_grant
    .map(|g| g.timeout.clone())
    .ok_or_else(|| denied("no-grant"))?;
let deadline = deadline_ms(&grant_timeout)?;
// …(policy clone・ctx・count_cap は不変)…
let route_exec = Arc::new(RouteExecution::new(
    RouteLogStore::new(&layout.dirs.user_config_dir),
    prepared.extra_env.clone(),
    prepared.supply.root.clone(),
    exec_grant,                    // :378 — route.grant → exec_grant
    exec_argv,                     // :379 — resolution.exec_args → exec_argv
    count_cap,
    policy.workflow.artifacts.max_read_bytes,
));
let run_id = route_exec.run_id().to_string();
let inputs = Map::from_iter([
    ("grant".to_string(), json!(exec_grant)),          // :385
    ("executable".to_string(), json!(prepared.resolution.executable)),
    ("argv".to_string(), json!(prepared.resolution.argv)),
    ("args".to_string(), json!(prepared.resolution.args)),
    ("execArgs".to_string(), json!(exec_argv)),        // :392
    // …残りは不変…
]);
```

- **`grant_timeout` lookup(:356-363)** — 投影済み policy
  (`prepare()` の `project_grants(&route.grant_set())` :121
  で `{route.grant} ∪ grants ∪ {substitute.grant} ∪ {fixed}`
  に限定済み)から `exec_grant` で lookup — deadline が
  「実行 grant の宣言 `timeout` + 120s」に追従(R8 —
  `deadline_ms` 自体は :325-332 で不変)。
- **`expected_input`(:176)** — `RouteExecution::new`
  (:148-182)の `json!({"name": grant, "args": exec_args})`
  構築式・signature は**不変** — caller が実行ペアを渡すため
  substitute mode では自動的に `{name: substitute.grant,
  args: substitute.argv}` になる(O4a)。
- **inputs** — `grant`/`execArgs` は実行ペア、
  `argv`/`args`/`executable` は原 command identity を維持。
  builtin `routed-exec.workflow.json`(:17 — `{name:
  $input.grant, args: $input.execArgs}`)が binding 無変更で
  両 mode に動くための契約。
- **D11 byte-match gate(`workflow_run.rs:467-484`)・
  `route_context` attach(:521)は一切不変** — 照合対象値が
  substitute ペアへ替わるだけで gate のコード・適用条件には
  触れない。
- **record・exit mirror は code 不変で auto-capture** —
  `finish()` の `details.{grant,argv}` 抽出(:480-504)と
  exitCode mirror(:467-474)・`build_route_record` は不変で、
  exec job input = substitute ペアであることがそのまま
  `executed`・`exitCode`/`ok` に記録される(O5a — 本 task
  では e2e assertion でのみ検証)。

### 2. test — run.rs unit + route_run.rs integration

- `run.rs` unit:
  - `prepare_projects_grant_set` 系 test(:733-770)に
    substitute 宣言 route の投影へ `substitute.grant` が
    含まれる case(`grant_set()` が投影名を供給 — 投影外なら
    exec job の `resolve_exec_grant` が失敗するため必須条件)。
  - deadline 選出 — `route.grant` と `substitute.grant` で
    `timeout` が異なる fixture で deadline が
    `substitute.grant` の宣言値由来になること。
- `route_run.rs` integration(substitute e2e — TSPEC Rules):
  - **exec-input 再 anchor(AC5)** — builtin
    `routed-exec.workflow.json` 無変更で exec job が
    `{name: substitute.grant, args: substitute.argv}` を実行
    (child marker + `executed` details で証跡)。`job_id
    == "exec"` で `route.grant`・原 exec_args・member 名を
    指定する job は `exec-input-mismatch`
    (`exec_job_named_member_is_input_mismatch` :594 と同型)。
  - **exit mirror(AC6)** — substitute command の exit
    code(0・非 0 — `io`/`ok` mode fixture + exit 7 case)が
    `route run` の process exit code として mirror。
  - **lifecycle anchor(AC7)** — envdump 系 fixture で
    `substitute.grant` の宣言 env が子へ届き `route.grant`
    の env は届かないこと(deadline は unit で pin)。
  - **record shape(AC8)** — `kind:"route"` record が
    `route`(宣言名)・`commandHash`(原 command)・
    `executed:{grant: substitute.grant, argv:
    substitute.argv}` を持ち `schemaVersion: 1`・新規
    field 無し。
  - **member 併用(AC10)** — `grants[]` member と併用し、
    `input.name` が member 名の `job_id != "exec"` job は
    従来どおり動作・投影外名は `PolicyViolation` で拒否
    (`member_fx` :466-503 と同型の substitute fixture)。
    `input.name = substitute.grant` の member job(`job_id
    != "exec"`)が member 規約で到達できること(design
    Verification Notes の AC gap 項目)。
  - **Option E 境界(AC11)** — exec job を持たない・
    `outputs.execResult` を持たない workflow は substitute
    宣言の有無に関わらず routed convention 違反で
    `InvalidRuntimePolicy`(`validate_routed_convention`
    :902-945 不変の確認)。
  - **merge suppressed provenance(assert 反転)** —
    `suppressed_project_provenance_reaches_route_record`
    (:1386-1456)の trusted branch は resolve 出力・record
    の `suppressed` 不在を assert していた — displaced
    記録で失敗するため、resolve 出力・record の
    `suppressed` が displaced route 名を記録するよう assert
    を反転して新 semantics を pin。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/substitute-exec-route-mode.md`
  (PLAN-034L2MF3X78MvKvcSN8jmV — T3)
- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk — §(c)/(d)/(e) verbatim・
  §(i) error 契約・Verification Notes が AUTHORITATIVE)
- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h — R5-R11)
- `docs/test-specs/substitute-exec-route-mode.md`
  (TSPEC-034L2MB0m2NOWZsyuTHsjA — 投影境界・実行 anchor・
  exit mirror・record attribution・member 併用・merge
  suppressed・Option E 境界 Rules)
- `docs/tasks/retarget-route-viability-to-substitute-exec-pair.md`
  (TASK-034L2N9r4zJdpE3jtoHivQ — 前提)

## Dependencies

- `docs/tasks/retarget-route-viability-to-substitute-exec-pair.md`
  (T2) — run.rs の `exec_pair` 配線自体は T1 成果物で
  compile するが、substitute e2e は T2 の viability
  retarget(substitute route が `Runnable` になること)が
  前提のため直列(T1 は T2 経由で推移的に完了前提)。
- 後続: T4 docs・T5 検証。

## Affected Files

- `crates/cheaper-cli/src/route/run.rs`(modify)
- `crates/cheaper-cli/tests/integration/route_run.rs`(modify)

## Done When

- [ ] `execute()` が `exec_pair` 由来の `exec_grant`/
      `exec_argv` を grant_timeout lookup・
      `RouteExecution::new`・inputs `grant`/`execArgs` に
      使い、`argv`/`args`/`executable` は原 command identity
      を維持している
- [ ] `expected_input`・`deadline` が substitute mode で
      `{substitute.grant, substitute.argv}`・`substitute.
      grant` timeout 由来になり、byte-matched では従来値と
      同値
- [ ] run.rs unit test が投影 substitute 含有・deadline
      選出を pin する
- [ ] `route_run.rs` が substitute e2e(exec pair 実行・
      `exec-input-mismatch`・exit mirror・lifecycle env・
      record `executed`・member 併用・member job 経由の
      `substitute.grant` 到達・convention 拒否)を pin する
- [ ] `suppressed_project_provenance_reaches_route_record`
      の assert が新 semantics(displaced 記録)へ反転されて
      いる
- [ ] `cargo test -p cheaper-cli` が pass

## Out of Scope

- `resolve.rs` の viability(T2)・docs/comment 追随(T4)
- `RouteExecution`・`finish()`・`prepare()`・
  `project_grants`・`workflow_run` gate・`usage_log`/
  `usage_report`・`tools/command` の構造・code 変更(全て
  不変 — `executed` は実測 echo で auto-capture)
- member job への routed-exec context 適用・workflow
  deadline の max-of-set 化(#166 規約不変)
- record schemaVersion bump・新規 record field・
  firing-gate anchor grant 明示 field

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::run` が pass(run.rs unit test)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run` が pass(route_run.rs integration)
- [ ] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0

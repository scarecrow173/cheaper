---
id: "PLAN-034FotRu2jWCjjdO4oBWJE"
type: "plan"
status: "approved"
title: "invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant を実装する"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/105"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/invocation-scoped-command-grants.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "../plans/implement-policy-gated-command-tool.md"
    - "../plans/implement-tool-operator-built-in-worker.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/invocation-scoped-command-grants.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/invocation-scoped-command-grants.md"
  references:
    - "../guides/command.md"
    - "../guides/workers.md"
    - "../guides/usage.md"
    - "../guides/configuration.md"
  defers: []
  deferred-by: []
---

# invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant を実装する Implementation Plan

## Goal

issue #105 の 3 component を実装する。(1) `cheaper run`・全 worker alias・`cheaper workflow run` が受ける repeatable `--grant <json>` flag — invoker 供給の grant を合成 `invocation` policy layer として `load_policy` の user merge 直後に `MergeMode::Replace` で畳み込み、当該 run の有効期間に限り user-policy grant と同一の runtime 挙動（deny list・argv pattern・executable pin・env/cwd 規則）を持たせる（永続化しない）。(2) tool-operator output schema への optional `grantRequests[]` field と emit 条件の prompt contract。(3) user-scope `<userConfigDir>/policy.yaml` の `command.grants` を validation 付きで管理する `cheaper policy grant add/list/remove`。`command` tool 本体の contract・trust boundary（model 入力・project policy は grant を authoring 不能）は不変とする。

## Scope

単一 implementation stream として扱う。core の invocation layer → CLI flag/plumbing → `policy grant` command → `grantRequests` contract → 検証 test → docs は同一の critical path と release intent（issue #105 の単一 feature）を共有し、独立に ship・verify できる subsystem には分割できないため、複数 plan への分割は行わない。

境界:

- In: `LoadPolicyOptions.invocation`（grants-only 制約 + collision error + error attribution 規約）、`--grant` repeatable flag（`name` 内包 JSON・64KiB 上限・非 run command の明示 reject）、`build_invocation_layer` 共有 builder、run / workflow run plumbing、`grants.invocation` span field + stderr `grant:` diagnostic、`tool-operator.output.schema.json` の `grantRequests` + worker prompt、`cheaper policy grant` command と core write 基盤（`read_policy_yaml`・`stringify_yaml`・`atomic_write` の core 移動）、関連 test 群、docs/APM。
- Out: model 出力経由・project policy 経由の grant authoring、`process.tasks` の同型機構、`--grant-file`、`usage.jsonl` record shape 拡張、`command` tool 本体の contract 変更、interactive confirm prompt、`command.grants` 以外の section 管理・project policy 対象化。

## Upstream Documents

- approved spec `docs/specs/invocation-scoped-command-grants.md`（要件・AC1–AC17 の正本、source: issue #105）。
- accepted ADR `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`（合成 trusted layer = Option A、flag 値形式・collision 意味論・workflow 適用範囲・`grantRequests` shape・`cheaper policy` UX・atomic write 基盤・audit surface・warn-and-continue 範囲の全副次決定）。

## Design Inputs

- approved design `docs/designs/invocation-scoped-command-grants.md`（DESIGN-034FntkIZGiffhdU4caSkN）。§(a)–(j) が全設計判断と file-level change map・compile-forced 波及列挙を確定済みであり、本 plan は §(h)/(i) と Verification Notes を task 分解へ投影したもの。設計判断の再検討は行わない。

## File Map

design §(h) を task 単位へ割り当てたもの。`LoadPolicyOptions.invocation`（8 literal site）・`RunWorkerDeps.invocation_grants`（6 site）・`ParsedCli` variant の exhaustive match は compile-forced propagation を持つため、全構築 site を導入 task が所有する（#101/#102 precedent）。なお `RunWorkerDeps {` の textual match は 7 件存在するが、7 件目は `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md:206` の TS interface 抜粋で compile 対象外 — literal site 数は Rust code の 6 件を指す。

| Path | Responsibility | Task |
| --- | --- | --- |
| `crates/cheaper-core/src/policy/load.rs` | `LoadPolicyOptions.invocation: Option<&Map<String, Value>>` field、user merge 直後の injection block（grants-only key 制約 → `PolicySchemaMode::Partial` validate → `command.grants` key collision 検査 → `MergeMode::Replace` merge → `policy.layer = "invocation"` debug event — record の emit site は sibling（user :60・project :110）同様に merge 呼出しの直前へ揃える。ordering 自体は cosmetic だが post-merge 配置だと log 行順が merge 結果の帰属を誤読させる）、`details.layer` 畳み込み private helper、load-order doc comment の 4-layer 更新（attribution gap の accepted limitation 明記）、unit test 4 件の literal 更新（:163/:193/:207/:222 — compile-forced）+ invocation 経路の新規 unit test。T3 で `read_policy_file` を `pub fn read_policy_yaml` として公開 | T1（layer）、T3（read_policy_yaml） |
| `crates/cheaper-core/tests/golden/policy_load.rs` + `fixtures/policy/load.json` | `LoadPolicyOptions` literal の `invocation` field（:18 — compile-forced）、harness が `opts.invocation` fixture key を読む経路追加、invocation layer case（user への追加・ordering・collision・grants 以外の key reject） | T1 |
| `crates/cheaper-core/src/yaml.rs` | `pub fn stringify_yaml` 追加 + `YamlError` の `pub` 化（`#![allow(deprecated)]` module scope は既存のまま） | T3 |
| `crates/cheaper-core/src/lib.rs` | `pub(crate) mod yaml` → `pub mod yaml`（:24）、`pub mod fs_util` 追加 | T3 |
| `crates/cheaper-core/src/fs_util.rs`（新規） | runtime から移動した `atomic_write` / `AtomicWriteOptions`（`pub`）— 複製しない | T3 |
| `crates/cheaper-runtime/src/{fs_util.rs,lib.rs}` | `fs_util.rs` 削除 + `mod fs_util`（lib.rs:35）除去 | T3 |
| `crates/cheaper-runtime/src/{auth/store.rs,catalog/persisted.rs}` | `crate::fs_util::` → `cheaper_core::fs_util::` 参照へ機械的書き換え（:263-271・:107-115） | T3 |
| `crates/cheaper-runtime/src/run/boundary.rs` | `RunWorkerDeps.invocation_grants: &'a [String]` field（:49-72） | T2 |
| `crates/cheaper-runtime/src/run/mod.rs` | `cheaper.run.tool_scope` span（:417-420）へ `grants.invocation = field::Empty` 宣言 + `tools.degraded` record（:476-482）と同じ sync 区間で宣言順 join を record | T2 |
| `crates/cheaper-runtime/tests/{run_dev_seam.rs:270,:1046, run_fixture_test.rs:116, ollama_e2e.rs:296}` | 各 `RunWorkerDeps` literal に `invocation_grants: &[]`（compile-forced） | T2 |
| `crates/cheaper-cli/src/args.rs` | T2: `STRING_OPTIONS`（:228-247、`"format"` 直後・`"log"` ブロック前）へ `"grant"`、`Values.grants: Vec<String>`、`set_string` push arm（last-wins 規約からの deliberate deviation 明記）、`RunCliArgs`（:71-86）/`WorkflowRunCliArgs`（:118-127）に `grants` field + populate（:903-912・:527-538）、64KiB bound、非 run 系 arm（usage:552-589・count-tokens:598-653・models:655-726・auth:728-827）の `!values.grants.is_empty()` reject、workflow arm allowlist（:501-510）の run 側のみ `"grant"` 追加、「非 run command は grants 非空検査を持つ」規約 comment。T3: `PolicyCliArgs`/`PolicyAction`/`ParsedCli::Policy`、format()/quiet() accessor arm（:181-204）、`policy` arm の positional 受理 + flag allowlist + `--grant` reject、`command required:` enumeration（:484） | T2（grant flag）、T3（policy parse） |
| `crates/cheaper-cli/src/grant.rs`（新規） | `build_invocation_layer(grants) -> Result<(Option<Map>, Vec<String>), CheaperError>` — JSON parse・object/name 検査・`name` strip・intra-invocation duplicate 検査・`{"version":1,"command":{"grants":{...}}}` 組立 + error attribution 規約の doc comment（load.rs と同一文言） | T2 |
| `crates/cheaper-cli/src/run.rs` | `build_invocation_layer` 呼出し → `load_policy`（:143-147）の `invocation` 接続、`grant: <names>` stderr diagnostic（`--quiet` 抑制）、非露出 warning（worker が `command` 非宣言時のみ — `tools.allow` 除外・pin 失敗は hard `POLICY_VIOLATION` のまま）、`RunWorkerDeps.invocation_grants` 配線（:234） | T2 |
| `crates/cheaper-cli/src/workflow_context.rs` | `prepare_workflow_context`（:267-299）に `invocation: Option<&Map<String, Value>>` param + `load_policy` literal（:277-281）接続 | T2 |
| `crates/cheaper-cli/src/workflow_run.rs` | `execute_run_inner`（:701）で layer 組立 + `grant:` stderr diagnostic、`Shared` へ names 保持、`WorkerJobExecutor` の `RunWorkerDeps`（:358-376）へ `invocation_grants`、`execute_interface_inner`（:665）は `None` 固定 | T2 |
| `crates/cheaper-cli/src/count.rs` | `LoadPolicyOptions` literal（:69-73）に `invocation: None`（compile-forced — T1 で field 追加時に設定） | T1 |
| `crates/cheaper-cli/src/run.rs` + `workflow_context.rs`（T1 分） | 上記 2 site の `LoadPolicyOptions` literal に `invocation: None`（compile-forced — T2 で実値へ置換） | T1 |
| `crates/cheaper-cli/src/cli.rs` | T2: HELP `Options:` に `--grant <json>`（repeatable 明記）1 行。T3: `ParsedCli::Policy` dispatch arm（utility command 規約、`record_usage` 非呼出し）、`command_name`（:450-462）に `"policy"`、HELP `Commands:` に `policy grant` 3 形態。`resolve_log_root`（:288-300）は変更不要 | T2（--grant HELP）、T3（policy dispatch） |
| `crates/cheaper-cli/src/policy.rs`（新規） | `pub async fn execute(&PolicyCliArgs, &mut CliIo, &CliHooks, f64) -> Result<CommandOut, BoxError>` — `resolve_user_config_dir`（`UserConfigDirOptions { env, homedir: None }`）で user scope 解決、add（spec parse → `read_policy_yaml` → 構造前提 pre-check → merged full-object `validate_policy_schema(Partial)` → `stringify_yaml` → `atomic_write`（`windows_remove_first: false` + POSIX `unix_mode: Some(0o600)`）→ `{grant:{name,executable}}` envelope）/ list（name 順 sort の `{grants:[{name,executable}]}`、file absent は空配列）/ remove（同 pre-check + delete + 同 full-object validation + 同 write path、absent name は `InvalidRequest`）。success envelope `result` に top-level `tokens` key を含めない。`atomic_write` は parent dir を作らない（caller 責務 — runtime `fs_util.rs:23-25` の doc 規約を core 移動後も維持）ため、`<userConfigDir>` 不在時は `create_dir_all` を呼ぶ。dir 作成・write の io error は `InternalError` + `details {file, ioError}`（`auth/store.rs::io_error` :275-283 と同一 shape） | T3 |
| `crates/cheaper-cli/src/lib.rs` | `pub mod grant;`（T2）・`pub mod policy;`（T3） | T2, T3 |
| `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json` | optional top-level `grantRequests[]`（`maxItems: 20`、item: `required:["executable"]` + `argvHint?: string[]` + `name?: string`（grant `propertyNames` と同一 pattern/maxLength）、全階層 `additionalProperties: false` — design §(e) verbatim） | T4 |
| `crates/cheaper-cli/resources/workers/tool-operator.md` | `## Output rules`（:51-）へ emit 条件条項（gated tool 欠如由来の `blocked`/`partial` のみ）+ request は実行すべき grant でなく invoker への提示・全 field は model 生成の untrusted 値である旨（英語 — worker file 言語規約） | T4 |
| `crates/cheaper-cli/tests/args.rs` | repeatable `--grant` parse table（`--grant <v>` / `--grant=<v>`・複数指定の順序保持・alias 受理）、非 run 系 command reject assert（AC17 — `usage`/`count-tokens`/`models`/`auth`/`workflow interface` は T2、`policy` arm は T3）、64KiB bound、T3: `policy grant` positional parse table + flag allowlist reject | T2, T3 |
| `crates/cheaper-cli/tests/policy.rs`（新規） | temp `CHEAPER_CONFIG_DIR` で add → list → remove round-trip、invalid spec（deny 該当・argv compile 失敗・schema 違反）の file 不変 + error envelope、`version` 欠落/≠1・非 object `command`/`command.grants` の `InvalidRuntimePolicy` + file 不変、存在しない name の remove が `InvalidRequest`、`result.tokens` 不在（AC16）、usage.jsonl 非記録（AC9）、POSIX `0600` write-mode。本 target は dev-seam 非 gate（`required-features` 無し — fake-stream seam は dev-seam feature 有効時のみ compile されるため model session を走らせられない）の utility-command test に限定し、`policy grant add/remove` 後の flag 無し run 挙動（AC6/AC7 の行為側半分）は `run_dev_seam.rs` 側が所有する | T3 |
| `crates/cheaper-cli/tests/common/mod.rs` | `Fixture::run`（:168-187）の multi-call 対応 — 現状は `out.0`/`err.0` を clear せず蓄積するため 2 回目の呼出しが連結 envelope で `serde_json::from_str` panic する。`run()` 冒頭で `self.out.0`/`self.err.0` を clear する（clear-before 採用 — caller が call ごとの stderr assert を行うため read 後 clear より自然）。`PipeStdin::read_all` が `mem::take` で stdin を消費するため、2 回目以降の呼出し前に caller が `f.stdin.data` を再装填する必要がある旨を doc comment に明記。`run_fake_stream_policy_grant_add_then_run_executes` 等の同一 Fixture 連続 `f.run` が前提とする | T5 |
| `crates/cheaper-cli/tests/run_dev_seam.rs` | AC1/AC2（grant 無し縮退 → `--grant` re-run で実 spawn → flag 無し再実行で再縮退 + policy file 不変）、AC3（deny executable が `InvalidRuntimePolicy` + `details.layer="invocation"`）、AC4/AC13（collision・intra-invocation duplicate が `InvalidRequest`）、AC5（`grantRequests` を含む blocked 出力の envelope `result` parse + schema 適合）、AC8（stderr `grant:` 行・`--quiet` 抑制）、AC10（`tool-operate` alias）、AC15（複数 grant 同時注入）、AC6/AC7 の行為側半分（`run_fake_stream_policy_grant_add_then_run_executes` — `f.run(&["policy","grant","add","self","<json>"])` 後に flag 無し opt-in run で marker spawn、`run_fake_stream_policy_grant_remove_then_run_degrades` — `remove` 後の flag 無し run で再縮退。`tests/policy.rs` は dev-seam 非 gate target で model session を持てないため本 file が所有 — `tests/common/mod.rs` の multi-call 修正が前提）、非露出 warn（`command` 非宣言 worker → warning + 続行、`tools.allow` 除外は `POLICY_VIOLATION` fail） | T5 |
| `crates/cheaper-cli/tests/workflow_dev_seam.rs` | AC14 — `workflow run --grant` が `prepare_workflow_context` 経路で同一挙動（`kind: tool` job が `inner.command` 再注入経由で grant を受ける）。`workflow_run_grant_reaches_tool_job` は実 spawn を伴うため file 内に child-process fixture fn が必要 — `workflow_dev_seam` は独立 test binary で `tests/workflow.rs` の `workflow_command_fixture_child` は `current_exe` 解決上再利用不可。`FIXTURE_COMMAND_CALL` analog の `workflow_dev_seam_command_fixture_child`（`current_exe` + `[<fn-name>,--exact,--nocapture]` self-reshim、`CHEAPER_PROC_CHILD_MODE=probe` で marker 書込み）を本 file に追加する | T5 |
| `crates/cheaper-cli/tests/observability_dev_seam.rs` | `grants.invocation` span field の確定 assert — `CHEAPER_LOG=debug` subprocess（`spawn_logged_run` 経路）+ `.cheaper/logs` JSON file sink の span record parse で `cheaper.run.tool_scope` の `grants.invocation` field を検証。`tools.degraded` と同じ sync 区間 record のため deterministic に assert 可能 — 任意検証ではなく必須 assert（in-process Fixture 経路は global subscriber を install できないため subprocess 経路で担保） | T5 |
| `crates/cheaper-cli/tests/regression/{cases,expected}/` | `extraArgs` に `--grant`（case schema は camelCase — `harness.rs` の `rename_all = "camelCase"` で serde field は `extraArgs`。snake_case `extra_args` は silent ignore され false-green hazard）、`config.files` で user policy seeding する case + expected（`CHEAPER_REGRESSION_CAPTURE=1` で生成し diff review）。static corpus は positive spawn を表現不能（grant executable は絶対・非 workspace path 必須で、bare name は deny set 該当または toolchain 非 deterministic — `current_exe` self-reshim は case JSON に書けない）ため、case は policy-layer 結果のみを固定する: grant 受理・collision・deny reject + error envelope の `details.layer`、`config.files` seeded user policy、`compare_stderr` opt-in による stderr `grant:` 行。positive spawn assert は dev-seam test のみ | T5 |
| `crates/cheaper-tools/tests/proc_invocation.rs` | parity test 群が変更無しで green であること（検証のみ、編集なし） | T5 |
| `docs/guides/{command,workers,usage,configuration}.md` + `README.md` | design §(j) の 4 項目（`--grant` contract・grantRequests → approval → re-run loop・`policy grant` UX + write caveat・cmdline 露出 warning）+ README commands 表の `policy` 行 | T6 |
| `packages/apm/.apm/skills/{cheaper-usage,cheaper-tool-operate}/SKILL.md` | grant-request → user approval → `--grant` re-run loop を loop の必須手順（request 由来 spec の review 必須）として文書化、`--grant`/`policy grant` の使用面 | T6 |
| `docs/impl/ir/`（新規 1 file） | 実装完了時の IMPL record（task-doc → impl-doc の既存規約に従い scaffold で生成） | T6 |

## Implementation Sequence

- [ ] T1: core `load_policy` に合成 `invocation` layer（`LoadPolicyOptions.invocation` + grants-only key 制約 + Partial validate + collision 検査 + `MergeMode::Replace` merge + `policy.layer = "invocation"` + `details.layer` helper + doc comment）を実装し、`invocation` field が強制する **全 8 literal site**（run.rs:143・workflow_context.rs:277・count.rs:69・load.rs unit test :163/:193/:207/:222・golden `policy_load.rs:18`）を `invocation: None` で同梱更新する。golden harness の `opts.invocation` fixture key 対応と `policy/load.json` の invocation case 追加、load.rs unit test を整備し `cargo build --workspace --all-targets` を green に保つ。
- [ ] T2: CLI `--grant` 表面（args.rs + grant.rs builder + run.rs/workflow plumbing + `RunWorkerDeps.invocation_grants` + `grants.invocation` span field + stderr diagnostics）を実装し、`RunWorkerDeps` field が強制する **全 6 literal site** を同梱更新する。args parse test を整備する。
- [ ] T3: `cheaper policy grant {add,list,remove}` — core write 基盤（`read_policy_yaml`・`stringify_yaml`・`atomic_write` core 移動 + runtime caller 更新）+ args parse + cli dispatch + `src/policy.rs` + `tests/policy.rs`。
- [ ] T4: `grantRequests` output contract — `tool-operator.output.schema.json` + `tool-operator.md` prompt contract。
- [ ] T5: dev-seam / regression / parity の結合検証 test を整備し AC1–AC5・AC6/AC7（行為側半分）・AC8・AC10・AC13–AC15 を網羅する（`tests/common/mod.rs` の `Fixture::run` multi-call 修正を先行）。
- [ ] T6: 公開面 docs（guides・README・APM SKILL × 2）と impl record を整備する。
- [ ] 全 task の green を確認した上で `mise run check`（canonical verification）を実行する。

## Dependencies and Risks

### Dependency Graph

| Task | Depends on | Blocks |
| --- | --- | --- |
| T1 `invocation-policy-layer`（compile-forced propagation 所有） | — | T2, T3, T5 |
| T2 `grant-flag-and-plumbing`（`RunWorkerDeps` propagation 所有） | T1（`LoadPolicyOptions.invocation` field + merge 意味論） | T5 |
| T3 `policy-grant-command` | T1（`load.rs` 共有 file — `read_policy_yaml` 公開は layer 実装と同 file） | T5 |
| T4 `grant-requests-contract` | —（schema/worker file は他 task と file 非交差） | T5（AC5 の schema 適合検証） |
| T5 `grant-e2e-verification` | T1, T2, T3, T4 | T6（docs が記述する挙動の確定） |
| T6 `grant-docs-and-apm` | T5 | — |

- critical path: T1 → T2 → T5 → T6。T3 は T1 以降 T5 までの任意位置、T4 は完全独立（file 非交差）だが逐次実行を既定とする。
- **T1 は compile-forced propagation を所有する**: `LoadPolicyOptions` への必須 field 追加は全 literal 構築点を compile error にする。**選択: `Option<&Map>` 型では churn を回避できない** — Rust の struct literal は全 field の明記を要求し、`Option` 型でも `invocation: None` の 8 site 更新は必須である（`..Default::default()` は既存 literal 様式との drift を生み、借用 field を持つ struct への部分 default パターンを新設する不利益が大きい）。したがって field は load.rs を触る最初の task（T1）に全 literal 更新とともに着地させ、**T1 完了時点で `cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が green** でなければならない。#101 の教訓どおり、compile-forced propagation は導入 task に同梱する（task は scope 境界であり commit 境界ではない）。
- T2 も同型の所有を持つ: `RunWorkerDeps.invocation_grants`（`pub` field — `Option` ではなく `&'a [String]`）の 6 literal site（cli `run.rs:234`・`workflow_run.rs:358`・runtime tests `run_dev_seam.rs:270`/:1046・`run_fixture_test.rs:116`・`ollama_e2e.rs:296`）を T2 が同梱更新し、T2 完了時点でも workspace green を維持する。
- **`ParsedCli::Policy` variant は T3 に集約する**: variant 追加は `format()`/`quiet()`/`command_name`/dispatch の exhaustive match を全て compile-forced にする。T2 で variant だけ追加すると dispatch arm が即座に必要になり未実装 stub を生むため、`policy` command の parse 表面（`PolicyCliArgs`/`PolicyAction`/accessor/enumeration/policy arm の `--grant` reject）と dispatch を T3 で一体実装する。T2 時点の `cheaper policy ... --grant` は `unknown command: policy` の `InvalidRequest` で fail するため silent drop は発生しない。

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | compile-forced propagation の波及 breadth — `LoadPolicyOptions`（8 site）・`RunWorkerDeps`（6 site）・`ParsedCli` exhaustive match で workspace が一時 build red になる | high（発生確定） | med | T1/T2 が全 literal site を同梱所有し dev-seam feature 付き `--all-targets` build を各 task の exit criteria にする（dev-seam 無しでは seam 依存 test target が検証から抜け落ちる — ADR-034DyGyITd0YaATg8fkld7）。漏れは compile error で検出されるため silent miss はない |
| 2 | `name` strip 忘れ — flag 値の `name` key を残したまま schema へ流すと grant spec の `additionalProperties: false` で常に失敗 | med | high | `build_invocation_layer` の unit test（T2）で `name` を含む flag 値が layer の `command.grants.<name>` に正しく畳まれること・`name` key が spec 側へ残らないことを固定 |
| 3 | `Vec<String>` forgotten-rejection hazard — 将来の command arm 追加で `!values.grants.is_empty()` 検査を書き忘れると `--grant` が silent drop する | med | med | args.rs に「非 run command は grants 非空検査を持つ」規約 comment を明記し、現行 command は per-command assert（tests/args.rs、AC17）で固定 |
| 4 | `grants.invocation` span field の検証コスト — in-process tracing-capture harness は存在しない | —（resolved） | low | `observability_dev_seam.rs` の既存経路（`CHEAPER_LOG=debug` subprocess + `.cheaper/logs` JSON file sink の span record parse、:291-342）で `cheaper.run.tool_scope` span の `grants.invocation` field を deterministic に assert 可能と review で確認（`tools.degraded` record と同じ sync 区間 — span close/new event の fields に出る）。stderr `grant:` 行の dev-seam assert と併せて確定検証とし、任意検証 hedge は解消した |
| 5 | golden / regression fixture churn — `policy_load` fixture 拡張と regression `expected` 生成で既存出力が意図せず変わる | med | low | `RuntimePolicy`・usage.jsonl record・envelope `meta` は shape 不変（design §(i)）。`policy/load.json` への case 追加は既存 case の `expected` を変えない。regression expected は `CHEAPER_REGRESSION_CAPTURE=1` 生成後に diff review |
| 6 | `policy grant` の write path 欠陥 — read-modify-write の lost-update・serde_yml rewrite による comment/整形喪失 | med | med | atomic write（`windows_remove_first: false` + POSIX `0600`）で torn write を防止し、last-wins・comment 非保存は accepted caveat として docs に明記（T6）。構造前提 pre-check（非 object `command`/`grants` は `InvalidRuntimePolicy`）で silent 畳み替えを防ぐ |
| 7 | `grantRequests` が advisory である取り違い — model 生成値を無検証で `--grant` へ流す親実装を助長 | med | med | worker prompt（T4）に untrusted 明記、docs/SKILL（T6）に「request 由来 spec を re-run 前に必ず review」を loop 必須手順として固定。deny list / argv compile が re-run 時に再検査するため fail-closed は保たれる |
| 8 | attribution gap の誤用 — merged map でのみ発火する検査（`grants` 合計 `maxProperties` 超過等）は `details.layer` 無しの `InvalidRuntimePolicy` となる | —（accepted limitation） | low | design §(a) の accepted limitation を `load.rs` doc comment に明記し、単一 grant の規則違反が standalone validate で `layer` 付き捕捉されることを unit test（T1）で固定 |

### Rollback Strategy

- 本 plan は永続 format 移行・破壊的 API 変更・依存追加を含まない。`LoadPolicyOptions.invocation`・`RunWorkerDeps.invocation_grants` は additive field、`grantRequests` は optional field、`cheaper policy` は新規 command であり、既存挙動への不可逆変更はない（`--grant` 無しの既存経路は `invocation: None` で差分なし）。
- **Trigger**: T1 で既存 `policy_load`/`policy_restrict`/`policy_schema` golden が regression、T2 で非 `--grant` run の既存 dev-seam suite が regression、T3 の `fs_util` 移動で credentials / catalog persisted write path が regression、または T5 の fixture test が設計意図と異なる失敗を示す場合。
- **Procedure**: task 単位の coherent commit を `git revert`。T3 の `fs_util` core 移動を revert する場合は core file 削除 + runtime 側 `mod fs_util` 復元 + caller 2 site の import 復元を一体で戻す（半移動状態を残さない）。`grantRequests`（T4）と `policy` command（T3）は flag/plumbing（T2）から独立して revert 可能。T1/T2 の revert は上位全 task（T3-T5）を巻き込むため fix-forward を優先する。
- **Verification**: revert 後に `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で workspace 全体が compile し（`LoadPolicyOptions`/`RunWorkerDeps` literal は runtime・cli・test target にも存在するため core だけの検証では不十分）、`mise exec -- cargo test -p cheaper-core -p cheaper-cli -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が pass すること（`fs_util`/`atomic_write` の移動 caller は `cheaper-runtime` 側 `auth/store.rs`・`catalog/persisted.rs` にあるため runtime test を含めないと T3 の半移動 revert を検出できない）。

## Task Breakdown

各 task は coherent な検証可能単位。task-doc 生成時は `depends-on` relation で下記 DAG を表現する（test-spec は plan approved 後に `test-spec-doc` で作成され、各 task doc の `verified-by` へ記録する — planning-flow 手順）。

- T1 `invocation-policy-layer`: `load.rs` に `LoadPolicyOptions.invocation: Option<&'a Map<String, Value>>` を追加し、user merge 直後（:67-70 の gap）に grants-only key 制約（top-level ⊆ `{version, command}`、`command` ⊆ `{grants}` — 違反は `InvalidRequest` + `details.layer="invocation"`）→ `validate_policy_schema(Partial)` → `command.grants` key collision 検査（`InvalidRequest` + `details.grant` + `details.layer` — validate 通過後に行い malformed spec が collision error を装う順序を避ける）→ `merge_policy_layers(Replace)` → `tracing::debug!(policy.layer = "invocation", ...)` の injection block を実装。shared validation 由来の失敗へ `details.layer="invocation"` を畳み込む private helper（`e.details().cloned().unwrap_or_default()` + insert + `.with_details`）を追加し、load-order doc comment を 4-layer（builtin → user → invocation → typed re-resolve → project）へ更新（attribution gap の accepted limitation 明記）。**加えて compile-forced propagation を所有し、全 8 literal site（run.rs:143・workflow_context.rs:277・count.rs:69・load.rs unit test :163/:193/:207/:222・golden `policy_load.rs:18`）に `invocation: None` を同梱追加する** — `Option` 型は literal 更新を回避できないため field は T1 に着地させる（`..Default::default()` 採用はしない — 借用 field struct への部分 default パターン導入は既存様式との drift）。golden harness の `opts.invocation` fixture key 対応 + `policy/load.json` に invocation layer case（user grant への追加・project より優先の ordering・collision error・grants 以外の key reject）を追加し、load.rs unit test に invocation 経路の固定 test（merge・key 制約 reject・collision・deny list 該当 spec が `InvalidRuntimePolicy` + `details.layer` で inner code 保持）を追加。**T1 完了時点で `cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が green**。depends-on: なし。
- T2 `grant-flag-and-plumbing`: args.rs に repeatable `--grant`（`STRING_OPTIONS` の `"format"` 直後・`"log"` ブロック前に `"grant"`、`Values.grants: Vec<String>` + `set_string` push arm — last-wins 規約からの deliberate deviation を comment 明記）・`RunCliArgs`/`WorkflowRunCliArgs` の `grants` field + populate・64KiB bound（populate 段で enforce — 非 run command では reject が先に発火）・非 run 系 arm（usage/count-tokens/models/auth）の `!values.grants.is_empty()` reject + workflow arm allowlist の run 側のみ `"grant"` を実装。新規 `src/grant.rs` に `build_invocation_layer`（`serde_json::from_str` → object/`name` 検査（`^[a-z][a-z0-9_-]{0,63}$`）→ `name` strip → intra-invocation duplicate 検査 → `{"version":1,"command":{"grants":{...}}}` + 宣言順 names、構造的失敗は `InvalidRequest` + `details.layer`）を実装し `lib.rs` に `pub mod grant;`。`run.rs` で layer 組立 → `load_policy` へ `invocation` 接続 + `grant: <names>` stderr diagnostic（`--quiet` 抑制）+ 非露出 warning（worker が `command` 非宣言時のみ — `tools.allow` 除外・pin 失敗は hard `POLICY_VIOLATION` のまま）。`workflow_context.rs` に `invocation` param、`workflow_run.rs` で `execute_run_inner` の layer 組立・`Shared` へ names 保持・`WorkerJobExecutor` の `invocation_grants`・stderr diagnostic、`execute_interface_inner` は `None` 固定。runtime 側は `RunWorkerDeps.invocation_grants: &'a [String]` + `cheaper.run.tool_scope` span の `grants.invocation` field（`tools.degraded` と同じ sync 区間 record）。**`RunWorkerDeps` の compile-forced propagation（6 literal site）を同梱所有し、T2 完了時点で workspace build green**。cli.rs HELP の `Options:` に `--grant <json>`（repeatable 明記）を追加。`tests/args.rs` に parse table（両形・順序保持・alias 受理・非 run reject・64KiB）と `src/grant.rs` unit test（`--lib`）を追加。depends-on: T1。
- T3 `policy-grant-command`: core write 基盤 — `read_policy_file` を `pub fn read_policy_yaml` として公開（`LayerKind`/`read_yaml_layer` は `pub(crate)` のまま）、`yaml.rs` に `pub fn stringify_yaml` + `YamlError` の `pub` 化 + `lib.rs` の `pub mod yaml` 昇格、`fs_util.rs`（`atomic_write`/`AtomicWriteOptions`）を runtime → core へ移動し `pub` 化 + runtime caller（`auth/store.rs:263-271`・`catalog/persisted.rs:107-115`）を `cheaper_core::fs_util::` 参照へ更新 + runtime `mod fs_util` 除去。args.rs に `PolicyCliArgs`/`PolicyAction`（`GrantAdd{name,spec}`/`GrantList`/`GrantRemove{name}`）/`ParsedCli::Policy` + format()/quiet() accessor arm + `policy` arm parse（positional 4/2/3 受理・grant name `^[a-z][a-z0-9_-]{0,63}$` 検査・utility flag allowlist 方式で `--grant`/`--worker-file`/`--cwd`/`--input`/`--timeout` 等を reject）+ `command required:` enumeration へ `policy`。`cli.rs` に dispatch arm（`cheaper.cli.command` span instrument・`record_usage` 非呼出し）+ `command_name` + HELP `Commands:` の `policy grant` 3 形態。新規 `src/policy.rs` に utility command 規約の `execute`（`resolve_user_config_dir` 直接呼出し、add: spec parse → `read_policy_yaml`（absent は `{"version":1}` base）→ 構造前提 pre-check → merged full-object `validate_policy_schema(Partial)` → `stringify_yaml` → `atomic_write`（`windows_remove_first: false` + POSIX `unix_mode: Some(0o600)` — parent dir の `create_dir_all` は caller 責務のため `<userConfigDir>` 不在時は先に作成、dir/write の io error は `InternalError` + `details {file, ioError}`）→ `{grant:{name,executable}}` envelope；list: name 順 sort、absent file は空配列；remove: 同 pre-check + delete + 同 full-object validation + 同 write path、absent name は `InvalidRequest`）。success envelope `result` に top-level `tokens` key を載せない（AC16）。`tests/policy.rs`（新規）に temp `CHEAPER_CONFIG_DIR` round-trip・invalid spec file 不変・POSIX `0600`・usage.jsonl 非記録（AC9）を、`tests/args.rs` に `policy` arm の parse table + `--grant` reject assert（AC17 の `policy` 分）を追加。depends-on: T1。
- T4 `grant-requests-contract`: `tool-operator.output.schema.json` に optional top-level `grantRequests`（design §(e) verbatim — `maxItems: 20`、item `required:["executable"]` + `argvHint?: string[]`（items maxLength 4096・maxItems 256）+ `name?: string`（`pattern: ^[a-z][a-z0-9_-]{0,63}$` + `maxLength: 64`）、`additionalProperties: false`）を追加し strict-mode 適合を維持。`tool-operator.md` の `## Output rules` に emit 条件条項（gated tool 欠如由来の `blocked`/`partial` のみ populate、entry は `executable` 必須 + `argvHint`/`name` optional、request は grant の実行ではなく invoker への提示、全 field は model 生成・prompt-injectable な untrusted 値）を英語で追加。depends-on: なし（file 非交差）。
- T5 `grant-e2e-verification`: まず `tests/common/mod.rs` の `Fixture::run` を multi-call 対応に修正する（`run()` 冒頭で `out.0`/`err.0` を clear — 蓄積のままでは 2 回目の呼出しが連結 envelope で panic。stdin は `PipeStdin::read_all` の `mem::take` で消費されるため caller が `f.stdin.data` を再装填する旨を doc comment に明記 — `policy grant` → flag 無し run の連続 `f.run` が前提とする）。`run_dev_seam.rs` に AC1/AC2（degrade → `--grant` re-run で実 spawn（`command_user_policy`/`FIXTURE_COMMAND_CALL` パターンの flag JSON 版）→ flag 無し再実行で再縮退 + policy file 不変）、AC3、AC4/AC13、AC5（`grantRequests` 含む blocked 出力の schema 適合 + envelope `result` parse）、AC6/AC7 の行為側半分（`run_fake_stream_policy_grant_add_then_run_executes`・`run_fake_stream_policy_grant_remove_then_run_degrades` — `tests/policy.rs` は dev-seam 非 gate target で model session を持てないため本 file が所有）、AC8（stderr `grant:` 行・`--quiet` 抑制）、AC10、AC15、非露出 warn + `tools.allow` 除外 `POLICY_VIOLATION` の fixture test を追加。`workflow_dev_seam.rs` に AC14（`workflow run --grant` 同一挙動・`kind: tool` job の `inner.command` 再注入経由の grant 受領 — `workflow_run_grant_reaches_tool_job` 用に `workflow_dev_seam_command_fixture_child` self-reshim を file 内に追加、`tests/workflow.rs` の同名 helper は別 test binary のため不可再利用）。`observability_dev_seam.rs` に `grants.invocation` span field の file-sink assert（`CHEAPER_LOG=debug` subprocess + `.cheaper/logs` JSON parse — `tools.degraded` と同じ sync 区間 record のため deterministic）。regression corpus に `extraArgs` + `config.files` seeding case を追加し expected を `CHEAPER_REGRESSION_CAPTURE=1` で生成 + diff review — static corpus は positive spawn を表現不能（executable は絶対・非 workspace path 必須）のため policy-layer 結果（受理・collision・deny + `details.layer`・`compare_stderr` の `grant:` 行）のみを固定し、positive spawn assert は dev-seam test のみとする。`proc_invocation` parity test が変更無しで green であることを確認。depends-on: T1, T2, T3, T4。
- T6 `grant-docs-and-apm`: `docs/guides/command.md`（`--grant` contract・cmdline 露出 warning・trust boundary）・`workers.md`（`grantRequests` output contract）・`usage.md`（`--grant`・`policy grant`・audit surface）・`configuration.md`（必要最小限）・`README.md` commands 表へ `policy` 行、`packages/apm/.apm/skills/cheaper-usage/SKILL.md`・`cheaper-tool-operate/SKILL.md` に grant-request → approval → `--grant` re-run loop（request 由来 spec の review 必須・untrusted 明記）と `policy grant` UX・write caveat（last-wins・comment 非保存）を文書化（AC11）。impl record（`docs/impl/ir/`）を生成。depends-on: T5。

## Verification

canonical verification は `mise run check`（`biome check` → `cargo fmt --check` → `cargo clippy` → version/doc-id check → `cargo test --workspace` → `node --test` → lockfile check → payload check）。cargo 系は `mise exec --` 経由で `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付き。

| Task | Verification command or criteria | Pass condition | AC |
| --- | --- | --- | --- |
| T1 | `mise exec -- cargo test -p cheaper-core`（load.rs unit test + `--test golden` の `policy_load`/`policy_schema`/`policy_restrict`/`policy_merge` を含む）+ `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | invocation merge・key 制約 reject・collision・deny spec の `InvalidRuntimePolicy` + `details.layer="invocation"` の unit test と golden の invocation case が pass。**workspace 全体（runtime・cli・test target・dev-seam target 含む）が compile green** — compile-forced propagation 完遂の証明 | AC3・AC4 の core 層固定 |
| T2 | `mise exec -- cargo test -p cheaper-cli --test args` + `mise exec -- cargo test -p cheaper-cli --lib`（`build_invocation_layer` unit test）+ `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam` + `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | repeatable `--grant` の両形・順序保持・alias 受理・非 run 系 reject（`policy` 以外 — T2 時点は `unknown command` で `InvalidRequest`）・64KiB reject の parse table と builder unit test（name strip・duplicate・malformed）が pass。`RunWorkerDeps` propagation 完遂で workspace green | AC15（parse 面）・AC17（`policy` 以外の arm） |
| T3 | `mise exec -- cargo test -p cheaper-cli --test policy` + `mise exec -- cargo test -p cheaper-cli --test args` + `mise exec -- cargo test -p cheaper-core` + `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam`（`fs_util`/`atomic_write` の移動 caller は `cheaper-runtime` 側 `auth/store.rs`・`catalog/persisted.rs` — core/cli の test だけでは半移動を検出できないため必須）+ `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | `policy grant` add/list/remove round-trip・invalid spec file 不変・`InvalidRuntimePolicy`/`InvalidRequest` 分類・`result.tokens` 不在・usage.jsonl 非記録・POSIX `0600` の test が pass。`fs_util` 移動後の runtime caller（credentials・persisted catalog）test が green。`policy` arm の `--grant` reject assert 追加で AC17 完結 | AC6・AC7・AC9・AC16・AC17 |
| T4 | `mise exec -- cargo test -p cheaper-cli --test builtin_workers` | `all_builtin_workers_resolve_with_schemas` が tool-operator の output schema（`grantRequests` 含む）を strict-mode で compile して pass — schema 自体の適合は本 task、emit 結果の検証は T5 | AC5 の schema 側 |
| T5 | `mise exec -- cargo test -p cheaper-cli --test run_dev_seam --features cheaper-cli/dev-seam` + `mise exec -- cargo test -p cheaper-cli --test workflow_dev_seam --features cheaper-cli/dev-seam` + `mise exec -- cargo test -p cheaper-cli --test observability_dev_seam --features cheaper-cli/dev-seam`（`grants.invocation` span field の file-sink assert）+ `mise exec -- cargo test -p cheaper-cli --test regression --features cheaper-cli/dev-seam`（`regression` target は `required-features = ["dev-seam"]` で gate 済みのため flag 必須。expected 生成は `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression`）+ `mise exec -- cargo test -p cheaper-tools --test proc_invocation` + `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | `--grant` re-run で degrade 解消 + 実 spawn・flag 無し再縮退 + policy 不変・deny/collision/duplicate reject・`grantRequests` の schema 適合 + envelope parse・`policy grant add/remove` 後の flag 無し run（同一 `Fixture` の連続 `f.run` — `tests/common/mod.rs` の multi-call 修正が前提）・stderr `grant:` 行・`grants.invocation` span field（observability file sink）・alias parity・workflow run parity（`kind: tool` job 用の self-reshim fixture 含む）・複数 grant 注入・非露出 warn + `POLICY_VIOLATION` 区別・regression case（policy-layer 結果のみ — positive spawn は dev-seam のみ）・proc_invocation parity 無変更 green が全て pass | AC1・AC2・AC3・AC4・AC5・AC6・AC7・AC8・AC10・AC13・AC14・AC15 |
| T6 | doc audit + `--grant`/`policy grant`/`grantRequests` loop の記述と実装挙動の突合 + impl record 生成 | design §(j) の 4 項目が guides・README・両 SKILL.md に記載され、cmdline 露出 warning・untrusted 明記・write caveat を含む | AC11 |
| 全 task | `mise run check` | pipeline 全段が exit 0 | AC12 |

design Verification Notes の未チェック項目は各 task の checklist にそのまま引き継ぐ（invocation golden case → T1、`grants.invocation` span field の file-sink assert → T5（`observability_dev_seam.rs` — review で accepted trade-off から確定検証へ格上げ、Risk #4 参照）、POSIX `0600` write-mode → T3、非露出 warn → T5、`proc_invocation` parity → T5）。

## Open Questions / Gaps

- 未解決の設計判断はない（spec / design の Deferred Design Concerns は `--grant-file`・`usage.jsonl` record 拡張・`process.tasks` 同型機構・`policy grant` の project 対象/structured flag で、いずれも本 plan の Out of Scope として明記済み）。
- focused test-spec は `docs/test-specs/invocation-scoped-command-grants.md`（TSPEC-034FoDgwSnThCuTO5aNUYj）として作成済みで本 plan と同時に `approved` — `relations.verified-by` へ記録済み。各 task doc 生成時に task の `verified-by` へも記録する（planning-flow 手順）。実行可能な検証コマンドは task の Verification に残し、行為保証（trusted-layer-only authoring・fail-loud collision・grants-only 制約・非永続性）の意図は test-spec 側で pin する。
- `grants.invocation` span field の assert は `observability_dev_seam.rs` の file-sink 経路（`CHEAPER_LOG=debug` subprocess → `.cheaper/logs` JSON parse で `cheaper.run.tool_scope` span record の fields を検証）で deterministic に成立するため確定検証とする（review で `tools.degraded` と同じ sync 区間 record と判明 — 元の「stderr 行のみ必須」hedge は解消）。
- `cheaper hook` は `parse_cli_args` に到達しない独立 parse path（`cli.rs:165-193` の early branch → `hook::execute_hook` → `hook::parse_hook_argv` が `--target` のみ認識し残りを fail-open で silent 無視）を持つため、`cheaper hook ... --grant` は reject されず silent ignore される。`hook` は run 系 command でなく AC17 の reject 契約対象外であり、spec 適合の known gap として明記する。
- `policy grant add` の spec-json も cmdline へ一時的に露出する同種 caveat を持つ — cmdline 非経由の file 入力は `--grant-file` として ADR が defer 済み。

## Self-Review

- [x] spec の AC1–AC17 が T1–T6 と Verification matrix に対応づいている（AC1/AC2/AC8/AC10/AC14→T5、AC3/AC4→T1+T5、AC5→T4+T5、AC6/AC7→T3（utility 面）+T5（run 行為側半分）、AC9/AC16→T3、AC11→T6、AC12→全 task、AC13→T2+T5、AC15→T2+T5、AC17→T2+T3）。
- [x] ADR の副次決定（Option A 合成 layer・`name` 内包 JSON + strip・explicit collision error・workflow 適用・`grantRequests` shape・`policy grant` UX・`read_policy_yaml`/`stringify_yaml`/`atomic_write` 基盤・audit 2 面・warn-and-continue 範囲限定・非 run reject）が task に反映されている。
- [x] placeholder（TBD/TODO/implement later）はない。
- [x] file path・relation key・status・command・用語は design §(h)/(i) と整合している。
- [x] compile-forced propagation の所有 task（T1: `LoadPolicyOptions` 8 site、T2: `RunWorkerDeps` 6 site、T3: `ParsedCli` match arm）と各 task 境界の workspace green 条件を明示している。

## Implementation Handoff

- Recommended mode: inline または delegated/subagent-capable implementation。critical path は T1 → T2 → T5 → T6 で、T3 は T1 以降・T4 は完全独立だが、逐次実行を既定とする（task 間の file 共有 — args.rs・cli.rs・load.rs — が並行化の実益を小さくする）。
- Review checkpoints: T1 完了時（invocation layer の merge 意味論 + workspace green）、T3 完了時（write path の validation 順序 + `0600` + atomic write 確認）、T5 完了時（AC coverage）で区切る。
- User approval required before dispatch: yes。
- 実行者は task ごとに上記 verification matrix の focused command を実行し、全 task 完了後に `mise run check` を通す。

## Review Handoff

- 重点 review 対象: T1 の grants-only key 制約と collision 検査順序（validate 通過後 — malformed spec が collision error を装わない）、`details.layer` attribution の shared-validation vs 構造的失敗の分類、T2 の `build_invocation_layer` の `name` strip と per-command reject 網羅、T3 の merged full-object validation（既存 file の pre-existing corruption 検査を含む）と `result.tokens` 非含有。
- 上流 doc: spec `docs/specs/invocation-scoped-command-grants.md`、design `docs/designs/invocation-scoped-command-grants.md`、ADR `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`。
- 明示的な前提: 各 task 境界で workspace は green を維持する（compile-forced propagation は導入 task に同梱する #101/#102 precedent — T1 が `LoadPolicyOptions`、T2 が `RunWorkerDeps` を所有）。`Option<&Map>` 型の採用は churn 削減ではなく semantic optionality（grant 無し = `None` で既存経路と差分なし）のためであり、literal 更新は避けられないため T1 に集約した。`grantRequests` は advisory であり approval/組立 spec の review は parent agent / user の責務 — cheaper は request の正しさを保証しない。

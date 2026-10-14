---
id: "PLAN-034EO1kmKwJ8MAHSHWyCws"
type: "plan"
status: "completed"
title: "policy-gated command tool を実装する"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/101"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/policy-gated-command-tool.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/implement-managed-process-canonical-tool.md"
    - "../plans/implement-managed-process-lifecycle-hardening.md"
    - "../plans/implement-invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/policy-gated-command-tool.md"
    - "../specs/policy-gated-command-tool.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/policy-gated-command-tool.md"
  references:
    - "../guides/managed-process.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/managed-process-lifecycle-hardening.md"
  defers: []
  deferred-by: []
---

# policy-gated command tool を実装する Implementation Plan

## Goal

canonical tool vocabulary に 9 個目の tool `command` を追加する。trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）で宣言された grant が許可する executable に限り、動的 argv を付けて managed subprocess として実行できるようにし、Worker と Workflow `kind: tool` job の両経路から同一の canonical 実装で利用可能にする。既存 `process` tool の contract と挙動は不変とする（AC1）。

## Scope

単一 implementation stream として扱う。core の policy/vocabulary → tools の shared `proc` 抽出と `command` tool → runtime/CLI plumbing → test → docs は同一の critical path と release intent（issue #101 の単一 feature）を共有し、独立に ship・verify できる subsystem には分割できないため、複数 plan への分割は行わない。

境界:

- In: `command` の canonical tool 化（vocabulary / validate / catalog / registry / invocation / workflow `kind: tool` / runtime plumbing）、`command` policy section と trusted-layer-only schema 境界（`PolicySchemaMode::ProjectPartial`）、argv pattern 言語、executable 解決 + pin + interpreter deny、writeScope fail-closed、独自 pool `"command"`、shared `tools/proc/` 抽出 refactor、builtin `policy.yaml` 更新、user-facing trust/sandbox 限界の docs。
- Out: unrestricted shell / interactive TTY / daemon、OS sandbox containment、tool-operator worker 本体（issue #102）、artifact-store への実行 log 永続化、`process` tool の contract/behavior 変更、`process.tasks` の project extend 権限変更、project 側 narrowing field、interpreter opt-in knob。

## Upstream Documents

- approved spec `docs/specs/policy-gated-command-tool.md`（要件・AC1–AC7 の正本、source: issue #101）。
- accepted ADR `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`（pure Option A の信頼境界と managed-process 再利用方針）。

## Design Inputs

- approved design `docs/designs/policy-gated-command-tool.md`。§(a)–(k) が全設計判断を確定済みであり、本 plan は §(k) file-level change map と Verification Notes を task 分解へ投影したもの。設計判断の再検討は行わない。

## File Map

design §(k) を task 単位へ割り当てたもの。

| Path | Responsibility | Task |
| --- | --- | --- |
| `crates/cheaper-core/src/tools/vocabulary.rs` | `TOOL_NAMES` + `ToolName::Command` + `as_str`/`from_canonical` | T1 |
| `crates/cheaper-core/src/policy/command.rs`（新規） | `command_policy_properties` / `resolve_command_policy` / `ResolvedCommandPolicy` / `CommandGrant` / `command_grant_resources` / `is_denied_command_executable` / `command_denied_executable_re` / `ArgPattern` compile+match / `command_invalid`（`process.rs` の `invalid` 相当 — message prefix "invalid command policy"） | T1 |
| `crates/cheaper-core/src/policy/process.rs` | `argv_shell_re` を `pub` 化、`relative`/`str_vec`/`bounded_duration` を `pub(crate)` 化。`bounded_duration`（:175-184）は section label を引数化して `policy::command` の grant 解決からも使える形にする（"invalid process policy"/"process timer limit" 固定 wording を呼出し側 label で置換。`invalid`（:107-112）自体は process 側に残し、command は `command_invalid` を別途定義 — design §(k) と同じ判断）。`relative` の "path must be workspace-relative without traversal" は section 非依存のまま共有可能 | T1 |
| `crates/cheaper-core/src/policy/schema.rs` | complete + user partial schema への `command` section 追加、`PolicySchemaMode::ProjectPartial` + 3-validator tuple、`RuntimePolicy.command`、`normalize_policy` / `validate_policy_schema` 配線 | T1 |
| `crates/cheaper-core/src/policy/load.rs` | project call site を `ProjectPartial` へ切替 | T1 |
| `crates/cheaper-core/src/worker/validate.rs` | `is_mutating_tool` に `Command` | T1 |
| `crates/cheaper-core/src/policy/{capability,restrict}.rs` 等他 `RuntimePolicy` literal 構築点（capability.rs:86・restrict.rs:100,113・schema.rs:457） | `command` field 追加（compile-forced） | T1 |
| `crates/cheaper-core/tests/golden/` + `fixtures/` | vocabulary 8→9、policy schema mode mapping に `project-partial`、project 拒否 / user 受理 fixture、`policy/{load,restrict,schema}.json` 再生成 | T1 |
| `crates/cheaper-tools/src/tools/command/mod.rs`（新規） | T1: `command_schema`（`{name,args}` input schema）・`command_details_schema`・deny-stub `command_tool_definition`（execute は `POLICY_VIOLATION "command unavailable"` で失敗 — catalog/registry の closed enum arm の compile 充足用。real wiring は T3）。T3: grant 名埋め込み description・grant 解決 + argv match + metachar + cwd prepare + pin 再検証の execute glue・`command_operation_declaration` へ差し替え | T1（schema + stub）、T3（実装） |
| `crates/cheaper-tools/src/tools/proc/`（新規） | `executable.rs`/`tree.rs`/`spawn.rs`/`monitor.rs`/`cleanup.rs` の移動・汎化、`ProcLimits`/`ProcPrepared`/`ProcIdentity`/`proc::run`/`minimal_env`/cwd prepare/abort helper。`executable.rs` の error message label（`resolve_process_executable` の bare `node` / `unsupported()` — executable.rs:33-38・:147 の "process task" 固定 wording）を呼出し側 label 引数へ parameterize し、command 経路で "process task" と出ないようにする（process 側 message は不変） | T2 |
| `crates/cheaper-tools/src/tools/process/{mod.rs,prepare.rs,tests.rs}` | task 固有 glue に縮小、移動 item を `pub use` re-export で公開 path 維持。`pub use` は `pub(super)` の到達範囲を維持できないため、移動 helper（`minimal_env`・`display`・`spawn_code`・`spawn_failure_is_async`・`StreamCapture`/`Captures`/`Diagnostics`/`read_stream`）は `pub(crate)` へ拡張するか、unit test を `tools/proc/` へ同伴移動させる（test 群: `capture_finish_*`・`display_*`・`minimal_env_*`・`spawn_code_*`/`spawn_failure_*`。`executable.rs` の in-file `#[cfg(test)]` は file ごと移動） | T2 |
| `crates/cheaper-tools/src/tools/mod.rs` | `pub mod command;`（T1 — deny-stub module 公開）、`pub mod proc;`（T2） | T1（command）、T2（proc） |
| `crates/cheaper-tools/src/catalog.rs` | T1（compile-forced）: `input_schema`/`output_schema`/`resources` の `ToolName::Command` arm — `command_schema`/`command_details_schema`/`command_grant_resources` を返す実 arm（stub ではなく正しい schema を早期に置く）。T3: `NAMES` へ `Command` 追加 — **T3 まで `command` は catalog `NAMES`・registered set に入れず dormant とする**（採用した方針: NAMES 追加を T3 に遅らせ、`exposes_eight_schemas`→`exposes_nine_schemas` の更新も T3 に置く） | T1（arms）、T3（NAMES） |
| `crates/cheaper-tools/src/registry.rs` | T1（compile-forced）: `ToolRuntimeOptions.command` field、`registered_definition`（:475-584 の closed `match name`）へ `Command` deny-stub arm — `command_tool_definition` の stub を返し execute が `POLICY_VIOLATION "command unavailable"` で失敗する定義。T3: nonempty-grants gate・writeScope×command 拒否・pin map 構築・`ToolCtx.command`・`is_process`→`is_managed` 一般化（:845）・declaration arm（:810-824） | T1（field + stub arm）、T3（gates/wiring） |
| `crates/cheaper-tools/src/invocation.rs` | T1（compile-forced）: `InvocationToolRuntimeOptions.command` field（:60-72）+ `create_worker_scope` 内 `runtime_options` literal（:235-243）への機械的 field 追加（この時点では `options.runtime.command` passthrough）。T3: `inner.command` 格納（`create_invocation_tool_runtime` で `resolve_command_policy`）+ declared-shape parity check（`options.process` 同等の re-resolve → `PartialEq`、:210-216 先例）。T4: `runtime_options.command` を `options.runtime.command` から `inner.command` 再注入へ確定（`process` の :240-243 と同型 — scope は invocation-resolved grants を受け取り `None` にはしない） | T1（field）、T3（inner/parity）、T4（再注入） |
| `crates/cheaper-runtime/src/run/mod.rs` | `InvocationToolRuntimeOptions`（:279-295）/`ToolRuntimeOptions`（:424-455）literal へ `command: serde_json::to_value(&policy.command)` を配線（`process` と同型の機械的 serialize、compile-forced） | T1 |
| `crates/cheaper-runtime/{src/dev/fixture.rs,tests/run_dev_seam.rs,tests/ollama_e2e.rs}` | `RuntimePolicy` literal（fixture.rs:747・run_dev_seam.rs:146・ollama_e2e.rs:219）に `command` field（compile-forced） | T1 |
| `crates/cheaper-cli/resources/policy.yaml` | `command: {maxConcurrent: 1, grants: {}}` + `tools.allow` に `command` | T4 |
| `crates/cheaper-cli/src/workflow_run.rs` | `ensure_invocation` の `InvocationToolRuntimeOptions`（:227-255）と `ToolJobExecutor` の `ToolRuntimeOptions`（:455-473）へ `command` field 配線（`process` と同型 serialize、compile-forced） | T1 |
| `crates/cheaper-workflow/tests/{process,artifacts}.rs` | `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal（process.rs:32,105・artifacts.rs:69,99）へ `command` field（compile-forced） | T1 |
| `crates/cheaper-workflow/tests/*.rs` | `kind: tool` job 用 `resource_inference` 挙動固定 test | T5 |
| `crates/cheaper-tools/tests/proc_command.rs`（新規） | grant gate / argv match / deny / metachar / cwd / timeout / abort / nonzero / output-cap / tree-cleanup（Windows・POSIX） | T5 |
| `crates/cheaper-tools/tests/{proc_runtime,characterization,differential,proc_invocation}.rs`、`crates/cheaper-rig-adapter/tests/adapter.rs` | `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal（proc_runtime.rs:89・characterization.rs:74・differential.rs:670・proc_invocation.rs:93,106,432・adapter.rs:314）へ `command` field（compile-forced）— T1。`exposes_eight_schemas`→`exposes_nine_schemas`（proc_runtime.rs:119-121）は catalog `NAMES` 更新と同じ T3 | T1（literals）、T3（exposes_nine） |
| `docs/guides/command.md`（新規）または `docs/guides/managed-process.md` 拡張 | trust / sandbox 限界、ambient PATH provenance、per-scope pin の残余 TOCTOU、argv pattern、capability 限界、`process.tasks` divergence、writeScope 非両立 | T6 |
| `docs/README.md` + root `README.md` | 新規 `docs/guides/command.md` を作成する場合、docs index（docs/README.md）と root README の guide links に `command.md` を追加 | T6 |
| `docs/guides/configuration.md` | builtin `policy.yaml` verbatim embed（:93-135）の同期更新 | T6 |
| `docs/guides/workers.md` | gate 記述（:37-38・:99）を三重ゲートへ更新、exposure ≠ authorization の明記 | T6 |

## Implementation Sequence

- [ ] T1: core vocabulary + command policy module + schema mode 分割を実装し、golden fixture を再生成する。加えて `ToolName` closed enum 拡張と必須 field 追加が強制する全 mechanical 変更（全 `RuntimePolicy`/`ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal、catalog/registry の deny-stub arm、`tools/command/` stub module）を同梱し、`cargo build --workspace --all-targets` を green に保つ。
- [ ] T2: `tools/proc/` 抽出 refactor を process 挙動不変で完了させる（refactor-only commit）。`pub(super)` の到達範囲制約は `pub(crate)` 拡張か test の同伴移動で解決し、executable error label を引数化する。
- [ ] T3: `tools/command/` の stub を実装へ差し替え、catalog `NAMES` 追加 + registry/invocation の gate・pin・parity plumbing を実装する。
- [ ] T4: `create_worker_scope` の `inner.command` 再注入と builtin policy の配線を完了させる。
- [ ] T5: unit + integration test と golden fixture を整備し、AC を網羅する。
- [ ] T6: user-facing docs（trust / sandbox 限界含む）と index/README link を整備する。
- [ ] 全 task の green を確認した上で `mise run check`（canonical verification）を実行する。

## Dependencies and Risks

### Dependency Graph

| Task | Depends on | Blocks |
| --- | --- | --- |
| T1 core vocabulary/policy/schema + compile-forced propagation | — | T3, T4, T5 |
| T2 `tools/proc/` 抽出 | — | T3 |
| T3 `tools/command/` 実装 + registry | T1（`resolve_command_policy`・deny set・`ToolName::Command`・options field/deny-stub arm）, T2（`proc::run`・shared prepare） | T4, T5 |
| T4 runtime/CLI plumbing + builtin policy | T1, T3 | T5（workflow 経路 test）, T6 |
| T5 tests | T1, T3, T4 | T6（docs が記述する挙動の確定） |
| T6 docs | T4（builtin `policy.yaml` 最終形）, T5（挙動固定） | — |

- critical path: T1 → T3 → T4 → T5 → T6。T2 は T1 と独立に先行・並行可能だが、T3 着手前に完了必須（T3 は `proc::run`・shared prepare・`resolve_process_executable` 移設先に依存）。
- T1 と T2 は file ownership がほぼ交差しないため並行実装可能。唯一の共有 file は `crates/cheaper-tools/src/tools/mod.rs`（T1 が `pub mod command`、T2 が `pub mod proc` を追加 — 1 行ずつの trivial conflict のみ）。
- **T1 は compile-forced propagation を所有する**: `ToolName` closed enum への `Command` 追加と `RuntimePolicy`/`ToolRuntimeOptions`/`InvocationToolRuntimeOptions` への必須 field 追加は、enum 参照点・struct literal 構築点を全て compile error にする。T1 の scope にはこれら全 mechanical 変更（全 literal の `command` field、catalog `input_schema`/`output_schema`/`resources` と registry `registered_definition` の `Command` deny-stub arm、`tools/command/` stub module + `pub mod command`）を含み、**T1 完了時点で `cargo build --workspace --all-targets` が green** でなければならない。compile-forced propagation は導入 task に同梱する（task は scope 境界であり commit 境界ではない）。deny-stub arm は dormant — `command` は T3 まで catalog `NAMES` / registered set に入れないため stub は実行経路から到達不能であり、schema arm は正しい `{name,args}` schema を返す（早期に実 arm を置く方が差分が小さい）。

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `tools/proc/` 抽出で `process` の lifecycle 挙動（timeout/abort/cleanup/error shape/details field）を退行させる | med | high | T2 は refactor-only commit とし、既存 `proc_process.rs` + unit + golden test を無変更で pass させることを merge 条件にする。`options.process.*` 直接参照の `ProcLimits` 化は旧 `mod.rs:305-309`・`:379` の読み取りを `proc::run` 内へ移すだけに限定。`tools::process` の `pub use` re-export で外部公開 path を維持 |
| 2 | `PolicySchemaMode::ProjectPartial` 分割で既存 user/project load 経路や `RestrictConcurrency` merge を壊す | med | high | `Partial` は user layer 用に温存し、project call site のみ切替。load/restrict/schema golden fixture を再生成し、`Partial` 受理・`ProjectPartial` 拒否の focused test を T1 で追加。`merge.rs` は無変更を維持 |
| 3 | interpreter/exec-wrapper deny list の網羅性不足で任意起動迂回が残る | med | high | design §(d) の静的 list（versioned name まで）を core `is_denied_command_executable` に集約し、declared basename・pin 時 resolved basename・spawn 前再検証の 3 適用点で評価。残余は「deny list は containment でなく known-bad への fail-fast」として docs に明記 |
| 4 | pin 境界の取り違い（`create_invocation_tool_runtime` に `names` が無い）で pin 漏れ・二重 pin | low | med | pin は `names` に `Command` を含む `create_tool_runtime` のみ。invocation 側は declared-shape resolve + `PartialEq` parity のみ（design §(b)/(g)）として test で固定 |
| 5 | golden fixture 再生成の漏れ（`RuntimePolicy.command` 追加で serialize 出力が変わる） | med | low | design §(k) の fixture 一覧（`policy/{load,restrict,schema}.json`、`vocabulary.json`、`worker_validate.rs` golden）を T1 の checklist 化し、`cargo test -p cheaper-core` の golden failure を全件解消 |
| 6 | `command` job の manifest narrowing が `resource_inference` で fail する経路の取りこぼし | low | med | code 変更不要（generic tool 分岐が保守的 manifest を返す）。underclaim が `declared {bucket} underclaims known access` で fail-closed する挙動固定 test を T5 で追加 |
| 7 | `is_mutating_tool` 追加による `savings.directlyComparable` / access.rs manifest の既存 worker への影響 | low | low | `Command` 追加のみで既存 tool の判定は不変。`command` 使用 worker で `directlyComparable == false` になることを T5 test で確認 |
| 8 | enum/必須 field 拡張の compile-forced propagation が task 分割を跨ぎ、T1 完了後〜T3 まで workspace が build red になる | —（T1 の scope 拡張で解消済） | high | T1 が `RuntimePolicy`/`ToolRuntimeOptions`/`InvocationToolRuntimeOptions` の全 literal 更新・options field 追加・catalog/registry の `Command` deny-stub arm・`tools/command/` stub module を同梱所有し、T1 完了時点で `cargo build --workspace --all-targets` を green にする。「package 単位で個別に gate すれば各 task が独立 compile する」という naive な前提は棄却 — compile-forced propagation は導入 task に同梱する（task は scope 境界であり commit 境界ではない） |

### Rollback Strategy

- 本 plan は永続 format 移行・破壊的 API 変更・依存追加を含まない。`ResolvedCommandPolicy` は新規型、`PolicySchemaMode::ProjectPartial` は新 variant であり、既存挙動への不可逆変更はない。
- **Trigger**: T2 refactor で `process` 既存 test が regression した場合、または `ProjectPartial` 導入で既存 policy load が拒否される場合。
- **Procedure**: task 単位の coherent commit を `git revert`。ただし **T2 の refactor が独立に revert 可能なのは T3 が land する前だけ** — T3 以降は `tools/command/`・registry gate・invocation が `proc/` 移設先へ依存するため、T3/T5 が land した後の revert は T5→T4→T3→T2（必要に応じ T1 まで）の chain を逆順に unwind するか、fix-forward を選ぶ。builtin `policy.yaml` の `command` section / `tools.allow` 追加を revert すれば `command` は vocabulary 上 dormant となり実行経路は全て閉じる（新 vocabulary のため既存 policy への影響なし）が、これは runtime kill switch であって code revert の代替ではない。
- **Verification**: revert 後に `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で workspace 全体が compile すること（`RuntimePolicy`/`ToolRuntimeOptions` literal は cheaper-runtime・cheaper-cli・test target にも存在するため core+tools だけの検証では不十分）、かつ `mise exec -- cargo test -p cheaper-core -p cheaper-tools` と既存 `proc_process.rs` suite が pass すること。

## Task Breakdown

各 task は coherent な検証可能単位。task-doc 生成時は `depends-on` relation で下記 DAG を表現し、`verified-by` に `../test-specs/policy-gated-command-tool.md` を記録する。

- T1 `command-core-policy`: core に `ToolName::Command`・`policy::command`（grant schema / resolve / `ArgPattern` / deny set / `command_grant_resources` / `command_invalid`）・`bounded_duration` の section label 引数化・`PolicySchemaMode::ProjectPartial` + load.rs 配線 + `normalize_policy` + `is_mutating_tool` + `RuntimePolicy.command` + golden fixture 再生成を実装する。**加えて compile-forced propagation を所有し、T1 完了時点で `cargo build --workspace --all-targets` が green であること**:
  - (a) `RuntimePolicy.command` 必須 field に伴う全 literal 更新（core の capability.rs/restrict.rs test fixture、cheaper-runtime の fixture.rs:747・run_dev_seam.rs:146・ollama_e2e.rs:219、および他の全構築点）。
  - (b) `ToolRuntimeOptions.command`・`InvocationToolRuntimeOptions.command` field 追加と全 struct literal 更新（cheaper-tools tests の proc_runtime.rs:89・characterization.rs:74・differential.rs:670・proc_invocation.rs:93,106,432、cheaper-rig-adapter adapter.rs:314、cheaper-cli workflow_run.rs:227-255 と :455-473、cheaper-runtime run/mod.rs:279-295 と :424-455、cheaper-workflow tests process.rs:32,105・artifacts.rs:69,99、invocation.rs:235-243 の `runtime_options` literal）。runtime/CLI 側は `serde_json::to_value(&policy.command)` の機械的 serialize 配線（`process` と同型）まで行う。
  - (c) `ToolName` closed enum の exhaustive match を compile 可能にする deny-stub arm: catalog.rs の `input_schema`/`output_schema`/`resources` に `Command` arm（実 `command_schema`/`command_details_schema`/`command_grant_resources` を返す）、registry.rs `registered_definition` に `Command` arm（`tools/command/` の stub `command_tool_definition` を返し、execute は `POLICY_VIOLATION "command unavailable"` で失敗）。`tools/command/mod.rs` stub + `pub mod command` も T1。**`command` は T3 まで catalog `NAMES` / registered set に入れず**、stub arm は到達不能の compile 充足のみ（`exposes_eight_schemas`→nine は NAMES 更新と同じ T3）。
  - 正当化: compile-forced propagation は導入 task に同梱する（task は scope 境界であり commit 境界ではない）。depends-on: なし。
- T2 `proc-extraction`: `tools/proc/` へ lifecycle 機構（spawn/monitor/cleanup/executable/tree/prepare-generic/`proc::run`）を抽出し、`tools::process` を task 固有 glue + `pub use` re-export へ縮小する。`pub use` は `pub(super)` の到達範囲を維持できないため、移動 helper（`minimal_env`・`display`・`spawn_code`・`spawn_failure_is_async`・`StreamCapture`/`Captures`/`Diagnostics`/`read_stream`）は `pub(crate)` 拡張か unit test の `tools/proc/` 同伴移動（test 群: `capture_finish_*`・`display_*`・`minimal_env_*`・`spawn_code_*`/`spawn_failure_*`）で対処する — **process unit tests relocated/widened as needed; no test silently deleted**。`resolve_process_executable` の "process task" 固定 wording（executable.rs:33-38・:147）は呼出し側 label 引数へ parameterize し、command 経路で "process task" を出さない（process 側 message は不変）。挙動変更なしの refactor-only。depends-on: なし。
- T3 `command-tool-and-registry`: `tools/command/` の deny-stub を実装へ差し替え（grant 名埋め込み description・grant 解決・argv match・metachar・cwd prepare・pin 再検証・`command_operation_declaration`）、catalog `NAMES` へ `Command` 追加（`exposes_eight_schemas`→`exposes_nine_schemas` 同時更新）、registry（nonempty-grants gate・writeScope 拒否・pin map・`ToolCtx.command`・`is_process`→`is_managed` 一般化・declaration arm）、invocation（`inner.command` 格納 + declared-shape parity check）を実装する。depends-on: T1, T2。
- T4 `runtime-cli-plumbing`: `create_worker_scope` の `runtime_options.command` を `options.runtime.command` passthrough から `inner.command` 再注入へ確定させる（invocation.rs:240-243 の `process` 再注入と同型 — scope は invocation-resolved grants を受け取り `None` にはしない）。builtin `policy.yaml` に `command: {maxConcurrent: 1, grants: {}}` section + `tools.allow` の `command` を追加し、complete schema validation を通す。depends-on: T1, T3。
- T5 `command-tests`: unit + integration（`proc_command.rs`、fake provider 経路、Windows/POSIX fixture、workflow `kind: tool` カバレッジ）と残りの golden/構築点更新を完了させる。depends-on: T1, T3, T4。
- T6 `command-docs`: `configuration.md` mirror・`workers.md` 三重ゲート記述・`managed-process.md` / 新規 `command.md` の trust/sandbox 限界を整備する。新規 `command.md` を作成する場合は `docs/README.md` index と root `README.md` の guide links も更新する。depends-on: T4, T5。

## Verification

canonical verification は `mise run check`（`biome check` → `cargo fmt --check` → `cargo clippy` → version/doc-id check → `cargo test --workspace` → `node --test` → lockfile check → payload check）。cargo 系は `mise exec --` 経由で `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付き。

| Task | Verification command or criteria | Pass condition | AC |
| --- | --- | --- | --- |
| T1 | `mise exec -- cargo test -p cheaper-core` + `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | vocabulary/policy/schema/load/restrict/worker_validate の unit + golden が pass。project-partial grant 拒否 / user-partial 受理 fixture が green。**workspace 全体（runtime・cli・test target 含む）が compile green** — compile-forced propagation 完遂の証明 | AC4（project schema 拒否） |
| T2 | `mise exec -- cargo test -p cheaper-tools` | 既存 `proc_process.rs`・process unit test が pass（refactor-only の証明）。unit test は `pub(crate)` 拡張または `tools/proc/` への relocate で維持され、**test が暗黙に削除・無効化されていない**（件数不減） | AC1 |
| T3 | `mise exec -- cargo test -p cheaper-tools`（全 test target を compile+run — `tests/proc_invocation.rs` 等の integration target 込み）+ `mise exec -- cargo test -p cheaper-tools --test proc_invocation`（gate/parity focused） | nonempty-grants gate・writeScope 拒否・pin・argv match・metachar/deny/cwd 拒否・`{name,args}` 以外の `INVALID_INPUT`・parity check・`exposes_nine_schemas` が pass。deny-stub が実装へ置換済み | AC2, AC3, AC4 |
| T4 | `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | run/dev-seam/workflow_run の compile + test pass。builtin `policy.yaml` が complete schema validation を通過 | AC3（両経路） |
| T5 | `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` + `mise exec -- cargo test -p cheaper-tools --test proc_command`（`tests/proc_command.rs` 作成後の focused run） | grant gate（worker + workflow 両経路）・dormant runtime・timeout/abort/nonzero/output-cap/tree-cleanup（Windows/POSIX）・`directlyComparable == false`・独自 pool cap・manifest underclaim fail-closed が全て pass | AC1–AC6 |
| T6 | doc audit + `docs/guides/configuration.md` と builtin `policy.yaml` の diff 確認 | trust/sandbox 限界・ambient PATH provenance・capability 限界・divergence・三重ゲートが docs に記載され、verbatim embed が同期 | AC7 |
| 全 task | `mise run check` | pipeline 全段が exit 0 | AC7 |

design Verification Notes の未チェック項目は T5 の test checklist にそのまま引き継ぐ（process 既存 test 不変、dormant runtime、parity、INVALID_INPUT、interpreter/exec-wrapper/loader/shell/workspace-resident/metachar/pattern 不一致/cwd escape の実行前 reject、timeout/abort/nonzero/output-cap/tree-cleanup、`directlyComparable`/`meta.changes`/pool cap）。

## Open Questions / Gaps

- 未解決の設計判断はない（spec の Deferred Design Concern は design §(a)–(i) で全て確定済み）。spec が「pin は policy load 時に一度だけ」と記述する点に対し、design が採る per-scope pin + spawn 前再検証は意図的 deviation であり、design §(b) と spec 本文の双方に整合記述が入っていることを実装時に確認する。
- focused test-spec を `docs/test-specs/policy-gated-command-tool.md` として作成する（trust-boundary の保証を対象: project schema での `command.grants` 拒否、nonempty-grants fail-closed、deny set の 3 適用点、writeScope 非両立、`{name,args}` 以外の `INVALID_INPUT` surface）。plan の `relations.verified-by` はこの test-spec を指し、**task doc は `verified-by` に同 test-spec を記録する**。実行可能な検証コマンドは task の Verification に残し、行為保証の意図は test-spec 側で pin する。
- deny list は網羅を保証しない known-bad fail-fast であり、発見時は `is_denied_command_executable` の拡張で運用する。

## Self-Review

- [x] spec の AC1–AC7 の全てが T1–T6 と Verification matrix に対応づいている。
- [x] ADR の副次決定（`{name,args}` input、nonempty-grants gate、invocation parity、managed tracking、merge.rs 無変更）が task に反映されている。
- [x] placeholder（TBD/TODO/implement later）はない。
- [x] file path・relation key・status・command・用語は design §(k) と整合している。

## Implementation Handoff

- Recommended mode: inline または delegated/subagent-capable implementation。T1 と T2 は file ownership がほぼ交差しない（`tools/mod.rs` の `pub mod` 行のみ共有）ため並行 dispatch 可能、T3 以降は critical path 順に逐次実行。
- Review checkpoints: T2 完了時（process 無退行の証明）、T3 完了時（信頼境界 gate の網羅確認）、T5 完了時（AC coverage）で区切る。
- User approval required before dispatch: yes。
- 実行者は task ごとに上記 verification matrix の focused command を実行し、全 task 完了後に `mise run check` を通す。

## Review Handoff

- 重点 review 対象: T1 の `ProjectPartial` schema 境界（信頼境界の本体）、T2 の refactor-only 性、T3 の pin/gate/parity（信頼境界の実行面）、deny list の適用点 3 箇所。
- 上流 doc: spec `docs/specs/policy-gated-command-tool.md`、design `docs/designs/policy-gated-command-tool.md`、ADR `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`。
- 明示的な前提: pin は per-scope（`create_tool_runtime` 境界）で spec 記述に対する design の意図的 deviation を採用。`tools.allow` は exposure toggle であり authorization ではない（信頼境界は `command.grants` の trusted-only authoring に集約）。

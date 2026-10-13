---
id: "PLAN-034ETMVrwmwh0Z0jq4gwtt"
type: "plan"
status: "completed"
title: "tool-operator built-in worker と cheaper.optionalTools 縮退機構を実装する"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/102"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/tool-operator-built-in-worker.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/tool-operator-built-in-worker.md"
    - "../specs/tool-operator-built-in-worker.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/tool-operator-built-in-worker.md"
  references:
    - "../guides/workers.md"
    - "../guides/command.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# tool-operator built-in worker と cheaper.optionalTools 縮退機構を実装する Implementation Plan

## Goal

built-in Worker `tool-operator`（CLI alias `tool-operate`）と、それを可能にする core 機構 `cheaper.optionalTools`（worker frontmatter で opt-in した scope-gated tool を、content-gate 不成立時に `POLICY_VIOLATION` ではなく未登録へ縮退する per-tool soft gating — ADR-034ESBcMTWxNCMDI9obdb8 の Option C1）を実装する。`read` / `grep` / `find` / `ls` / `count_tokens` / `process` / `command` を宣言し、builtin default policy（`process.tasks: {}` / `command.grants: {}`）でも scope 構築が死なず model session まで到達して、evidence 付き compact structured result を親へ返す worker を提供する。既存 worker（`optionalTools` 未指定）の意味論は一切変えない。

## Scope

単一 implementation stream として扱う。core の `optionalTools` machinery → registry degrade path + prompt surfacing → worker 定義 / schema / alias → fixture DSL + test → docs / APM は同一の critical path と release intent（issue #102 の単一 feature）を共有し、独立に ship・verify できる subsystem には分割できないため、複数 plan への分割は行わない。

境界:

- In: `cheaper.optionalTools` frontmatter key の parse・validate・propagation、`is_scope_gated_tool` predicate、registry の opt-in degrade path + `degraded_names()` 報告、実登録集合からの "Available tools" 組み立てと利用不可注記（conditional blocked 誘導行）、`tools.degraded` span field、`tool-operator` worker 定義 + input/output JSON Schema、CLI alias `tool-operate`、fixture DSL + dev-seam / regression / golden test、公開面 docs + APM skill `cheaper-tool-operate`。
- Out: `cheaper tool-operate "<task>"` 形の positional-sugar parser 変更（canonical invocation は `--input` / stdin JSON のみ、parser 規約ごと別 spec）、per-worker execution 制限 override（global `maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000` に従う）、`ollama_e2e.rs` の実 provider e2e（本 iteration 対象外）、usage.jsonl / envelope `meta` の shape 変更、worker 内からの再帰的 worker delegation・独自 agent loop・追加 agent engine、`write` / `edit` capability、pin 失敗・`resolve_*_policy` error・writeScope × `command` reject の soft 化（opt-in tool に対しても hard `POLICY_VIOLATION` のまま）、builtin `policy.yaml` への非空 `tasks` / `grants` 同梱（Option D — security regression のため採用しない）。

## Upstream Documents

- approved spec `docs/specs/tool-operator-built-in-worker.md`（要件・AC1–AC13 の正本、source: issue #102）。
- accepted ADR `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`（Option C1: per-tool opt-in 縮退機構の外部挙動と touch point。admission（`tools.allow` / `check_capabilities`）と content-gate（nonempty tasks / grants）の 2 層分離を固定）。

## Design Inputs

- approved design `docs/designs/tool-operator-built-in-worker.md`。§(a)–(j) が全設計判断を確定済みであり、本 plan は §(i) file-level change map と Verification Notes を task 分解へ投影したもの。設計判断の再検討は行わない。

## File Map

design §(i) を task 単位へ割り当てたもの。`ToolRuntimeOptions.degradable` と `WorkerCheaper.optional_tools` は struct literal 構築の compile-forced propagation を持つため、全構築 site を T1 が所有する（design §(j) 列挙済み — #101 と同じ機械的波及）。

| Path | Responsibility | Task |
| --- | --- | --- |
| `crates/cheaper-core/src/tools/vocabulary.rs` | `pub fn ToolName::resolve`（canonical 名 → `tool_alias` alias table の single-sourced resolver）、`is_scope_gated_tool` predicate、`normalize_tool_names` の per-element path（:111）を `resolve` 呼び出しへ refactor | T1 |
| `crates/cheaper-core/src/worker/validate.rs` | `CHEAPER_KEYS`（:94）へ `"optionalTools"` 追加、`WorkerCheaper.optional_tools: Vec<ToolName>`（構築 literal :250 — compile-forced）、design §(b) の 7 rule（`optionalTools-invalid` / `-empty` / `-unknown-tool` / `-undeclared` / `-not-gated` / `-duplicate` / `-not-proper-subset`、全て `InvalidWorkerDefinition` via `invalid()`）+ unit test | T1 |
| `crates/cheaper-core/tests/golden/common/mod.rs` | `dump_worker_def`（:415-455）へ `optionalTools` の conditional emit（非空時のみ `cheaper.optionalTools` を出力 — `input` / `writeScope` と同じ pattern。T4 の accept golden の前提） | T1 |
| `crates/cheaper-tools/src/registry.rs` | `ToolRuntimeOptions.degradable: Vec<ToolName>` field（:90-123）追加 — degrade path 本体は T2 | T1 |
| `crates/cheaper-tools/src/invocation.rs` | `create_worker_scope` 内の `ToolRuntimeOptions` 再構成（:257-287）へ `degradable` pass-through 1 行。`WorkerScopeOptions` / `InvocationToolRuntimeOptions` は shape 不変 | T1 |
| `crates/cheaper-runtime/src/run/mod.rs` | `create_tool_scope` の `ToolRuntimeOptions` literal（:427-459）へ `degradable: worker.cheaper.optional_tools.clone()` 配線 | T1 |
| `crates/cheaper-cli/src/workflow_run.rs` | `ToolJobExecutor` の `ToolRuntimeOptions` literal（:462-482）へ `degradable: Vec::new()`（`kind: tool` job は frontmatter を持たず opt-in 不能。`kind: worker` job は run 経路で自動的に縮退が効く） | T1 |
| `crates/cheaper-tools/tests/{proc_command.rs:202, proc_invocation.rs:107, proc_runtime.rs:89, characterization.rs:74, differential.rs:670}` | 各 `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及） | T1 |
| `crates/cheaper-rig-adapter/tests/adapter.rs:314` | `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及） | T1 |
| `crates/cheaper-workflow/tests/{artifacts.rs:69, process.rs:32, common/mod.rs:109}` | `ToolRuntimeOptions` literal 2 箇所と `WorkerCheaper` literal（:109 — compile-forced 波及。構築 site は validate.rs:250 とここの 2 箇所のみ） | T1 |
| `crates/cheaper-tools/src/registry.rs` | degrade path: `create_tool_runtime`（gate 分岐 :1063-1095）で effective names 決定 — `options.degradable` membership 時は `names.retain` + `degraded.push`、それ以外は従来の `POLICY_VIOLATION`。grants 非空時の writeScope × `command` reject は hard のまま。冒頭の `options.degradable.retain(is_scope_gated_tool)` clamp（optional defensive）と degradation 後の有効 names 空なら `POLICY_VIOLATION` の guard。`degraded` を `options.names` 宣言順に整列。pin 判定（:1139）・`ScopeAccessOptions.names`（:1174）・registered loop（:1246）・`RuntimeInner.names`（:1257）の全 consumer を effective names へ。`RuntimeInner.degraded` + `pub fn ToolRuntime::degraded_names()` | T2 |
| `crates/cheaper-runtime/src/run/mod.rs` | `create_session`（:474-516）の `tool_names` を `self.effective_tools` ではなく `scope.definitions()` の実登録集合から組み立て、`scope.degraded_names()` を `unavailable_tools` として `build_system_prompt` 第 4 引数へ。`cheaper.run.tool_scope` span（:417）に `tools.degraded = field::Empty` を宣言し、inner block 閉鎖（:462 — `_stage_guard` はここで drop 済み）と `scope.ready().instrument(tool_scope_stage)`（:463-465 — `instrument` が span を move するため、これが record の真の期限）の間で record（宣言順 join、metadata-only 規約） | T2 |
| `crates/cheaper-rig-adapter/src/agent.rs` | `build_system_prompt`（:225-264）へ第 4 引数 `unavailable_tools: &[String]` + 非空時の利用不可注記行（`- Declared but unavailable under current policy: {join}`）+ `output_schema` の `properties.status.enum` に `"blocked"` を含む場合のみ emit する blocked 誘導行。unit test で文言固定 | T2 |
| `crates/cheaper-rig-adapter/tests/adapter.rs:364,560,572,742` | `build_system_prompt` 既存 4 call site を第 4 引数追加の signature change へ追従（compile-forced 波及）+ 注記文言・blocked 誘導行 conditional の新規 unit test | T2 |
| `crates/cheaper-cli/resources/workers/tool-operator.md`（新規） | design §(a) の frontmatter（`tools` + `cheaper.optionalTools: [process, command]` + `model.tier: low` + `workload.outputPerInput: 0.3` + input/output schema file、`writeScope` なし）と英語 body（role / tool usage discipline（gated tool は条件付き言及のみ）/ evidence requirements / never dump large output / blocked semantics / partial・failed の必須 field mapping / respect caps / honesty about mutations） | T3 |
| `crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json`（新規） | design §(h) の shape — `task`（required、maxLength 4000）+ `context` / `constraints` / `output` hints。`$defs` なし、全 object `additionalProperties: false` | T3 |
| `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`（新規） | design §(h) の shape — `status`（`success`/`partial`/`failed`/`blocked`）+ `summary` + `findings[]`（`evidence` required）+ `unresolved` + `nextActions` required、top-level `evidence[]` optional、`$defs.evidence` は `kind` discriminator 単一 object（`anyOf` 不使用） | T3 |
| `crates/cheaper-cli/src/args.rs` | `alias_worker()`（:39-46）に `"tool-operate" => Some("tool-operator")` + error enumeration（:483 の `command required: run | bulk-read | code-write | doc-write | ...`）へ `tool-operate` | T3 |
| `crates/cheaper-cli/src/cli.rs` | HELP（:516-518）に `  tool-operate           Alias for: run tool-operator` | T3 |
| `crates/cheaper-cli/tests/args.rs` | `run_alias` table（:77-79）へ `("tool-operate", "tool-operator")` + extra positional reject の既存 pattern | T3 |
| `crates/cheaper-runtime/src/dev/fixture.rs` | `FixtureWorker.optional_tools: Vec<String>`（struct 既存の `#[serde(default)]` に乗る field 追加 — fixture DSL key は他 field と同じく snake_case `optional_tools`。rename は不要）+ `to_markdown()`（:291-312）が非空時のみ `cheaper:` 配下へ frontmatter contract key `optionalTools:` を emit — regression harness が `worker_markdown()` を無条件で seed するため DSL knob は必須（design §(i)） | T4 |
| `crates/cheaper-cli/tests/run_dev_seam.rs` | degraded run（builtin-default policy で `POLICY_VIOLATION` 無しに model 到達）・grant 定義環境 run（`command_user_policy` + `FIXTURE_COMMAND_CALL` で実 spawn）・capability-denied run（`tools.allow` 除外は従来通り run 全体 fail）・mixed-policy 片側縮退・scripted degraded-name call → `UNKNOWN_TOOL`・scripted undeclared `write`/`edit` call の reject・`status` 各値（`success`/`partial`/`failed`/`blocked`）の出力・`meta.changes` 空・`directlyComparable=false`・usage record | T4 |
| `crates/cheaper-runtime/tests/`（`FakeSessionFactory` handle 層） | 縮退 run の system prompt に実登録集合 + 利用不可注記（blocked 誘導行の conditional emit 分岐含む）が含まれる assert | T4 |
| `crates/cheaper-cli/tests/regression/{cases,expected}/` | degraded run の regression corpus case（FixtureWorker DSL 経由 — file seed は harness に clobber されるため不可） | T4 |
| `crates/cheaper-core` worker_validate golden fixture | `optionalTools` の accept（正規化結果 emit）/ reject（reason 固定）fixture | T4 |
| `README.md` | built-in workers 表（:29-30・:67-70・:84-86）へ `tool-operator` / `tool-operate` | T5 |
| `docs/guides/workers.md` | built-in worker 表（:66-68）への行追加、「入力 schema（要点）」（:78-85）の per-worker bullet への `tool-operator` 項目追加、frontmatter key リファレンス（:36-60）へ `cheaper.optionalTools` key 項目（構文: `tools` 宣言名の部分集合・gated tool のみ・全 reject は `InvalidWorkerDefinition`）、縮退挙動の説明、opt-in worker は `blocked` 相当の status を output schema に含める推奨 | T5 |
| `docs/guides/usage.md` | コマンド構成の embed 済み alias 行（:13-16）へ `tool-operate           Alias for: run tool-operator` 追加、alias 列挙文（:28）へ `tool-operate` 追加、per-alias 使用例（:90-107）へ tool-operate 例追加、`savings.directlyComparable=false` の扱い（`process` / `command` 宣言で常に false — `is_directly_comparable_worker` の mutating 判定） | T5 |
| `docs/guides/command.md` | :9 の「`command` を expose する runtime 構築は `POLICY_VIOLATION` で fail closed」への opt-in caveat をその場で補足 + `tools.allow` admission と content-gate の棲み分け（admin が `command` を禁止しつつ縮退実行を許すには grant を定義しない） | T5 |
| `docs/guides/configuration.md` | 変更基準: worker frontmatter key の記述を持たないため原則無変更。実装中に `optionalTools` と交差する記述（worker との関係節 :201 周辺）を発見した場合のみ最小追記 | T5 |
| `packages/apm/.apm/skills/cheaper-tool-operate/`（新規） | `cheaper-bulk-read` / `cheaper-code-write` / `cheaper-doc-write` と parity の SKILL.md | T5 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | command list に `tool-operate` 追加 + tool-operator への delegation guidance（multi-step 操作 loop の委譲先として言及） | T5 |
| `docs/impl/ir/`（新規 1 file） | 実装完了時の IMPL record（task-doc → impl-doc の既存規約に従い scaffold で生成） | T5 |

## Implementation Sequence

- [ ] T1: `optionalTools` core machinery — `ToolName::resolve` + `is_scope_gated_tool`（vocabulary.rs）、`WorkerCheaper.optional_tools` parse + 7 rule validate + unit test（validate.rs）、`ToolRuntimeOptions.degradable` field と **全 compile-forced literal site**（run/mod.rs・workflow_run.rs・cheaper-tools 5 test・rig-adapter adapter.rs・cheaper-workflow 3 file）+ invocation.rs pass-through + `WorkerCheaper` literal 2 箇所 + golden dump emit。**T1 完了時点で `cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が green**。
- [ ] T2: registry degrade path + prompt surfacing — effective-names の gate 拡張（process / command の empty content-gate のみ縮退、pin 失敗・writeScope × command・`tools.allow` は hard のまま）、宣言順 degraded list + `degraded_names()`、zero-effective-names `POLICY_VIOLATION` guard、`tools.degraded` span field（record 位置は sync 区間規約）、"Available tools" を `scope.definitions()` から組み立て + `build_system_prompt` 第 4 引数と conditional blocked 誘導行。
- [ ] T3: `tool-operator` worker 定義 + input/output schema + CLI alias `tool-operate`（args.rs + HELP + error enumeration + args test）。
- [ ] T4: fixture DSL（`FixtureWorker.optional_tools` + `to_markdown` emit）+ dev-seam fixture test + runtime prompt assert + regression corpus + worker_validate golden — approved test-spec（`docs/test-specs/tool-operator-built-in-worker.md`、plan 作成時点で存在済み）が pin する行為保証を検証する test を実装する。
- [ ] T5: docs（README・workers.md・usage.md・command.md）+ APM skill `cheaper-tool-operate` + cheaper-usage SKILL.md 更新 + impl record。
- [ ] 全 task の green を確認した上で `mise run check`（canonical verification）を実行する。

## Dependencies and Risks

### Dependency Graph

| Task | Depends on | Blocks |
| --- | --- | --- |
| T1 `optionalTools` core machinery（compile-forced propagation 所有） | — | T2, T3, T4 |
| T2 registry degrade path + prompt surfacing | T1（`ToolRuntimeOptions.degradable` field・`WorkerCheaper.optional_tools`・`is_scope_gated_tool`） | T3, T4 |
| T3 `tool-operator` worker + schema + alias | T1（`optionalTools` key の parse/validate が無いと worker file が strict-key reject で load 不能）, T2（worker の normative 挙動である縮退が成立していること） | T4, T5 |
| T4 fixture DSL + tests | T1, T2, T3（degraded-run fixture test が worker file に依存） | T5 |
| T5 docs + APM | T3（記述対象の worker/alias が存在）, T4（挙動固定） | — |

- critical path: T1 → T2 → T3 → T4 → T5 の逐次。T1 が root。
- T3 の worker file 本体と T4 の fixture DSL は file ownership が交差しないが、T4 の dev-seam / regression fixture test が実 worker file（`tool-operator.md`）と T2 の degrade path の双方を検証するため並行化の実益は小さく、逐次のままとする。
- **T1 は compile-forced propagation を所有する**: `ToolRuntimeOptions.degradable` と `WorkerCheaper.optional_tools` の必須 field 追加は全 literal 構築点を compile error にする。T1 の scope は design §(i)/(j) が列挙した全 mechanical 変更を含み、**T1 完了時点で `cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が green** でなければならない（#101 precedent — compile-forced propagation は導入 task に同梱する。task は scope 境界であり commit 境界ではない）。`src/dev/fixture.rs` は serde DSL で literal を持たず compile-forced 対象外（`FixtureWorker.optional_tools` knob は T4）。

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | compile-forced propagation の波及 breadth — `ToolRuntimeOptions.degradable`（8 literal site + run/cli 配線）と `WorkerCheaper.optional_tools`（2 site）の必須 field 化で workspace が一時 build red になる | high（発生確定） | med | design §(i)/(j) の全 site 列挙を T1 checklist 化し、dev-seam feature 付き `--all-targets` build を T1 の exit criteria にする（dev-seam 無しでは seam 依存 test target が検証から抜け落ちる — ADR-034DyGyITd0YaATg8fkld7）。漏れは compile error で検出されるため silent miss はない |
| 2 | golden / fixture churn — worker_validate golden・regression expected・dev-seam fixture に `optionalTools` 系 case を追加する際、既存 golden が意図せず変わる | med | low | `RuntimePolicy` / usage.jsonl record / envelope `meta` / vocabulary.json の toolNames は shape 不変（design §(j)）のため広い再生成は不要の見込み。差分が出た場合は本設計の対象かを確認し、対象外なら実装 defect として調査 |
| 3 | prompt-injection consistency — system prompt の利用不可注記と worker body の gated tool 言及、および output schema の `status` enum が矛盾する（schema が受理しない `blocked` を prompt が促す等） | med | med | blocked 誘導行は `status` enum に `"blocked"` を含む schema のみ emit（design §(f)）。worker body で `process` / `command` の言及は常に「"Available tools" に列挙されている場合に限る」条件付き表現とする。文言は rig-adapter unit test と runtime `FakeSessionFactory` assert で固定 |
| 4 | regression harness clobbering — harness が `f.worker_markdown()` を無条件で `.cheaper/workers/<name>.md` へ書き込むため、file seed した opt-in worker は上書きされ degraded case を表現不能 | med | med | `FixtureWorker.optional_tools` + `to_markdown()` emit を T4 の先行 item とし、全 regression/dev-seam case を DSL 経由で表現する（design §(i) で必須 knob と明記済み） |
| 5 | `tools.degraded` span field の検証コスト — in-process の tracing-capture test harness は存在しないが、`--log-format json` の file sink 経由なら assert は技術的に可能（`observability_dev_seam.rs:291-342` が `.cheaper/logs` の JSON sink を parse して span name・`time.busy` field を検証している） | —（accepted trade-off） | low | span field は observability mechanism として残し、検証は `degraded_names()` accessor の unit test + prompt 注記の内容固定で代替（file-sink parse より低コストな代替という design Risks / Verification Notes の accepted 決定）。判断を覆す場合は observability_dev_seam.rs と同じ pattern で file sink を parse する test を追加できる |
| 6 | 後方互換 regression — 非 opt-in worker の意味論が変わる（`tools: [command]` 非 opt-in worker が fail しなくなる等） | low | high | 既存 test `run_fake_stream_command_worker_denied_without_grants` を変更せず green に保つことを T2/T4 の pass condition にする。`optionalTools` 未指定 worker は `optional_tools` 空 → degrade path 不発の構造で保証 |
| 7 | model が degraded tool 名を呼んで invalid-tool retry budget を消費する | med | low | prompt 注記で抑止し、呼ばれても registry 未登録のため `UNKNOWN_TOOL` 経路で安全に失敗（実行経路自体が存在しない）。T4 の scripted degraded-name call test で `UNKNOWN_TOOL` を直接 assert |
| 8 | user-defined opt-in worker が `blocked` 相当 status を output schema に持たず repair/retry を浪費する | low | low | `tool-operator` は enum に `blocked` を含める。user-defined worker 向けには workers.md で「opt-in worker は blocked 相当の status を schema に含める」推奨を明記（T5） |

### Rollback Strategy

- 本 plan は永続 format 移行・破壊的 API 変更・依存追加を含まない。`cheaper.optionalTools` は opt-in の strict key であり、`WorkerCheaper.optional_tools` 空の既存 worker の意味論は不変。`ToolRuntime` への `degraded_names()` accessor 追加は additive。
- **Trigger**: T1 で既存 worker_validate golden が regression、T2 で非 opt-in の gate 挙動（`run_fake_stream_command_worker_denied_without_grants` 等）が regression、または T4 の fixture test が設計意図と異なる失敗を示す場合。
- **Procedure**: task 単位の coherent commit を `git revert`。**T1+T2 は T3 が land する前なら unit として revert 可能**（機構だけを戻しても参照する worker が存在しない）。**T3 以降に T1/T2 を revert する場合は `resources/workers/tool-operator.md`（と schema file、必要なら alias）の除去を必ず伴わせる** — `optionalTools` key が validate から消えた状態で worker file が残ると strict-key validation で built-in resource load が死ぬ。T5 → T4 → T3 → T2+T1 の逆順 unwind か fix-forward を選ぶ。
- **Verification**: revert 後に `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で workspace 全体が compile し、`mise exec -- cargo test -p cheaper-core -p cheaper-tools` と既存 dev-seam suite が pass すること。

## Task Breakdown

各 task は coherent な検証可能単位。task-doc 生成時は `depends-on` relation で下記 DAG を表現し、`verified-by` には `../test-specs/tool-operator-built-in-worker.md`（approved・plan 作成時点で存在済み）を記録する。

- T1 `optional-tools-core-machinery`: vocabulary.rs に `ToolName::resolve`（canonical → `tool_alias` alias table の順の single-sourced resolver）と `is_scope_gated_tool`（`Process` / `Command` の一権威 predicate）を追加し、`normalize_tool_names` の per-element path を `resolve` 呼び出しへ refactor。validate.rs の `CHEAPER_KEYS` に `"optionalTools"`、`WorkerCheaper` に `optional_tools: Vec<ToolName>`（未指定時は空 vec）を追加し、design §(b) の 7 rule を全て `InvalidWorkerDefinition`（`invalid()` 経由 — `tools` field の `UnknownTool` code は使わない）で実装 + unit test。`ToolRuntimeOptions.degradable` field 追加 + design §(i)/(j) 列挙の全 literal site 更新 + invocation.rs pass-through + run/mod.rs 配線（`worker.cheaper.optional_tools.clone()`）+ workflow_run.rs 空 vec + `WorkerCheaper` literal 2 箇所（validate.rs:250・workflow tests common/mod.rs:109）+ golden `dump_worker_def` の conditional emit。**T1 完了時点で `cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` green**。depends-on: なし。
- T2 `registry-degrade-and-prompt-surfacing`: registry.rs で effective names を冒頭決定し、process/command の empty content-gate 不成立時のみ `options.degradable` membership で縮退（それ以外の gate — `resolve_*_policy` error・pin 失敗・writeScope × `command` — は hard `POLICY_VIOLATION` のまま）。`options.degradable` の `is_scope_gated_tool` clamp + zero-effective-names guard。`degraded` を `options.names` 宣言順に整列し `RuntimeInner.degraded` + `ToolRuntime::degraded_names()` を追加、pin 判定・`ScopeAccessOptions.names`・registered loop・`RuntimeInner.names` の全 consumer を effective names へ。run/mod.rs で `tool_names` を `scope.definitions()` から組み立て `unavailable_tools` を prompt へ渡し、`tools.degraded` span field を sync 区間（create_worker_scope return 後・instrument 前）で record。agent.rs の `build_system_prompt` 第 4 引数 + 利用不可注記 + `status` enum 条件付き blocked 誘導行 + unit test。depends-on: T1。
- T3 `tool-operator-worker-and-alias`: `resources/workers/tool-operator.md`（design §(a) frontmatter + 英語 body — gated tool 言及は条件付き表現、evidence field 名・blocked semantics・partial/failed 必須 field mapping・caps・honest mutations）と schemas/tool-operator.{input,output}.schema.json（design §(h) verbatim shape、strict-mode-safe）を追加。args.rs alias table + error enumeration、cli.rs HELP、tests/args.rs alias + positional reject test。加えて `tests/common::builtin_dir()` 経由で `resources/workers/*.md` 全件を loop し `resolve_worker` が各 builtin worker を parse + input/output schema compile できることを assert する cli test（`tool-operator` を含む builtin class 全体を pin し、T3→T1 依存 — `optionalTools` key 未実装だと strict-key reject で load 不能 — を機械的に enforce する）。depends-on: T1, T2。
- T4 `fixture-dsl-and-verification`: approved test-spec `docs/test-specs/tool-operator-built-in-worker.md`（plan 作成時点で存在済み — T4 で作成・変更しない）が pin する行為保証（非 opt-in hard 維持・degraded tool 未登録・UNKNOWN_TOOL・admission vs content-gate 分離）を検証する test を実装する。fixture.rs に `FixtureWorker.optional_tools`（snake_case DSL key）+ `to_markdown()` の `optionalTools:` emit を先行実装し、run_dev_seam.rs に degraded run・grant-env run（`command_user_policy` + `FIXTURE_COMMAND_CALL`）・capability-denied run・mixed-policy 片側縮退・scripted degraded-name call → `UNKNOWN_TOOL`・scripted undeclared `write`/`edit` call の reject・`status` 各値の出力・`meta.changes` 空・`directlyComparable=false`・usage record の fixture test を追加。runtime `FakeSessionFactory` で system prompt の実登録集合 + 利用不可注記 + conditional blocked 分岐を assert。regression corpus case + expected（expected は `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression` で生成し diff review する）、worker_validate golden accept/reject fixture。depends-on: T1, T2, T3。
- T5 `docs-and-apm`: README built-in workers 表、workers.md（表 + 入力 schema 要点 + `cheaper.optionalTools` frontmatter key リファレンス + 縮退挙動 + blocked-status 推奨）、usage.md（コマンド構成の alias 行・alias 列挙文・per-alias 使用例 + `directlyComparable=false`）、command.md（opt-in caveat + admission/content-gate 棲み分け）を更新し、`packages/apm/.apm/skills/cheaper-tool-operate/` を parity で追加、cheaper-usage SKILL.md の command list + delegation guidance を更新、実装完了時に impl record（`docs/impl/ir/`）を生成。depends-on: T3, T4。

## Verification

canonical verification は `mise run check`（`biome check` → `cargo fmt --check` → `cargo clippy` → version/doc-id check → `cargo test --workspace` → `node --test` → lockfile check → payload check）。cargo 系は `mise exec --` 経由で `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付き。

| Task | Verification command or criteria | Pass condition | AC |
| --- | --- | --- | --- |
| T1 | `mise exec -- cargo test -p cheaper-core` + `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | `optionalTools` の 7 reject rule + accept + 未指定時の既存挙動不変の unit test が pass。**workspace 全体（runtime・cli・test target・dev-seam target 含む）が compile green** — compile-forced propagation 完遂の証明 | —（機構基盤） |
| T2 | `mise exec -- cargo test -p cheaper-tools -p cheaper-rig-adapter -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` + `mise exec -- cargo test -p cheaper-cli --test run_dev_seam --features cheaper-cli/dev-seam` | degrade path の unit test（effective names・`degraded_names()` 宣言順・zero-effective-names guard・非 opt-in / pin 失敗 / writeScope × command の hard 維持・clamp）+ `build_system_prompt` の注記 emit / blocked 誘導行 conditional test が pass。非 opt-in reject test（`run_fake_stream_command_worker_denied_without_grants` — `crates/cheaper-cli/tests/run_dev_seam.rs:294` にあり -p 3 crate では対象外のため cheaper-cli 側 command を併記）が無変更で green | AC4（挙動機構）, AC8 |
| T3 | `mise exec -- cargo test -p cheaper-cli --features cheaper-cli/dev-seam`（`--test args` の alias table + positional reject 含む） | `tool-operator` が worker discovery で load され input/output schema が解決、`tool-operate` が `run tool-operator` と等価に解約され extra positional を reject。`tests/common::builtin_dir()` 経由で `resources/workers/*.md` 全件を loop する cli test が `resolve_worker`（worker parse + input/output schema compile を含む）を全 builtin worker で成功させる — `tool-operator` を含む class 全体を pin し、`optionalTools` key 未実装（T1）では strict-key reject で fail するため T3→T1 ordering を機械的に enforce する | AC1 |
| T4 | `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` + `mise exec -- cargo test -p cheaper-cli --test regression --features cheaper-cli/dev-seam`（作成後の focused run — `regression` target は `required-features = ["dev-seam"]` で gate 済みのため flag 必須） | degraded run の model 到達・grant-env の実 spawn・capability-denied の run 全体 fail・mixed-policy の片側のみ列挙・`UNKNOWN_TOOL`・blocked status・prompt 注記 assert・`meta.changes` 空・`directlyComparable=false`・usage record・golden/regression が全て pass。AC 別の concrete coverage: AC2 は fake stream が `status: "success"` + `findings[].evidence` 付き最終 JSON を返す fixture test で envelope `result` の output-schema 適合を assert、AC3 は `status: "partial"` / `"failed"` の scripted 最終 JSON で `unresolved` + `nextActions` populated を assert、AC5 は巨大中間 tool 出力を発生させる scripted run で envelope `result` が中間生出力を含まず `output.maxTokens`（`policy.output.max_tokens` → `SessionSpec.max_output_tokens`）以内の bounded result であることを assert、AC6 は既存 generic regression case（`run-max-turns.json` / `run-max-tool-calls.json` / `run-timeout-delay.json` — worker 非依存の engine 経路で打ち切りを担保）を coverage として引用し縮退 run 固有の経路差分がある場合のみ tool-operator 向け case を追加、AC7 は scripted `write` / `edit`（undeclared）call が unknown-tool / undeclared 経路で拒否される fixture test | AC2–AC11 |
| T5 | doc audit + README / workers.md / command.md / usage.md / SKILL.md の記述と実装挙動の突合 | built-in workers 表・workers.md 入力 schema 要点・`optionalTools` key リファレンス・opt-in caveat・admission/content-gate 棲み分け・usage.md の alias 行/列挙文/使用例・`directlyComparable` 記述・`cheaper-tool-operate` skill・cheaper-usage command list が揃う | AC12 |
| 全 task | `mise run check` | pipeline 全段が exit 0 | AC13 |

design Verification Notes の未チェック項目は T4 の test checklist にそのまま引き継ぐ（`optionalTools` validate 規則の unit test、grant 定義環境の `command` 登録 + 実 spawn、`command` × writeScope の hard reject 維持、workflow `kind: worker` job の同一縮退挙動、`tools.degraded` span field は automated assert 対象外）。

## Open Questions / Gaps

- 未解決の設計判断はない（spec / design の Deferred Design Concerns は positional-sugar・per-worker limits・実 provider e2e の 3 点で、いずれも本 plan の Out of Scope として明記済み）。
- test-spec `docs/test-specs/tool-operator-built-in-worker.md`（TSPEC-034EUg38cXy0yzKhkggeIg）は approved で既に存在する — plan の `relations.verified-by` は設定済み。task doc 生成時にも各 doc の `verified-by` へ同 path を記録する。
- `tools.degraded` span field は automated assert 対象外（design Risks の accepted trade-off — `--log-format json` file sink 経由なら assert は可能だが、`degraded_names()` unit test + prompt 注記固定による低コストな代替検証を選ぶ決定。Risk #5 参照）。
- `docs/guides/configuration.md` は原則無変更の想定（worker frontmatter key の記述を持たない）。T5 で交差記述を発見した場合のみ最小追記とし、差分が設計外であれば review 対象として報告する。

## Self-Review

- [x] spec の AC1–AC13 が T1–T5 と Verification matrix に対応づいている（AC1→T3/T4、AC2/AC3→T4、AC4→T2/T4、AC5/AC6→T4、AC7→T3/T4、AC8→T2/T4、AC9/AC10→T4、AC11→T4、AC12→T5、AC13→全 task）。
- [x] ADR の副次決定（admission vs content-gate の 2 層分離、opt-in のみ・capability-shrinking 専用、envelope `meta` shape 不変、span field + prompt 注記のみの observability）が task に反映されている。
- [x] placeholder（TBD/TODO/implement later）はない。
- [x] file path・relation key・status・command・用語は design §(i) と整合している。

## Implementation Handoff

- Recommended mode: inline または delegated/subagent-capable implementation。critical path が T1 → T2 → T3 → T4 → T5 の直列で独立 stream を持たないため、逐次実行を既定とする。
- Review checkpoints: T1 完了時（workspace green + validate rule coverage）、T2 完了時（hard gate 維持の証明 + prompt 文言固定）、T4 完了時（AC coverage）で区切る。
- User approval required before dispatch: yes。
- 実行者は task ごとに上記 verification matrix の focused command を実行し、全 task 完了後に `mise run check` を通す。

## Review Handoff

- 重点 review 対象: T1 の `optionalTools` validate rule 集合と `is_scope_gated_tool` の一権威性（gated 集合の二重定義を生まないこと）、T2 の degrade path（hard のまま残す gate の網羅: pin 失敗・writeScope × command・`tools.allow` admission）と `tools.degraded` record 位置（sync 区間規約）、T3 の worker body の条件付き gated-tool 言及、T4 の regression harness clobbering 対策（DSL 経由必須）。
- 上流 doc: spec `docs/specs/tool-operator-built-in-worker.md`、design `docs/designs/tool-operator-built-in-worker.md`、ADR `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`。
- 明示的な前提: 各 task 境界で workspace は green を維持する（T1 が compile-forced propagation を同梱所有する #101 precedent）。`optionalTools` は strictly capability-shrinking であり、worker origin（built-in / project / user / `--worker-file`）を問わず権限拡大には使えない。

---
id: "TSPEC-034EUg38cXy0yzKhkggeIg"
type: "test-spec"
status: "approved"
title: "tool-operator built-in worker と cheaper.optionalTools 縮退機構の行為保証"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/102"
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
  related:
    - "../designs/tool-operator-built-in-worker.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-tool-operator-built-in-worker.md"
    - "../specs/tool-operator-built-in-worker.md"
  derived-by: []
  verifies:
    - "../specs/tool-operator-built-in-worker.md"
    - "../designs/tool-operator-built-in-worker.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# tool-operator built-in worker と cheaper.optionalTools 縮退機構の行為保証

## Purpose

issue #102 で追加される built-in worker `tool-operator`（CLI alias `tool-operate`）と、その基盤となる `cheaper.optionalTools` opt-in 縮退機構の行為保証を固定する。保護する intent は「`process` / `command` の content-gate（nonempty `tasks` / `grants`）不成立を、worker が frontmatter で opt-in した tool だけが『未登録への縮退』として表現し、admission（`tools.allow` / `check_capabilities`）と call 経路の認可機構（grant 解決・argv pattern・executable pin/検証）は一切緩めない」という一点にある。緩和は登録面を縮めるだけで広げず、非 opt-in worker の fail-closed 意味論は不変でなければならない。`tool-operator` が廃止されるか、`cheaper.optionalTools` 機構の外部 contract（ADR-034ESBcMTWxNCMDI9obdb8 の Option C1）が改訂されたときにのみ退役可能。

## Feature

Feature: `cheaper.optionalTools` で opt-in 宣言した scope-gated tool は content-gate 不成立時に未登録へ縮退し、`tool-operator` は利用可能な tool 集合で run を完走するか理由付き `blocked` を返す — run 全体が `POLICY_VIOLATION` で終了するのは admission 拒否（`tools.allow` による宣言 tool 除外）と opt-in 対象外の gate 失敗だけである。

## Rules

- Rule: `tool-operator` worker 定義は builtin resource として load 可能で、frontmatter は `tools: [read, grep, find, ls, count_tokens, process, command]` + `cheaper.optionalTools: [process, command]` + `cheaper.model.tier: low` + `workload.outputPerInput: 0.3` を宣言し、`writeScope` を持たず、参照する input/output JSON Schema が compile される（AC1 前提）。対象: `crates/cheaper-cli` の builtin resource 経由 worker resolution（`tests/common::builtin_dir` + `Fixture` harness の named worker run）と cheaper-core `worker_validate` golden の accept fixture（`tests/golden/worker_validate.rs` + `fixtures/worker/validate.json`）。
- Rule: `cheaper tool-operate` は `cheaper run tool-operator` と等価に解約され、extra positional を受理しない（canonical invocation は `--input <file>` または stdin JSON）（AC1）。対象: `crates/cheaper-cli/tests/args.rs` の `run_alias` table（`("tool-operate", "tool-operator")`）+ positional reject の既存 pattern。
- Rule: builtin default policy（`process.tasks: {}`・`command.grants: {}`）で opt-in worker は scope 構築を `POLICY_VIOLATION` 無しに通過し、`process` / `command` を除く実登録集合で model session まで到達して run が完走する。`ToolRuntime::degraded_names()` は `options.names` 宣言順で縮退名を報告する（AC4・AC8）。対象: `crates/cheaper-cli/tests/run_dev_seam.rs` の degraded run fixture test（`FixtureWorker.optionalTools` DSL 経由）+ `crates/cheaper-tools` registry gate の unit test（effective names・宣言順 `degraded_names()`・zero-effective-names guard・`degradable` clamp）。
- Rule: `tasks` / `grants` が定義された環境では opt-in worker の `process` / `command` が通常通り登録・実行される — 縮退は content-gate 不成立時のみで、認可済み環境の挙動は非 opt-in と同一（AC4・AC8 の逆側）。対象: `run_dev_seam.rs` の `command_user_policy` + `FIXTURE_COMMAND_CALL` pattern による実 spawn test（marker file 到達で spawn と grant env 伝播を証明）。
- Rule: `tools.allow` から宣言 tool が除外された policy では opt-in worker も `check_capabilities` で run-level `POLICY_VIOLATION`（`missing` details 付き）となる — `optionalTools` は admission を免除しない。admission と content-gate の 2 層分離は invariant（AC4）。対象: `run_dev_seam.rs` の capability-denied fixture test。
- Rule: 縮退は per-tool — `process.tasks` 非空 × `command.grants` 空、およびその逆の mixed policy では failed-gate の tool のみが縮退し、もう片方は gate 通過込みで登録される。利用不可注記は縮退名だけを正確に列挙する（AC4）。対象: `run_dev_seam.rs` の mixed-policy fixture test + cheaper-runtime 層の prompt 注記 assert。
- Rule: 縮退した tool は registry に登録されず実行経路を持たない — model が名前で呼んでも対応 `ToolResult` の `output.error` が `UNKNOWN_TOOL`（"unknown or unregistered tool"）で安全に失敗する（AC8）。対象: fake seam の scripted `ToolCalls{name:"command"}` step（`FixtureStep` → `ScriptStep::ToolCalls` → `ToolRuntime::execute` の実経路）への `UNKNOWN_TOOL` assert（`run_dev_seam.rs`）。
- Rule: 非 opt-in worker（`optionalTools` 未指定）の fail-closed gate は不変 — `tools: [command]` worker は grants 不在で引き続き `POLICY_VIOLATION`（exit 4、"nonempty authorized grants"）で fail する（後方互換・AC8）。対象: 既存 test `run_fake_stream_command_worker_denied_without_grants`（無変更 green のまま維持）。
- Rule: 縮退 run の system prompt は "Available tools" を実登録集合（`scope.definitions()`）から組み立て、宣言済みだが利用不可の tool 名列挙行（`- Declared but unavailable under current policy: ...`）を載せる。blocked 誘導行は run の output schema の `status` enum に `"blocked"` を含む場合のみ emit される — schema が受理しない終了状態を prompt が促さない（AC4）。対象: `crates/cheaper-runtime` test 層（`FakeSessionFactory` handle 経由で prompt を捕捉 — cli 層ではない）+ `crates/cheaper-rig-adapter` `build_system_prompt` unit test（文言と conditional 分岐を固定）。
- Rule: `cheaper.optionalTools` の validate は `optionalTools-invalid`（非配列・非 string 要素）/ `optionalTools-empty` / `optionalTools-unknown-tool` / `optionalTools-undeclared` / `optionalTools-not-gated` / `optionalTools-duplicate` / `optionalTools-not-proper-subset` の 7 reject reason を全て `InvalidWorkerDefinition`（`invalid()` 経由 — `UnknownTool` code は使わない）で返し、未指定 worker の既存挙動は不変。対象: `crates/cheaper-core` `src/worker/validate.rs` unit test + `worker_validate` golden fixture の reject/accept case（accept は `dump_worker_def` の conditional emit で `cheaper.optionalTools` の正規化結果を比較）。
- Rule: pin 失敗・`resolve_*_policy` error・`command` × writeScope 不併存は opt-in tool に対しても hard `POLICY_VIOLATION` のまま — 縮退対象は empty `tasks` / `grants` の content-gate 不成立のみ（`command` が空 grants で縮退した scope では writeScope check は不発 — ADR の accepted 挙動）。対象: `crates/cheaper-tools` registry gate test（hard 維持の reject case）。
- Rule: workflow `kind: worker` job が opt-in worker を実行する経路で縮退は run と同一挙動（`kind: tool` job は frontmatter を持たず `degradable` 空 — opt-in 対象外）。対象: `crates/cheaper-cli` workflow test 経路（`workflow_dev_seam.rs` 系）。
- Rule: output contract — `status` enum の 4 値（`success` / `partial` / `failed` / `blocked`）はそれぞれ宣言 output schema 適合で表現可能。非 success（`partial` / `failed` / `blocked`）では `unresolved` と `nextActions` が populated で `summary` は空にしない。`findings[].evidence` は required で `kind` discriminator（`location` / `commit` / `exitCode` / `truncation` / `note`）の単一 object shape に従い tool 生出力を含まない。`blocked` は成功 run envelope の `result.status == "blocked"` であり run-level error envelope とは別物（AC2・AC3・AC4）。対象: `run_dev_seam.rs` で fake stream が各 status の最終 JSON を返す fixture test。
- Rule: 巨大な中間 tool 出力を発生させる run でも、親へ返る envelope に中間出力が含まれず、`result` は宣言 output schema 適合で `output.maxTokens` 以内に収まる — truncation の事実は evidence としてのみ記録される（AC5）。対象: `run_dev_seam.rs` fixture test。
- Rule: `execution.maxTurns` / `maxToolCalls` / `timeout` / `output.maxTokens` / command 出力 cap が遵守され、超過時は規定の打ち切り挙動に従う — tool-operator 固有の per-worker override は存在しない（AC6）。対象: 既存 generic 打ち切り regression case（`tests/regression/cases/run-max-turns.json`・`run-max-tool-calls.json`・`run-timeout-delay.json` — worker 非依存の engine 経路で担保）との parity で網羅し、縮退 run 固有の経路差分を固定する場合のみ tool-operator 向け新規 case を追加する。
- Rule: `write` / `edit` は宣言されず undeclared tool として到達不能 — scripted call は unknown-tool / invalid-call 経路で拒否され、worker 契約上の書き込み boundary は存在しない（AC7）。対象: `run_dev_seam.rs` の scripted undeclared-name call fixture test。
- Rule: `meta.changes` は常に空（write/edit 非宣言 + `command` / `process` 子 process 副作用を audited change として主張しない honest reporting）。`savings.directlyComparable` は宣言 tools を見る保守的判定で常に `false`（縮退の有無で変えない）。run record は usage.jsonl に `kind:"run"` で記録され `cheaper usage` で集計可能（AC9・AC10）。対象: `run_dev_seam.rs` の envelope assert（`meta.changes.files`・`meta.savings.directlyComparable`）+ `Fixture::usage_log()` assert。
- Rule: issue #102 の test case は dev-seam fixture（degraded / granted / capability-denied / mixed / scripted `UNKNOWN_TOOL` / blocked status）と regression corpus case でカバーされる（AC11）。対象: `crates/cheaper-cli/tests/regression/cases/` + `expected/` への縮退 run case 追加 — expected は `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression` で生成する（harness が `worker_markdown()` を無条件 seed するため、opt-in worker は `FixtureWorker.optionalTools` DSL 経由でのみ表現可能 — file seed は clobber される）。

## Examples

- Example: builtin default policy で stdin JSON `{"task":"..."}` 付き `cheaper tool-operate` を実行すると、run は `POLICY_VIOLATION` 無しに完走し、`process` / `command` は model の tool 一覧に存在しない。
- Example: user-scope `policy.yaml` に `command.grants.self` を定義した環境で opt-in worker を run すると `command` が登録され、scripted `command` call が実 subprocess を spawn して workspace 内 marker を書く（`command_user_policy` + `FIXTURE_COMMAND_CALL`）。
- Example: `tools.allow` から `read` を外した policy では opt-in worker も `missing` details 付き capability `POLICY_VIOLATION` で run 全体が fail する — `optionalTools` は admission を免除しない。
- Example: `process.tasks` 非空・`command.grants` 空の mixed policy では `command` のみ縮退し `process` は登録される — 利用不可注記は `command` だけを列挙する。
- Example: degraded run に scripted `ToolCalls{name:"command"}` step を置くと、対応 `ToolResult` の error が `UNKNOWN_TOOL` を含む — 縮退 tool への実行経路が存在しないことの直接証明。
- Example: fake stream が `status: "blocked"` を返す degraded run では、output schema 適合で envelope `result.status == "blocked"` となる — run-level `POLICY_VIOLATION` の error envelope とは区別される。
- Example: `status` enum に `"blocked"` を持たない output schema の opt-in fixture worker では利用不可列挙行のみ emit され blocked 誘導行は出ない（`FakeSessionFactory` で conditional 分岐を固定）。
- Example: `optionalTools: [read]`（非 gated tool）は `optionalTools-not-gated`、`optionalTools` と `tools` が一致する宣言は `optionalTools-not-proper-subset` で `InvalidWorkerDefinition`。
- Example: `tools: [command]` のみの非 opt-in worker は grants 不在で従来通り `POLICY_VIOLATION`（exit 4、"nonempty authorized grants"）。

## Guarantees

- `process` / `command` が `tools.allow` で許可されていても `tasks` / `grants` が空の環境で、opt-in worker が model session まで到達し run が完走する（degraded run）。
- 縮退は登録面を縮めるだけで広げない — 縮退した tool への実行経路は存在せず、call 経路の認可機構（grant 解決・argv pattern・executable pin/検証）は不変。
- `tools.allow`（admission）と content-gate（認可済み content の有無）の 2 層が分離したまま保たれる — `optionalTools` は `check_capabilities` を迂回しない。
- `optionalTools` 未指定の既存 worker の意味論は完全に不変（非 opt-in の `tools: [command]` worker は引き続き fail-closed）。
- 縮退した tool 名は system prompt 注記と `cheaper.run.tool_scope` span の `tools.degraded` field で観測可能 — envelope `meta` からは読めない（accepted limitation。span field は `.cheaper/logs` JSON sink 経路での assert が可能だが本 scope では対象外とし、`degraded_names()` unit test + prompt 注記の内容固定で代替検証する）。
- output contract として `status` 4 値は全て表現可能で、非 success では `unresolved` / `nextActions` が required、evidence は `kind` discriminator で表現され tool 生出力を含まず、envelope は bounded（`output.maxTokens` 以内）。
- 計測面の honest reporting — `meta.changes` は常に空、`savings.directlyComparable` は常に `false`、run record は usage.jsonl に記録される。

## Non-goals

- `cheaper tool-operate "<task>"` 形の positional-sugar（canonical `--input` / stdin 経路で要件を満たす — parser 規約ごと別 spec の deferred concern）。
- per-worker execution 制限 override 機構（global `maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000` に従う — 機構自体が plan の Out of Scope）。
- `ollama_e2e.rs` 実 provider e2e 検証（本 iteration 対象外 — dev-seam fixture + regression corpus が primary 検証）。
- `tools.degraded` span field の自動 assert（`.cheaper/logs` JSON sink 経路では可能だが本 scope では対象外 — `degraded_names()` accessor unit test + prompt 注記で代替）。
- pin 失敗・`resolve_*_policy` error・writeScope × `command` reject の soft 化（hard のまま — soft 化されないことを Rules で保証する側）。
- 公開面 docs・APM skill の記述内容（AC12 — plan T5 の doc audit が担う。行為保証の対象外）。
- `mise run check` pipeline 通過そのもの（AC13 — task の runnable Verification が担う CI contract であり、本 test-spec の行為保証ではない）。
- workflow `kind: tool` job の resource inference 詳細（`degradable` 空の generic tool 分岐で担保 — 既存 test-spec の責務）。

## Risk

- 縮退機構が落ちると、builtin default install（`tasks` / `grants` が空）では `tool-operator` が必ず scope 構築で死に、worker が model 未到達の dead feature 化する。
- 逆に gate を広く緩めた場合は未認可の `process` / `command` call 経路が生まれ、trusted grant 信頼境界（ADR-034EMgaatFhsXjR97kH14c）が崩れる — workspace policy file を編集できる actor が任意 executable を起動し得る。
- `tools.allow` 免除が混入すると admin の明示的 admission 境界が run 成功で覆される（capability-shrinking 専用という設計意図の喪失）。
- output contract や bounded envelope の保証が落ちると、巨大な中間 tool 出力が親 context に流れ込み、cost offload という worker の存在意義が失われる。

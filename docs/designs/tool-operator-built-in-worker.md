---
id: "DESIGN-034ETcI8q4hTRH502qnQZc"
type: "design"
status: "approved"
title: "tool-operator built-in worker と cheaper.optionalTools opt-in 縮退機構の設計"
created: "2026-10-12"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "../specs/policy-gated-command-tool.md"
    - "../designs/policy-gated-command-tool.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/tool-operator-built-in-worker.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "../discovery/tool-operator-worker-built-in.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../guides/workers.md"
    - "../guides/command.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# tool-operator built-in worker と cheaper.optionalTools opt-in 縮退機構の設計

## Context

issue #102 の built-in Worker `tool-operator`（CLI alias `tool-operate`）と、それを可能にする core 機構 `cheaper.optionalTools`（ADR-034ESBcMTWxNCMDI9obdb8 で確定した Option C1）の実装設計。spec `../specs/tool-operator-built-in-worker.md` が要件・acceptance criteria・pinned decision を、ADR `../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md` が per-tool opt-in 縮退機構の外部挙動と touch point を確定済みであり、本 design は両者が defer した残りの判断 —— worker file と JSON Schema の具体 shape、`optionalTools` の validate rule と error reason、scope 構築の縮退経路、prompt surfacing、file-level change map —— を確定する。

現行の scope 構築は fail-closed である。`create_tool_runtime`（`crates/cheaper-tools/src/registry.rs:1066-1095`）は `names` に `Process` を含み `process.tasks` が空なら `POLICY_VIOLATION` "process exposure requires nonempty authorized tasks"、`Command` を含み `command.grants` が空なら同じく `POLICY_VIOLATION` "command exposure requires nonempty authorized grants" で scope 生成を拒否する。builtin `policy.yaml` は `process.tasks: {}`・`command.grants: {}` で出荷されるため、この gate を素通しでは `tool-operator` は default install で必ず scope 構築で死ぬ。本 design が緩めるのは「worker が frontmatter で opt-in した gated tool だけを、content-gate（empty tasks / grants）不成立時に未登録へ縮退する」一点のみであり、`tools.allow` による admission（`check_capabilities`、`crates/cheaper-runtime/src/run/boundary.rs:100-101`、scope 構築より先に評価）と call 経路の認可機構（grant 解決・argv pattern・executable pin/検証）は一切変えない。

## Scope

### In Scope

- built-in worker 定義 `crates/cheaper-cli/resources/workers/tool-operator.md`（frontmatter + 英語 prompt body）と input/output JSON Schema（`resources/workers/schemas/tool-operator.{input,output}.schema.json`）。
- worker frontmatter 新規 strict key `cheaper.optionalTools` の parse・validate（cheaper-core `worker/validate.rs`）と scope-gated 集合の一権威 predicate `is_scope_gated_tool`（`tools/vocabulary.rs`）。
- 縮退許可集合の propagation（`ToolRuntimeOptions.degradable` → `create_worker_scope` pass-through → `create_tool_runtime`）と registry degrade path、縮退名の `ToolRuntime` 報告。
- run 層の surfacing: `build_system_prompt` の "Available tools" を実登録集合から組み立て、宣言済みだが利用不可の tool を prompt 注記と `cheaper.run.tool_scope` span field に載せる（`run/mod.rs` + rig-adapter `agent.rs`）。
- CLI alias `tool-operate`（`args.rs`・`cli.rs` HELP・error message enumeration・test）。
- 検証面: validate unit test、registry gate test、dev-seam fixture（degraded / granted / capability-denied）、regression corpus case、args test。
- 公開面 docs: README built-in workers 表、`docs/guides/workers.md`、`docs/guides/usage.md`、`docs/guides/command.md`、APM skill `cheaper-tool-operate`。

### Out of Scope

- worker 内からの再帰的 worker delegation、独自 agent loop・追加 agent engine、`write` / `edit` capability（writer 系 worker の scope を侵食しない）。
- per-worker execution 制限 override（global policy 制限 `maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000` に従う）。
- `ollama_e2e.rs` の実 provider 検証（本 iteration 対象外。primary 検証は dev-seam fixture + regression corpus）。
- `cheaper tool-operate "<task>"` 形の positional-sugar（canonical invocation は `--input` / stdin JSON のみ。parser 規約ごと別 spec）。
- policy pin 失敗・`resolve_command_policy` / `resolve_process_policy` error・writeScope × `command` reject の soft 化（opt-in tool に対しても hard `POLICY_VIOLATION` のまま）。
- envelope `meta` の shape 変更（縮退 tool 名は span field と system prompt 注記のみに記録する）。
- builtin `policy.yaml` への非空 `tasks` / `grants` 同梱（Option D — 常時認可面を作る security regression であり採用しない）。

## Components and Boundaries

### (a) worker 定義 file: `resources/workers/tool-operator.md`

frontmatter（既存 worker と同じ strict-keys 構造。`writeScope` は宣言しない — write/edit を持たず書き込み boundary が存在しないため）:

```yaml
---
name: tool-operator
description: Iterate tool calls toward an operational goal and return a compact evidence-backed result.
tools:
  - read
  - grep
  - find
  - ls
  - count_tokens
  - process
  - command
cheaper:
  optionalTools:
    - process
    - command
  model:
    tier: low
    workload:
      # findings + evidence + unresolved + nextActions を返す中間的
      # workload(bulk-reader 0.05 と writer 系 1.0 の間、code-writer
      # と同じ 0.3)
      outputPerInput: 0.3
  input:
    schema:
      file: ./schemas/tool-operator.input.schema.json
  output:
    schema:
      file: ./schemas/tool-operator.output.schema.json
---
```

body は既存 3 worker（`bulk-reader.md`・`code-writer.md`・`doc-writer.md`）と同じく **英語の system prompt** とする（worker file の言語規約は英語 — persona/prompt は英語使用可の AGENTS 規約に従う）。構成は既存 file の `## Procedure` / `## Output rules` / `## Prohibitions` 形を踏襲し、次をカバーする:

- **role**: "an operation actor" — `task` で与えられた目標に向かって tool call を反復し（operation 実行 → 結果の解釈 → 次の operation → 圧縮された最終結果の返却）、親へ compact structured result だけを返す。
- **tool usage discipline**: 利用可能な tool のみを使い、read/grep は bulk-reader 流の狭い window で呼ぶ。`process` / `command` は "Available tools" に列挙されている場合に限り使用する（利用不可注記が出ている環境では呼ばない — gated tool の言及は常にこの条件付き表現とし、prompt が degraded 注記と矛盾しないようにする）。呼べる場合でも `process` は declared task 名、`command` は authorized grant 名のみに限定される（definition の description に一覧が埋め込まれる）。独立した call は same-turn batching の contract に従う。
- **evidence requirements**: 各 `findings[]` 要素は `evidence[]` を必須とし、output schema の `kind` discriminator と payload field 名（`location`（`path` + `lines`）・`commit`・`exitCode`・`truncation`（`truncated` flag）・`note`（`detail`））で表現する。tool 出力の生テキストを evidence に含めない。
- **never dump large output**: 巨大な中間 tool 出力を `summary` / `detail` / `findings` へ転記しない。grep は narrow pattern + context window、`read` は offset/limit window、`command` は出力 cap で truncated になり得ることを前提に、truncation が起きたらその事実を evidence として記録する。
- **blocked semantics**: 目標が利用不可の capability（利用不可注記の `process` / `command` 等）なしには開始・継続できない場合、`status: "blocked"` で終了し、欠けた capability を `unresolved` と `nextActions` に記録する。権限昇格の試行・別 tool での迂回は禁止。
- **partial/failed 終了時の必須 field mapping**: `status: "partial"` / `"failed"` で終了する場合も schema required の `unresolved`（未達成・未検証の残項目）と `nextActions`（親 agent が取り得る次の操作）を必ず populated にし、`summary` は空にしない — status ごとの field 対応を prompt で固定し、required field の空配列・空文字逃げを防ぐ。
- **respect caps**: turn / tool call / 時間 / 出力の制限内で動き、上限を使い切る前に未達成分を `unresolved` へ畳み込んで終了する。
- **honesty about mutations**: `process` / `command` の子 process 副作用は runtime が監査しないため、「監査済みの file 変更」として報告しない。実行した操作と観測した結果（exitCode 等）を事実として `findings` / `summary` に記録するに留める。

### (b) frontmatter contract: `cheaper.optionalTools` の parse と validate

`crates/cheaper-core/src/worker/validate.rs` の変更:

- `CHEAPER_KEYS`（validate.rs:94）に `"optionalTools"` を追加し、`WorkerCheaper` に `optional_tools: Vec<ToolName>`（未指定時は空 vec）を追加する。`WorkerCheaper` は struct literal 構築のため新 field は compile-forced — 構築 site は `validate.rs:250`（validate 本体）と `cheaper-workflow/tests/common/mod.rs:109`（fixture）の 2 箇所のみ（(i)/(j) 参照）。
- 値は `string[]` を期待する。各要素の名解決には新規 `pub fn ToolName::resolve(name: &str) -> Option<ToolName>`（vocabulary.rs — canonical 名 → `tool_alias` alias table の順の single-sourced resolver）を使う。現状 `tool_alias`（vocabulary.rs:81-86）は private で validate 層から参照できないため、`from_canonical().or_else(tool_alias)` の組み立て順を vocabulary 内に集約し、`normalize_tool_names` の per-element path（vocabulary.rs:111）も `resolve` 呼び出しへ refactor する（`tools` と `optionalTools` で名解決規則が drift しない一権威化）。`resolve` で `None` となる名を含め、次を全て `InvalidWorkerDefinition`（`invalid()` 経由 — `tools` field の `UnknownTool` code は使わない。ADR が `optionalTools` の全 reject を `InvalidWorkerDefinition` で固定）で検査する:

| 条件 | details.reason |
| --- | --- |
| 配列でない / 非 string 要素 | `optionalTools-invalid` |
| 空配列 | `optionalTools-empty` |
| canonical 名にも alias にも解決できない名 | `optionalTools-unknown-tool` |
| `tools` に宣言されていない名 | `optionalTools-undeclared` |
| scope-gated tool でない名（後述 predicate） | `optionalTools-not-gated` |
| 正規化後の重複 | `optionalTools-duplicate` |
| `optionalTools ⊇ tools`（全 tool が縮退し得る = tool 0 個の run を許す nonsense 宣言） | `optionalTools-not-proper-subset` |

- 既存 worker（`optionalTools` 未指定）の挙動は不変 — `optional_tools` は空で、degrade path は発火しない。

### (c) `is_scope_gated_tool` predicate（vocabulary.rs）

`crates/cheaper-core/src/tools/vocabulary.rs` に追加する一権威 predicate:

```rust
/// scope 構築時に content-gate(nonempty tasks/grants)を持つ tool の
/// 一権威集合。`cheaper.optionalTools` の validate 検査が使う
/// (registry 層は直接 caller への防御的 clamp でのみ参照し得る —
/// (e) 参照)。
pub fn is_scope_gated_tool(name: ToolName) -> bool {
    matches!(name, ToolName::Process | ToolName::Command)
}
```

この predicate を実装上使うのは validate 層（(b) の `optionalTools-not-gated` 検査）のみである。registry 層（(e)）は既存の empty-tasks / empty-grants 分岐の内側で `options.degradable` の membership のみを検査し、gated 集合の再判定はしない — worker 経路では validate 済みの `degradable` が既に gated 集合へ限定されているため、分岐固有の条件（`names.contains` + 空 tasks/grants）自体が gated 判定を兼ねる。validate を bypass する直接 caller（`create_tool_runtime` を手で呼ぶ経路）への防御として、`create_tool_runtime` 冒頭で `options.degradable.retain(|n| is_scope_gated_tool(*n))` 相当の clamp を置くことは許容する — その場合に限り registry 層も predicate を参照し、gated 集合の二重定義は依然として発生しない。

### (d) 縮退許可集合の propagation

- `ToolRuntimeOptions`（registry.rs:90-123）に `degradable: Vec<ToolName>` を追加する。worker frontmatter の `optional_tools` が scope 構築時に届く唯一の経路であり、未指定の既存構築 site は全て空 vec を渡す。
- `WorkerScopeOptions`（invocation.rs:55-58）は `{owner_id, runtime: ToolRuntimeOptions}` の wrapper であり、`runtime` field 経由で新 field を運ぶため shape 変更は不要。`create_worker_scope` 内の `ToolRuntimeOptions` 再構成（invocation.rs:257-279）に `degradable: options.runtime.degradable` の pass-through を 1 行追加する。policy re-resolve parity check（process / command）は変更しない — `degradable` は policy ではなく scope 個別の option であり、`names` と同じ扱い。
- `InvocationToolRuntimeOptions`（invocation.rs:61-78）は変更しない。`names` を持たず per-scope の概念を保持しないため（`degradable` は `names` と同じく scope 単位の宣言）。
- `crates/cheaper-runtime/src/run/mod.rs` の `create_tool_scope`（:401-470、`ToolRuntimeOptions` literal は :427-459）で `degradable: worker.cheaper.optional_tools.clone()` を渡す。
- `crates/cheaper-cli/src/workflow_run.rs` の `ToolJobExecutor` scope 構築（`ToolRuntimeOptions` literal :462-482）は `degradable: Vec::new()` — `kind: tool` job は worker frontmatter を持たず opt-in できない。workflow の `kind: worker` job は `run_worker` 経路（`check_capabilities` → `create_tool_scope`）を通るため worker 定義の `optional_tools` が自動的に流れ、run と同一の縮退挙動となる。

### (e) registry degrade path（`create_tool_runtime`）

registry.rs:1063-1095 の gate 分岐（process gate :1066-1071、command gate :1075-1095 — writeScope reject 含む）を次の形に再構成する。`create_tool_runtime` の冒頭で有効 names を決定し、以降の全 consumer（pin 判定 :1139、`ScopeAccessOptions.names` :1174、registered loop :1246、`RuntimeInner.names` :1257）には有効 names を渡す:

```text
let mut names = options.names.clone();
let mut degraded: Vec<ToolName> = Vec::new();
// process gate(既存 :1066-1071 を拡張)
if names.contains(&Process) && process.tasks.is_empty() {
    if options.degradable.contains(&Process) {
        names.retain(|n| *n != Process); degraded.push(Process);
    } else {
        return Err(POLICY_VIOLATION "process exposure requires nonempty authorized tasks");
    }
}
// command gate(既存 :1075-1095 を拡張)
if names.contains(&Command) {
    if command.grants.is_empty() {
        if options.degradable.contains(&Command) {
            names.retain(|n| *n != Command); degraded.push(Command);
        } else {
            return Err(POLICY_VIOLATION "command exposure requires nonempty authorized grants");
        }
    } else if options.write_scope.is_some() {
        // grants 非空で command が登録される限り writeScope 不併存は
        // 従来通り hard(縮退対象外)。grants 空で縮退した場合は不発。
        return Err(POLICY_VIOLATION "command is incompatible with worker writeScope");
    }
}
```

- **hard のまま残るもの**: opt-in tool であっても縮退対象は empty `tasks` / `grants` の content-gate 不成立のみ。`resolve_*_policy` の error、pin 失敗（pin 不能・deny 該当・workspace-resident executable）、writeScope × `command` 不併存は全て従来通り `POLICY_VIOLATION`。
- **degraded の並び順**: `degraded` は gate 分岐の push 順（process → command 固定）ではなく `options.names` の宣言順に整列してから `RuntimeInner` へ格納する。`tools: [command, process]` のように宣言順が gate 順と逆の worker でも、prompt 注記と `tools.degraded` span field の出力が宣言順で stable になるようにするため。
- **defensive guard（直接 caller 向け・optional）**: validate を bypass する直接 caller 対策として 2 点を許容する。(1) `create_tool_runtime` 冒頭で `options.degradable` を `is_scope_gated_tool` で clamp（(c) 参照）。(2) degradation 後の有効 `names` が空になる場合は tool 0 個の runtime を生成せず `POLICY_VIOLATION` で fail する — worker 経路では (b) の `optionalTools-not-proper-subset` + `tools` non-empty rule が既に防ぐが、registry 直接呼出しでは fail-closed に倒す。
- 縮退は登録面を縮めるだけで広げない — `names` から除外された tool は `registered` に載らず、`ScopeAccessOptions.names` の capability 面にも残らないため、実行経路自体が存在しない。managed-subprocess tracking（`is_process` flag）は実登録集合に対してのみ効くため変更不要。
- 縮退名の報告: `RuntimeInner` に `degraded: Vec<ToolName>` を保持し、`ToolRuntime` に `pub fn degraded_names(&self) -> &[ToolName]` を追加する。run 層は「`effective_tools` と `definitions()` の差分」を再計算するのではなく、この accessor を一権威として使う（どの gate で落ちたかを呼出し側が推測しない）。

### (f) prompt surfacing: 実登録集合と利用不可注記

`crates/cheaper-runtime/src/run/mod.rs`（`create_session` :474-516）:

- `tool_names` を `self.effective_tools` ではなく **実登録集合** `scope.definitions().iter().map(|d| d.name.as_str().to_string())` から組み立てる（`CanonicalToolDefinition.name` は `ToolName`、`crates/cheaper-tools/src/definition.rs:38`）。縮退した tool は model の tool 一覧からも消えるため、model 側から見て degraded tool は存在しない。名前での call は既存の unknown-tool / invalid-call 経路（`invalid_tool_retries` budget 内）で安全に失敗する。
- 縮退名を `scope.degraded_names()` から取得し、`build_system_prompt` へ `unavailable_tools` として渡す。加えて `cheaper.run.tool_scope` span（:417）に `tools.degraded = tracing::field::Empty` field を宣言し、縮退名の join（`options.names` 宣言順 — (e)）を `record` する（payload ではなく分類値の metadata-only 規約に従う）。**record の位置**: `create_worker_scope` の return（:462、entered guard 保持中の sync 区間）と `scope.ready().instrument(tool_scope_stage)`（:465 — `instrument` が span を move する）の間で `tool_scope_stage.record("tools.degraded", …)` を呼ぶ。async ready 区間以後では record できず、guard drop 後の再 enter にも依存しない。

`crates/cheaper-rig-adapter/src/agent.rs` の `build_system_prompt`（:225-264）に第 4 引数 `unavailable_tools: &[String]` を追加する。非空時のみ、"Available tools" 行の直後に利用不可注記を emit する（文言を cheaper-runtime test で固定する）:

```text
- Declared but unavailable under current policy: {unavailable_tools.join(", ")}
```

- 列挙行は `unavailable_tools` 非空なら常に emit する（schema-neutral — どの output schema の opt-in worker とも矛盾しない）。
- blocked 誘導行は **条件付き emit** — run の output schema の `properties.status.enum` に `"blocked"` を含む場合のみ、列挙行の直後に次を追加する。`build_system_prompt` は既に `output_schema` を引数に持つため、enum membership は関数内で検査し追加引数は不要:

```text
- If the task requires an unavailable tool, do not call it; finish with status "blocked" and list the missing capability in unresolved and nextActions.
```

- `status` enum に `"blocked"` を持たない user-defined opt-in worker には blocked 誘導行を出さない — schema が受理しない終了状態を prompt が促す矛盾を防ぐ。
- 既存行・文言は変えない（TS parity 規約 — 新規行は本設計の Rust 側拡張として明示的に追加する）。
- `status: "blocked"` への言及は prompt hint であり enforcement ではない。`optionalTools` を宣言する worker は `blocked` 相当の終了状態を output schema で表現できることが期待される（`tool-operator` は enum に含め、blocked 誘導行が emit される）。docs で「opt-in worker は blocked 相当の status を schema に含めること」を推奨として明記する。

### (g) CLI alias `tool-operate`

- `crates/cheaper-cli/src/args.rs` の `alias_worker()`（:39-46）に `"tool-operate" => Some("tool-operator")` を追加。
- `crates/cheaper-cli/src/cli.rs` の HELP（:516-518）に `  tool-operate           Alias for: run tool-operator` を追加。
- `args.rs:483` の `command required: run | bulk-read | code-write | doc-write | ...` enumeration に `tool-operate` を追加。
- alias は既存 worker alias と同じ解約（`run` に畳まれ、extra positional は reject）。canonical invocation は `cheaper tool-operate --input <file>` または stdin JSON。
- `crates/cheaper-cli/tests/args.rs` の `run_alias` table（:77-79）に `("tool-operate", "tool-operator")` を追加し、positional reject を含む既存 pattern で test。

### (h) input / output JSON Schema の具体 shape

`resources/workers/schemas/tool-operator.input.schema.json`（draft 2020-12、strict-mode-safe: `additionalProperties: false` を全 object に付ける既存規約に従う）:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["task"],
  "properties": {
    "task": { "type": "string", "minLength": 1, "maxLength": 4000 },
    "context": {
      "type": "array", "maxItems": 30,
      "items": { "type": "string", "maxLength": 1000 }
    },
    "constraints": {
      "type": "array", "maxItems": 20,
      "items": { "type": "string", "maxLength": 300 }
    },
    "output": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "focus": {
          "type": "array", "maxItems": 10,
          "items": { "type": "string", "maxLength": 300 }
        },
        "maxFindings": { "type": "integer", "minimum": 1, "maximum": 30 }
      }
    }
  }
}
```

input schema は `$defs` を持たない — `$defs.path` はどの property からも `$ref` されない dead 定義のため置かない（workspace 相対 path 入力を受けないため `path` pattern 自体も不要）。`task` の `maxLength` は 4000 — `task` のみ必須で file target を持たない構成が bulk-reader と同型のため揃える（writer 系の 6000 は `targets` を併せ持つ worker の値）。

`resources/workers/schemas/tool-operator.output.schema.json` — spec の論理 field を bulk-reader schema の慣例（`$defs` + `additionalProperties: false` + maxLength/maxItems bounded）で具体化する。`evidence` は `kind` discriminator の単一 object shape とし（`anyOf` を避け strict-mode-safe を優先 — file:line / commitId / exitCode / truncation を optional payload field で表現）:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$defs": {
    "path": { "…input と同じ…" },
    "lines": { "…bulk-reader と同じ {start,end} integer ≥1…" },
    "evidence": {
      "type": "object",
      "additionalProperties": false,
      "required": ["kind"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["location", "commit", "exitCode", "truncation", "note"]
        },
        "path": { "$ref": "#/$defs/path" },
        "lines": { "$ref": "#/$defs/lines" },
        "commit": { "type": "string", "maxLength": 100 },
        "exitCode": { "type": "integer" },
        "truncated": { "type": "boolean" },
        "detail": { "type": "string", "maxLength": 600 }
      }
    }
  },
  "type": "object",
  "additionalProperties": false,
  "required": ["status", "summary", "findings", "unresolved", "nextActions"],
  "properties": {
    "status": {
      "type": "string",
      "enum": ["success", "partial", "failed", "blocked"]
    },
    "summary": { "type": "string", "minLength": 1, "maxLength": 1500 },
    "findings": {
      "type": "array",
      "maxItems": 30,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["title", "detail", "evidence"],
        "properties": {
          "title": { "type": "string", "maxLength": 120 },
          "detail": { "type": "string", "maxLength": 1200 },
          "evidence": {
            "type": "array", "maxItems": 10,
            "items": { "$ref": "#/$defs/evidence" }
          }
        }
      }
    },
    "evidence": {
      "type": "array", "maxItems": 30,
      "items": { "$ref": "#/$defs/evidence" }
    },
    "unresolved": {
      "type": "array", "maxItems": 20,
      "items": { "type": "string", "maxLength": 500 }
    },
    "nextActions": {
      "type": "array", "maxItems": 20,
      "items": { "type": "string", "maxLength": 500 }
    }
  }
}
```

- `status` 意味論: `success` = 目標達成 + evidence 完備、`partial` = 一部達成 + 未達成分を `unresolved` へ、`failed` = 達成不能、`blocked` = 権限・制約により開始または継続不能。`blocked` は成功 run envelope 内の `result.status` であり、run-level error envelope（`POLICY_VIOLATION` 等）とは別物。
- top-level `evidence` は optional（aggregate / tool-execution evidence — 実行した command の exitCode 一覧や全体の truncation 事実等）。findings の `evidence` は required（各 finding が evidence を運ぶ spec 要件）。
- evidence には tool 出力の生テキストを入れない（`detail` は 600 文字 cap の注記用）。

### (i) file-level change map

**`crates/cheaper-core`**

- `src/tools/vocabulary.rs`: `is_scope_gated_tool` predicate（(c)）と `pub fn ToolName::resolve`（canonical → alias の single-sourced resolver、(b)）追加。`normalize_tool_names` の per-element path（:111）を `resolve` 呼び出しへ refactor。
- `src/worker/validate.rs`: `CHEAPER_KEYS` に `"optionalTools"`、`WorkerCheaper.optional_tools: Vec<ToolName>`（構築 literal :250 に field 追加 — compile-forced）、(b) の検査 rule + unit test。`is_directly_comparable_worker` は無変更（**宣言** tools を見る保守的判定 — `process` / `command` が縮退した run でも `directlyComparable=false` のまま）。
- `tests/golden/common/mod.rs`: `dump_worker_def`（:415-455）に `optionalTools` の conditional emit を追加（非空時のみ `cheaper.optionalTools` を出力 — `input` / `writeScope` と同じ pattern）。dump shape が `WorkerDefinition` を観測する唯一の経路であり、これが無いと worker_validate の accept fixture で `optionalTools` の正規化結果を golden 比較できない。

**`crates/cheaper-tools`**

- `src/registry.rs`: `ToolRuntimeOptions.degradable`、(e) の degrade path、有効 names の全 consumer 置換、`RuntimeInner.degraded` + `ToolRuntime::degraded_names()`。
- `src/invocation.rs`: `create_worker_scope` の `ToolRuntimeOptions` 再構成（:257-287）へ `degradable` pass-through 1 行。
- `tests/proc_command.rs:202`・`tests/proc_invocation.rs:107`・`tests/proc_runtime.rs:89`・`tests/characterization.rs:74`・`tests/differential.rs:670`: 各 `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及）。

**`crates/cheaper-runtime`**

- `src/run/mod.rs`: `create_tool_scope` で `degradable` 配線 + `cheaper.run.tool_scope` span に `tools.degraded` field（record 位置は (f) の順序規約に従う）、`create_session` で `tool_names` を `scope.definitions()` から組み立て + `build_system_prompt` 第 4 引数へ縮退名を渡す（(f)）。
- `src/dev/fixture.rs`: `FixtureWorker` に `optionalTools: Vec<String>` field（serde `#[serde(default, rename = "optionalTools")]`）を追加し、`to_markdown()`（:291-312）が非空時のみ `cheaper:` 配下へ `  optionalTools: [...]` を emit（(a) の frontmatter shape と同形）。**この knob は必須** — regression harness（`crates/cheaper-cli/tests/regression/harness.rs:162-165`）が `f.worker_markdown()` を無条件で `.cheaper/workers/<name>.md` へ書き込み、`case_argv`（harness.rs:209-219）が `run <fixture.worker.name>` を auto-compose する。file seed した opt-in worker はこの write で clobber されるため、DSL 経由でなければ degraded-run の regression case を表現できない。`fixture.rs` に `ToolRuntimeOptions` literal は存在せず compile-forced の対象外。
- `tests/`（`FakeSessionFactory` handle 層）: 縮退 run の system prompt に実登録集合と利用不可注記（blocked 誘導行の conditional emit 含む）が含まれる assert。

**`crates/cheaper-rig-adapter`**

- `src/agent.rs`: `build_system_prompt` に第 4 引数 `unavailable_tools` + 利用不可注記 emit（blocked 誘導行は `status` enum 条件付き — (f)）。呼出し側は `run/mod.rs` の 1 site + test。
- `tests/adapter.rs:314`: `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及）。

**`crates/cheaper-workflow`**

- `tests/common/mod.rs:109`: `WorkerCheaper` literal に `optional_tools` field（compile-forced 波及 — 構築 site はここと validate.rs:250 の 2 箇所）。
- `tests/artifacts.rs:69`・`tests/process.rs:32`: 各 `ToolRuntimeOptions` literal に `degradable` field（compile-forced 波及）。

**`crates/cheaper-cli`**

- `resources/workers/tool-operator.md` + `resources/workers/schemas/tool-operator.{input,output}.schema.json`: 新規（(a)/(h)）。worker discovery は dir 走査のため code 側の一覧変更不要。
- `src/args.rs`（alias table + error enumeration）、`src/cli.rs`（HELP）、`tests/args.rs`（alias table + positional reject）。
- `src/workflow_run.rs`: `ToolJobExecutor` の `ToolRuntimeOptions` literal に `degradable: Vec::new()`（compile-level。`kind: worker` job は run 経路で自動的に縮退が効く）。
- `tests/run_dev_seam.rs`: degraded run・grants 環境 run・capability 拒否の fixture test（`command_user_policy` / `FIXTURE_COMMAND_CALL` / opt-in `--worker-file` fixture worker pattern）。
- `tests/regression/cases/` + `expected/`: 縮退 run の case。

**docs / APM**

- `README.md` built-in workers 表（:29-30・:67-70・:84-86）、`docs/guides/usage.md`（`directlyComparable=false` の扱い）、`docs/guides/configuration.md`（`optionalTools` frontmatter key の言及が必要なら）。
- `docs/guides/workers.md`: worker 表（:66-68）への `tool-operator` 行追加と縮退挙動の説明に加え、frontmatter key リファレンス（:36-60 の bullet 列）に `cheaper.optionalTools` の key 項目を明示的に追加する — 挙動説明や表だけでなく key 自体の構文（`tools` 宣言名の部分集合・gated tool のみ・全 reject は `InvalidWorkerDefinition`）を記載する。
- `docs/guides/command.md`: `tools.allow` admission と content-gate の棲み分け（admin が `command` を禁止しつつ縮退実行を許すには `tools.allow` ではなく grant を定義しないことで実現する点）に加え、:9 の「`command` を expose する runtime 構築は `POLICY_VIOLATION` で fail closed する」記述に opt-in caveat をその場で補足する — `cheaper.optionalTools` で opt-in した worker のみ gate 不成立時に未登録へ縮退し run を継続する旨。
- `packages/apm/`（`.apm/` primitives）: `cheaper-tool-operate` skill を `cheaper-bulk-read` 等との parity で追加。

### (j) back-compat / 波及面

- `optionalTools` 未指定の既存 worker の意味論は完全に不変 — 非 opt-in の `tools: [command]` worker は引き続き `POLICY_VIOLATION` で fail し、既存 test `run_fake_stream_command_worker_denied_without_grants` は変更なしで green であり続けること。
- `optionalTools` は built-in に限らず全 worker origin（project / user / `--worker-file`）で利用可能だが、strictly capability-shrinking であり grant authoring・未登録 executor への到達はいずれの origin でも不可能。
- `ToolRuntimeOptions` の新 field は struct literal の compile error で全構築 site を強制列挙する（`run/mod.rs:427`・`workflow_run.rs:462`・cheaper-tools tests `proc_command.rs:202` / `proc_invocation.rs:107` / `proc_runtime.rs:89` / `characterization.rs:74` / `differential.rs:670`・`cheaper-rig-adapter/tests/adapter.rs:314`・`cheaper-workflow/tests/artifacts.rs:69` / `tests/process.rs:32` — #101 と同じ機械的波及）。`WorkerCheaper.optional_tools` も同じく compile-forced（構築 site は `validate.rs:250` と `cheaper-workflow/tests/common/mod.rs:109` の 2 箇所）。`InvocationToolRuntimeOptions`・`WorkerScopeOptions` は shape 不変のため波及しない。`src/dev/fixture.rs` は serde DSL で `ToolRuntimeOptions` literal を持たず compile-forced の対象外 — `FixtureWorker.optionalTools` knob の追加は (i) の別項目として扱う。
- golden/fixture churn は限定的: `RuntimePolicy`・usage.jsonl record・envelope `meta` の shape は不変。`vocabulary.json` fixture の toolNames も不変（predicate 追加のみ）。worker_validate golden に `optionalTools` の reject/accept fixture を追加する。

## Data and Control Flow

- run 経路: `run_worker_inner` → `check_capabilities`（`tools` 宣言 ⊆ `tools.allow` + tier 検査 — hard、不変）→ `create_tool_scope`（`ToolRuntimeOptions{names: effective_tools, degradable: worker.optional_tools}`）→ `create_worker_scope`（parity check 後 `degradable` pass-through）→ `create_tool_runtime`（content-gate 不成立の opt-in gated tool を `names` から除去、それ以外の gate は hard のまま）→ `ToolRuntime{definitions, degraded_names}` → `create_session`（`tool_names` = 実登録集合、`unavailable_tools` = 縮退名を宣言順で prompt 注記へ — blocked 誘導行は `status` enum 条件付き、`cheaper.run.tool_scope` span に `tools.degraded`）→ model session → output schema validation → envelope。
- degraded tool への model からの call: registry に未登録のため unknown-tool 経路で安全に失敗（invalid-tool retry budget 消費のみ、権限変化なし）。
- `tools.allow` による宣言 tool の除外: `check_capabilities` で scope 構築**より先**に `POLICY_VIOLATION` — `optionalTools` は免除しない（admission と content-gate の 2 層分離）。
- workflow `kind: worker` job: `run_worker` 経路で同一の縮退挙動。`kind: tool` job: `degradable` 空、frontmatter を持たないため opt-in 対象外。

## Non-Functional Constraints

- `meta.changes` は実測 write/edit のみ — tool-operator は write/edit を宣言しないため常に空。`command` / `process` の子 process 副作用を audited change として主張しない（honest reporting）。worker prompt にも同旨を書く。
- `savings.directlyComparable` は `is_directly_comparable_worker` が宣言 tools を見るため `command` / `process` 宣言により常に `false`（縮退の有無で変えない保守的扱い）。
- execution 制限は global policy のみ（`maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000`）。縮退 run でも同じ制限が効く。
- degraded-name の観測可能性は `cheaper.run.tool_scope` span field（`tools.degraded`）と system prompt 注記に限定される — envelope `meta` からは読めない（shape を変えない ADR 副次決定、accepted limitation）。
- `command` × writeScope の reject は環境依存で挙動が分岐する（grants 非空で登録される環境では hard-fail、grants 空で縮退する環境では check 不発）— ADR の accepted 挙動どおり。

## Risks and Trade-offs

- **struct-literal compile-forced 波及**: `ToolRuntimeOptions` 新 field は全 literal 構築 site の compile error を強制する（(i)/(j) で列挙済み — `run/mod.rs`・`workflow_run.rs`・cheaper-tools/rig-adapter/workflow 各 test の literal。`src/dev/fixture.rs` は literal を持たず対象外）。`WorkerCheaper.optional_tools` も同様。漏れは compile error で検出されるため silent miss はない。
- **golden / fixture churn**: worker_validate golden・regression expected・dev-seam fixture に `optionalTools` 系 case を追加する。`RuntimePolicy` / vocabulary / envelope shape は不変のため広い fixture 再生成は不要の見込み — 差分が出た場合は本設計の対象かどうかを確認する。
- **degraded-name observability**: 「tool が allow 済みだが未登録」の中間状態が span + prompt にしか残らない。prompt 注記の内容は cheaper-runtime test で固定するが、`tools.degraded` span field の assert は行わない — repo に tracing-capture test harness が存在しない（`tracing_subscriber` は cheaper-observability `init` のみ）。span field は observability mechanism として残し、検証は `degraded_names()` accessor の unit test + prompt 注記で代替する（ADR consequence と同じ accepted trade-off）。
- **user-defined worker の opt-in**: `optionalTools` は全 origin で使えるが capability-shrinking 専用 — 緩和が広げるのは「gate 不成立で未登録化する」挙動のみであり、権限を広げる方向には使えない。誤用（非 gated / 未宣言名の列挙）は validate error で通知する。
- **model が degraded tool 名を呼ぶ**: prompt 注記で抑止し、呼ばれても unknown-tool 経路で安全に失敗するが invalid-tool retry budget を消費し得る（accepted）。
- **`optionalTools` + schema の整合**: opt-in worker が `blocked` 相当の status を output schema に持たない場合、model の blocked 表現が schema validation で reject され repair/retry を消費する。`tool-operator` は enum に含める。user-defined opt-in worker 向けに docs で推奨を明記する。

## Deferred Design Concerns

- `cheaper tool-operate "<task>"` 形の positional-sugar — 既存 alias は extra positional を reject する規約であり canonical `--input` / stdin 経路で要件を満たせる。positional 入力の需要が実測された場合、parser 規約ごと別 spec で検討（spec の deferred item と同じ）。
- execution 制限（global `maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000`）が multi-step 反復に実用上足りるかの実測 — per-worker override が要ると判明した場合は strict `cheaper` keys の拡張 = core 変更として別 spec へ。
- `ollama_e2e.rs` の実 provider 検証 — provider 依存の問題が出た場合に別途 revisit。

## Verification Notes

- [ ] AC1: `cheaper tool-operate --input <file>` および stdin JSON（`{"task": "..."}`）が `cheaper run tool-operator` と等価に動作し、args test（alias table + positional reject）と dev-seam fixture で検証される。
- [ ] AC2: 成功 run が `status: success`・summary・evidence 付き findings を宣言 output schema 適合で返す dev-seam fixture test。
- [ ] AC3: `partial` / `failed` の run が `status`・`unresolved`・`nextActions` を返す test。
- [ ] AC4: builtin-default policy（`tasks` / `grants` 空）で opt-in worker が `POLICY_VIOLATION` 無しに scope 構築を通過し `process` / `command` を除く実登録集合で run 完走する dev-seam fixture test。`FakeSessionFactory` handle で system prompt の実登録集合 + 利用不可注記を assert — `status` enum に `"blocked"` を持つ schema では列挙行 + blocked 誘導行の 2 行、持たない schema では列挙行のみ emit される conditional 分岐も併せて固定する。fake stream の `status: blocked` 出力が schema 適合で `result.status == "blocked"` となる test。`tools.allow` による除外は従来通り run 全体を fail させる test（`missing` details 付き capability `POLICY_VIOLATION`）。
- [ ] AC4 補足 — 片側のみの縮退: mixed-policy fixture（`process.tasks` 非空 + `command.grants` 空 → `command` のみ縮退し `process` は gate 通過込みで登録。および/またはその逆）で、unavailable 注記が failed-gate の tool だけを正確に列挙することを assert する。
- [ ] AC5: 巨大中間出力を発生させる run で親へ返る envelope に中間出力が含まれず `result` が output schema 適合・`output.maxTokens` 以内となる test。
- [ ] AC6: `maxTurns` / `maxToolCalls` / `timeout` / 出力 cap の打ち切りが既存機構どおり効く test（既存 regression `run-max-turns` 等との parity）。
- [ ] AC7: `write` / `edit` が宣言されず undeclared tool として利用不可であることの検証。
- [ ] AC8: policy が許可しない `process` / `command` が registry に登録されず実行経路を持たないこと、権限なしに実行されないことの検証（degraded fixture + 非 opt-in 拒否の既存 test green）。degraded run 内に scripted `{"tool_calls": [{"name": "command", "input": {…}}]}` step（fake seam `ToolCallSpec` — fixture.rs `FixtureStep::to_script_step` :476-484 → `ScriptStep::ToolCalls`、実行は `ToolRuntime::execute` の実経路 fake_provider.rs:629-643）を置き、対応 `ToolResult` の `output.error` が `UNKNOWN_TOOL`（"unknown or unregistered tool: command"、registry.rs:388-399）を含むことを assert する — 縮退 tool への実行経路が存在しないことの直接証明。
- [ ] AC9: `command` 経由実行を含む run で `meta.changes` が空のまま（honest reporting）である test。
- [ ] AC10: run record が usage.jsonl に記録され `cheaper usage` で集計可能、`savings.directlyComparable=false` が付く test。
- [ ] AC11: issue #102 の test case が dev-seam fixture（degraded / granted / capability-denied）と regression corpus case でカバーされる。
- [ ] AC12: README built-in workers 表・`workers.md`・`command.md`・`usage.md`・`cheaper-tool-operate` skill が更新される。
- [ ] AC13: `mise run check` が pass する。
- [ ] `optionalTools` validate 規則（非配列・空配列・vocabulary 外・未宣言・非 gated・重複・`optionalTools ⊇ tools` の reject、未指定時の既存挙動不変）の unit test が通る。
- [ ] grant 定義環境で opt-in worker の `command` が登録され `command_user_policy` + `FIXTURE_COMMAND_CALL` パターンで実 spawn する test が通る。
- [ ] `command` × writeScope 宣言 worker は `command` が登録される限り従来通り hard reject される test が通る。
- [ ] workflow `kind: worker` job が opt-in worker を実行する経路で縮退が同一挙動となる test が通る。
- [ ] `tools.degraded` span field は automated assert 対象外とする（repo に tracing-capture test harness が無い — `tracing_subscriber` は cheaper-observability `init` のみ）。span field 自体は observability mechanism として残し、検証は prompt 注記の内容固定と `degraded_names()` accessor の unit test で代替する。

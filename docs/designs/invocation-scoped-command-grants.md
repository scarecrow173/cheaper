---
id: "DESIGN-034FntkIZGiffhdU4caSkN"
type: "design"
status: "approved"
title: "invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant の設計"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/105"
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
    - "../designs/policy-gated-command-tool.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/invocation-scoped-command-grants.md"
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "../discovery/invocation-scoped-command-grants.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../guides/command.md"
    - "../guides/workers.md"
    - "../guides/usage.md"
    - "../guides/configuration.md"
  defers: []
  deferred-by: []
---

# invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant の設計

## Context

issue #105 が採用した 3 component —— invoker が run-lifetime の grant を注入する repeatable `--grant` flag、tool-operator output の machine-readable `grantRequests` hint、user policy へ永続化する `cheaper policy grant add/list/remove` —— の実装設計。spec `../specs/invocation-scoped-command-grants.md`（SPEC-034FlOz4hiN9m0nQW7lBN2）が要件と AC を、ADR `../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`（ADR-034FlTS3BaiV3WKz8bX1Xv — accepted）が全ての機構決定を確定済みである。本 design が残すのは、確定済み decision を具体 code へ写す際の surface 形状（field 名・関数・envelope shape・file map）と compile-forced 波及の列挙のみであり、新たな mechanism 決定は行わない。

中心となる構造的事実は declared-shape parity 契約である。`create_invocation_tool_runtime` は `options.command` を `resolve_command_policy` して `inner.command` に保持し（`crates/cheaper-tools/src/invocation.rs:389-447`）、`create_worker_scope` は `options.runtime.command` を再 resolve して `inner.command` と `PartialEq` 比較する（invocation.rs:226-238）。run 経路は同一の `deps.policy.command` を 2 call site で serialize する（`crates/cheaper-runtime/src/run/mod.rs:284`・`:436`）、workflow 経路は `ensure_invocation` が `shared.policy.command` を serialize する（`crates/cheaper-cli/src/workflow_run.rs:242-246`）。したがって `--grant` 由来の grant は `RuntimePolicy.command` に畳み込まれなければならず、その唯一の注入点が `load_policy` の user merge 直後（`crates/cheaper-core/src/policy/load.rs:67-70` の gap）の合成 `invocation` layer である（ADR Option A）。

## Scope

### In Scope

- `crates/cheaper-core/src/policy/load.rs`: `LoadPolicyOptions.invocation` field と、user merge 直後の grants-only 検査 → Partial validate → collision 検査 → `MergeMode::Replace` merge の注入点、`policy.layer = "invocation"` の tracing label。
- `crates/cheaper-cli/src/args.rs`: repeatable `--grant`（`STRING_OPTIONS` + `Values.grants: Vec<String>` + push）、`RunCliArgs` / `WorkflowRunCliArgs` への populate、非 run 系 command の明示 reject、`policy` command の parse 表面、64KiB flag 値上限、HELP・command enumeration 更新。
- `crates/cheaper-cli/src/run.rs`・`workflow_context.rs`・`workflow_run.rs`: flag 値 → invocation layer object の組立（共有 builder）と `load_policy` への接続、`execute_interface` の `None` 固定、stderr diagnostic・非露出 warning。
- `crates/cheaper-runtime/src/run/mod.rs` + `run/boundary.rs`: `RunWorkerDeps.invocation_grants` field と `cheaper.run.tool_scope` span の `grants.invocation` field record。
- `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`: optional top-level `grantRequests[]`。
- `crates/cheaper-cli/resources/workers/tool-operator.md`: emit 条件の prompt contract。
- `crates/cheaper-cli/src/policy.rs`（新規 command module）+ `cli.rs` dispatch / `command_name` / HELP。
- `crates/cheaper-core`: user `policy.yaml` 読み出し wrapper、`yaml.rs` の write wrapper、`atomic_write` の runtime → core 移動と runtime caller 更新。
- 検証面: `tests/args.rs` parse table、dev-seam fixture（`run_dev_seam.rs`）、regression corpus case、core `policy_load` golden、`proc_invocation` parity。
- 公開面 docs: `docs/guides/{command,workers,usage,configuration}.md`、`packages/apm/.apm/skills/{cheaper-usage,cheaper-tool-operate}/SKILL.md`、`README.md` commands 表。

### Out of Scope

- model 出力経由・project policy 経由の grant authoring（設計上、経路自体を作らない。`ProjectPartial` schema が `command` section を reject する既存境界は不変）。
- `process.tasks` への同型 invocation-scope 機構、`--grant-file`（file からの spec 読み込み）、`usage.jsonl` record shape の拡張 — いずれも ADR が本 iteration から defer した事項。
- `command` tool 本体の contract 変更（deny list・argv pattern・executable pin・env/cwd 規則・managed-process lifecycle の意味論は不変）。
- cheaper 内部での interactive confirm prompt（approval は parent agent の責務）。
- `cheaper policy` の `command.grants` 以外の section 管理・project policy 対象化。

## Components and Boundaries

### (a) `LoadPolicyOptions.invocation` と merge point

`crates/cheaper-core/src/policy/load.rs` の `LoadPolicyOptions`（:44-48）に第 4 の field を追加する:

```rust
pub struct LoadPolicyOptions<'a> {
    pub builtin_policy_path: &'a str,
    pub user_config_dir: &'a str,
    pub project_dir: &'a str,
    /// 合成 trusted layer — `{"version":1,"command":{"grants":{..}}}` のみ
    /// 受理する grants-only 入力。user merge 直後・typed re-resolve 直前
    /// に Partial validate + Replace merge される。
    pub invocation: Option<&'a Map<String, Value>>,
}
```

`load_policy`（:52-132）の user merge 直後・`let mut with_defaults` 直前（:67-70 の gap）に、次の順序で injection block を挿入する:

1. **grants-only key 制約**（構造検査 — schema ではなく merge 前検査）: `invocation` map の top-level key ⊆ `{version, command}`、かつ `command` が存在するならその値が object で key ⊆ `{grants}`。違反は `InvalidRequest` + `details.layer = "invocation"`（offending key を `details` に載せる）。`Partial` schema だけでは `execution` / `tools` / `model` 等他 section が素通りするため、この制約が flag/将来の内部 caller を「grant 供給のみ」へ縛る構造的 guard である（ADR の主張どおり型ではなく merge 前検査で保証する — CLI が `{"version":1,"command":{"grants":{...}}}` へ既に組み立てる経路と同一規則を共有するため）。
2. **Partial validate**: `validate_policy_schema(&Value::Object(invocation.clone()), PolicySchemaMode::Partial)`。内部で `command` section 存在時に `resolve_command_policy` が走る（`schema.rs:375-377`）ため、deny list（`command.rs:346-355`）・`ArgPattern::compile`（:219-249）・env/cwd 規則・duration bound の全検査を再実装なしで継承する。失敗は inner code（`InvalidRuntimePolicy`）を保持し、details へ `layer = "invocation"` を畳み込む（後述の error attribution 規約 (a)）。
3. **collision 検査**: `effective` の `command.grants`（builtin + user の merge 済み raw map）の key と invocation layer の `command.grants` の key の交差を取り、非空なら `InvalidRequest` + `details.grant = <衝突名>` + `details.layer = "invocation"` で load を fail させる。validate 通過後に行う — malformed spec が collision error を装う順序を避ける。`details.grant` は複数衝突時も先頭 1 件の singular 値とする（cosmetic — 配列化しない）。
4. **merge**: `merge_policy_layers(&effective, &validated, MergeMode::Replace).expect("replace-mode merge cannot fail")` — user merge（:61-66）と同型。
5. **tracing**: `tracing::debug!(policy.layer = "invocation", "policy layer merged")` — 既存の `"user"` / `"project"` label 規約に一致する第 3 の layer label。

merge 後は既存経路がそのまま働く: `resolve_command_policy`（:87）が merged `command` を typed re-resolve し、project layer（:108-126）は `ProjectPartial` schema が `command` を reject するため invocation grant を override 不能。`load.rs` 冒頭の doc comment（:1-4 の load order 記述）を builtin → user → **invocation** → typed re-resolve → project → complete の 4-layer 順序へ更新し、grants-only 制約を明記する。

edge: merged map でのみ発火する検査（`grants` の `maxProperties: 128` が builtin + user + invocation の合計で超過する場合等）は (2) の standalone validate では捕捉できず、:87 の typed re-resolve または :128 の Complete revalidate で layer 帰属なしの `InvalidRuntimePolicy` となる。単一 grant の規則違反は全て (2) で `layer = "invocation"` 付きで捕捉されるため、この attribution gap は accepted limitation として doc comment に明記する。

### (b) `--grant` flag の args 表面（`crates/cheaper-cli/src/args.rs`）

- **`STRING_OPTIONS`（:228-247）** に `"grant"` を追加する。位置は `"format"` の直後・`"log"`/`"log-format"`（global option — argv scan で消費される別経路、:242-246 の comment 参照）のブロック前とする。member 判定のみに使われるため順序は意味を持たないが、declaration-order 規約に倣い末尾へ追記する。
- **`Values`（:256-273）** に `grants: Vec<String>`（`Default` は空 vec）を追加し、`set_string`（:299-318）に arm を追加する:

```rust
// `--grant` は repeatable — 本 file の冒頭に文書化された
// last-wins 規約からの deliberate deviation(Node `parseArgs` の
// `multiple: true` 相当)。宣言順を保持するため push で畳む。
"grant" => self.grants.push(value),
```

- **flag 値サイズ上限 64KiB**: 各 `values.grants` 要素が `64 * 1024` byte を超える場合 `InvalidRequest` で reject する。enforce 位置は `parse_cli_args` 内の populate 段 — `run` / `workflow run` が `grants` を消費する直前に検査し、非 run command では `!values.grants.is_empty()` の reject が先に発火する（「その command では使えない」旨の error が size error より自然に優先される）。cmdline 長制限と無制限入力への sanity bound を兼ねる。
- **`RunCliArgs`（:71-86）と `WorkflowRunCliArgs`（:118-127）に `grants: Vec<String>`** を追加し、populate 経路（`RunCliArgs` literal :903-912、`WorkflowRunCliArgs` literal :527-538）で `values.grants` を渡す。alias command（`bulk-read` / `code-write` / `doc-write` / `tool-operate`）は `ParsedCli::Run` fold（:850-863）により無変更で `--grant` を受ける。
- **per-command rejection（silent drop 防止）**: `Values.grants` は default-empty の `Vec` であり、command arm が検査を書き忘れると `--grant` が silent に捨てられる。これを許さず、非 run 系の全 command arm — `usage`（:552-589）、`count-tokens`（:598-653）、`models`（:655-726）、`auth`（:728-827）、`policy`（新設）— に `if !values.grants.is_empty() { return Err(invalid_request("--grant is only valid for `run` and `workflow run`", None)) }` を既存の per-flag reject 規約（`--worker-file is only valid for \`run\`` 等）に倣って追加する。`workflow` arm（:490-546）は allowlist 方式のため、`run` 側の `allowed` 表（:501-510）にのみ `"grant"` を追加する — `workflow interface` 側の表（:512）には入れず自動的に reject される。各 arm の reject は `tests/args.rs` で command ごとに assert し、guard の抜けを test で固定する（AC17）。
- **`policy` command の parse**: `positionals` で `policy grant add <name> <spec-json>`（4 要素）/ `policy grant list`（2 要素）/ `policy grant remove <name>`（3 要素）を受理し、それ以外の形状は `InvalidRequest`（`"policy requires grant add <name> <spec>|list|remove <name>"` 相当）。grant name は add/remove とも parse 段で `^[a-z][a-z0-9_-]{0,63}$`（`command.rs:81` の grant `propertyNames` と同一）を検査する。`policy` が受ける flag は utility 規約の `--format` / `--quiet` / `--log` / `--log-format` のみ — `option_tokens` を走査する allowlist 方式（workflow arm と同型）で、`--grant` / `--worker-file` / `--cwd` / `--input` / `--timeout` 等他 flag は全て reject する。
- **`ParsedCli` に `Policy(PolicyCliArgs)` variant** を追加する:

```rust
pub struct PolicyCliArgs {
    pub action: PolicyAction,
    pub format: CliFormat,
    pub quiet: bool,
}

pub enum PolicyAction {
    /// `policy grant add <name> <spec-json>` — spec は生 JSON 文字列。
    GrantAdd { name: String, spec: String },
    GrantList,
    GrantRemove { name: String },
}
```

- `format()` / `quiet()` accessor（:181-204）の match arm に `Policy` を追加する（utility command 規約 — `Some(a.format)` / `Some(a.quiet)`）。`command required:` enumeration（:484）に `policy` を追加する。
- **HELP**（cli.rs:512-551）: `Commands:` ブロックに `policy grant` 3 形態を、`Options:` ブロックに `--grant <json>` を追加し、repeatable である旨を一行で明記する。

### (c) flag 値 → invocation layer の組立と run / workflow plumbing

flag 値 → `{"version":1,"command":{"grants":{<name>:<spec>}}}` の組立ロジックは run 経路と workflow run 経路の 2 caller が共有するため、新規 `crates/cheaper-cli/src/grant.rs` に単一 helper を置く:

```rust
/// `--grant` 値列 → invocation layer + 宣言順の grant name 一覧。
/// 構造的失敗(malformed JSON・`name` 欠落/pattern 違反・同名重複)は
/// `InvalidRequest` + `details.layer = "invocation"` で返す。
pub(crate) fn build_invocation_layer(
    grants: &[String],
) -> Result<(Option<Map<String, Value>>, Vec<String>), CheaperError>;
```

各値に対し:

1. `serde_json::from_str::<Value>` — parse 失敗は `InvalidRequest` + `details.layer = "invocation"`（`details.grant` は name 未確定のため省略、flag 値の先頭断片は載せない — cmdline 由来値をそのまま echo しない）。
2. `Value::Object` であること。`name` key が存在し string で `^[a-z][a-z0-9_-]{0,63}$` に合致すること — 欠落・非 string・pattern 違反は `InvalidRequest` + `details.layer`（pattern 違反時は `details.grant` に当該値）。
3. `name` を object から **strip** する — `name` は grant spec schema の `additionalProperties: false`（`command.rs:82-99`）で reject される key のため、残したまま schema へ流す実装は必ず失敗する。残りの key は grant spec（`command.grants.<name>` の map value）と同一 shape としてそのまま畳む。
4. intra-invocation duplicate: 同名 name が既に map にあれば `InvalidRequest` + `details.grant` + `details.layer` — 宣言順 push の暗黙 last-wins を許さない。
5. `grants` map が空なら `(None, vec![])`、非空なら `(Some({"version":1,"command":{"grants":{...}}}), names)` を返す。`names` は flag 宣言順 — serde `preserve_order` により map の insertion order と一致するが、diagnostic / span field 用に Vec で保持する。

**run 経路（`crates/cheaper-cli/src/run.rs`）**: `execute_inner` の `load_policy` 呼び出し（:143-147）で `invocation: invocation_layer.as_ref()` を渡す。policy load 成功後（`restrict_policy` 前後どちらでも可 — grants は restrict 対象外）に `grant_names` が非空なら stderr diagnostic 1 行を emit する（`grant: <n1>, <n2>`、`--quiet` で抑制 — `model:` line・catalog warning と同じ diagnostics 規約）。さらに `worker.tools` が `ToolName::Command` を含まない場合（`--worker-file` worker 含む）は `warning: --grant supplied but worker does not declare the command tool` 相当の stderr warning を出して続行する。worker が `command` を宣言しているのに `tools.allow` が除外する場合は `check_capabilities`（`crates/cheaper-core/src/policy/capability.rs:17-62`）の既存 `POLICY_VIOLATION`、pin 失敗は `pin_command_grants`（`crates/cheaper-tools/src/registry.rs:1201-1210`）の hard `POLICY_VIOLATION` に任せ、**warning に変換しない**。

**workflow 経路**: `prepare_workflow_context`（`workflow_context.rs:267-299`）に `invocation: Option<&Map<String, Value>>` param を追加し、内部の `load_policy`（:277-281）へそのまま渡す。`execute_run_inner`（`workflow_run.rs:701`）は `args.grants` から `build_invocation_layer` で組み立てた layer を渡し、`execute_interface_inner`（:665）は `None` を渡す — interface 検査経路に grant を流す意味がない。`execute_run_inner` は `grant_names` 非空時に run 経路と同じ `grant: <names>` stderr diagnostic（`--quiet` で抑制）を emit する — workflow の `kind: tool` job は `cheaper.run.tool_scope` span を通らないため、workflow 経路ではこの stderr 行が invocation grant の audit surface になる（(d)）。`kind: tool` job の `command: None`（:468）は不変 — parity check を skip するが scope 側は `inner.command`（invocation grant 込みの resolved policy）を再注入する（`invocation.rs:268-272`）ため grant は tool job にも届く。workflow run の `kind: worker` job では `WorkerJobExecutor` の `RunWorkerDeps`（:358-376）にも `invocation_grants` を渡すため、names を `Shared` に保持する。

**count-tokens**（`count.rs:69-73`）は `invocation: None` を渡す — grant を受けない command。

**error attribution 規約**（`load_policy` の invocation 処理と CLI の `build_invocation_layer` で共有する単一 rule — `grant.rs`・`load.rs` 双方の doc comment に同一文言で明記）:

- **shared validation 由来の失敗**（`validate_policy_schema` / `resolve_command_policy` が返す schema 違反・deny list 該当・argv compile 失敗・env/cwd 規則違反）: inner error code（`InvalidRuntimePolicy`）を保持し、既存 details に `layer = "invocation"` を畳み込む。実装は `e.details().cloned().unwrap_or_default()` に `("layer", "invocation")` を insert して `.with_details` で差し替える小さな helper（`load.rs` 内の private fn）で行う — 同一 spec を user policy に置いた場合と error code を揃え、policy 内容由来の失敗であることを表す。
- **invocation 入力自体の構造的失敗**（malformed JSON flag 値・`name` key 欠落/pattern 違反・grants 以外の key・intra-invocation 同名重複・policy との collision）: `InvalidRequest` + `details.layer = "invocation"`、grant name が関与する場合は `details.grant` に載せる。
- **flag 値 echo 規約の accepted exception**: 「cmdline 由来値を echo しない」規約には 2 つの例外面がある。(a) shared validation の失敗は `schema_error_entries`（`crates/cheaper-core/src/worker/schema/mod.rs:159`）が `message` に offending instance 値を埋め込むため、`--grant` flag 値の断片が `details.errors[].message` 経由で echo されうる — message 形式は TS parity のため `schema_error_entries` を変更しない accepted exception。(b) pre-existing の `unknown_option`（`args.rs:347-352`）は raw argv 値を error message に echo するため、`-` 始まり等で option token として誤認された `--grant` 値はこの surface を継承する — 同様に変更しない。

### (d) audit surface: span field + stderr diagnostic

ADR が採用した 2 面を実装する:

- **`grants.invocation` span field**: `cheaper.run.tool_scope` span（`crates/cheaper-runtime/src/run/mod.rs:417-420`）の宣言に `grants.invocation = tracing::field::Empty` を加え、`tools.degraded` の record（:476-482）と同じ sync 区間（`create_worker_scope` return 後・`.instrument` 前）で `deps.invocation_grants` の宣言順 join（`names.join(", ")`）を record する — metadata-only field 規約（payload ではなく分類値）に従う。names は runtime 層では policy と区別不能のため、`RunWorkerDeps`（`run/boundary.rs:49-72`）に `pub invocation_grants: &'a [String]` を追加して cli 層から運ぶ（compile-forced 波及は (i) 参照）。workflow `kind: worker` job でも同じ span に同じ値が載る。
- **stderr diagnostic line**: (c) の `grant: <names>` 行 — `run` と `workflow run`（`execute_run_inner`）の双方で emit し、log 無効でも常時観測可能にする（`--quiet` で抑制 — 他 diagnostics と同規約）。workflow の `kind: tool` job は `cheaper.run.tool_scope` span を通らないため、workflow 経路ではこの stderr 行が唯一の audit surface となる。
- `usage.jsonl` record への field 追加は行わない（fixed key list 拡張と集計意味論の設計を伴うため ADR が defer — record shape は不変）。

### (e) `grantRequests` output contract

`tool-operator.output.schema.json`（全階層 `additionalProperties: false`・evidence kind は closed enum — request を既存 field へ混ぜられない）へ optional top-level field を追加する:

```json
"grantRequests": {
  "type": "array",
  "maxItems": 20,
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["executable"],
    "properties": {
      "executable": { "type": "string", "minLength": 1, "maxLength": 4096 },
      "argvHint": {
        "type": "array",
        "maxItems": 256,
        "items": { "type": "string", "maxLength": 4096 }
      },
      "name": {
        "type": "string",
        "pattern": "^[a-z][a-z0-9_-]{0,63}$",
        "maxLength": 64
      }
    }
  }
}
```

- `executable` の bound は grant spec の `bounded_string`（`command.rs:67` — `minLength: 1, maxLength: 4096`）に合わせ、`argvHint` の items/maxItems も grant `argv` と同じ上界（`command.rs:88`）で揃える — request から組み立てた spec がそのまま spec schema を通ることを保証する。
- `name` の `pattern` + `maxLength` は grant `propertyNames`（`command.rs:81`）と同一 — 提案名がそのまま合法な grant name になる。
- `maxItems: 20` は `unresolved` / `nextActions`（:67-76）の既存 bound 語彙に合わせた値。
- strict-mode 適合（`crates/cheaper-core/src/worker/schema/strict.rs`）: `required ⊆ properties`・`format` 不使用・union `type` 不使用・全階層 `additionalProperties: false` — 全て満たす形状。`status` との連動は schema で強制しない（emit 条件は prompt contract）。
- `tool-operator.md` の `## Output rules` に条項を追加する — gated tool の欠如が原因で `status` が `blocked` または `partial` となった場合に限り `grantRequests` を populate し、各 entry は不足した `executable`（必須）と意図した argv の形状 hint（`argvHint`、optional）、提案する grant name（`name`、optional）を運ぶ旨。request は「実行すべき grant」ではなく invoker へ表面化する request であり、権限昇格の試行ではない旨を併記する（prompt は英語 — worker file の言語規約）。さらに `grantRequests` の全 field（`executable` / `argvHint` / `name`）は model 生成値であり prompt injection で汚染されうる untrusted 入力である旨を worker contract に明記し、配布 SKILL・guides 側では「request から組み立てた grant spec を re-run 前に invoker（user / parent agent）が必ず review する」ことを loop の必須手順として文書化する（(j)）。

### (f) `cheaper policy` command

`crates/cheaper-cli/src/policy.rs`（新規）に `pub async fn execute(args: &PolicyCliArgs, io: &mut CliIo<'_>, hooks: &CliHooks, started_at: f64) -> Result<CommandOut, BoxError>` を utility command 規約（`usage_report::execute_usage` 型 — `cheaper.cli.command` span で instrument）で実装する。dispatch arm（cli.rs:323-444）に `ParsedCli::Policy` を追加し `write_output` で envelope を出す。`command_name`（:450-462）に `"policy"` を追加。`record_usage` は呼ばない — `policy` は utility command であり usage.jsonl 記録対象外（AC9）。

- **user config 解決**: `resolve_user_config_dir`（`crates/cheaper-core/src/config/dirs.rs:37-53`）を `UserConfigDirOptions { env: Some(io.env), homedir: None }` で直接呼ぶ — `resolve_config_dirs`（builtin dir・project dir 解決を含む）は使わない。`--cwd` は flag として reject 済みのため policy path は cwd に依存しない。`cli.rs` の `resolve_log_root`（:288-300）は `policy` を `cwd_flag_relevant` に含めないため `io.cwd` を使う現状どおりで変更不要。
- **add `<name> '<spec-json>'` flow**:
  1. spec-json を `serde_json::from_str` で parse — object でない・`name` key を含む入力は `InvalidRequest`（spec は `command.grants` の map value shape であり name を含まない）。
  2. `<userConfigDir>/policy.yaml` を `read_policy_yaml`（(g)）で読む。file absent は新規 file とみなし in-memory base を `{"version":1}` とする（新規 file の `version` は本 command が authoring する — 以降は既存 file と同一の merged-object 経路）。
  3. 既存 file の構造前提 pre-check: `command` が存在し object でない場合、および `command.grants` が存在し object でない場合は `InvalidRuntimePolicy` で fail する — silent に空 map へ畳み替えない（schema の型制約と同じ判断を write 側で先に表面化する）。既存 `command.grants` に `<name>` が既にあれば `InvalidRequest`（`details.grant` — collision と同じ fail-loud 規約。置き換えたい場合は `remove` 後に `add` する導線）。
  4. 読んだ policy object **全体**の `command.grants` を merged map（既存 + 新規）で置き換えた full object（他 section は一切触れない）を `validate_policy_schema(Partial)` へ流す。新規 grant 個別の検査（schema・deny list・argv compile・env/cwd 規則は `resolve_command_policy` 経由）に加え、Partial schema の `version`（`required` + `const: 1`、`schema.rs:265-267`）と `command.grants` の `maxProperties: 128`（既存 + 新規の合計で判定）により、既存 file の pre-existing corruption も検査される。既存 file が `version` を欠く・`version ≠ 1` の場合もここで `InvalidRuntimePolicy` となり、黙って `version: 1` を書き足す rewrite は行わない。失敗は file を変更せず error envelope へ。**executable の存在確認は行わない** — pin は tools 層の実行時検査であり、存在しない executable は scope 生成時に `POLICY_VIOLATION` で報告される（user policy 直書きと同一の意味論）。
  5. 検査通過後、その merged full object を `stringify_yaml`（(g)）で serialize → parent dir を `create_dir_all` → `atomic_write`（`windows_remove_first: false`、POSIX `unix_mode: Some(0o600)`）で書き込む。
  6. 成功 envelope の `result` は `{ "grant": {"name": ..., "executable": ...} }` 相当の compact 形。**top-level `tokens` key は載せない** — `usage_log::build_usage_record`（`usage_log.rs:99-107`）が utility envelope の `result.tokens` を count record の判定に使うため、同名 key は誤記録を誘発する。
- **list**: `command.grants` を読み、`{ "grants": [{"name": ..., "executable": ...}, ...] }` を utility envelope で返す（name 順 stable のため key sort）。file absent は空配列。
- **remove `<name>`**: `add` と同じ構造前提 pre-check（非 object の `command`/`command.grants` は `InvalidRuntimePolicy`）→ grants map から `<name>` を削除 → write 前に **`add` と同じ full-object `validate_policy_schema(Partial)`** を通す → 同じ write path で書く。remove 自体は新たな invalidity を持ち込めないが、同一検査を適用する方が単純かつ一貫する — 破損が削除対象の grant 自身なら remove 後の object は valid になり通過する一方、他 section の破損や `version` 欠落を持つ file では remove も loud に失敗し、その場合は user が file を直接修正してから再実行する運用とする（accepted）。name が存在しない・file absent の場合は `InvalidRequest` の error envelope（utility command の入力検証として一貫）。
- write 失敗の io error は `InternalError` + `details {file, ioError}`（credential store の `io_error` 形状、auth/store.rs:275-283 に倣う）。

### (g) user policy write の基盤（core の新規 public API）

`cheaper policy` が必要とする 3 点を core の public API として提供する:

- **policy.yaml 読み出し**: `load.rs` の private `read_policy_file`（:30-40 — `read_yaml_layer` + `POLICY_LAYER` の薄い wrapper）を `pub fn read_policy_yaml(file_path: &str, required: bool) -> Result<Option<Map<String, Value>>, CheaperError>` として公開する（`LayerKind` と `read_yaml_layer` 自体は `pub(crate)` のまま — 公開面は policy 用途に絞る）。
- **YAML write**: `crates/cheaper-core/src/yaml.rs` に `pub fn stringify_yaml(value: &Value) -> Result<String, YamlError>` を追加し、`YamlError` を `pub` 化する。`lib.rs:24` の `pub(crate) mod yaml` は **`pub mod yaml` へ昇格**させる（または `pub use yaml::{stringify_yaml, YamlError}` を re-export）— `pub(crate)` のままでは cli crate から到達不能で write path が組めない。module-level の `#![allow(deprecated)]`（yaml.rs:16）は既に module scoped で serde_yml shim を隔離済み — 新規依存は持ち込まない。
- **atomic write**: `crates/cheaper-runtime/src/fs_util.rs`（:1-74 全体 — `atomic_write` + `AtomicWriteOptions`）を **`crates/cheaper-core/src/fs_util.rs` へ移動**し `pub` にする（複製しない）。runtime 側の `mod fs_util` 宣言を除去し、既存 caller を `cheaper_core::fs_util::atomic_write` 参照へ機械的に書き換える — `auth/store.rs:263-271`（credentials、`windows_remove_first: true` + `0o600`）と `catalog/persisted.rs:107-115`（`windows_remove_first: false`）。policy.yaml の write option は `windows_remove_first: false`（`std::fs::rename` の `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` で atomic rename が成立し、事前 remove は置換前の非 atomic な窓を作るため credentials 継承 workaround を持ち込まない）+ POSIX `unix_mode: Some(0o600)`（grant の `env` 値が secret を保持しうるため owner-only readable）。

### (h) file-level change map

**`crates/cheaper-core`**

- `src/policy/load.rs`: `LoadPolicyOptions.invocation` field、(a) の injection block（key 制約 → Partial validate → collision → Replace merge → `policy.layer = "invocation"` debug event）、`details.layer` を畳み込む private helper、`read_policy_yaml` の公開化、load-order doc comment の 4-layer 更新。unit test 4 件の `LoadPolicyOptions` literal（:163/:193/:207/:222）に `invocation: None` を追加（compile-forced）。
- `src/yaml.rs`: `stringify_yaml` + `YamlError` の `pub` 化。
- `src/lib.rs`: `pub(crate) mod yaml` → `pub mod yaml` 昇格（または `stringify_yaml` / `YamlError` の pub re-export）+ `pub mod fs_util` 追加。
- `src/fs_util.rs`（新規）: runtime から移動した `atomic_write` / `AtomicWriteOptions`（`pub`）。
- `tests/golden/policy_load.rs:18`: `LoadPolicyOptions` literal に `invocation` field（compile-forced — golden harness が `opts.invocation` の fixture key を読む経路を追加し、invocation layer case を `policy/load.json` fixture に足す）。

**`crates/cheaper-runtime`**

- `src/fs_util.rs`: 削除（core 移動）。
- `src/auth/store.rs`・`src/catalog/persisted.rs`: `crate::fs_util::` → `cheaper_core::fs_util::` 参照へ変更。
- `src/run/boundary.rs`: `RunWorkerDeps` に `pub invocation_grants: &'a [String]`。
- `src/run/mod.rs`: `cheaper.run.tool_scope` span に `grants.invocation` field + record（(d)）。
- `tests/run_dev_seam.rs:270`・`:1046`、`tests/run_fixture_test.rs:116`、`tests/ollama_e2e.rs:296`: 各 `RunWorkerDeps` literal に `invocation_grants: &[]`（compile-forced）。

**`crates/cheaper-cli`**

- `src/args.rs`: (b) の全変更（STRING_OPTIONS・`Values.grants`・`set_string` arm・`RunCliArgs`/`WorkflowRunCliArgs`・`PolicyCliArgs`/`PolicyAction`/`ParsedCli::Policy`・accessor arm・per-command reject・64KiB bound・enumeration）。
- `src/grant.rs`（新規）: `build_invocation_layer`。
- `src/run.rs`: `load_policy` へ layer 接続、stderr diagnostic、非露出 warning、`RunWorkerDeps.invocation_grants`。
- `src/workflow_context.rs`: `prepare_workflow_context` に `invocation` param + `load_policy` literal 更新。
- `src/workflow_run.rs`: `execute_run_inner` で layer 組立・`Shared` へ names 保持・worker job の `invocation_grants`・`grant:` stderr diagnostic（`--quiet` 抑制）、`execute_interface_inner` は `None`。
- `src/count.rs:69`: `invocation: None`（compile-forced）。
- `src/lib.rs`: `pub mod grant;`・`pub mod policy;` の module 宣言（既存 `pub mod` 群へ追記 — 新規 module を compile 対象に含める）。
- `src/policy.rs`（新規）: `policy grant` command 本体。
- `src/cli.rs`: dispatch arm・`command_name`・HELP（`policy grant` + `--grant` 記述）。`resolve_log_root` は変更不要。
- `resources/workers/schemas/tool-operator.output.schema.json`・`resources/workers/tool-operator.md`: (e)。

**tests**

- `crates/cheaper-cli/tests/args.rs`: repeatable `--grant`（`--grant <v>` / `--grant=<v>` 両形・複数指定の順序保持）、alias 経由受理、非 run 系全 command（`usage` / `count-tokens` / `models` / `auth` / `policy` / `workflow interface`）の `InvalidRequest` reject、`policy grant` parse table、64KiB bound。
- `crates/cheaper-cli/tests/run_dev_seam.rs`: AC1/AC2/AC3/AC4/AC8/AC10/AC13/AC15 系の fixture test。
- `crates/cheaper-cli/tests/policy.rs`（新規）: `policy grant add/list/remove` の temp `CHEAPER_CONFIG_DIR` round-trip・write-mode（POSIX `0600`）・error envelope。
- `crates/cheaper-cli/tests/regression/cases/` + `expected/`: `extraArgs` に `--grant`、`config.files` で user policy seeding する case。
- `crates/cheaper-tools/tests/proc_invocation.rs`: parity test が変更無しで green であること。

**docs / APM**（(j) 参照）

### (i) compile-forced propagation / back-compat

- `LoadPolicyOptions` の新 field は struct literal の compile error で全構築 site を強制列挙する — `run.rs:143`・`workflow_context.rs:277`・`count.rs:69`・`load.rs` unit test :163/:193/:207/:222・golden `policy_load.rs:18` の 8 site。漏れは compile error で検出されるため silent miss はない。
- `RunWorkerDeps.invocation_grants` も同様 — literal site は `run.rs:234`・`workflow_run.rs:358`・runtime tests `run_dev_seam.rs:270`/:1046・`run_fixture_test.rs:116`・`ollama_e2e.rs:296` の 6 site。
- `RunCliArgs` / `WorkflowRunCliArgs` / `PolicyCliArgs` の literal 構築 site は `args.rs:903`・`:527` の 2 site のみ（tests は `parse_cli_args` の戻り値を destructure するため波及しない）。`ParsedCli::Policy` variant の追加は `format()`/`quiet()`/`command_name`/dispatch の exhaustive match で compile-forced。
- grant 未指定（`--grant` 無し）の既存挙動は完全に不変 — `invocation: None` で load 経路の差分は発生しない。`RuntimePolicy`・usage.jsonl record・envelope `meta` の shape は不変。
- `--grant` は model 入力・worker prompt・project policy のいずれからも到達不能 — 供給経路は `RunCliArgs`/`WorkflowRunCliArgs` の CLI 引数のみであり、project layer は `command` section を schema reject するため override も不能。
- `atomic_write` の移動は公開面の追加（`cheaper_core::fs_util`）を伴うが、移動自体は機械的 — runtime caller 2 site の import 書き換えのみ。

### (j) 公開面 docs の deliverable

`docs/guides/{command,workers,usage,configuration}.md`・`packages/apm/.apm/skills/{cheaper-usage,cheaper-tool-operate}/SKILL.md`・`README.md` commands 表へ反映する内容:

- `--grant` contract（repeatable・`name` 内包 JSON・collision / intra-invocation duplicate は error・非永続・64KiB 上限）と `workflow run` への適用。
- `grantRequests` → user approval → `--grant` re-run loop。`grantRequests` の全 field は model 生成・prompt-injectable な untrusted 値であり、invoker は request から組み立てた grant spec を re-run 前に必ず review する — この手順を loop の必須要件として明記する（(e)）。
- `cheaper policy grant {add,list,remove}` の UX と write path caveat（read-modify-write の last-wins・serde_yml rewrite が comment/整形を保存しない）。
- cmdline 露出 warning: `--grant` 値は `ps`/`/proc`/shell history に露出する。spawn は `env_clear` + `minimal_env(grant.env)` で ambient 環境変数は子へ届かないため、secret は user policy の grant `env`（0600 の `policy.yaml`）か子プロセス自身が読む credential file へ置く。`policy grant add <name> '<json>'` も spec を一時的に cmdline へ載せる。

## Data and Control Flow

- **`--grant` run 経路**: `parse_cli_args`（`--grant` を `Vec<String>` へ push、64KiB bound・非 run command reject）→ `run::execute_inner` が `build_invocation_layer` で `{"version":1,"command":{"grants":{...}}}` + names を組立 → `load_policy` が builtin → user merge → **invocation merge**（key 制約 → Partial validate + `resolve_command_policy` 系統 full validation → collision 検査 → Replace merge）→ typed re-resolve で `RuntimePolicy.command` に normalize → `restrict_policy` → `run_worker`（`create_invocation_tool_runtime` / `create_worker_scope` が同一 serialize で parity 維持）→ `grants.invocation` span field record + stderr `grant:` 行 → session。grant は policy file に一切書かれず、flag 無しの次回 run で効果を持たない。
- **`workflow run --grant`**: `execute_run_inner` が同じ `build_invocation_layer` → `prepare_workflow_context` → `load_policy` で同一 merge → `grant:` stderr diagnostic を emit。`ensure_invocation` が `shared.policy.command`（grant 込み）を serialize するため parity は自動。`kind: tool` job は `inner.command` 再注入経由で grant を受ける。
- **grant-request → re-run loop**: tool-operator run が grant 欠如で `command` を縮退（`cheaper.optionalTools`）→ model が `blocked`/`partial` で `grantRequests: [{executable, argvHint?, name?}]` を emit → parent agent が `result.grantRequests` を読み → user approval → `--grant '{"name":...,"executable":...,"argv":...}'` 付き re-run → degrade 解消。cheaper 側に interactive prompt は作らない。
- **`policy grant add`**: spec-json parse → `read_policy_yaml`（absent は `{"version":1}` base）→ 非 object `command`/`grants` の pre-check・既存 name reject → grants map を畳んだ **merged full object** を `validate_policy_schema(Partial)`（`version` const・`maxProperties`・他 section の破損ごと検査）→ `stringify_yaml` → `atomic_write`（`0600`、`windows_remove_first: false`）→ utility envelope。`list` は parse + project、`remove` は同 pre-check + delete + 同じ full-object validation + 同 write path。

## Non-Functional Constraints

- **非永続性**: `--grant` の grant は `RuntimePolicy` の in-memory 表現にのみ存在し、policy file・usage.jsonl・`.cheaper/` のいずれにも書き込まれない。
- **secrets-in-argv**: `--grant` の flag 値（grant の `env` に secret を含みうる）は invoker の process cmdline（`ps`・`/proc`・shell history）に露出する。spawn される command は `env_clear` + `minimal_env(grant.env)` で起動し ambient 環境変数は子プロセスへ一切届かない（`crates/cheaper-tools/src/tools/command/mod.rs:443`・`proc/spawn.rs:154`）ため、secret は flag 値ではなく user policy（`policy.yaml`、POSIX `0600`）の grant `env` 値か子プロセス自身が読む credential file へ置く運用を docs で推奨する。`policy grant add <name> '<json>'` も spec を cmdline へ一時的に載せる同種の caveat を持つ。
- **atomic write の保証範囲**: `atomic_write` が保証するのは torn write 防止のみ — `policy grant add`/`remove` の read-modify-write は lost-update を防げない（並行 add・手編集が割り込むと last-wins）。serde_yml rewrite は comment・key 順・整形を保存しない。いずれも docs に明記する accepted caveat。
- **`grantRequests` は advisory**: model が不正確な `argvHint` を emit し得る。全 field は model 生成・prompt-injectable な untrusted 値であり、approval 判断と組み立てた spec の review は parent agent / user の責務 — cheaper は request の正しさを保証しない。docs に明記。
- **audit は 2 面のみ**: `grants.invocation` span field（log 有効時のみ）+ stderr diagnostic（`--quiet` で抑制、他 diagnostics と同規約）。`usage.jsonl` record shape は不変。
- **`policy` command の stdout 契約**: utility envelope 1 枚、`result` に top-level `tokens` key を含めない、usage.jsonl には記録しない。

## Risks and Trade-offs

- **secrets-in-argv**: flag 値が cmdline に露出する構造的 risk。child へは ambient env が届かない（`env_clear` + `minimal_env`）ため、緩和は docs warning + `policy grant add` 経由の user policy `env` / 子側 credential file への導線のみ — cmdline を経由しない `--grant-file` は Deferred へ。
- **lost-update races**: `policy grant` の read-modify-write に lock 機構は入れない（YAGNI — 単一 user の user-scope file で実害は稀、atomic write で torn write のみ防止）。
- **comment / 整形の喪失**: serde_yml rewrite が user policy の comment・key 順を消す accepted trade-off — hand-editing 併用時の注意を docs に明記。
- **`Vec<String>` forgotten-rejection hazard**: `Values.grants` が default-empty のため、将来追加される command arm が `--grant` 検査を書き忘れると silent drop になる。現行 command は per-command assert（`tests/args.rs`）で固定し、新規 command 追加時の規約として「非 run command は `grants` の非空検査を持つ」を args.rs の code comment に明記する。
- **advisory hint の誤り**: model が存在しない executable・不正な `argvHint` を emit し得る — schema は shape のみ保証し、意味的正しさは approval 側の責務（re-run 時に deny list / argv compile が再検査するため fail-closed は保たれる）。
- **attribution gap**: merged map でのみ発火する検査（`grants` 合計 `maxProperties` 超過等）は `details.layer` 無しの `InvalidRuntimePolicy` となる（(a) — accepted limitation、doc comment に明記）。
- **compile-forced 波及**: `LoadPolicyOptions` 8 site・`RunWorkerDeps` 6 site・`ParsedCli` match arm — 全て compile error で列挙されるため silent miss はないが、`cheaper-runtime` test の literal 更新が漏れやすい点として (h) に固定する。

## Deferred Design Concerns

- **`--grant-file`**: cmdline 露出を避ける file 入力経路。flag 値の secret 混入が実害化した場合に revisit（spec の範囲外として ADR が defer）。
- **`usage.jsonl` record への invocation-grant field**: fixed key list 拡張と集計意味論の設計を伴う。集計要件が実測された場合に別途検討。
- **`process.tasks` の同型 invocation-scope 機構**: spec の out of scope — 需要が生じた場合に同一 layer 機構で設計可能。
- **`policy grant` の project policy 対象・structured flag（`--executable` 等）**: spec schema との drift を生むため採用しない（ADR 却下済み）。

## Verification Notes

- [ ] AC1/AC2: grant 無しで `command` が縮退した opt-in worker へ `--grant` 付き re-run で degrade が解消し command が実 spawn する dev-seam fixture test（`command_user_policy` / `FIXTURE_COMMAND_CALL` パターン — grant spec は flag JSON 経由）。flag 無し再実行で再び縮退し `policy.yaml` が不変であることを同 test で確認。
- [ ] AC3: deny list 該当 executable の `--grant` が load 時に reject され、inner code（`INVALID_RUNTIME_POLICY`）+ `details.layer="invocation"` を持つ test。
- [ ] AC4/AC13: user policy 同名 grant との collision が `INVALID_REQUEST`（`details.grant` + `details.layer`）で、intra-invocation 同名重複も `INVALID_REQUEST` で fail する test。
- [ ] AC5: `grantRequests` を含む blocked 出力が output schema 適合（`additionalProperties: false` discipline）で envelope `result` から parse できる test。
- [ ] AC6/AC7/AC16: temp `CHEAPER_CONFIG_DIR` で `add` → `list`（name + executable）→ `remove` round-trip、invalid spec（deny 該当・argv compile 失敗・schema 違反）が file 不変で error、存在しない name の `remove` が `INVALID_REQUEST`、success envelope `result` に `tokens` key が無い test（`tests/policy.rs`）。加えて `version` 欠落/≠1・非 object の `command`/`command.grants`・他 section 破損を持つ既存 file への `add`/`remove` が merged full-object validation で `INVALID_RUNTIME_POLICY` かつ file 不変となる case。
- [ ] AC8: `--grant` run と `workflow run --grant` で stderr diagnostic 行が観測できる test（workflow の `kind: tool` job は `tool_scope` span を通らないため stderr 行が audit surface — span field は `.cheaper/logs` JSON sink 経路がなければ accessor/emit 経路で代替検証、`tools.degraded` と同じ accepted trade-off）。
- [ ] AC9: `cheaper policy` 系 command が `usage.jsonl` に record を残さない test。
- [ ] AC10: `tool-operate` alias 経由で `--grant` が同等に効く test。
- [ ] AC11: `cheaper-usage` / `cheaper-tool-operate` SKILL・guides に grant-request → approval → `--grant` re-run loop が文書化される。
- [ ] AC12: `mise run check`（dev-seam feature 付き）が pass する。
- [ ] AC14: `cheaper workflow run --grant` が `prepare_workflow_context` 経路で同一挙動となる test。`kind: tool` job が `inner.command` 再注入経由で grant を受けることを確認。
- [ ] AC15: repeatable `--grant` の parse（両形・複数指定・順序保持）と複数 grant 同時注入の args table / run 経路 case。
- [ ] AC17: 非 run 系全 command（`usage` / `count-tokens` / `models` / `auth` / `policy` / `workflow interface`）への `--grant` が全 arm で `INVALID_REQUEST` reject される command ごとの assert。
- [ ] `policy_load` golden に invocation layer case（user への追加・ordering・collision・grants 以外の key の reject）を追加。
- [ ] `proc_invocation` parity test が変更無しで green。
- [ ] 非露出 warn: `--grant` を渡したが worker が `command` を非宣言の run（`--worker-file` worker 含む）で stderr warning + 続行、`tools.allow` 除外は `POLICY_VIOLATION` で fail の test。
- [ ] POSIX で `policy.yaml` が `0600` で書かれ、`windows_remove_first: false` 経路の atomic rename が行われる write-mode test。
- [ ] regression case（`extraArgs` + `config.files` seeding）の追加。

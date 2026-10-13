---
id: "DISC-034FlIAczzK2ruZW3HTcJi"
type: "discovery"
status: "draft"
title: "invocation-scoped command grant と grant-request フローを整理する"
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
    - "../discovery/tool-operator-worker-built-in.md"
    - "../specs/tool-operator-built-in-worker.md"
    - "../specs/policy-gated-command-tool.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/invocation-scoped-command-grants.md"
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# invocation-scoped command grant と grant-request フローを整理する

## Exploration Goal

issue #105 は、`command` tool の grant を 3 つの採用済み component で拡張する:

1. **`--grant` CLI flag** — 起動時に invoker が run-lifetime の grant を注入する。model が宣言するものでも project policy が供給するものでもない。parent agent は自身で任意 command を実行できる以上、invoker が approval authority となるのは権限拡大ではなく delegation 範囲の明示である。
2. **machine-readable grant-request hint** — tool-operator の output に、blocked となった grant 要求を機械可読で載せる。parent agent が blocked を読み → user に確認 → `--grant` 付きで re-run する loop を成立させる。
3. **`cheaper policy grant add/list/remove` 管理 command** — 繰り返し必要になる grant を user policy へ validation 付きで永続化する。

この discovery が答える問い: これらを既存の policy load layering・invocation parity 契約・CLI 解析・tool output schema・永続化機構にどう配線するか。特に、grant を `RuntimePolicy.command` に畳み込む injection point と、spec / ADR に残す decision space の輪郭を確定することが目的である。

## Constraints

issue #105 由来の要求:

- grant の供給元は invoker（`--grant` flag / user policy）に限る。model 入力・project policy は grant を authoring できない（後者は `ProjectPartial` schema が `command` section 自体を reject 済み — `crates/cheaper-core/src/policy/load.rs:111-114`）
- grant の lifetime は run 単位。`--grant` は永続化しない；永続化は `cheaper policy grant` が別経路で担う
- blocked → user 確認 → `--grant` re-run の flow を parent agent が機械的に回せること（free text のみでは不十分）
- grant の validation は既存の `resolve_command_policy` 系統を再利用する。`--grant` 値・`policy grant add` 入力ともに別 validation 経路を作らない
- `command` tool の既存 contract（default deny・trusted-layer-only authoring）は不変

## Key Issues and Assumptions

確認済み（コードベース探索）:

### Policy load と injection point

- `LoadPolicyOptions`（`crates/cheaper-core/src/policy/load.rs:44-48`）は `builtin_policy_path` / `user_config_dir` / `project_dir` のみを持つ。`load_policy`（:52-132）の layer 順序: builtin（Complete schema）→ user `<userConfigDir>/policy.yaml`（Partial + `MergeMode::Replace`、:58-67）→ typed re-resolve（workflow / process / command / execution、:68-106）→ project（`ProjectPartial` + `RestrictConcurrency`、:108-126、`command` section は schema reject）→ Complete revalidate + `normalize_policy`（:128-129）。
- **最良の injection point**: user merge 直後・typed re-resolve 直前の gap（load.rs:67-70）に新 `invocation` layer を Partial validate + `MergeMode::Replace` で merge する。`resolve_command_policy`（:87）が再解決するため grant validation を全て継承し、`command` が project schema で reject される以上 project layer は override 不能。`LoadPolicyOptions` への field 追加 = signature 変更で、caller（run.rs・`workflow_context.rs:277`）の更新が要る。

### Invocation parity 契約

- `create_invocation_tool_runtime` は `options.command` を `resolve_command_policy` して `inner.command` へ格納する（`crates/cheaper-tools/src/invocation.rs:389-447`）。`create_worker_scope` は `options.runtime.command` を再 resolve して `inner.command` と `PartialEq` 比較する declared-shape parity check を持つ（invocation.rs:232-238）— scope 生成後の広い grant set の密輸防止。
- run 経路の 2 call site は同一の `deps.policy.command` を serialize する（`crates/cheaper-runtime/src/run/mod.rs:284` と `:436`）。Workflow 経路では `ensure_invocation` が `shared.policy.command` を serialize し（`crates/cheaper-cli/src/workflow_run.rs:243`）、tool job は `command: None` を渡す。同じ policy layer が `prepare_workflow_context` の `load_policy`（`workflow_context.rs:277`）を通る。
- **帰結**: grant は `RuntimePolicy.command` に畳み込む必要がある。別 field・別 param で運ぶと declared-shape parity が即座に破綻し、両 run call site + `ensure_invocation` + scope 側へ並行 plumbing が要る — 表現ドリフトの温床。

### `--grant` flag の args 表面

- `args.rs` の `STRING_OPTIONS` table（:228-）と `Values` / `set_string`（:298-318）は last-wins セマンティクス。repeatable `--grant` には `Vec<String>` + push が要り、文書化された last-wins からの deliberate deviation となる（Node `multiple: true` 相当）。
- `RunCliArgs` の populate は args.rs:903-912。alias command は `ParsedCli::Run` に fold されるため、alias 経由でも `--grant` は無料で効く。
- HELP・command-required enumeration の更新が伴う。

### `cheaper policy` 管理 command

- `ParsedCli::Policy` variant + `PolicyCliArgs`、`cli.rs` の dispatch arm、`command_name`（cli.rs:450）+ HELP、`utility_success_envelope`（cli.rs:365）が接続点。
- **新規 API が要る**: policy.yaml 読み出し helper（core の `read_yaml_layer` は `pub(crate)`）、YAML write（`yaml.rs` に `serde_yml::to_string` wrapper を置くか、JSON-as-YAML-subset を書くか）、atomic write（`crates/cheaper-runtime/src/fs_util.rs:28-59` の `atomic_write` は runtime-internal の `pub(crate)` — core へ移すか複製するか）。
- validation 再利用経路: `validate_policy_schema(Partial)` に `{"version":1,"command":{"grants":{...}}}` を流すと内部で `resolve_command_policy` が走り、deny-list / argv / env 規則が全て検査される。

### tool-operator output schema

- `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json` は全階層 `additionalProperties: false`（:10, :19, :36, :49）。`unresolved` / `nextActions` は maxLength 500・maxItems 20 の free string（:67-76）。evidence kind は `location | commit | exitCode | truncation | note` の closed enum（:24）。
- grant-request は **新規 optional top-level field**（例: `grantRequests[]`）が要る。evidence kind への追加は誤り — request は observation ではない。
- strict schema 制約（`crates/cheaper-core/src/worker/schema/strict.rs`）: `required` ⊆ `properties`、`format` 不可、union `type` は non-null 2 個まで。

### 現行の degrade 意味論

- `optionalTools` は gate が空のとき tool を degrade し（`crates/cheaper-tools/src/registry.rs:1091-1119`）、`degraded_names()`（:732）で宣言順に返す。prompt note が利用不可 tool を列挙し、result の `status` は `blocked` を取りうる。**機械可読な grant request は現状存在せず free text のみ**。`tools.degraded` span field が先例（registry.rs:1150-1156）。

### Grant validation 再利用 map

- `resolve_command_policy`（`crates/cheaper-core/src/policy/command.rs:283`〜 — schema 検査、deny 2-stage、`ArgPattern::compile`、cwd / env 規則、defaults）、`is_denied_command_executable`（:170）、`resolve_command_grant`（:436）。
- flag 値 → `serde_json::from_str` → `json!({"grants":{<name>:<spec>}})` → `resolve_command_policy` で full validation が得られる。専用 `key=value` mini-format は validation を再実装するため却下済み。

### Audit 面の選択肢

- `usage.jsonl` record は fixed key list — いずれにせよ code 変更が要る。span field 先例 `tools.degraded` に倣う `grants.invocation` が有力。stderr diagnostic line も選択肢。

### Test 表面

- `tests/args.rs` の parse table、dev-seam の Fixture + fake_stream（`run_dev_seam.rs` — `--grant` → degrade 解消 → command usable の E2E が書ける）、regression harness（`extra_args` + `config.files` で user policy seeding）、core の golden `policy_load`、`proc_invocation` parity test。

## Alternatives and Comparison

### 軸1: invocation grant の格納先

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `load_policy` に `invocation` layer を新設（user merge 直後・Partial + Replace） | grant validation を `resolve_command_policy` 経由で全継承。`RuntimePolicy.command` に畳み込まれるため parity 契約を自動的に満たす。project は override 不能 | `LoadPolicyOptions` signature 変更と caller 更新。layer 追加で load 順の意味論が増える | 候補 |
| B: `RuntimePolicy` とは別の field / param で grant を運ぶ | load.rs 不変 | `create_worker_scope` の declared-shape parity が即破綻。run/mod.rs 2 箇所 + `ensure_invocation` + scope 側への並行 plumbing = 表現ドリフト | 却下寄り |
| C: `--grant` 値を user policy 相当 layer として user merge 前に流す | layer 追加なしで Replace に乗る | user policy が invocation grant を上書き可能になり「invoker が approval authority」が崩れる。順序の意味も不自然 | 却下寄り |

### 軸2: `--grant` flag 値の形式

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `--grant '{"<name>":{...spec}}'`（`{name:spec}` map JSON） | `json!({"grants":...})` に直接畳め、`resolve_command_policy` の validation がそのまま効く | shell quoting 面倒。name を JSON 内に埋める | 候補 |
| B: `--grant <name>={...spec}`（name を JSON の外に） | name が flag 上で視認しやすい | 分割 parse が一段増える。spec 側に name が混入したときの扱いを定める要あり | 候補 |
| C: 専用 `key=value` mini-format（executable=..., argv=...） | shell friendly | grant spec の validation（deny / ArgPattern / cwd / env）を再実装することになる | 却下済み |

### 軸3: flag と user policy の同名 collision

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: invocation layer が後段にあるため replace wins | layer 規則の自然な帰結。error path を増やさない | user policy の grant が静かに上書きされる — audit 面での可視化が要る | 候補 |
| B: 同名衝突を explicit error にする | 意図しない上書きを防ぐ | merge 層に name-aware な衝突検出を入れるか caller 側で事前比較する要あり | 候補 |

### 軸4: grant-request hint の形状

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `{name?, executable, argv hint?}` を持つ `grantRequests[]` optional top-level field | parent agent が `--grant` 値を機械生成できる。evidence enum を汚さない | output schema 拡張。hint に含める粒度の設計が要る | 候補 |
| B: executable のみの最小 hint | schema 変更小 | argv 制約まで分からず re-run の grant spec を parent が推測し直す | 候補 |
| C: `unresolved` / `nextActions` の free text に定型文を入れる | schema 変更不要 | machine-readable 要求を満たさない | 却下寄り |

### 軸5: `cheaper policy grant` の UX 形状

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: structured flags（`add <name> --executable ... --argv ...`） | CLI 慣習に沿う。各 flag を validate できる | grant spec の全 field を flag 化する表面積が大きい | 候補 |
| B: JSON 引数（`add <name> '<spec-json>'`） | spec shape と 1:1、`resolve_command_policy` に直通 | 手入力の UX は劣る | 候補 |

### 軸6: user policy の YAML 書き込み方式

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `yaml.rs` に `serde_yml::to_string` wrapper | YAML として自然に round-trip | serializer 依存を policy write 経路へ追加 | 候補 |
| B: JSON を書く（YAML subset として valid） | 新規 serializer 不要 | 人間が読む user policy が JSON になる | 候補 |

### 軸7: audit surface

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `usage.jsonl` record に field 追加 | 永続・集計可能 | record schema は fixed key list — 拡張は code 変更 | 候補 |
| B: tracing span field（`tools.degraded` 先例 → `grants.invocation`） | 既存先例に一致、実装軽い | log 有効時のみ観測可能 | 候補 |
| C: stderr diagnostic line | 常時可視 | 構造化されず集計不能 | 候補 |

## Tentative Conclusions and Hypotheses

- 確認済み: injection point は load layer 一択に近い。`create_worker_scope` の declared-shape `PartialEq` check（invocation.rs:232-238）が `RuntimePolicy.command` への畳み込みを事実上強制しており、side-field 案（軸1-B/C）は parity を破るか全経路への並行 plumbing を要求する。
- 確認済み: `--grant` は `Vec<String>` + push が必須で、args.rs の last-wins 規約からの deliberate deviation。alias は `ParsedCli::Run` fold により無料で対応。
- 仮説: tool-operator output には `grantRequests[]` のような新規 optional top-level field が要る。`additionalProperties: false` と evidence enum の閉鎖性から、既存 field への埋め込みは schema を壊すか意味論を歪める。
- 確認済み: flag 値 → JSON → `{"grants":{...}}` → `resolve_command_policy` の validation 再利用経路は end-to-end で存在する。`key=value` mini-format（軸2-C）は validation 再実装のため却下。
- 確認済み: `cheaper policy grant` は core に新規 public API を要する — policy.yaml read helper・YAML write・atomic write の 3 点が現状 crate-internal。
- 仮説: invoker が approval authority という根拠により、`--grant` は権限昇格ではなく delegation gate の narrowing 指定であり、prompt injection 経路（model 出力 → grant authoring）は設計上そもそも存在しない。

## Open Questions

- `--grant` 値の JSON 形式詳細: name を JSON 内に含める `{name:spec}` map か、`<name>={spec}` の分割形式か（軸2）。
- collision 意味論: flag と user policy の同名 grant — invocation layer の replace wins か explicit error か（軸3）。
- workflow への `--grant` 適用範囲: `cheaper workflow run` も `--grant` を受けるか。tool job は `command: None` を渡すため、適用するなら `ensure_invocation` 側の policy に畳む形になる。
- `grantRequests[]` の shape: executable + argv hint まで含めるか executable のみか。emit は `status: blocked` のみか、`partial`（degrade しつつ成功）でも出すか（軸4）。
- `cheaper policy grant` の UX: structured flags か JSON arg か（軸5）。
- YAML write アプローチ: `serde_yml::to_string` wrapper か JSON-as-YAML-subset か（軸6）。
- audit surface: usage.jsonl field / span field / stderr line のどれを採るか（複数可）（軸7）。
- `--grant` を渡したが worker が `command` を expose していない場合に warning を出すか。

## Scope

### In Scope

- `run`（alias 含む）への repeatable `--grant` flag と `load_policy` の `invocation` layer。
- tool-operator output への machine-readable grant-request field。
- `cheaper policy grant add/list/remove`（user policy への validation 付き永続化）。
- 上記に伴う audit 面・test 表面の拡張。

### Out of Scope

- model 出力経由の grant authoring（設計上存在しない）。
- project policy への grant 永続化（`ProjectPartial` が `command` section を reject する trust boundary は維持）。
- `process.tasks` への同型 invocation-scope 機構。
- cheaper 内部での interactive confirm prompt（確認は parent agent 側の責務）。

## Promotion Candidates

- [x] spec-doc needed: `--grant` flag 契約（repeatable・JSON 形式・collision 意味論）、`grantRequests` output field shape、`cheaper policy grant` command UX、audit surface の製品要件。→ `../specs/invocation-scoped-command-grants.md`（SPEC-034FlOz4hiN9m0nQW7lBN2）へ promote 済み。
- [x] adr-doc needed: `load_policy` への `invocation` layer 挿入位置と ordering 意味論、parity 契約との整合、collision 意味論の決定。→ `../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`（ADR-034FlTS3BaiV3WKz8bX1Xv）へ promote 済み。

## Sources and Relations

source は issue #105。背景: `command` tool の trust boundary（trusted-layer-only grant authoring・default deny）は `../specs/policy-gated-command-tool.md` と discovery `policy-gated-command-tool`、本機能の主要 consumer である tool-operator worker の degrade 意味論は `../discovery/tool-operator-worker-built-in.md` と `../specs/tool-operator-built-in-worker.md` を参照。

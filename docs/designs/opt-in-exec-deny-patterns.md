---
id: "DESIGN-034KLwE8GiJQ2zvQY20ob8"
type: "design"
status: "draft"
title: "opt-in security.execDenyPatterns profile — A9 の降格実装"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-core/src/policy/security.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/mod.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_runtime.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/characterization.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/configuration.md"
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
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../adr/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../discovery/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/exec-contract-content-audit-removal.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../specs/exec-contract-content-audit-removal.md"
    - "../discovery/exec-contract-content-audit-removal.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../../crates/cheaper-core/src/policy/schema.rs"
    - "../../crates/cheaper-core/src/policy/command.rs"
    - "../../crates/cheaper-core/src/policy/load.rs"
    - "../../crates/cheaper-tools/src/tools/command/mod.rs"
    - "../../crates/cheaper-tools/src/registry.rs"
    - "../../crates/cheaper-tools/src/invocation.rs"
    - "../../crates/cheaper-cli/src/route/resolve.rs"
    - "../../crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# opt-in security.execDenyPatterns profile — A9 の降格実装

## Context

issue #164（#151 実装分解 2/7）は、#163 で mandatory 層から除去された
A9 `command_denied_executable_re` deny list を、operator opt-in の
deterministic executable deny-list `security.execDenyPatterns` として
再配置する。spec（SPEC-034KLgfJ0Gka7fkRKHpfSH・proposed）と
ADR（ADR-034KLgkao8SgWWKIIsCqaB・proposed — Option A: top-level
`security` 節・regex 配列・stem 完全一致 + case-insensitive OR・
load 時 fail-closed・3 適用点への signature 配管）で方針は確定済み
であり、discovery（DISC-Fad7xt2qdtfpcCHhjMhkVy・resolved）が
schema 挿入点・call site inventory・旧 builtin regex を file:line
精度で固定している。

本 design は「新 module の API 形状・options/signature 配管の具体形・
各適用点の error 文言・test/docs の配置」を実装契約として固定する。
引用する file:line は全て branch `feature/issue-164-exec-deny-patterns`
の現行 code で実測済み。

## Scope

### In Scope

- 新 module `crates/cheaper-core/src/policy/security.rs` —
  `security` section schema・`SecurityPolicy`・
  `resolve_security_policy`・`ExecDenyList`（compiled deny
  predicate）・`executable_stem`（basename 正規化）の集約。
- policy schema への `security` 節挿入（complete/partial schema の
  optional property・project partial からの除外・
  `validate_policy_schema`/`normalize_policy`/`load_policy` の
  section resolve・`RuntimePolicy.security` field 追加）。
- deny 判定の 3 適用点注入（`resolve_command_policy`・
  `pin_command_grant(s)`・`verify_pinned_executable`）と
  `RuntimePolicy.security` → options → 各 signature の配管。
- error 分類と文言の固定（resolve 段階 `InvalidRuntimePolicy`・
  pin/verify 段階 `PolicyViolation` + `grant_violation` 規約）。
- test・golden fixture・docs（`command.md`・`configuration.md`・
  builtin `resources/policy.yaml`）の更新。

### Out of Scope

- `process.tasks` executable への deny 適用 — 別 trust boundary
  （project も authoring できる fixed-argv task。spec Out of Scope
  と同一）。
- deny の CLI flag・env var 注入、glob 等の非 regex 記法、merge
  意味論の変更（配列 wholesale replace 維持）。
- `security` 節への他 key の追加（本 issue は
  `execDenyPatterns` のみ）。
- 文書全体の正直性 sweep（issue #169 の scope）。
- deny による viability/reporting UX の変更（`Viability::NoGrant`
  へ畳まれる現行分類のまま）。

## Components and Boundaries

### 新 module — `crates/cheaper-core/src/policy/security.rs`

`command.rs`（section schema + predicate 同居）と
`filesystem.deny_paths`（resolved string 列 → `PathDenylist`
compiled artifact）の 2 先例を合成した責務配置。宣言形
（serde-native・`PartialEq`）と compiled 形（`ExecDenyList`）を
分離する — これが本 design の中核境界。

```rust
/// `security` section の JSON schema properties。
pub fn security_policy_properties() -> Value;

/// resolved `security` section(trusted layer のみ authoring 可能 —
/// `command`/`route` と同型に project partial schema は section
/// 全体を除外する)。serde-native: pattern source string を保持し、
/// compiled regex は載せない(`CommandGrant.argv` と同じ規約 —
/// invocation parity check が declared-shape equality を見るため)。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SecurityPolicy {
    #[serde(rename = "execDenyPatterns")]
    pub exec_deny_patterns: Vec<String>,
}

/// raw `security` section(`Option<&Value>`)を resolved
/// `SecurityPolicy` へ passthrough 解決する。`None`/`null`/非
/// object は `[]` 等価(`route` と同じ schema-gated 前提 —
/// 値 domain は schema が保証済み、item は string のみ収集)。
/// **compile は行わない** — compile 境界は `ExecDenyList` のみ。
pub fn resolve_security_policy(
    input: Option<&Value>,
) -> Result<SecurityPolicy, CheaperError>;

/// `security` section → compiled deny predicate の一回経路
/// (resolve + compile)。deny 注入点が raw JSON から直接 compiled
/// 形を得るための helper。
pub fn resolve_exec_deny_list(
    input: Option<&Value>,
) -> Result<ExecDenyList, CheaperError>;

/// compiled executable deny predicate — operator 宣言 pattern 集合。
/// 空集合(既定)は何も deny しない。`Default` = 空。
#[derive(Debug, Clone, Default)]
pub struct ExecDenyList {
    patterns: Vec<regex::Regex>,
}

impl ExecDenyList {
    /// 各 pattern を `^(?:pattern)$` 相当の stem 完全一致・
    /// case-insensitive で compile。compile 不能な pattern は
    /// `InvalidRuntimePolicy` で fail-closed(`ArgPattern::compile`
    /// の compile 時拒否と同型)。pattern 配列は OR 評価。
    pub fn compile(patterns: &[String]) -> Result<Self, CheaperError>;

    /// declared/resolved executable 文字列を `executable_stem` で
    /// 正規化した stem へ OR match。空集合は常に false。
    pub fn is_denied(&self, executable: &str) -> bool;

    /// deny 無効化の判定用(predicate 注入の有無ではなく集合の空で
    /// 表す — `exec_deny.is_empty()` が既定 no-op の同値条件)。
    pub fn is_empty(&self) -> bool;
}

/// basename 正規化 — `/`・`\` 区切りの最終要素 → 小文字化 →
/// `.exe`/`.com` suffix を 1 回 strip → stem。declared spec と
/// resolved canonical path の両方に同一適用(旧
/// `is_denied_command_executable` の規約を引き継ぐ)。
pub fn executable_stem(executable: &str) -> String;

/// `command_invalid` と同型の error helper —
/// `"invalid security policy: {reason}"` + `InvalidRuntimePolicy`。
fn security_invalid(reason: &str) -> CheaperError;
```

確定する意味論:

- **pattern 記法** — `regex` crate の regex(`cheaper-core` の
  workspace dep 済み)。各 pattern は
  `RegexBuilder::new(&format!("^(?:{pattern})$"))`
  `.case_insensitive(true)` で個別 compile する。per-pattern
  compile は失敗 pattern を error `details` に載せる attribution
  のため(1 本の合成 `p1|p2|…` regex にしない)。
- **anchoring** — 全 pattern を `^(?:pattern)$` で wrap する
  stem 完全一致。unanchored `is_match` にすると `sh` が
  `bash`/`sshd` に substring match する false positive(deny の
  誤発動 = 正当 grant の拒否)を生じる。substring が欲しい
  operator は `.*` で明示する。
- **case** — stem は小文字化されるが、pattern 側は
  `case_insensitive(true)` で compile する(二重適用で
  `Python` のような mixed-case 宣言も `python` に match — ADR
  の「pattern も case-insensitive 評価」を満たす)。
- **match 対象** — `executable_stem` は旧実装と同一
  (`rsplit(['/', '\\']).next()` → `to_lowercase()` →
  `.exe`/`.com` を 1 回 strip)。declared(raw spec 文字列)と
  resolved(canonical path)で同一正規化。multiline flag は
  立てないため `^`/`$` は絶対 anchor。
- **invalid pattern** — `ExecDenyList::compile` が
  `security_invalid("invalid execDenyPatterns regex")`
  (`"invalid security policy: invalid execDenyPatterns regex"` +
  `InvalidRuntimePolicy`)を返す。`details.pattern` に失敗した
  source string、`.with_cause(regex_err)` に regex error を載せる。
  warn-and-skip は deny を silent に弱めるため不採用(ADR 確定)。
- **空** — 未指定・`null`・`[]` は全て空集合 = deny 無し
  (`is_denied` は常に false)。`exec_deny.is_empty()` が no-op
  判定の単一手段。

### schema 契約 — `security_policy_properties()`

```rust
json!({
    "execDenyPatterns": {
        "type": ["array", "null"],
        "maxItems": 128,
        "uniqueItems": true,
        "items": {"type": "string", "minLength": 1, "maxLength": 4096},
    },
})
```

- `["array","null"]` は `route.rewrite.approvalBypass` の nullable
  先例(schema.rs:274-280)— 明示 `null` ≡ 未指定 ≡ `[]`。
- `maxItems: 128` は `grants` の `maxProperties: 128`(command.rs:99)
  と同じ bounding 方針。`items` の bound は `bounded_string`
  (minLength 1・maxLength 4096、command.rs:86)に揃える — 旧
  builtin regex 単一要素(約 700 文字)も余裕で収まる。
- `uniqueItems: true` — OR 評価で重複は意味を持たないため fail-loud
  (`approvalBypass` 先例)。

### schema 配線 — `crates/cheaper-core/src/policy/schema.rs`

| site | lines(現行) | 変更 |
|------|------------|------|
| `policy_complete_schema()` | 287-312 | properties に `"security": object_schema(&[], security_policy_properties())` を追加。**required には加えない**(`route` 先例 — optional section、既存 complete policy との後方互換) |
| `policy_partial_schema()` | 316-333 | 同上(user 層の authoring 入口) |
| `policy_project_partial_schema()` | 340-355 | **追加しない** — `additionalProperties: false` が `security` key を含む project policy を `InvalidRuntimePolicy` で拒否(`command`/`route` と同型の section 丸ごと除外 fail-closed) |
| `validate_policy_schema()` | 401-433 | schema 検査後の section resolve で、まず `let exec_deny = resolve_exec_deny_list(policy.get("security"))?;` を**無条件に**評価(`command` 欠落でも security-only doc の invalid regex を layer 単位で fail-closed)し、`if let Some(command)` ブロック(:429-431)で `resolve_command_policy(Some(command), &exec_deny)` を呼ぶ |
| `normalize_policy()` | 501-633 | `RuntimePolicy.security` を `route` と同型に passthrough 解決 — `let security = resolve_security_policy(raw.get("security"))?;` + `let exec_deny = ExecDenyList::compile(&security.exec_deny_patterns)?;`(1 resolve・1 compile)で `command: resolve_command_policy(raw.get("command"), &exec_deny)?`、struct literal に `security` を畳み込む |
| `RuntimePolicy` struct | 124-140 | `route` field(:133)の直後に `pub security: SecurityPolicy` を追加(trusted-layer 限定 section の隣接配置 — serialized 出力は serde_json `BTreeMap` 整列で `"route"` と `"tools"` の間に現れる) |

### signature 契約(変更後)

| function | 変更後 signature | 備考 |
|----------|-----------------|------|
| `resolve_command_policy` | `(input: Option<&Value>, deny: &ExecDenyList) -> Result<ResolvedCommandPolicy, CheaperError>` | `command.rs:232`。raw `security` ではなく **compiled** `&ExecDenyList` を取る — caller が compile 境界を 1 回だけ持ち、options 経由の再 resolve でも同じ deny 集合を共有する。test 呼出しは `&ExecDenyList::default()` |
| `pin_command_grants` | `(policy, root, symlink_policy, denylist, exec_deny: &ExecDenyList) -> Result<BTreeMap<String,String>, ToolError>` | `command/mod.rs:167-179`(pub(crate)) |
| `pin_command_grant` | `(name, grant, root, symlink_policy, denylist, exec_deny: &ExecDenyList) -> Result<String, ToolError>` | `command/mod.rs:185-202`。`_name` → `name`(deny error の `grant_violation` で使用 — discovery Open Question 確定) |
| `verify_pinned_executable` | `(name, pinned, exec_deny: &ExecDenyList) -> Result<(), ToolError>` | `command/mod.rs:210-224`(private)。唯一の呼出しは `check_command_execution`(:391-394)の `options.exec_deny` |
| `check_command_execution` | **不変** `(options, name, args)` — `CommandToolOptions` の新 field 経由で deny を読む | `command/mod.rs:368` |

`resolve_command_policy` に compiled `&ExecDenyList` を渡す選択理由
(ADR Option A の確定形): raw `Option<&Value>`(security section)を
取る案は caller ごとに compile を繰り返し、`security` section の
解釈責務が `command.rs` へ漏れる。compiled 注入にすると
`command.rs` は「deny 集合に対する照合」だけを知り、section
schema/compile の責務は `security.rs` に閉じる。

### options/struct 配管

| struct/field | 型 | 根拠 |
|--------------|-----|------|
| `ToolRuntimeOptions.security` | `Option<Value>` | `registry.rs:90-133`。`command`/`process`/`concurrency` と同じ raw JSON 規約 — caller は `serde_json::to_value(&policy.security)` を渡し、`create_tool_runtime` 内で `resolve_exec_deny_list(options.security.as_ref())` で compiled 化 |
| `InvocationToolRuntimeOptions.security` | `Option<Value>` | `invocation.rs:61-78`。同上 |
| `InvocationInner.security` | `SecurityPolicy` | parity check の比較対象(`inner.command` と同型の declared-shape 保持) |
| `InvocationInner.exec_deny` | `ExecDenyList` | scope 構築・parity 再 resolve・再注入で共有する compiled 形 |
| `ToolCtx.exec_deny` | `ExecDenyList` | `registry.rs:213-233`。`command()` が `CommandToolOptions` へ clone |
| `CommandToolOptions.exec_deny` | `ExecDenyList` | `command/mod.rs:233-244`。`denylist: Option<PathDenylist>` 先例の owned field。非 Option — 空 `Default` が no-op を表現する単一の形 |
| `CommandPolicyView.exec_deny` | `&'a ExecDenyList` | `route/resolve.rs:69-76`。全 field borrowed の struct 規約に従い `&'a`(`Copy` 維持) |
| `RouteSupply.exec_deny` | `ExecDenyList` | `route/supply.rs:28-35`。`load_with_builtin` が `ExecDenyList::compile(&policy.security.exec_deny_patterns)?` で populate(load 済み policy 由来のため compile 失敗は到達不能想定だが `?` で伝搬) |

### call site 配線(全一覧)

| site | lines(現行) | 変更内容 |
|------|------------|----------|
| `load.rs` | 208 | `resolve_command_policy(effective.get("command"))` → `let exec_deny = resolve_exec_deny_list(effective.get("security"))?; resolve_command_policy(effective.get("command"), &exec_deny)`。**merged effective(含む invocation grant)への deny 評価点** — merge 直後の typed re-resolve で発火 |
| `registry.rs` | 1114-1115 | `resolve_exec_deny_list(options.security.as_ref())` で `exec_deny` を得て `resolve_command_policy(options.command.as_ref(), &exec_deny)` |
| `registry.rs` | 1208-1214 | `pin_command_grants(&command, &resources.root, symlink, denylist, &exec_deny)` |
| `registry.rs` | 213-233,317-327 | `ToolCtx.exec_deny` field 追加、`command()` が `exec_deny: self.exec_deny.clone()` を `CommandToolOptions` へ |
| `invocation.rs` | 390-398 | `create_invocation_tool_runtime` で `let security = resolve_security_policy(options.security.as_ref())?;` + `let exec_deny = ExecDenyList::compile(&security.exec_deny_patterns)?;` → `resolve_command_policy(options.command.as_ref(), &exec_deny)`。`InvocationInner` に `security`・`exec_deny` を保持 |
| `invocation.rs` | 219-238 | parity check — `options.runtime.security` が `Some` なら `resolve_security_policy(Some(raw))` で再 resolve し `inner.security` と `PartialEq` 比較、不一致は `invalid_binding("security policy mismatch")`(`InvalidRequest` + `details.reason`)。`options.runtime.command` の再 resolve には `&inner.exec_deny` を使う |
| `invocation.rs` | 271-274 | scope 再注入 — `security: Some(serde_json::to_value(&inner.security).expect(...))` を `runtime_options` へ追加(`inner.command` と同型: caller の `None`/任意 raw に関わらず scope は常に invocation-resolved の security を受け取る) |
| `run/mod.rs` | 335-354 | `InvocationToolRuntimeOptions` に `security: Some(serde_json::to_value(&policy.security).unwrap_or_default())` |
| `run/mod.rs` | 487-524 | worker scope `ToolRuntimeOptions` に同じく `security: Some(serde_json::to_value(&policy.security).unwrap_or_default())`(parity check 入力 — `command` と同型) |
| `workflow_run.rs` | 236-271 | `ensure_invocation` の `InvocationToolRuntimeOptions` に `security: Some(serde_json::to_value(&shared.policy.security).map_err(InternalError)?)` |
| `workflow_run.rs` | 491-518 | `kind: tool` worker scope `ToolRuntimeOptions` に `security: None`(`command: None` と同型 — parity skip、inner の値が再注入される) |
| `route/supply.rs` | 104-123 | `load_with_builtin` で `ExecDenyList::compile(&policy.security.exec_deny_patterns)?` → `RouteSupply.exec_deny`;`command_view()`(:40-47)が `exec_deny: &self.exec_deny` を `CommandPolicyView` へ |
| `route/resolve.rs` | 311-319 | viability の `pin_command_grant` 呼出しに `input.command_policy.exec_deny` を渡す |
| `route/resolve.rs` | 320-332 | viability の `CommandToolOptions` literal に `exec_deny: input.command_policy.exec_deny.clone()` |
| `hook/routing_policy.rs` | 142 | `RouteEvalInput.command_policy` は `CommandPolicyView` — view が deny-aware になるため本経路は自動適用(hook eval の viability も同一 deny を見る)。signature 変更不要、test literal(:2100-2106)のみ追随 |

### 適用点と error 契約

| 適用点 | site(現行) | 挿入位置 | error |
|--------|-----------|----------|-------|
| 1. declared basename | `command.rs` grant loop :285-291 | `executable` 抽出直後(argv compile 前 — 旧 `:377-381` と同じ相対位置) | `command_invalid("executable is denied by security.execDenyPatterns")` → `"invalid command policy: executable is denied by security.execDenyPatterns"` + `InvalidRuntimePolicy`。`details.grant`(grant 名)・`details.executable`(declared spec)を `with_details` で載せる — pin 段階の `details.grant` 規約と揃え、deny した spec を operator が特定可能にする |
| 2. resolved basename | `command/mod.rs` `pin_command_grant` :192-201 | `resolve_process_executable` の戻りを `let resolved = ...?;` で受け、`exec_deny.is_denied(&resolved)` で `return Err(grant_violation(name, "resolved executable is denied by security.execDenyPatterns"))` | `PolicyViolation` + `details.grant`(`command grant "{name}": resolved executable is denied by security.execDenyPatterns`) |
| 3. spawn 前再検証 | `command/mod.rs` `verify_pinned_executable` :210-224 | pin-equality(`resolved != pinned`、:217-222)の直後に `exec_deny.is_denied(&resolved)` → 同規約 `grant_violation` | 同上 |

適用点 3 は構造的に pin 時判定と等価(pin map は scope 構築時点で
immutable、pin-equality が path 同一性を強制)だが、「全適用点で同一
predicate を再適用する」旧モデルの uniform 契約を維持するため残す
(discovery 適用点 3 注記と同一判断)。

### layer-agnostic 適用と attribution 規約

resolved `command.grants` map は layer 由来情報を持たず
(`CommandGrant` — `command.rs:35-48` に layer field 無し)、
`load_policy` は invocation 層を user merge 直後・typed re-resolve
前に `MergeMode::Replace` で畳み込む(`load.rs:119-188`)。したがって:

- **invocation 合成 grant(`--grant`)も deny 対象** — `load.rs:208`
  の merged re-resolve で `effective.security` の deny が invocation
  由来 grant を含む全 grant に及ぶ(R7)。layer provenance の
  後付け exempt は行わない(ADR Option D 棄却)。
- **attribution gap(accepted)** — invocation layer 自身の
  `validate_policy_schema(Partial)`(`load.rs:155-159`)は `security`
  key を持たないため empty deny で通り、merged re-resolve(:208)の
  deny error は `InvalidRuntimePolicy` だが `details.layer` を
  **持たない**。deny 元(builtin/user layer)と grant 元
  (invocation)が別 layer であり、merged-map 評価の帰属不能は
  `load.rs:18-24` に既に文書化された attribution gap と同じ構造 —
  error code は `with_invocation_layer` 経由時と同じ
  `InvalidRuntimePolicy` で揃う。
- **invocation layer が `security` を書けない構造保証は既存で
  変更不要** — `load.rs:124-131` の top-level keys ⊆
  `{version, command}` guard が `security` を未知 key として
  `InvalidRequest` + `details.layer="invocation"` で拒否し、
  `:132-148` の `command` keys ⊆ `{grants}` guard が
  `command.security` も同様に拒否する。
- **`policy grant add` の authoring 境界** — `policy.rs:251-252`
  の `write_validated` が merged full object を
  `validate_policy_schema(Partial)` で検査するため、user policy
  file 自身の `security.execDenyPatterns` に match する executable
  spec は add 時点で `InvalidRuntimePolicy`(fail-closed)。code
  変更不要 — module doc(:8-9)の検査内容列挙に deny 照合を
  追記するのみ。

### merge 意味論

`merge_policy_layers`(`policy/merge.rs:42-109`)は object
deep-merge + array wholesale replace のため変更不要。user 層の
`security.execDenyPatterns` は builtin の配列を**丸ごと置換**
(`tools.allow` と同じ replace 意味論 — union ではない)。
builtin が `[]` の現行構成では「user 層が宣言した配列がそのまま
effective」になる。`RestrictConcurrency` mode の field 一覧
(`RESTRICT_*_FIELDS`)に `security` は加えない — project 層は
section 自体を書けないため到達不能。

`restrict_policy`(`policy/restrict.rs:100-103`)は
`RuntimePolicy` の `..policy.clone()` で `security` をそのまま
運ぶ — code 変更不要。

## Data and Control Flow

```
policy.yaml layers          load 経路                          runtime 経路
─────────────────────────────────────────────────────────────────────────
builtin (Complete) ──┐
user (Partial)       ──┼─ validate_policy_schema ── resolve_exec_deny_list(security)
                     │      ├─ schema(complete/partial/project-partial)
                     │      └─ resolve_command_policy(command, &deny)  … 適用点1
invocation --grant   ──┤   (構造 guard: keys ⊆ {version, command.grants} —
                     │     security key は InvalidRequest + layer)
                     ├─ merge_policy_layers(Replace ×2 / RestrictConcurrency)
                     ├─ typed re-resolve: resolve_exec_deny_list(effective.security)
                     │      → resolve_command_policy(effective.command, &deny)
                     │      … 適用点1 が merged map(invocation grant 含む)に発火
                     ├─ project (ProjectPartial — security key で schema reject)
                     └─ normalize_policy → RuntimePolicy.security: SecurityPolicy

RuntimePolicy.security ── to_value ──→ ToolRuntimeOptions.security /
                                     InvocationToolRuntimeOptions.security
   create_invocation_tool_runtime: resolve_security_policy + ExecDenyList::compile
     → inner.{security, exec_deny}
   create_worker_scope: parity(inner.security) → runtime_options.security 再注入
   create_tool_runtime: resolve_exec_deny_list(options.security)
     → resolve_command_policy … 適用点1
     → pin_command_grants(…, &exec_deny) … 適用点2
     → ToolCtx.exec_deny → CommandToolOptions.exec_deny
        → check_command_execution → verify_pinned_executable … 適用点3
   route: RouteSupply.exec_deny → CommandPolicyView.exec_deny
        → viability(pin_command_grant + CommandToolOptions) … 適用点2/3 相当
```

## Non-Functional Constraints

- **regex compile/match cost** — pattern は `maxItems: 128`・
  `items.maxLength: 4096` で上限済み。compile は layer ごとの
  `validate_policy_schema`・`load.rs` re-resolve・invocation/scope
  構築時に発生するが、いずれも run 単位の一過性 cost(数十
  pattern × 数 10µs 規模)で pin/execute の hot path には乗らない。
  `regex` crate は finite automaton 評価のため backtracking 爆発
  (ReDoS)は構造的に起きない — operator 宣言 pattern の match
  cost は入力長に線形。
- **deny × route viability** — denied grant の pin は
  `pin_command_grant` で `Err` になり、`resolve.rs:311-319` の
  viability 評価は `Viability::NoGrant` へ畳む(unpinnable と同じ
  分類 — error ではなく route 不成立)。ただし declared stem が
  match する grant は `load_policy` 段階で既に全体が失敗するため、
  viability まで到達する resolved-stem deny は declared ≠ resolved
  の rename/symlink 経路に限定される観測。
- **表現規約** — 「deterministic executable deny-list」と記述し、
  「保護」「sandbox」「containment」とは呼ばない(issue・spec R12)。
  名一覧方式の構造的 bypass(改名・別 tool・子プロセス経由)は
  docs で正直に明記する。
- **serde-native 維持** — `SecurityPolicy` は source string 列で
  `PartialEq`/`Serialize` を持ち、compiled `ExecDenyList` は
  serialize しない。round-trip `to_value → resolve` が同値を
  返すため invocation parity check が成立する(`CommandGrant.argv`
  と同じ規約)。
- **既存挙動の不変性** — 既定(空)では `is_denied` が常に
  false のため全適用点は no-op。#163 後の既定挙動は一切変わらない。

## Risks and Trade-offs

- **deny の誤発動(false positive)** — unanchored match を避ける
  ため stem 完全一致 + `(?i)` を強制。operator が `sh` を書くと
  `bash` には match しない — substring 意図は `.*` で明示させる
  ことで「executable 名の deny-list」mental model を保つ。
  mixed-case pattern も小文字化 stem + case-insensitive compile の
  二重正規化で silent 不発を防ぐ。
- **merged deny の attribution gap** — invocation grant への deny
  は `load.rs:208` で `InvalidRuntimePolicy` だが
  `details.layer="invocation"` を持たない(load.rs:18-24 の既存
  attribution gap と同構造)。error code は揃うため wire 上の
  分類影響は無い — accepted として本 design で固定。
- **policy 全体 fail の波及面** — invalid regex・deny-matching
  grant は `load_policy` 全体を失敗させるため、route arm の
  fail-open(`load_or_none` — `supply.rs:127-141`)では route arm
  全体が不活性になる。これは「invalid policy を silent に
  ignore しない」既存 convention と整合 — hook eval の不活性化は
  R39 の規約範囲。
- **pin 署名 churn** — `resolve_command_policy`/`pin_command_grant(s)`/
  `verify_pinned_executable`/`CommandToolOptions`/`CommandPolicyView`/
  `ToolRuntimeOptions`/`InvocationToolRuntimeOptions`/`RuntimePolicy`
  の field/param 追加で、call site・test literal・golden fixture に
  広い churn が生じる(下表)。機能差分は適用点の deny 評価のみで、
  残りは機械的な型追随。
- **structural bypass の残存** — 名一覧方式は改名・別 tool・
  plugin/extension 機構・子プロセス経由で構造的に bypass 無限。
  containment を主張せず「既知名への fail-fast」と正直に記述する
  前提を引き継ぐ(#163 ADR・親 design と同一)。
- **compile タイミングの重複** — `RuntimePolicy` 構築から scope
  構築までに複数回 compile する経路がある(load re-resolve・
  validate・runtime/invocation 構築・route supply)。各 site が
  independent に fail-closed する設計であり、compiled 形を
  `RuntimePolicy` に載せる案(serialize 不能・parity 破壊)より
  コストは小さい — YAGNI で共有 cache は設けない。

### Churn 一覧(実測 — signature/field 追随)

- `resolve_command_policy` 呼出し更新 — `schema.rs:430`・`:570`・
  `load.rs:208`・`registry.rs:1115`・`invocation.rs:234,398`・
  `command.rs` 内 test(:416,424,430,455,461,508 他)・
  `proc_command.rs:409→430,1551→1560`・`route/resolve.rs:385`・
  `routing_policy.rs:2068`。
- `pin_command_grant`/`pin_command_grants` 呼出し更新 —
  `command/mod.rs:175`・`route/resolve.rs:311`・
  `proc_command.rs:1577`・`registry.rs:1209`・mod.rs 内 test
  (:712,730,746,767)。
- `CommandToolOptions` literal — `registry.rs:317-327`・
  `route/resolve.rs:320-332`・`proc_command.rs:1564,1585`。
- `CommandPolicyView` literal — `route/supply.rs:41-47`・
  `route/resolve.rs:365-372`・`routing_policy.rs:2100-2106`。
- `ToolRuntimeOptions` literal — `invocation.rs:257-289`・
  `run/mod.rs:487-524`・`workflow_run.rs:493-518`・
  `proc_command.rs:206-227`・`proc_invocation.rs:106-131`・
  `proc_degrade.rs:47-…`・`proc_runtime.rs:88-…`・
  `characterization.rs:73-…`・`differential.rs:669-…`・
  `adapter.rs:314-…`・`cheaper-workflow/tests/process.rs:32-…`・
  `filesystem_contracts/artifacts.rs:69-…` — 全て `security: None`
  (または policy 由来値)を追加。
- `InvocationToolRuntimeOptions` literal — `run/mod.rs:335-354`・
  `workflow_run.rs:236-271`・`proc_invocation.rs:92-…,:436-…`・
  `cheaper-workflow/tests/process.rs:108-…`・
  `filesystem_contracts/artifacts.rs:102-…`。
- `RuntimePolicy` literal — `restrict.rs:113`・`capability.rs:86`・
  `dev/fixture.rs:759`・`run_dev_seam.rs:177,189`・
  `ollama_e2e.rs:219` — `security: SecurityPolicy::default()` を追加
  (`Default` derive 付き)。
- golden fixture — `policy/load.json`・`policy/schema.json` の
  normalized policy 出力に `"security":{"execDenyPatterns":[]}` key
  が出現する(期待値更新 + `security` を持つ case を追加)。
  `policy/merge.json` は raw map merge のため変更不要(optional で
  array replace の pin case を追加してよい)。

## Docs 更新契約

- `docs/guides/command.md` —
  - `:32` — 「native magic … `command` 専用の deny list が以下を
    既定で拒否する」の記述は #163 で mandatory deny・magic
    判定が除去済みのため stale。executable 解説(bare name/
    absolute path・ambient PATH 解決・canonical pin)は残し、
    deny 部分を `security.execDenyPatterns` の意味論・match 規約
    (basename 正規化・stem 完全一致・case-insensitive・OR・
    trusted layer 限定・merge = array wholesale replace)・
    「deterministic executable deny-list であり containment では
    ない」記述・**旧既定値**一覧(discovery が git から復元した
    verbatim regex)・migration seed 例(verbatim 1 要素版 + 分類別
    分解 yaml)へ書き換える。shell/shim 名が旧 deny 一覧に含まれ
    ない旨(A8 は別 predicate として除去済み)も migration 注記で
    明示する。
  - `:45` — invocation grant の deny 言及を opt-in 前提に
    restate(merged re-resolve で `InvalidRuntimePolicy`、
    `details.layer` は付かない accepted gap を含めて正直に)。
  - `:80` — 「deny list は containment 機構ではない」注意を
    opt-in profile の表現へ引き継ぐ。
- `docs/guides/configuration.md` — policy.yaml リファレンスの
  yaml block(:153-159 `route` 節の後)に `security` 節を追加し、
  `route.rewrite.approvalBypass` 小節(:186 以降)と同型の
  `### security.execDenyPatterns` 小節を新設: 既定 `[]`・
  `null`/未指定/`[]` 等価・trusted layer(builtin + user
  `policy.yaml`)のみ authoring 可・project/invocation からの
  宣言はそれぞれ `InvalidRuntimePolicy`/`InvalidRequest` +
  `details.layer="invocation"`・merge は配列の丸ごと置換・
  match 規約と適用点は command.md へ委譲。
- `crates/cheaper-cli/resources/policy.yaml` — `security:` 節を
  `route` の後に追加(`execDenyPatterns: []` + trusted layer 限定・
  opt-in・非 containment の comment)。knob 可視化は
  `route.approvalBypass: []` 先例 — **明示 key を置く**(
  spec の実装判断を本 design で確定)。

## Deferred Design Concerns

- **`process.tasks` executable への deny 対象拡大** — 別 trust
  boundary(project も authoring できる fixed-argv task)のため
  据え置き。将来の grant/task merge(issue 3/7)で exec grant
  モデルが統一された場合に再検討。
- **deny の CLI/env 供給経路** — `--grant` と同型の invocation
  供給は行わない(authoring は trusted 層のみ — spec Out of
  Scope)。要請が出た場合に再検討。
- **per-grant deny スコーピング** — `execDenyPatterns` は policy
  全体 posture であり grant 別の適用外指定は設けない(YAGNI)。
- **compiled deny の scope 間共有** — `ExecDenyList` を
  `RuntimePolicy`/invocation で cache 共有する最適化は現行
  cost(compile は一過性)に見合わない。

## Verification Notes

test 計画(spec Acceptance Criteria の各項目への対応):

- [ ] **schema accept/reject** — `security` が complete/partial で
  受理され `RuntimePolicy.security` へ normalize される・既定
  (未指定/`null`/`[]`/`{"execDenyPatterns":null}`)が空集合・
  project partial が `security` key を `InvalidRuntimePolicy` で
  拒否・`security` の malformed shape(非 object・非配列・非
  string item・未知 key・`security: null`)を schema が拒否
  (`schema.rs` 内 test — `route_section_schema_and_normalize`
  と同型)。
- [ ] **predicate 意味論** — `executable_stem` の正規化
  (`/`・`\`・`.exe`/`.com` strip・小文字化)、`ExecDenyList` の
  stem 完全一致(`sh` ≠ `bash`/`sshd`)、case-insensitive
  (`python.*` が `python3`/`PYTHON.EXE` に match・`Python`
  pattern が `python` に match)、OR・空集合 no-op、invalid
  regex の `InvalidRuntimePolicy`(`security.rs` 内 unit test)。
- [ ] **適用点 1** — `resolve_command_policy` の declared
  basename deny: `security.execDenyPatterns` + matching grant で
  `InvalidRuntimePolicy` + `details.grant`/`details.executable`
  (`command.rs`・`load.rs` 経由の両方)。
- [ ] **適用点 2/3** — tempdir 内の executable file(stem が
  pattern に match する名または test binary + matching
  pattern)で `pin_command_grant`/`check_command_execution` が
  `PolicyViolation` + `details.grant` を返す(mod.rs・
  `proc_command.rs`)。
- [ ] **既定 no-op** — `ExecDenyList::default()` で `python`/
  `ssh`/`tar` 等の grant が resolve・pin で受理される(既存
  `declared_executable_names_pass_policy_resolution` 系 test が
  signature 追随でそのまま担保)。
- [ ] **invocation 対象化** — user policy の deny + `--grant`
  由来 grant の match で `load_policy` が
  `InvalidRuntimePolicy`(layer attribution 無し — accepted gap
  の pin)、invocation layer の `security` key が
  `InvalidRequest` + `details.layer="invocation"`(既存 guard
  test の延長、`load.rs` tests)。
- [ ] **merge** — builtin `[a]` + user `[b]` で effective
  `[b]`(wholesale replace)の `load.rs` pin。
- [ ] **配管** — `create_tool_runtime`/`create_invocation_
  tool_runtime` が `security` raw JSON を受け deny を適用、
  parity check の `security` 不一致が
  `invalid_binding("security policy mismatch")`、scope 再注入が
  inner の security を運ぶ(`proc_invocation.rs`)、route
  viability の denied grant が `NoGrant`(`route/resolve.rs`
  unit)。
- [ ] **docs** — `command.md` の旧既定値一覧 + migration
  seed・`configuration.md` の `security` 節記述を doc review
  で確認。「deterministic executable deny-list」表現・非
  containment 記述の存在を確認。
- [ ] `mise run check` 全緑 + `node scripts/check-doc-ids.mjs` +
  `audit_docs` 0 error。

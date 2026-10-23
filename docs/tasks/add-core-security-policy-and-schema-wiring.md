---
id: "TASK-034KLYvyO70lji85Lv8jvN"
type: "task"
status: "done"
title: "cheaper-core security policy module と schema 配線・resolve signature"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
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
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  blocks:
    - "docs/tasks/plumb-exec-deny-through-tools.md"
    - "docs/tasks/document-exec-deny-patterns-profile.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  references:
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/discovery/opt-in-exec-deny-patterns.md"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
---

# cheaper-core security policy module と schema 配線・resolve signature

## Work

plan `docs/plans/opt-in-exec-deny-patterns.md`
（PLAN-034KLWpEpVSlofJ7Idynh5）の T1。cheaper-core の policy 層に
`security` section と compiled deny predicate を新設し、schema
配線・`RuntimePolicy.security`・`resolve_command_policy` の
signature 変更・適用点 1（declared basename deny）を実装する。
design（DESIGN-034KLwE8GiJQ2zvQY20ob8）の API 形状・schema 契約・
配線表・error 文言をそのまま実装する — 再解釈しない。

### 1. `policy/security.rs`（新設）

design の module contract 通りに集約する:

- `pub fn security_policy_properties() -> Value` — `security`
  section の JSON schema properties。design の verbatim json:

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

- `pub struct SecurityPolicy` — serde-native・`Debug, Clone,
  Default, PartialEq, Serialize`・`#[serde(rename =
  "execDenyPatterns")] pub exec_deny_patterns: Vec<String>`。
  compiled regex は載せない（`CommandGrant.argv` と同じ規約 —
  invocation parity check が declared-shape equality を見る）。
- `pub fn resolve_security_policy(input: Option<&Value>) ->
  Result<SecurityPolicy, CheaperError>` — `None`/`null`/非 object
  は `[]` 等価（`route` と同型の schema-gated 前提 — item は
  string のみ収集）。**compile は行わない**。
- `pub fn resolve_exec_deny_list(input: Option<&Value>) ->
  Result<ExecDenyList, CheaperError>` — resolve + compile の一回
  経路（deny 注入点が raw JSON から直接 compiled 形を得る
  helper）。
- `pub struct ExecDenyList { patterns: Vec<regex::Regex> }` —
  `Debug, Clone, Default`。
  - `pub fn compile(patterns: &[String]) -> Result<Self,
    CheaperError>` — 各 pattern を
    `RegexBuilder::new(&format!("^(?:{pattern})$"))
    .case_insensitive(true)` で**個別** compile（失敗 pattern を
    `details.pattern` に載せる attribution のため 1 本の合成
    regex にしない）。compile 不能は
    `security_invalid("invalid execDenyPatterns regex")` →
    `InvalidRuntimePolicy` + `details.pattern` + `.with_cause(
    regex_err)`。warn-and-skip は不採用（deny を silent に弱める
    — ADR 確定）。
  - `pub fn is_denied(&self, executable: &str) -> bool` —
    `executable_stem` で正規化した stem へ OR match。空集合は常に
    false。
  - `pub fn is_empty(&self) -> bool` — no-op 判定の単一手段
    （`exec_deny.is_empty()` が既定 no-op の同値条件）。
- `pub fn executable_stem(executable: &str) -> String` —
  `rsplit(['/', '\\']).next()` → `to_lowercase()` → `.exe`/`.com`
  suffix を 1 回 strip（旧 `is_denied_command_executable` の
  規約を引き継ぐ — declared spec と resolved canonical path に
  同一適用）。
- `fn security_invalid(reason: &str) -> CheaperError` —
  `command_invalid` と同型 — `"invalid security policy: {reason}"`
  + `InvalidRuntimePolicy`。

### 2. `policy/mod.rs`

module 一覧に `pub mod security;` を追加（alphabetic 位置 —
`schema` と `workflow` の間）。

### 3. `policy/schema.rs`（配線 6 site — design 表通り）

| site | lines（現行） | 変更 |
|------|------------|------|
| `policy_complete_schema()` | 287-312 | properties に `"security": object_schema(&[], security_policy_properties())` を追加。**required には加えない**（`route` 先例 — optional section、既存 complete policy との後方互換） |
| `policy_partial_schema()` | 316-333 | 同上（user 層の authoring 入口） |
| `policy_project_partial_schema()` | 340-355 | **追加しない** — `additionalProperties: false` が `security` key を含む project policy を `InvalidRuntimePolicy` で拒否（`command`/`route` と同型の section 丸ごと除外 fail-closed） |
| `RuntimePolicy` struct | 124-140 | `route` field（:133）の直後に `pub security: SecurityPolicy` を追加（trusted-layer 限定 section の隣接配置 — serialized 出力は `"route"` と `"tools"` の間に現れる） |
| `validate_policy_schema()` | 401-433 | schema 検査後の section resolve で、まず `let exec_deny = resolve_exec_deny_list(policy.get("security"))?;` を**無条件に**評価（`command` 欠落でも security-only doc の invalid regex を layer 単位で fail-closed）し、`if let Some(command)` ブロック（:429-431）で `resolve_command_policy(Some(command), &exec_deny)` を呼ぶ |
| `normalize_policy()` | 501-633 | `route` と同型の passthrough 解決 — `let security = resolve_security_policy(raw.get("security"))?;` + `let exec_deny = ExecDenyList::compile(&security.exec_deny_patterns)?;`（1 resolve・1 compile）で `command: resolve_command_policy(raw.get("command"), &exec_deny)?`、struct literal に `security` を畳み込む |

import は `use crate::policy::security::{...}` を現行の
`use` group 規約に従って追加。

### 4. `policy/command.rs` — signature + 適用点 1

- `resolve_command_policy`（:232）の signature を
  `(input: Option<&Value>, deny: &ExecDenyList) ->
  Result<ResolvedCommandPolicy, CheaperError>` へ変更 — raw
  `security` ではなく **compiled** `&ExecDenyList` を取る
  （caller が compile 境界を 1 回だけ持ち、options 経由の再
  resolve でも同じ deny 集合を共有する — design 確定形）。
- grant loop（:285-291）の `executable` 抽出直後（argv compile
  前 — 旧 `:377-381` と同じ相対位置）に適用点 1 を挿入:
  `exec_deny.is_denied(&executable)` →
  `command_invalid("executable is denied by
  security.execDenyPatterns")` + `InvalidRuntimePolicy`。
  `with_details` で `details.grant`（grant 名）・
  `details.executable`（declared spec 文字列）を載せる — pin
  段階の `details.grant` 規約と揃え、deny した spec を operator
  が特定可能にする。
- file 内 test 呼出し（:416,424,430,455,461,508 他 —
  compiler が全て捕捉）を `&ExecDenyList::default()` または
  compile 済み deny へ追随。

### 5. `policy/load.rs` — merged re-resolve（:208）

`resolve_command_policy(effective.get("command"))` を
`let exec_deny = resolve_exec_deny_list(effective.get("security"))?;
resolve_command_policy(effective.get("command"), &exec_deny)` へ
更新 — **merged effective（invocation grant 含む）への deny
評価点**として merge 直後の typed re-resolve で発火させる。
`resolve_security_policy`/`ExecDenyList`/`resolve_exec_deny_list`
の import を追加。

### 6. `policy/restrict.rs`(:113)・`policy/capability.rs`(:86)

`RuntimePolicy` literal に `security: SecurityPolicy::default()`
を追加（`Default` derive 付き）。`restrict_policy`（:100-103）は
`..policy.clone()` で `security` をそのまま運ぶ — code 変更
不要。`RESTRICT_*_FIELDS` に `security` は**加えない**（project
層は section 自体を書けないため到達不能）。

### 7. unit test（src 内 `#[cfg(test)]` — TSPEC の Rules に対応）

- `security.rs` — `executable_stem` 正規化（`/`・`\`・
  `.exe`/`.com` strip・小文字化）、stem 完全一致（`sh` ≠
  `bash`/`sshd`）、case-insensitive 双方向（`python.*` が
  `python3`/`PYTHON.EXE` に・`Python` が `python` に match）、
  OR・空集合 no-op（`is_empty`/`is_denied` false）、invalid
  regex の `InvalidRuntimePolicy` + `details.pattern`。
- `schema.rs` — `security` が complete/partial で受理され
  `RuntimePolicy.security` へ normalize・既定形（未指定・
  `{"execDenyPatterns":null}`・`{"execDenyPatterns":[]}`）が
  空集合・project partial が `security` key を
  `InvalidRuntimePolicy` で拒否・malformed shape（`security`
  値が非 object — `"security":null` を含む、
  `execDenyPatterns` が非配列・非 string item・空 string・
  129 個超・重複 item・`security` object の未知 key）を拒否
  （`route_section_schema_and_normalize` と同型）。section 値の
  `null` は `object_schema` が `"type":"object"`（nullable で
  はない）のため schema 段階で拒否される — resolve 側は
  `route` 先例通り `None`/`null`/非 object を `[]` 等価に扱う
  が、schema-gated pipeline では `security: null` は到達しない。
- `command.rs` — 適用点 1: matching deny で
  `InvalidRuntimePolicy` + `details.grant`/`details.executable`。
  既定 `&ExecDenyList::default()` で `python`/`ssh`/`tar` 受理
  （signature 追随で既存受理 pin が担保）。
- `load.rs` — merged re-resolve の declared deny（user deny +
  `--grant` matching grant → `InvalidRuntimePolicy`、
  `details.layer` **無し** — accepted gap pin）、invocation
  layer の `security` key → `InvalidRequest` +
  `details.layer="invocation"`（既存 guard test の延長）、
  builtin `[a]` + user `[b]` → effective `[b]`（wholesale
  replace pin）、invalid regex の load 時 fail-closed。

### 8. golden fixture（`crates/cheaper-core/tests/golden/fixtures/policy/`）

- `schema.json` — normalized policy 期待値へ
  `"security":{"execDenyPatterns":[]}` を追加し、`security` を
  持つ case（受理・project partial 拒否・invalid regex 拒否
  等）を追加。
- `load.json` — 同上（normalized 出力に key 出現 + deny 発火
  case を追加）。
- `restrict.json` — `policy_restrict.rs:9-12,34` が
  `serde_json::to_value(&p)` で全 `RuntimePolicy` を serialize
  するため 7 箇所（`route` key 所在行: 70,181,294,460,697,812,
  927）に `"security":{"execDenyPatterns":[]}` が出現 — 期待値
  更新。
- `merge.json` — raw map merge のため変更不要（optional で
  `execDenyPatterns` の array wholesale replace pin case を
  追加してよい）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/opt-in-exec-deny-patterns.md`
  （PLAN-034KLWpEpVSlofJ7Idynh5 — T1）
- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8 — module contract・schema
  契約・配線 6 site 表・signature 契約・適用点 1 error 契約・
  Churn 一覧が AUTHORITATIVE）
- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH — R1/R2/R4/R5/R8/R10/R11）
- `docs/test-specs/opt-in-exec-deny-patterns.md`
  （TSPEC-034KLXPTSxUL1oOQI06HM3 — schema accept/reject・
  predicate 意味論・適用点 1・既定 no-op・invocation 対象化・
  merge・serialized shape・全層 fail-closed Rules）

## Dependencies

- なし（起点 task）。design の行指定は現行コードと一致済み
  （branch `feature/issue-164-exec-deny-patterns` で実測）。
- 既知の transient: 本 task で `resolve_command_policy` の
  signature を変えるため、cheaper-tools（`registry.rs:1115`・
  `invocation.rs:234,398`）と cheaper-cli（`route/resolve.rs:385`・
  `routing_policy.rs:2068`）が T2/T3 の追随まで compile 不可。
  本 task の検証は `cargo test -p cheaper-core` に scope する
  （cheaper-core は単独で compile・test 可能）。

## Affected Files

- `crates/cheaper-core/src/policy/security.rs`（add）
- `crates/cheaper-core/src/policy/mod.rs`（modify）
- `crates/cheaper-core/src/policy/schema.rs`（modify）
- `crates/cheaper-core/src/policy/command.rs`（modify）
- `crates/cheaper-core/src/policy/load.rs`（modify）
- `crates/cheaper-core/src/policy/restrict.rs`（modify）
- `crates/cheaper-core/src/policy/capability.rs`（modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/schema.json`
  （modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/load.json`
  （modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/restrict.json`
  （modify）

## Done When

- [x] `policy/security.rs` が design の module contract 通りの
      pub API（`security_policy_properties`・`SecurityPolicy`・
      `resolve_security_policy`・`resolve_exec_deny_list`・
      `ExecDenyList`・`executable_stem`）を持ち、`mod.rs` が
      `pub mod security;` を宣言する
- [x] `policy_complete_schema`/`policy_partial_schema` が
      `security` を optional property として受理し（required
      非含有）、project partial が `security` key を拒否する
- [x] `RuntimePolicy.security` が `route` 直後の field として
      normalize され、serialized 出力に
      `"security":{"execDenyPatterns":[]}` が現れる
- [x] `resolve_command_policy` が `deny: &ExecDenyList` を取り、
      grant loop で適用点 1 が `InvalidRuntimePolicy` +
      `details.grant`/`details.executable` を返す
- [x] `validate_policy_schema` が `resolve_exec_deny_list` を
      無条件評価し（`command` 欠落の security-only doc でも
      invalid regex を拒否）、`normalize_policy`・`load.rs:208`
      が compiled deny で `resolve_command_policy` を呼ぶ
- [x] `restrict.rs`・`capability.rs` の `RuntimePolicy` literal
      が `security: SecurityPolicy::default()` を持つ
- [x] schema/security/command/load の unit test が TSPEC の
      Rules に対応して存在し、golden fixture 3 件が更新され
      `cargo test -p cheaper-core` が pass

## Out of Scope

- `crates/cheaper-tools` 側の signature 追随・options 配管・
  test churn（T2）
- `crates/cheaper-cli`/`cheaper-runtime`/`cheaper-workflow`/
  `cheaper-rig-adapter` 側の call site・literal churn（T3）
- docs（`command.md`・`configuration.md`・`resources/policy.yaml`）
  （T4 — T1 完了後に並列着手可）
- `process.tasks` executable への deny・`RESTRICT_*_FIELDS` への
  `security` 追加（到達不能）・merge 意味論の変更・`security`
  節への他 key
- compiled `ExecDenyList` の scope 間 cache 共有（YAGNI）

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` exits 0
      （unit test + golden target 全 pass — single-crate では
      dev-seam feature 適用外のため workspace feature 指定なし）
- [x] `mise exec -- cargo clippy -p cheaper-core --all-targets`
      exits 0（dead import・dead code なし）
- [x] `grep -n "security" crates/cheaper-core/src/policy/mod.rs`
      が `pub mod security;` を返す

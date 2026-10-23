---
id: "PLAN-034KLWpEpVSlofJ7Idynh5"
type: "plan"
status: "completed"
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
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
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
  implements:
    - "docs/specs/opt-in-exec-deny-patterns.md"
  implemented-by:
    - "docs/tasks/add-core-security-policy-and-schema-wiring.md"
    - "docs/tasks/plumb-exec-deny-through-tools.md"
    - "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
    - "docs/tasks/document-exec-deny-patterns-profile.md"
    - "docs/tasks/verify-exec-deny-patterns.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/discovery/opt-in-exec-deny-patterns.md"
  derived-by:
    - "docs/tasks/add-core-security-policy-and-schema-wiring.md"
    - "docs/tasks/plumb-exec-deny-through-tools.md"
    - "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
    - "docs/tasks/document-exec-deny-patterns-profile.md"
    - "docs/tasks/verify-exec-deny-patterns.md"
  verifies: []
  verified-by:
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/discovery/exec-contract-content-audit-removal.md"
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/registry.rs"
    - "crates/cheaper-tools/src/invocation.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# opt-in security.execDenyPatterns profile — A9 の降格実装 Implementation Plan

## Goal

DESIGN-034KLwE8GiJQ2zvQY20ob8 の実装契約に従い、policy schema に
`security` 節を新設し、operator 宣言の executable 名 regex list
`security.execDenyPatterns`（既定: 空）を deterministic executable
deny-list として実装する（ADR-034KLgkao8SgWWKIIsCqaB Option A —
top-level `security` 節・regex 配列・stem 完全一致 +
case-insensitive OR・load 時 fail-closed・3 適用点への signature
配管）。deny は `resolve_command_policy`（declared basename）・
`pin_command_grant`/`pin_command_grants`（resolved basename）・
`verify_pinned_executable`（spawn 前再検証）の既存 3 適用点で、
layer 由来情報を持たない resolved `command.grants` map 全体に効く
— invocation 合成 grant（`--grant`）を exempt しない（spec R7）。
既定（空集合）では `is_denied` が常に false であり、#163 後の
既定挙動は一切変わらない。

## Scope

単一の implementation stream。「`security` section + deny predicate
+ 3 適用点 + options 配管」という 1 つの behavioral 変更に、signature
churn（options/literal/test/golden fixture）が横断的に付随する構成
であり、API 形状・適用点・error 契約・churn 一覧は design が
file:line 精度で固定済み — 分割するのは commit の論理境界
（crate 層 + docs + 検証）のみで、plan は 1 つに収める。

| In | Out |
|---|---|
| 新 module `crates/cheaper-core/src/policy/security.rs` — `security_policy_properties()`・`SecurityPolicy`・`resolve_security_policy`・`resolve_exec_deny_list`・`ExecDenyList`・`executable_stem` の集約 | `process.tasks` executable への deny 適用（別 trust boundary — project も authoring できる fixed-argv task） |
| schema 配線 — complete/partial schema の optional property・project partial からの section 除外・`validate_policy_schema`/`normalize_policy`/`load_policy` の section resolve・`RuntimePolicy.security` field | deny の CLI flag・env var 注入、glob 等の非 regex 記法、merge 意味論の変更（配列 wholesale replace 維持） |
| deny 判定の 3 適用点注入と `RuntimePolicy.security` → options（`ToolRuntimeOptions`/`InvocationToolRuntimeOptions`/`ToolCtx`/`CommandToolOptions`/`CommandPolicyView`/`RouteSupply`/`InvocationInner`）→ 各 signature の配管 | `security` 節への他 key の追加（本 issue は `execDenyPatterns` のみ） |
| error 分類（resolve 段階 `InvalidRuntimePolicy` + `details.grant`/`details.executable`・pin/verify 段階 `PolicyViolation` + `grant_violation` 規約）と load 時 fail-closed（invalid regex 拒否） | 文書全体の正直性 sweep（issue #169 の scope）・viability/reporting UX の変更（`Viability::NoGrant` へ畳まれる現行分類のまま） |
| test・golden fixture・docs（`command.md`・`configuration.md`・builtin `resources/policy.yaml`）の更新 | deny 集合の scope 間 compiled cache 共有（YAGNI — design Deferred） |

## Upstream Documents

- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH、approved — R1-R12・Acceptance Criteria）
- `docs/adr/opt-in-exec-deny-patterns.md`
  （ADR-034KLgkao8SgWWKIIsCqaB、accepted — Option A: top-level
  `security` 節・regex・stem 完全一致 + case-insensitive OR・
  fail-closed・signature 配管・layer-agnostic 適用・棄却 option の理由）
- `docs/discovery/opt-in-exec-deny-patterns.md`
  （DISC-Fad7xt2qdtfpcCHhjMhkVy、resolved — 3 適用点の実測
  signature・call site inventory・旧 builtin regex 復元の
  ground truth）

## Design Inputs

- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8、approved — AUTHORITATIVE な
  HOW: `security.rs` module API 形状・schema 契約（
  `security_policy_properties()` の verbatim json）・schema 配線
  6 site 表・signature 契約 5 行・options/struct 配管 7 行・
  call site 配線全一覧 14 row・適用点 error 契約 3 row・
  layer-agnostic 適用と attribution gap・merge 意味論・
  Churn 一覧・Docs 更新契約・Verification Notes）。
  本 plan・task は design の行指定を転記するだけで、API 形状・
  error 文言・適用位置を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-core/src/policy/security.rs` | **新設** — `security_policy_properties`・`SecurityPolicy`（serde-native・`PartialEq`/`Serialize`・`exec_deny_patterns: Vec<String>`）・`resolve_security_policy`・`resolve_exec_deny_list`・`ExecDenyList`（`compile`/`is_denied`/`is_empty`、per-pattern `^(?:pattern)$` + `case_insensitive` compile）・`executable_stem`・`security_invalid` | T1。宣言形と compiled 形の分離が中核境界 — `ExecDenyList` は serialize しない |
| `crates/cheaper-core/src/policy/mod.rs` | `pub mod security;` を module 一覧に追加（`route`/`schema` の alphabetic 位置） | T1 |
| `crates/cheaper-core/src/policy/schema.rs` | `policy_complete_schema()`(:287-312)・`policy_partial_schema()`(:316-333) に `"security": object_schema(&[], security_policy_properties())` 追加（required 非追加）、`policy_project_partial_schema()`(:340-355) は**追加しない**、`validate_policy_schema()`(:401-433) の無条件 `resolve_exec_deny_list` + `resolve_command_policy(.., &exec_deny)`、`normalize_policy()`(:501-633) の security passthrough + compile、`RuntimePolicy`(:124-140) に `pub security: SecurityPolicy`（`route` 直後） | T1。project partial は `additionalProperties: false` による section 丸ごと除外で fail-closed |
| `crates/cheaper-core/src/policy/command.rs` | `resolve_command_policy` signature へ `deny: &ExecDenyList` 追加（:232）、grant loop（:285-291）の `executable` 抽出直後に declared basename deny（適用点 1 — `command_invalid("executable is denied by security.execDenyPatterns")` + `details.grant`/`details.executable`）、test 呼出し churn（:416,424,430,455,461,508 他） | T1。compiled `&ExecDenyList` を取る — raw `security` は渡さない（compile 境界は caller が 1 回だけ持つ） |
| `crates/cheaper-core/src/policy/load.rs` | :208 の typed re-resolve を `resolve_exec_deny_list(effective.get("security"))?` → `resolve_command_policy(.., &exec_deny)` へ更新 — merged effective（invocation grant 含む）への deny 評価点 | T1。merged deny error は `details.layer` を持たない（accepted gap — load.rs:18-24 と同構造） |
| `crates/cheaper-core/src/policy/restrict.rs`(:113)・`capability.rs`(:86) | `RuntimePolicy` literal に `security: SecurityPolicy::default()` 追加 | T1。`restrict_policy` は `..policy.clone()` で `security` をそのまま運ぶ（code 変更不要 — literal のみ） |
| `crates/cheaper-core/tests/golden/fixtures/policy/{schema,load,restrict}.json` | normalized policy 出力の `"security":{"execDenyPatterns":[]}` key 出現に合わせ期待値更新（restrict.json は 7 箇所: `route` key 所在行 70,181,294,460,697,812,927）+ `security` を持つ case を追加 | T1。`merge.json` は raw map merge のため変更不要（optional で replace pin case 追加可） |
| `crates/cheaper-tools/src/tools/command/mod.rs` | `pin_command_grants`(:167-179)・`pin_command_grant`(:185-202、`_name`→`name`)・`verify_pinned_executable`(:210-224) に `exec_deny: &ExecDenyList` param、pin の resolved basename deny（適用点 2 — `resolve_process_executable` 戻りを `let resolved` で受け `exec_deny.is_denied(&resolved)` で `grant_violation(name, "resolved executable is denied by security.execDenyPatterns")`）、verify の pin-equality 直後に同一 deny（適用点 3）、`CommandToolOptions.exec_deny: ExecDenyList` field（:233-244）、`check_command_execution`(:391-394) が `options.exec_deny` を渡す、mod.rs 内 test churn（:583,:603,:712,:730,:746,:750,:767,:778,:781,:789,:794） | T2。`check_command_execution` signature は不変（options field 経由） |
| `crates/cheaper-tools/src/registry.rs` | `ToolRuntimeOptions.security: Option<Value>`（:90-133）、`create_tool_runtime` で `resolve_exec_deny_list(options.security.as_ref())` → `resolve_command_policy(.., &exec_deny)`（:1114-1115）+ `pin_command_grants(.., &exec_deny)`（:1208-1214）、`ToolCtx.exec_deny` field（:213-233）+ `command()` clone（:317-327）+ literal（:1298） | T2 |
| `crates/cheaper-tools/src/invocation.rs` | `InvocationToolRuntimeOptions.security: Option<Value>`（:61-78）、`InvocationInner.security: SecurityPolicy` + `inner.exec_deny: ExecDenyList`、`create_invocation_tool_runtime`（:390-398）の resolve + compile、parity check（:219-238）の `security` 比較（不一致は `invalid_binding("security policy mismatch")`）+ command 再 resolve は `&inner.exec_deny`、scope 再注入（:271-274）へ `security: Some(serde_json::to_value(&inner.security).expect(...))` | T2 |
| `crates/cheaper-tools/tests/{proc_command,proc_invocation,proc_degrade,proc_runtime,characterization,differential}.rs` | options literal へ `security: None`（または policy 由来値）追加 + 適用点 2/3・parity・再注入の新規 test | T2。`proc_command.rs:1585` の `..options` spread は `exec_deny` を自動継承（差分確認対象） |
| `crates/cheaper-cli/src/route/resolve.rs` | `CommandPolicyView.exec_deny: &'a ExecDenyList`（:69-76 — 全 field borrowed の struct 規約、`Copy` 維持）、viability の `pin_command_grant` 呼出し（:311-319）+ `CommandToolOptions` literal（:320-332）、test literal（:365-372）+ `resolve_command_policy` 呼出し（:385）、denied resolved grant → `Viability::NoGrant` pin の新規 test | T3。`RouteEvalInput.command_policy` は `CommandPolicyView` のため hook eval 経路は自動適用 |
| `crates/cheaper-cli/src/route/supply.rs` | `RouteSupply.exec_deny: ExecDenyList`（:28-35）、`load_with_builtin`（:104-123）で `ExecDenyList::compile(&policy.security.exec_deny_patterns)?` populate、`command_view()`（:40-47）が `exec_deny: &self.exec_deny` | T3。load 済み policy 由来のため compile 失敗は到達不能想定だが `?` で伝搬 |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | test literal（:2100-2106）の `CommandPolicyView`・`resolve_command_policy` 呼出し（:2068）の signature 追随のみ | T3。`:142` の `RouteEvalInput` 経路は signature 変更不要 |
| `crates/cheaper-runtime/src/run/mod.rs` | `InvocationToolRuntimeOptions`（:335-354）+ worker scope `ToolRuntimeOptions`（:487-524）へ `security: Some(serde_json::to_value(&policy.security).unwrap_or_default())` | T3。worker scope は parity check 入力（`command` と同型） |
| `crates/cheaper-cli/src/workflow_run.rs` | `ensure_invocation`（:236-271）へ `security: Some(serde_json::to_value(&shared.policy.security).map_err(InternalError)?)`、`kind: tool` worker scope（:491-518）へ `security: None`（`command: None` と同型 — parity skip、inner の値が再注入される） | T3 |
| `crates/cheaper-cli/src/policy.rs` | module doc（:8-9）・`write_validated` doc（:245-247）の検査内容列挙へ deny 照合を追記 | T3。code 変更不要 — `write_validated` は merged full object を `validate_policy_schema(Partial)` で検査するため add 時点で fail-closed 済み |
| `crates/cheaper-runtime/src/dev/fixture.rs`(:759)・`tests/run_dev_seam.rs`(:177,189)・`tests/ollama_e2e.rs`(:219) | `RuntimePolicy` literal に `security: SecurityPolicy::default()` 追加 | T3 |
| `crates/cheaper-rig-adapter/tests/adapter.rs`(:314-…)・`crates/cheaper-workflow/tests/process.rs`(:32-…,:108-…)・`crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs`(:69-…,:102-…) | `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal へ `security: None`（または policy 由来値）追加 | T3 |
| `docs/guides/command.md` | :32 の stale deny 記述を `security.execDenyPatterns` 意味論・match 規約・旧既定値一覧・migration seed へ書き換え、:45 の invocation grant deny 言及を opt-in 前提に restate、:80 の非 containment 注意を引き継ぐ、新規記述（`.*` deny-all・`\` 区切り正規化の false-positive・blast radius）を追加 | T4。T1 の semantics 確定後に着手可能 |
| `docs/guides/configuration.md` | policy.yaml リファレンス yaml block（:153-159 `route` 節の後）に `security` 節追加 + `### security.execDenyPatterns` 小節新設（`approvalBypass` 小節 :186 以降と同型） | T4 |
| `crates/cheaper-cli/resources/policy.yaml` | `route` の後に `security:` 節追加（`execDenyPatterns: []` + trusted layer 限定・opt-in・非 containment の comment — knob 可視化は `route.approvalBypass: []` 先例） | T4 |

## Implementation Sequence

- [x] TASK-034KLYvyO70lji85Lv8jvN (T1): cheaper-core の
      `policy/security.rs` 新設 + schema 配線（`mod.rs`・
      `schema.rs` 6 site・`command.rs` signature + 適用点 1・
      `load.rs:208` re-resolve・`RuntimePolicy.security`）+
      `restrict.rs`/`capability.rs` literal churn + golden
      fixture 3 件更新 + core unit test。`cargo test -p
      cheaper-core` で閉じる
- [x] TASK-034KLYAfuIu8es8ZgpuQRl (T2): cheaper-tools 配管 —
      `command/mod.rs` の 3 signature + 適用点 2/3 注入・
      `CommandToolOptions.exec_deny`、`registry.rs` の
      `ToolRuntimeOptions.security` + `ToolCtx.exec_deny`、
      `invocation.rs` の `InvocationToolRuntimeOptions.security` +
      `InvocationInner.{security,exec_deny}` + parity + 再注入、
      tools test churn + 適用点・parity・再注入の新規 test。
      `cargo test -p cheaper-tools` で閉じる
- [x] TASK-034KLYAKiwETFc4o0BnVI1 (T3): cheaper-cli/runtime 配管 —
      `CommandPolicyView.exec_deny`・`RouteSupply.exec_deny`・
      `resolve.rs` viability・`routing_policy.rs` literal・
      `run/mod.rs` 2 site・`workflow_run.rs` 2 site・
      `policy.rs` doc restate + runtime/workflow/rig-adapter の
      test literal churn。cli test target + workspace compile 回復で
      閉じる
- [x] TASK-034KLYEMV6mRczcWiAQpy1 (T4): docs —
      `docs/guides/command.md`・`docs/guides/configuration.md`・
      `resources/policy.yaml`。T1 後なら T2/T3 と並列実行可能
- [x] TASK-034KLYFkg2bLCiG6WDNNpx (T5): 最終検証 — targeted
      cargo tests・`node scripts/check-doc-ids.mjs`・
      `audit_docs`・`mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 core security + schema | — | T2, T4 |
| T2 tools 配管 | T1 | T3 |
| T3 cli/runtime 配管 | T2 | T5 |
| T4 docs | T1 | T5 |
| T5 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T3 → T5。T4 は T1 完了後に T2/T3 と
並列実行可能（docs は compile 依存を持たない）。

既知の transient compile 断:

- T1 境界 — `resolve_command_policy` の signature 変更で
  cheaper-tools（`registry.rs:1115`・`invocation.rs:234,398`）と
  cheaper-cli（`route/resolve.rs:385`・`routing_policy.rs:2068`）が
  compile 不可。T1 の検証は `cargo test -p cheaper-core` に scope。
- T2 境界 — `pin_command_grant(s)`・`verify_pinned_executable`・
  `CommandToolOptions` の signature/field 変更で `route/resolve.rs`
  （:311・:320-332）が compile 不可。T2 の検証は `cargo test -p
  cheaper-tools` に scope し、T3 が cli call site を追随させて
  workspace compile を回復する。

各 task は 1 commit で、file-local の unit test・doc comment は
同 commit で追随する。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1/T2 commit 境界で downstream crate が compile 不可（signature churn — 必発の transient） | high(必発) | low | 既知の transient — 各 task の検証を単一 crate test に scope し直列実行。T3 で workspace compile 回復後に cli test を実測 |
| 2 | deny の誤発動(false positive) — unanchored match で `sh` が `bash`/`sshd` に substring match し正当 grant を拒否 | med | high | design 確定の stem 完全一致(`^(?:pattern)$` wrap)+ `case_insensitive` compile を強制 — TSPEC の predicate 意味論 Rule が `sh` ≠ `bash`/`sshd`・`python.*` → `python3`/`PYTHON.EXE` を pin |
| 3 | merged deny の attribution gap — invocation grant への deny error が `details.layer` を持たず operator が deny 元を特定しにくい | high(必発・受理済み) | low | design accepted gap — `load.rs` test で `InvalidRuntimePolicy` + layer 無しを pin し、command.md の restate で正直に明記 |
| 4 | invalid policy による波及 — invalid regex・deny-matching grant が `load_policy` 全体を失敗させ、route arm の fail-open(`load_or_none`)で route arm 全体が不活性化 | med | med | 「invalid policy を silent に ignore しない」既存 convention と整合 — design 受理済み、route supply の `?` 伝搬を維持 |
| 5 | 適用点 2/3 の no-op 化 — `ExecDenyList` 注入漏れ・`security` option を passthrough する経路で空 Default が混入し deny が効かない | med | high | 全経路（scope 構築・route viability・parity check・再注入）を TSPEC の配管 Rule が網羅 — `proc_invocation` の parity mismatch・re-injection test が漏れを検出 |
| 6 | golden fixture 更新漏れ — `RuntimePolicy` serialize の `"security"` key で schema/load/restrict の expected が stale | high(必発) | low | churn 一覧に行番号済み — `cargo test -p cheaper-core` の golden target が即検出 |
| 7 | `\` 区切り正規化による unix 上の奇名 false-positive（`my\python.exe` が deny に match） | low | low | 旧 builtin 由来の構造制約として design で確定 — 方向は fail-closed（漏らさない）。command.md の新規記述で明記 |
| 8 | `.*` による全 grant load 失敗（schema 上許容の明示的 deny-all） | low | med | 意図利用のみ — command.md に「grants を持つ policy は全て load 失敗」と明記 |
| 9 | data-loss / breaking-change | — | — | opt-in の additive schema 変更（`security` は optional・required 非追加）のため既存 policy file・既定挙動への影響なし。`RuntimePolicy` serialized shape に `"security"` key が出るのは golden fixture のみが観測面 |
| 10 | external dependency | — | — | 新規 crate 依存ゼロ（`regex`・`serde_json` は workspace dep 済み — design 明記） |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step（migration・外部 format 固定）は無い — `security` 節は
optional property のため既存 policy file は revert 後も valid。

## Task Breakdown

- T1 `docs/tasks/add-core-security-policy-and-schema-wiring.md`
  （`TASK-034KLYvyO70lji85Lv8jvN`） — `policy/security.rs` 新設・
  `mod.rs`/`schema.rs`/`command.rs`/`load.rs` 配線・
  `restrict.rs`/`capability.rs` literal・golden fixture 3 件・
  core unit test。
  verified by `docs/test-specs/opt-in-exec-deny-patterns.md`
- T2 `docs/tasks/plumb-exec-deny-through-tools.md`
  （`TASK-034KLYAfuIu8es8ZgpuQRl`） — `command/mod.rs` signature +
  適用点 2/3・`registry.rs`・`invocation.rs` 配管・tools test churn
  + 新規 test。
  verified by `docs/test-specs/opt-in-exec-deny-patterns.md`
- T3 `docs/tasks/plumb-exec-deny-through-cli-and-runtime.md`
  （`TASK-034KLYAKiwETFc4o0BnVI1`） — `route/resolve.rs`・
  `route/supply.rs`・`routing_policy.rs`・`run/mod.rs`・
  `workflow_run.rs`・`policy.rs` + runtime/workflow/rig-adapter
  test churn。
  verified by `docs/test-specs/opt-in-exec-deny-patterns.md`
- T4 `docs/tasks/document-exec-deny-patterns-profile.md`
  （`TASK-034KLYEMV6mRczcWiAQpy1`） — `command.md`・
  `configuration.md`・`resources/policy.yaml`。
  verified by `docs/test-specs/opt-in-exec-deny-patterns.md`
- T5 `docs/tasks/verify-exec-deny-patterns.md`
  （`TASK-034KLYFkg2bLCiG6WDNNpx`） — targeted tests・doc-id・
  `audit_docs`・`mise run check`・status 更新。

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` で security.rs/
      schema.rs/command.rs/load.rs unit test + golden target が
      pass（T1 — single-crate では dev-seam feature 適用外のため
      flag 無し）
- [x] `mise exec -- cargo test -p cheaper-tools` で command/mod.rs
      unit・proc_* integration・characterization・differential が
      pass（T2 — 同上 flag 無し）
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`
      + `cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test contracts --test run_dev_seam`
      + `cargo test -p cheaper-workflow --test filesystem_contracts`
      + `cargo test -p cheaper-rig-adapter` が pass（T3）
- [x] spec Acceptance Criteria 全項目に対応する test が存在し pass
      （TSPEC の Rules — schema accept/reject・predicate 意味論・
      3 適用点・既定 no-op・invocation 対象化・merge・配管・
      viability NoGrant・serialized shape・docs）（T5）
- [x] `node scripts/check-doc-ids.mjs` が pass（T5）
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で新規 doc の error 0（T5）
- [x] `mise run check` が pass（T5）

## Open Questions / Gaps

- なし — module API・schema 契約・signature・options 配管・
  適用点 error 文言・Churn 一覧・docs 契約の全 site は design が
  file:line 精度で固定済み。実装者の判断余地は新規 test/case の
  命名と restate 後の docs 文言のみ。
- merged deny の `details.layer` 無しは design accepted gap —
  `load.rs:18-24` の既存 attribution gap と同構造として pin・
  docs 明記するが、layer provenance の後付けは行わない（ADR
  Option D 棄却）。

## Self-Review

- [x] Every spec requirement maps to a task.（R1/R2/R4/R5/R8/R10 →
      T1 schema+predicate+適用点 1、R3/R7/R11 → T1 load.rs + T2/T3
      配管、R6 → T1+T2+T3 の 3 適用点、R9 → T2 の scope 限定、
      R12 → T4 docs）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
      consistent（全 file:line は branch 現行 code で実測確認済み）。

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: T4（docs）は T1 完了後に T2/T3 と並列
  実行可能。T1→T2→T3→T5 は直列（compile 依存）。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: design（DESIGN-034KLwE8GiJQ2zvQY20ob8）は review-protocol
  で approved 済み — 本 plan は機械的分解であり再審査不要。
- reviewer の焦点: T1/T2 の transient compile 境界（task の
  verification scope が単一 crate test に限定されること）、
  attribution gap pin（`details.layer` 無しを test が明示的に
  固定すること）、`security` option を passthrough する全経路が
  compiled `ExecDenyList` で終端すること。

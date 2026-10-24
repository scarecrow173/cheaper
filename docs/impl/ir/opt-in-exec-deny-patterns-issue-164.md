---
id: "IMPL-034KNKasRWZvDa31FnQqiW"
type: "impl"
status: "completed"
title: "Opt-in exec deny patterns issue 164"
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
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/mod.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/characterization.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_runtime.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/configuration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-core-security-policy-and-schema-wiring.md"
    - "docs/tasks/plumb-exec-deny-through-tools.md"
    - "docs/tasks/plumb-exec-deny-through-cli-and-runtime.md"
    - "docs/tasks/document-exec-deny-patterns-profile.md"
    - "docs/tasks/verify-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/opt-in-exec-deny-patterns.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  references:
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Opt-in exec deny patterns issue 164

## Summary

issue #164(#151 実装分解・A9 降格 issue)— #163 で mandatory 層から
除去された executable deny を、operator opt-in profile
`security.execDenyPatterns` として再配置した(ADR-034KLgkao8SgWWKIIsCqaB
Option A — top-level `security` 節・regex 配列・stem 完全一致 +
case-insensitive OR・load 時 fail-closed・既存 3 適用点への
signature 配管・layer-agnostic 適用)。deny は trusted layer
(builtin + user-scope `policy.yaml`)のみが authoring でき、既定
`[]` は「何も拒否しない」— #163 後の既定挙動は一切変わらない。

1. `cheaper-core`(T1): `policy/security.rs` を新設 —
   `security_policy_properties()`・`SecurityPolicy`(serde-native
   宣言形・`exec_deny_patterns: Vec<String>`)・
   `resolve_security_policy`・`resolve_exec_deny_list`・
   `ExecDenyList`(compiled 形 — `compile`/`is_denied`/`is_empty`、
   per-pattern `^(?:pattern)$` wrap + `case_insensitive` 個別
   compile・失敗 pattern を `details.pattern` へ帰属)・
   `executable_stem`・`security_invalid`。schema 配線 —
   complete/partial schema へ optional `security` 追加・project
   partial は section 丸ごと除外(`additionalProperties: false` で
   fail-closed)・`validate_policy_schema`/`normalize_policy`/
   `load_policy` の section resolve・`RuntimePolicy.security`。
   `resolve_command_policy` へ `deny: &ExecDenyList` + 適用点 1
   (declared basename → `InvalidRuntimePolicy` +
   `details.grant`/`details.executable`)、`load.rs` merged
   re-resolve、`restrict.rs`/`capability.rs` literal、golden
   fixture 3 件更新。
2. `cheaper-tools`(T2): `command/mod.rs` の `pin_command_grants`・
   `pin_command_grant`・`verify_pinned_executable` へ
   `exec_deny: &ExecDenyList` — 適用点 2(resolved basename →
   `PolicyViolation` + `details.grant`)と適用点 3(spawn 前再検証、
   pin-equality 直後に同一 deny)。`CommandToolOptions.exec_deny`、
   `registry.rs` `ToolRuntimeOptions.security` + `ToolCtx.exec_deny`、
   `invocation.rs` `InvocationToolRuntimeOptions.security` +
   `InvocationInner.{security,exec_deny}` + parity check
   (`invalid_binding("security policy mismatch")`) + scope 再注入。
3. `cheaper-cli`/`cheaper-runtime`(T3): `CommandPolicyView.exec_deny`
   (全 borrowed struct 規約・`Copy` 維持)・`RouteSupply.exec_deny` +
   `load_with_builtin` で policy 由来 compile・viability の
   `pin_command_grant`/`check_command_execution` 呼出し(denied →
   `Viability::NoGrant`)・`run/mod.rs` の invocation/worker scope
   2 site・`workflow_run.rs` 2 site・`policy.rs` doc restate・
   runtime/workflow/rig-adapter の test literal churn。
4. docs(T4): `command.md` の stale な mandatory deny 記述を opt-in
   意味論・match 規約・3 適用点・非 containment 注意・旧既定値
   verbatim + 分解 seed + migration 注記へ書き換え、
   `configuration.md` へ `security.execDenyPatterns` 小節新設、
   `resources/policy.yaml` へ `security:` 節(雛形 + 境界 comment)
   追加。
5. 検証(T5): targeted test 全 pass・`check-doc-ids` 1132 files
   OK・`audit_docs` error 0・`mise run check` exit 0・plan/task
   status 更新。

## Context

TASK-034KLYvyO70lji85Lv8jvN(T1)・TASK-034KLYAfuIu8es8ZgpuQRl(T2)・
TASK-034KLYAKiwETFc4o0BnVI1(T3)・TASK-034KLYEMV6mRczcWiAQpy1(T4)・
TASK-034KLYFkg2bLCiG6WDNNpx(T5)。PLAN-034KLWpEpVSlofJ7Idynh5 /
DESIGN-034KLwE8GiJQ2zvQY20ob8 / SPEC-034KLgfJ0Gka7fkRKHpfSH /
ADR-034KLgkao8SgWWKIIsCqaB / TSPEC-034KLXPTSxUL1oOQI06HM3 に従う。
3 適用点の実測 signature・call site inventory・旧 builtin regex
復元の ground truth は DISC-Fad7xt2qdtfpcCHhjMhkVy で確定済み —
#163 が「cheaper の責務は宣言された実行契約の強制」と確定し
mandatory 層から除去した際、if-block 丸ごと除去で保全した
再挿入位置(resolve・pin・spawn 前再検証の 3 適用点)をそのまま
再利用する降格実装。本 IR は `derives-from`/`implements`/
`verified-by`/`depends-on` を chain 内 doc 集合で閉じ、
cross-chain doc(#163 IR・#151 design/ADR・#163 design)は
`related`/`references` でのみ参照する — lineage edge は持たない。

## Implementation

### Changes

- `crates/cheaper-core/src/policy/security.rs`: **新設**。宣言形
  (`SecurityPolicy` — serde-native・pattern の source string を
  保持し compiled regex は載せない — invocation parity check が
  declared-shape equality を見るため `CommandGrant.argv` と同規約)
  と compiled 形(`ExecDenyList`)を分離し、compile 境界は
  `ExecDenyList` のみが持つ(`filesystem.denyPaths` → `PathDenylist`
  先例と同型)。match 規約: stem 完全一致(`^(?:pattern)$` wrap)+
  `case_insensitive` の per-pattern 個別 compile・配列 OR 評価・
  `executable_stem` は `/`・`\` 区切りの最終要素 → 小文字化 →
  `.exe`/`.com` suffix を 1 回 strip。空集合(既定)は常に no-op、
  invalid regex は `InvalidRuntimePolicy` fail-closed
  (warn-and-skip 不採用)。
- `crates/cheaper-core/src/policy/mod.rs`: `pub mod security;` を
  alphabetic 位置へ追加。
- `crates/cheaper-core/src/policy/schema.rs`:
  `policy_complete_schema()`・`policy_partial_schema()` へ
  `"security": object_schema(&[], security_policy_properties())`
  (required 非追加)、`policy_project_partial_schema()` へは
  **追加しない**(`additionalProperties: false` により section
  丸ごと `InvalidRuntimePolicy` で fail-closed)。
  `validate_policy_schema()` が無条件 `resolve_exec_deny_list` +
  `resolve_command_policy(.., &exec_deny)`、`normalize_policy()` が
  security passthrough + compile、`RuntimePolicy` へ
  `pub security: SecurityPolicy`(`route` 直後)。
- `crates/cheaper-core/src/policy/command.rs`:
  `resolve_command_policy` signature へ `deny: &ExecDenyList`
  (:234-237 — caller が 1 回 compile 済みを取り raw `security`
  は渡さない)。grant loop の `executable` 抽出直後に適用点 1
  (:300-312)— `command_invalid("executable is denied by
  security.execDenyPatterns")` + `details.grant`/
  `details.executable`。
- `crates/cheaper-core/src/policy/load.rs`: :206-216 の typed
  re-resolve を `resolve_exec_deny_list(effective.get("security"))?`
  → `resolve_command_policy(.., &exec_deny)` へ更新 — merged
  effective(invocation 合成 grant 含む)への deny 評価点。merged
  deny error は `details.layer` を持たない(accepted gap — 従来の
  re-resolve error と同構造)。
- `crates/cheaper-core/src/policy/restrict.rs`・`capability.rs`:
  `RuntimePolicy` literal に `security: SecurityPolicy::default()`
  (`restrict_policy` は `..policy.clone()` で `security` をそのまま
  運ぶため code 変更不要)。
- `crates/cheaper-core/tests/golden/fixtures/policy/`:
  `schema.json`(`security` 節の accept/reject case + normalized
  出力の `"security":{"execDenyPatterns":[]}` key)、`load.json`
  (`security` merge/replace・invocation 拒否 case)、
  `restrict.json`(normalized 出力 7 箇所)を更新。
- `crates/cheaper-tools/src/tools/command/mod.rs`:
  `pin_command_grants`(:173)・`pin_command_grant`(:196、`_name`→
  `name`)・`verify_pinned_executable`(:231)へ `exec_deny:
  &ExecDenyList`。適用点 2(:210-213 — `resolve_process_executable`
  戻りを `let resolved` で受け `exec_deny.is_denied(&resolved)` で
  `grant_violation(name, "resolved executable is denied by
  security.execDenyPatterns")` → `PolicyViolation` +
  `details.grant`)、適用点 3(:246-249 — pin-equality 直後に同一
  deny)。`CommandToolOptions.exec_deny: ExecDenyList` field(:274)、
  `check_command_execution`(:427)が `options.exec_deny` を
  `verify_pinned_executable` へ渡す(signature 不変)。
- `crates/cheaper-tools/src/registry.rs`:
  `ToolRuntimeOptions.security: Option<Value>`(:109)、
  `create_tool_runtime` で `resolve_exec_deny_list(options.security.
  as_ref())` → `resolve_command_policy(.., &exec_deny)` +
  `pin_command_grants(.., &exec_deny)`(:1124-1131)、
  `ToolCtx.exec_deny` field + `command()` clone + literal。
- `crates/cheaper-tools/src/invocation.rs`:
  `InvocationToolRuntimeOptions.security: Option<Value>`(:73)、
  `InvocationInner.{security: SecurityPolicy, exec_deny:
  ExecDenyList}`(:107-110)の declared-shape + compiled 二重保持、
  `create_invocation_tool_runtime`(:425-435)で resolve + compile。
  parity check(:243-259)の `security` 比較 — 不一致は
  `invalid_binding("security policy mismatch")`、command 再
  resolve は `&inner.exec_deny`。scope 再注入(:299-303)へ
  `security: Some(serde_json::to_value(&inner.security))`。
- `crates/cheaper-cli/src/route/resolve.rs`:
  `CommandPolicyView.exec_deny: &'a ExecDenyList`(:81 — 全 field
  borrowed の struct 規約・`Copy` 維持)。viability の
  `pin_command_grant` 呼出し(:317-323)と `CommandToolOptions`
  literal(:334)へ配線 — pin 失敗(deny match 含む)は
  `Viability::NoGrant` に畳み込まれる現行分類を維持。
- `crates/cheaper-cli/src/route/supply.rs`:
  `RouteSupply.exec_deny: ExecDenyList`(:39)、`load_with_builtin`
  で `ExecDenyList::compile(&policy.security.exec_deny_patterns)?`
  (:126)、`command_view()` が `exec_deny: &self.exec_deny`。
- `crates/cheaper-cli/src/hook/routing_policy.rs`: test literal・
  `resolve_command_policy` 呼出しの signature 追随のみ
  (`RouteEvalInput.command_policy` は `CommandPolicyView` のため
  hook eval 経路は自動適用)。
- `crates/cheaper-runtime/src/run/mod.rs`:
  `InvocationToolRuntimeOptions`(:344)+ worker scope
  `ToolRuntimeOptions`(:500)へ
  `security: Some(serde_json::to_value(&policy.security)
  .unwrap_or_default())` — worker scope は `command` と同型の
  parity check 入力。
- `crates/cheaper-cli/src/workflow_run.rs`: `ensure_invocation`
  (:259)へ `security: Some(serde_json::to_value(&shared.policy.
  security).map_err(InternalError)?)`、`kind: tool` worker scope
  (:511)へ `security: None`(`command: None` と同型 — parity skip、
  inner の resolved 値が再注入される)。
- `crates/cheaper-cli/src/policy.rs`: module doc の検査内容列挙へ
  deny 照合を追記(`write_validated` は merged full object を
  `validate_policy_schema(Partial)` で検査するため add 時点で
  fail-closed 済み — code 変更不要)。
- `crates/cheaper-cli/resources/policy.yaml`: `route` の後に
  `security:` 節追加(`execDenyPatterns: []` + trusted layer 限定・
  opt-in・非 containment comment — knob 可視化は
  `route.rewrite.approvalBypass: []` 先例)。
- `docs/guides/command.md`: stale な mandatory deny 記述を
  `### security.execDenyPatterns` 小節へ書き換え — opt-in 意味論・
  match 規約(basename stem・anchored case-insensitive・OR・
  merge replace)・3 適用点の error 契約・非 containment 注意・
  `.*` deny-all・`\` 区切り正規化の unix false-positive・
  `process.tasks` 非対象・旧 mandatory 一覧 verbatim + 分類別
  分解 seed + migration 注記(shell 拒否は別 predicate だった
  旨の正直な記述)。
- `docs/guides/configuration.md`: policy.yaml reference block へ
  `security` 節追加 + `### security.execDenyPatterns` 小節新設
  (`approvalBypass` 小節と同型)。
- test churn + 新規 pin: core unit(security.rs 6・schema.rs 3・
  command.rs 1・load.rs 4 — merge replace・invocation 拒否・
  attribution gap・fail-closed)、tools(`proc_command` 適用点 2/3・
  `proc_invocation` parity/再注入/`details.layer` 無し pin・他
  literal `security: None`/`exec_deny` default 追随)、cli
  (`routing_policy.rs` literal・`resolve.rs` NoGrant pin — unix
  gate)、runtime/workflow/rig-adapter options literal。

### Decisions During Implementation

- **宣言形と compiled 形の分離** — `SecurityPolicy`(source string
  保持・parity check の declared-shape equality 対象・
  `PartialEq`/`Serialize`)と `ExecDenyList`(compiled・serialize
  しない)を別型にし、compile 境界は `ExecDenyList` のみが持つ。
  `resolve_command_policy`/`pin_command_grant`/
  `verify_pinned_executable` は compiled `&ExecDenyList` を取り、
  raw `security` JSON を適用点へ渡さない — compile は各注入点で
  1 回だけ行う(load/scope 構築/supply 構築/invocation 構築)。
- **merged re-resolve で invocation 合成 grant を対象化** —
  `load.rs` の typed re-resolve は effective(merged)
  `command.grants` 全体を deny 評価するため、`--grant` 合成
  grant も exempt しない(spec R7 — deny は layer 由来情報を
  持たない resolved map に効く)。trade-off として merged deny
  error は `details.layer` を持たず deny 元層の帰属が取れない
  accepted gap(design 受理済み)を
  `invocation_grant_hits_user_exec_deny_without_layer_attribution`
  で pin し、誤って「修正」されないよう固定。
- **per-pattern 個別 compile** — 失敗した source string を error
  `details.pattern` へ帰属させるため 1 本の合成 regex にしない。
  invalid regex は compile 境界で `InvalidRuntimePolicy`
  fail-closed — warn-and-skip は deny を silent に弱めるため
  不採用(ADR 確定)。
- **parity check は declared-shape equality** — `security` を
  `command`/`process` と同型に `resolve_security_policy` した
  declared 形で比較し、不一致は `invalid_binding("security
  policy mismatch")`。worker scope の `security` 供給は
  invocation-resolved の再注入(`workflow_run.rs` の
  `kind: tool` scope は `security: None` で parity skip —
  `command: None` と同型)。
- **viability は `Viability::NoGrant` に畳み込む** — route
  viability で denied resolved grant を error 化せず pin 失敗と
  同じ `NoGrant` 分類へ畳み込む現行 viability/reporting UX を
  維持(spec 明示の scope 外)。
- **T1→T2→T3 の compile transient は scoped verification** —
  T1 が `resolve_command_policy` signature を、T2 が
  `pin_command_grant(s)`/`verify_pinned_executable`/
  `CommandToolOptions` を変えるため、各 task を単一 crate test
  で閉じ T3 で workspace compile を回復(#163 と同じ既知の直列
  依存処理 — design で予告済み)。

## Related Experiments

### Adopted

なし(known-solution task。design が module API 形状・schema 契約・
signature 契約・配管一覧・適用点 error 契約を file:line 精度で
固定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

targeted cargo test(2026-10-23 実測、全 pass / 0 failed):

- `mise exec -- cargo test -p cheaper-core`: exit 0 — 253 pass
  (unit 231 + golden/他 target 22。security.rs 6・schema.rs 3・
  command.rs 1・load.rs 4 の新規 deny test を含む)
- `mise exec -- cargo test -p cheaper-tools`: exit 0 — 357 pass
  (lib 259・characterization 10・differential 11・proc_command
  35・proc_degrade 9・proc_invocation 17・proc_process 9・
  proc_runtime 7)
- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`:
  exit 0 — 350 pass(integration 288・regression 1・run_dev_seam
  38・workflow 18・workflow_dev_seam 5)
- `cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam --test contracts --test run_dev_seam`:
  exit 0 — 99 pass(contracts 71・run_dev_seam 28)
- `cargo test -p cheaper-workflow --test compile_contracts --test filesystem_contracts`:
  exit 0 — 89 pass(46 + 43)
- `cargo test -p cheaper-rig-adapter`: exit 0 — 21 pass
- 行動 pin(全 pass — spec Acceptance Criteria ↔ TSPEC Rule
  対応確認済み):
  - schema accept/reject — `security_section_schema_and_normalize`
    (complete/partial 受理・project partial 拒否・malformed 拒否)
  - predicate 意味論 — `executable_stem_normalizes_basename`・
    `deny_list_matches_stem_exactly_case_insensitive`(`sh` ≠
    `bash`/`sshd`/`git-shell`、`python.*` → `python3`/`PYTHON.EXE`)・
    `deny_list_pattern_case_is_insensitive_too`・
    `empty_deny_list_is_noop`・
    `invalid_regex_fails_closed_with_pattern_attribution`・
    `resolve_security_policy_defaults_to_empty`・
    `resolve_exec_deny_list_compiles_in_one_pass`
  - 適用点 1 — `security_deny_rejects_matching_declared_executable`
    (`details.grant`/`details.executable`)+
    `declared_executable_deny_is_opt_in_and_fail_closed` +
    `security_exec_deny_applies_at_scope_construction`
  - 適用点 2/3 — `security_exec_deny_pin_blast_radius` +
    `check_command_execution_applies_exec_deny_at_verify`
    (`PolicyViolation` + `details.grant`)
  - 既定 no-op(旧 builtin 名受理)—
    `declared_executable_names_pass_policy_resolution`
    (`python`/`ssh`/`tar`/`sh`/`bash`/`cmd.exe`/`powershell` 受理)+
    `empty_deny_list_is_noop`
  - invocation 対象化 + attribution gap —
    `invocation_grant_hits_user_exec_deny_without_layer_attribution`
    (`InvalidRuntimePolicy`・`details.layer` 無し pin)+
    `invocation_layer_cannot_author_security`
    (`InvalidRequest` + `details.layer="invocation"`、top-level
    `security` と `command.security` 双方)
  - merge replace — `security_exec_deny_patterns_merge_and_
    invocation_scope`(user `[b]` が builtin `[a]` を丸ごと置換、
    user 未指定時は builtin 継承)
  - 配管 — `invocation_resolves_and_compiles_security`・
    `worker_scope_rejects_security_policy_mismatch`・
    `worker_scope_receives_invocation_resolved_security`・
    `worker_scope_denied_declared_keeps_inner_error_code`
  - viability NoGrant — `denied_resolved_grant_is_no_grant`
    (unix gate)
  - 全層 fail-closed —
    `security_invalid_regex_fails_closed_at_all_layers` +
    `invalid_exec_deny_regex_fails_load_at_layer`
- grep sanity: `security`/`exec_deny`/`ExecDenyList` 注入点が
  `policy/security.rs`・`schema.rs`・`command.rs`・`load.rs`・
  `registry.rs`・`invocation.rs`・`command/mod.rs`・
  `route/resolve.rs`・`route/supply.rs`・`run/mod.rs`・
  `workflow_run.rs` に全て接続。`TODO`・`unimplemented!`・stub
  残存なし(T5 実測)。
- `node scripts/check-doc-ids.mjs`: exit 0(1132 markdown files
  scanned)
- `node .agents/skills/doc-status/scripts/audit_docs.js --json`:
  本 issue の新規 doc 7 件(plan・test-spec・task×5)で error 0
  — finding は warning(reciprocal-relation drift 等、
  `blocking: false`)のみ
- `mise run check`: exit 0(biome 33 files・`cargo fmt --check`・
  clippy `--all-targets` + dev-seam・version 1.0.0・doc-ids・
  `cargo test --workspace` 50 test target 全 ok / 0 failed・
  `node --test` scripts 43 + benchmarks 12・lockfile・apm
  payload — T5 実測)

証跡 commit: planning chain `fdd4d51`(discovery seed)→
`3ac3cdf`(briefing — discovery 拡充 + spec + ADR)→
`45a71c8`(design)→ `0b42a46`(design review 指摘反映・approved)→
`11aec77`(plan + test-spec + tasks)→ 実装 `3f885a5`(T1 core)→
`6255a41`(T2 tools)→ `950fc2b`(T3 cli/runtime)→ `7a5ed5b`(T4
docs)→ `4d80c5b`(T3/T4 完了記録 + `policy.rs` module doc 整理)→
`4e2480b`(T5 検証記録 + plan/task status 更新)。本 IR はその後に
追加する。

## Risks

- **deny の誤発動(false positive)** — anchored stem 完全一致 +
  case-insensitive で `sh` ≠ `bash`/`sshd` は保証されるが、unix
  上でも `\` を区切り・`.com` suffix strip を行うため
  `my\python.exe`・`foo.com` のような奇名が deny に match し得る
  (旧 builtin 由来の構造制約 — 方向は fail-closed「漏らさない」
  で意図的に保持。`command.md` に明記済み)。
- **`.*` deny-all** — schema 上許容される明示的 deny-all で、
  宣言すると grant を持つ policy は全て load 失敗になる
  (意図的利用のみ — `command.md` 明記)。
- **merged deny の attribution gap** — invocation grant への
  deny error は `details.layer` を持たず、operator が deny 元
  (builtin/user)を特定しにくい(design 受理済み gap — test pin
  で誤修正を防止)。
- **invalid policy の波及** — invalid regex・deny-matching
  grant が `load_policy` 全体を失敗させ、route arm の
  `load_or_none` fail-open で route arm 全体が不活性化し得る
  (「invalid policy を silent に ignore しない」既存 convention
  と整合・design 受理 — supply の `?` 伝搬を維持)。
- **非 containment** — 任意 argv を別 process 起動へ変換できる
  binary(plugin/extension 機構を持つ tool・将来の新 runtime)は
  列挙を免れ得る。`execDenyPatterns` に何を書いても `command`
  の executable への grant は shell 相当の信頼として扱う
  (`command.md` で明言)。
- **blast radius** — 1 grant の match で `pin_command_grants`
  全体が `Err` となり command-tool scope 構築が失敗する
  (unpinnable grant と同規約 — blast radius 自体は既存で、
  deny が operator-reachable になっただけ)。
- **`process.tasks` は非対象** — deny は `command.grants` の
  executable にのみ適用。project も authoring できる固定 argv
  task は別 trust boundary(spec 明示の scope 外)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #164)由来の未分類・未処理の follow-up はない。下記は
全て spec「Out of Scope」・design「Deferred Design Concerns」・
各 task「Out of Scope」で scope 外確定・記録済みの defer、または
本 issue 由来でない観測の記録であり、新規 chain は起こさない。

- **route viability 層の UnsafeCommand 分類整理** — `route/
  resolve.rs` の metachar argv unsafe 分類(`argv_shell_re`
  由来)は本 issue でも未変更。command tool 受理可能 vs route
  不発の diverge は #163 から意図的残置が続く既知事項であり、
  issue #166(#151 分解 4)が追跡する。
- **docs 全体の正直性 sweep + breaking-change migration
  guide** — issue #169。「shell-free」/「boundary/sandbox」
  表現の sweep は #169 の scope。旧 mandatory 一覧の verbatim +
  分解 seed + migration 注記は本 issue で `command.md` に記録
  済み(残る sweep・guide 化のみ #169 へ defer)。
- **`process.tasks` executable deny** — project も authoring
  できる固定 argv task は別 trust boundary(spec「Out of Scope」
  明示)。適用が必要になれば別 issue。
- **compiled deny の scope 間 cache 共有** — design Deferred
  (YAGNI)。現行は注入点ごとに `ExecDenyList::compile` する
  単純構造のまま。
- **triage note — `proc_process.rs::cleans_live_descendants_on_
  exit_and_failure_and_abort_and_timeout`** が 2026-10-23 の
  targeted test 実測で full-suite 負荷下 1 回 timeout flake
  (単独再実行 + `mise run check` 内の再実行では pass)。#164
  非対象 file・deny 経路非依存の既知 flake であり、本 issue
  由来の regression ではない — 別途 flake 調査 issue の候補
  (起票は所有者判断 — issue #180 として起票済み)。

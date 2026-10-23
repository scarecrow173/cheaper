---
id: "TSPEC-034KLXPTSxUL1oOQI06HM3"
type: "test-spec"
status: "draft"
title: "opt-in security.execDenyPatterns の検証契約 (issue #164)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
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
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/discovery/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies:
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
  verified-by: []
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

# opt-in security.execDenyPatterns の検証契約 (issue #164)

## Purpose

issue #164（#151 実装分解 2/7）で確定した「operator opt-in の
deterministic executable deny-list `security.execDenyPatterns`」を、
schema 受理・predicate 意味論・3 適用点・trust boundary・配管・
serialized shape の 6 面で固定する。SPEC-034KLgfJ0Gka7fkRKHpfSH
R1-R12・Acceptance Criteria と DESIGN-034KLwE8GiJQ2zvQY20ob8 の
Verification Notes が authoritative。

保つべき意図は 3 つ。(1) **opt-in のみの発動**: 既定
（`security` 未指定・`execDenyPatterns` が `null`/空配列）は
空集合 = 全適用点 no-op であり、#163 後の既定挙動を壊さない —
opt-in 宣言時のみ deny が効く。(2) **fail-closed の一貫性**: invalid regex は
`validate_policy_schema`（layer 単位）・`normalize_policy`・
`load.rs` re-resolve・options 経由の再 resolve の全境界で
`InvalidRuntimePolicy` を返し、deny を silent に弱めない。
(3) **layer-agnostic 適用の正確さ**: resolved `command.grants`
map 全体に deny が及び invocation 合成 grant を exempt せず、
merged deny error は `details.layer` を持たない（design accepted
gap）— pin で誤帰属を防ぐ。

退役条件: `security` 節の構造・trust boundary・3 適用点の配置が
変わるまで存続する。`process.tasks` への deny 拡大や per-grant
スコーピングが導入されても本 TSPEC の核心（execDenyPatterns が
grant executable の opt-in deny-list として効く）は変わらない。

## Feature

`security` 節（top-level、trusted 層のみ authoring 可能）の
`execDenyPatterns` regex 配列を `RuntimePolicy.security` へ
normalize し、compiled `ExecDenyList` を options/signature 配管で
3 適用点（declared basename resolve・resolved basename pin・
spawn 前再検証）へ注入する。deny は stem 完全一致・
case-insensitive・OR 評価し、resolve 段階は
`InvalidRuntimePolicy`、pin/verify 段階は `PolicyViolation` +
`grant_violation` 規約で返す。

## Rules

- Rule: **schema accept/reject**（spec R1/R2 — `schema.rs` 内
  test、`route_section_schema_and_normalize` と同型）
  - `policy_complete_schema`・`policy_partial_schema` が `security`
    property を optional で受理し（required 非含有）、
    `normalize_policy` が `RuntimePolicy.security` へ解決する。
  - 既定形（`security` 未指定・`{"execDenyPatterns":null}`・
    `{"execDenyPatterns":[]}`）が全て空集合に normalize され、
    resolve 層も `None`/`null`/非 object 入力を `[]` 等価に
    扱う（`route` と同型の schema-gated 前提）。
  - `policy_project_partial_schema` が `security` key を含む
    project policy を `InvalidRuntimePolicy` で拒否する
    （`additionalProperties: false` による section 丸ごと除外 —
    `command`/`route` と同型）。
  - malformed shape — `security` 値が非 object（`"security":
    null` を含む — section は `object_schema` のため nullable
    でない）、`execDenyPatterns` が非配列・非 string item・
    空 string item・129 個超・重複 item（`uniqueItems`）・
    未知 key を含む `security` object — を schema が拒否する。

- Rule: **predicate 意味論**（spec R4 — `security.rs` 内 unit
  test）
  - `executable_stem`: `/`・`\` 区切りの最終要素 → 小文字化 →
    `.exe`/`.com` suffix を **1 回** strip — 例
    `"/usr/bin/Python3.EXE"` → `"python3"`、`"dir\\tool.com"` →
    `"tool"`。declared spec と resolved canonical path に同一
    適用。
  - stem 完全一致: pattern `sh` は `bash`/`sshd` に match しない
    （`^(?:pattern)$` wrap による anchor — substring 意図は
    operator が `.*` で明示する）。
  - case-insensitive 双方向: pattern `python.*` が `python3`・
    `PYTHON.EXE` に match し、mixed-case pattern `Python` が
    `python` に match する。
  - OR 評価: 複数 pattern のいずれかが match すれば denied。
  - 空集合 no-op: `ExecDenyList::default()` で `is_empty()` が
    true・`is_denied` が常に false。
  - invalid regex: `ExecDenyList::compile` が
    `security_invalid("invalid execDenyPatterns regex")` →
    `InvalidRuntimePolicy` + message `"invalid security policy:
    invalid execDenyPatterns regex"` を返し、`details.pattern` に
    失敗した source string・cause に regex error を載せる
    （per-pattern compile — 合成 regex にしない）。

- Rule: **適用点 1 — declared basename**（spec R6/R10 —
  `command.rs`・`load.rs` 経由の両方）
  - `resolve_command_policy(input, &deny)` で declared
    executable の stem が deny に match する grant は
    `InvalidRuntimePolicy` + message `"invalid command policy:
    executable is denied by security.execDenyPatterns"` +
    `details.grant`（grant 名）+ `details.executable`（declared
    spec 文字列）。適用位置は grant loop の `executable` 抽出
    直後（argv compile 前）。
  - `load.rs:208` の merged re-resolve 経路でも同 error が発火
    する（declared deny は load 段階で policy 全体を失敗させる）。

- Rule: **適用点 2/3 — resolved basename + spawn 前再検証**
  （spec R6/R10 — `command/mod.rs` unit test +
  `proc_command.rs`）
  - tempdir 内に実在する executable file（stem が pattern に
    match する名、または test binary + matching pattern）で
    `pin_command_grant`/`pin_command_grants` が
    `PolicyViolation` + `details.grant` を返す — message
    `command grant "{name}": resolved executable is denied by
    security.execDenyPatterns`。
  - `check_command_execution` → `verify_pinned_executable` 経路が
    pin-equality（`resolved != pinned`）通過後に同規約
    `grant_violation` で拒否する。
  - denied resolved grant が 1 つでも `pin_command_grants` 全体が
    `Err` となり command-tool scope 構築が失敗する（unpinnable
    grant と同規約の blast radius）。

- Rule: **既定 no-op**（spec R8 — 既存 test が signature 追随で
  担保）
  - `ExecDenyList::default()` または `security` 未指定の
    options で、`python`/`ssh`/`tar` 等の旧 builtin deny 名を
    含む grant が `resolve_command_policy`・`pin_command_grant`・
    `check_command_execution` を受理される（#163 後の受理 pin
    test がそのまま維持）。
  - `policy grant add` も既定では deny-matching 名の spec を
    受理する。

- Rule: **invocation 対象化と trust boundary**（spec R3/R7 —
  `load.rs` tests）
  - user policy の `execDenyPatterns` + `--grant` 由来の matching
    executable で `load_policy` が `InvalidRuntimePolicy` を返す。
    この error は **`details.layer` を持たない**（deny 元
    builtin/user と grant 元 invocation が別 layer — merged-map
    評価の帰属不能は design accepted gap として pin）。
  - invocation layer の top-level `security` key は既存構造
    guard（keys ⊆ `{version, command}`）が `InvalidRequest` +
    `details.layer="invocation"` で拒否し、`command.security`
    は `command` keys ⊆ `{grants}` guard が同様に拒否する
    （本 issue で guard 自体の変更は無い — 既存 test の延長で
    pin）。
  - `policy grant add` の `write_validated` が merged full
    object を `validate_policy_schema(Partial)` で検査するため、
    user policy 自身の `execDenyPatterns` に match する
    executable spec は add 時点で `InvalidRuntimePolicy`
    （fail-closed — code 変更不要の経路 pin）。

- Rule: **merge = wholesale replace**（spec R11 — `load.rs`
  pin）
  - builtin `execDenyPatterns: [a]` + user layer `[b]` で
    effective が `[b]`（配列の丸ごと置換 — union ではない）。
    未指定の user layer は builtin をそのまま継承する。
    golden `policy/merge.json` は raw map merge のため変更
    不要（optional で replace pin case を追加してよい）。

- Rule: **options/scope 配管**（spec R6 — `proc_invocation.rs`
  + `proc_command.rs` + `registry.rs`/`invocation.rs` 経由）
  - `create_tool_runtime` が `ToolRuntimeOptions.security` の
    raw JSON を `resolve_exec_deny_list` で compiled 化し、
    `resolve_command_policy`（適用点 1）・`pin_command_grants`
    （適用点 2）へ同一 `ExecDenyList` を渡す — raw `security`
    JSON を含む options で runtime 構築すれば deny が発火する。
  - `create_invocation_tool_runtime` が
    `InvocationToolRuntimeOptions.security` を resolve + compile
    し `inner.{security, exec_deny}` に保持する。
  - parity check: `options.runtime.security` が `Some` なら
    `resolve_security_policy` で再 resolve して `inner.security`
    と `PartialEq` 比較 — 不一致は
    `invalid_binding("security policy mismatch")`
    （`InvalidRequest` + `details.reason`）。`options.runtime.
    command` の再 resolve には `&inner.exec_deny` を使い、
    caller 供給 command の declared executable が deny に match
    した場合の error は `invalid_binding` ではなく
    `InvalidRuntimePolicy`（`with_invocation_layer` が inner
    code を保持する規約 — `load.rs:92-98` 先例）。
  - scope 再注入: caller の `None`/任意 raw に関わらず scope は
    常に invocation-resolved の `inner.security` を
    `serde_json::to_value` で受け取る（`inner.command` と同型）。
    invocation に deny 済み policy を持つ場合、scope 構築で
    deny が再発火する。

- Rule: **route viability**（spec R6 — `route/resolve.rs` unit
  test + `routing_policy.rs` literal 追随）
  - `CommandPolicyView.exec_deny` が `RouteSupply.exec_deny`
    （`load_with_builtin` で compiled）から供給され、viability
    評価の `pin_command_grant` + `CommandToolOptions` が deny-aware
    になる。
  - resolved stem が deny に match する grant の viability は
    `Viability::NoGrant`（error ではなく route 不成立 —
    unpinnable と同じ分類）。declared stem が match する grant は
    `load_policy` 段階で全体失敗するため、viability まで到達
    する resolved-stem deny は declared ≠ resolved の
    rename/symlink 経路に限定される観測（test は tempdir 内
    symlink/別名 pin で構成）。
  - `RouteEvalInput.command_policy` は `CommandPolicyView` の
    ため hook eval の viability も同一 deny を見る（signature
    変更不要、test literal のみ追随）。

- Rule: **serialized shape と RuntimePolicy literal churn**
  （`RuntimePolicy.security` field 追加 — golden + literal）
  - `RuntimePolicy` serialize に `"security":{"execDenyPatterns":[]}`
    が出現する — golden `policy/schema.json`・`policy/load.json`
    の normalized 期待値と `policy/restrict.json` の 7 箇所
    （`route` key 所在行: 70,181,294,460,697,812,927）を更新し、
    `security` を持つ case を追加する。
  - `RuntimePolicy` literal 構築（`restrict.rs`・`capability.rs`・
    `dev/fixture.rs`・`run_dev_seam.rs`・`ollama_e2e.rs`）は
    `security: SecurityPolicy::default()` を追加 — `restrict_policy`
    は `..policy.clone()` で `security` をそのまま運ぶ（code
    変更不要の pin）。

- Rule: **全層 fail-closed**（spec R5 — `schema.rs`・
  `load.rs`・`proc_*`）
  - invalid regex を含む `security` 節が `validate_policy_schema`
    の layer 単位検査で `InvalidRuntimePolicy` を返す —
    `command` section 欠落の security-only doc でも発火する
    （`resolve_exec_deny_list` を**無条件に**評価する design
    契約）。
  - `normalize_policy`・`load.rs` merged re-resolve・
    `create_tool_runtime`/`create_invocation_tool_runtime` の
    options resolve でも同 error（各境界が independent に
    fail-closed）。

- Rule: **docs contract**（spec R12 — doc review で確認）
  - `docs/guides/command.md` が `security.execDenyPatterns` の
    意味論・match 規約（basename 正規化・stem 完全一致・
    case-insensitive・OR・trusted layer 限定・merge = array
    wholesale replace）・旧既定値 verbatim 一覧・migration
    seed 例（verbatim 1 要素版 + 分類別分解）・非 containment
    記述・`.*` deny-all 注意・`\` 区切り正規化の false-
    positive・blast radius の記述を含む。
  - `docs/guides/configuration.md` が `security` 節 yaml +
    `### security.execDenyPatterns` 小節（既定 `[]`・
    `null`/未指定/`[]` 等価・trusted layer 限定・project/
    invocation 拒否の error code・merge replace）を含む。
  - `crates/cheaper-cli/resources/policy.yaml` が `security:` 節
    （`execDenyPatterns: []` + comment）を持つ。
  - 表現は「deterministic executable deny-list」—「保護」
    「sandbox」「containment」とは呼ばない。

- Rule: **full pipeline**（`mise run check` 全緑 +
  `node scripts/check-doc-ids.mjs` + `audit_docs` 0 error —
  design Verification Notes 末尾）

## Examples

- Example: `{"security":{"execDenyPatterns":["python.*"]}}` +
  grant `{"executable":"python3"}` →
  `resolve_command_policy(.., &deny)` が `InvalidRuntimePolicy` +
  `details.grant`・`details.executable`。同 grant が
  `{"executable":"bash"}` なら受理（pattern `sh` は `bash` に
  match しない anchor pin）。
- Example: tempdir `bin/tool`（denied stem 名で作成）を declared
  `./bin/tool`・pattern `tool` で pin → `pin_command_grant` が
  `PolicyViolation` + `details.grant="tool"`。pinned 済み grant の
  `check_command_execution` でも同 error（適用点 3）。
- Example: builtin `["a"]` + user `["b"]` → effective
  `execDenyPatterns == ["b"]`（wholesale replace pin）。
- Example: `--grant {"executable":"python"}` + user deny
  `python.*` → `load_policy` が `InvalidRuntimePolicy`（
  `details.layer` 無し — accepted gap）。invocation layer の
  `{"security":{...}}` → `InvalidRequest` +
  `details.layer="invocation"`。
- Example: `create_invocation_tool_runtime` に `security:
  {"execDenyPatterns":["x"]}` を渡した後、scope 構築で
  `runtime.security` に `{"execDenyPatterns":["y"]}` を供給 →
  `invalid_binding("security policy mismatch")`（`InvalidRequest`
  + `details.reason`）。`runtime.command` の declared spec が
  inner deny に match → `InvalidRuntimePolicy`（`invalid_binding`
  ではない — inner code 保持）。
- Example: route viability — declared `./run` → resolved
  `/tmp/.../run`（stem `run` が deny match）で `Viability::
  NoGrant`（`Runnable`/`StdinUnsupported` ではない）。
- Example: `security` を含まない既存 complete/partial policy
  file が変更なく受理され、normalized 出力が
  `"security":{"execDenyPatterns":[]}` を含む（後方互換 +
  serialized shape pin）。
- Example: invalid regex `["("]` を含む `security` 節のみの
  partial doc（`command` 欠落）が `validate_policy_schema` で
  `InvalidRuntimePolicy` — unconditional resolve の pin。

## Guarantees

- 実装者が design を再解釈しなくても、schema accept/reject・
  predicate 意味論・3 適用点の error code + details・既定
  no-op・invocation 対象化・wholesale replace・配管 parity・
  viability NoGrant・serialized shape の全項目が unit +
  integration + golden + literal-churn test で機械的に検証できる。
- `ExecDenyList` の no-op 判定は `is_empty()` のみ — predicate
  注入の有無ではなく集合の空で表現する単一規約が pin される。
- merged deny の attribution gap（`details.layer` 無し）が
  accepted として明示 pin され、誤って「修正」されない。
- 全 fail-closed 境界（validate・normalize・load re-resolve・
  runtime/invocation options resolve）が invalid regex を
  `InvalidRuntimePolicy` で拒否し、warn-and-skip が存在しない。
- docs が「deterministic executable deny-list」表現と非
  containment 明記を持ち、旧既定値 + migration seed が記録
  される。

## Non-goals

- `process.tasks` executable への deny 適用・旧 F3 機械の復活
  （別 trust boundary — spec/design Out of Scope）。
- glob 等の非 regex 記法、deny の CLI flag・env var 注入
  （`--grant` と同型の invocation 供給はしない）。
- merge 意味論の変更（union/差分 merge）・`security` 節への
  他 key 追加・per-grant deny スコーピング（YAGNI）。
- `ExecDenyList` の `RuntimePolicy`/invocation 間 cache 共有
  （compile は一過性 cost — design Deferred）。
- 文書全体の正直性 sweep・「boundary/sandbox/protection」表現の
  全量訂正（issue #169 の scope）。
- deny による viability/reporting UX の変更（`Viability::
  NoGrant` 分類の見直し）。
- 検証コマンド・実行結果の記録 — task の `## Verification` と
  impl record の責務。

## Risk

- unanchored match を許すと `sh` が `bash`/`sshd` に誤 match し
  正当 grant を拒否する — stem 完全一致 + case-insensitive の
  predicate pin がないと fail-closed 方向の false positive を
  見落とす。
- `security` option を passthrough する経路（scope 構築・route
  viability・parity check・再注入）で compiled deny の注入漏れが
  あると、declared 側で受理された grant が pin/verify 段階で
  no-op 化する — 配管 Rule の経路網羅が唯一の検出手段。
- attribution gap を pin しないと、merged deny の
  `details.layer` 無しが regression として「修正」され
  layer provenance 後付け（ADR Option D 棄却方向）へ drift する。
- golden fixture（schema/load/restrict）の `"security"` key 更新
  を忘れると serialized shape の破壊が silent 化する — golden
  target が検出するが、期待値更新と `security` case 追加の両方を
  task に明記する必要がある。
- transient compile 境界（T1 後の tools/cli・T2 後の cli）で
  downstream test を実行すると signature 不一致で必敗する —
  各 task の verification を単一 crate に scope しないと誤検証
  する。

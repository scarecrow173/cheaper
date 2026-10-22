---
id: "TSPEC-034KsKB8SYklsDeiJQS9ar"
type: "test-spec"
status: "draft"
title: "route.rewrite.approvalBypass policy flag contract (issue #150)"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
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
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
    - "docs/specs/invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  derived-by: []
  verifies:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
  verified-by:
    - "docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
  references:
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
  defers: []
  deferred-by: []
---

# route.rewrite.approvalBypass policy flag contract (issue #150)

## Purpose

`permissionDecision:"allow"` 併記の hook rewrite emit(親 agent の
approval prompt を素通しする応答)を有効化する per-target opt-in flag
`route.rewrite.approvalBypass` が、SPEC-034KrX7RnDs0gLVcMPf7vy
R1-R5/R15 と ADR-034KrYGRHZuvOIx1d0S0nE D1/D6 の contract を
満たし続けることを pin する。

保つべき意図は 2 つ。(1) **fail-closed trust 境界**: approval を
素通しする emit の許可は trusted 層(builtin/user)のみが authoring
でき、untrusted checkout(project 層 `<workspace>/.cheaper/`)は
flag を「消す」ことしかできず「有効化」は構造的に不能であること
— `command` section 除外と同型の構造保証。(2) **fail-loud + safe
default**: 値 domain 外の flag は `InvalidRuntimePolicy` で表面化し
(silent ignore しない)、未指定・`null`・空配列は `[]` 等価 =
bypassing emit 無効(現行 fallback 挙動)であること。

flag の配置・粒度が再設計された場合(例: per-route 化、別 config
surface 化、値 domain の拡張)、あるいは approval bypass を必要とする
emit が全廃された場合にのみ退役する。

## Feature

`policy.yaml` の `route` section に追加される
`route.rewrite.approvalBypass`(bypassing emit を許可する hook target
名の string 配列)の schema 受理・3 層 load・normalized
`RuntimePolicy.route.rewrite.approval_bypass` への反映と、
`RewritePolicy::approval_bypass_enabled(target)` による render 層への
flag 判定供給。

## Rules

- Rule: `route` section は complete/partial schema で optional
  property として受理される(required には入らない — 未指定は
  default `[]`)。`approvalBypass` の schema type は
  `["array","null"]`(`execution.maxTurns` の nullable 先例
  schema.rs:196 と同型)で、明示 `null` ≡ 未指定 ≡ `[]` を schema
  層で保証する。
- Rule: 値 domain は bypassing emit を持ちうる hook target 名
  `codex`/`cursor`/`copilot` に限る — 未知名・重複・非配列・
  非文字列 item・非 object の `rewrite`/`route` は
  `InvalidRuntimePolicy` として fail-loud に表面化する(「invalid
  policy is never silently ignored」、`policy/load.rs:101` 規約)。
- Rule: project 層 policy(`<workspace>/.cheaper/policy.yaml`)から
  `route` section を authoring できない —
  `policy_project_partial_schema`(schema.rs:289)に `route`
  property を置かず、`additionalProperties: false` が `route` key
  を持つ project policy を load error で拒否する。field 個別列挙
  ではなく section 丸ごと除外(`command` と同型)で、将来 `route`
  配下に field が増えても自動的に project から閉じる。
- Rule: merge 規則は既存 layer merge(Replace)に従う — user 層で
  列挙された target のみ bypassing emit が有効になり、配列は
  wholesale replace(object deep-merge の一部)。
- Rule: normalized 表現は `RoutePolicy{rewrite:
  RewritePolicy{approval_bypass: Vec<String>}}` で yaml 表現と 1:1
  (serialize は `approvalBypass` camelCase)。
  `approval_bypass_enabled(&self, target: &str) -> bool` が flag
  判定の単一 owner — render 層は policy path を直接たどらない。
- Rule: route supply / policy の load 失敗は従来どおり fail-open —
  `load_or_none`(route/supply.rs:127)が `None` を返す場合は flag
  未供給 = bypass 無効(conservative default)で、route arm 自体も
  不活性のまま。flag 読み取り失敗のみを理由に新しい error 経路を
  増やさない。
- Rule: flag の意味は「当該 target へ affirmative permission
  decision を併記した rewrite emit の送信を user が許可した」—
  flag は emit 有効化ではなく許可であり、emit contract が未検証の
  target(cursor/copilot)は flag に含まれても emit されない。

## Examples

- user `policy.yaml` に `route: {rewrite: {approvalBypass: [codex]}}`
  を書く → load され、effective `RuntimePolicy.route.rewrite.
  approval_bypass == ["codex"]`、`approval_bypass_enabled("codex")
  == true`、`approval_bypass_enabled("cursor") == false`。
- project `.cheaper/policy.yaml` に `route` key を持つ → project
  partial schema 違反で `load_policy` 全体が `InvalidRuntimePolicy`
  で失敗 → `load_or_none` は `None` → route arm 不活性 + bypass
  無効(fail-closed)。
- `approvalBypass: [windsurf]`・`approvalBypass: "codex"`・
  `approvalBypass: [1]`・`approvalBypass: [codex, codex]`・
  `route: []` の各形 → `InvalidRuntimePolicy`。
- `route` 未指定・`route: {rewrite: {approvalBypass: null}}`・
  `route: {rewrite: {approvalBypass: []}}` → いずれも
  `approval_bypass == []`。
- routes load 失敗(policy 破損等)の firing → bypass 無効で
  route arm 不活性、hook は従来評価へ fallthrough。

## Guarantees

- untrusted project checkout はいかなる policy 記述でも approval
  bypass を有効化できない(構造保証 — untrusted は flag を消せるが
  付けられない)。
- domain 外の flag 値は load 時に必ず error として表面化し、
  silent に無効化・切り詰め・既定化されない。
- flag 未供給のあらゆる経路(未指定・null・load 失敗)で bypassing
  emit は物理的に発行されない。
- `RuntimePolicy.route` は serialize 対称性を持ち、golden
  `expected.policy` に `route` が現れる(merge/load 経路でも
  drift しない)。

## Non-goals

- flag が render 層へ届いた後の emit 挙動そのもの(codex emit
  shape・負 shape)は `codex-hook-rewrite-emit` test spec が担う。
- cursor `preToolUse` 登録・normalize は
  `cursor-pretooluse-registration-and-normalize` test spec が担う。
- `routes.yaml` 側の route 宣言 schema・per-route trust(route の
  宣言と policy の `route` section は別 surface)。
- env var による flag override(本 spec では対象外 — YAGNI)。
- invocation 層(`{"version","command.grants"}` 限定の合成 layer)の
  `route` 拒否 — 既存の grants-only key 検査が拒否する既存振る舞い
  であり新規保証ではない。

## Risk

- この保証を落とすと、untrusted checkout が含まれる workspace で
  approval prompt を素通しする emit が user の同意なしに発行され
  得る(supply-chain 経路: project policy.yaml に `route` key を
  忍ばせるだけで bypass が有効化される)。
- domain 検証を落とすと typo(`approvalBypass: [codx]`)が silent
  no-op になり、「有効化したつもりで fallback のまま」の診断不能な
  設定ミスを生む。
- default `[]` 等価の保証を落とすと、flag 非認知の既存 user の
  挙動が変わり得る(flag 未供給 = bypass 無効の保守的既定が崩れる)。

---
id: "TASK-034KsKLJ7FGv1ytMp8eLbx"
type: "task"
status: "todo"
title: "Add route.rewrite.approvalBypass policy schema and normalized flag"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  blocks:
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-rewrite-approval-bypass-policy-flag.md"
  references:
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
---

# Add route.rewrite.approvalBypass policy schema and normalized flag

## Work

plan `docs/plans/implement-per-target-hook-rewrite-enablement.md`
(PLAN-034KsKo6nYOBOpIpkNoviY)の T1。`policy.yaml` に `route`
section を新設し、`route.rewrite.approvalBypass`(bypassing emit を
許可する hook target 名の string 配列)を trusted 層のみ authoring
可能な schema + normalized 表現として追加する。**schema 変更・
`RuntimePolicy` field 追加・literal 構築 site 追随・golden fixture
再生成・unit test は全て同一 commit の原子単位**(どれか欠けると
compile か test が壊れる)。

### 1. schema — `route` section 追加(complete/partial のみ)

- `crates/cheaper-core/src/policy/schema.rs` に
  `route_properties()` を追加: `{"rewrite": object_schema(&[],
  {"approvalBypass": {"type": ["array","null"], "items": {"type":
  "string", "enum": ["codex","cursor","copilot"]}, "uniqueItems":
  true}})}`。nullable は `execution.maxTurns`
  (`{"type": ["integer","null"]}`、schema.rs:196)の先例に倣い、
  明示 `null` ≡ 未指定 ≡ `[]` を schema 層で保証する(R2)。
- `policy_complete_schema`(:238)・`policy_partial_schema`(:266)
  の properties に `"route": object_schema(&[], route_properties())`
  を追加 — required には入れない(未指定は default `[]`)。
- `policy_project_partial_schema`(:289)には `route` property を
  **置かない** — `additionalProperties: false` が `route` key を
  持つ project policy を `InvalidRuntimePolicy` で拒否する
  (`command` section 除外と同型の section 丸ごと fail-closed、R3)。
- 値 domain 外の fail-loud 化: 未知 target 名は enum で、非配列・
  非文字列 item・重複は type/items/uniqueItems で、非 object の
  `rewrite`/`route` は object_schema で、それぞれ
  `InvalidRuntimePolicy` として表面化する(R1 — schema 側で
  表現できる分は schema に寄せ、normalize 側で二重に解釈しない)。

### 2. normalized 型 — `RoutePolicy`/`RewritePolicy`

- `RuntimePolicy` に `pub route: RoutePolicy` field を追加。
  `RoutePolicy{rewrite: RewritePolicy{approval_bypass: Vec<String>}}`
  — yaml 表現と 1:1 の構造を保ち、serialize は
  `approvalBypass`(camelCase、`#[serde(rename)]`)。
- `RewritePolicy::approval_bypass_enabled(&self, target: &str) ->
  bool` を flag 判定の単一 owner として実装(render 層は policy
  path を直接たどらない)。
- `normalize_policy`(:450)に `route` の passthrough 解決を追加 —
  `null`/欠落 → default `[]`(`RoutePolicy` default は
  `approval_bypass: vec![]`)。

### 3. builtin 既定 + literal 構築 site 追随

- `crates/cheaper-cli/resources/policy.yaml` に
  `route: {rewrite: {approvalBypass: []}}` を追記(self-documenting
  既定値)。
- `RuntimePolicy{…}` literal 構築 site へ `route` field を追記
  (design manifest): `cheaper-runtime/src/dev/fixture.rs:226,758`、
  `cheaper-core/src/policy/restrict.rs:112`、
  `cheaper-core/src/policy/capability.rs:86`、
  `cheaper-runtime/tests/run_dev_seam.rs:189`、
  `cheaper-runtime/tests/ollama_e2e.rs:219`。compile error 強制で
  漏れを検出する(field 追加直後に `cargo check` で網羅確認)。

### 4. golden fixture 再生成

- `RuntimePolicy` は `Serialize` を持つため `route` が serialize
  出力へ載り、`expected.policy` を保持する golden fixture
  `crates/cheaper-core/tests/golden/fixtures/policy/{load,restrict,
  schema}.json`(harness `policy_{load,schema,restrict}.rs`)を
  再生成する。「無変更で pass」は実行時挙動に限る(R17)。

### 5. unit test(schema.rs / load.rs tests)

- schema test: `route` section が complete/partial で受理・project
  partial で `route` key 拒否。`approvalBypass` の未知 target 名・
  非配列・非文字列 item・重複・非 object `rewrite`/`route` が
  `InvalidRuntimePolicy`。`null`/未指定で default `[]`。
- load test(`policy/load.rs` tests): user policy の `route` merge
  が effective policy に反映(array は wholesale replace)、project
  policy の `route` key が load error(untrusted project が bypass
  を有効化できない構造保証)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-per-target-hook-rewrite-enablement.md` — 本
  task が implement する plan(File Map・Dependency Graph の T1)
- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1)— `route_properties()`・
  `RoutePolicy`/`RewritePolicy` 形・`approval_bypass_enabled` の
  AUTHORITATIVE 定義
- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy)— R1-R5・R15
- `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
  (ADR-034KrYGRHZuvOIx1d0S0nE)— Option A・D1・D6 の根拠

## Dependencies

- なし(後続 T2・T4 は本 task に依存 — `blocks` 参照)。

## Affected Files

- `crates/cheaper-core/src/policy/schema.rs`(modify — schema 3 mode・
  normalized 型・`normalize_policy`)
- `crates/cheaper-cli/resources/policy.yaml`(modify — builtin 既定)
- `crates/cheaper-core/src/policy/restrict.rs`・`capability.rs`
  (modify — literal 追記)
- `crates/cheaper-runtime/src/dev/fixture.rs`・
  `crates/cheaper-runtime/tests/run_dev_seam.rs`・
  `crates/cheaper-runtime/tests/ollama_e2e.rs`(modify — literal 追記)
- `crates/cheaper-core/tests/golden/fixtures/policy/{load,restrict,
  schema}.json`(regenerate)
- `crates/cheaper-core/tests/golden/policy_{load,schema,restrict}.rs`
  (regen 反映のみ)

## Done When

- [ ] `route.rewrite.approvalBypass` が complete/partial schema で
      受理され、`["array","null"]` + enum + uniqueItems で domain
      検証される
- [ ] project partial schema が `route` key を持つ policy を拒否する
- [ ] `RoutePolicy`/`RewritePolicy`/`approval_bypass_enabled` が存在し、
      `RuntimePolicy.route` が serialize される
- [ ] builtin `resources/policy.yaml` に `route: {rewrite:
      {approvalBypass: []}}` が追記される
- [ ] `RuntimePolicy{…}` literal 5 site + golden fixture 3 件が
      追随され、`cargo check`/`cargo test` が compile・pass する
- [ ] schema/load unit test が design §Verification Notes の列挙
      (受理/拒否/domain 外/default/merge)を網羅する

## Out of Scope

- `targets.rs` の flag 供給・codex render(T2)
- cursor hooks.json・`cursor_normalize`(T3)
- integration test(T4)
- `docs/guides/configuration.md`(T5)
- `route/supply.rs`・`hook/mod.rs`・`policy/load.rs` の merge 経路
  (変更不要 — `route` は既存 Replace merge に乗る)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0(schema・load・golden 全 pass)
- [ ] `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0(literal site の compile・test 追随)
- [ ] `mise exec -- cargo clippy -p cheaper-core -p cheaper-runtime --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

---
id: "TASK-034KVxDgWqxNof9qYv0dbf"
type: "task"
status: "todo"
title: "route grant-set projection の docs 更新"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/route-grant-set-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/route-grant-set-projection.md"
    - "docs/tasks/add-route-grants-declaration-and-grant-set.md"
  blocks:
    - "docs/tasks/verify-route-grant-set-projection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/route-grant-set-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-grant-set-projection.md"
  references:
    - "docs/designs/route-grant-set-projection.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/discovery/route-grant-set-projection-issue-166.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/guides/usage.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "crates/cheaper-cli/resources/routes.yaml"
  defers: []
  deferred-by: []
---

# route grant-set projection の docs 更新

## Work

plan `docs/plans/route-grant-set-projection.md`
(PLAN-034KVx51kVYoMwO8UmJ8VE)の T4。design
(DESIGN-034KUgF5QB3WuzFG6PmVvr)の Docs 更新契約に従い、
`grants` 宣言面・層制約・member job authoring 規約・record
`executed` とその平文 arg 露出を 4 guide + routes.yaml comment
+ pretooluse spec(content update のみ)へ反映する。
design の指定行と記述内容を転記するだけで、意味論を再解釈
しない。

### 1. `docs/guides/parent-agent-integration.md`

- routes.yaml 宣言例(:248-259 付近)と field 説明に `grants`
  を追加 — 意味: `{route.grant} ∪ grants` 有効 set・冪等
  dedupe・`trusted && !fixed`(`is_route_eligible`)
  eligibility・member argv は自身の grant pattern で bound・
  member job は宣言 lifecycle/cwd/env/`exitCodes`・route
  context(exec_cwd/passthrough extra_env/accept_any_exit_code/
  log_tee)非適用。
- **層制約の明記** — `grants` を宣言できるのは builtin/user
  層のみで、project routes.yaml に書くと route-load error
  (`InvalidRuntimePolicy`)になること。anchor `grant:` 単数は
  project でも宣言可である非対称を併記。
- **workflow authoring 規約** — member を呼ぶ `command` job は
  `job_id != "exec"` で宣言する(`job_id=="exec" &&
  tool=="command"` は expected_input byte-match gate が
  `route.grant` + 原 argv を要求し `exec-input-mismatch` で
  失敗する)。
- record 節(:349-357 付近)に `executed:{grant,argv}` を
  additive field として記述し、`executed.argv` が**平文の
  arg 値**を記録する露出を明記(usage.jsonl 初の cleartext
  arg — `commandHash` は一方向 hash・env-prefix 値は prefixed
  argv 拒否で漏洩しないが arg 値自体の secret は残る)。

### 2. `docs/guides/configuration.md`

- unified exec 参照節(:215-217)の「route の `grant:` 参照は
  trusted+`!fixed` のみ」を、`grants:` member も同 predicate
  (`is_route_eligible`)へ従う記述へ更新。`grants` 宣言自体は
  builtin/user 層限定で project routes.yaml は持てない旨も
  併記。

### 3. `docs/guides/command.md`

- route grant 参照記述(:38-42)を、`grants` member も同じ
  eligibility 規則(scope==trusted && !fixed)に従う旨へ更新。

### 4. `docs/guides/usage.md`

- additive-only 規約節(:296-303)に `kind:"route"` record の
  `executed` を additive field の例として追記し、
  `executed.argv` が平文 arg 値を保持する点を 1 文で明記
  (`route` record field の canonical 記述は
  parent-agent-integration 側 — DRY)。

### 5. `crates/cheaper-cli/resources/routes.yaml`

- `grant:` 直下に `# grants: [name, …]  # optional — route.grant
  以外の追加委譲 grant(builtin/user 層限定)` の shape comment
  を追加(宣言面の記述のみ — builtin route は member 非宣言)。

### 6. `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`

- refine 対象 spec として content update のみ: route field
  一覧(R1 — :215-229 付近)に `grants?`(trusted 層限定)・
  viability 記述(R12 — :299-306 付近)に member 検査
  (存在 + eligibility + pin → NoGrant fold)・record field
  (R34 — :447-457 付近)に `executed` を追記。relation
  (`refines` 等)は本 chain からは張らない — spec chain 側の
  管理に委ねる(design Docs 更新契約)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/route-grant-set-projection.md`
  (PLAN-034KVx51kVYoMwO8UmJ8VE — T4)
- `docs/designs/route-grant-set-projection.md`
  (DESIGN-034KUgF5QB3WuzFG6PmVvr — Docs 更新契約が
  AUTHORITATIVE: 各 file の対象行と必須記述)
- `docs/specs/route-grant-set-projection.md`
  (SPEC-034KTMfXYD5ZlGZCRb8Paj — R1/R6/R7)
- `docs/test-specs/route-grant-set-projection.md`
  (TSPEC-034KVxaBMzQE5YtVOx4j3w — 宣言層制約・anchor 規約・
  record split Rules)
- `docs/tasks/add-route-grants-declaration-and-grant-set.md`
  (TASK-034KVxrNr5J7VrsdNaZDAP — 前提)

## Dependencies

- `docs/tasks/add-route-grants-declaration-and-grant-set.md`
  (T1) — `grants` schema・層制約・`grant_set` semantics の
  確定後に着手。T2/T3 とは並列実行可能(docs は compile
  依存を持たない — plan Dependency Graph)。

## Affected Files

- `docs/guides/parent-agent-integration.md`(modify)
- `docs/guides/configuration.md`(modify)
- `docs/guides/command.md`(modify)
- `docs/guides/usage.md`(modify)
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (modify — content update のみ)
- `crates/cheaper-cli/resources/routes.yaml`(modify — comment)

## Done When

- [ ] parent-agent-integration に `grants` field 説明(有効
      set・dedupe・eligibility・member bounds)・builtin/user
      層限定と project 拒否・`job_id != "exec"` authoring
      規約・record `executed` + 平文 arg 露出が記述されている
- [ ] configuration.md・command.md が `grants` member の
      `is_route_eligible`(trusted+`!fixed`)規則と層制約を
      反映している
- [ ] usage.md の additive-only 節が `executed` を例として
      含み平文 arg 値を 1 文で明記している
- [ ] routes.yaml に `grants` の shape comment が存在する
- [ ] pretooluse spec の route field 一覧・viability・record
      field に `grants?`/member 検査/`executed` が追記され、
      本 chain から relation が張られていない

## Out of Scope

- product code・test(全て T1-T3)
- `docs/guides/usage.md` への `route` record field 詳細記述
  (canonical は parent-agent-integration — DRY)
- substitute mode(#167)の docs・文書全体の正直性 sweep(#169)
- pretooluse spec への frontmatter relation 追加(spec chain
  側の管理に委ねる)

## Verification

- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type spec --json` で pretooluse spec を含め本 task 更新 doc が error 0
- [ ] 記述内容が design Docs 更新契約・TSPEC Rules と照合して
      矛盾しないこと(doc review)

---
id: "TASK-034L2Nh979kTbVrrg1v0zA"
type: "task"
status: "done"
title: "substitute-exec route mode の docs 更新・comment 追随"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
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
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/substitute-exec-route-mode.md"
  implemented-by: []
  depends-on:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/tasks/wire-route-run-substitute-exec-pair.md"
  blocks:
    - "docs/tasks/verify-substitute-exec-route-mode.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/substitute-exec-route-mode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/substitute-exec-route-mode.md"
  references:
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/guides/usage.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "crates/cheaper-cli/resources/routes.yaml"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
  defers: []
  deferred-by: []
---

# substitute-exec route mode の docs 更新・comment 追随

## Work

plan `docs/plans/substitute-exec-route-mode.md`
(PLAN-034L2MF3X78MvKvcSN8jmV)の T4。design
(DESIGN-034L0RxGVVTOqtaUUThowk)の Docs 更新契約・Churn
一覧「comment 追随」項に従い、`substitute` 宣言面・層制約・
viability retarget・interactive union・実行 anchor/record
attribution の substitute 規約を 4 guide + routes.yaml
comment + pretooluse spec(content update のみ)+ code
comment 追随へ反映する。design の指定行と記述内容を転記する
だけで、意味論を再解釈しない。

### 1. `docs/guides/parent-agent-integration.md`

- routes.yaml 宣言例と field 説明に `substitute` を追加 —
  意味: `{substitute.grant, substitute.argv}` が typed
  command の代わりに実行される substitute-exec mode・
  `argv` は executable を含まない宣言固定 args・`argv: []`
  合法・trusted 層(builtin/user)限定で project routes.yaml
  に書くと route-load error・`grant:` 単数は substitute
  宣言の有無に関わらず required(firing gate・attribution
  anchor)。
- viability の retarget(原 argv 照合を行わず substitute
  ペアを完全検査する)・`route.grant` の member 級降格・
  typed/substitute 両側の interactive 分類(substitute 側は
  宣言 executable 名と pin 解決名の union)・
  `expected_input` が substitute ペアを要求する点・record
  `executed` が substitute ペアを記録する点を明記。
- workflow authoring 規約 — exec job input は従来どおり
  `{name: $input.grant, args: $input.execArgs}`(inputs が
  実行ペアを運ぶため builtin binding は両 mode で無変更)。
- **stale 箇所の mode 追随**(design Docs 更新契約で指名):
  - 「実効 grant set は `{route.grant} ∪ grants`」(:303)→
    `{substitute.grant}` を追加。
  - 「`{grant, argv}` の expected-input 束縛は anchor が
    担う」(:296-299)・「`job_id == "exec"` … anchor grant +
    元 argv の expected-input に束縛」(:319-321)→
    substitute mode では束縛対象が宣言 substitute ペアに
    なる旨を追記。
  - 「子 process の exit code をそのまま mirror — 元
    command が非ゼロなら route run も非ゼロ」(:352-354)→
    substitute mode では substitute command の exit を
    mirror する旨を追記。
- semantic substitution の意味差(親 agent は substitute の
  exit/output を原 command のものとして解釈する trusted
  opt-in)を明記。

### 2. `docs/guides/configuration.md`

- unified exec 参照節の「route の `grant:`/`grants:` 参照は
  trusted+`!fixed` のみ」に `substitute.grant` も同
  predicate(`is_route_eligible`)へ従う記述を追加し、
  `substitute` 宣言自体は builtin/user 層限定である旨を併記。

### 3. `docs/guides/command.md`

- route grant 参照記述を `substitute.grant` も同じ
  eligibility 規則(scope==trusted && !fixed)に従う旨へ更新。

### 4. `docs/guides/usage.md`

- `kind:"route"` record の `executed` 記述に「substitute
  mode では `executed` が宣言 substitute ペアを記録する
  (route 帰属は宣言 identity・`commandHash` は原 command
  のまま)」を 1 文追記(record shape・additive-only 規約は
  不変 — `route` record field の canonical 記述は
  parent-agent-integration 側、DRY)。

### 5. `crates/cheaper-cli/resources/routes.yaml`

- `grants:` 直下(:6-8)に `# substitute: {grant: name, argv:
  [arg, …]}  # optional — substitute-exec mode(builtin/user
  層限定)` の shape comment を追加(宣言面の記述のみ —
  builtin route は substitute 非宣言)。

### 6. `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`

- refine 対象 spec として content update のみ: route field
  一覧に `substitute?`(trusted 層限定)・viability 記述に
  実行ペア retarget・`expected_input`/record 記述に
  substitute mode の anchor 規約を追記。relation
  (`refines` 等)は本 chain からは張らない — spec chain 側の
  管理に委ねる(design Docs 更新契約)。

### 7. code comment 追随(code 不変・文言のみ)

design Churn 一覧「comment 追随」項の site を全件:

| file | site | 変更 |
|------|------|------|
| `route/routes.rs` | `Route.grant` doc(:44-46) | 「expected_input anchor」の記述は substitute mode で実行 anchor が `substitute.grant` へ移るため mode 追随 |
| `route/routes.rs` | `Route.grants` doc(:48-51) | 有効 set 式へ `∪ {substitute.grant}` を追記 |
| `route/routes.rs` | `SuppressedRoute` doc(:77-78) | 「trust 不在で負けた project route」→「merge 敗北 route」へ semantics 拡張 |
| `route/routes.rs` | file header(:12-14) | merge 規約 comment へ displaced 記録を追記 |
| `route/resolve.rs` | `RouteResolution.suppressed` field doc(:101) | 「trust 不在で負けた project route 名」→ displaced 下位層 route も含む merge 敗北 route 名へ拡張 |
| `route/run.rs` | `executed` attribution comment(:475-479) | 「env-prefix-strip 済みの元 exec argv」は byte-matched の記述 — substitute mode では実測 substitute argv |
| `policy/exec.rs` | `project_grants` doc(:118-123) | set 式へ `substitute.grant` を追記(code・predicate は不変) |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/substitute-exec-route-mode.md`
  (PLAN-034L2MF3X78MvKvcSN8jmV — T4)
- `docs/designs/substitute-exec-route-mode.md`
  (DESIGN-034L0RxGVVTOqtaUUThowk — Docs 更新契約・Churn
  一覧「comment 追随」が AUTHORITATIVE: 各 file の対象行と
  必須記述)
- `docs/specs/substitute-exec-route-mode.md`
  (SPEC-034KXdoK2bMG3gQtIj8l6h — R1-R12)
- `docs/test-specs/substitute-exec-route-mode.md`
  (TSPEC-034L2MB0m2NOWZsyuTHsjA — 全 Rules)
- `docs/tasks/wire-route-run-substitute-exec-pair.md`
  (TASK-034L2NdmhEj1VYd7riHhtL — 前提)

## Dependencies

- `docs/tasks/wire-route-run-substitute-exec-pair.md`
  (T3) — comment 追随 site が T1-T3 の全 code file
  (routes.rs・resolve.rs・run.rs)にまたがるため、全 code
  着地後に着手(T1/T2 は T3 経由で推移的に完了前提)。

## Affected Files

- `docs/guides/parent-agent-integration.md`(modify)
- `docs/guides/configuration.md`(modify)
- `docs/guides/command.md`(modify)
- `docs/guides/usage.md`(modify)
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (modify — content update のみ)
- `crates/cheaper-cli/resources/routes.yaml`(modify — comment)
- `crates/cheaper-cli/src/route/routes.rs`・
  `crates/cheaper-cli/src/route/resolve.rs`・
  `crates/cheaper-cli/src/route/run.rs`・
  `crates/cheaper-core/src/policy/exec.rs`
  (modify — comment のみ、code 不変)

## Done When

- [ ] parent-agent-integration に `substitute` field 説明
      (mode 意味・`argv` 形・`argv: []` 合法)・trusted 層限定
      と project 拒否・viability retarget・interactive
      union・`expected_input` substitute ペア・record
      `executed`・workflow authoring 規約・semantic
      substitution の意味差が記述され、stale 4 箇所
      (:303/:296-299/:319-321/:352-354)が mode 追随している
- [ ] configuration.md・command.md が `substitute.grant` の
      `is_route_eligible`(trusted+`!fixed`)規則と builtin/
      user 層限定を反映している
- [ ] usage.md が substitute mode の `executed` を 1 文で
      明記している
- [ ] routes.yaml に `substitute` の shape comment が存在する
- [ ] pretooluse spec の route field 一覧・viability・record
      記述に `substitute?`/retarget/anchor 規約が追記され、
      本 chain から relation が張られていない
- [ ] code comment 追随 7 site(routes.rs ×4・resolve.rs
      :101・run.rs :475-479・exec.rs :118-123)が全件更新
      され code 差分がない

## Out of Scope

- product code・test(全て T1-T3)
- `docs/guides/usage.md` への `route` record field 詳細記述
  (canonical は parent-agent-integration — DRY)
- pretooluse spec への frontmatter relation 追加(spec chain
  側の管理に委ねる)
- `route resolve` 出力・usage record schema の変更(不変
  規約 — docs にも echo を書かない)

## Verification

- [x] `node scripts/check-doc-ids.mjs` exits 0 — 実測: "doc id check OK: 1168 markdown files scanned"
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type spec` で pretooluse spec を含め本 task 更新 doc が error 0 — 実測: exit 0(error 0 件 — `related` 非相互 link の warning のみ、spec chain 側管理のため本 chain からは張らない契約どおり)
- [x] 記述内容が design Docs 更新契約・TSPEC Rules と照合して矛盾しないこと(doc review) — 実測: 4 guide・routes.yaml・pretooluse spec・7 comment site が design の指定記述どおりに mode 追随済み
- [x] comment 追随 site に code 差分がないこと(`git diff` で comment のみ) — 実測: `git diff` で routes.rs ×4・resolve.rs :101・run.rs :475-479・exec.rs :118-123 の全 site が comment のみの差分であることを確認

---
id: "TASK-034L59Zaf7xiLEfPo8rlQF"
type: "task"
status: "todo"
title: "trusted route flag issuer redesign 最終検証 — targeted tests・doc-id・audit_docs・mise run check"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/168"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/plans/trusted-route-flag-issuer-redesign.md"
      - type: "file"
        path: "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
      - type: "file"
        path: "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
      - type: "file"
        path: "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
      - type: "file"
        path: "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
      - type: "file"
        path: "docs/specs/trusted-route-flag-issuer-redesign.md"
      - type: "file"
        path: "docs/adr/trusted-route-flag-issuer-redesign.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  implemented-by: []
  depends-on:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/remove-route-trusted-field-and-add-layer-gate.md"
    - "docs/tasks/add-project-trusted-hook-fail-open-regression.md"
    - "docs/tasks/document-trusted-route-flag-issuer-redesign.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/trusted-route-flag-issuer-redesign.md"
  references:
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign 最終検証 — targeted tests・doc-id・audit_docs・mise run check

## Work

plan `docs/plans/trusted-route-flag-issuer-redesign.md`
(PLAN-034L59uXskRD4ink3h2a13)の T4。T1-T3 で完了した実装が
design(DESIGN-034L4tqYwtc8q88WEIXPh9)の Verification
Notes・spec(SPEC-034LwdoInh3ie1KPlSjkJl)の Acceptance
Criteria 通りに成立していることを、targeted test・TSPEC
Rule 対応確認・full verification pipeline で閉じる。
product code は変更しない(検証と doc status 更新のみ —
gap 検出時のみ該当 test file へ pin 追加)。

### 1. spec Acceptance Criteria ↔ TSPEC Rule の対応確認

spec(SPEC-034LwdoInh3ie1KPlSjkJl)の Acceptance Criteria
各項目に対応する test/確認が存在し pass していることを確認
する(TSPEC-034L59Ghpuk8XWUN8qgXJu の Rules に対応 —
欠落があれば該当 task の責務として pin 追加):

| AC | 対応 test/確認領域 |
|----|-------------------|
| AC1(issue AC1): trust 発行主体 = 宣言層 provenance が spec/ADR に記録 | spec R1/R2 + ADR O1a/O2a Decision Outcome — 文書確認(実装済み判定は merge code + 層 gate の構造確認 §2) |
| AC2(issue AC2): project 層 `trusted` key → `InvalidRuntimePolicy` route-load error・`trusted: true` 宣言 project route が下位層を shadow できない・非宣言 project route は衝突時 `suppressed` 報告継続 | `routes.rs` unit `project_layer_trusted_is_rejected`(3 形 presence 拒否)+ `route.rs` integration `resolve_project_trusted_rejected`(exit 3)+ untrusted 衝突 arm(`suppressed` 報告 pin) |
| AC3(issue AC3): route trust と grant `scope` の整合文書化・`is_route_eligible` 単一 seam 不変 | spec R5 記述 + `parent-agent-integration.md` grant 参照節 + `policy/exec.rs` :114-116 無差分確認(diff) |
| AC4: `Route.trusted` field・merge `route.trusted` 読取り除去・builtin/user shadowing・同一層 dup `invalid` 隔離・resolve 出力 shape・record shape・hook fail-open に回帰なし | compile(field 削除完結)+ `usage_log.rs` record shape pin(:298-353)+ `hook.rs` fail-open 回帰 test + `cargo test -p cheaper-cli` 既存 route/hook/run 群無改訂 pass(`trusted` 前提 arm は新規約へ更新済み) |
| AC5: `trusted` 含む project routes.yaml 全件 load 失敗の破壊的変更が spec Scope/AC・design §Deferred・guides に明記(修復 = key 削除 1 行) | spec/design/guides 文書確認(#169 migration guide 材料) |

design Verification Notes 11 項目の対応: VN1/VN4 =
presence 拒否(AC2・unit + integration)、VN2 = builtin/user
受理・inert + 非 bool 型検査(unit)、VN3 = untrusted 衝突
suppressed(merge unit + route.rs + route_run.rs)、VN5 =
builtin/user 無条件 shadow 回帰(unit rename test)、VN6 =
非衝突 project 有効化(unit + 既存群)、VN7 = field/shape
不変(compile + usage_log pin + 出力 shape assertion)、
VN8 = hook fail-open(hook.rs 新規 test)、VN9 = grant
`scope` seam 不変・整合文書化(diff + docs 確認)、VN10 =
既存 test 群 pass(§3)、VN11 = 破壊的変更告知材料(文書確認)。

### 2. 構造確認

- 層 gate 配置 — `validate_route` で project 層 `trusted`
  presence 拒否が `passthroughEnv` parse 後・`trusted` bool
  形状検査前にあり(presence-first — 非 bool 値でも
  `not a boolean` ではなく `not declarable` が先に返る)、
  `grants`/`substitute` gate と同型の `schema_error` →
  `InvalidRuntimePolicy` 経路を辿ること。
- merge 無条件形 — `RouteLayer::Project` 分岐が
  `route.trusted` を読まず `effective.contains_key` のみで
  分岐し、displaced 記録 arm が除去されていること。
- field 削除完結 — `grep -rn "Route {" crates/` の literal
  site 全件に `trusted` field がなく、`grep -rn
  "route.trusted\|\.trusted" crates/` で merge 以外の残存
  consumer がないこと(`KNOWN_KEYS`・層 gate・受理 bool
  検査・test fixture YAML・追従済み comment 内の `trusted`
  言及は残存してよい — `Route` field・`route.trusted` 参照
  の残存 0 を確認)。
- `KNOWN_KEYS` 維持 — `"trusted"` が KNOWN_KEYS に残り、
  typo(`trustd`)が unknown key で拒否される規約が不変
  (unit test・code 確認)。
- suppressed provenance — `suppressed` が運ぶのは負けた
  project route のみで、「`suppressed` が下位層 route 名を
  報告する」assert が test 群に残っていないこと
  (`grep -rn "suppressed" crates/cheaper-cli` で assert
  文言を確認)。
- hook fail-open — 新規 test が違反 file を **project 層**
  `<workspace>/.cheaper/routes.yaml` に置いていること
  (user 層 `f.cfg` ではない — user 層受理で違反にならない
  ため必須条件)。
- code 不変境界 — `route/supply.rs`・`route/run.rs`・
  `route_cmd.rs`・`usage_log.rs`・`policy/exec.rs`・
  `policy/load.rs`・`tools/*`・`hook/targets.rs`・
  `routed-exec.workflow.json` が merge-base から無差分で
  あること(`git diff` 確認 — `is_route_eligible` 単一
  seam :114-116 不変)。
- `TODO`・`unimplemented!`・`todo!`・stub の残存なし。

### 3. full verification・status 更新

- `mise run check` が pass すること(biome・fmt・clippy
  `--all-targets`・version・doc-ids・`cargo test --workspace`・
  `node --test`・lockfile・apm-payload — `--features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きの
  canonical pipeline)。
- T1-T4 の task status を `done`・plan status を `completed`
  へ更新する(実測結果を各 task の `## Verification` 欄へ
  記録)。spec(SPEC-034LwdoInh3ie1KPlSjkJl)・ADR
  (ADR-034LT2o68CgcOinMEs8PZ8)は現行 `proposed` — status
  昇格(approved/accepted)の要不要を含め spec chain 側の
  慣行に従う(#167 では planning commit 前に spec=approved・
  ADR=accepted 済み — 本 chain では昇格処理を spec chain
  側へ報告してから status 追随を確定する)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/trusted-route-flag-issuer-redesign.md`
  (PLAN-034L59uXskRD4ink3h2a13 — T4)
- `docs/designs/trusted-route-flag-issuer-redesign.md`
  (DESIGN-034L4tqYwtc8q88WEIXPh9 — Verification Notes が
  AUTHORITATIVE)
- `docs/specs/trusted-route-flag-issuer-redesign.md`
  (SPEC-034LwdoInh3ie1KPlSjkJl — Acceptance Criteria)
- `docs/test-specs/trusted-route-flag-issuer-redesign.md`
  (TSPEC-034L59Ghpuk8XWUN8qgXJu — 全 Rules の最終確認)

## Dependencies

- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`
  (T1) — field 削除・層 gate・merge 無条件形・結合 test
  rewrite が完了し workspace が compile していること。
- `docs/tasks/add-project-trusted-hook-fail-open-regression.md`
  (T2) — hook fail-open 回帰 test が着地していること。
- `docs/tasks/document-trusted-route-flag-issuer-redesign.md`
  (T3) — docs 更新契約が完了していること。
- (T2・T3 は並列可能 — 本 task は両者の完了後)

## Affected Files

- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`・
  `docs/tasks/add-project-trusted-hook-fail-open-regression.md`・
  `docs/tasks/document-trusted-route-flag-issuer-redesign.md`・
  `docs/tasks/verify-trusted-route-flag-issuer-redesign.md`
  (status 更新)
- `docs/plans/trusted-route-flag-issuer-redesign.md`(status 更新)
- `docs/specs/trusted-route-flag-issuer-redesign.md`・
  `docs/adr/trusted-route-flag-issuer-redesign.md`(status/AC
  追随 — spec chain 側の慣行)
- 観測 pin 欠落が見つかった場合のみ、該当 test file へ pin
  追加(本来は T1/T2 の責務 — 本 task で新規 test を書くのは
  gap 検出時に限る)

## Done When

- [ ] spec Acceptance Criteria AC1-AC5 全項目に対応する
      test/確認が存在し pass(TSPEC Rule との対応表が §1 で
      確認済み)
- [ ] 層 gate 位置(presence-first)・merge 無条件形・field
      削除完結・`KNOWN_KEYS` 維持・suppressed 狭義化・hook
      fail-open の構造確認が実測されている(§2)
- [ ] `grep -rn "Route {" crates/` の literal site 全件
      `trusted` field なし・`route.trusted`/`.trusted` 参照
      残存 0・stub 残存なし
- [ ] `mise run check` exits 0
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `audit_docs` が本 issue の新規 doc で error 0
- [ ] 4 task + plan(+ spec/ADR の status 追随)が更新されて
      いる

## Out of Scope

- product code の追加変更(実装漏れが見つかった場合は該当
  task へ差し戻す — T4 で大規模な実装をやり直さない)
- operator/user 側 consent registry(O1b)・builtin/user inert
  `trusted` の schema 完全除去(Deferred)
- 後続分解 issue(#169 migration guide・文書正直性 sweep)の
  作業
- `mise run check` 以外の新規検証基盤
- 既存 test-spec 内の stale `trusted` 記述の bookkeeping
  sweep(test-spec chain 側の管理 — design §Docs 更新契約)

## Verification

- [ ] 上記 §1-§3 の全項目が pass(コマンド実測を記録)
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` で本 issue の新規 doc が error 0
- [ ] `mise run check` exits 0

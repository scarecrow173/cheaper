---
id: "TASK-034L59YCvXZOpmmELxVllt"
type: "task"
status: "done"
title: "trusted route flag issuer redesign の docs 更新契約"
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
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
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
  blocks:
    - "docs/tasks/verify-trusted-route-flag-issuer-redesign.md"
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
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/command.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "crates/cheaper-cli/resources/routes.yaml"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign の docs 更新契約

## Work

plan `docs/plans/trusted-route-flag-issuer-redesign.md`
(PLAN-034L59uXskRD4ink3h2a13)の T3。design
(DESIGN-034L4tqYwtc8q88WEIXPh9)の Docs 更新契約に従い、
「trust = 宣言層 provenance」「project 層 `trusted` presence
は load 全体を `InvalidRuntimePolicy` で失敗させる破壊的
変更」「builtin/user 層の `trusted:` は受理・inert」「route
trust と grant `scope` の単一モデル整合(spec R5)」を
3 guide + routes.yaml comment + pretooluse spec(content
update のみ)へ反映する。design の指定行と記述内容を転記する
だけで、意味論を再解釈しない。

### 1. `docs/guides/parent-agent-integration.md`

- route 宣言層説明(:233-246)を rewrite: 「project 層が下位を
  shadow するのは `trusted: true` のときのみ」を
  「project 層 route は下位層を shadow しない — trust は
  宣言値ではなく宣言層 provenance(builtin/user 層宣言 =
  trusted)が決める。`match.command` 衝突した project route
  は `suppressed` provenance として報告され、下位 route は
  active のまま残る。project routes.yaml の entry に
  `trusted` key が存在すると値に関わらず route load 全体が
  `InvalidRuntimePolicy` で失敗する。builtin/user 層の
  `trusted:` は受理されるが inert」へ追従。
- route trust と grant `scope` の整合(spec R5 — route は宣言
  層・grant は authoring 層の provenance が trust を決める
  同一モデルの 2 面、route grant 参照は `is_route_eligible`
  `scope=="trusted" && !fixed` 単一 seam 不変)を grant
  参照記述(:291-295 近辺)へ 1 文追加。
- 破壊的変更の症状を明記 — hook 経路では fail-open で
  workspace の全層 routing が不活性化し、症状は debug log
  のみで `route resolve` を実行するまで可視化されないこと
  (strict 経路は exit 3 — migration guide #169 向け材料と
  して design §Risks/§Deferred の記述を反映)。

### 2. `docs/guides/configuration.md`

- unified exec 参照節(:215-225)の「project routes.yaml の
  entry に `grants`・`substitute` key が存在すると route
  load が失敗」列挙へ `trusted` を追加し、「project が宣言
  できるのは単数 `grant:` anchor 参照のみ(非衝突
  `match.command` で)」の記述と整合させる。

### 3. `docs/guides/command.md`

- route grant 参照記述(:42)の「project 層 entry に
  `grants`・`substitute` key が存在すると route load が
  失敗」へ `trusted` を追加。

### 4. `crates/cheaper-cli/resources/routes.yaml`

- `substitute` shape comment(:8)直下へ
  `# trusted: bool  # optional — builtin/user 層のみ受理(inert);project 層宣言は load error`
  の 1 行を追加(builtin route 自身は `trusted` を宣言しない
  — 記述のみ)。

### 5. `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`

- refine 対象 spec として content update のみ:
  - R1 field 一覧(:216-219)の `trusted?` 注記を
    「builtin/user 層のみ受理される inert bool — project
    層宣言は route-load error」へ追従。
  - `trusted` 記述(:253-256)を宣言層 provenance trust へ
    追従。
  - R7(:285-298)の層間 trust を「project 層 route は下位層
    を shadow しない — trust は `trusted: true` 宣言値では
    なく宣言層 provenance が決める(O1a/#168 — project 層
    `trusted` presence は `InvalidRuntimePolicy` で拒否)」
    へ追従。
- `refines` relation は本 chain からは張らない — spec chain
  側の管理に委ねる(design Docs 更新契約)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/trusted-route-flag-issuer-redesign.md`
  (PLAN-034L59uXskRD4ink3h2a13 — T3)
- `docs/designs/trusted-route-flag-issuer-redesign.md`
  (DESIGN-034L4tqYwtc8q88WEIXPh9 — Docs 更新契約が
  AUTHORITATIVE: 各 file の対象行と必須記述)
- `docs/specs/trusted-route-flag-issuer-redesign.md`
  (SPEC-034LwdoInh3ie1KPlSjkJl — R5 整合記述・R6・AC5
  破壊的変更告知)
- `docs/test-specs/trusted-route-flag-issuer-redesign.md`
  (TSPEC-034L59Ghpuk8XWUN8qgXJu — grant `scope` seam 整合・
  破壊的変更告知材料 Rules)
- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`
  (TASK-034L59KS9oEjgDzshYZUSn — 前提: 確定 semantics を
  docs へ転記する)

## Dependencies

- `docs/tasks/remove-route-trusted-field-and-add-layer-gate.md`
  (T1) — 記述対象の semantics(field 削除・層 gate・merge
  無条件形・suppressed 狭義化)が code に着地していること。
  T2 とは file set が disjoint のため並列可能。

## Affected Files

- `docs/guides/parent-agent-integration.md`(modify)
- `docs/guides/configuration.md`(modify)
- `docs/guides/command.md`(modify)
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (modify — content update のみ)
- `crates/cheaper-cli/resources/routes.yaml`(modify —
  comment のみ)

## Done When

- [x] parent-agent-integration.md の route 宣言層説明が
      「project は下位層を shadow しない(trust = 宣言層
      provenance)・project `trusted` presence は load 全体
      失敗・builtin/user `trusted:` は受理・inert・衝突
      project route は `suppressed` 報告」を記述し、hook
      経路 fail-open の症状(debug log のみ・`route resolve`
      まで不可視)と grant `scope` 整合の 1 文が反映されて
      いる
- [x] configuration.md・command.md の層 gate 列挙が
      `grants`・`substitute`・`trusted` の 3 key 拒否を記述
      している
- [x] resources/routes.yaml に `trusted` shape comment が
      存在する
- [x] pretooluse spec の R1 field 一覧・`trusted` 記述・R7
      層間 trust が宣言層 provenance trust へ追従し、本
      chain から relation が張られていない
- [x] `node scripts/check-doc-ids.mjs` exits 0

## Out of Scope

- product code・test(全て T1/T2 — code comment 追随も T1 に
  含まれる: 全 site が T1 の file に閉じるため)
- pretooluse spec への frontmatter relation 追加(spec chain
  側の管理に委ねる)
- `route resolve` 出力・usage record schema の変更(不変規約
  — docs にも echo を書かない)
- migration guide 本文(#169 が担う — 本 task は破壊的変更の
  症状・修復(key 削除 1 行・user 層 vendor 経路)を guides
  の該当節へ記すのみ)
- 既存 test-spec(`route-resolution.md`・`substitute-exec-
  route-mode.md`・`route-grant-set-projection.md`)内の
  `trusted: true` shadow・displaced 記録記述の更新 —
  本 chain の TSPEC が新規約の owner であり、旧 test-spec の
  stale 記述の bookkeeping sweep は spec/test-spec chain 側
  の管理(design §Docs 更新契約の記述どおり後続 flow 扱い)

## Verification

- [x] `node scripts/check-doc-ids.mjs` exits 0 — 実測: "doc id check OK: 1179 markdown files scanned"
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type spec` で pretooluse spec を含め本 task 更新 doc が error 0 — 実測: exit 0(error 0 件 — `related` 非相互 link の warning のみ、spec chain 側管理の契約どおり)
- [x] 記述内容が design Docs 更新契約・TSPEC Rules と照合して矛盾しないこと(doc review) — 実測: 3 guide・routes.yaml comment・pretooluse spec R1/R7 が design の指定記述どおり宣言層 provenance trust へ追従
- [x] `git diff` で対象 file が記述/comment のみの差分であること(code・relation 差分なし) — 実測: frontmatter・relation 差分なし、記述/comment のみ

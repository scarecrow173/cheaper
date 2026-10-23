---
id: "TASK-034KQsAwXBVQJ56nGgagYT"
type: "task"
status: "todo"
title: "unified exec grant model の最終検証と status/evidence 更新"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/plans/unified-exec-grant-model.md"
      - type: "file"
        path: "docs/test-specs/unified-exec-grant-model.md"
      - type: "file"
        path: "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
      - type: "file"
        path: "docs/tasks/plumb-unified-exec-through-tools.md"
      - type: "file"
        path: "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
      - type: "file"
        path: "docs/tasks/document-unified-exec-grant-model.md"
      - type: "file"
        path: "docs/tasks/verify-unified-exec-grant-model.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
    - "docs/tasks/document-unified-exec-grant-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/unified-exec-grant-model.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/unified-exec-grant-model.md"
  references:
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
---

# unified exec grant model の最終検証と status/evidence 更新

## Work

plan `docs/plans/unified-exec-grant-model.md`
（PLAN-034KQs6XKnAqahkSVke78h）の T5（final task）。T1–T4
の実装・文書が TSPEC（TSPEC-034KQs1NIL8ahirht6QteV）の全
Rule・SPEC（SPEC-034KOpKfZpegX2MLPD0uq6）の受入基準を
満たすことを**実測で**検証し、plan/task/test-spec の
status・verification evidence を実結果で更新する。gate は
「実際に走らせて pass した check のみを claim する」— 未
実行の check を pass と記録しない（design・AGENTS 原則）。

### 1. targeted cargo test（TSPEC Rules の実測）

TSPEC の各 Rule が対応する test target で実際に pass する
ことを確認する。single-crate では dev-seam feature 適用外
のため、scoped crate test は flag 無し、workspace/CLI/
runtime target は dev-seam flag 付きで実行する。

```bash
# core — unified schema/resolver/provenance/collision/parity/deny 適用点 1
mise exec -- cargo test -p cheaper-core
# tools — disjoint surface・deny 2/3・writeScope tri-state・
# pin-fail×degradable・catalog fallback・parity・harness seam
mise exec -- cargo test -p cheaper-tools
# workspace — route eligibility・project_grant・invocation
# parity・--grant reject・tool:"process" 互換・全 crate compile
mise exec -- cargo test --workspace \
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam
# CLI route/invocation 回帰（module filter）
mise exec -- cargo test -p cheaper-cli \
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam \
  --test integration route_run::
```

### 2. canonical check contract

```bash
# CI 同等の canonical verification — 全 pipeline を一度に
mise run check
```

`mise run check` = `biome check` → `cargo fmt --check` →
`cargo clippy` → version/doc-id check → `cargo test
--workspace` → `node --test` → lockfile/payload check
（cargo 系は dev-seam feature 付きで実行済み — ADR-
034DyGyITd0YaATg8fkld7）。

### 3. doc-suite check

```bash
# document id/index 整合
node scripts/check-doc-ids.mjs
# plan/task/test-spec audit — 新規 file で error 0
# (audit_docs skill 経由 — plan・task・test-spec 各対象)
```

`check-doc-ids.mjs` が `docs/` frontmatter id・README index
link・relation path を整合させる。audit_docs で plan・
task・test-spec の必須 section・relation（`implements`/
`depends-on`/`blocks`/`verified-by`/`references`/`derives-
from`/`source`/`changes`）・placeholder 残存・cross-chain
edge を検査し、**新規 file で error 0** を確認する。

### 4. status・evidence 更新

実測で pass した check のみを根拠に frontmatter/body を更新:

- `docs/plans/unified-exec-grant-model.md` — status
  `approved` → `completed`（全 task done 時）。verification
  matrix へ実行済み command・結果・commit hash を記録。
- `docs/test-specs/unified-exec-grant-model.md` — status
  `draft` → `approved`（全 Rule が実装 test で pass 確認
  後）または `proposed`。evidence（pass した test target・
  日付・hash）を記録。
- `docs/tasks/{T1..T4}` — status `todo` → `done`（実装・
  検証済みのもののみ）。verification checkbox を実測結果で
  tick。
- `docs/tasks/verify-unified-exec-grant-model.md`（本 task）—
  status → `done`・verification checkbox を実測で tick。
- `docs/{plans,test-specs,tasks}/README.md` — status 列を
  frontmatter 変更と整合させる（index script 再生成 or
  手動）。

未 pass・未実行の項目は status/evidence に pass と記録せず、
残課題として Open Gaps/未実行 check を明記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/unified-exec-grant-model.md`
  （PLAN-034KQs6XKnAqahkSVke78h — T5・verification matrix が
  更新対象）
- `docs/test-specs/unified-exec-grant-model.md`
  （TSPEC-034KQs1NIL8ahirht6QteV — 全 Rule が検証対象）
- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — R1–R11 受入基準）
- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT — Verification Notes・
  transient compile boundary・error 契約）
- `docs/tasks/plumb-unified-exec-through-cli-and-runtime.md`
  （TASK-034KQsrpnzHdeRKf5M3KiN — 前提・workspace compile）
- `docs/tasks/document-unified-exec-grant-model.md`
  （TASK-034KQsvyJ7l63g3i4Gjnwl — 前提・文書整合）

## Dependencies

- `docs/tasks/plumb-unified-exec-through-cli-and-runtime.md`
  （T3） — workspace 全 compile + route/invocation/`--grant`
  の unified 追随が検証対象。
- `docs/tasks/document-unified-exec-grant-model.md`（T4） —
  guide/reference 整合が検証対象。
- transitive: T1・T2（T3 経由）。DAG の terminal — outgoing
  edge なし。

## Affected Files

- `docs/plans/unified-exec-grant-model.md`（status→completed・
  evidence）
- `docs/test-specs/unified-exec-grant-model.md`（status→
  approved・evidence）
- `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （status→done・evidence）
- `docs/tasks/plumb-unified-exec-through-tools.md`（status→
  done・evidence）
- `docs/tasks/plumb-unified-exec-through-cli-and-runtime.md`
  （status→done・evidence）
- `docs/tasks/document-unified-exec-grant-model.md`（status→
  done・evidence）
- `docs/tasks/verify-unified-exec-grant-model.md`（本 file —
  status→done・evidence）
- `docs/{plans,test-specs,tasks}/README.md`（status 列整合）

## Done When

- [ ] `cargo test -p cheaper-core`・`-p cheaper-tools`・
      `cargo test --workspace --features cheaper-runtime/
      dev-seam,cheaper-cli/dev-seam`・`--test integration
      route_run::` が**実際に実行され exits 0**
- [ ] `mise run check` が**実際に実行され**全 pipeline pass
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] audit_docs で plan/task/test-spec の新規 file が
      **error 0**
- [ ] plan/test-spec/task/README の status が実測 pass のみ
      根拠に更新され、evidence（command・結果・hash）が記録
      され、未 pass/未実行は Open Gaps/未実行 check として
      明記されている
- [ ] TSPEC 全 Rule・SPEC R1–R11 が実装 test で pass 確認
      （未達は残課題として記録）

## Out of Scope

- T1–T4 の実装本体（本 task は検証・status 更新のみ）
- 新規 test/code の追加（検証で欠落が判明した場合は該当
  task へ差し戻し・新規 task 化）
- 別 issue の残課題（#166 route grant-set `grants[]`・
  substitute exec mode issue 5・trusted flag 主体 issue 6・
  #169 残 sweep）

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools` exits 0
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration route_run::` exits 0
- [ ] `mise run check` exits 0（全 pipeline）
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] audit_docs（plan/task/test-spec）新規 file error 0

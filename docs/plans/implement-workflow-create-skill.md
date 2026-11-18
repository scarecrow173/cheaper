---
id: "PLAN-034ZudkXx2PLdh6cgGpRns"
type: "plan"
status: "approved"
title: "implement cheaper-workflow-create skill"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/workflow-create-skill-issue-204.md"
    - "docs/adr/workflow-create-skill-uses-interface-validation-and-reviewable-catalog-diff.md"
  implemented-by:
    - "docs/tasks/author-cheaper-workflow-create-skill.md"
    - "docs/tasks/link-workflow-create-skill-from-guides.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-create-skill-issue-204.md"
    - "docs/adr/workflow-create-skill-uses-interface-validation-and-reviewable-catalog-diff.md"
    - "docs/designs/workflow-create-skill.md"
    - "docs/discovery/workflow-create-skill-issue-204.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# implement cheaper-workflow-create skill Implementation Plan

## Goal

APM skill `cheaper-workflow-create` を新設し、親 Agent が Workflow
JSON v1 を `workflow interface` 検証と reviewable catalog diff で
設計・保存・登録できるようにする。

## Scope

1 つの implementation stream とする。skill 本文と guide 導線は
同一の contract(schema v1・interface 検証・catalog manifest・承認
境界)を共有するため分割しない。

境界:

- Rust/`crates/`・hook 実装・deploy 物には触れない。
- `workflow run` 起動・副作用 smoke は行わない。
- catalog 書込み CLI・新規 schema・discover(#203)実装は対象外。

## Upstream Documents

- Approved requirements: `../specs/workflow-create-skill-issue-204.md`
- Accepted boundary decision: `../adr/workflow-create-skill-uses-interface-validation-and-reviewable-catalog-diff.md`

## Design Inputs

- Approved implementation boundary: `../designs/workflow-create-skill.md`
- Source exploration and verified CLI evidence: `../discovery/workflow-create-skill-issue-204.md`

## File Map

| Files | Owner slice | Responsibility |
| --- | --- | --- |
| `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` (new) | Skill authoring | 発動・承認境界・authoring 手順・schema 要点・template 5 形態・検証手順・catalog 登録・出力契約 |
| `docs/guides/workflow-usage.md` | Guide 導線 | catalog 節に create skill への導線 |
| `docs/guides/parent-agent-integration.md` | Guide 導線 | skills 一覧の更新(5→6 件)・workflow 発見節の承認境界整合 |
| `docs/impl/ir/` | Impl record | 実装・検証の記録 |

File ownership は task ごとに排他とする。skill 本文と guide 更新は
別 task とし、skill 確定後に guide を行う。

## Implementation Sequence

1. `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` を
   新設する。front matter(`name`/`description`)・When to use・
   Approval boundary・Authoring procedure・Workflow JSON v1
   anatomy・Templates(検証済み 5 形態)・Validate(error 対処表)・
   Register in the catalog・Report back・Good vs bad の節構成。
   template は `tmp/wf-fixtures/` で `workflow interface` を実測
   pass した fixture(minimal-worker・tool-worker-when・fan-in)と
   `routed-exec.workflow.json`・`saved-workflow.json` の形に揃える。
2. `docs/guides/workflow-usage.md` の catalog 節に create skill
   への導線を追加、`docs/guides/parent-agent-integration.md` の
   skills 一覧と workflow 発見節を更新する。
3. doc_status audit・`mise run check`・`git diff --check` を実行し、
   impl record を残す。

## Dependency Graph

- Step 1 → Step 2: guide の導線は確定した skill 名・手順を指す。
- Step 3 は Step 1-2 に依存する。

## Dependencies and Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| skill 内例が `workflow interface` と drift する | Low | High | 実 fixture で pass した形のみ採用し、検証手順を skill 必須 step にする |
| catalog 登録手順が #201 契約とずれる | Low | Medium | manifest field/bound/status/diagnostics は `workflow-usage.md` と `catalog.rs` の語彙で記述 |
| 承認境界の記述が弱く、実行/権限変更へ踏み込む | Low | High | Approval boundary を独立節にし、出力契約で「実行は別承認」を明記 |

## Task Breakdown

plan 承認後、owner slice ごとに task document を作成する:
(a) SKILL.md authoring(検証済み template・手順・例)、
(b) guide 導線更新 + impl record。
各 task は実 file 変更と検証を同一 slice で行う。

## Verification

| Slice | Sequence step | Command or check | Pass condition |
| --- | --- | --- | --- |
| Skill templates | 1 | `workflow interface` on skill fixture 群 | 全 template が `ok:true` exit 0、invalid case が exit 2 |
| Catalog 手順 | 1 | `workflow list` on scratch workspace | 追記 entry が `status:"ok"`、duplicate/missing が diagnostics |
| Doc 整合 | 3 | `doc_status audit --type all --blocking --json` | blocking 0 |
| Full canonical | 3 | `mise run check` | exit code 0 |

## Rollback Strategy

- Trigger: skill が現行 CLI と不整合な手順を教える場合、catalog
  契約の誤記が判明した場合に rollback する。
- feature branch の #204 commit を revert する。docs/skill 追加のみ
  で永続 data を生成しないため cleanup 対象は無い。

## Open Questions / Gaps

- blocking な gap は無い。#203 discover skill との相互参照は
  deferred(片方向参照で不整合なし)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution。docs/skill 追加のみで
  subagent 分割の実益がない。
- Independent work streams: なし(単一 stream)。
- User approval required before dispatch: yes
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Assumptions: skill は単一 SKILL.md で完結し reference file を
  追加しない。template の pointer は closed schema で確かめられる
  path のみ使う。
- Review focus: skill の承認境界が issue の発動・保存/実行分離と
  一致すること、例が `workflow interface` pass の実測形と一致する
  こと、catalog 手順が #201 manifest 契約と一致すること。

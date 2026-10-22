---
id: "TASK-034KvxH8TvZ1HF0bfjydpk"
type: "task"
status: "done"
title: "実装 issue URL の記録と issue #151 成果物基準の検証"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/151"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/file-permission-model-redesign-implementation-issues.md"
    modified:
      - type: "file"
        path: "docs/plans/file-permission-model-redesign-implementation-issues.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  implemented-by: []
  depends-on:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/permission-model-redesign-issue-decomposition.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/discovery/permission-model-responsibility-redesign.md"
  defers: []
  deferred-by: []
---

# 実装 issue URL の記録と issue #151 成果物基準の検証

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T8(終端 task)。T1-T7 で起票した
7 issue の URL を集約記録し、issue #151 の成果物基準が満たされた
ことを検証する。

### 手順

1. T1-T7 各 task doc の「起票結果」欄から issue URL・番号を回収
   する。未記録の task があれば本 task を blocked とし、該当
   task を先に完了させる。
2. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計" --state all --json number,title,url`
   で 7 件丁度あることを確認し、回収した URL・番号と突き
   合わせる。
3. 各 issue の body を確認し、deps 記述が placeholder
   (`<ISSUE`)を含まず実番号で記述されていること、`## Scope`・
   `## Acceptance Criteria`・`## Dependencies`・`## 参照` の
   4 節を持つことを検証する(TSPEC-034Kvx6yD4sfTj8KFkFIF3 の
   Rules)。
4. `docs/plans/file-permission-model-redesign-implementation-issues.md`
   の Task Breakdown 末尾「起票した issue の URL 記録欄」の
   `(未起票)` を実 URL で埋める。
5. implementation record を作成する:
   `node .agents/skills/impl-doc/scripts/new_impl_record.js --title "権限モデル再設計の実装 issue 起票 (issue #151)" --task docs/tasks/record-issue-urls-and-verify-investigation-criteria.md --status completed`
   生成後、`docs/impl/ir/file-permission-model-redesign-
   implementation-issues.md` へ rename し、起票した 7 issue の
   URL・起票日時・検証結果(手順 2-3 の確認結果)を Summary/
   Implementation 節へ記録する。
6. issue #151 へ分解結果サマリ comment を 1 件投稿する:
   `gh issue comment 151 --repo scarecrow173/cheaper --body-file <temp file>`。
   comment 本文は「## 実装 issue 分解(完了)」と題し、7 issue の
   URL + deps 一覧表 + 参照 doc(design `docs/designs/
   permission-model-responsibility-redesign.md`、ADR
   `docs/adr/enforce-declared-contracts-over-content-audit.md`、
   spec・discovery・plan)を含める。
7. issue #151 の成果物基準 5 項目を確認する:
   - (1) 現状調査表 →
     `docs/discovery/permission-model-responsibility-redesign.md`
     (DISC-034KuleA9g82Ir4xWfmTYq — 53 項目の分類表)
   - (2) trust/permission モデル提案 → spec + ADR(Option B
     宣言契約モデル — ADR-034KuqZelggTIb8Z8K9QB8 accepted)
   - (3) ADR/spec 改訂案 → accepted ADR + approved spec/design
     + design「§Supersede / Refine 一覧」の refine 対象確定
   - (4) 実装分割と依存関係の記録 → 7 issue + deps(design
     「§実装 issue 分解」と一致)
   - (5) 検証計画 → 各 issue の AC + design「§Verification
     Notes」+ 本検証手順
   5 項目全てに対応する証跡を IR に記録する。
8. `node scripts/check-doc-ids.mjs` と
   `node .agents/skills/doc-status/scripts/audit_docs.js --json --type task` / `--type plan` / `--type test-spec`
   を実行し、新規 error が 0 であることを確認する
   (`--type` は idea|brainstorm|discovery|spec|plan|task|design|
   adr|test-spec|all を受理 — impl は対象外)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
- `docs/test-specs/permission-model-redesign-issue-decomposition.md`
  (TSPEC-034Kvx6yD4sfTj8KFkFIF3 — 検証 Rules)
- `docs/specs/permission-model-responsibility-redesign.md`
- `docs/designs/permission-model-responsibility-redesign.md`
- `docs/adr/enforce-declared-contracts-over-content-audit.md`
- `docs/discovery/permission-model-responsibility-redesign.md`

## Dependencies

- T1-T7 全 task(7 issue の起票結果が揃っていることが前提)

## Affected Files

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
  (Task Breakdown の URL 記録欄)
- `docs/impl/ir/file-permission-model-redesign-implementation-issues.md`
  (新規 — implementation record)
- (GitHub) issue #151 への comment ×1

## Done When

- [x] 7 issue の URL が plan Task Breakdown 記録欄・IR に記録され、
      各起票 task の起票結果欄にも URL がある
- [x] issue #151 に分解結果サマリ comment が 1 件投稿されている
- [x] 成果物基準 5 項目の証跡が IR に記録されている
- [x] `node scripts/check-doc-ids.mjs` が pass
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json --type task` / `--type plan` / `--type test-spec`
      で新規 error 0

## Verification

- [x] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計" --state all --json number,title,url`
      がちょうど 7 件を返す
- [x] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計" --state all --json body --jq ".[].body"`
      の各出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を含み `<ISSUE` を含まない
- [x] `node scripts/check-doc-ids.mjs` が exit 0
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json --type task`
      の error 件数が本 task 群の追加前と同じ(新規 error 0)

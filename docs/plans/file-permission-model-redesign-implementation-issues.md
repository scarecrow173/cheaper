---
id: "PLAN-034KvwX4x7oaZlaECqhHc1"
type: "plan"
status: "approved"
title: "権限モデル再設計の実装 issue 起票計画 (issue #151)"
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
      - type: "file"
        path: "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
      - type: "file"
        path: "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
      - type: "file"
        path: "docs/tasks/file-issue-unified-exec-grant-model.md"
      - type: "file"
        path: "docs/tasks/file-issue-route-grant-set-and-record-split.md"
      - type: "file"
        path: "docs/tasks/file-issue-substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
      - type: "file"
        path: "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/permission-model-responsibility-redesign.md"
  implemented-by:
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "docs/test-specs/permission-model-redesign-issue-decomposition.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  derived-by:
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
  verifies: []
  verified-by:
    - "docs/test-specs/permission-model-redesign-issue-decomposition.md"
  references:
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
  defers: []
  deferred-by: []
---

# 権限モデル再設計の実装 issue 起票計画 (issue #151) Implementation Plan

## Goal

DESIGN-034KuDkWOmNXiBddEQ4XWK「§実装 issue 分解」で確定した
7 つの実装 issue を `gh` CLI で `scarecrow173/cheaper` へ起票し、
発行された issue URL を本 plan・各 task・implementation record へ
記録する。最後に issue #151 の成果物基準(現状調査表・trust モデル
提案・ADR/spec 改訂・実装分割の記録・検証計画)が満たされたことを
検証する。

## Scope

単一の implementation stream。issue 番号は作成順に確定し、後続
issue の deps 記述が先行 issue の番号を引用するため、起票順序が
内容を左右する — 分割せず 1 plan に収める。

| In | Out |
|---|---|
| 実装 issue 7 件の起票(title/scope/deps/AC は design からの転記済み — 各 task に全文収録) | 起票した issue の実装本体(product コード変更は後続 issue 群の仕事) |
| 各 issue body への dependency graph 記述 + `## 参照` での issue #151・design・ADR へのリンク | issue への label/milestone/assignee 付与(repo 体系未確認のため素起票) |
| issue #151 への分解結果サマリ comment(1 件、T8) | Option E exec-less route の採否決定(design で open のまま — issue 5 本文に open 事項として記録) |
| issue URL の plan/task/IR への記録 + 成果物基準の検証 | ADR-034KuqZelggTIb8Z8K9QB8 の ratification(既に accepted — issue 1 の前提を満たす) |

## Upstream Documents

- `docs/specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5, approved)
- `docs/adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8, accepted — issue 1 着手前提の
  ratification は完了済み)

## Design Inputs

- `docs/designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK, approved) — 「§実装 issue
  分解」が起票内容の authoritative source。Disposition・M1-M5・
  handoff 表・refine 一覧は全てここから各 task へ転記済み。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| (GitHub) `scarecrow173/cheaper` の新規 issue ×7 | 実装 issue 群の本文と deps 記述 | `gh issue create` で発行。番号は T1→T7 の作成順に確定 |
| `docs/tasks/file-issue-*.md`(7 件) | 各 issue の起票仕様(title・body 全文・deps・AC・comment 手順) | worker が design を再解釈しないよう body を全文収録 |
| `docs/tasks/record-issue-urls-and-verify-investigation-criteria.md` | URL 記録 + 検証 task の仕様 | T8 |
| `docs/plans/file-permission-model-redesign-implementation-issues.md` | 本 plan。Task Breakdown の URL 欄を T8 で追記 | 自己参照の記録先 |
| `docs/impl/ir/file-permission-model-redesign-implementation-issues.md` | implementation record。起票 URL・日時・検証結果を記録 | T8 で `new_impl_record.js` により新規作成 |

## Implementation Sequence

- [ ] 1. T1: issue 1(exec-contract cleanup)を起票 — deps なしの起点
- [ ] 2. T2: issue 2(opt-in execDenyPatterns)を起票 — 本文 deps に issue 1 の番号を引用
- [ ] 3. T3: issue 3(統一 exec grant 統合)を起票 — 本文 deps に issue 1 の番号を引用(T2 と独立・並列可)
- [ ] 4. T4: issue 4(route 審査整理 + grant-set + record split)を起票 — deps に issue 1・3 を引用
- [ ] 5. T5: issue 5(substitute-exec mode)を起票 — deps に issue 4 を引用
- [ ] 6. T6: issue 6(trusted flag 発行主体)を起票 — deps に issue 1・4・5 を引用
- [ ] 7. T7: issue 7(文書正直性 + migration guide)を起票 — deps に issue 1-6 を引用
- [ ] 8. T8: 7 issue の URL を本 plan・各 task・IR へ記録、issue #151 へ分解サマリ comment、成果物基準を検証

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `gh` 認証失敗・rate limit で起票が途中停止 | med | med | 各 task は起票前に `gh issue list --search "<title prefix>"` で重複確認し、既存なら作成を skip して URL 記録へ進む(冪等再開) |
| 2 | 先行 issue 番号未確定のまま後続 issue を起票し deps 記述が空番号になる | med | high | task の depends-on が T1→T7 の鎖を固定。T8 の検証で全 deps 記述が実番号であることを確認 |
| 3 | issue body から design 決定(M1-M5・disposition・open 事項)が脱落し、後続実装が旧制限を復活させる | low | high | 各 task に issue body 全文を収録し転記のみ許可。TSPEC-034Kvx6yD4sfTj8KFkFIF3 の Rules で必須節を固定、T8 で照合 |
| 4 | 重複起票で同題 issue が 2 本立つ | low | med | task 手順の重複 check + T8 で `gh issue list` による 7 件丁度の確認 |
| 5 | 破壊的変更の告知漏れ(issue 7 の対象) | low | med | issue 7 本文に破壊的変更一覧(deny list 削除・内容審査除去・grant 形状)を転記済み |

## Task Breakdown

| Task | 対応 GitHub issue | Depends on | Blocks |
|------|-------------------|------------|--------|
| T1 `TASK-034KvxjG6fUOLjtn1cOFTF` | issue 1: exec-contract cleanup — 内容審査 7 項目の mandatory 層除去 | — | T2,T3,T4,T6,T7,T8 |
| T2 `TASK-034KvxvHeO6yOVxflkxTPO` | issue 2: opt-in `security.execDenyPatterns` profile(A9 降格) | T1 | T7,T8 |
| T3 `TASK-034Kvxwfa7svHN4AuX95dA` | issue 3: command.grants / process.tasks 統一 exec grant モデル統合 | T1 | T4,T7,T8 |
| T4 `TASK-034KvxwOy3rcRuUnlZu4Ej` | issue 4: route/hook 審査依存整理 + grant-set 投影 + record split | T1,T3 | T5,T6,T7,T8 |
| T5 `TASK-034KvxFptiaMDpvi8xe9dM` | issue 5: substitute-exec route mode | T4 | T6,T7,T8 |
| T6 `TASK-034KvxFX6oCd4TlEowM1kd` | issue 6: trusted route flag 発行主体の再設計(D4) | T1,T4,T5 | T7,T8 |
| T7 `TASK-034KvxGAbTTWEsA1RbX7cn` | issue 7: 文書正直性 sweep + migration guide | T1-T6 | T8 |
| T8 `TASK-034KvxH8TvZ1HF0bfjydpk` | URL 記録 + issue #151 comment + 成果物基準検証 | T1-T7 | — |

起票した issue の URL 記録欄(T8 が埋める):

| Issue | Title | URL |
|-------|-------|-----|
| 1 | exec-contract cleanup: 内容審査 7 項目を mandatory 層から除去 | (未起票) |
| 2 | opt-in `security.execDenyPatterns` profile(A9 降格) | (未起票) |
| 3 | command.grants / process.tasks の単一 exec grant モデル統合 | (未起票) |
| 4 | route/hook 審査依存整理 + grant-set 投影 + record split | (未起票) |
| 5 | substitute-exec route mode | (未起票) |
| 6 | trusted route flag 発行主体の再設計(D4) | (未起票) |
| 7 | 文書正直性 sweep + breaking-change migration guide | (未起票) |

## Verification

- [ ] T8 が `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計" --state all --json number,title` で 7 件の起票を確認(検索語は task 本文の共通接尾辞に一致)
- [ ] 各 issue body が `## Scope`・`## Acceptance Criteria`・`## Dependencies`・`## 参照` の 4 節を持ち、deps が design §実装 issue 分解と一致する(TSPEC-034Kvx6yD4sfTj8KFkFIF3 R2/R4/R6)
- [ ] issue #151 の成果物基準 5 項目(現状調査表・trust モデル提案・ADR/spec 改訂・実装分割の記録・検証計画)が chain + 起票 issue で満たされることを T8 が確認
- [ ] `node scripts/check-doc-ids.mjs` が pass
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --json --type plan` / `--type task` / `--type test-spec` で新規 error 0

## Open Questions / Gaps

- issue の label/milestone は本 plan では指定しない(repo の label
  体系は未確認 — issue #151 自体も label 無し。必要なら実装
  stage で別途判断)。
- issue 番号は T1-T7 の実行まで不明。各 task 本文は「前段 task が
  記録した issue 番号/URL を deps 記述へ引用する」手順で依存を
  表現する。

## Self-Review

- [x] Every spec requirement maps to a task.  — spec の「実装タスクへの分割と依存関係」要求が T1-T8 に対応。R1-R9 の残りは起票先 issue 群の AC として task 本文へ転記済み。
- [x] No placeholders remain.  — 「(未起票)」は T8 が埋める記録欄であり、作業指示の placeholder ではない。
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(`gh` CLI 呼び出し中心の機械的作業。T2/T3 は T1 完了後に並列化可能だが、量が小さく逐次で十分)
- Independent work streams: T2/T3(T1 後)、それ以外は deps 鎖に従う逐次実行
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: 各 task に収録した issue body 全文が
  DESIGN-034KuDkWOmNXiBddEQ4XWK「§実装 issue 分解」・
  Disposition・M1-M5・handoff 表・refine 一覧と矛盾・脱落
  していないか(特に issue 3 の M1-M5 と issue 7 の正直性
  記述)。
- 前提確認: ADR-034KuqZelggTIb8Z8K9QB8 は accepted 済みで、
  issue 1 の着手前提(ratification)は満たされている。

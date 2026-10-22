---
id: "IMPL-034KvX8YRwdpwNQVlWW230"
type: "impl"
status: "completed"
title: "権限モデル再設計の実装 issue 起票 (issue #151)"
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
      - type: "file"
        path: "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  implemented-by: []
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
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# 権限モデル再設計の実装 issue 起票 (issue #151)

## Summary

調査 issue #151 の「実装」= DESIGN-034KuDkWOmNXiBddEQ4XWK「§実装
issue 分解」で確定した 7 つの実装 issue を `gh` CLI で
`scarecrow173/cheaper` へ起票し、URL を plan・各 task・本 IR へ
記録した。起票日時: 2026-10-23。

| Issue | Title | URL | Depends on |
|-------|-------|-----|------------|
| 1 | 権限モデル再設計 1/7: exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去 | https://github.com/scarecrow173/cheaper/issues/163 | — |
| 2 | 権限モデル再設計 2/7: opt-in security.execDenyPatterns profile (A9 降格) | https://github.com/scarecrow173/cheaper/issues/164 | #163 |
| 3 | 権限モデル再設計 3/7: command.grants / process.tasks の単一 exec grant モデル統合 | https://github.com/scarecrow173/cheaper/issues/165 | #163 |
| 4 | 権限モデル再設計 4/7: route/hook 審査依存整理 + grant-set 投影 + record split | https://github.com/scarecrow173/cheaper/issues/166 | #163, #165 |
| 5 | 権限モデル再設計 5/7: substitute-exec route mode | https://github.com/scarecrow173/cheaper/issues/167 | #166 |
| 6 | 権限モデル再設計 6/7: trusted route flag 発行主体の再設計 (D4) | https://github.com/scarecrow173/cheaper/issues/168 | #163, #166, #167 |
| 7 | 権限モデル再設計 7/7: 文書正直性 sweep + breaking-change migration guide | https://github.com/scarecrow173/cheaper/issues/169 | #163-#168 |

## Context

PLAN-034KvwX4x7oaZlaECqhHc1 の T8(終端 task
TASK-034KvxH8TvZ1HF0bfjydpk)。issue #151 は調査・再設計 issue で
あり、実装成果物は 7 実装 issue の起票と記録(TSPEC-034Kvx6yD4sfTj8KFkFIF3)。

## Implementation

### Changes

- `gh issue create` で T1→T7 の順に 7 issue を起票。各 body は
  対応 task doc の fenced block を転記し、`#<ISSUE-N>`
  placeholder を先行 issue の実番号で置換。
- issue 3(#165)の本文は issue 4 への前方参照を含むため、
  #166 起票後に `gh issue edit 165` で `#166` へ置換済み。
- 各 task doc(T1-T7)の `status` を `done` に更新し、起票結果
  欄へ URL・番号を記録、Done When チェックを完了。
- plan の Implementation Sequence 全項目と Task Breakdown 末尾の
  URL 記録欄を実 URL で更新。
- issue #151 へ分解結果サマリ comment を 1 件投稿(#151 は
  close しない — 親 agent 判断)。

### Decisions During Implementation

- issue 3 の `#<ISSUE-4>` 前方参照は起票時点で番号不明のため、
  placeholder のまま起票 → #166 確定後に body を edit する手順を
  採用(deps 鎖の起票順を維持するため)。
- label/milestone/assignee は plan の Out of scope どおり付与
  していない。

## Related Experiments

known-plan execution のため JSONL experiment は作成していない。
metadata.experiments の adopted/rejected は空。

## Validation

- `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 in:title" --state all`
  でちょうど 7 件(#163-#169)を確認。
- 各 issue body が `## Scope`・`## Acceptance Criteria`・
  `## Dependencies`・`## 参照` の 4 節を持ち、`<ISSUE`
  placeholder を含まないことを `gh issue view` で全 7 件確認。
- deps が design の依存グラフと一致: 1: なし / 2: {163} /
  3: {163} / 4: {163,165} / 5: {166} / 6: {163,166,167} /
  7: {163,164,165,166,167,168}。
- `node scripts/check-doc-ids.mjs` pass。
- `node .agents/skills/doc-status/scripts/audit_docs.js --json
  --type task` / `--type plan` / `--type test-spec` で新規 error 0。

## issue #151 成果物基準の検証(T8 手順 7)

| # | 基準 | 証跡 | 判定 |
|---|------|------|------|
| 1 | 現状調査表 | `docs/discovery/permission-model-responsibility-redesign.md` (DISC-034KuleA9g82Ir4xWfmTYq — 53 項目分類表) | 満たす |
| 2 | trust/permission モデル提案 | spec SPEC-034KuqPkTK2b4mHsykH9A5 + ADR-034KuqZelggTIb8Z8K9QB8(Option B 宣言契約モデル、accepted) | 満たす |
| 3 | ADR/spec 改訂案 | accepted ADR + approved spec/design + design「§Supersede / Refine 一覧」で refine 対象確定 | 満たす |
| 4 | 実装分割と依存関係の記録 | 起票済み 7 issue(#163-#169)+ deps(上記 Validation の依存グラフと一致) | 満たす |
| 5 | 検証計画 | 各 issue の AC + design「§Verification Notes」+ TSPEC-034Kvx6yD4sfTj8KFkFIF3 | 満たす |

## Risks

- Option E(exec-less route)の採否は design で open のまま —
  issue 5(#167)本文に open 事項として記録済み。採否判断は
  後続 chain の責務。
- issue #151 は close していない — 本 IR 完了後の親 agent /
  reviewer 判断に委ねる。

## Follow-ups

- 7 実装 issue(#163-#169)の実装本体は各 issue の AC に従う
  後続 chain の仕事。
- issue #151 の close 判断(成果物基準 5 項目は上記のとおり
  満たされていることを確認済み)。

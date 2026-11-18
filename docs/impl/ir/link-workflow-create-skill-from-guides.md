---
id: "IMPL-034ZulA2jDGtiiAEJW5oC4"
type: "impl"
status: "completed"
title: "Link workflow create skill from guides"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - {"type":"doc","path":"docs/guides/workflow-usage.md"}
      - {"type":"doc","path":"docs/guides/parent-agent-integration.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/link-workflow-create-skill-from-guides.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Link workflow create skill from guides

## Summary

`docs/guides/workflow-usage.md` の catalog 節に
`cheaper-workflow-create` skill への導線を追加し、
`docs/guides/parent-agent-integration.md` の skills 一覧を 6 件へ
更新して同 skill の役割と承認境界(定義保存と実行の分離、
grant/policy/Route 非改変)を記載した。

## Context

- task: `../../tasks/link-workflow-create-skill-from-guides.md`
- plan: `../../plans/implement-workflow-create-skill.md`
- design: `../../designs/workflow-create-skill.md`
- issue: #204(Epic #200 の子)

## Implementation

### Changes

- `docs/guides/workflow-usage.md`: catalog の登録・更新節に create
  skill が Workflow JSON v1 の設計・`workflow interface` 検証・
  catalog 登録・`workflow list` 確認を担う旨と、workflow 実行・
  grant/policy/Route 変更を行わない境界を追記。
- `docs/guides/parent-agent-integration.md`: skills 一覧を 5 件→
  6 件に更新し、`cheaper-workflow-create` を追加。skill の
  説明に定義保存と実行の分離・権限設定の非改変を含めた。

### Decisions During Implementation

- 導線は最小限の追記に留め、登録手順の正本は引き続き
  `workflow-usage.md` の catalog 節とした(skill 側は同契約を
  参照する形で重複記述を避けた)。
- 日本語のみの guide のため localized 同期の追加作業は発生しない。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `doc_status audit --type all --blocking --json`: blocking 0。
- `git diff --check`: clean。
- `mise run check`: exit 0(変更は docs/skill markdown のみで
  Rust 挙動に影響しない)。

## Risks

- skills 一覧の件数表記が将来の skill 追加時に再更新を要する点は
  従来どおり。

## Follow-ups

- `cheaper-workflow-discover`(#203) 実装時に discover→create 間の
  導線を guide へ追加するか検討する。

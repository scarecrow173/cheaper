---
id: "TASK-034DkRYMmhq7VdzfD1NcT8"
type: "task"
status: "done"
title: "Rewrite cheaper-usage SKILL.md hook path section"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
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
    - "docs/plans/cheaper-usage-skill-hook-path-rewrite.md"
  implemented-by:
    - "docs/impl/ir/cheaper-usage-skill-md-hook-path-section-rewrite.md"
  depends-on:
    - "docs/plans/cheaper-usage-skill-hook-path-rewrite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/cheaper-usage-skill-hook-path-rewrite.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Rewrite cheaper-usage SKILL.md hook path section

## Work

`cheaper-usage` SKILL.md の "Locating the CLI" 節内 path 例示
ブロック（:44-73）を design item 1-8 の構成で書換え、APM の
`<hooks_root>/<dep-name>/` deploy 規則に一致させる（issue #87）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cheaper-usage-skill-hook-path-rewrite.md`
- `docs/designs/cheaper-usage-skill-hook-path-section-rewrite.md`
- `docs/specs/cheaper-usage-skill-hook-path-examples.md`
- `docs/adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md`

## Dependencies

- なし（単一 task）

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（:44-73 のみ。
  節内 bullets :74-100・他 section・front matter は変更しない）

## Done When

- [x] PATH 優先・導入文 target 非依存化・"standalone executables"
  訂正・`<hooks_root>/<dep-name>/` 規則・canonical 実例
  （`hooks/apm/` + copilot `scripts/apm/`）・5 target
  hooks_root 表・discovery 手順（target 依存 config 名 +
  claude `${CLAUDE_PROJECT_DIR}` 注記）・launcher 段落接続・
  "either form" 橋渡しの全てが本文に反映される
- [x] install 名読み替え注記（旧 :71-72）が削除される
- [x] `grep -rn "hooks/cheaper" packages/apm/.apm/` で path
  例示が残らない

## Verification

- [x] `grep -rn "hooks/cheaper" packages/apm/.apm/` が 0 件
- [x] temp consumer `apm install`（APM 0.28.0、path dep
  basename `apm`）で `.windsurf/hooks/apm/cheaper` 実在を確認
- [x] `mise run check` pass

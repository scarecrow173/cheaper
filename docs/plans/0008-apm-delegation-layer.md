---
id: "PLAN-0008"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Implement APM delegation skills and pre_read_code hook"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
  implemented-by: []
  depends-on:
    - "../plans/0007-canonical-token-counter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM Delegation Layer Implementation Plan

## Goal

`.apm/` producer layout で 4 skill と `pre_read_code` hook を実装し、
Windsurf parent agent が large file を read する前に bulk-reader へ
redirect できる状態にする。

## Upstream Documents

- SPEC-0008 / DESIGN-0013 / DISC-0005

## Design Inputs

- hook deploy 形式は DISC-0005 の実測どおり `./` 相対 script 参照
- skill は `name`/`description` frontmatter + 英語本文
- threshold は env 上書き可能な provisional default 8192

## File Map

| File | Change |
|---|---|
| `.apm/skills/cheaper-usage/SKILL.md` | 新規 |
| `.apm/skills/cheaper-bulk-read/SKILL.md` | 新規 |
| `.apm/skills/cheaper-code-write/SKILL.md` | 新規 |
| `.apm/skills/cheaper-doc-write/SKILL.md` | 新規 |
| `.apm/hooks/hooks.json` | 新規 |
| `.apm/hooks/pre-read-code.js` | 新規 |
| `packages/cli/test/e2e/pre-read-hook.test.ts` | 新規 |
| `docs/{discovery,specs,designs,plans,tasks}/README.md` | index 更新 |
| `README.md` | APM skill/hook の存在を追記 |

## Implementation Sequence

1. TASK-0070: cheaper-usage skill
2. TASK-0071: 3 worker delegation skills
3. TASK-0072: hooks.json + pre-read-code.js + e2e test
4. TASK-0073: index/docs 更新・baseline 検証

## Task Breakdown

- TASK-0070 create cheaper-usage skill
- TASK-0071 create worker delegation skills (bulk-read/code-write/doc-write)
- TASK-0072 implement pre_read_code hook and e2e test
- TASK-0073 update indexes and verify delegation layer

## Verification

- `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck`
- hook e2e: large→exit2+stderr / small→0 / missing→0 / dir→0 / bad stdin→0
- doc audits clean

## Self-Review

- [x] File paths, relations, status values, and terminology are consistent.
- [x] Each task is independently verifiable.

## Implementation Handoff

skill は英語・実際の CLI contract に忠実に書く。hook script は fail-open
原則を厳守する。

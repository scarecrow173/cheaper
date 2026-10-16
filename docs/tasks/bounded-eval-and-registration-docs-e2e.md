---
id: "TASK-034H3Q05R2fMnFMJwYEuLl"
type: "task"
status: "done"
title: "Bounded eval and registration docs/e2e follow-through"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/cli-owned-hook-config-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Bounded eval and registration docs/e2e follow-through

## Work

先行 spec/ADR/design/task・`cheaper-bulk-read`/`cheaper-usage` skill・
`distribution.md` を新 contract と矛盾しない記述へ更新し、E2E
シナリオ（lifecycle trust・untrusted・`APM_NO_SCRIPTS`・reinstall・
update・uninstall・mixed consumer・POSIX/Windows）の Node test を
拡張する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md
- docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md

## Dependencies

- `cli-owned-hook-config-lifecycle`（docs が CLI 実挙動を参照）。

## Affected Files

- `docs/guides/distribution.md`
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`・
  `cheaper-bulk-read/SKILL.md`
- 先行 docs（`case-insensitive-…` spec・`devin-native-hooks-…`
  design・旧 task の記述）
- `scripts/test/devin-hooks-install.test.mjs`・
  `scripts/test/apm-install-targets.test.mjs`

## Done When

- [ ] 先行 docs が「whole-file 計測」「Windows-only」「skill のみで
  導入可能」等の stale 記述を含まない。
- [ ] distribution guide が lifecycle trust・fail-soft・per-dep
  `targets:` 移行（`agent-skills` 必須条件含む）・`read_config_from`
  無効化・verify 手順・POSIX 直接実行を説明する。
- [ ] E2E test が新シナリオを列挙し、mock-only と real-CLI coverage
  が区別される。
- [ ] `mise run check` が全緑で、doc-id/audit に新規警告なし。

## Verification

- [ ] `mise run check` が pass。
- [ ] `node scripts/check-doc-ids.mjs` が pass。
- [ ] `audit_docs.js --type all` で新規 warning なし。

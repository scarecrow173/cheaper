---
id: "PLAN-000004taRcf5XDevo8jYJz"
type: "plan"
status: "completed"
title: "distributed skill / parent-agent guide sync — implementation plan"
test-spec-skip: "Documentation-only change; verified by contract checks against implementation output and doc audit, not executable tests."
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
    - "../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
  implemented-by:
    - "../tasks/sync-cheaper-usage-skill-md-with-current-cli-and-hook-surface.md"
    - "../tasks/correct-fail-open-fail-closed-contract-in-parent-agent-guide.md"
  depends-on:
    - "../adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# distributed skill / parent-agent guide sync — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | modify | auth command surface・hook target matrix・credential 配置を current CLI/実装に同期 |
| `docs/guides/parent-agent-integration.md` | modify | fail-open/fail-closed contract を hook 実装の非対称仕様に同期 |
| `docs/guides/usage.md` | check | CLI surface 記述との矛盾確認 |
| `docs/guides/providers.md` | check | canonical guide との矛盾確認（変更しない） |
| `README.md` | check | hook/auth への言及との矛盾確認 |
| `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` | check | FILE_TOO_LARGE / delegation 記述との矛盾確認 |
| `packages/apm/.apm/hooks/pre-read-code.js` | check | fail-open/closed 分岐の一次情報源（変更しない） |
| `packages/apm/.apm/hooks/pre-read-code-copilot.js` | check | Copilot adapter 分岐の一次情報源（変更しない） |
| `packages/cli/test/e2e/apm-install-targets.test.ts` 他 | check | target matrix の一次情報源（変更しない） |

## Tasks

- **TASK-06WDQr0NHYErusR6Cmye43**: `cheaper-usage` SKILL.md の
  command surface・hook matrix・credential 記述を同期
- **TASK-002DRbiVRuFJ5XWHvsWW6i**: `parent-agent-integration.md`
  の fail-open/fail-closed contract 修正 + 整合確認・doc audit・
  最終検証

## Dependency Graph

```
TASK-06WDQr0NHYErusR6Cmye43 (SKILL.md sync)
   └─> TASK-002DRbiVRuFJ5XWHvsWW6i (guide fix + consistency + verification)
```

## Risk Register

- skill の command 表が実 `--help` output と微妙にずれる:
  実 output をそのまま一次情報源として転記する。
- `FILE_TOO_LARGE` fail-closed の説明が閾値超過 block と混同
  される: 「計測不能だが確実に巨大と authoritative に判定」
  という別 verdict として明記する。
- Copilot JSON asset gap の記述をうっかり削除する: diff review
  で維持を確認する（acceptance 明記）。

## Rollback

- docs のみの変更のため file revert で完全復帰。code 非変更。

## Verification Matrix

| Requirement | Verification |
|---|---|
| command surface = current CLI | `node packages/cli/bin/cheaper.js --help` output と skill 表を突合 |
| auth 系 + options が skill から確認できる | `rg "auth (status|login|logout)|--method|--type" SKILL.md` |
| credential project-local 誤解なし | `rg -i "project" SKILL.md` で credential 文脈を確認 |
| hook matrix 正確 | guide/skill の target 記述と hook script・E2E を突合 |
| FILE_TOO_LARGE fail-closed 区別 | guide 記述と hook script 分岐を突合 |
| Copilot JSON gap 維持 | guide 内の該当段落が残っていることを確認 |
| providers.md と矛盾なし | credential/auth 記述の突合 |
| 回帰 | `mise run check` + `audit_docs.js --type all` no findings |

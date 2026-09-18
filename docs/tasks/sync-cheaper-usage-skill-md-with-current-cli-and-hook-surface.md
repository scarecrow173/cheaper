---
id: "TASK-06WDQr0NHYErusR6Cmye43"
type: "task"
status: "done"
title: "Sync cheaper-usage SKILL.md with current CLI and hook surface"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md"
  implemented-by: []
  depends-on:
    - "docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md"
  blocks:
    - "docs/tasks/correct-fail-open-fail-closed-contract-in-parent-agent-guide.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synchronize-distributed-skill-and-parent-agent-guide.md"
    - "docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-06WDQr0NHYErusR6Cmye43: Sync cheaper-usage SKILL.md with current CLI and hook surface

## Work

配布される `cheaper-usage` skill
（`packages/apm/.apm/skills/cheaper-usage/SKILL.md`）を current
CLI surface・hook target matrix・credential 配置に同期する
（Issue #34 / SPEC-003hd7W00xCikuCXnwHA4C）。

### 1. auth command surface の追加

Commands ブロックへ `cheaper --help` の実 output に基づく auth 系
3 command を追加する:

- `auth status [provider]` — Show stored credentials and provider
  readiness
- `auth login <provider>` — Sign in to a provider
- `auth logout <provider>` — Remove a provider credential

option 表へ以下を追加する:

- `--method <id>` — Auto-answer auth login select prompts
- `--type <oauth|api_key>` — Auth method for `auth login`
  (default: `oauth`)

### 2. hook target matrix の修正

「The pre-read hook currently functions on Windsurf only」を現行
contract に修正する:

- Windsurf: native `pre_read_code` event で functional
- GitHub Copilot: `preToolUse` adapter
  （`pre-read-code-copilot.js`）で functional。deny は
  `{"permissionDecision":"deny"}` + exit 0
- Claude / Codex / Cursor: hook 登録は deploy されるが no-op
  （functional hook ではない）

canonical matrix（DISC-0009 系
`cross-agent-hook-distribution-and-contract-matrix`）への参照は
維持する。

### 3. credential 配置

#33 で最小修正した箇所を全面同期する。credential は user-scope の
`<userConfigDir>/pi/auth.json`（`cheaper auth` で管理）に保存
され、project-local `.cheaper/` には置かないことを明示する。
provider setup の詳細は canonical guide
（`docs/guides/providers.md`）への参照とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md`
- `docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md`
- `docs/adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md`
- `docs/designs/synchronize-distributed-skill-and-parent-agent-guide.md`

## Dependencies

- なし（plan の先頭 task）

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（modify）

## Done When

- [x] command 表が `cheaper --help` 実 output と一致し auth 系
  3 command が含まれる
- [x] `--method` / `--type <oauth|api_key>` が option 表に含まれる
- [x] hook matrix が Windsurf + Copilot functional・Claude /
  Codex / Cursor no-op と一致する
- [x] credential が user-scope `<userConfigDir>/pi/auth.json` と
  記述され project-local 誤解がない

## Verification

- [x] `node packages/cli/bin/cheaper.js --help` の実 output と
  command/option 表を突合
- [x] `rg "auth status|auth login|auth logout|--method|--type"
  packages/apm/.apm/skills/cheaper-usage/SKILL.md`
- [x] `rg -i "windsurf|copilot|claude|codex|cursor"
  packages/apm/.apm/skills/cheaper-usage/SKILL.md` で matrix 記述
  を確認
- [x] `rg -i "pi/auth.json|userConfigDir|project"
  packages/apm/.apm/skills/cheaper-usage/SKILL.md` で credential
  記述を確認

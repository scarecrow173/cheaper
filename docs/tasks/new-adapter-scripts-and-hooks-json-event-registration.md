---
id: "TASK-034rAcF1RPJJogj2vzFDrF"
type: "task"
status: "todo"
title: "New adapter scripts and hooks.json event registration"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
    - "docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md"
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
    - "docs/test-specs/cross-agent-adapter-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# New adapter scripts and hooks.json event registration

## Work

DESIGN-034rA79Uog7JOa7evw6rOL の adapter 仕様どおりに実装する:

- `pre-run-command.js`（windsurf shell、`pre_run_command` /
  `tool_info.command_line`、deny は stderr + exit 2）
- `pre-read-code-pretooluse.js`（claude + codex 共用、`tool_name`/
  `tool_input`、Read → file-read、Bash/PowerShell → shell、deny は
  `hookSpecificOutput.permissionDecision:"deny"` JSON + exit 0）
- `pre-read-code-cursor.js`（`beforeReadFile` / `beforeShellExecution`、
  deny は `{permission:"deny"}` JSON + exit 0、全経路 schema 適合応答）
- `pre-read-code-copilot.js` 拡張（`bash`/`powershell`/`shell` toolName
  → shell request、foreign shape payload → dual allow JSON）
- `hooks.json` 更新（`pre_run_command` / nested `PreToolUse` group /
  `beforeReadFile` / `beforeShellExecution` / `preToolUse` matcher
  `view|bash|powershell|shell`）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cross-agent-native-routing-adapters-implementation.md`
- `docs/designs/cross-agent-native-routing-adapter-architecture.md`

## Dependencies

- `docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md`
  （routing-policy.js が前提）

## Affected Files

- `packages/apm/.apm/hooks/pre-run-command.js`（新規）
- `packages/apm/.apm/hooks/pre-read-code-pretooluse.js`（新規）
- `packages/apm/.apm/hooks/pre-read-code-cursor.js`（新規）
- `packages/apm/.apm/hooks/pre-read-code-copilot.js`
- `packages/apm/.apm/hooks/hooks.json`

## Done When

- [ ] 4 adapter が canonical request を policy へ渡し、verdict を
      target native response へ変換する
- [ ] `hooks.json` に 6 event 登録があり、APM verbatim pass-through
      で generated config に伝搬する
- [ ] adapter に判定ロジック（threshold / count-tokens）の複製がない

## Verification

- [ ] 各 adapter の stdin 注入 contract test が pass（次 task で追加）

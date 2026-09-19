---
id: "TASK-034rAcFy8if4UmbPEMTdg1"
type: "task"
status: "done"
title: "Adapter contract tests and APM install E2E assertions"
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
    - "docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/test-specs/cross-agent-adapter-contract-tests.md"
  defers: []
  deferred-by: []
---

# Adapter contract tests and APM install E2E assertions

## Work

TSPEC-034rAaJtM27Ibpy78mq0kP の Rules/Examples を stdin 注入 test で
実装する。`makeIsolatedEnv` + `spawnSync` の既存流儀を使う。

- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`（新規）:
  pretooluse adapter（Read deny / Bash allow seam / unrelated tool /
  malformed / missing field / FILE_TOO_LARGE）、cursor adapter
  （file_path deny / command allow / malformed → allow JSON）、windsurf
  `pre-run-command.js`（shell allow seam / malformed → exit 0）。
- `copilot-pre-read-hook.test.ts` 拡張: `toolName:"bash"` shell
  mapping（exit 0 silent）、foreign shape → dual allow JSON、
  never-nonzero 維持。
- `apm-install-targets.test.ts` 拡張: 各 target の generated config
  に native event と adapter command が含まれる assert（claude
  `PreToolUse` nested group、cursor `beforeReadFile` /
  `beforeShellExecution`、copilot `preToolUse`、codex `PreToolUse`
  nested group、windsurf `pre_read_code`/`pre_run_command`）と
  adapter script の deploy assert。既存 `.json` drift assert は維持。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cross-agent-native-routing-adapters-implementation.md`
- `docs/test-specs/cross-agent-adapter-contract-tests.md`

## Dependencies

- `docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md`

## Affected Files

- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`（新規）
- `packages/cli/test/e2e/copilot-pre-read-hook.test.ts`
- `packages/cli/test/e2e/apm-install-targets.test.ts`

## Done When

- [ ] 全 adapter の normalize / deny / fail-open / seam が test で
      pin される
- [ ] per-target install E2E が native event + adapter deploy を
      assert する
- [ ] copilot never-nonzero と cursor schema-適合応答が test で
      保証される

## Verification

- [ ] `pnpm --filter @cheaper/cli test` が全件 pass

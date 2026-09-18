---
id: "TASK-0112"
type: "task"
status: "done"
title: "Create usage/configuration guides and CONTRIBUTING.md"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0023-user-facing-docs-information-architecture.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0023-user-facing-docs-information-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0028-user-facing-docs-information-architecture.md"
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0112: Create usage/configuration guides and CONTRIBUTING.md

## Work

DESIGN-0028 の content responsibility map に従い、3 文書を実コード
突合で作成する。

### 1. `docs/guides/usage.md`

- commands: `run <worker>`・`bulk-read`・`code-write`・
  `doc-write`・`count-tokens <path>`・`auth status|login|logout`
- options: `--input`・`--worker-file`・`--cwd`・`--timeout`・
  `--method`・`--type`・`--format`・`--quiet`
- stdin: worker input schema 準拠の JSON / stdout: envelope 1 行
  （ok/result/meta or error）+ exit code 概観
- 例: bulk-read の stdin→stdout の最小例
- usage.jsonl: run・count-tokens が `<userConfigDir>/usage.jsonl`
  に計測 record を残す旨（ADR-0025 参照・2-3 行）

### 2. `docs/guides/configuration.md`

- config discovery: user scope（`CHEAPER_CONFIG_DIR` →
  `XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper`）・project
  scope（`<cwd>/.cheaper/`）・precedence（project > user）
- `models.yaml`（version・overrides・preferences）・`policy.yaml`
  （execution/tools/filesystem/model/output/retry の役割概観、
  built-in 値は `packages/cli/policy.yaml` 参照として実値を記載）
- env vars: `CHEAPER_CONFIG_DIR`・`CHEAPER_COUNT_MAX_BYTES`・
  `CHEAPER_READ_TOKEN_THRESHOLD`・`PI_CODING_AGENT_DIR`・
  `HOME/USERPROFILE`（config dir 解決への影響）
- provider credential 自体は providers.md へ誘導

### 3. `CONTRIBUTING.md`

- 目的: 人間向け contributor entrypoint
- setup: `mise run setup` / `mise run check`（ADR-0020 の canonical
  contract、fresh worktree での trust 注意）
- test: `pnpm test`・e2e の概観・実 provider 検証（Ollama
  container 手順 + real-provider-ollama.test.ts の auto-skip —
  providers.md から移す smoke 手順の受け皿）
- repository 構造: `packages/{cli,core,runtime-pi}`・
  `packages/apm`（distributable）・root `apm.yml`（dev-only）
- architecture 記録: `docs/README.md` への導線
- Coding Agent 利用時は `AGENTS.md` を参照

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0023-user-facing-docs-information-architecture.md`
- `docs/designs/0028-user-facing-docs-information-architecture.md`

## Dependencies

- なし（PLAN-0023 の先頭 task）

## Affected Files

- `docs/guides/usage.md`（add）
- `docs/guides/configuration.md`（add）
- `CONTRIBUTING.md`（add）

## Done When

- [ ] 3 文書が存在し、記載 command・path・env var・値が実装と
      一致する
- [ ] CONTRIBUTING.md が AGENTS.md の複製でなく entrypoint として
      機能する

## Verification

- [ ] 記載コマンド・option が `args.ts`/HELP と一致
- [ ] config path・env var が `dirs.ts`・`cli.ts` と一致
- [ ] policy 実値が `packages/cli/policy.yaml` と一致

## Verification results

- `docs/guides/usage.md`・`docs/guides/configuration.md`・
  `CONTRIBUTING.md` を作成。
- 記載 command（run/bulk-read/code-write/doc-write/count-tokens/
  auth status|login|logout）・option（--input/--worker-file/--cwd/
  --timeout/--method/--type/--format/--quiet/--help/--version）は
  `args.ts`・HELP（`cli.ts`）と一致。
- config dir 解決（CHEAPER_CONFIG_DIR → XDG_CONFIG_HOME →
  ~/.config/cheaper、project `.cheaper/`）・policy 実値
  （timeout 120s/maxTurns 8/maxToolCalls 64/tools 7 種/
  symlinks within-workspace/maxTier medium/tierFallback none/
  maxTokens 2000/repairAttempts 1/transientAttempts 1）は
  `dirs.ts`・`packages/cli/policy.yaml` と一致。
- env vars（CHEAPER_CONFIG_DIR・CHEAPER_COUNT_MAX_BYTES・
  CHEAPER_READ_TOKEN_THRESHOLD・PI_CODING_AGENT_DIR）は
  `cli.ts`・`dirs.ts`・`pre-read-code.js` と一致。

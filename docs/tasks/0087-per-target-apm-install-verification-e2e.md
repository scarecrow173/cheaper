---
id: "TASK-0087"
type: "task"
status: "todo"
title: "Per-target apm install verification e2e"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/0013-cross-agent-distribution-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0013-cross-agent-distribution-verification.md"
    - "docs/tasks/0086-parameterize-apm-e2e-fixture-by-target.md"
  blocks:
    - "docs/tasks/0088-finalize-contract-matrix-and-consumer-pointer.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0018-cross-agent-distribution-verification.md"
    - "docs/specs/0013-cross-agent-distribution-verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0013-cross-agent-distribution-verification.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Per-target apm install verification e2e

## Work

新規 `packages/cli/test/e2e/apm-install-targets.test.ts` を作成し、
non-windsurf 5 target（`copilot`, `codex`, `claude`, `cursor`,
`agent-skills`）を table-driven で回す。windsurf は既存
`apm-install.test.ts` が網羅済みのため対象外（DESIGN-0018 §2）。

既存パターン踏襲: `beforeEach` で `apmAvailable()` 判定 +
`mkdtempSync` + `makeConsumerLayout(root, target)`、各 `it` 先頭で
`apmAvailable()` なければ `t.skip("apm CLI not available on PATH")`、
`afterEach` で `rmSync`。

### hook 対象 4 target 共通 assertion（DESIGN-0018 §3）

- `apmInstall(consumer, root, target)` の `status === 0`
- bundle layout が存在: `<bundleRoot>/pre-read-code.js`、
  `cli/bin/cheaper.mjs`、`cli/version.json`、`cli/workers/`、
  `cli/policy.yaml`（`cli/package.json` は APM が deploy 対象外にする
  ため sidecar は `version.json` — DESIGN-0018 §3 注記）
- `generatedConfigPaths(consumer, target)` の各 path が存在し
  `JSON.parse` 可能
- config 内の登録 event key が **`pre_read_code` verbatim** である
  ことを assert — deploy の証明であると同時に「この event は
  non-windsurf では発火しない = documented no-op」を test 上の記録
  として固定する（ADR-0016）
- bundled CLI が consumer workspace で動く:
  `node <cli> --version` → exit 0 + `/^\d+\.\d+\.\d+$/`、
  `node <cli> count-tokens <file> --cwd <consumer>` → exit 0
  （`process.execPath` 経由で spawn）

### target 固有 assertion

- `cursor` / `copilot`: config の `version === 1`
- `claude`: `.claude/apm-hooks.json` と `.claude/settings.json` の
  双方が存在し、settings.json が nested `hooks → event → matcher →
  hooks` shape で `pre_read_code` を verbatim 含む
  （`${CLAUDE_PROJECT_DIR}` 補間はそのまま残る）
- `copilot`: `.github/hooks/` 配下の `*-hooks.json` は file 名が dep
  名由来のため hardcode せず `readdirSync` で一意に解決する
- `agent-skills`: `.agents/skills/` 配下に 4 skill
  （`cheaper-usage` / `cheaper-bulk-read` / `cheaper-code-write` /
  `cheaper-doc-write`）の `SKILL.md` が deploy され、hook bundle
  dir と generated config file が**存在しない**ことを assert
  （skills-only deploy が正しい挙動。不存在 assert は config file と
  bundle dir に限定し、空 dir 生成等の apm 挙動変化には結合しない）

config assertion は「存在 + JSON parse + event key」レベルに留め、
apm の生成 schema 内部へ過剰結合しない（DESIGN-0018 non-functional
constraints）。新規 code からは必ず明示 target を渡すこと（fixture
default は既存 call site 互換用）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0013-cross-agent-distribution-verification.md`
- `docs/designs/0018-cross-agent-distribution-verification.md`
- `docs/specs/0013-cross-agent-distribution-verification.md`

## Dependencies

- PLAN-0013、TASK-0086（fixture の target パラメータ化 API）

## Affected Files

- `packages/cli/test/e2e/apm-install-targets.test.ts`（新規）

変更しない file（明示）: `apm-fixture.ts` 以外の既存 e2e file すべて。

## Done When

- [ ] 5 target の `it` が個別に記録され、hook 対象 4 target は共通 +
      固有 assertion を全て満たす
- [ ] `agent-skills` は skills deploy あり・hook 不存在の assert が
      入る
- [ ] 既存 e2e file が無変更のまま pass / skip する

## Verification

- [ ] `mise exec -- pnpm -F cheaper test -- apm-install-targets` が
      exit 0（apm 有環境では 5 target pass、不在では skip）
- [ ] `mise exec -- pnpm -F cheaper test` が exit 0（`apm-install` /
      `pre-read-hook` / `delegation-loop` 含む全 suite green — windsurf
      regression なし）

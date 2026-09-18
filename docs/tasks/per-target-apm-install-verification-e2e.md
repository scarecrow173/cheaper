---
id: "TASK-034r05iHQeQpDqhxabCiHf"
type: "task"
status: "done"
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
    - "docs/plans/cross-agent-distribution-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cross-agent-distribution-verification.md"
    - "docs/tasks/parameterize-apm-e2e-fixture-by-target.md"
  blocks:
    - "docs/tasks/finalize-contract-matrix-and-consumer-pointer.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/cross-agent-distribution-verification.md"
    - "docs/specs/cross-agent-distribution-verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/cross-agent-distribution-verification.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Per-target apm install verification e2e

## Work

新規 `packages/cli/test/e2e/apm-install-targets.test.ts` を作成し、
6 target（`windsurf`, `copilot`, `codex`, `claude`, `cursor`,
`agent-skills`）を table-driven で回す。windsurf の hook runtime
挙動は既存 `apm-install.test.ts` が網羅済みだが、generated
config の完全性（`apm-hooks.json` を含む）は本 file の windsurf
case でカバーする（DESIGN-034r05iHy2gdnoLTC43RP5 §2）。

既存パターン踏襲: `beforeEach` で `apmAvailable()` 判定 +
`mkdtempSync` + `makeConsumerLayout(root, target)`、各 `it` 先頭で
`apmAvailable()` なければ `t.skip("apm CLI not available on PATH")`、
`afterEach` で `rmSync`。

### hook 対象 5 target 共通 assertion（DESIGN-034r05iHy2gdnoLTC43RP5 §3）

- `apmInstall(consumer, root, target)` の `status === 0`
- bundle layout が存在: `<bundleRoot>/pre-read-code.js`、
  `cli/bin/cheaper.mjs`、`cli/workers/`、`cli/policy.yaml`
  （`cli/version.json` は **non-copilot のみ** — copilot は
  `.json` 欠落で存在しない。`cli/package.json` は APM が deploy
  対象外にするため sidecar は `version.json` — DESIGN-034r05iHy2gdnoLTC43RP5 §3
  注記）
- `generatedConfigPaths(consumer, target)` の各 path が存在し
  `JSON.parse` 可能
- config 内の登録 event key が **`pre_read_code` verbatim** である
  ことを assert — deploy の証明であると同時に expected no-op 分類の
  根拠となる deploy evidence（event 名が native contract と不一致）
  を test 上の記録として固定する。no-op 自体は documented
  contract からの推論で runtime loader は実機未検証（ADR-034r05iHrZUE6W9RDF0V73）
- bundled CLI が consumer workspace で動く:
  `node <cli> count-tokens <file> --cwd <consumer>` → exit 0
  （`process.execPath` 経由で spawn）。`--version` は
  **non-copilot のみ** exit 0 + `/^\d+\.\d+\.\d+$/` を assert
  — copilot は `version.json` 欠落により exit 70
  （"failed to resolve package version"）が期待値

### target 固有 assertion

- `cursor` / `copilot`: config の `version === 1`
- `claude`: `.claude/apm-hooks.json` と `.claude/settings.json` の
  双方が存在し、settings.json が nested `hooks → event → matcher →
  hooks` shape で `pre_read_code` を verbatim 含む
  （`${CLAUDE_PROJECT_DIR}` 補間はそのまま残る）
- `windsurf` / `codex` / `cursor`: `hooks.json`（`hooks` 直下に
  event を nest）に加え、同じ `pre_read_code` 登録を top-level
  event key shape で持つ `apm-hooks.json` が生成されることを
  assert（apm 0.28.0 実測）
- `copilot`: `.github/hooks/` 配下の `*-hooks.json` は file 名が dep
  名由来のため hardcode せず `readdirSync` で一意に解決する
- `copilot` の `.json` 欠落（現状の観測挙動の記録）: APM 0.28.0 は
  copilot の scripts deploy から全 `.json` を除外するため
  `cli/version.json`・`cli/package.json`・
  `cli/workers/schemas/*.json` が存在しないことを assert。
  併せて `--version` → exit 70（"failed to resolve package
  version"）、file-schema worker（bulk-read）→
  `OUTPUT_SCHEMA_NOT_FOUND`（exit 3）を expected failure として
  assert する（`count-tokens` は exit 0）
- `agent-skills`: `.agents/skills/` 配下に 4 skill
  （`cheaper-usage` / `cheaper-bulk-read` / `cheaper-code-write` /
  `cheaper-doc-write`）の `SKILL.md` が deploy され、hook bundle
  dir と generated config file が**存在しない**ことを assert
  （skills-only deploy が正しい挙動。不存在 assert は config file と
  bundle dir に限定し、空 dir 生成等の apm 挙動変化には結合しない）

config assertion は「存在 + JSON parse + event key」レベルに留め、
apm の生成 schema 内部へ過剰結合しない（DESIGN-034r05iHy2gdnoLTC43RP5 non-functional
constraints）。新規 code からは必ず明示 target を渡すこと（fixture
default は既存 call site 互換用）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cross-agent-distribution-verification.md`
- `docs/designs/cross-agent-distribution-verification.md`
- `docs/specs/cross-agent-distribution-verification.md`

## Dependencies

- PLAN-034r05iHE1Fbkp2TZ2Z666、TASK-034r05iHQeQpDqgk8XoZ8E（fixture の target パラメータ化 API）

## Affected Files

- `packages/cli/test/e2e/apm-install-targets.test.ts`（新規）

変更しない file（明示）: `apm-fixture.ts` 以外の既存 e2e file すべて。

## Done When

- [ ] 6 target の `it` が個別に記録され、hook 対象 5 target は共通 +
      固有 assertion を全て満たす（copilot は `--version` exit 70 /
      worker exit 3 の expected failure を含む）
- [ ] `agent-skills` は skills deploy あり・hook 不存在の assert が
      入る
- [ ] 既存 e2e file が無変更のまま pass / skip する

## Verification

- [ ] `mise exec -- pnpm -F cheaper test -- apm-install-targets` が
      exit 0（apm 有環境では 6 target pass、不在では skip）
- [ ] `mise exec -- pnpm -F cheaper test` が exit 0（`apm-install` /
      `pre-read-hook` / `delegation-loop` 含む全 suite green — windsurf
      regression なし）

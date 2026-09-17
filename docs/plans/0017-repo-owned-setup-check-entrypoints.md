---
id: "PLAN-0017"
type: "plan"
status: "approved"
test-spec-skip: "Verified by clean-state worktree verification, CI parity, and existing test suite; no dedicated test-spec doc authored."
title: "Repository-owned setup/check entrypoints (mise run setup / mise run check)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0022-repo-owned-setup-check-entrypoints.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0022-repo-owned-setup-check-entrypoints.md"
    - "../specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md"
  implemented-by:
    - "../tasks/0099-define-mise-setup-check-tasks-and-lockfile-setting.md"
    - "../tasks/0100-align-ci-with-canonical-entrypoints.md"
    - "../tasks/0101-update-docs-and-verify-clean-state-setup-check.md"
  depends-on:
    - "../plans/0016-dev-apm-and-distributable-package-separation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md"
    - "../adr/0020-canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0022-repo-owned-setup-check-entrypoints.md"
  derived-by: []
  verifies:
    - "../specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-0017: Repository-owned setup/check entrypoints

## Goal

SPEC-0017 / ADR-0020 / DESIGN-0022 に従い、`mise run setup` と
`mise run check` を canonical contract として `mise.toml` に定義し、
CI・README・AGENTS.md を同じ contract に揃える。fresh checkout
（git worktree）で setup → check が成功することを実測する。

## Scope

### In Scope

- `mise.toml`: `lockfile = true` setting、`[tasks.setup]`、
  `[tasks.check]` の追加。
- `.github/workflows/ci.yml`: `install_args: "--locked"` と
  canonical task 呼び出しへの集約。
- `README.md` / `AGENTS.md`: canonical contract への更新。
- clean-state（git worktree）での `mise run setup` → `mise run
  check` 実測と記録。

### Out of Scope

- worktree lifecycle・orchestration layer・Dev Container・Nix・
  Docker の通常開発必須化・credential 復元（Issue #7 Non-Goals
  そのまま）。
- tool version の bump や lockfile の再生成方針変更（既存 pin を
  維持）。
- `mise run setup` 内での Ollama 起動。

## File Map

| File | Change |
| --- | --- |
| `mise.toml` | `lockfile = true`、`[tasks.setup]`、`[tasks.check]` 追加 |
| `.github/workflows/ci.yml` | locked install + `mise run setup`/`mise run check` へ集約 |
| `README.md` | セットアップ節を canonical contract に更新 |
| `AGENTS.md` | canonical contract + worktree 前提注意を追記 |

## Task Breakdown

1. **TASK-0099**: `mise.toml` に `lockfile = true`・
   `[tasks.setup]`・`[tasks.check]` を定義し、ローカルで
   `mise tasks`/`mise run` の動作を確認する。
2. **TASK-0100**: CI を `install_args: "--locked"` + canonical
   task 呼び出しへ集約する。
3. **TASK-0101**: README/AGENTS.md を更新し、git worktree による
   clean-state 検証（`mise run setup` → `mise run check`）を
   実施・記録、status 同期。

## Dependency Graph

```text
TASK-0099 (mise tasks)
   └─ TASK-0100 (CI parity)        ※ task 定義が前提
        └─ TASK-0101 (docs + clean-state verification)
```

## Risk Register

| Risk | Mitigation |
| --- | --- |
| `mise run` が toolset を自動 install しない環境で setup が失敗 | auto_install は default true。README で toolchain は mise が解決する契約を明記。clean-state 検証で実測。 |
| CI で `mise run check` の `git diff` が失敗する | checkout 直後の diff は生成物再生成差分のみ。bundle 再生成が deterministic であることを既存 freshness で確認済み。 |
| `apm install` が network 不可の環境で失敗 | 既存 contract と同じ依存。clean-state 検証は network 利用可能環境で実施。 |
| worktree 検証で untracked な pre-existing file が混入 | `git worktree add` は committed state のみを含むため混入しない。 |

## Rollback Strategy

`mise.toml`・CI・docs のみの変更で、revert は単純な逆変更。
task 定義を外しても個別 command は従来どおり実行可能。

## Verification Matrix

| Acceptance Criterion (SPEC-0017) | Verification |
| --- | --- |
| `mise run setup` が canonical setup entrypoint | `[tasks.setup]` 定義 + `mise run setup` 実測 exit 0 |
| setup 完了後に開発・テストへ移れる | `mise run check` がそのまま exit 0 |
| `mise run check` が canonical verification | `[tasks.check]` 定義 + lint/typecheck/test/freshness 実行 |
| toolchain が `mise.lock` で再現可能 | `mise.lock` Git 管理（既存）+ `lockfile = true` + CI `--locked` |
| pnpm deps が frozen 再現 | `pnpm install --frozen-lockfile` が setup に含まれる |
| APM deps が `apm.lock.yaml` から再現 | `apm install` が setup に含まれる |
| CI が同じ contract を利用 | ci.yml が `mise run setup`/`mise run check` を呼ぶ |
| README/AGENTS が同じ contract を参照 | 文書 diff 確認 |
| worktree lifecycle 等を実装していない | task 定義に worktree 操作がないこと |
| Docker/Ollama が setup 必須でない | task 定義に docker compose がないこと |
| clean-state から setup/check 成功 | git worktree での実測記録（TASK-0101） |
| credential を setup に含めない | task 定義に credential 複製がないこと |

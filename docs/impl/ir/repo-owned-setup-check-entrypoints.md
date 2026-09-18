---
id: "IMPL-034r05iHy2gdnpv1ZWjBbz"
type: "impl"
status: "completed"
title: "Repository-owned setup/check entrypoints (PLAN-034r05iHE1Fbkp7833DuTu)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "apm.lock.yaml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/define-mise-setup-check-tasks-and-lockfile-setting.md"
    - "docs/tasks/align-ci-with-canonical-entrypoints.md"
    - "docs/tasks/update-docs-and-verify-clean-state-setup-check.md"
  implemented-by: []
  depends-on:
    - "docs/plans/repo-owned-setup-check-entrypoints.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/repo-owned-setup-check-entrypoints.md"
    - "docs/specs/repo-owned-setup-contract-for-fresh-checkouts.md"
    - "docs/adr/canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/repo-owned-setup-contract-for-fresh-checkouts.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Repository-owned setup/check entrypoints (PLAN-034r05iHE1Fbkp7833DuTu)

## Summary

`mise run setup` / `mise run check` を repository-owned contract として
`mise.toml` の TOML task に定義し、CI・README・AGENTS.md を同じ
contract に揃えた。fresh checkout（git worktree）で `setup` →
`check` が exit 0 であることを実測した。toolchain 再現は既存
`mise.lock`（全 5 tool lock 済み）+ `lockfile = true` setting +
CI の `--locked` install で実現。

## Context

TASK-034r05iHQeQpDqvWrATTbr〜0101（PLAN-034r05iHE1Fbkp7833DuTu / SPEC-034r05iHK8bA7gpcCJQya9 / ADR-034r05iHrZUE6WeDAi63F3 / DESIGN-034r05iHy2gdnoRcfSVFq3）に
従う。Issue #7。PLAN-034r05iHE1Fbkp5YZSKJM2（Issue #9）の dev/distribution 分離を
前提に、root `apm.yml` が development-only である状態で canonical
bootstrap contract を定義した。

## Implementation

### Changes

- `mise.toml`: `[settings]` に `lockfile = true`（lockfile 維持・
  locked version 優先の明示）。`[tasks.setup]`（
  `pnpm install --frozen-lockfile` → `apm install` → `pnpm build`）
  と `[tasks.check]`（`pnpm lint` → `pnpm typecheck` → `pnpm test`
  → `node scripts/bundle-cli.mjs` → `git diff --exit-code --
  packages/apm/.apm/hooks/cli`）を追加。`check` には `depends` を
  付けず責務を分離（ADR-034r05iHrZUE6WeDAi63F3）
- `apm.lock.yaml`: `mise run setup` 内の `apm install` 実行で
  refresh され、旧 root `.apm/` layout の self-deploy 記録（71
  件相当）が除去された。dev-only project の正しい反映
- `.github/workflows/ci.yml`: `install_args: "--locked"` を
  mise-action に追加し、個別 pnpm step と bundle freshness step を
  `mise run setup` / `mise run check` へ集約
- `README.md`: セットアップ節を canonical contract に更新し、
  setup の内部復元内容を短く説明
- `AGENTS.md`: 同じ contract を記載し、fresh worktree では
  `node_modules`/`dist` 等の非存在を前提に `mise run setup` する
  こと、worktree 操作・生成物流用は Agent の責務外であることを
  明記

### Decisions During Implementation

- **`check` に `depends = ["setup"]` を付けない**: 「セットアップ
  済み checkout の検証」という責務に留め、setup と check の
  重複を避ける。外部は `setup` → `check` の順で呼ぶ contract を
  文書で示す
- **CI の `install_args: "--locked"`**: mise-action v4.1.0+ は
  `mise.lock` 検出で自動 `--locked` だが、action バージョンに
  関わらず明示的に保証するため記載
- **toolchain 復元を task command にしない**: `mise run` が task
  実行前に toolset を解決・install する（auto_install 既定 true、
  lockfile 優先）ため、task 内で `mise install` を呼ばず
  declarative に委ねる
- **IMPL 番号**: #8 PR が 0010、#9 が 0011 を使用するため 0012

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise tasks`: `setup` / `check` が一覧される
- ローカル（開発 checkout）: `mise run setup` exit 0
- **clean-state 実測**（`git worktree add` の fresh checkout、
  `node_modules`/`dist`/`apm_modules`/APM deploy なし）:
  - `mise trust` 後 `mise run setup` exit 0 — pnpm frozen install
    （175 packages）→ apm install（25 deps を lock SHA で復元、
    cheaper-* self-deploy なし）→ pnpm build（3 packages）
  - `mise run check` exit 0 — lint（warnings のみ）→ typecheck →
    test（core 270 + runtime-pi 255 + cli 131 = 656 tests 全
    pass）→ bundle 再生成 → `git diff --exit-code`（clean）
- `node .claude/skills/doc-status/scripts/audit_docs.js --type all`:
  192 files / no findings

## Risks

- fresh checkout では mise の trust 機構により初回 `mise trust`
  が必要（mise 標準挙動。trust していない config は読まれない）。
- `mise run check` は `pnpm test`（e2e 含む）を実行するため数十
  秒かかる。CI 相当の canonical contract として許容。
- `apm install` は remote dep 解決のため network が必要（既存
  contract と同一）。

## Follow-ups

- Issue #14: grep batchMaxBytes invariant の明文化/強制
- Issue #11: `apm.yml` ×2・`version.json`・`--version` の version
  source 統一（lockfile 群と同じ「canonical source」系の話題）
- Issue #15: README の user journey 再構成（今回のセットアップ節
  更新は最小限の範囲）

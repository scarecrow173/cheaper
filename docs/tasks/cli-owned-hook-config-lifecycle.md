---
id: "TASK-034H3PzzZ8InWeNS8Du0wB"
type: "task"
status: "done"
title: "CLI-owned hook-config lifecycle"
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
    - "docs/tasks/bounded-search-and-ranged-read-evaluation.md"
  blocks:
    - "docs/tasks/bounded-eval-and-registration-docs-e2e.md"
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
    - "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  defers: []
  deferred-by: []
---

# CLI-owned hook-config lifecycle

## Work

`cheaper hook-config {sync|verify|remove}` を実装し、project/user
scope・legacy-conflict gate・atomic replace・orphan sweep・
`--event-stdin`（APM lifecycle 写像 + post-uninstall packages gate）
を提供する。`install-devin-hooks.ps1` を CLI 委譲の thin wrapper
に縮退し、capability line に `"hookConfig": true` を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md
- docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md
- docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md

## Dependencies

- `bounded-search-and-ranged-read-evaluation`（capability line の
  matcher canonical が依存）。

## Affected Files

- `crates/cheaper-cli/src/hook/hook_config.rs`（新規）
- `crates/cheaper-cli/src/hook/mod.rs`・`targets.rs`（capability line）
- `crates/cheaper-cli/src/args.rs`・`cli.rs`（dispatch）
- `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
- `crates/cheaper-cli/tests/integration/hook.rs`・`cli.rs`
- `scripts/test/devin-hooks-install.test.mjs`

## Done When

- [ ] `hook-config sync|verify|remove` が project/user scope で動作し、
  unrelated config を保持、own entry のみを canonical 集合で識別する。
- [ ] legacy cheaper read-hook が既知登録元に残る場合 `sync` が
  書かず error、`verify` が非ゼロ + 修復案内。
- [ ] malformed JSON/JSONC・重複/改変・symlink・同時変更が非破壊
  error、temp+rename atomic replace + orphan sweep。
- [ ] `--event-stdin` が post-install/post-update → sync、
  post-uninstall（packages に cheaper 含有時のみ）→ remove、
  pre-* → no-op、未知 event/malformed → error。
- [ ] `--scope user` が POSIX `~/.config/devin/config.json` /
  Windows `%APPDATA%\devin\config.json` を対象にする。
- [ ] PS script が CLI へ委譲し、`hook targets --json` に
  `"hookConfig": true` が含まれる。

## Verification

- [ ] `cargo test -p cheaper-cli` の hook-config 系 test が pass。
- [ ] `node --test scripts/test/devin-hooks-install.test.mjs` pass。
- [ ] POSIX container で `hook-config` の scope 解決が動作。

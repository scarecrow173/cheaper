---
id: "TASK-034DmVRCxSEu4CdutLtxfM"
type: "task"
status: "done"
title: "docs and cheaper-usage skill rewrite for cargo source install"
created: "2026-10-10"
updated: "2026-10-10"
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
    - "docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md"
    - "docs/tasks/slim-payload-acquisition-machinery-removal-and-check-rewiring.md"
  blocks:
    - "docs/tasks/upstream-doc-reconciliation-and-final-verification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# docs and cheaper-usage skill rewrite for cargo source install

## Work

- Rewrite `docs/guides/distribution.md` to the cargo-source-install model (install/update commands, PATH + IDE/agent caveat, prerequisites: Rust ≥1.95 / git / OS C toolchain; version table drops `cli/version.json`).
- Shrink `docs/guides/release.md` to bump + `--force` update flow.
- Update `docs/guides/parent-agent-integration.md` (drop `#offline-preseed` link), `docs/guides/workers.md`, `README.md` "Parent-agent 連携", `AGENTS.md` (binary-free contract + `cli/` payload paragraph + setup/check text), `CONTRIBUTING.md` (bundle pipeline), `Cargo.toml:12-15` comment, `crates/cheaper-cli/src/embedded.rs:25-26` comment.
- Rewrite `packages/apm/.apm/skills/cheaper-usage/SKILL.md` "Locating the CLI" → `cheaper --version` preflight + install + PATH guidance; remove sibling/Release/cache guidance and obsolete deployed-`cli/` limitation notes.

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md
- docs/specs/cargo-source-install-distribution-for-apm-consumers.md

## Dependencies

- docs/tasks/slim-payload-acquisition-machinery-removal-and-check-rewiring.md (documents the settled state)

## Affected Files

- `docs/guides/{distribution,release,parent-agent-integration,workers}.md`, `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `Cargo.toml`, `crates/cheaper-cli/src/embedded.rs`, `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

## Done When

- [ ] No acquisition-era guidance remains in the listed live files
- [ ] `cheaper-usage` SKILL.md gives a working `cheaper --version` preflight path

## Verification

- [ ] `rg -n 'cli/version\.json|cli/bin|release-acquisition|sibling|preseed' docs/guides/ packages/apm/.apm/skills/cheaper-usage/` returns no live guidance

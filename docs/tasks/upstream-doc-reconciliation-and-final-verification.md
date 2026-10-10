---
id: "TASK-034DmVS8k50GMCHNnui6Wb"
type: "task"
status: "done"
title: "upstream doc reconciliation and final verification"
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
    - "docs/tasks/docs-and-cheaper-usage-skill-rewrite-for-cargo-source-install.md"
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
  references: []
  defers: []
  deferred-by: []
---

# upstream doc reconciliation and final verification

## Work

- Mark superseded (status + reciprocal `superseded-by` + index rows): ADR `remove-the-committed-esbuild-cli-payload-…`; specs `version-pinned-launcher-acquisition`, `one-dispatch-release-workflow`, `apm-self-contained-cli-distribution`, `payload-untracking-migration`; designs `cli-bundle-layout`, `one-dispatch-release-workflow`, `apm-executable-payload-layout-and-platform-launchers`, `payload-untracking-migration`; test-specs enumerated in spec item 6.
- Amend `docs/specs/unified-version-source-of-truth.md` + `docs/designs/cheaper-hook-subcommands-replacing-…` with superseded-clause notes.
- Update `docs/designs/overview.md` index + per-dir READMEs.
- Final live-file sweeps; temp-consumer `apm install` deploy assertions + prune/mode outcome record; `cargo install --path crates/cheaper-cli --locked` smoke; `mise run check`; `git diff --check`.

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md
- docs/specs/cargo-source-install-distribution-for-apm-consumers.md

## Dependencies

- docs/tasks/docs-and-cheaper-usage-skill-rewrite-for-cargo-source-install.md (final gate)

## Affected Files

- superseded upstream docs + indexes listed in spec item 6; `docs/designs/overview.md`

## Done When

- [ ] All spec AC items verified and evidence recorded
- [ ] `mise run check` passes; sweeps clean

## Verification

- [ ] `mise run check`
- [ ] `cargo install --path crates/cheaper-cli --locked` → `cheaper --version` + `cheaper models`
- [ ] temp-consumer `apm install` deploy check
- [ ] `git diff --check` + live-file `rg` sweeps

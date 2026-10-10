---
id: "TASK-034DmVR8wrCmjvk2D3gXFq"
type: "task"
status: "done"
title: "slim payload acquisition machinery removal and check rewiring"
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
    - "docs/tasks/thin-launchers-rename-rewrite-and-hooks-json-references.md"
  blocks:
    - "docs/tasks/docs-and-cheaper-usage-skill-rewrite-for-cargo-source-install.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/thin-path-launchers-and-slim-apm-payload-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# slim payload acquisition machinery removal and check rewiring

## Work

- Delete `packages/apm/.apm/hooks/cli/`, the `.gitignore` `cli/bin/*/` rule + comment, 7 acquisition scripts, 4 acquisition tests + `launcher-fixture.cs`, 3 workflows (`build-executables`, `release-acquisition`, `release`).
- Rewrite `scripts/check-apm-payload.mjs` (slim contract incl. BOM/CRLF, shebang/LF, `100755`, hooks.json↔launcher consistency, no bare `./cheaper`).
- New `scripts/check-lockfile.mjs` (`cargo metadata --locked` piped + `git diff --exit-code -- Cargo.lock`).
- Update `check-version.mjs` (drop `cli/version.json` decl + dead `readJsonVersion`), `set-version.mjs` (drop bundle-cli guidance), `lib/release-artifacts.mjs` (keep only the 3 version helpers).
- `mise.toml` check task rewire + description; `packages/apm/apm.yml` description drop "bundled CLI".
- Rewrite `scripts/test/apm-payload.test.mjs` and `scripts/test/apm-install-targets.test.mjs` for the slim contract.

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md
- docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md

## Dependencies

- docs/tasks/thin-launchers-rename-rewrite-and-hooks-json-references.md (renamed launcher must exist for payload checks)

## Affected Files

- `packages/apm/.apm/hooks/cli/` (delete), `scripts/*.mjs`, `scripts/test/*`, `scripts/test/fixtures/launcher-fixture.cs`, `scripts/test/lib/apm-fixture.mjs`, `.github/workflows/{build-executables,release,release-acquisition}.yml`, `.gitignore`, `mise.toml`, `packages/apm/apm.yml`

## Done When

- [ ] `node scripts/check-apm-payload.mjs` + `node scripts/check-lockfile.mjs` pass
- [ ] `node --test scripts/test/` passes; no `hooks/cli/` tree remains

## Verification

- [ ] `node scripts/check-apm-payload.mjs`
- [ ] `node scripts/check-lockfile.mjs`
- [ ] `node --test scripts/test/`

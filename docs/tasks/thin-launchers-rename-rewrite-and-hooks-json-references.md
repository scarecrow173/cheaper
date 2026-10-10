---
id: "TASK-034DmVQG3C2W8kElIVtwPN"
type: "task"
status: "done"
title: "thin launchers rename rewrite and hooks.json references"
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
  blocks:
    - "docs/tasks/slim-payload-acquisition-machinery-removal-and-check-rewiring.md"
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

# thin launchers rename rewrite and hooks.json references

## Work

- `git mv packages/apm/.apm/hooks/cheaper packages/apm/.apm/hooks/cheaper.sh`; verify `git ls-files -s` shows `100755` (fix with `git update-index --chmod=+x` if dropped).
- Rewrite `cheaper.sh` to the design's thin launcher (`command -v` guard → stderr install/PATH guidance + exit 1, else `exec cheaper "$@"`).
- Rewrite `cheaper.ps1` to the design's thin launcher (`Get-Command cheaper -CommandType Application`, Quote-Arg + `ProcessStartInfo`, `$proc.ExitCode`, explicit exit-1 fallback); UTF-8 with BOM + CRLF.
- `hooks.json`: all POSIX `command` fields → `sh ./cheaper.sh`; `powershell` fields unchanged.
- New `scripts/test/launcher-passthrough.test.mjs` (fake-PATH stub passthrough + missing-CLI guard, both launchers); drop `cargoBinary` helper, keep `nodeFreePath` in `scripts/test/lib/apm-fixture.mjs`.

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md
- docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md
- docs/specs/cargo-source-install-distribution-for-apm-consumers.md

## Dependencies

- none (chain root)

## Affected Files

- `packages/apm/.apm/hooks/cheaper` → `cheaper.sh`, `cheaper.ps1`, `hooks.json`
- `scripts/test/launcher-passthrough.test.mjs` (new), `scripts/test/lib/apm-fixture.mjs`

## Done When

- [ ] `node --test scripts/test/launcher-passthrough.test.mjs` passes
- [ ] `git ls-files -s` shows `100755` for `cheaper.sh`
- [ ] hooks.json POSIX fields all use `sh ./cheaper.sh`; no bare `./cheaper`

## Verification

- [ ] `node --test scripts/test/launcher-passthrough.test.mjs`
- [ ] `git ls-files -s packages/apm/.apm/hooks/cheaper.sh`
- [ ] powershell.exe smoke of `cheaper.ps1` stub passthrough (Windows host)

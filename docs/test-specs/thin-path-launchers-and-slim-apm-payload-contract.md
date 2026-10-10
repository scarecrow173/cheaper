---
id: "TSPEC-034DmUtUjWCThCnJulok0c"
type: "test-spec"
status: "approved"
title: "thin PATH launchers and slim APM payload contract"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/85"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes:
    - "docs/test-specs/payload-free-source-acquisition-and-native-verification.md"
    - "docs/test-specs/release-manifest-and-snapshot-integrity.md"
    - "docs/test-specs/release-publication-and-native-smoke.md"
    - "docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md"
    - "docs/test-specs/rust-migration-distribution.md"
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
    - "docs/test-specs/version-pinned-launcher-integrity-and-stream-preservation.md"
    - "docs/test-specs/embedded-builtin-resource-resolution-across-distribution-forms.md"
    - "docs/test-specs/standalone-executable-contract-parity.md"
  superseded-by: []
  related:
    - "docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md"
  derived-by: []
  verifies:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# thin PATH launchers and slim APM payload contract

## Purpose

Pin the post-#85 distribution contract: the APM package ships only
text assets (skills + hooks.json + `cheaper.sh`/`cheaper.ps1`), the
launchers resolve a Cargo-installed `cheaper` from `PATH`, and the
missing-CLI path degrades through documented stderr guidance +
nonzero exit while preserving each target's fail-open shape.

## Feature

Thin launchers (`cheaper.sh`, `cheaper.ps1`), `hooks.json`
references, the slim `check-apm-payload.mjs` contract, and the
committed-lockfile guard.

## Rules

- `cheaper.sh`: `command -v cheaper` guard → stderr guidance +
  exit 1 (no stdout) when absent; `exec cheaper "$@"` otherwise —
  argv/stdin/stdout/exit-code verbatim.
- `cheaper.ps1`: `Get-Command cheaper -CommandType Application`
  guard → stderr guidance + exit 1 when absent; `ProcessStartInfo`
  spawn with quoted argv → `Process.ExitCode`; launch failure →
  exit 1. UTF-8 BOM + CRLF preserved.
- `hooks.json`: every POSIX `command` uses `sh ./cheaper.sh`;
  every `powershell` field uses `./cheaper.ps1`; no bare
  `./cheaper`; fail-open suffixes unchanged.
- Package: no `cli/` tree, no binary payloads (extension
  allowlist + PE/ELF sniffing); `cheaper.sh` index mode `100755`.
- Lockfile: `cargo metadata --locked` passes AND
  `git diff --exit-code -- Cargo.lock` passes after build steps.

## Examples

- Fake-PATH stub `cheaper` echoes args+stdin: `cheaper.sh`
  forwards argv `"a" "b c"`, stdin bytes, stdout bytes, exit 7 →
  launcher exits 7, output identical.
- PATH without `cheaper`: `cheaper.sh` writes install command +
  PATH guidance to stderr, empty stdout, exit 1.
- `cheaper.ps1` under powershell.exe: same passthrough and guard
  behavior; `$LASTEXITCODE` never reads `$null` as success.
- hooks.json audit: copilot entry still ends `|| exit 0`; cursor
  entries still echo allow-JSON.

## Guarantees

- `node --test scripts/test/` covers passthrough + missing-CLI for
  both launchers (ps1 where a PowerShell is available).
- `check-apm-payload.mjs` fails on: missing/empty launcher,
  `cli/` presence, binary payload, BOM-less/CRLF-less `.ps1`,
  non-`100755` `cheaper.sh`, hooks.json referencing a nonexistent
  launcher or bare `./cheaper`.
- `check-lockfile.mjs` fails on manifest↔lockfile drift and on
  committed-state regeneration drift.

## Non-goals

- Hook-time install/update/download/prompt (issue non-goal).
- APM deploy mode-preservation proof beyond a conditional assert +
  documented outcome.
- Remote `cargo install --git` verification pre-merge (local
  `--path` equivalent covers the build contract).

## Risk

A stripped exec bit or missing `sh` in a hook exec environment —
mitigated by `sh ./cheaper.sh` in every POSIX command (universal
`sh` presence assumed per POSIX).

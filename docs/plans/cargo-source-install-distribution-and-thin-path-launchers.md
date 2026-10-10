---
id: "PLAN-034DmUmGAPaPy50HcfoPOK"
type: "plan"
status: "completed"
title: "cargo source install distribution and thin PATH launchers"
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
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/thin-path-launchers-and-slim-apm-payload-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# cargo source install distribution and thin PATH launchers Implementation Plan

## Goal and Approach

Execute SPEC-034Dmb2S8y3tdJI8WdKhG0 per
DESIGN-034Dmk2RqH0fMzF3gLrR0T. The change decomposes into four
independently reviewable units ordered so the runnable contract
(launchers + hooks.json + tests) lands first, the machinery
removal follows, docs/skill track it, and upstream doc
reconciliation + final verification close the chain.

## Task Decomposition

### TASK 1 — Thin launchers + hooks.json + passthrough tests

- `git mv packages/apm/.apm/hooks/cheaper
  packages/apm/.apm/hooks/cheaper.sh`; verify `git ls-files -s`
  shows `100755` (run `git update-index --chmod=+x` if it drops).
- Rewrite `cheaper.sh` to the thin launcher (`command -v` guard +
  stderr install/PATH guidance + `exec cheaper "$@"`), per the
  design's code block.
- Rewrite `cheaper.ps1` to the thin launcher
  (`Get-Command cheaper -CommandType Application`, Quote-Arg +
  `ProcessStartInfo` spawn, `$proc.ExitCode`, explicit exit-1
  fallback), UTF-8 **with BOM** + CRLF.
- `hooks.json`: all POSIX `command` fields → `sh ./cheaper.sh`;
  `powershell` fields unchanged.
- New `scripts/test/launcher-passthrough.test.mjs`: fake-PATH stub
  binary; assert argv/stdin/stdout/exit-code passthrough for
  `cheaper.sh` and `cheaper.ps1` (powershell.exe present on this
  host); empty-PATH → stderr guidance, empty stdout, exit 1.
- Keep `nodeFreePath` in `scripts/test/lib/apm-fixture.mjs` for
  the controlled minimal PATH (drop only `cargoBinary`).

Done when: `node --test scripts/test/launcher-passthrough.test.mjs`
passes; `git ls-files -s` shows `100755` for `cheaper.sh`.

### TASK 2 — Slim payload + machinery removal + check rewiring

- Delete `packages/apm/.apm/hooks/cli/` and the `.gitignore`
  `cli/bin/*/` rule + its comment line.
- Delete scripts: `bundle-cli.mjs`, `lib/apm-bundle.mjs`,
  `build-apm-bundle.mjs`, `assemble-release-bundle.mjs`,
  `generate-release-checksums.mjs`, `check-binary-size.mjs`,
  `smoke-release.mjs`.
- Delete tests + fixture: `launcher-acquisition.test.mjs`,
  `binary-size.test.mjs`, `release-artifacts.test.mjs`,
  `release-workflow.test.mjs`,
  `scripts/test/fixtures/launcher-fixture.cs`.
- Delete workflows: `build-executables.yml`,
  `release-acquisition.yml`, `release.yml`.
- Rewrite `scripts/check-apm-payload.mjs` (slim contract:
  required assets, `cli/` absence, extension allowlist +
  PE/ELF sniffing, BOM/CRLF + shebang/LF byte asserts,
  `git ls-files -s` → `100755`, hooks.json↔launcher consistency,
  no bare `./cheaper`).
- New `scripts/check-lockfile.mjs`: `cargo metadata --locked
  --format-version 1` (stdio piped) + `git diff --exit-code --
  Cargo.lock`.
- Update `check-version.mjs` (drop `cli/version.json` declaration,
  header comment, dead `readJsonVersion` lambda),
  `set-version.mjs` (drop bundle-cli guidance),
  `lib/release-artifacts.mjs` (keep only
  `cargoWorkspaceVersion`/`apmPackageVersion`/
  `setCargoWorkspaceVersion`).
- `mise.toml`: check task → drop bundle-cli + `git diff hooks/cli`
  steps, add `node scripts/check-lockfile.mjs` after the node test
  steps and `node scripts/check-apm-payload.mjs` last; update the
  task description.
- Rewrite `scripts/test/apm-payload.test.mjs` (slim contract
  asserts; remove `git check-ignore` block + fixture Cargo.toml
  copy) and `scripts/test/apm-install-targets.test.mjs`
  (`cheaper.sh`/`cheaper.ps1` deployed, `cli/` absent, conditional
  exec-bit assert).
- `packages/apm/apm.yml` description: drop "bundled CLI".

Done when: `node scripts/check-apm-payload.mjs`,
`node scripts/check-lockfile.mjs`,
`node --test scripts/test/` all pass; `git status` shows no
`hooks/cli/` tree.

### TASK 3 — Docs and skill rewrite

- `docs/guides/distribution.md` — rewrite to cargo-source-install
  model (install/update commands, PATH requirements incl.
  IDE/agent caveat, consumer prerequisites: Rust ≥1.95, git, OS C
  toolchain/MSVC; version-declaration table drops
  `cli/version.json`; no Release/preseed/cache sections).
- `docs/guides/release.md` — shrink to `set-version.mjs` bump +
  `cargo install --git … --force` update flow.
- `docs/guides/parent-agent-integration.md` — install/launcher
  description; drop `#offline-preseed` link.
- `docs/guides/workers.md`, `README.md` "Parent-agent 連携"
  section, `AGENTS.md` (binary-free contract text, `cli/` payload
  paragraph, setup/check descriptions), `CONTRIBUTING.md`
  (bundle pipeline), `Cargo.toml:12-15` comment,
  `crates/cheaper-cli/src/embedded.rs:25-26` comment,
  `packages/apm/apm.yml` (done in T2 — verify).
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — "Locating
  the CLI" → `cheaper --version` preflight + install command +
  PATH guidance; remove sibling/Release/cache guidance and the
  obsolete deployed-`cli/` limitation notes.

Done when: no acquisition-era guidance remains in the listed live
files; `cheaper-usage` gives a working preflight path.

### TASK 4 — Upstream doc reconciliation + final verification

- Supersede (status + reciprocal `superseded-by` + index rows):
  ADR `remove-the-committed-esbuild-cli-payload-…`; specs
  `version-pinned-launcher-acquisition`,
  `one-dispatch-release-workflow`,
  `apm-self-contained-cli-distribution`,
  `payload-untracking-migration`; designs `cli-bundle-layout`,
  `one-dispatch-release-workflow`,
  `apm-executable-payload-layout-and-platform-launchers`,
  `payload-untracking-migration`; test-specs enumerated in spec
  item 6 (incl. `version-pinned-launcher-integrity-…` — its own
  retire condition is met).
- Amend: `docs/specs/unified-version-source-of-truth.md` and
  `docs/designs/cheaper-hook-subcommands-replacing-…` — add
  superseded-clause notes for the removed declarations/wiring.
- Update `docs/designs/overview.md` index table + per-dir READMEs.
- Final sweeps (live files only): no bare `./cheaper`, no
  `hooks/cheaper\b`, no `cli/version.json`, no `bundle-cli`, no
  `release-acquisition`/`smoke-release` references.
- Temp-consumer `apm install` (path dep): verify deployed
  `cheaper.sh`/`cheaper.ps1`, no `cli/`, and record whether APM
  prunes stale files / preserves modes (document outcome).
- `cargo install --path crates/cheaper-cli --locked` →
  `cheaper --version`, `cheaper models` smoke.
- `mise run check` full pass; `git diff --check`.

Done when: all spec AC items verified; sweeps clean.

## Ordering

T1 → T2 → T3 → T4. T2 depends on T1's renamed launcher existing
for payload checks; T3 documents the settled state; T4 audits and
verifies everything.

## Verification

- `node --test scripts/test/` (new + rewritten tests)
- `node scripts/check-apm-payload.mjs`,
  `node scripts/check-lockfile.mjs`
- Temp-consumer `apm install` deploy assertions
- `cargo install --path crates/cheaper-cli --locked` smoke
- `mise run check` (full pipeline incl. `cargo test --workspace`)
- `git diff --check`; live-file `rg` sweeps

## Sources and Relations

- SPEC-034Dmb2S8y3tdJI8WdKhG0, ADR-034Dmb3kNAO4Ub6FxuQCtC,
  DESIGN-034Dmk2RqH0fMzF3gLrR0T
- https://github.com/scarecrow173/cheaper/issues/85

---
id: "ADR-034Dmb3kNAO4Ub6FxuQCtC"
type: "adr"
status: "accepted"
title: "Distribute CLI via cargo source install and thin PATH launchers"
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
  implements:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes:
    - "docs/designs/version-pinned-launcher-acquisition.md"
    - "docs/adr/version-pinned-launcher-acquisition.md"
    - "docs/adr/one-dispatch-release-workflow.md"
    - "docs/adr/payload-untracking-migration.md"
    - "docs/adr/remove-the-committed-esbuild-cli-payload-and-move-the-canonical-version-to-cargo.md"
  superseded-by: []
  related:
    - "docs/discovery/cargo-source-install-replaces-release-binary-acquisition-for-apm-consumers.md"
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
    - "docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"
    - "docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/cargo-source-install-replaces-release-binary-acquisition-for-apm-consumers.md"
  derived-by:
    - "docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Distribute CLI via cargo source install and thin PATH launchers

## Context and Problem Statement

The current APM distribution model (DESIGN-034zQkHD65kAeG9xVGSoFX)
ships a generated `cli/` payload inside the hook bundle and a
119-line POSIX / 216-line PowerShell launcher that locates a
platform sibling executable, verifies its SHA256SUMS against
`cli/version.json`, and otherwise downloads the pinned binary into
a user cache. That model requires a published Release — and
Release `v1.0.0` does not exist, so consumers currently cannot
obtain a working CLI at all (issue #85). The acquisition machinery
is also the dominant source of launcher complexity, test surface,
and CI work (build-executables, release, release-acquisition
workflows).

Meanwhile the CLI is already a plain Cargo binary
(`cheaper-cli`, workspace version `1.0.0`) whose workers and policy
are embedded via `include_dir!`/`include_str!` — nothing in the
`cli/` payload is needed once the executable comes from
`cargo install`. Issue #85 decides to stop shipping binaries
altogether.

## Decision Drivers

* Installability must not depend on release publication state
  (issue #85 AC).
* No built executables in Git / Git LFS / GitHub Releases / the APM
  package (issue #85 decision).
* Launcher contract (argv/stdin/stdout/exit code, per-target
  fail-open) must be preserved while acquisition is removed
  (issue #85 AC).
* Keep the committed `Cargo.lock` installable: consumer installs
  use `--locked`, so lockfile drift breaks real installs.

## Considered Options

* Option 1: cargo source install + thin PATH launchers (chosen)
* Option 2: publish v1.0.0 and keep Release acquisition
* Option 3: direct `cheaper hook` calls in hooks.json, no launchers
* Option 4: hook-time auto-install of the CLI

## Decision Outcome

Chosen option: "Option 1: cargo source install + thin PATH
launchers", because issue #85 explicitly decides the model and it
removes the entire acquire/verify/cache surface while preserving
hook contracts.

1. Consumers install the CLI from source:
   `cargo install --git https://github.com/scarecrow173/cheaper.git
   --locked cheaper-cli`. `--locked` pins the dependency lockfile;
   no `--tag`/`--rev`/`--branch` — "latest" is the default branch
   HEAD at install time. Updates are the same command plus
   `--force`.
2. The APM package ships only: skills, `hooks.json`, and two thin
   launchers — `cheaper.sh` (POSIX, renamed from `cheaper`,
   shebang + executable bit preserved) and `cheaper.ps1` (Windows,
   filename kept). Each resolves `cheaper`/`cheaper.exe` from
   `PATH` and forwards argv/stdin/stdout/exit code verbatim. No
   extensionless compatibility wrapper. `cheaper.sh` keeps index
   mode 100755 (POSIX hook entries have no fail-open fallback —
   a stripped exec bit silently breaks windsurf/claude hooks);
   `cheaper.ps1` keeps UTF-8 BOM + CRLF for Windows PowerShell 5.1.
3. Uninstalled state: the launcher itself guards with
   `command -v cheaper` / `Get-Command cheaper[.exe]`, writes the
   install command and PATH guidance (`~/.cargo/bin`,
   `%USERPROFILE%\.cargo\bin`, IDE/agent PATH-inheritance caveat)
   to stderr, and exits nonzero without writing to stdout —
   preserving each target's fail-open contract handled by
   hooks.json.
4. All Release/binary acquisition machinery is deleted: `cli/`
   payload, download/checksum/version-pin/sibling/cache/
   mirror/preseed code, acquisition-only scripts, tests, and the
   build-executables/release/release-acquisition workflows. Cargo
   version declarations and embedded resources stay — they are
   product/runtime concerns, not distribution concerns. The
   committed `Cargo.lock` is now consumer-facing (it backs every
   `--locked` install), so `mise run check` gains a lockfile
   consistency guard. Consumer prerequisites are documented:
   Rust ≥1.95 (rig's `rust-version`), `git`, and an OS C
   toolchain (MSVC build tools on Windows).

### Consequences

#### Positive

- Installability no longer depends on release publication state.
- Launchers shrink from ~120/216 lines of acquisition logic to a
  PATH lookup + exec — simpler contract to test and audit.
- The APM package shrinks to text-only assets (skills, hooks.json,
  two launcher scripts): no executables in Git, LFS, Releases, or
  the package.
- CI drops the executable build matrix, release publishing, and
  acquisition smoke legs.
- `cheaper --version` becomes the single honest preflight: the
  version reported is the version the user installed.

#### Negative / Costs

- Consumers need a Rust toolchain + `cargo` (OS-specific
  prerequisites documented). No more "works without Rust" install.
- No offline/binary distribution path; every install compiles from
  source (first `cargo install --git` is minutes, not seconds).
- Hook-time installs are impossible by design — a missing CLI is a
  documented preflight failure, not a fix-it-now hook action.
- Existing consumers carry stale `cli/` payload + old launchers
  until they re-run `apm install` (migration is reinstall; old
  acquisition cache is orphaned but harmless).

### Confirmation

- `mise run check` passes with the slim package (spec AC).
- Temp-consumer `apm install` deploys `cheaper.sh`/`cheaper.ps1`
  with no `cli/` payload; exec bit verified (spec AC).
- No `./cheaper` (bare) reference remains in the package, docs, or
  tests (spec AC).

## Implementation Plan

See the spec's scope item 4 for the full file map (deletions,
rewrites, script updates, workflow removals) and item 6 for the
upstream doc supersede set.

## Verification

`cargo install --path crates/cheaper-cli --locked` produces a
working `cheaper --version` / `cheaper models` locally; the remote
`--git` install is verified post-merge. Launcher pass-through and
missing-CLI guard behavior are covered by rewritten node tests.

## Pros and Cons of the Options

### Option 1: cargo source install + thin PATH launchers

Good: no release dependency; minimal launcher surface; text-only
package; matches issue #85 decision.
Bad: requires consumer Rust toolchain; slow first install; no
offline binary path.

### Option 2: publish v1.0.0 and keep Release acquisition

Good: zero code change; binary distribution for users without
Rust.
Bad: explicitly rejected by issue #85 — keeps the download,
checksum, cache, mirror/preseed machinery plus the build/release
CI legs permanently.

### Option 3: direct `cheaper hook` calls in hooks.json

Good: zero launcher files.
Bad: rejected by the issue — the launcher is the single place that
can emit human-readable stderr guidance while each target's
hooks.json entry decides its own fail-open shape (copilot
`|| exit 0`, cursor allow JSON).

### Option 4: hook-time auto-install

Good: missing CLI self-heals.
Bad: non-goal per issue #85 — hooks must not install, update,
download, or prompt during execution.

## More Information

Supersedes:

- `docs/designs/version-pinned-launcher-acquisition.md`
  (DESIGN-034zQkHD65kAeG9xVGSoFX) — the pinned-Release acquisition
  contract, in full.
- `docs/adr/version-pinned-launcher-acquisition.md`
  (ADR-034zIOoBXZtluQdBMlJQge) — the upstream accepted ADR.
- `docs/adr/one-dispatch-release-workflow.md`
  (ADR-034zThoekpLqCcHhYO4MPe) — the release publication pipeline
  this change deletes.
- `docs/adr/payload-untracking-migration.md` — the tracked/
  untracked `cli/bin` payload model no longer exists.

#71/#73-era distribution assumptions (bundled `cli/` payload,
executable siblings) are retired; the derived specs/designs/
test-specs are marked superseded where they assert removed
behavior — the full set is enumerated in the spec.

---
id: "SPEC-034Dmb2S8y3tdJI8WdKhG0"
type: "spec"
status: "implemented"
title: "cargo source install distribution for APM consumers"
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
    - "docs/specs/version-pinned-launcher-acquisition.md"
    - "docs/specs/one-dispatch-release-workflow.md"
    - "docs/specs/apm-self-contained-cli-distribution.md"
    - "docs/specs/payload-untracking-migration.md"
  superseded-by: []
  related:
    - "docs/discovery/cargo-source-install-replaces-release-binary-acquisition-for-apm-consumers.md"
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
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
---

# cargo source install distribution for APM consumers

## What Should Be Built

Replace Release-binary acquisition with Cargo source install as the
distribution model for APM consumers. The consumer install path
becomes `cargo install --git <repo> --locked cheaper-cli`; the APM
package ships skills, `hooks.json`, and thin launchers
(`cheaper.sh`, `cheaper.ps1`) that resolve `cheaper` from `PATH`.
All Release-download, version-pin, sibling-executable, user-cache,
and bundled `cli/` payload machinery is removed, along with its
dedicated scripts, tests, workflows, and documentation.

## Why Now

Release `v1.0.0` is unpublished, so the current acquisition model
cannot deliver a working CLI to APM consumers at all. Issue #85
decides the new model rather than shipping a release: no built
executables in Git / Git LFS / Releases / the APM package; the
consumer's Rust toolchain builds from the repository's latest
source. This also eliminates the acquire/verify/cache surface that
dominates launcher complexity and CI time.

## Users and Value

- APM consumers get a well-defined install path that does not
  depend on release publication state (the ref resolves to
  default-branch HEAD at install time; the lockfile pins deps).
- Maintainers drop the acquisition machinery (download, checksum
  manifest, cache boundary, platform sibling, mirror/preseed) and
  the build-executables/release-acquisition CI legs.
- Parent agents get launchers that always invoke the CLI the user
  installed, with a clear preflight path when it is absent.

## Scope

### In Scope

1. **Launchers.** Rename POSIX launcher `packages/apm/.apm/hooks/
   cheaper` → `cheaper.sh` (keep shebang + executable bit) and
   rewrite both `cheaper.sh` and `cheaper.ps1` as thin launchers
   that resolve the Cargo-installed `cheaper`/`cheaper.exe` from
   `PATH`, forwarding argv/stdin/stdout/exit code verbatim.
   Uninstalled state: a `command -v`/`Get-Command` guard writes the
   install command + PATH guidance to stderr and exits nonzero
   without writing to stdout — preserving each target's fail-open
   contract (copilot `|| exit 0`, cursor allow-JSON fallback,
   claude/pretooluse schema, windsurf).
2. **hooks.json.** All POSIX commands switch `./cheaper` →
   `./cheaper.sh`; PowerShell entries keep `./cheaper.ps1`.
3. **Payload removal.** Delete `packages/apm/.apm/hooks/cli/`
   (version.json, workers, policy.yaml, bin/) — the cargo-built
   binary embeds workers + policy via `include_dir!`/`include_str!`.
   Remove `bundle-cli.mjs`/`lib/apm-bundle.mjs` payload generation
   from `mise run check` (replace with a hooks.json ↔
   launcher-file consistency check) and delete the
   payload-freshness `git diff` step. Remove the dead
   `.gitignore` rule for `cli/bin/*/`.
4. **Acquisition machinery removal — full file map:**
   - **Delete scripts:** `bundle-cli.mjs`, `lib/apm-bundle.mjs`,
     `build-apm-bundle.mjs`, `assemble-release-bundle.mjs`,
     `generate-release-checksums.mjs`, `check-binary-size.mjs`,
     `smoke-release.mjs`.
   - **Delete tests:** `launcher-acquisition.test.mjs`,
     `binary-size.test.mjs`, `release-artifacts.test.mjs`,
     `release-workflow.test.mjs`, and the orphaned fixture
     `scripts/test/fixtures/launcher-fixture.cs`; drop the dead
     `cargoBinary` helper from `scripts/test/lib/apm-fixture.mjs`
     (keep `nodeFreePath` — the new passthrough test reuses it
     for a controlled minimal PATH).
   - **Add tests/scripts:** `scripts/test/launcher-passthrough.test.mjs`
     (argv/stdin/stdout/exit-code passthrough + missing-CLI guard
     for both launchers) and `scripts/check-lockfile.mjs`
     (committed-lockfile guard: `cargo metadata --locked` +
     `git diff --exit-code -- Cargo.lock`).
   - **Rewrite tests:** `apm-payload.test.mjs` (assert the slim
     package: launchers + hooks.json + skills, no `cli/` assets,
     no executables), `apm-install-targets.test.mjs` (deployed
     `cheaper.sh` + `cheaper.ps1`, `cli/` absent, exec-bit
     assertion where the platform supports it). Keep
     `apm-package-layout.test.mjs` as-is.
   - **Update scripts:** `check-apm-payload.mjs` (rewrite for the
     slim package; required asset `cheaper` → `cheaper.sh`),
     `check-version.mjs` (drop the `cli/version.json` declaration
     entry + header comment — otherwise `mise run check` hard-fails
     on the deleted file), `set-version.mjs` (drop the
     `bundle-cli.mjs` regeneration guidance).
     `lib/release-artifacts.mjs` is kept only for the version
     helpers (`cargoWorkspaceVersion`, `apmPackageVersion`,
     `setCargoWorkspaceVersion`, `readJsonVersion`) consumed by
     check/set-version; `verifyReleaseVersion` and all
     release/executable helpers are removed.
   - **Delete workflows:** `build-executables.yml`,
     `release-acquisition.yml`, and `release.yml` (already a
     retired refusal stub). `ci.yml` needs no edit — it only runs
     `mise` tasks.
   - **mise.toml:** update the `check` task (drop bundle-cli
     regeneration + payload freshness diff, add `check-lockfile.mjs`
     — the committed-state lockfile guard, since earlier build
     steps regenerate a stale working-tree lockfile and a
     working-tree-only check would be a no-op in CI) and its
     description.
5. **Docs & skill.** Rewrite acquisition guidance in:
   `docs/guides/distribution.md` (incl. its version-declaration
   table), `docs/guides/release.md` (shrink to the bump/`--force`
   update flow — the release-acquisition body is deleted),
   `docs/guides/parent-agent-integration.md` (incl. the
   `#offline-preseed` anchor it links), `docs/guides/workers.md`,
   README "Parent-agent 連携" section (lines ~104-109 — there is no
   install section), `AGENTS.md` (binary-free contract text +
   setup/check descriptions + `cli/` payload paragraph),
   `CONTRIBUTING.md` (bundle-cli/payload pipeline docs), plus
   stale comments at `Cargo.toml:12-15`,
   `crates/cheaper-cli/src/embedded.rs:25-26`, and
   `packages/apm/apm.yml:3` ("bundled CLI" description). Rewrite
   `cheaper-usage` SKILL.md "Locating the CLI" → Cargo-install
   preflight (`cheaper --version`) + PATH guidance incl. IDE/agent
   environments, and remove the now-obsolete deployed-`cli/`
   limitation notes (the Copilot `.json` exclusion gap disappears
   because schemas are embedded in the binary).
6. **Upstream doc reconciliation — supersede set:**
   - ADRs (supersede): `version-pinned-launcher-acquisition`,
     `one-dispatch-release-workflow`, `payload-untracking-migration`,
     `remove-the-committed-esbuild-cli-payload-and-move-the-canonical-version-to-cargo`
     (accepted; mandates bundle-cli + `cli/version.json`).
   - Specs (supersede): `version-pinned-launcher-acquisition`,
     `one-dispatch-release-workflow`,
     `apm-self-contained-cli-distribution`,
     `payload-untracking-migration` (`approved`/`implemented`).
   - Specs (amend — core decision survives, one clause removed):
     `unified-version-source-of-truth` (Cargo stays canonical; the
     `cli/version.json` secondary-declaration clause is deleted).
   - Designs (supersede): `version-pinned-launcher-acquisition`
     (already marked), `cli-bundle-layout`,
     `one-dispatch-release-workflow`,
     `apm-executable-payload-layout-and-platform-launchers`,
     `payload-untracking-migration`.
   - Designs (amend): `cheaper-hook-subcommands-replacing-node-based-hook-adapters`
     (subcommands stay; `./cheaper`/`cli/bin` wiring clauses
     removed).
   - Test-specs (supersede/retire where they assert removed
     behavior): `payload-free-source-acquisition-and-native-verification`,
     `release-manifest-and-snapshot-integrity`,
     `release-publication-and-native-smoke`,
     `release-workflow-manual-dispatch-and-artifact-contract`,
     `rust-migration-distribution`,
     `apm-executable-payload-layout-and-launcher-dispatch-contract`,
     `cheaper-hook-subcommands-and-hooks-json-executable-contract`,
     `version-pinned-launcher-integrity-and-stream-preservation`
     (its own Purpose states it retires when the acquisition
     contract is abolished),
     `embedded-builtin-resource-resolution-across-distribution-forms`,
     `standalone-executable-contract-parity` — mark each where it
     asserts removed behavior; keep hook-contract assertions that
     remain true.
   - Index updates: per-dir `README.md` rows plus
     `docs/designs/overview.md`'s index table.
   - Historical task/plan/impl-record artifacts documenting past
     work stay as-is (record of what was done, not live contract);
     where they define a live contract superseded by this chain,
     mark superseded.

### Out of Scope

- Publishing any GitHub Release or binary artifacts (explicitly
  rejected by the issue).
- Replacing the launchers with direct `cheaper hook` calls in
  hooks.json (non-goal per issue).
- Hook-time auto-install / auto-update / interactive prompts
  (non-goal).
- Rust toolchain provisioning for consumers (documented
  prerequisite only).
- `cli/` payload's workers/policy content itself — unchanged,
  still embedded in the binary.

## Requirements

- R1: `hooks.json` POSIX commands invoke `sh ./cheaper.sh`
  (the `sh` prefix removes dependence on the deployed file mode —
  a stripped exec bit would silently deny hooks on targets with
  no `||` fallback); PowerShell references `./cheaper.ps1`; no
  entry references the old `./cheaper` name.
- R2: `cheaper.sh` is a thin POSIX launcher: `command -v cheaper`
  guard → stderr guidance + nonzero exit on missing CLI, else
  `exec cheaper "$@"`. `cheaper.ps1` is the PowerShell equivalent:
  `Get-Command cheaper -CommandType Application` (avoids
  functions/aliases resolving to an empty `.Source`) → stderr
  guidance, else spawn via `ProcessStartInfo` with manual argv
  quoting — the existing launcher's deliberate PS 5.1 workaround
  (native `& exe @args` drops empty-string args and mangles
  quotes) — and propagate `Process.ExitCode` with an explicit
  nonzero fallback when launch fails. Neither writes to stdout on
  failure. `cheaper.sh` keeps mode 100755 in the git index
  (add `git update-index --chmod=+x` if the rename drops it, and
  assert the mode in tests). `cheaper.ps1` stays UTF-8 **with
  BOM** + CRLF — enforced by a byte-level payload-check assert —
  because Windows PowerShell 5.1 misreads BOM-less non-ASCII
  files (`docs/guides/distribution.md` documents the BOM
  requirement).
- R3: No file under `packages/apm/.apm/hooks/cli/` remains; no
  committed or generated executable ships in the APM package.
- R4: `mise run check` passes with the slim payload: bundle-cli
  regeneration and the `cli` freshness diff are removed; a check
  asserting hooks.json ↔ launcher-file consistency replaces them.
- R5: Acquisition-only scripts/tests/workflows listed in scope
  item 4 are deleted; remaining scripts that depended on them are
  updated.
- R6: Install docs tell consumers to run
  `cargo install --git https://github.com/scarecrow173/cheaper.git
  --locked cheaper-cli` (no tag/rev pin; `--locked` pins the
  lockfile, not the ref), `--force` to update, and document:
  minimum toolchain (Rust ≥1.95 — rig's `rust-version`
  requirement — plus `git`, and OS build prerequisites: a C
  toolchain / MSVC build tools on Windows), and the
  `~/.cargo/bin` / `%USERPROFILE%\.cargo\bin` PATH requirements
  incl. the IDE/agent-startup caveat. A committed-`Cargo.lock`
  consistency guard (`cargo metadata --locked` or equivalent) is
  added to `mise run check` so drift cannot break consumer
  `--locked` installs.
- R7: `cheaper-usage` SKILL.md preflights `cheaper --version` and
  guides install/PATH setup when missing; it no longer describes
  sibling or Release acquisition.
- R8: The supersede set enumerated in scope item 6 is marked
  (accepted ADRs / approved specs / approved designs /
  acquisition test-specs), with reciprocal `superseded-by`
  relations; historical task/plan/impl records stay as-is.

## Acceptance Criteria

- [ ] Positive check: every POSIX `command`/`shell:bash` entry in
      `hooks.json` invokes `sh ./cheaper.sh` and every `powershell`
      entry invokes `./cheaper.ps1` (asserted by
      `check-apm-payload.mjs`); negative check: no bare `./cheaper`
      (no extension) reference remains in **live files** —
      `packages/apm/`, `scripts/`, `.github/`, `docs/guides/`,
      README/AGENTS/CONTRIBUTING, `crates/`, and non-superseded
      specs/designs/test-specs (superseded/historical docs are
      excluded by scope, not edited).
- [ ] Temp consumer `apm install` (path dep) deploys
      `.windsurf/hooks/<dep>/cheaper.sh` + `cheaper.ps1` + no `cli/`
      payload; `cheaper.sh` has index mode 100755 in the repo and
      retains an executable bit after APM deploy (verify; if APM
      strips modes, document the fallback decision — e.g. invoke
      via `sh` — rather than silently failing hooks). Reinstall
      over a prior-version deploy: verify stale `cli/` + old
      `cheaper` launcher are pruned or explicitly document the
      accepted stale-remnant behavior.
- [ ] With a cargo-installed `cheaper` on PATH, a deployed
      `cheaper.sh` launcher passes stdin/argv/stdout/exit code
      through (hook contract smoke via local fixture or fake PATH
      stub).
- [ ] With `cheaper` absent from PATH, the launcher writes the
      install command to stderr, writes nothing to stdout, and
      exits nonzero; copilot/cursor hooks.json fallbacks still
      produce their allow/fail-open outputs.
- [ ] `mise run check` passes — including the lockfile guard —
      and `git status` shows no `packages/apm/.apm/hooks/cli/`
      tree.
- [ ] `cargo install --path crates/cheaper-cli --locked` (local
      equivalent of the documented `--git` command; the remote
      `--git` install is verified post-merge against the pushed
      branch or main HEAD) produces a working `cheaper --version`
      and `cheaper models`.
- [ ] No workflow under `.github/workflows/` builds or publishes
      distribution executables.
- [ ] `cheaper-usage` SKILL.md contains no sibling/Release/cache
      guidance and includes the cargo-install preflight.

## Deferred Design Concerns

- Registry-published APM package reference (`apm install cheaper`
  without path/git ref) — depends on APM registry flow, tracked
  separately.
- Whether the thin launchers should also fall back to a sibling
  `cli/bin/` for repo-local dev convenience — deferred; the issue
  scopes launchers to PATH resolution only (devs use
  `cargo install --path`).
- Hook guard duplication between `cheaper.sh`/`cheaper.ps1` and
  `hooks.json` inline guards — launcher-internal guard chosen per
  the issue; inline hooks.json guards only if a target proves the
  launcher guard insufficient.

## Sources and Relations

- https://github.com/scarecrow173/cheaper/issues/85 (decisions
  1-8, implementation tasks, acceptance criteria — authoritative
  input)
- discovery:
  `docs/discovery/cargo-source-install-replaces-release-binary-acquisition-for-apm-consumers.md`
- ADR:
  `docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md`
- Superseded design: `docs/designs/version-pinned-launcher-acquisition.md`
  (DESIGN-034zQkHD65kAeG9xVGSoFX)

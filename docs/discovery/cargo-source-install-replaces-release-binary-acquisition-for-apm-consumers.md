---
id: "DISC-034DlM8Pf1tWBtLqX1VWFc"
type: "discovery"
status: "resolved"
title: "cargo source install replaces release binary acquisition for APM consumers"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cargo source install replaces release binary acquisition for APM consumers

## Exploration Goal

Issue #85: Release `v1.0.0` is unpublished, so APM consumers cannot
acquire a working CLI binary. The issue decides (not proposes) a
distribution redesign: **no built executables in Git / Git LFS /
GitHub Releases / the APM package**; consumers install the CLI from
source via `cargo install --git ... --locked cheaper-cli`, and the
APM package ships only skills + hooks.json + thin launchers that
invoke a Cargo-installed `cheaper` on `PATH`.

The discovery work is the audit: enumerate every asset in the repo
whose sole purpose is Release-binary acquisition, sibling lookup,
version pinning, or bundled `cli/` payload, and decide keep /
rewrite / delete for each — because the issue's implementation tasks
presume that inventory.

## Key Issues and Assumptions

### Confirmed asset inventory (current state on `main`)

**Hook bundle (`packages/apm/.apm/hooks/`)**
- `cheaper` — 119-line POSIX launcher: sibling `cli/bin/<platform>/`
  lookup, `cli/version.json` pin, `CHEAPER_RELEASE_BASE_URL`
  download, `CHEAPER_CACHE_DIR` user cache, SHA256SUMS verification,
  lockfile, LFS-pointer guard. Contract:
  `DESIGN-034zQkHD65kAeG9xVGSoFX` ("version 固定 Release
  acquisition").
- `cheaper.ps1` — 216-line PowerShell equivalent.
- `hooks.json` — POSIX commands reference `./cheaper`; PowerShell
  entries reference `./cheaper.ps1`.
- `cli/` — generated payload: `version.json`, `workers/` +
  schemas, `policy.yaml`, `bin/` (empty/ignored platform dirs).
  Exists solely for sibling/acquisition. The cargo-built binary
  embeds `resources/workers` (`include_dir!`) and
  `resources/policy.yaml` (`include_str!`) — see
  `crates/cheaper-cli/src/embedded.rs:24-30` — so a PATH-resolved
  CLI needs none of it.

**Scripts** (`scripts/`)
- `bundle-cli.mjs` + `lib/apm-bundle.mjs` — generate `cli/` payload.
- `build-apm-bundle.mjs` — adds platform executables to the bundle.
- `check-apm-payload.mjs` — payload freshness/required-assets check.
- `check-version.mjs`, `check-binary-size.mjs`,
  `set-version.mjs`, `generate-release-checksums.mjs`,
  `assemble-release-bundle.mjs`, `lib/release-artifacts.mjs`,
  `smoke-release.mjs`.

**Tests** (`scripts/test/`)
- `launcher-acquisition.test.mjs` — the big POSIX+pwsh acquisition
  suite (sibling, download, preseed, cache, fail-closed, concurrent
  first-run, Windows ARM64, Invoke-WebRequest fallback).
- `apm-install-targets.test.mjs`, `apm-payload.test.mjs`,
  `apm-package-layout.test.mjs`, `binary-size.test.mjs`,
  `release-artifacts.test.mjs`, `release-workflow.test.mjs`.

**CI** (`.github/workflows/`)
- `build-executables.yml`, `release.yml`, `release-acquisition.yml`
  — executable build matrix, release publishing, acquisition smoke.
- `ci.yml` — canonical check incl. bundle-cli regeneration +
  payload check (kept, but payload content changes).

**Docs/skills**
- `docs/guides/distribution.md`, `docs/guides/release.md`,
  `docs/guides/parent-agent-integration.md`, `docs/guides/workers.md`,
  `AGENTS.md` (binary-free source contract text),
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md` ("Locating the
  CLI" — sibling/Release acquisition guidance, just rewritten in
  #87), README.

**Upstream docs to reconcile**
- `DESIGN-034zQkHD65kAeG9xVGSoFX` (pinned acquisition contract),
  ADR-034z1xMK43jRs3D79Urhpc (SEA executables — already
  `superseded`), distribution/dispatch specs; #71/#73-era docs.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: full issue-#85 scope in one chain (launcher rename + thin launchers + payload removal + scripts/tests/CI/docs/skill) | Single coherent migration; no intermediate broken state | Large diff; review surface is wide | yes |
| B: thin launchers first, removal later | Smaller diffs | Dead acquisition machinery lingers; two migrations | no |
| C: publish v1.0.0 release instead (keep acquisition) | Zero code change | Contradicts the issue's explicit design decision | no |

## Tentative Conclusions and Hypotheses

- **Confirmed:** everything under `cli/` in the hook bundle is
  acquisition-only — the cargo-installed binary embeds workers and
  policy via `include_dir!`/`include_str!`.
- **Confirmed:** POSIX `hooks.json` commands must switch
  `./cheaper` → `./cheaper.sh`; PowerShell stays `./cheaper.ps1`.
  APM deploys dotfiles; whether `.sh` extension survives target
  install unchanged should be verified via temp consumer install
  (measured APM 0.28.0 deploys hook bundle files verbatim, e.g.
  `hooks/apm/cheaper.ps1` — same expectation for `.sh`).
- **Confirmed:** the thin launchers resolve `cheaper` /
  `cheaper.exe` from `PATH` and preserve argv/stdin/stdout/exit
  code; uninstalled state gets a launcher-internal
  `command -v`/`Get-Command` guard writing guidance to stderr
  without breaking per-target fail-open contracts (copilot
  `|| exit 0`, cursor allow-JSON fallback).
- **Hypothesis:** the `cli/` payload deletion removes the need for
  `bundle-cli.mjs` regeneration in `mise run check` and most of
  `check-apm-payload.mjs`'s required-assets list — exact removal
  set to be settled in spec/design, per the issue's audit task.

## Open Questions

- ~~Which scripts/tests are deletable vs partial~~ — resolved:
  `check-version.mjs`/`set-version.mjs` directly read
  `cli/version.json` and need edits, not just the lib helpers;
  `apm-payload.test.mjs`/`apm-install-targets.test.mjs` need
  rewrites (they assert the payload). Enumerated in the spec's
  file map.
- Executable bit on `cheaper.sh` through `apm install` — the repo
  index has mode 100755 on `cheaper` today; whether APM deploy
  preserves file modes and prunes files removed from the package
  (stale `cli/` + old `cheaper` in existing consumers) is verified
  during implementation via temp-consumer install.
- Does anything outside `scripts/`/`hooks/` consume `cli/` payload
  — confirmed no; only the acquisition launchers, payload check,
  and tests reference it.

## Promotion Candidates

- [x] spec-doc needed: the issue's decisions + AC formalized into
      requirements and file map
- [x] adr-doc needed: durable decision — distribution model switch
      (binary acquisition → cargo source install), superseding the
      pinned-acquisition design docs

## Sources and Relations

- https://github.com/scarecrow173/cheaper/issues/85
- `packages/apm/.apm/hooks/cheaper` / `cheaper.ps1` / `hooks.json` /
  `cli/`
- `scripts/bundle-cli.mjs`, `scripts/lib/apm-bundle.mjs`,
  `scripts/check-apm-payload.mjs`, `scripts/smoke-release.mjs`
- `scripts/test/launcher-acquisition.test.mjs`,
  `scripts/test/apm-install-targets.test.mjs`
- `.github/workflows/{build-executables,release,release-acquisition,ci}.yml`
- `crates/cheaper-cli/src/embedded.rs` (include_dir/include_str)
- `DESIGN-034zQkHD65kAeG9xVGSoFX` (pinned acquisition contract)

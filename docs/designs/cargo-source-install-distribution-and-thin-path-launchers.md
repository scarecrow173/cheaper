---
id: "DESIGN-034Dmk2RqH0fMzF3gLrR0T"
type: "design"
status: "approved"
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
  implements:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes:
    - "docs/designs/cli-bundle-layout.md"
    - "docs/designs/one-dispatch-release-workflow.md"
    - "docs/designs/apm-executable-payload-layout-and-platform-launchers.md"
    - "docs/designs/payload-untracking-migration.md"
  superseded-by: []
  related:
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
    - "docs/discovery/cargo-source-install-replaces-release-binary-acquisition-for-apm-consumers.md"
    - "docs/test-specs/thin-path-launchers-and-slim-apm-payload-contract.md"
    - "docs/designs/overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  derived-by:
    - "docs/plans/cargo-source-install-distribution-and-thin-path-launchers.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cargo source install distribution and thin PATH launchers

## Context

Implements SPEC-034Dmb2S8y3tdJI8WdKhG0 under ADR-034Dmb3kNAO4Ub6FxuQCtC.
The issue fixes the failure that no Release `v1.0.0` exists: APM
consumers acquire the CLI via `cargo install --git … --locked
cheaper-cli`, and the APM package degenerates to skills +
`hooks.json` + two thin launchers. Everything whose sole job was
binary acquisition is removed.

## Scope

### In Scope

Per spec scope items 1–6: launchers, hooks.json, `cli/` payload
removal, acquisition machinery file map, docs/skill rewrite,
upstream supersede marking, exec-bit + BOM/CRLF + lockfile-guard
requirements.

### Out of Scope

Per spec: no release publication, no hook-time installs, no
registry work, no Rust-code changes (embedded resources and Cargo
version declarations are untouched).

## Components and Boundaries

### `packages/apm/.apm/hooks/cheaper.sh` (renamed from `cheaper`)

```sh
#!/bin/sh
# PATH 上の Cargo install 済み cheaper を起動する thin launcher。
# 契約: ADR-034Dmb3kNAO4Ub6FxuQCtC。
set -eu
if ! command -v cheaper >/dev/null 2>&1; then
  printf '%s\n' \
    "cheaper launcher: 'cheaper' CLI is not installed or not on PATH." \
    "install: cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli" \
    "then ensure ~/.cargo/bin (POSIX) or %USERPROFILE%\.cargo\bin (Windows) is on the IDE/agent PATH and restart it." \
    >&2
  exit 1
fi
exec cheaper "$@"
```

- `exec` preserves argv/stdin/stdout and propagates the CLI's exit
  code exactly; the guard writes nothing to stdout.
- `git mv cheaper cheaper.sh` then verify index mode stays
  `100755` (`git ls-files -s`); add
  `git update-index --chmod=+x cheaper.sh` if it drops. The bit is
  still required by the issue AC even though hooks.json invokes
  via `sh` (belt-and-braces; a `cheaper` shell function matching
  `command -v` but failing `exec` still exits nonzero, which is
  contract-safe).

### `packages/apm/.apm/hooks/cheaper.ps1` (rewritten)

```powershell
# PATH 上の Cargo install 済み cheaper を起動する thin launcher。
# 契約: ADR-034Dmb3kNAO4Ub6FxuQCtC。
$ErrorActionPreference = "Stop"
$cli = Get-Command cheaper -CommandType Application -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $cli -or -not $cli.Source) {
    [Console]::Error.WriteLine("cheaper launcher: 'cheaper' CLI is not installed or not on PATH.")
    [Console]::Error.WriteLine("install: cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli")
    [Console]::Error.WriteLine("then ensure %USERPROFILE%\.cargo\bin is on the IDE/agent PATH and restart it.")
    exit 1
}
function Quote-Arg([string]$a) {
    if ($a -notmatch '[\s"]') { return $a }
    return '"' + ($a -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $cli.Source
$psi.UseShellExecute = $false
$psi.Arguments = ($args | ForEach-Object { Quote-Arg $_ }) -join ' '
try {
    $proc = [System.Diagnostics.Process]::Start($psi)
    $proc.WaitForExit()
    exit $proc.ExitCode
} catch {
    [Console]::Error.WriteLine("cheaper launcher: failed to start cheaper: " + $_.Exception.Message)
    exit 1
}
```

- `-CommandType Application` avoids functions/aliases/PS scripts
  (a bare `Get-Command cheaper` could resolve a function whose
  `.Source` is empty — passing the guard then exiting 0 — or a
  deployed `cheaper.ps1`, recursing). `.exe` resolves via PATHEXT.
- `ProcessStartInfo` + manual quoting retains the repo's existing
  PS 5.1 workaround (native `& exe @args` drops empty-string args
  and mangles quotes — the current launcher's own comment
  documents this). `UseShellExecute=$false` with no stream
  redirects inherits the console handles, so stdin/stdout pass
  through; `Start()` failure throws → exit 1; `ExitCode` is read
  directly so a launch failure can never masquerade as exit 0.
- File stays UTF-8 **with BOM** + CRLF (PS 5.1 requirement,
  `.gitattributes` already pins `eol=crlf` for `*.ps1`; the BOM is
  enforced by a byte-level assert in `check-apm-payload.mjs`).

### `packages/apm/.apm/hooks/hooks.json`

POSIX `command` fields of all six hook types (`pre_read_code`,
`pre_run_command`, `preToolUse`, `PreToolUse`, `beforeReadFile`,
`beforeShellExecution`) become `sh ./cheaper.sh …` — the `sh`
prefix removes the deployed-mode dependency entirely: even if a
checkout or APM deploy strips the executable bit, the hook still
runs (review finding: otherwise a stripped bit is a silent
deny-all on windsurf/claude/codex, which have no `||` fallback).
`powershell` fields and all fail-open suffixes (`|| exit 0`,
`|| echo '{"permission":"allow"}'`, matchers/schemata) unchanged.
The 100755 index mode is still kept and asserted — `sh` is belt,
the bit is braces.

### `scripts/check-apm-payload.mjs` (rewritten)

Slim contract:
- Required nonempty assets: `cheaper.sh`, `cheaper.ps1`,
  `hooks.json` (LFS-pointer + JSON-parse guards kept).
- `cli/` must not exist; binary-payload detection is an
  **extension allowlist** (`sh`, `ps1`, `json`, `md`, `yaml`,
  `txt`) plus PE/ELF magic-byte sniffing — explicitly **not** a
  POSIX-mode check (`cheaper.sh` is legitimately `100755`).
- Byte-level asserts: `cheaper.ps1` starts `EF BB BF` (UTF-8 BOM)
  and contains CRLF; `cheaper.sh` has `#!/bin/sh` shebang and LF
  endings; git index mode of `cheaper.sh` is `100755`
  (`git ls-files -s`, conditional when index info is available).
- Launcher↔hooks.json consistency folded in here (not a new
  script): every POSIX command references `sh ./cheaper.sh`,
  every `powershell` field references `./cheaper.ps1`, both files
  exist, and no bare `./cheaper` reference remains in hooks.json.
- Drop the `cli/version.json` canonical-version comparison (that
  declaration site disappears).

### `scripts/check-version.mjs` / `scripts/set-version.mjs` /
`scripts/lib/release-artifacts.mjs`

- `check-version.mjs`: remove the `cli/version.json` declaration
  entry (lines ~30-33), its header-comment line, and the
  now-unused local `readJsonVersion` lambda (+ `readFileSync`
  import if orphaned). Remaining declarations: Cargo workspace
  version ↔ `packages/apm/apm.yml`.
- `set-version.mjs`: remove "run `node scripts/bundle-cli.mjs` to
  regenerate version.json" guidance (the file is gone — `build.rs`
  generates version into `OUT_DIR` at compile time).
- `lib/release-artifacts.mjs`: keep only
  `cargoWorkspaceVersion`, `apmPackageVersion`,
  `setCargoWorkspaceVersion` — the exports actually imported by
  `check-version.mjs`/`set-version.mjs` (both define their own
  `repoRoot`; `check-apm-payload.mjs` takes repoRoot as a
  parameter). Delete `verifyReleaseVersion`, `repoRoot`, and all
  release target/artifact/SHA256SUMS/sibling helpers.

### `mise.toml` `tasks.check` + `scripts/check-lockfile.mjs` (new)

The consumer `--locked` install contract needs a **committed-state**
lockfile check, not a working-tree one: `mise run setup` runs
`cargo build`, and clippy/test also regenerate a stale lockfile —
so a working-tree guard placed after them is a no-op in CI
(review P1). New `scripts/check-lockfile.mjs` (~30 lines) does
both legs:

1. `cargo metadata --locked --format-version 1` (stdio piped —
   the JSON blob must not spam check output): fails if the
   committed `Cargo.lock` is inconsistent with the manifests.
2. `git diff --exit-code -- Cargo.lock`: fails if earlier build
   steps regenerated the lockfile — i.e. the committed file
   drifted.

Placed as the last step before `check-apm-payload.mjs` (or final):
all cargo commands have run by then, so leg 2 catches real drift.

```
biome check → cargo fmt --check → cargo clippy →
node scripts/check-version.mjs → node scripts/check-doc-ids.mjs →
cargo test --workspace → node --test scripts/test/ →
node --test benchmarks/test/ →
node scripts/check-lockfile.mjs → node scripts/check-apm-payload.mjs
```

Removed: `node scripts/bundle-cli.mjs` regeneration and the
`git diff --exit-code -- packages/apm/.apm/hooks/cli` freshness
step. Description updated ("payload check" not "bundle
freshness").

### Deletions

Scripts (7): `bundle-cli.mjs`, `lib/apm-bundle.mjs`,
`build-apm-bundle.mjs`, `assemble-release-bundle.mjs`,
`generate-release-checksums.mjs`, `check-binary-size.mjs`,
`smoke-release.mjs`.
Tests (4): `launcher-acquisition.test.mjs`,
`binary-size.test.mjs`, `release-artifacts.test.mjs`,
`release-workflow.test.mjs`; plus their fixture
`scripts/test/fixtures/launcher-fixture.cs`; drop the
`cargoBinary` helper in `scripts/test/lib/apm-fixture.mjs` —
**keep `nodeFreePath`**, it is reused by the new passthrough test
to provide a controlled minimal PATH.
Workflows (3): `build-executables.yml`, `release-acquisition.yml`,
`release.yml`.
Tree: `packages/apm/.apm/hooks/cli/`; `.gitignore` rule
`/packages/apm/.apm/hooks/cli/bin/*/` **and its comment line**.

### Test rewrites and additions

- `apm-payload.test.mjs` (rewrite): required-asset list becomes
  `cheaper.sh`/`cheaper.ps1`/`hooks.json`; new asserts — `cli/`
  absent, no binary assets, `sh ./cheaper.sh`/`./cheaper.ps1`
  references resolve with no bare `./cheaper`, BOM/CRLF/LF/shebang
  byte checks, exec-bit via `git ls-files -s` → `100755`.
  Explicitly remove: the `git check-ignore` block on
  `cli/bin/*` (the rule it asserts is deleted → it would fail),
  and the fixture `Cargo.toml` copy used only for the dropped
  version.json comparison.
- `apm-install-targets.test.mjs` (rewrite): deployed bundle
  asserts switch `cheaper` → `cheaper.sh`; assert `cli/` absent;
  keep per-target layout/skill/generated-config asserts; deployed
  exec-bit assert where the platform exposes mode (conditional —
  POSIX `statSync.mode` is not observable on Windows).
- `scripts/test/launcher-passthrough.test.mjs` (new): fake-PATH
  stub `cheaper` proves argv/stdin/stdout/exit-code passthrough
  for `cheaper.sh` (and `cheaper.ps1` under pwsh/powershell where
  available — this Windows host has real `powershell.exe`);
  empty-PATH run asserts stderr guidance + empty stdout + exit 1.
  Reuses `nodeFreePath` for a controlled minimal PATH.

### Docs

- `docs/guides/distribution.md`: rewrite to cargo-source-install
  model; version-declaration table drops `cli/version.json`.
- `docs/guides/release.md`: shrink to the `set-version.mjs` bump +
  `cargo install --git … --force` update flow; the
  release-acquisition body is removed.
- `docs/guides/parent-agent-integration.md`: update install/
  launcher description; remove `#offline-preseed` link target.
- `docs/guides/workers.md`, README "Parent-agent 連携", `AGENTS.md`
  (binary-free contract + `cli/` payload paragraph), `CONTRIBUTING.md`
  (bundle pipeline), comments in `Cargo.toml:12-15`,
  `crates/cheaper-cli/src/embedded.rs:25-26`,
  `packages/apm/apm.yml:3` description.
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`: "Locating the
  CLI" → preflight `cheaper --version`; missing → install command +
  PATH guidance; remove sibling/Release/cache guidance and the
  obsolete deployed-`cli/` limitation notes.

### Upstream supersede marking

Per spec scope item 6 (extended by review): flip status →
`superseded` + reciprocal `superseded-by` for docs whose contract
is fully removed; **amend** (add a superseded-clause note) for
docs whose core decision survives but one clause is deleted —
`docs/specs/unified-version-source-of-truth.md` (Cargo stays
canonical; the `cli/version.json` secondary-declaration clause is
removed) and `docs/designs/cheaper-hook-subcommands-replacing-
node-based-hook-adapters.md` (hook subcommands stay; the
`./cheaper`/`cli/bin` wiring clauses are removed). Newly
enumerated by review, added to the spec: ADR
`remove-the-committed-esbuild-cli-payload-and-move-the-canonical-
version-to-cargo` (mandates bundle-cli + cli/version.json),
test-spec `version-pinned-launcher-integrity-and-stream-
preservation` (its own Purpose says it retires exactly when the
acquisition contract is abolished), design
`payload-untracking-migration`, and test-specs
`embedded-builtin-resource-resolution-across-distribution-forms` /
`standalone-executable-contract-parity` where they assert removed
distribution forms. `docs/designs/overview.md`'s index table is
updated alongside the per-dir READMEs. A final
`rg 'bundle-cli|cli/version\.json|cli/bin|\./cheaper' docs/`
sweep at implementation time confirms nothing asserting removed
behavior remains unmarked. Already done at briefing: the
version-pinned design and the three accepted ADRs.

## Data and Control Flow

### Hook invocation (installed CLI)

```
IDE hook → hooks.json entry → ./cheaper.sh (or cheaper.ps1)
  → command -v cheaper → exec cheaper "$@"
  → Cargo-installed binary (embedded workers+policy)
  → stdout/stdin/argv/exit-code passthrough
```

### Hook invocation (missing CLI)

```
./cheaper.sh → command -v cheaper fails
  → stderr: install command + PATH guidance; exit 1; stdout empty
  → copilot: `|| exit 0` swallows; cursor: `|| echo allow-json`;
    windsurf/claude/codex: nonzero propagates (documented
    preflight failure, not silent acquisition retry)
```

## Non-Functional Constraints

- No new runtime dependencies; no Rust source changes.
- POSIX launcher must remain `#!/bin/sh`-clean (no bashisms);
  PowerShell must run under Windows PowerShell 5.1 (BOM+CRLF, no
  PS7-only syntax) and pwsh.
- No network access, no installs, no prompts at hook time.
- `cargo install --git` must succeed with `--locked` ⇒ committed
  `Cargo.lock` consistency is enforced by the new check step.

## Risks and Trade-offs

- **Exec bit loss through `git mv` on Windows**: mitigated by
  post-rename `git ls-files -s` verification and a test-time
  mode assert; `git update-index --chmod=+x` as the fix-up.
- **APM deploy stripping file modes**: verify via temp-consumer
  install during implementation; if stripped, document the
  workaround (target conventions still invoke `./cheaper.sh` via
  the shell — recorded as an accepted limitation if observed, not
  a silent failure).
- **Stale deployed payload in existing consumers**: reinstall
  verification shows whether APM prunes removed files; if not,
  document the inert-remnant behavior (old launcher is never
  invoked because hooks.json now references `cheaper.sh`).
- **Lockfile drift breaking consumer `--locked` installs**: new
  `cargo metadata --locked` check step.
- **Deleted-script callers missed**: the discovery + review sweep
  greps cover docs/tests/CI; final `rg` pass in implementation
  confirms zero references before commit.

## Deferred Design Concerns

- Sibling `cli/bin/` dev convenience — rejected scope (devs use
  `cargo install --path`).
- Registry-ref deployment naming follow-up (from #87 chain) —
  rechecked after this lands.
- Whether the launcher should print a distinct message when
  `cheaper` exists but fails to start — left to stderr passthrough
  of the CLI's own error (launcher only guards absence).

## Verification Notes

- `node scripts/test/apm-payload.test.mjs` + install-targets test
  (path dep, temp consumer): deploy `cheaper.sh`/`cheaper.ps1`,
  no `cli/`, exec bit.
- Fake-PATH stub: `cheaper.sh` forwards argv/stdin/stdout/exit
  code; absent-`cheaper` PATH → stderr guidance, empty stdout,
  exit 1. Same for `cheaper.ps1` under pwsh/powershell where
  available (Windows host: real powershell.exe available).
- `mise run check` full pass incl. new lockfile guard.
- `cargo install --path crates/cheaper-cli --locked` →
  `cheaper --version` / `cheaper models` smoke.
- `rg` sweeps scoped to **live files only** — `packages/apm/`,
  `scripts/`, `.github/`, `docs/guides/`, `README.md`,
  `AGENTS.md`, `CONTRIBUTING.md`, `Cargo.toml`, `crates/`, and
  non-superseded specs/designs/test-specs — no bare `./cheaper`,
  no `hooks/cheaper\b`, no `cli/version.json`, no `bundle-cli`
  references. Superseded/historical docs (tasks, plans,
  impl-records, retired test-specs) keep their historical content
  and are excluded by scope, not edited.
- `git diff --check`.

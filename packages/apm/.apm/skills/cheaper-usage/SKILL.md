---
name: cheaper-usage
description: How to invoke the cheaper CLI from a parent coding agent — commands, stdin/stdout contract, envelope fields, exit codes, and provider/model configuration. Read this before calling any cheaper command.
---

# cheaper CLI usage

`cheaper` is a Worker Runtime that delegates mechanical or large-context work
to cheaper models. You call it as a one-shot CLI process; each run reads a
JSON task from stdin and writes a single result envelope to stdout.

## Commands

```bash
cheaper <command> [options]

  run <worker>           Run a named worker (or --worker-file <path>)
  bulk-read              Alias for: run bulk-reader
  code-write             Alias for: run code-writer
  doc-write              Alias for: run doc-writer
  count-tokens <path>    Count canonical tokens of a workspace file
  auth status [provider] Show stored credentials and provider readiness
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential
```

| Option | Meaning |
|---|---|
| `--input <file\|->` | Task input JSON (default: stdin) |
| `--worker-file <path>` | Run an explicit worker file (`run` only) |
| `--cwd <path>` | Workspace root (default: current directory) |
| `--timeout <duration>` | Narrow policy timeout (e.g. `30s`, `5m`) |
| `--method <id>` | Auto-answer auth login select prompts (e.g. `device`) |
| `--type <oauth\|api_key>` | Auth method for `auth login` (default: `oauth`) |
| `--format <json\|text>` | Output format (default: `json`) |
| `--quiet` | Suppress stderr diagnostics |

`count-tokens` never starts a model. It returns the canonical token count of
one workspace file so you can measure context cost before reading it.

## Locating the CLI

- Preflight: run `cheaper --version`. If it prints a version, invoke
  `cheaper` directly — the hook launchers resolve the same PATH.
- If `cheaper` is not on `PATH`, install it from source (Cargo source
  install — there is no Release download, cache, or bundled binary):

  ```bash
  cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli
  # update: add --force to the same command
  ```

  Prerequisites: Rust toolchain ≥ 1.95 (the `rig` dependency requires
  it), `git`, and an OS C toolchain (MSVC build tools on Windows).
  Ensure Cargo's bin directory is on the IDE/agent `PATH`
  (`~/.cargo/bin` on POSIX, `%USERPROFILE%\.cargo\bin` on Windows) and
  restart the IDE/agent so it inherits the updated PATH. Verify with
  `cheaper --version`, then `cheaper models` as a smoke check.
- In a workspace where cheaper was installed via APM, the deployed
  hook bundle lives under `<hooks_root>/<dep-name>/`, where
  `<dep-name>` is the name the dependency was installed under —
  the basename for `path:` dependencies, or the last path segment
  of a virtual package reference. For the canonical install
  (`scarecrow173/cheaper/packages/apm`) that name is `apm`:

  ```bash
  .windsurf/hooks/apm/cheaper.sh <command> [options]               # POSIX (via sh)
  powershell .windsurf/hooks/apm/cheaper.ps1 <command> [options]  # Windows
  ```

  Per-target hook bundle roots:

  | Target | Hook bundle root |
  |---|---|
  | Windsurf | `.windsurf/hooks/<dep-name>/` |
  | Claude Code | `.claude/hooks/<dep-name>/` |
  | OpenAI Codex | `.codex/hooks/<dep-name>/` |
  | Cursor | `.cursor/hooks/<dep-name>/` |
  | GitHub Copilot | `.github/hooks/scripts/<dep-name>/` |

  If you don't know the deployed bundle directory, read the
  generated hook config — its `command` entries carry the real path
  (`.windsurf/hooks.json`, `.claude/settings.json`,
  `.codex/hooks.json`, `.cursor/hooks.json`, or
  `.github/hooks/<dep-name>-hooks.json` depending on target, plus
  `apm-hooks.json` duplicates on some targets; Claude commands may
  contain a `${CLAUDE_PROJECT_DIR}` placeholder).

  The bundle contains only thin launchers — `cheaper.sh` (POSIX)
  and `cheaper.ps1` (Windows) — which resolve `cheaper` from `PATH`
  and forward argv, stdin, stdout, and the exit code verbatim. They
  never download, cache, or install anything; when the CLI is
  missing they print the install command and PATH guidance to
  stderr and exit nonzero. Hook registrations invoke
  `cheaper hook <event> --target <agent>` through these launchers
  (POSIX entries run `sh ./cheaper.sh`, so a missing executable bit
  does not break the hook).
- The large-read hook is functional on these targets (all routed
  through `cheaper hook <event> --target <agent>`; Node.js is not
  required):
  - **Windsurf** — native `pre_read_code` event; oversized reads are
    blocked (exit 2) with a redirect to `bulk-read`. `pre_run_command`
    also blocks large reads inside `cat`/`head`/`tail`/`less`/`more`
    shell commands.
  - **GitHub Copilot** — `preToolUse`; oversized `view` and shell-tool
    calls are denied via `{"permissionDecision":"deny"}` on stdout
    with exit 0.
  - **Claude Code** — `PreToolUse` (`Read`/`Bash`/`PowerShell`
    matcher); denies return `hookSpecificOutput.permissionDecision:
    "deny"` on stdout with exit 0.
  - **Cursor** — `beforeReadFile` / `beforeShellExecution`; every
    response is a schema-valid `{"permission": ...}` JSON, exit 0.
- On **OpenAI Codex** the same `PreToolUse` registration is deployed
  and shell reads (`Bash`/`PowerShell`) are covered, but file-read
  tools do not emit `PreToolUse` (upstream gap `openai/codex#18491`),
  so file reads cannot be intercepted.
- Hook executions do not write to `usage.jsonl`; only explicit
  `cheaper run`/`count-tokens` calls are recorded.
  For the per-target contract, see the canonical matrix in
  DISC-034r05iHy2gdnpdai9KGpW
  (`docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`).

## stdin → stdout contract

Worker runs read exactly one JSON document from stdin matching the worker's
input schema, and write exactly one envelope line to stdout:

```json
{
  "schemaVersion": 1,
  "ok": true,
  "worker": "bulk-reader",
  "result": { "...": "worker output, conforming to its output schema" },
  "meta": {
    "model": { "provider": "...", "id": "...", "tier": "low" },
    "changes": { "files": [], "totals": { "files": 0, "linesAdded": 0, "linesDeleted": 0 } },
    "usage": { "totalTokens": 0 },
    "turns": 1,
    "durationMs": 0
  }
}
```

On failure the same envelope shape carries `ok: false` and `error`:

```json
{
  "schemaVersion": 1,
  "ok": false,
  "worker": "bulk-reader",
  "error": { "code": "TIMEOUT", "message": "...", "exitCode": 7, "retryable": true }
}
```

Key points:

- `meta.changes` is **measured by the runtime journal**, not reported by the
  model. Treat it as ground truth for what files a writer actually touched.
- Diagnostics (model line, progress) go to stderr, never stdout. With
  `--format json`, stdout is always a single parseable JSON line.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 2 | Invalid request or input (bad args, bad stdin JSON) — or a measured refusal such as `FILE_TOO_LARGE`; inspect `error.code` |
| 3 | Worker/schema/model-config resolution failure |
| 4 | Policy violation |
| 5 | Model tier unavailable or provider failure (retryable) |
| 6 | Tool execution or output validation failure |
| 7 | Timeout (retryable) |
| 8 | Aborted |
| 9 | Partial mutation — the worker wrote files but did not finish cleanly |
| 70 | Internal error |

Exit 2 alone does **not** mean "invalid input" — always inspect
`error.code` in the envelope. In particular `error.code ===
"FILE_TOO_LARGE"` (exit 2, non-retryable) is returned by `count-tokens`
when the file exceeds the measurement byte cap: an accurate token count
cannot be obtained. Do **not** retry `count-tokens` — treat the file as
large/unmeasurable and delegate it to `bulk-read`. `error.details`
carries `path`, `bytes`, and `maxBytes`. The cap defaults to 16 MiB and
can be overridden with the `CHEAPER_COUNT_MAX_BYTES` env var (positive
integer, bytes; invalid values fall back to the default).

## Provider and model selection

Worker runs resolve a model through cheaper's tier system (low/medium/high)
and project/user `models.yaml` overrides — you do not pass a model name.
`meta.model` reports what was actually used. Provider credentials are
stored user-scope in `<userConfigDir>/credentials.json` (managed via
`cheaper auth login` / `auth logout` / `auth status`) — never in the
project or `.cheaper/`. The model map is configured via
`models.yaml`; see `docs/guides/providers.md` in the cheaper repository.

## Delegation guidance

- Before reading a file you suspect is large, run
  `cheaper count-tokens <path> --cwd <workspace>` and check `result.tokens`.
- If `count-tokens` answers `ok: false` with `error.code ===
  "FILE_TOO_LARGE"`, skip straight to `bulk-read` — the file is too large
  to measure, so it is certainly too large to load into your context.
- Delegate large-file reading to `bulk-read` (see the `cheaper-bulk-read`
  skill), mechanical code changes to `code-write` (`cheaper-code-write`),
  and mechanical document work to `doc-write` (`cheaper-doc-write`).

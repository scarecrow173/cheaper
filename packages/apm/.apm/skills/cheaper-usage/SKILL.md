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

- If `cheaper` is on `PATH`, use it directly.
- In a Windsurf workspace where cheaper was installed via APM, the
  hook bundle ships a platform launcher plus standalone executables
  (no Node.js required):

  ```bash
  .windsurf/hooks/cheaper/cheaper <command> [options]          # POSIX
  pwsh .windsurf/hooks/cheaper/cheaper.ps1 <command> [options] # Windows
  ```

  The launcher dispatches to `cli/bin/<platform>/cheaper[.exe]`.
  If that sibling is absent on linux-x64, linux-arm64, or win32-x64, it
  acquires the Release tag pinned by `cli/version.json` into a user cache
  outside the hook directory (override: absolute `CHEAPER_CACHE_DIR`).
  It verifies the same-tag `SHA256SUMS`, binary SHA-256, and executable
  version on acquisition and cache hits; it never resolves `latest`.
  Offline preseed requires both the executable and the published manifest
  under `<cache>/<version>/<platform>/`. Missing version metadata, unsupported
  acquisition targets, or failed verification produce stderr and nonzero exit.
  Copilot's JSON deploy exclusion also removes version.json, so acquisition
  there requires a future deployable version source; keep the sibling payload
  until that migration is complete. PowerShell hook entries use cheaper.ps1
  through a child PowerShell process and retain their existing fallback policy.
  A JS bundle also ships at `cli/bin/cheaper.mjs`
  (`node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs
  <command> [options]`) as a fallback for environments where the
  platform executable is missing (e.g. an unsupported OS); it
  requires Node.js on `PATH`. Hook registrations invoke the
  launcher/executable path directly via `cheaper hook
  <event> --target <agent>`.
  Substitute `cheaper` for the actual hook bundle directory name if the
  package was installed under a different name. Everywhere below,
  `cheaper <args>` means either form.
- The large-read hook is functional on these targets (all routed
  through `cheaper hook <event> --target <agent>`; Node.js is not
  required):
  - **Windsurf** — native `pre_read_code` event; oversized reads are
    blocked (exit 2) with a redirect to `bulk-read`. `pre_run_command`
    also blocks large reads inside `cat`/`head`/`tail`/`less`/`more`
    shell commands.
  - **GitHub Copilot** — `preToolUse`; oversized `view` and shell-tool
    calls are denied via `{"permissionDecision":"deny"}` on stdout
    with exit 0. Note the known gap: APM does not deploy `.json`
    assets to the copilot target, so file-schema workers (`bulk-read`
    / `code-write` / `doc-write`) cannot run there until the
    upstream fix lands — `count-tokens` and the hook still work.
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
stored user-scope in `<userConfigDir>/pi/auth.json` (managed via
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

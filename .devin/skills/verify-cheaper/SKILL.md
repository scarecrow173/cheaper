---
name: verify-cheaper
description: Drive the real cheaper CLI binary against a scratch workspace to prove user-visible behavior — workflow catalog/listing, interface validation, tool-job runs, usage/history recording, and the claims skill SKILL.md files make about commands. Use before merging CLI/workflow changes, when a reported bug needs reproduction against the real binary, or when a skill's command examples must be re-proven after a refactor. Not for unit-test coverage — this drives the shipped binary the way a user would.
---

# Verify cheaper

`cheaper` is a one-shot CLI (Rust, `cheaper-cli`). There is no server,
no daemon, no port. Every drive is a separate process invocation whose
entire observable contract is the JSON envelope on stdout, the exit
code, and any files it writes. A "run" is cheap; make many.

## Launch

Build once per session (or reuse an existing debug binary — check
`target/debug/cheaper.exe --version` first):

```bash
mise exec -- cargo build -p cheaper-cli
```

Binary path: `target/debug/cheaper` (POSIX) / `target/debug/cheaper.exe` (Windows).

Each drive needs two isolation roots so it never touches the operator's
real config or workspace:

```bash
VROOT="target/verify-evidence/$(date +%Y%m%d-%H%M%S)"
WS="$VROOT/ws"          # scratch workspace -> becomes --cwd
CFG="$VROOT/config"     # scratch config dir -> becomes CHEAPER_CONFIG_DIR
mkdir -p "$WS" "$CFG"
export CHEAPER_CONFIG_DIR="$CFG"
BIN="$PWD/target/debug/cheaper"
```

Ready condition: `"$BIN" --version` prints a semver and exits 0.
Teardown: nothing to kill — `cheaper` exits per invocation. `workflow
run` spawns short-lived child processes only for `command`/`process`
jobs, which this skill never drives without grants anyway.

## Doctor

One read-only check answering "is this binary worth driving?"

```bash
"$BIN" --version                       # prints a bare semver (NOT JSON), exit 0
"$BIN" workflow list --cwd "$WS"       # utility command, ok:true envelope, exit 0
```

If `--version` fails, the binary is stale or broken — rebuild. If a
utility command errors, `CHEAPER_CONFIG_DIR`/`--cwd` is miswired
(pointing at the operator's real config is the dangerous case — check
the env var resolves inside `$VROOT`).

## Drive

Invoke `"$BIN" <command> --cwd "$WS"` and parse the envelope:

- `{"schemaVersion":1,"ok":true,"worker":null|<name>,"result":{...},"meta":{...}}`
  — `ok:true` + exit 0 is success; read `result`, not `meta`.
- `{"schemaVersion":1,"ok":false,"error":{"code","message",...}}` —
  fail-closed; exit is non-zero (`2` input/request, `6` policy).

Stable handles (assert on these, not on prose text):

| Surface | Handle |
|---|---|
| Success | `"ok":true`, exit 0 |
| Error | `error.code` (`INVALID_INPUT`, `INVALID_REQUEST`, `POLICY_VIOLATION`, `WORKFLOW_*`) |
| Catalog | `result.catalog.present`, `result.workflows[].status` (`ok`/`missing`/`invalid`/`duplicate`), `result.diagnostics[].kind` |
| Interface | `result.inputs[name].required`, `result.outputs[name].schema` |
| Run | `"workflow":true` marker; `result` IS the outputs map (`result.<outputName>`); `jobs[].status` (`succeeded`/`failed`/`skipped`) at top level, not under `result` |
| Run side effect | `$CFG/history/<uuid>/{started,terminal}.json` — lifecycle records (`kind:"workflow"`/`"job"`, `event`, `status`, `durationMs`) per executed job |
| Usage side effect | `$CFG/usage.jsonl` lines (`kind:"run"|"count"|"route"`) — only for usage-bearing records, never for `list`/`interface`/`route check`/`history`/`usage` |

Rules of engagement:

- Grant-needing jobs (`"kind":"tool","tool":"command"|"process"`) are
  expected to fail with `POLICY_VIOLATION` + exit 6 in the scratch
  workspace — that failure IS the proof that grants gate execution.
  Do not create `.cheaper/policy.yaml` grants to make them pass unless
  the feature under test is the grant path itself.
- `workflow run` takes an explicit file path — there is intentionally
  no `workflow run <catalog-id>` lookup. Assert that gap, don't
  work around it.
- Skill-claim verification (SKILL.md docs ship in `packages/apm/.apm/skills/`):
  extract each ` ```bash ` block and every error-table `error.message`
  row, run them against `$BIN` in `$WS`, and diff observed envelope
  fields/`error.code` against what the skill claims. A skill claim that
  fails on the real binary is a doc bug — report file:line, don't
  patch the claim to match.
- Feature files under `features/` map the user-facing surface; drive
  the journey they name, not a shortcut that happens to pass.

## Evidence

Capture to `$VROOT/evidence/` (inside `target/` — gitignored, survives
cleanup, never pollutes the repo):

1. `commands.txt` — every invocation verbatim, one per line.
2. `<seq>-<command-slug>.json` — raw stdout envelope per drive.
3. `exitcodes.txt` — `<seq> <exit>` pairs.
4. `side-effects.txt` — `find "$WS" "$CFG" -type f` after the run
   (proves what the command wrote — or wrote nothing for read-only).
5. `result.md` — verdict per feature file driven, with the envelope
   fields observed (measured, not asserted).

Proof standards: drive the real command a user runs (not a test seam
or internal helper); capture the action AND the resulting state
(`$CFG/usage.jsonl`, `$WS/.cheaper/` contents); verify side effects by
looking at files, not by trusting `ok:true`.

## Cleanup

```bash
# Nothing to kill — cheaper is one-shot.
# Evidence MUST survive: only remove the scratch workspace+config, keep $VROOT/evidence.
rm -rf "$VROOT/ws" "$VROOT/config"
ls "$VROOT/evidence/"   # confirm commands.txt, envelopes, result.md remain
```

If a drive wedged mid-run (never observed for tool jobs; possible for
`command`/`process` jobs), `tasklist //FI "IMAGENAME eq cheaper.exe"`
on Windows or `pgrep -fl cheaper` on POSIX — kill only PIDs whose
command line contains your `$VROOT` path.

## Helpers

No shipped scripts — the drives are single binary invocations. Feature
journeys in `features/` carry their exact commands inline.

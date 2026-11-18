# workflow run

Execute a Workflow JSON v1 file by explicit path. Tool jobs that need
no exec grant (`count_tokens`, `grep`, `find`, `read`, `ls`) run in the
scratch workspace; `command`/`process` jobs must fail
`POLICY_VIOLATION`. Runs write lifecycle records under
`$CFG/history/<uuid>/{started,terminal}.json` (`kind:"workflow"`/`"job"`,
`event`, `status`) — not `usage.jsonl`, which is for usage-bearing
records only.

## Sub-features

- Tool-only workflow → `ok:true`, `"workflow":true`, `result` is the
  outputs map directly (`result.<name>`), `jobs[].status` at top level
  (`succeeded`/`failed`/`skipped`)
- `--input` as file path or `-` (stdin); inline JSON string is rejected
- Grant-needing job → `POLICY_VIOLATION` + exit 6 (proves grants gate
  execution — the catalog gives no execution rights)
- Named lookup does not exist: `workflow run <id>` fails (file-not-found
  class error), not a catalog resolution

## How to get to it (user POV)

A user points at a workflow file explicitly:
`cheaper workflow run flows/peek.json --input @inputs.json --cwd proj`.
The path is the authority — nothing else grants execution.

## Driving it with the CLI

```bash
cd "$WS"   # run resolves relative workflow paths from --cwd
"$BIN" workflow run .cheaper/workflows/flows/peek.json --cwd "$WS"
# → ok:true, workflow:true, result.n = token count of main.rs,
#   jobs[0].status "succeeded"

"$BIN" workflow run .cheaper/workflows/flows/peek.json --input - --cwd "$WS" <<'EOF'
{}
EOF

# grant boundary: a command job must fail closed
cat > "$WS/needs-grant.json" <<'EOF'
{"schemaVersion":1,"inputs":{},"jobs":{
 "exec":{"kind":"tool","tool":"command",
         "input":{"name":{"$literal":"cargo-test"},
                  "args":{"$literal":[]}}}},
 "outputs":{"r":{"schema":{},"value":{"$from":"exec","path":""}}}}
EOF
"$BIN" workflow run needs-grant.json --cwd "$WS"
# → ok:false, error.code POLICY_VIOLATION, exit 6

# no named lookup
"$BIN" workflow run peek --cwd "$WS"
# → error (cannot read workflow file), NOT a catalog hit
```

Verify side effect: `find "$CFG/history" -name terminal.json` grows by
two per run (workflow + job records); `usage.jsonl` gains nothing from
tool-only runs; read-only commands add nothing to either store.

## Gotchas

- `command`/`process` `input.name` is a **policy grant name**, not an
  executable name — `cargo-test` is the built-in grant the repo's own
  policy.yaml ships, absent in scratch (hence POLICY_VIOLATION).
- Job outputs are materialized as `null` when a `when`-conditioned job
  is skipped — a non-nullable output schema then fails `INVALID_OUTPUT`
  at run time even though `interface` passed.

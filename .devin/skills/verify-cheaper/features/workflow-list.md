# workflow list

Read the project workflow catalog at `<ws>/.cheaper/workflows/catalog.json`
and return entries with per-entry status and diagnostics. Read-only:
no workflow file contents, no usage record, no writes.

## Sub-features

- Absent catalog → `catalog.present:false`, empty lists, still `ok:true`
- Valid entries → `status:"ok"` with `{id,path,summary,tags}`
- Per-entry diagnostics: `missing-path`, `duplicate-id`, `malformed-entry`,
  `path-out-of-workspace`, `denied-path`, `not-regular-file`
- Manifest-level failure (bad JSON, wrong `schemaVersion`, oversized)
  → `INVALID_INPUT` error envelope + non-zero exit
- `--limit` truncates `workflows` only; `truncated:true`; diagnostics intact

## How to get to it (user POV)

A user (or parent agent) runs `cheaper workflow list --cwd <project>`
to discover what workflows exist before deciding to reuse one.

## Driving it with the CLI

```bash
# absent catalog
"$BIN" workflow list --cwd "$WS"          # ok:true, catalog.present:false

# seed a catalog + one real workflow file
mkdir -p "$WS/.cheaper/workflows/flows"
cat > "$WS/.cheaper/workflows/flows/peek.json" <<'EOF'
{"schemaVersion":1,"inputs":{},"jobs":{
  "peek":{"kind":"tool","tool":"count_tokens",
          "input":{"path":{"$literal":"main.rs"}}}},
 "outputs":{"n":{"schema":{"type":"number"},
                 "value":{"$from":"peek","path":"/details/tokens"}}}}
EOF
cat > "$WS/.cheaper/workflows/catalog.json" <<'EOF'
{"schemaVersion":1,"workflows":[
 {"id":"peek","path":"flows/peek.json","summary":"count tokens","tags":["demo"]},
 {"id":"ghost","path":"flows/nope.json","summary":"missing file"},
 {"id":"peek","path":"flows/peek.json","summary":"dup id"}]}
EOF
echo 'fn main(){}' > "$WS/main.rs"

"$BIN" workflow list --cwd "$WS"          # statuses: duplicate, missing, duplicate
"$BIN" workflow list --cwd "$WS" --limit 1  # truncated:true
```

## Gotchas

- Duplicate `id` marks **every** affected entry `duplicate` — there is
  no last-wins.
- `catalog.json` paths are relative to `.cheaper/workflows/`, not the
  workspace root.
- `workflow list` is a utility command: it must not append to
  `$CFG/usage.jsonl` — check `side-effects.txt`.

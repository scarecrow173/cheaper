# workflow interface

Statically validate a Workflow JSON v1 file and print its public
contract (inputs with `required` flags, outputs with schemas) — without
exposing private `jobs`/prompts/bindings. The reuse-inspection path for
cataloged workflows.

## Sub-features

- Valid file → `ok:true`, `result.inputs`, `result.outputs`, exit 0
- Malformed / schema / reference failures → `ok:false`, `error.code`,
  `error.message`, exit 2
- Public contract only: `result` must not contain `jobs`, `prompt`,
  `binding`, `worker` internals

## How to get to it (user POV)

Before reusing a cataloged workflow, a user/agent runs
`cheaper workflow interface <path>` to see what inputs it needs and
what it returns — instead of reading the whole private definition.

## Driving it with the CLI

```bash
# reuse the peek.json seeded by the workflow-list journey (or re-seed)
"$BIN" workflow interface "$WS/.cheaper/workflows/flows/peek.json" --cwd "$WS"
# → ok:true; inputs:{} ; outputs.n.schema:{type:number}

# negative battery — each must exit 2 with a specific error.code
echo '{bad' > "$WS/bad.json"
"$BIN" workflow interface "$WS/bad.json" --cwd "$WS"
# → INVALID_INPUT, "workflow file must contain valid JSON"

cat > "$WS/cycle.json" <<'EOF'
{"schemaVersion":1,"inputs":{},"jobs":{
 "a":{"kind":"tool","tool":"count_tokens","needs":["b"],
      "input":{"path":{"$literal":"x"}}},
 "b":{"kind":"tool","tool":"count_tokens","needs":["a"],
      "input":{"path":{"$literal":"x"}}}},
 "outputs":{"o":{"schema":{},"value":{"$from":"a","path":""}}}}
EOF
"$BIN" workflow interface "$WS/cycle.json" --cwd "$WS"
# → error containing "workflow dependency cycle"
```

## Gotchas

- `{"$from":"x"}` without `path` reports differently by position:
  `invalid Workflow JSON v1` (schema, oneOf) under `outputs[].value`
  vs `invalid binding wrapper` inside a job `input`.
- Validation proves structure only — it does NOT prove grants exist or
  the run will succeed. That boundary is by design; don't treat a pass
  as run-readiness.

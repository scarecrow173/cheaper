---
name: cheaper-workflow-create
description: Design, statically validate, and register a reusable Workflow JSON v1 definition (`.cheaper/workflows/*.json` + project catalog) when the user explicitly asks to create a workflow or approves a discover suggestion. Verified with `cheaper workflow interface`; never runs the workflow or changes grants, policy, or routes.
---

# Create a cheaper workflow

Author a **Workflow JSON v1** definition — a saved DAG of tool calls and
worker runs with a typed public input/output contract — and register it
in the project workflow catalog so it can be discovered and invoked by
explicit path later.

## When to use

- The user **explicitly asks to create a workflow** ("make a workflow
  that…", "save this as a reusable workflow"), or
- The user **approves a suggestion** produced by the
  `cheaper-workflow-discover` skill.

Do **not** persist anything on candidate discovery alone. Do **not**
wrap work that a single tool call or a single `cheaper run <worker>`
handles fine — a workflow earns its place when you need a stable public
interface over multiple steps (fan-out/fan-in, tool→worker chains,
conditional jobs), not for one-shot operations.

## Approval boundary

- **Saving a definition is not running it.** Never launch
  `cheaper workflow run` or any side-effecting smoke (command/process
  grants, file writes, billable model calls) without explicit user
  approval. This skill's own writes are limited to the workflow file and
  the catalog entry.
- **Never modify grants, policy, or routes** to make a definition work.
  If the definition needs a capability the policy doesn't allow
  (e.g. a `command`/`process` grant name), say so and let the user
  decide — any grant proposal is the user's to approve.
- **Ask before overwriting** an existing workflow file or replacing a
  catalog entry; keep catalog edits as reviewable diffs, never silent
  last-wins.

## Authoring procedure

1. **Pin the public contract.** From the request (or the approved
   discover proposal), decide: which inputs callers supply, which outputs
   the workflow returns, and what side effects it has. Keep the contract
   small — intermediate job results stay private to the DAG.
2. **Compare against the catalog.** Run `cheaper workflow list --cwd
   <workspace>`. If an existing entry covers the need, prefer reusing or
   amending it (an amend is a normal file edit the user reviews) over
   adding a near-duplicate.
3. **Choose the minimal shape.** One worker job beats a forced DAG. A
   direct tool call beats a workflow that only wraps that tool.
4. **Assemble the JSON v1** (anatomy below): `schemaVersion: 1`,
   `inputs` (JSON Schema per input; `default` marks it optional),
   `jobs` (each `kind:"tool"|"worker"` with the matching name field and
   `input`), `outputs` (`{schema, value}` per output).
5. **Pick the path.** Default convention:
   `.cheaper/workflows/<purpose>.json` under the workspace root
   (kebab-case purpose). The loader resolves paths inside the canonical
   `--cwd` workspace boundary — confirm before overwriting an existing
   file.
6. **Validate.** Run `cheaper workflow interface <path> --cwd
   <workspace>` and require `ok: true` (see "Validate" below). Fix and
   re-validate on failure — never deliver an invalid definition.
7. **Register** in `.cheaper/workflows/catalog.json` (see "Register").
8. **Confirm** with `cheaper workflow list --cwd <workspace>` — your
   entry should report `status: "ok"`.
9. **Report** the saved path, contract summary, validation result, and
   invocation examples (see "Report back").

## Workflow JSON v1 anatomy

Top level — exactly these four keys, all required:

| Field | Shape |
|---|---|
| `schemaVersion` | literal `1` |
| `inputs` | object: input name → JSON Schema object |
| `jobs` | object: job id → job object |
| `outputs` | object: output name → `{schema, value}` |

### `inputs`

Each input is a JSON Schema object. **Presence of `default` makes the
input optional; no `default` means required.** The default must satisfy
its own schema and is applied at the top level only — a
`"default": null` under a nullable schema is a valid optional input.

### `jobs`

Each job is one of two shapes (`kind` selects which name field is used):

```json
"someJob": { "kind": "tool",   "tool":   "<tool-name>",   "input": {…},
             "needs": ["otherJob"], "when": {…}, "resources": {…} }
"otherJob": { "kind": "worker", "worker": "<worker-name>", "input": {…} }
```

- `tool`/`worker` names must resolve. Canonical tools: `read`, `write`,
  `edit`, `ls`, `grep`, `find`, `count_tokens`, `process`, `command`.
  Built-in workers: `bulk-reader`, `code-writer`, `doc-writer`,
  `condense`, `tool-operator`. Custom workers resolve from
  `.cheaper/workers/` (project) then the user config dir — only name
  workers that actually exist there; **never invent job, tool, or worker
  names**.
- `needs`: array of job ids — declares ordering without a data
  dependency (e.g. "the report job runs only after the build job
  finishes"). A `$from` binding already implies the edge, so `needs`
  is for ordering-only legs.
- `when`: predicate `{left, op, right?}`. `op` is `exists` (no `right`
  — true when `left` materializes non-null) or `eq`/`ne`/`gt`/`lt`
  (`right` required; `gt`/`lt` compare numbers, non-numeric operands
  make it false). Both operands accept bindings.
- `resources`: optional `{reads|writes|produces: [{path, scope}]}`
  declarations for run-time resource inference — `path` is a literal
  string or `{"$input": <name>}` (never `$from`), `scope` is `exact` or
  `subtree`. Omit it unless you need declared file access.

### Bindings

Bindings are exact object wrappers — usable anywhere inside a job
`input`, a `when` operand, or an output `value`:

| Wrapper | Meaning |
|---|---|
| `{"$input": "<name>"}` | Materialize the declared workflow input, JSON type preserved |
| `{"$from": "<jobId>", "path": "<pointer>"}` | Materialize a value from a finished producer's result; `path` is an **RFC 6901 JSON Pointer** into the result root (`""` = whole result). Also creates the dependency edge |
| `{"$literal": <any JSON>}` | Inject a literal; needed when the literal itself is an object containing a `$`-key that would be misparsed as a binding |

There is **no string interpolation** — a plain string is always a
literal. Any object containing `$input`/`$from`/`$literal` must be one
of the exact wrappers above (extra keys or a missing `path` are
`invalid binding wrapper`).

Pointer targets: every tool result is `{content:[{type:"text",text}],
details}` — `/content/0/text` reaches the primary text. Useful
`command` details: `/details/stdout/text`, `/details/exitCode`,
`/details/outputTokens`, `/details/logRef`, `/details/argv`; `process`
shares `/details/stdout/text` and `/details/exitCode`. Worker results
follow the worker's output schema (e.g. bulk-reader `/summary`,
`/findings/0/title`; tool-operator `/summary`). Pointers are checked
against the producer's declared output schema — impossible paths on
closed schemas fail statically.

### `when` skip semantics

A job whose `when` evaluates false is **condition-skipped**: it does
not run, and the skip does **not** cascade. Every `$from` binding to a
skipped producer materializes as `null`. Dependents with their own
`when` re-evaluate; dependents without `when` still run with `null`
inputs, and outputs bound from a skipped job materialize as `null` — so
give such outputs a nullable schema (`{"type": ["string", "null"]}`) or
the consumer its own `when`.

## Templates

Every template below is a complete, valid Workflow JSON v1 (verified
against `cheaper workflow interface`).

### 1. Minimal single worker

```json
{
  "schemaVersion": 1,
  "inputs": {},
  "jobs": {
    "analyze": {
      "kind": "worker",
      "worker": "bulk-reader",
      "input": { "task": "Analyze the reusable saved workflow fixture" }
    }
  },
  "outputs": {
    "summary": {
      "schema": { "type": "string" },
      "value": { "$from": "analyze", "path": "/summary" }
    }
  }
}
```

### 2. Inputs with defaults (required vs optional)

```json
{
  "schemaVersion": 1,
  "inputs": {
    "task": { "type": "string", "minLength": 1 },
    "paths": {
      "type": "array",
      "items": { "type": "string" },
      "default": []
    }
  },
  "jobs": {
    "read": {
      "kind": "worker",
      "worker": "bulk-reader",
      "input": {
        "task": { "$input": "task" },
        "paths": { "$input": "paths" }
      }
    }
  },
  "outputs": {
    "summary": {
      "schema": { "type": "string" },
      "value": { "$from": "read", "path": "/summary" }
    },
    "relevantFiles": {
      "schema": { "type": "array" },
      "value": { "$from": "read", "path": "/relevantFiles" }
    }
  }
}
```

### 3. Tool → worker chain

```json
{
  "schemaVersion": 1,
  "inputs": {
    "taskName": { "type": "string", "minLength": 1 }
  },
  "jobs": {
    "test": {
      "kind": "tool",
      "tool": "process",
      "input": { "task": { "$input": "taskName" } }
    },
    "interpret": {
      "kind": "worker",
      "worker": "tool-operator",
      "input": {
        "task": { "$literal": "Interpret the test output and suggest next steps" },
        "context": [{ "$from": "test", "path": "/details/stdout/text" }]
      }
    }
  },
  "outputs": {
    "summary": {
      "schema": { "type": "string" },
      "value": { "$from": "interpret", "path": "/summary" }
    }
  }
}
```

The `$from` binding already makes `interpret` depend on `test`; `needs`
is for ordering without data flow.

### 4. Fan-in (two producers, one consumer)

```json
{
  "schemaVersion": 1,
  "inputs": {
    "pattern": { "type": "string", "minLength": 1 },
    "glob": { "type": "string", "default": "*.rs" }
  },
  "jobs": {
    "search": {
      "kind": "tool",
      "tool": "grep",
      "input": {
        "pattern": { "$input": "pattern" },
        "glob": { "$input": "glob" }
      }
    },
    "locate": {
      "kind": "tool",
      "tool": "find",
      "input": { "pattern": { "$input": "glob" } }
    },
    "digest": {
      "kind": "worker",
      "worker": "tool-operator",
      "needs": ["locate", "search"],
      "input": {
        "task": { "$literal": "Summarize what the search and locate results show" },
        "context": [
          { "$from": "search", "path": "/content/0/text" },
          { "$from": "locate", "path": "/content/0/text" }
        ]
      }
    }
  },
  "outputs": {
    "summary": {
      "schema": { "type": "string" },
      "value": { "$from": "digest", "path": "/summary" }
    }
  }
}
```

`search` and `locate` have no dependency and may run concurrently;
`digest` fans in both result texts through `context`.

### 5. Conditional job (`when`) and skip tolerance

```json
{
  "schemaVersion": 1,
  "inputs": {
    "grantName": { "type": "string", "minLength": 1 },
    "args": { "type": "array", "items": { "type": "string" }, "default": [] },
    "tokenThreshold": { "type": "number", "minimum": 0, "default": 2000 }
  },
  "jobs": {
    "exec": {
      "kind": "tool",
      "tool": "command",
      "input": {
        "name": { "$input": "grantName" },
        "args": { "$input": "args" }
      }
    },
    "analyze": {
      "kind": "worker",
      "worker": "condense",
      "when": {
        "left": { "$from": "exec", "path": "/details/outputTokens" },
        "op": "gt",
        "right": { "$input": "tokenThreshold" }
      },
      "input": {
        "logRef": { "$from": "exec", "path": "/details/logRef" },
        "originalTokens": { "$from": "exec", "path": "/details/outputTokens" },
        "command": { "$from": "exec", "path": "/details/argv" }
      }
    }
  },
  "outputs": {
    "execResult": {
      "schema": { "type": "object" },
      "value": { "$from": "exec", "path": "" }
    },
    "summary": {
      "schema": { "type": ["string", "null"] },
      "value": { "$from": "analyze", "path": "/summary" }
    }
  }
}
```

When the command output stays under `tokenThreshold`, `analyze` is
skipped and `summary` is `null` — hence the nullable output schema.
`command` jobs name a policy grant (`name`), not an executable: if the
grant isn't authorized, describe the needed grant to the user instead
of running.

## Validate

Static validation is the `interface` command — it runs the real
compiler's definition validation plus the public-interface projection
and nothing else (no invocation inputs, no model/tool/worker runs).
A pass proves the definition is **structurally valid** — it does **not**
prove runtime success, that required grants/permissions are in place,
or that the policy will allow the jobs:

```bash
cheaper workflow interface <path> --cwd <workspace>
```

- `ok: true`, exit 0 → `result.inputs[name] = {schema, required}` and
  `result.outputs[name] = {schema}` are the public contract callers
  see. Check that `required` flags match your intent (missing `default`
  ⇒ `required: true`).
- `ok: false`, exit 2 → read `error.code`/`message`, fix, re-run.
  Common failures:

| `error.message` | Cause → fix |
|---|---|
| `workflow file must contain valid JSON` | Malformed JSON — fix syntax (`INVALID_INPUT`) |
| `invalid Workflow JSON v1` | Top-level/job/output shape broke the schema — check `details.errors[].instancePath` (e.g. `schemaVersion` ≠ 1, extra keys like `name`, missing `outputs[].value`) |
| `unknown tool: <name>` / `unknown worker: <name>` | Invented or misspelled name — use a real tool/worker |
| `unknown input reference: <name>` | `$input` points at an undeclared input |
| `unknown job reference: <name>` | `$from` points at a job id that doesn't exist |
| `unknown dependency: <a> -> <b>` | `needs` names a job id that doesn't exist |
| `invalid binding wrapper` | A `$`-key object isn't an exact wrapper (e.g. `{"$from":"x"}` missing `path`) |
| `invalid JSON Pointer: <ptr>` | `path` must start with `/` (or be `""`) and use `~0`/`~1` escapes only |
| `primitive output has no child` / `unknown closed output property` / `impossible array output path` | Pointer can't exist under the producer's output schema |
| `invalid input default: <name>` | `default` fails the input's own schema |
| `invalid workflow input schema` / `invalid workflow output schema` | The JSON Schema itself doesn't compile (unknown `type`, bad keywords) |
| `workflow dependency cycle` | `needs`/`$from` edges form a cycle |

## Register in the catalog

After validation, append one entry to
`<workspace>/.cheaper/workflows/catalog.json` (create the file with
`{"schemaVersion": 1, "workflows": []}` if absent):

```json
{ "id": "read-and-summarize", "path": ".cheaper/workflows/read-and-summarize.json",
  "summary": "Read workspace files with bulk-reader and return a summary",
  "tags": ["read", "summary"] }
```

- `id`: `[A-Za-z0-9][A-Za-z0-9._-]*`, ≤64 chars, unique in the catalog
  — a duplicate marks **both** entries `status:"duplicate"`, so pick a
  new id or get the user's OK to replace the old one.
- `path`: workspace-root-relative (≤512 chars, never absolute).
- `summary`: one sentence, ≤500 chars. `tags`: optional, ≤16 strings.
- The manifest is bounded (64 KiB, 256 entries); keep the edit a small
  reviewable diff — don't reformat the file.
- Confirm with `cheaper workflow list --cwd <workspace>`: the new entry
  should be `status:"ok"` with no `missing-path`/`duplicate-id`
  diagnostics. Catalog is a discovery index only — there is no
  `workflow run <id>` lookup; execution stays explicit-path.

## Report back

Tell the user:

- the **explicit path** of the saved definition,
- the public contract: input names (required/optional + defaults) and
  output names with schemas,
- validation result (`workflow interface` pass; state anything left
  unverified as unverified),
- catalog state: registered id or "not registered" and why,
- required permissions/dependencies (grant names, custom workers,
  authorized `process` tasks) the run will need,
- invocation examples:

  ```bash
  cheaper workflow interface .cheaper/workflows/<purpose>.json --cwd <workspace>
  cheaper workflow run .cheaper/workflows/<purpose>.json --cwd <workspace> --input inputs.json
  cheaper workflow run .cheaper/workflows/<purpose>.json --cwd <workspace> --input - <<'EOF'
  {"taskName": "test"}
  EOF
  ```

  `--input` takes a **file path or `-` (stdin)** — never inline JSON
  like `--input '{...}'`. When a run isn't approved yet, present the
  commands as examples to run later, and don't run them.

## Good vs bad

| Good | Bad |
|---|---|
| `{"$from": "exec", "path": "/details/stdout/text"}` | `"$from": "exec"` (missing `path`), or string interpolation like `"${exec.details.stdout.text}"` |
| Output bound from a skippable job: `{"type": ["string","null"]}` | Output schema `{"type":"string"}` bound from a `when`-guarded job (materializes `null` on skip → `INVALID_OUTPUT` at run time) |
| `needs` only for ordering with no data flow | `needs` restating every `$from` edge (harmless but noise) |
| One worker job for one bounded task | Wrapping a single `read` call or one `cheaper run` in a DAG — call it directly |
| Real names: `bulk-reader`, `grep`, `command` | Invented names: `"summarizer"`, `"run-tests"`, `"write-file"` — `interface` rejects them |
| `cheaper workflow run <path> --input inputs.json` | `cheaper workflow run <path> --input '{"task":"x"}'` (inline JSON is not accepted) |
| Ask the user before a `command`/`process` grant, then document it | Auto-adding grants/policy/routes to make a definition run |
| Register after `interface` passes, verify with `workflow list` | Registering unverified definitions or silently replacing a duplicate id |

## Related skills

- `cheaper-usage` — the canonical CLI contract (envelope, exit codes,
  `--input`/`--cwd`, `workflow` commands). Read it before invoking any
  cheaper command.
- `cheaper-bulk-read` / `cheaper-code-write` / `cheaper-doc-write` /
  `cheaper-tool-operate` — per-worker input/output contracts for the
  jobs a workflow can contain.
- `cheaper-workflow-discover` — proposes candidates; a user-approved
  proposal is one of this skill's triggers.

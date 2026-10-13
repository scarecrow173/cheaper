---
name: cheaper-tool-operate
description: Delegate multi-step operational loops (run an operation, interpret its result, choose the next step) to the cheaper tool-operator worker instead of iterating tool calls in your own context. Returns a compact evidence-backed result.
---

# Delegate operational loops to cheaper

## When to use

Use `tool-operate` when the work is a **multi-step operation loop** — run an
operation, interpret the result, choose the next operation — that you would
otherwise iterate in your own context.

- Running an authorized build/test/lint command and reporting the outcome
- Chained inspect-then-act sequences toward an operational goal
- Verifying a workspace condition (a check passes, a file exists, output
  matches an expectation)

Do **not** use it for file edits — the worker has no write/edit capability
(use `code-write` / `doc-write`). For one-shot large reads use `bulk-read`.
It cannot run arbitrary shell commands: `process` and `command` only invoke
tasks and grants that trusted policy has authorized.

## How to invoke

See the `cheaper-usage` skill for the full CLI contract. In short:

```bash
echo '{"task":"run the test suite and report whether it passed","context":["the `test` named task runs the suite"],"constraints":["do not install anything new"]}' \
  | cheaper tool-operate --cwd <workspace>
```

Input (`tool-operator` schema):

| Field | Type | Meaning |
|---|---|---|
| `task` (required) | string ≤4000 | The operational goal to work toward |
| `context` | array ≤30 | Background facts the worker needs |
| `constraints` | array ≤20 | Hard rules it must follow |
| `output` | object | Hints — `focus` (array ≤10) and `maxFindings` (int 1–30) |

## Consuming the result

`result` is a compact structured outcome — not a tool transcript:

- `status`: `success` (goal met with evidence), `partial`, `failed`, or
  `blocked` (a required capability was unavailable under policy — a normal
  run result, not a run-level error).
- `summary`: the direct outcome of `task`.
- `findings[]`: `{title, detail, evidence[]}` — each evidence entry is a
  `kind`-tagged record (`location`, `commit`, `exitCode`, `truncation`,
  `note`), never raw tool output.
- `unresolved[]` / `nextActions[]`: what remains and the concrete next steps
  you can take (populated on `partial` / `failed` / `blocked`).

`meta.changes` is always empty — tool-operator has no write/edit, and child
process side effects are not audited. `savings.directlyComparable` is always
`false` (mutating tools are declared).

## Degrade semantics

The worker works without any `process`/`command` authorization: both are
declared via `cheaper.optionalTools`, so under a policy with no authorized
tasks or grants they are simply not registered and the run continues with
the remaining tools — the worker completes what it can or returns
`status: "blocked"` naming the missing capability. When the policy does
authorize tasks/grants, the worker uses `process`/`command` normally,
limited to the declared task and grant names. A `tools.allow` exclusion is
different: it fails the whole run as a policy violation, not a degrade.

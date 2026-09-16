---
name: cheaper-code-write
description: Delegate mechanical code changes to the cheaper code-writer worker. Use for boilerplate generation, renames, repetitive edits, and other low-judgment code mutations; verify the result through the runtime-measured change journal.
---

# Delegate mechanical code changes to cheaper

## When to use

Use `code-write` when the change is **mechanical**: the decision is already
made and only the typing remains.

- Generating boilerplate / scaffolding files
- Applying the same edit pattern across files (rename, signature change)
- Mechanical refactors with a precise specification

Do **not** use it for changes that need design judgment, ambiguous
requirements, or security-sensitive logic — decide yourself, then delegate
the typing if it stays mechanical.

## How to invoke

See the `cheaper-usage` skill for the full CLI contract. In short:

```bash
echo '{"task":"create a retry wrapper following src/retry.ts conventions","targets":[{"path":"src/withRetry.ts","intent":"create"}],"references":["src/retry.ts"],"constraints":["no new dependencies"]}' \
  | cheaper code-write --cwd <workspace>
```

Input (`code-writer` schema):

| Field | Type | Meaning |
|---|---|---|
| `task` (required) | string ≤6000 | Precise description of the mechanical change |
| `targets` (required) | array 1–20 | `{path, intent: create\|modify\|create-or-modify, notes?}` — the files the worker may touch |
| `references` | array ≤30 | Paths to read for conventions/examples |
| `constraints` | array ≤20 | Hard rules (e.g. "no new dependencies") |

## Verifying the result

You must verify — the worker writes into your workspace.

- `meta.changes` is **measured by the runtime journal**, not claimed by the
  model. Check `meta.changes.files[].path` ⊆ your declared `targets`.
  A file outside `targets` is an unintended mutation — review it.
- `result.summary` / `result.notes` describe what was done;
  `result.unresolved` lists anything the worker could not finish.
- Review the actual diff in the target files before accepting the change.

On failure, `error.code === "PARTIAL_MUTATION"` (exit 9) means files were
written but the run did not finish cleanly — inspect `meta.changes` to see
what was already mutated.

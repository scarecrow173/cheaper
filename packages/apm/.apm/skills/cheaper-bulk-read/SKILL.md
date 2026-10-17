---
name: cheaper-bulk-read
description: Delegate reading large files or many files to the cheaper bulk-reader worker instead of loading them into your own context. Use when a file is over the token threshold, when you need a compressed summary of many files, or when a pre_read_code hook redirects you here.
---

# Delegate bulk reading to cheaper

## When to use

Use `bulk-read` instead of reading files directly when:

- `cheaper count-tokens <path>` reports a file above the read threshold —
  loading it would burn your context on mostly-irrelevant content.
- `cheaper count-tokens <path>` fails with `error.code === "FILE_TOO_LARGE"`
  — the file is too large to even measure, so it is certainly too large to
  read directly. Do not retry `count-tokens`; delegate here.
- A `pre_read_code` hook blocked your read and told you to delegate —
  take the `workspace-relative path:` value from its output verbatim and
  put it in `paths` (the schema requires workspace-relative paths).
- You need findings across many files (call graph, feature location,
  impact analysis) where raw content is not what you need.

Do **not** use it when you need the exact bytes of a file (you are about to
edit it line-by-line), or when the file is small enough to read directly.

## How to invoke

See the `cheaper-usage` skill for the full CLI contract. In short:

```bash
echo '{"task":"find where the token counting boundary is enforced","paths":["src/**/*.ts"]}' \
  | cheaper bulk-read --cwd <workspace>
```

Input (`bulk-reader` schema):

| Field | Type | Meaning |
|---|---|---|
| `task` (required) | string | What you need to learn — write the question, not a file dump request |
| `paths` | array | Workspace-relative paths or globs to analyze |
| `hints` | array | Terms, symbol names, or areas to focus on |
| `maxFindings` | int ≥1, or null | Cap on returned findings — absent/`null` leaves the count to the worker |

## Consuming the result

`result` is a compressed, structured summary — not file contents:

- `summary`: short answer to `task`.
- `findings[]`: `{title, detail, confidence, locations[{path, lines, excerpt}]}` —
  each finding carries small excerpts with precise locations.
- `relevantFiles[]`: `{path, reason}` — files worth knowing about.
- `suggestedReads[]`: `{path, lines, reason}` — the *small* line ranges you
  should read yourself if you still need exact content.
- `openQuestions[]`: what the worker could not determine.

`meta.changes` is always empty (read-only worker). `meta.usage` reports the
tokens the cheaper model spent.

## Output shape

The worker's output is schema-validated — the schema enforces **structure**,
not size: no field has a character or item-count cap. Canonical source:
`crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json`
in the cheaper repository, embedded into the binary — there is no CLI command
that dumps a worker schema.

| Field | Contract |
|---|---|
| `summary` | string (required) |
| `findings[]` | array of objects (required) |
| `findings[].confidence` | `high` / `medium` / `low` |
| `locations[].path` | workspace-relative (no absolute, no `..`) |
| `locations[].lines` | `{start, end}` integers ≥1 (optional) |
| `relevantFiles[]` / `suggestedReads[]` | objects with `path` + `reason` |
| `openQuestions[]` | array of strings |

All five top-level fields are required and every object is closed
(`additionalProperties: false`) — extra or misspelled keys and missing
required keys still fail validation. Size is a worker-prose guideline, not a
hard limit: an oversized `title` or `excerpt` no longer fails the run.

## When it fails with `INVALID_OUTPUT`

`error.code === "INVALID_OUTPUT"` (exit 6, `retryable: false`) means the
worker's output failed schema validation even after the bounded repair
attempt(s). On worker `run` invocations the envelope carries:

- `error.details.errors` — the first 20 schema/JSON-parse errors (after a
  repair attempt, from the last attempt). Use them to find the offending
  field, e.g. `/findings/4/locations/0/path` being an absolute path.
- `error.details.rawOutput` — the worker's raw output truncated to the first
  2000 characters. **It is a diagnostic excerpt, not a validated result**,
  and may be empty. Do not adopt it — or anything parsed out of it — as the
  answer.
- `error.details.rawOutputPath` — when present, an absolute path to the
  **complete, untruncated** raw output saved under
  `<userConfigDir>/invalid-outputs/`. Read that file to recover or inspect
  the full output; the key is omitted when the raw output was empty or the
  save failed (best-effort).

Recovery, in order:

1. Re-run with a narrower scope — a more specific `task`, fewer `paths`,
   or a lower `maxFindings` — guided by `details.errors`.
2. If failures repeat, raise `output.repairAttempts` (bundled default is 1;
   `docs/guides/configuration.md` in the cheaper repo recommends 4 for
   smaller models) or `output.maxTokens` in your project/user `policy.yaml`.
3. Fall back to reading the files directly or splitting the work into
   smaller `bulk-read` calls.

## When it fails with budget exhaustion or timeout

- `error.code === "TIMEOUT"` (exit 7, retryable) means the run went
  silent for the whole `execution.timeout` stall window (default 300s) —
  not that it ran too long in total. `transientAttempts` retries only
  provider transient failures, so they never extend this window; the
  backoff itself counts toward it. Re-run, or raise `execution.timeout`
  in your `policy.yaml` if legitimate single steps (large `process`
  tasks, non-streaming completions) outlast the window.
- `error.code === "MAX_TURNS_EXCEEDED"` or `"MAX_TOOL_CALLS_EXCEEDED"`
  (exit 6 — or exit 9 `PARTIAL_MUTATION`; bulk-reader is read-only so
  6 applies) means a **configured** budget was hit. `execution.maxTurns`
  defaults to `null` (no policy turn cap), so this only occurs when you
  or policy bounded it — or when `maxToolCalls` (default 64) was
  reached. These errors carry `error.details.checkpoint = {path,
  observationsCount}` pointing to a saved checkpoint under
  `<userConfigDir>/checkpoints/`; continue with
  `cheaper bulk-read --resume <path>` (same worker, fresh budget, prior
  observations injected; pass `--grant` again if used). Resumed runs
  that exhaust again accumulate observations into a new checkpoint, so
  chained resume works. Prefer resuming over blind retry — the worker
  keeps its prior findings instead of restarting from scratch.

## Recommended flow

```text
count-tokens says file is large (over threshold or FILE_TOO_LARGE)
  → bulk-read with a precise task + the file in paths
  → consume summary/findings
  → if a finding needs exact code, Read only the suggested lines range
```

This keeps your context to findings + targeted excerpts instead of whole
files.

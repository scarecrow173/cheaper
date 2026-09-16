---
name: cheaper-bulk-read
description: Delegate reading large files or many files to the cheaper bulk-reader worker instead of loading them into your own context. Use when a file is over the token threshold, when you need a compressed summary of many files, or when a pre_read_code hook redirects you here.
---

# Delegate bulk reading to cheaper

## When to use

Use `bulk-read` instead of reading files directly when:

- `cheaper count-tokens <path>` reports a file above the read threshold —
  loading it would burn your context on mostly-irrelevant content.
- A `pre_read_code` hook blocked your read and told you to delegate.
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
| `task` (required) | string ≤4000 | What you need to learn — write the question, not a file dump request |
| `paths` | array ≤50 | Workspace-relative paths or globs to analyze |
| `hints` | array ≤20 | Terms, symbol names, or areas to focus on |
| `maxFindings` | int 1–30 | Cap on returned findings |

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

## Recommended flow

```text
count-tokens says file is large
  → bulk-read with a precise task + the file in paths
  → consume summary/findings
  → if a finding needs exact code, Read only the suggested lines range
```

This keeps your context to findings + targeted excerpts instead of whole
files.

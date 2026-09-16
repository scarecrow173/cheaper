---
name: cheaper-doc-write
description: Delegate mechanical document creation and editing to the cheaper doc-writer worker. Use for READMEs, guides, ADRs, release notes, and other structured prose where you supply the decisions and facts and the worker supplies the typing.
---

# Delegate mechanical document work to cheaper

## When to use

Use `doc-write` when the document's **content decisions are already made**
and the remaining work is structured typing.

- Generating a README, guide, or reference page from known facts
- Writing an ADR/spec/plan document from decisions you supply
- Applying mechanical edits to existing docs (sections, lists, tables)

Do **not** use it to make the decisions themselves — `decisions` and `facts`
are inputs you provide. For source code changes, use `code-write`
(`cheaper-code-write`) instead.

## How to invoke

See the `cheaper-usage` skill for the full CLI contract. In short:

```bash
echo '{"task":"write a getting-started guide","documentType":"guide","targets":[{"path":"docs/guide.md","intent":"create"}],"decisions":["package installs via apm"],"facts":["CLI name is cheaper"],"constraints":["English"]}' \
  | cheaper doc-write --cwd <workspace>
```

Input (`doc-writer` schema):

| Field | Type | Meaning |
|---|---|---|
| `task` (required) | string ≤6000 | What document work to do |
| `documentType` (required) | enum | `adr`, `spec`, `design`, `plan`, `task`, `readme`, `guide`, `reference`, `release-notes`, `other` |
| `targets` (required) | array 1–20 | `{path, intent, notes?}` — docs the worker may create/modify |
| `decisions` | array ≤30 | Settled decisions the document must reflect |
| `facts` | array ≤30 | Verified facts to include |
| `references` | array ≤30 | Paths to read for style/format |
| `constraints` | array ≤20 | Hard rules (language, format, length) |

## Consuming the result

- `result.summary` describes what was written; `result.notes` lists
  caveats; `result.unresolved` lists missing inputs you should supply
  (e.g. an unknown release date) rather than letting the worker invent.
- `meta.changes` is measured by the runtime journal — verify written paths
  are within your declared `targets`.
- Read the document before accepting it; doc-writer output is a draft.

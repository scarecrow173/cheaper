---
id: "TSPEC-034MGCFe4bqwpXhKCEYgRA"
type: "test-spec"
status: "proposed"
title: "Execution correlation and privacy projection"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/designs/execution-history-issue-192.md"
  derived-by: []
  verifies:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/designs/execution-history-issue-192.md"
    - "docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Execution correlation and privacy projection

## Purpose

Preserve trustworthy correlation and the History privacy allowlist across
standalone runs, nested work, Hook decisions, and managed child processes.
Retire this spec only if the identity carrier or persisted projection contract
is deliberately superseded.

## Feature

Feature: History records expose bounded, validated correlation evidence without
persisting raw invocation data.

## Rules

- Each cheaper process issues its own UUID v4 `executionId`; a root issues a
  `traceId`, and only an actual lifecycle-owned edge may become
  `parentExecutionId`.
- Incoming or environment-carried parent values are validated hints stored as
  `parentExecutionHintId` with `inherited-unverified` provenance. Missing or
  invalid hints create an unrelated root and never affect grants or policy.
- Provider/model fields come only from the resolved catalog identity projection;
  raw model input, endpoint/base URL, display label, command text, paths,
  prompts, tool data, credentials, and raw error text are not persisted.
- Persisted kinds, statuses, verdicts, reasons, identifiers, and metric values
  use finite allowlists, canonical IDs, length limits, and numeric caps;
  unmeasured values remain `null` or are omitted rather than inferred.
- History projection cannot alter Hook native stdout/stderr/exit, policy,
  grants, or the product result.

## Examples

- Given a managed child with an explicit parent carrier, when its record is
  written, then its local execution ID is distinct and the carrier is shown as
  an unverified hint unless the lifecycle established the parent edge.
- Given an invalid parent or trace hint, when a run starts, then no malformed
  relation is persisted and the run remains an independent root.
- Given a resolved model ID `amazon.nova-2-lite-v1:0`, when identity is
  projected, then that official ID is preserved while endpoint and raw input
  are absent.
- Given Hook input containing a path, prompt, secret, or raw error message,
  when typed evidence is persisted, then only the allowlisted decision and
  bounded measurements remain and the native response is byte-equivalent.

## Guarantees

- Correlation distinguishes verified containment from unverified external
  context and never guesses relations from names, paths, timing, or `logRef`.
- A privacy audit can show that every persisted field is allowlisted and that
  official catalog identities and measured/null metrics retain their meaning.
- Best-effort History recording cannot become a new authorization or output
  channel, and privacy failures do not change the underlying operation.

## Non-goals

- This spec does not define usage aggregation, event cardinality, retention,
  query pagination, or external-host behavior after a Hook response.
- It does not reject an otherwise valid catalog definition merely because its
  History-only identity projection is omitted.

## Risk

- Treating a forged carrier as a verified parent can mislead incident tracing
  and cross-process ownership. Persisting raw input or error text can expose
  secrets and user data, while coupling projection to native output can break
  existing Hook integrations.

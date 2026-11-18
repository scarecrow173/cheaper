---
id: "SPEC-034LNA8FqEjwFaDnqvIUc5"
type: "spec"
status: "implemented"
title: "Workflow discover skill (L1) for issue 203"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/203"
    - "https://github.com/scarecrow173/cheaper/issues/200"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/workflow-discover-skill-structure.md"
    - "../plans/implement-workflow-discover-skill.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
    - "../specs/workflow-catalog-issue-201.md"
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-discover-skill-issue-203.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
    - "docs/guides/usage.md"
  defers:
    - "https://github.com/scarecrow173/cheaper/issues/204"
    - "https://github.com/scarecrow173/cheaper/issues/205"
  deferred-by: []
---

# Workflow discover skill (L1) for issue 203

## What Should Be Built

A new APM skill `cheaper-workflow-discover`
(`packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md`) — the
L1 evaluator of epic #200. Given a candidate flagged by the L0 check
(#202) or an explicit user request, it compares the realistic
execution options — direct tools, an existing worker, a configured
route, a cataloged workflow (#201), or a new workflow — and returns an
evidence-labeled recommendation, including recommending **against** a
workflow. It proposes only: nothing is created, persisted, granted,
or executed. Two consumer guides gain a summary + pointer each (skills
list / L0→L1 handoff note in `parent-agent-integration.md`, a
discovery cross-reference in `workflow-usage.md`). No CLI, crate,
manifest, or hook changes.

## Why Now

Epic #200's three-layer conduit needs its middle layer: #201 shipped
the catalog L1 reads, and #202 shipped the L0 gate that decides
whether to load this skill. Without L1, candidates detected by L0 have
nowhere to go, and L2 (#204) has no reviewed proposal to consume.
The skill body is also the only surface that reaches every APM target
uniformly — per-target firing limits were established in #202 and
apply identically here.

## Users and Value

Parent coding agents (and their users) get a disciplined comparison
before any workflow work is spent: reuse and parameterization are
checked first, simple tasks get an explicit `not-recommended`, and
every benefit claim carries `measured`/`estimated`/`unknown` so
unmeasured savings are never asserted. Users keep full control —
the output is a proposal that requires explicit approval before
anything persists or runs.

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md`:
  front matter (`name` + `description` written so a parent agent
  loads it when comparing delegation/workflow options, including the
  L0 handoff case), the required inputs, the ordered comparison
  procedure, evidence-label rules, the output contract, the
  conditional L1→L2 handoff, and non-goals.
- `docs/guides/parent-agent-integration.md`: skills list updated to
  six entries with `cheaper-workflow-discover`, plus a short L0→L1
  handoff note (what L1 does once loaded; canonical = the skill body).
- `docs/guides/workflow-usage.md`: a discovery cross-reference
  section noting that L1 applies this guide's selection criteria —
  pointer only, no duplicated policy.
- The doc-driven chain: discovery/spec/ADR/design/plan/task/
  impl-record + README index rows.

### Out of Scope

- `cheaper-workflow-create` (L2, #204 — unmerged; referenced only
  conditionally), cross-layer E2E (#205).
- Any CLI command, crate, manifest (`apm.yml`), hook, schema, or
  workflow-format change; deployed copies under `.agents/skills/`.
- Resident monitoring, per-turn LLM/CLI checks, conversation-log or
  full-history collection, a bespoke cost estimator, a completeness-
  proving duplicate detector.
- Any text instructing the agent to write files, run `workflow run`,
  or modify grants/policy/catalog — including during a proposal.

## Requirements

- The skill's only persistent decision vocabulary is
  `reuse | modify | create | direct-tool | direct-worker | route |
  not-recommended`; a simple task that a direct tool or single worker
  covers must be able to end in an explicit negative recommendation.
- Comparison order is reuse-first: direct adequacy → existing workers
  → existing routes → catalog → interfaces of fitting candidates →
  unified comparison → economics. Existing reuse, input
  parameterization, and small modification are compared before new
  creation.
- Inspection surfaces are read-only and already verified:
  `cheaper workflow list` (catalog metadata only — never an
  authority for execution/permissions), `cheaper workflow interface
  <workspace-relative path>` (public I/O only), `cheaper route check`
  / `route resolve --command|--argv` (read-only diagnostics),
  `cheaper usage` / `history` / `count-tokens` / `models` as
  evidence inputs. The skill must never instruct reading workflow
  definition files to inspect them.
- Catalog anomalies (`missing`/`invalid`/`duplicate` entry status,
  malformed-manifest error, absent catalog) are reported honestly —
  no silent substitution, no treating an absent catalog as proof of
  non-existence.
- Economics separates `initialAuthoringCost + N × workflowPerRunCost`
  from `N × directPerRunCost`; a break-even N is computed only with a
  positive difference plus observed values or a stated pricing
  assumption — otherwise qualitative judgment + `unknown`.
- Every benefit/cost claim carries `measured | estimated | unknown`.
  `cheaper usage`/`history` reflect cheaper's own runs only; parent
  tokens, unobserved repetitions, and other sessions' cost are never
  fabricated or upgraded to `measured`.
- The output contract requires: the decision, alternatives
  considered, evidence labels, the conceptual proposal summary
  (issue's YAML shape — explicitly not a persisted schema or CLI
  input), and a plain disclaimer that a proposal is not creation or
  execution and approval is required before persisting.
- `cheaper-workflow-create` is referenced only as "if installed";
  without it the skill reports the proposal and names manual
  authoring/L2 as the path — it never generates or saves JSON itself.
- The catalog lookup is workspace-scoped only; no user-global
  workflow lookup is claimed.

## Acceptance Criteria

- [x] `cheaper-workflow-discover/SKILL.md` exists under
  `packages/apm/.apm/skills/` with `name`/`description` front matter
  that fires on comparison intent and names the L0 handoff.
- [x] A simple direct-tool task and a single-worker task can both end
  in an explicit `direct-tool`/`direct-worker`/`not-recommended`
  result — no workflow is proposed.
- [x] When the catalog holds a fitting workflow (matching purpose +
  compatible I/O via `workflow interface`), the procedure prefers
  `reuse`/`modify` over `create`.
- [x] A proper fan-in/repetition case can produce a `create` proposal
  while the private DAG is never read (interface + list only).
- [x] Cost-unknown inputs produce `unknown` labels, never invented
  numbers or savings rates.
- [x] No text instructs writing definition files, editing the catalog
  or grants, or running `workflow run` before explicit user approval.
- [x] The conceptual L2 summary shape and a user-facing proposal
  example are documented in the skill.
- [x] Guide updates are cross-references only — no duplicated policy;
  `workflow-usage.md` keeps its selection-criteria authority.
- [x] `doc_status` audit reports zero blocking findings and
  `mise run check` passes.

## Deferred Design Concerns

- `cheaper-workflow-create` (#204) merge state | referenced
  conditionally ("if installed") until merged | when #204 lands, the
  handoff text needs no change — the conditional already covers it
- Cross-layer E2E (L0→L1→catalog→L2 on real runtimes) | #205 owns it
  | this skill's instructed CLI surfaces are each verified against the
  built binary, but implicit-firing behavior is runtime-dependent
- A persisted proposal schema or CLI-carried handoff | the issue
  defines a conceptual shape only | revisit only if L2 needs a
  machine-readable contract — would be a separate issue

## Sources and Relations

Primary source: GitHub issue #203; epic #200 fixes the invariants.
Depends on the #201 catalog contract (`workflow list` + manifest
schema) and the #202 L0 trigger (conditional handoff naming this
skill). Boundary decision recorded in
ADR-034aSc75jH9YL83FNQRuwH; discovery in
DISC-0349RTCACbyKZHJy1Cvnpv.

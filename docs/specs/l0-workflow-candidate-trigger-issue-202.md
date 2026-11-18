---
id: "SPEC-034Zk1kCap2wKUvTt21Sil"
type: "spec"
status: "implemented"
title: "L0 workflow candidate trigger rule for issue 202"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/202"
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
    - "../designs/l0-workflow-candidate-trigger-distribution.md"
    - "../plans/l0-workflow-candidate-trigger-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/apm.yml"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers:
    - "https://github.com/scarecrow173/cheaper/issues/203"
    - "https://github.com/scarecrow173/cheaper/issues/201"
  deferred-by: []
---

# L0 workflow candidate trigger rule for issue 202

## What Should Be Built

A minimal, always-on parent-agent judgment rule ("L0") that notices
workflow-candidate signals during ordinary work and decides only one
thing: whether the L1 skill `cheaper-workflow-discover` (#203) is worth
loading. L0 is delivered as a compact trigger clause in the `cheaper-usage`
skill's `description` plus a short L0 section in its `SKILL.md`, a small
instruction section in the consumer-facing parent-agent integration guide
(carrying the per-target firing-limit matrix, fallback procedure, and an
opt-in paste-in template), and a cross-reference in the workflow-usage
guide. No standalone skill, no new CLI command, no hook event, and no code
change.

## Why Now

Epic #200 builds the L0 → L1 → L2 pipeline that turns recurring parent-agent
work into reviewed, saved workflows. #201 (lightweight catalog) and #203
(L1 skill) can proceed in parallel, but L0 is the entry gate: without a
cheap always-on trigger, the deeper layers never engage during normal
work. L0 must ship as a tiny convention so it adds no per-turn LLM/CLI
cost and does not depend on components that do not exist yet.

## Users and Value

Consumers' parent coding agents (Claude Code, Codex, Cursor, Copilot,
Windsurf Cascade, Devin CLI, and other `agent-skills` runtimes) get a
low-noise signal that repeated or orchestration-heavy work may deserve a
workflow — without a resident watcher, extra CLI calls, or premature
workflow creation. Users keep control: L0 only suggests reading L1.

## Scope

### In Scope

- A compact trigger clause appended to `cheaper-usage`'s front-matter
  `description` in `packages/apm/.apm/skills/cheaper-usage/SKILL.md`, so
  skill-capable runtimes surface the skill when workflow-candidate signals
  appear in normal work.
- A short "Workflow candidate check (L0)" section at the end of the same
  `SKILL.md` carrying the full rule: four candidate signals, negative
  cases, suppression conditions, and the conditional handoff to L1.
- A compact L0 section in `docs/guides/parent-agent-integration.md`: what
  the rule is and where it lives, the per-target skill
  discoverability/firing-limit matrix, the explicit-invocation fallback,
  and a minimal opt-in instruction template consumers can paste into
  their own agent instructions.
- A one-line cross-reference in `docs/guides/workflow-usage.md` pointing
  at the L0 section so its selection criteria stay the cost judgment
  authority L1 will apply.
- The doc-driven chain: discovery/spec/ADR/design/plan/task/impl-record.

### Out of Scope

- `cheaper-workflow-discover` (L1, #203), `cheaper-workflow-create` (L2),
  the lightweight workflow catalog (#201), and cross-layer E2E (#205).
- Any new skill file, CLI command, hook event, resident monitor,
  conversation-log collection, candidate persistence, or parent-agent
  history reconstruction.
- Changes to `packages/apm/apm.yml`, hook packages, worker skills, or any
  Rust crate; no changes to existing bulk-read hook, route hook, or worker
  skill semantics.
- Deployed copies under `.agents/skills/` (APM-generated, not tracked).

## Requirements

- L0 decides only "load L1 or not". It must not judge whether the work
  should become a workflow, search for duplicates, run cost analysis, or
  persist anything.
- Candidate signals (all judged on context already visible to the parent
  agent — no probing): (1) the same semantic procedure repeated twice or
  more in the current work, or an explicit user request for recurring
  operation; (2) manual fan-out/fan-in across independent tools/workers or
  hand-assembled tool→worker chains; (3) long intermediate outputs (logs,
  files, process results) that would be better summarized inside a
  workflow so only the public result reaches the parent; (4)
  parameterized boilerplate that becomes reusable by swapping inputs.
- Not candidates: one-shot simple read/grep/command, work a single
  existing worker call already covers, contexts where the user declined
  suggestions, or insufficient context to judge.
- Suppression conditions: each distinct candidate raised at most once per
  session and declined candidates not re-proposed without new evidence;
  no per-turn `cheaper` catalog/`history`/`usage` or file scans for this
  check; no assumption of knowledge from past sessions or other agents;
  no confusion with the PreToolUse route hook (#142) — L0 is a
  parent-agent judgment rule and must not trigger LLM evaluation per tool
  call; no claim that auto-firing is guaranteed on every target.
- The L1 handoff is conditional: if `cheaper-workflow-discover` is
  installed, load it; if it is not installed (it ships separately in
  #203), report the observed pattern to the user instead of attempting to
  invoke anything unavailable. No text may tell the agent to execute a
  capability that does not exist yet.
- `cheaper-usage` keeps being the CLI-contract skill: the L0 section must
  not duplicate its CLI documentation, and unrelated tasks must not be
  forced to read L1 or run extra commands.
- The guide section must not duplicate the skill text verbatim — it
  states the contract, references the skill section as canonical, and
  adds what only the guide can carry: the target matrix, the fallback
  procedure, and the opt-in template.
- The per-target matrix records deploy destination, runtime skill
  discovery, and firing character honestly — verified facts only, with
  unverifiable environments covered by the documented fallback.

## Acceptance Criteria

- [x] `cheaper-usage` skill metadata (description + short L0 body section)
  carries the L1 trigger condition and suppression conditions.
- [x] Unrelated tasks are not forced to read the L1 body or run extra
  CLI/LLM work: the L0 rule lives in existing skill surfaces and performs
  no per-turn calls.
- [x] Observed repetition, explicit recurring-operation requests,
  fan-out/fan-in or long intermediate outputs route to L1 (or to the
  reported-candidate fallback when L1 is not installed).
- [x] One-shot simple work and declined candidates are not repeatedly
  suggested.
- [x] `docs/guides/parent-agent-integration.md` contains a per-target
  table of skill discoverability and firing limits plus the fallback for
  environments where auto-firing cannot be guaranteed.
- [x] The guide and `cheaper-usage` do not duplicate each other; they
  cross-reference.
- [x] No semantics change to the bulk-read hook, route hook, or worker
  skills; `mise run check` passes and `doc_status` audit reports zero
  blocking findings. (Note: two pre-existing flaky timing-boundary tests
  intermittently exceeded their internal budgets under parallel load on
  the dev host — each passes in isolation; see the impl record.)

## Deferred Design Concerns

- `cheaper-workflow-discover` (L1) implementation | owned by #203; L0
  names the skill so a rename there needs a one-line sync here | risk:
  dangling name reference if #203 changes the skill name
- `cheaper-workflow-create` (L2) and the catalog (#201) | separate issues
  in epic #200 | no impact on L0
- Per-runtime auto-firing E2E on real agent binaries | requires harness
  licenses/environments out of scope here | the fallback procedure covers
  unverified environments; revisit if #205 adds real-agent E2E

## Sources and Relations

Primary source: GitHub issue #202; epic #200 fixes the layer split and the
invariants (proposal ≠ creation ≠ execution; measured/estimated/unknown
savings claims only). Deploy destinations verified against apm 0.33
(`apm targets --all --json`); runtime discoverability verified against
official Cursor, Codex, Copilot, and Windsurf Cascade skill docs (see the
discovery doc's reference list). Boundary decision is recorded in
ADR-034Zk1CgFRRcnAwV6sZWqQ.

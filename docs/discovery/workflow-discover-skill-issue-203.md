---
id: "DISC-0349RTCACbyKZHJy1Cvnpv"
type: "discovery"
status: "resolved"
title: "Workflow discover skill (L1) for issue 203"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/203"
    - "https://github.com/scarecrow173/cheaper/issues/200"
    - "https://github.com/scarecrow173/cheaper/issues/201"
    - "https://github.com/scarecrow173/cheaper/issues/202"
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
  related:
    - "./workflow-catalog-issue-201.md"
    - "./l0-workflow-candidate-trigger-issue-202.md"
    - "../specs/workflow-catalog-issue-201.md"
    - "../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/workflow-discover-skill-issue-203.md"
    - "../adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/apm.yml"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
    - "docs/guides/usage.md"
    - "crates/cheaper-cli/src/workflow_list.rs"
    - "crates/cheaper-cli/src/args.rs"
  defers:
    - "https://github.com/scarecrow173/cheaper/issues/204"
    - "https://github.com/scarecrow173/cheaper/issues/205"
  deferred-by: []
---

# Workflow discover skill (L1) for issue 203

## Exploration Goal

Issue #203 asks for the L1 skill `cheaper-workflow-discover`: given a
workflow candidate from L0 (#202) or an explicit user request, compare
the realistic execution options — direct tools, existing workers,
routes, cataloged workflows, new workflow candidates — and return an
evidence-labeled recommendation, including recommending against a
workflow. The questions to settle: which read-only CLI surfaces the
skill can instruct, what the catalog (#201) actually exposes, how the
measured/estimated/unknown evidence vocabulary maps onto existing
telemetry, and how the L1→L2 handoff must be phrased while #204 is
unmerged.

## Key Issues and Assumptions

- Confirmed: `packages/apm/.apm/skills/` carries five skills; `apm.yml`
  does not enumerate skills (hook wiring only), so a sixth skill
  directory needs no manifest or payload changes — `check-apm-payload`
  validates hook packages only.
- Confirmed (verified against the built binary, 2026-11-18):
  `cheaper workflow list --cwd <ws>` returns a utility envelope
  (`worker:null`, exit 0) with `result` =
  `{catalog:{path,present}, workflows:[{id,path,summary,tags,status}],
  diagnostics:[{kind,index,id?,path?,message}], truncated}`; absent
  catalog → `present:false` + empty list + exit 0. Accepted flags:
  `--cwd`, `--limit` (1..256), `--format`, `--quiet`, `--log`,
  `--log-format`; execution flags are `INVALID_REQUEST`.
- Confirmed: `cheaper workflow interface <path>` returns
  `{schemaVersion, inputs, outputs}` (schema + `required` per field) —
  the public I/O contract with no private jobs/prompt/bindings. The
  path must resolve inside the workspace; outside paths fail
  `INVALID_REQUEST` (exit 2). Catalog `path` values are already
  workspace-relative, so list → interface composes directly.
- Confirmed: `cheaper route check` is strictly read-only (report of
  layers/routes/configuration/grant readiness/diagnostics; exit class
  0/1/2/3/70) and `cheaper route resolve --command|--argv` is a
  dry-run returning `{matched, route, target, grant, viability}`
  (`runnable`/`no-grant`/…). Both are safe for the skill to instruct.
- Confirmed: `cheaper usage` aggregates cheaper's own usage.jsonl
  (`usage.totalTokens`, `usage.costUsd`, `measured.sourceTokens/
  returnedTokens/avoidedContextTokens/comparable.costUsd`,
  `--compare-model` estimated savings); `cheaper history list` has no
  `--cwd` flag; `cheaper count-tokens` and `cheaper models` are the
  measured/pricing inputs available to a parent agent.
- Confirmed: `cheaper usage`/`history` observe cheaper's own runs
  only — the parent agent's context, turn count, and tokens are
  unmeasured by construction (#44 #67 #183). Any parent-side savings
  claim is therefore `estimated` at best, usually `unknown`.
- Confirmed: `cheaper-workflow-create` (L2, #204) is on an unmerged
  parallel branch and is NOT in this worktree's main. All references
  must be conditional ("if installed"), exactly as #202's L0 section
  does for this skill.
- Assumption: the proposal summary from the issue is a conceptual
  shape only — a structured note the parent holds in context and hands
  to L2, not a persisted schema, CLI input, or file format.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. Standalone skill `cheaper-workflow-discover` in `.apm/skills/` carrying the full comparison procedure | Issue mandates this name/placement; heavy body loads only when L0 or the user pulls it (epic invariant 2); skill body is the only surface that reaches all APM targets uniformly | Adds a sixth skill to the package; description must be written to fire on comparison intent, not only on L0 handoff | yes |
| B. Fold the L1 procedure into `cheaper-usage` | Zero new primitives; same deploy surface as L0 | Makes every CLI-contract read pay for the comparison text; defeats the L0 lazy-load design the same issue chain just built | no |
| C. New read-only CLI (e.g. `workflow discover`) that emits a comparison | Deterministic output; testable | Out of scope per issue (no new CLI); a fixed command cannot carry the judgment/uncertainty policy, and would push private context gathering into the CLI | no |
| D. Guide-only instructions | Zero distribution cost | Guides never enter agent context — cannot fire; same rejection as #202 option C | no |

## Tentative Conclusions and Hypotheses

Confirmed: option A — the comparison procedure, evidence rules, output
contract, and conditional L2 handoff live in a dedicated
`cheaper-workflow-discover` SKILL.md; the guides carry only a
summary + pointer (canonical/pointer convention, same as #202).

Confirmed: the skill's decision vocabulary is the issue's set —
`reuse | modify | create | direct-tool | direct-worker | route |
not-recommended` — applied to the five options (a)–(e) under one
criterion set (reuse value, latency, cost, uncertainty, constraints).

Confirmed: evidence labels are `measured / estimated / unknown`, with
`measured` limited to the parent's own observations, cheaper telemetry
(`usage`/`history`/`count-tokens`), and benchmark artifacts; parent
token savings is never `measured` (unobservable) — matching the claim
boundary `workflow-usage.md` already enforces.

Confirmed: handoff phrasing — `create`/`modify` + installed
`cheaper-workflow-create` → offer after user approval; not installed →
report the proposal and name manual authoring/L2 as the path;
`reuse` → state explicit path + interface; `direct-*`/`route`/
`not-recommended` → stop without dangling a workflow suggestion.

## Open Questions

- None blocking. #205 (cross-layer E2E) owns end-to-end verification of
  the L0→L1→catalog→L2 flow on real runtimes; this skill's CLI
  instructions were each verified against the built binary.

## Promotion Candidates

- [x] spec-doc needed: skill contract — inputs, procedure, evidence
  labels, output/handoff contract, boundaries →
  `../specs/workflow-discover-skill-issue-203.md`
- [x] adr-doc needed: where the L1 comparison policy lives (skill body
  vs `cheaper-usage` vs CLI) →
  `../adr/l1-workflow-comparison-lives-in-discover-skill-body.md`

## Sources and Relations

Issue #203 is the primary product source; epic #200 fixes the
invariants (proposal ≠ creation ≠ execution; measured/estimated/
unknown only; discovery via `workflow list` + `workflow interface`,
never private definitions). #201 supplies the catalog contract
(`workflow list` read-only surface + `catalog.json` schema) and #202
supplies the L0 entry rule whose conditional handoff names this skill.
CLI output shapes verified locally against `target/debug/cheaper.exe`
on a scratch workspace (catalog present/absent, `workflow interface`,
`route check`/`route resolve`, `usage`, `history list`, `count-tokens`,
`models`). #204 stays conditional until merged; #205 owns cross-layer
E2E.

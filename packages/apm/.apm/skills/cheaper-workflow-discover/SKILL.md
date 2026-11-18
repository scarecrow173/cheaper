---
name: cheaper-workflow-discover
description: Compare how to execute repeated or orchestration-heavy work — direct tools, an existing cheaper worker, a configured route, a cataloged workflow, or a new workflow candidate — and return an evidence-labeled recommendation. Load this when the cheaper-usage L0 check flags a workflow candidate, when you keep hand-assembling the same fan-out/fan-in or tool-to-worker chain, when a recurring procedure produces large intermediate outputs, or when the user asks whether some work should become a workflow. Evaluates reuse value, latency, cost, and uncertainty; a proposal is not creation and is not execution — this skill never writes files, never runs a workflow, and never changes grants, policy, or the catalog.
---

# Discover and compare execution options

You are the L1 evaluator. Given a candidate (from the L0 check in
`cheaper-usage`, or an explicit user request like "should this recurring
work become a workflow?"), compare the realistic execution options and
recommend one — including recommending **against** a workflow when that
is the honest answer. You decide and propose; you never create,
persist, or execute.

## Boundaries

- **Discovery only.** Never write or edit files, never run
  `cheaper workflow run`, never modify `catalog.json`, grants, policy,
  or routes. Approval from the user is required before anything is
  persisted — and persisting is another skill's job, not yours.
- **Never read workflow definition files to inspect them.** A workflow
  JSON holds a private DAG, prompts, and bindings — loading it into
  your context defeats the purpose. Inspect cataloged workflows only
  through `cheaper workflow list` and `cheaper workflow interface`.
- **Workspace-scoped lookup only.** `workflow list` reads the project
  catalog at `<workspace>/.cheaper/workflows/catalog.json`. There is no
  user-global workflow catalog — do not invent one.
- **Your context is not cheaper's telemetry.** `cheaper usage` and
  `cheaper history` record cheaper's own runs only — not your token
  usage, not past sessions, not other agents' work. Do not fabricate
  repetition counts you did not observe, executions that did not
  happen, or user cost figures.
- **Stay cheap.** Gather only what the comparison needs. Do not collect
  raw huge logs, full conversation transcripts, or secret inputs, and
  do not run broad file scans "just in case".

## What to gather

Collect a short picture of the work before comparing. Classify every
fact by how you know it — `observed` (you saw it happen),
`user-stated` (the user told you), or `unknown`:

- **Procedure summary** — the steps, in a few lines.
- **Repetition** — how many times it ran (observed count), how many
  times it is expected to run (user-stated plan), or `unknown`. Keep
  these three sources distinct; never merge them into one number.
- **Representative inputs and outputs** — what varies per run and what
  a result looks like.
- **Intermediate results you currently carry** — logs, file contents,
  process output you read and re-feed into the next step.
- **Side effects and budget** — what the work is allowed to touch, and
  any latency or duration constraint.
- **Known comparators** — workflows, routes, or workers the user or L0
  already named.

If a decision-relevant fact is missing, prefer one direct question to
the user over an assumption. If it stays missing, record it as
`unknown` — an honest unknown is a valid input, a fabricated number is
not.

## Comparison procedure

Work through the options in this order. An early adequate answer beats
a later clever one — reuse and parameterization always come before new
creation.

1. **Is automation even warranted?** If the task is one-shot, or a
   single deterministic tool call or a single existing worker call
   covers it end to end, return `direct-tool` or `direct-worker` /
   `not-recommended` explicitly. A workflow is not the default answer.
2. **Existing workers.** Would one built-in worker call
   (`bulk-reader`, `code-writer`, `doc-writer`, `tool-operator` — see
   their skills) or a custom worker file cover the work? If yes, that
   is the baseline every other option must beat.
3. **Existing routes.** If the work starts from a shell command,
   inspect configured routes without executing them:
   `cheaper route check` reports every route layer, grant readiness,
   and diagnostics; `cheaper route resolve --command '<cmd>'` (or
   `--argv '<json>'`) dry-runs one input and reports `matched` and
   `viability` (`runnable`, `no-grant`, …). Both are read-only.
4. **Cataloged workflows.** Run `cheaper workflow list --cwd
   <workspace>` for the short discovery metadata
   `{id, path, summary, tags, status}` plus `diagnostics`. An absent
   catalog returns an empty list — that is a fact, not an error.
   Entries with `status` other than `ok` (`missing`, `invalid`,
   `duplicate`) are not usable candidates; note them in diagnostics,
   do not silently substitute another entry. `--limit` bounds the list
   when `truncated` could matter.
5. **Interfaces of fitting candidates.** For catalog entries whose
   summary/tags suggest the same purpose, read the public contract
   only: `cheaper workflow interface <workspace-relative path>`
   returns `{inputs, outputs}` with schemas and required flags — never
   the private jobs. Compare the candidate's inputs/outputs against
   what the work actually needs: an existing workflow that fits by
   parameterization (`reuse`) or with small changes (`modify`) beats a
   new definition (`create`). The catalog cannot prove total
   duplication coverage — say so rather than claiming none exists.
6. **Compare all options on one scale.** Score each surviving option —
   (a) direct tool/CLI, (b) single worker, (c) existing route,
   (d) existing workflow, (e) new workflow — on the same criteria:
   reuse value (does an existing option already do this, or does the
   work repeat enough to justify a saved definition?), latency
   (orchestration turns, CLI startup, workflow overhead — a simple
   tool-only task is often *slower* as a workflow), cost (see below),
   uncertainty (how much of the claim rests on unmeasured guesses),
   and constraints (side effects, grants, runtime limits). The
   workflow-specific questions: does it need a DAG/fan-in, does it
   keep intermediate outputs out of your context, does it cut your
   orchestration turns, will it actually repeat?
7. **Economics.** Separate authoring cost from per-run cost. Compare
   `initialAuthoringCost + N × workflowPerRunCost` against
   `N × directPerRunCost`. Compute a break-even N only when the
   difference is positive **and** you have observed values or a stated
   pricing assumption — otherwise give a qualitative judgment and mark
   the benefit `unknown`. Useful measured inputs when they exist:
   `cheaper count-tokens <file>` for real input sizes, `cheaper usage`
   for cheaper's own per-worker token/cost aggregates (including
   `measured` offload figures and `--compare-model` pricing
   estimates), `cheaper models` for per-model pricing. Remember the
   Boundaries section: these describe cheaper's runs, not yours.

## Evidence labels

Every benefit or cost claim carries exactly one label:

- **measured** — taken from an actual observation: values you watched
  happen, `cheaper usage`/`history` records of cheaper's own runs, or
  benchmark artifacts.
- **estimated** — derived from measured inputs under stated
  assumptions (a repetition count the user gave you, a pricing table,
  a modeled N).
- **unknown** — anything else. When in doubt, choose `unknown`.

Never assert token or cost savings you did not measure. In particular,
`sourceTokens`/`returnedTokens` offload figures and benchmark byte
comparisons do **not** prove parent-agent tokens saved — the parent's
context is unobserved by cheaper (issue #67/#183 evidence). Worker-side
usage and parent-side usage are different measurements; keep them
separate in both the analysis and the proposal.

## Output contract

End with a short report to the user containing:

1. **Recommendation** — exactly one decision:
   `reuse` (existing workflow fits as-is), `modify` (existing workflow
   fits with small changes), `create` (new workflow justified),
   `direct-tool`, `direct-worker`, `route`, or `not-recommended`.
2. **Alternatives** — the options you compared and why each lost.
3. **Evidence** — the labeled benefit/cost claims, with the
   assumptions behind every `estimated` figure and the limits of every
   `measured` one.
4. **Proposal summary** — when the decision is `reuse`, `modify`, or
   `create`, a small structured summary the parent agent can hold and
   hand to L2 (conceptual shape only — not a persisted schema, never
   written to disk by you):

   ```yaml
   task: "..."
   evidence: {source: "observed|user-stated|unknown", repetitions: "..."}
   decision: "reuse|modify|create|direct-tool|direct-worker|route|not-recommended"
   existingWorkflowPath: null  # explicit workspace-relative path, only when known
   proposedInputs: ["..."]
   proposedOutputs: ["..."]
   proposedRoute: null  # when the repeated work starts from a shell command: {name, matchCommand: [token, ...], grant: "<existing grant name>"} so L2 can bind it in .cheaper/routes.yaml — else null
   benefit: {status: "measured|estimated|unknown", rationale: "...", assumptions: []}
   risksAndEffects: ["..."]
   approval: "required-before-persist"
   ```

   Fill `proposedRoute` only when the trigger is a repeated shell
   command — the hook can then intercept that command and run the
   workflow in its place. `grant` must name an existing
   `command.grants` entry; if none exists, flag the missing grant as a
   risk and leave the decision to the user rather than implying it is
   covered. Two constraints shape `matchCommand`: it is a token
   **prefix**, and the resolved executable must be non-interactive —
   `python`/`node`/`powershell`/`bash` and friends resolve
   `stdin-unsupported` and never fire, even with a script argument.
   When the repeated command is an interpreter invocation, propose the
   binding on a named task entry instead (`npm run <task>`,
   `mise run <task>`, `just <task>`).

5. **The disclaimer** — state plainly that this is a proposal: nothing
   was created, saved, registered, or executed, and nothing will be
   without explicit approval.

Example user-facing proposal:

> Recommendation: `reuse` — the cataloged workflow `review-change`
> (`.cheaper/workflows/review-change.json`) already takes a diff and
> returns findings; `workflow interface` shows inputs `{diff, scope}`
> which match this task. Estimated benefit: avoids re-orchestrating
> ~5 tool calls per run (estimated — repetition count is user-stated,
> per-run token figures unknown). Risks: summary field says diff-only;
> multi-file refactors may exceed its inputs. Nothing was saved or
> executed — say the word and I can hand this to the create skill (if
> installed) or run it via `workflow run` on your approval.

## Handoff to L2

- If the decision is `create` or `modify` and the
  `cheaper-workflow-create` skill is installed: offer to hand the
  proposal summary over — but only after the user approves creation.
  If it is not installed, report the proposal and say that turning it
  into a saved workflow needs the create skill or manual authoring;
  do not attempt to generate or save the JSON yourself.
- If the decision is `reuse`: name the explicit path and the public
  interface to call it with; execution still happens only on the
  user's explicit request.
- If the decision is `direct-tool`, `direct-worker`, `route`, or
  `not-recommended`: say why a workflow loses, and stop — do not
  leave a workflow suggestion dangling.

## Non-goals

- Resident monitoring or per-turn scanning — you run once, when L0 or
  the user asks, then stop. The L0 check in `cheaper-usage` owns
  candidacy detection.
- Workflow authoring, JSON generation, saving, catalog registration,
  grant/policy changes — all belong to `cheaper-workflow-create` (L2).
- Workflow execution — `workflow run` is never part of discovery.
- Reconstructing parent-agent history from `cheaper usage`/`history`,
  or estimating user cost from other sessions.
- A bespoke cost estimator or duplicate-detection engine — the catalog
  gives bounded discovery metadata, not a completeness proof.

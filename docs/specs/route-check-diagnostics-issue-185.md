---
id: "SPEC-034MMqMYf3tBIsJoiV5qLu"
type: "spec"
status: "proposed"
title: "Diagnose route configuration readiness with route check"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["cheaper maintainers"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/185"]
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
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/route-grant-set-projection.md"
    - "../adr/unified-exec-grant-model.md"
    - "../adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/explore-route-check-diagnostics-issue-185.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Diagnose route configuration readiness with route check

## What Should Be Built

Add `cheaper route check`, a diagnostic utility that inspects all configured routes across builtin, user, and project layers and reports their activation, provenance, static configuration readiness, and—only when a sample command is provided—dynamic resolution viability. It emits stable machine-readable diagnostic codes and actionable human-readable guidance.

## Why Now

`cheaper route resolve` answers one supplied command at a time. Users have no single view of all declared routes and why a route is inactive, suppressed, misconfigured, missing a grant, workflow, or target. The builtin `cargo-test` route intentionally opts in through a grant, so its default no-grant state is otherwise difficult to discover.

## Users and Value

Parent-agent and CLI users configuring routes, grants, workflows, and supported hook targets can identify the next configuration change without running a route or weakening policy.

## Scope

### In Scope

- Add `cheaper route check` with an optional project cwd, optional `--argv` or `--command` sample, and text/JSON format consistent with existing CLI argument conventions.
- Report every effective route's name, match prefix, target layer, activation/viability, suppressed provenance, grant and additional grant declarations, substitute declaration, and source-layer errors.
- Report static workflow path/validation, trust and grant requirements, executable pin/readiness, project shadow rules, route/workflow input contract, and target-specific rewrite renderer capability. Values not provable without input or host runtime are `unknown`/`requires-sample-input` or `unverified`.
- Reuse strict per-layer route loading/validation, route merge, resolver viability, workflow validation, and shared exec grant/pinning checks. Never duplicate policy predicates or use the hook fail-open loader.
- Keep a supplied sample's raw argv, command, and environment values out of diagnostic output and logs.
- Preserve a strict side-effect-free dry run: do not execute commands, workflows, or workers; do not change route/policy/credential configuration; do not write History, Usage, RouteLogs, tracing/observability file logs, or any other logs; do not authenticate or access the network; do not expose secrets. The no-filesystem-mutation guarantee has no exceptions: embedded builtin temp-cache materialization and CLI logging sinks must not create or modify files for this command, regardless of ambient logging settings.
- Provide stable diagnostic codes, remediation guidance tied to the configuration layer, and exit status classes distinguishing invalid configuration, warnings/partly inactive routes, and all-clear.
- Document that target renderer support is static capability only; host registration, approval, and successful host rewrite execution are not verified. #162 host verification remains out of scope and must be labelled `unverified`.

### Out of Scope

- Applying configuration fixes, issuing grants, relaxing policy, or adding project overrides.
- Executing routes/workflows/workers, contacting providers, or validating host-side registration/approval.
- Treating #162 real-host checks as part of this feature.

## Requirements

- Each builtin/user/project route layer is diagnosed independently and reported with its layer/source. An absent route file is a normal empty layer rather than a warning; unreadable, parse, or schema errors remain whole-file failures reported as a layer diagnostic without discarding diagnostics for other layers. Existing row-level invalid/suppressed representations are reported as such.
- A shared policy or denylist load failure is reported as a diagnostic; routes loaded from valid layers are still listed, and policy-dependent statuses become `unknown`/`unverified`.
- Without a concrete sample, command-argument pattern fit, unsafe-shell gate, interactive restrictions, and substitute target eligibility must not be reported as `runnable`.
- A static executable check does not claim a successful spawn; it reuses established pin/readiness policy and never spawns.
- The diagnostic command is read-only across filesystem, config, usage/history/log, credential, network, and process-execution boundaries.

## Acceptance Criteria

- [ ] JSON and text output describe every configured route in successfully loaded layers, causes, source layer/provenance, status, and repair guidance, using stable diagnostic codes; a failed layer is reported as an error entry without dropping other layers.
- [ ] Fixtures verify builtin `cargo-test` no-grant and grant-enabled states, missing/invalid workflow, project-layer violations, suppressed routes, and unresolved executable readiness.
- [ ] No sample input means dynamic argument fit and final activation remain `unknown`/`requires-sample-input`; sample input follows `route resolve`'s resolver/policy result.
- [ ] Cross-platform tests prove no route/workflow/worker spawn, no config/history/usage/route-log/credential/tracing file writes, no network/auth, no filesystem mutation including builtin temp-cache creation, and no raw argv/env secret leakage, including runs with ambient logging enabled.
- [ ] Tests distinguish static hook-renderer capability from unverified host registration/approval/execution.
- [ ] CLI help and `docs/guides/parent-agent-integration.md` / `configuration.md` describe usage, exit classes, and limits; canonical `mise run check` passes.

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- <!-- concern | reason | re-engagement trigger | risk if ignored -->

## Sources and Relations

Primary source: GitHub issue #185. Derived from discovery `explore-route-check-diagnostics-issue-185.md`. Existing constraints: implemented route resolver/workflow delegation SPEC and accepted ADRs for route resolution, grant-set projection, and unified exec grants. No new dependency is expected. A new ADR is not presumed unless design uncovers a cross-cutting architecture decision beyond those existing contracts.

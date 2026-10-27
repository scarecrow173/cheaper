---
id: "DISC-034MMpylycKIyUlHMbqW8z"
type: "discovery"
status: "active"
title: "Explore route check diagnostics for issue 185"
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
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Explore route check diagnostics for issue 185

## Exploration Goal

Determine how `cheaper route check` can explain setup readiness for every registered route while preserving the resolver's exact policy and merge contracts, and how its dry-run guarantee applies to embedded builtin materialization and host-specific hook capabilities. Issue #185 is explicit about no execution/config changes/logging, but the current shared loader can materialize embedded resources into a temp cache.

## Key Issues and Assumptions

- Confirmed: `route::supply::load` uses `embedded::builtin_dir()` when no builtin directory is supplied; `embedded::builtin_dir()` materializes embedded resources into a version/fingerprint temp directory. This is a filesystem write even though it is outside workspace, `.cheaper`, and user config.
- Confirmed: existing resolver viability and shared exec grant checks already encode trust, grant eligibility, pinning, denylist, argv, cwd, and unsafe-command policy. A diagnostic must call these contracts rather than reproduce them.
- Confirmed: route config schema/parse errors are file/layer failures; row-level invalid routes and suppressed provenance have explicit representations in `EffectiveRoutes`.
- Confirmed: renderer support for a target is statically knowable; host registration/approval and successful rewrite execution are not verified by this CLI. Issue #162's real-host validation is excluded.
- Assumption: `dry-run ... 副作用なし` means no filesystem writes, including builtin cache materialization; this needs explicit review against the issue wording before implementation is approved.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Reuse `RouteSupply::load` and accept its embedded temp-cache materialization as a runtime prerequisite | Reuses exact loader/policy behavior with little code | Conflicts with a strict reading of side-effect-free dry-run; must document the write as an exception | No, pending review |
| Add/reuse a read-only embedded-resource view and load builtin config without materializing to disk | Preserves strict no-filesystem-mutation guarantee | Requires a safe abstraction compatible with current path-based policy/route/workflow loaders | Preferred if implementable within existing boundaries |
| Restrict the no-write claim to user/workspace/history/usage paths | Lowest implementation cost | Silently narrows the issue's broad dry-run guarantee and omits a material effect | No |

## Tentative Conclusions and Hypotheses

Confirmed: use `routes::load`, strict `supply::load`/`load_with_builtin`, and the pure resolver for route merge and viability; never use hook `load_or_none` because it folds load errors into an inactive route arm.

Hypothesis: the command should fail closed on a builtin materialization write unless a read-only embedded resource path can satisfy the existing configuration loaders. The permitted interpretation of the dry-run contract is a review item; no narrow exception is assumed.

Confirmed: without sample input, argv matching, substitute eligibility, interactive restrictions, and final route activation remain `unknown` / `requires-sample-input`; prefix match alone cannot establish `runnable`.

## Open Questions

- Can the builtin resource view be consumed without filesystem materialization using existing embedded-resource APIs? Resolve during design/source audit.
- If strict no-write requires a new abstraction, is that still the smallest coherent change at the shared loader boundary? Resolve in design review.

## Promotion Candidates

- [x] spec-doc needed: all-route diagnostics, output/exit contract, sample-input boundaries, and no-side-effect guarantees.
- [ ] adr-doc needed: none expected if implementation reuses existing loader/resolver contracts; reconsider only if design requires a new cross-cutting resource-loading abstraction.

## Sources and Relations

Issue #185 is the primary product source. Consulted implemented SPEC `pretooluse-route-resolver-and-workflow-delegation`, accepted ADRs `pretooluse-auto-routing-via-route-resolver-and-workflow-delegation`, `route-grant-set-projection`, and `unified-exec-grant-model`. Relevant source locations: `route/supply.rs`, `route/routes.rs`, `route/resolve.rs`, `route_cmd.rs`, `embedded.rs`, and workflow loader/compiler.

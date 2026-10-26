---
id: "PLAN-034MGcX6T5YNz3OkB3yNKD"
type: "plan"
status: "approved"
title: "Implement Execution History for Issue 192"
created: "2026-10-27"
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
  implements:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  implemented-by:
    - "docs/tasks/execution-history-core.md"
    - "docs/tasks/execution-history-query-cli.md"
    - "docs/tasks/execution-history-worker-usage.md"
    - "docs/tasks/execution-history-carrier-catalog.md"
    - "docs/tasks/execution-history-route.md"
    - "docs/tasks/workflow-job-history.md"
    - "docs/tasks/hook-history-evidence.md"
    - "docs/tasks/execution-history-integration.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/execution-history-issue-192.md"
    - "docs/discovery/execution-history-issue-192.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/execution-history-storage-lifecycle.md"
    - "docs/test-specs/execution-history-bounded-queries.md"
    - "docs/test-specs/execution-history-identity-and-usage-ownership.md"
    - "docs/test-specs/workflow-job-history-lifecycle.md"
    - "docs/test-specs/hook-route-history-decision-evidence.md"
    - "docs/test-specs/execution-history-correlation-and-privacy.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement Execution History for Issue 192

## Goal

Deliver a privacy-bounded, per-execution History store and CLI, record accurate Worker/Workflow Job/Route/Hook lifecycle evidence, and correlate applicable new Usage rows without changing #184 aggregation semantics.

## Scope

Use one plan because all event owners share the History schema/store and release goal; keep implementation slices file-owned and sequence them behind the shared contract. Add a direct `uuid.workspace = true` dependency edge to `cheaper-cli` using the existing root workspace pin; no new package or version is introduced. #162 remains out of scope; #185 diagnostic utilities and #189 are later separate PRs.

## Upstream Documents

- Approved requirements: `../specs/execution-history-issue-192.md`.
- Accepted storage/identity decision: `../adr/record-per-execution-history-with-bounded-best-effort-storage.md`.
- Compatibility reference: `../specs/usage-analysis-scope-issue-184.md`.

## Design Inputs

- Approved implementation boundaries: `../designs/execution-history-issue-192.md`.
- Source exploration and alternatives: `../discovery/execution-history-issue-192.md`.

## File Map

| Files | Owner slice | Responsibility |
| --- | --- | --- |
| `crates/cheaper-cli/src/history/{mod,identity,schema,store,query}.rs`, `crates/cheaper-cli/src/lib.rs`, `crates/cheaper-cli/Cargo.toml`, `Cargo.lock` | History core | Register the module; add the direct `uuid.workspace = true` edge using the already pinned workspace v4 dependency; lock metadata may add only the existing UUID edge; typed record schema, bounded store, retention, query, cursor codec |
| `crates/cheaper-cli/src/args.rs`, `cli.rs`, `text_format.rs` | History CLI | Register list/show/trace grammar, read-only dispatch, text/JSON responses; keep Hook native responses unchanged |
| `crates/cheaper-cli/src/run.rs`, `usage_log.rs` | Standalone Worker and Usage | Start before worker resolution, terminal on returned result, additive execution ID projection where Usage is emitted |
| `crates/cheaper-cli/src/route/run.rs`, `route/logs.rs`, `route/resolve.rs` | Route/refetch | Typed pure resolution result; record decisions only at Route run/Hook owners; actual run guard and refetch guard before log read; preserve runId/logRef |
| `crates/cheaper-cli/src/hook/routing_policy.rs`, `hook/targets.rs`, `hook/eval_cache.rs` | Hook decisions | Typed decision evidence before native rendering; preserve route/read outcomes and dedup provenance |
| `crates/cheaper-cli/src/workflow_run.rs`, `crates/cheaper-workflow/src/engine.rs` | Workflow/Jobs | History-neutral state observer, distinct Job UUIDs, actual Workflow parent, skip/blocked evidence |
| `crates/cheaper-runtime/src/catalog/` | Catalog identity projection | Validate only History-projected provider/model IDs; omit invalid IDs without rejecting catalog entries or affecting execution |
| `crates/cheaper-runtime/src/run/mod.rs` | Runtime carrier source | Put the plain trace/parent values into `ToolRuntimeOptions` for the current execution; no History types in runtime APIs |
| `crates/cheaper-tools/src/invocation.rs`, `registry.rs`, `tools/exec/mod.rs`, `tools/proc/`, `command/`, `process/` | Child correlation carrier | Pass optional string carrier through `ToolRuntimeOptions`/`ExecToolOptions` to every managed spawn after `env_clear`; no dependency on CLI History types |
| `crates/cheaper-cli/src/history/schema.rs` | Schema/privacy tests | Metric boundaries, event-size drop, allowlisted projection, raw-field omission |
| `crates/cheaper-cli/tests/integration/main.rs`, `tests/integration/history.rs`, `tests/integration/hook_history.rs`, `tests/integration/route_history.rs` | CLI/Worker/Hook/Route integration tests | Register `mod history;`, `mod hook_history;`, and `mod route_history;`; verify test listing is nonempty before accepting filtered runs |
| `crates/cheaper-cli/benches/history_hook.rs` | Hook hot-path measurement | Standard-library benchmark comparing Hook decision with and without History write; no benchmark dependency |
| `crates/cheaper-workflow/tests/engine.rs`, `crates/cheaper-tools/src/tools/proc/tests.rs` | Workflow/process contract tests | Job transitions, carriers across `env_clear`, Windows/POSIX and child environment contracts |
| `docs/guides/usage.md`, `docs/designs/overview.md` | User docs | History commands, data/retention policy and design map |

File ownership is exclusive per task. `cli.rs` remains with the History CLI/dispatch slice; other lifecycle owners use their dedicated files and call the public recorder API. `route_cmd.rs` and the `cheaper route resolve` utility remain read-only and must not write History/Usage; `route/resolve.rs` stays a pure resolver/evidence source. Only Route run and authoritative Hook decision owners persist route outcomes.

## Implementation Sequence

1. Declare `pub mod history;` in `cheaper-cli/src/lib.rs` and add `uuid.workspace = true` to `cheaper-cli/Cargo.toml`. Reuse root workspace uuid v4 version; do not add a package/version or modify mise/APM locks.
2. Add focused contract fixtures and implement the History core API/store. Verify lazy creation, pending publication, valid-start preservation, first-terminal-wins, retention bounds, metric field omission, raw-field exclusion, event >8 KiB drop, and best-effort writer failure.
3. Add read-only `history list/show/trace` query and CLI. Register the integration module explicitly. Verify exact grammar/JSON/text shapes, per-file show read caps, stable ordering, complete-scan cursor and incomplete-scan no-cursor behavior; assert the module-filter list contains named tests before running it.
4. Wire standalone Worker command guard and additive Usage IDs. Start before worker/model/input/policy resolution; preserve existing Usage owner/cardinality and result behavior.
5. Wire Route decision/run/refetch owners. No-match/non-runnable from a Route run command is a decision only; actual runnable Route and refetch get their own execution ID and terminal path. Keep `cheaper route resolve` entirely read-only.
6. Add Workflow engine lifecycle observer and CLI adapter. Capture each Job transition, issue a separate Job UUID, parent it to Workflow, record terminal-only skip/blocked decisions, and keep Job Worker work out of standalone `kind:"run"`.
7. Add typed Hook and pure Route diagnostic evidence before response rendering. Preserve evaluator/native verdict/fallback distinction and cache leader/hit/fallback semantics without recording each Hook receipt.
8. Add explicit trace/parent carrier propagation through runtime options, tool invocation/exec options and managed spawn construction. Add History-only catalog identity projection. Keep received context unverified and preserve catalog/model behavior.
9. Complete focused cross-surface/privacy/concurrency tests, update usage guide, run canonical check and independent implementation review. Measure Hook overhead with the named benchmark; require p50 added latency ≤5 ms and p95 ≤25 ms per decision on local temp storage after 1,000 representative decisions, while proving no shared lock, full fsync, or external call.

## Dependencies and Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Retention deletes useful history or loses a valid start | Medium | High | Pending publication, valid-start orphan exclusion, bounded fixtures around retention/late terminal races; no destructive migration |
| Existing output/Usage cardinality changes | Medium | High | Snapshot contract tests for Hook stdout/stderr/exit, `kind` mapping, #184 semantics, and best-effort failures |
| Sensitive endpoint/payload leaks through identifiers or errors | Medium | High | Typed field projections, no raw fallback, private fixture audit, provider/model source-specific projection |
| Concurrent writes or Windows rename behavior differ | Medium | Medium | Unique IDs and `create_new`, same-parent pending rename, multi-process tests on Windows and POSIX CI |
| External dependency unavailable | Low | Low | No new package/version; reuse the already pinned workspace UUID and filesystem APIs |
| Parallel tasks edit shared entrypoint files | Medium | Medium | Exclusive file ownership table; tasks touching `cli.rs` are sequenced and not dispatched concurrently |

## Task Breakdown

Create one contract test spec for each durable behavior group after plan approval: store/retention; read-only query/paging; execution identity/Usage ownership; Workflow Job lifecycle; Hook/Route decision evidence; carrier/catalog privacy. Implement in the sequence above. Within each task, add the targeted fixture before changing its owner code and capture expected-red evidence when the test can compile against the existing public boundary. Avoid broad full-workspace checks after each slice; run the named focused command, then the canonical `mise run check` once at integration completion.

## Verification

| Slice | Command or check | Pass condition |
| --- | --- | --- |
| History core/privacy | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam history::` | Store/identity/query/schema tests pass, including metric zero/max/max+1, negative/fraction/nonfinite omission, raw sensitive field exclusion, and event size 8 KiB/8 KiB+1 |
| CLI and Worker | First list tests with `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history:: -- --list`; fail if output has no `history::` tests. Then run `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history::` | Named History CLI/Worker/Usage integration tests exist and pass; zero tests is failure |
| Hook decisions | List with `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history:: -- --list`; fail if output has no `hook_history::` tests. Then run without `-- --list` | Native output stays byte-compatible; typed route/read evidence, fallback and cache cardinality tests pass |
| Managed process | `mise exec -- cargo test -p cheaper-tools proc::tests` | Explicit carrier remains after `env_clear`; no grant/environment contract regression |
| Workflow lifecycle | List with `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -p cheaper-workflow --test engine job_history:: -- --list`; fail if output has no `job_history::` tests. Then run the same command without `-- --list` | Named job lifecycle tests exist and pass for success, failure, skip, blocked, cancel and timeout |
| Catalog projection | List with `mise exec -- cargo test -p cheaper-runtime catalog:: -- --list`; require a `history_identity` test. Then run `mise exec -- cargo test -p cheaper-runtime catalog::` | Official colon-bearing ID is preserved; invalid custom identity is omitted from History without changing model/catalog resolution |
| Route utility | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_history::` | The registered `route_history` module proves `cheaper route resolve` creates no History/Usage writes; Route run/Hook owners record decisions separately |
| Hook hot path | `mise exec -- cargo bench -p cheaper-cli --bench history_hook` | Across 1,000 local-temp decisions, added History write p50 ≤5 ms and p95 ≤25 ms; benchmark also confirms no shared append lock, full fsync, or network call |
| Documents and IDs | `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` | No blocking document/index/relations findings |
| Full canonical verification | `mise run check` | Exit code 0 |

If a focused test target differs from repository integration layout, keep the same module filter and use the actual target named by the test file owner; do not disable or skip a target.

## Rollback Strategy

- Trigger rollback if any focused contract shows altered native Hook output, policy/grant behavior, #184 aggregation, or History write failure changing a product result.
- Revert only the Issue #192 implementation commits on the feature branch; do not delete `<userConfigDir>/history/` or alter old `usage.jsonl` rows.
- Verify rollback with `git status --short`, the pre-issue Usage/Hook integration tests, and `mise run check`. Existing additive unknown Usage fields remain readable by current readers; the unreferenced History directory is left intact for a later compatible implementation.

## Open Questions / Gaps

- None blocking planning. Exact concrete enum variants and builder names are owned by the implementation tasks, constrained by the finite semantic vocabulary in the approved spec/design/ADR. No task may add free-form reason text or change accepted caps.

## Self-Review

- [x] Every spec acceptance group maps to a task slice and a behavior test spec.
- [x] No placeholders remain; file ownership and verification commands are named.
- [x] File paths, relation targets, status values, and terminology match the upstream spec/design/ADR.

## Implementation Handoff

- Recommended mode: delegated/subagent-capable implementation after task DAG validation; run independent owner slices only after History core API is stable.
- Independent work streams: Workflow/Job observer, Hook typed evidence, and Route command lifecycle can proceed in parallel after the core API; child-carrier and catalog-projection work follows core context types and has separate file ownership.
- User approval required before dispatch: already authorized for this Issue #192 work in the session; use only the available local worker capability and keep each task's file ownership explicit.
- If a slice cannot be delegated, execute it inline in the same dependency order and retain its review checkpoint.

## Review Handoff

- Assumptions: no new package/version; the existing workspace UUID is declared as a direct cheaper-cli dependency; best-effort History may be absent; Usage rows remain owned by the existing recorder; inherited process context remains unverified.
- Review focus: check each event owner against the design lifecycle table, verify no task stores raw data, and ensure every write path remains best-effort while every query path is read-only and bounded.
- Approval evidence (2026-10-27): fresh independent A/B plan review, separate Validator, and separate Judge completed under the user's authorized review-protocol approval substitute; the Judge returned `ACCEPT` with no confirmed P0/P1 blockers.

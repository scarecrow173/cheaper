---
id: "DESIGN-034MFy7WDFzWfi8i74Cd6J"
type: "design"
status: "approved"
title: "Execution History storage, identity, and query design (Issue 192)"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/192"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/usage-analysis-scope-issue-184.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/execution-history-issue-192.md"
    - "../adr/record-per-execution-history-with-bounded-best-effort-storage.md"
    - "../discovery/execution-history-issue-192.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Execution History storage, identity, and query design (Issue 192)

## Context

The accepted spec establishes per-unit execution identity, meaningful decision records, additive usage IDs, read-only query, privacy allowlists, and incomplete status on missing terminal events. The accepted ADR selects immutable per-execution files with bounded best-effort retention. This design maps those contracts to existing Rust CLI/runtime ownership boundaries without combining usage aggregation and history event cardinality.

The implementation uses the workspace `uuid` v4 dependency and existing user config directory resolution. It adds no dependency and does not change `usage.jsonl` semantics or Hook native output.

## Scope

### In Scope

- Typed history identity/context, record projection, store, retention, and query modules under `crates/cheaper-cli/src/history/`.
- Lifecycle wiring in `crates/cheaper-cli/src/cli.rs`, `run.rs`, `workflow_run.rs`, `route/run.rs`, `route/logs.rs`, and Hook targets; a History-neutral job observer in `crates/cheaper-workflow/src/engine.rs`.
- Correlation carrier values through `crates/cheaper-tools/src/tools/proc/`, `command/`, and `process/` environment preparation.
- Safe catalog ID projection at `crates/cheaper-runtime/src/catalog/` without changing catalog acceptance or model resolution.
- A pure typed Hook/Route diagnostic result that preserves evaluator reason, measurement, viability, and final native verdict before formatting.
- Explicit process-carrier propagation through the existing environment allowlist boundary.
- `cheaper history list|show|trace` text/JSON command integration and additive IDs on applicable new Usage records.
- Contract tests for storage publication, writer races, privacy, query bounds, and owner/cardinality mapping.

### Out of Scope

- Changing #184 usage aggregation, `usage --since` parsing, refetch joins, or historical record backfill.
- Inferring host behavior after Hook output or treating Hook Deny/Rewrite as a Worker/Route execution.
- Full input, argv, environment, tool, prompt, path, or error-body transcript storage.
- Implementing #162, #185, or #189 in this PR.

## Components and Boundaries

| Component | Responsibility | Side effects / authority |
| --- | --- | --- |
| `history::identity` | Generate local UUID v4 IDs; hold actual in-process parent and inherited-unverified context separately | No policy/grant inputs; env carrier never proves parenthood |
| `history::schema` | Typed allowlisted event and projection types; bounded field validation | Reject free-form reason/status; omit only an invalid metric field while keeping the event |
| `history::store` | Lazy per-execution directories, pending publication, immutable JSON files, bounded sweep | Best-effort only; returns failure to recorder boundary, never to product operation |
| `history::query` | Read and project retained events for list/show/trace | No create, write, prune, Usage append, provider access, or execution |
| CLI history command | Parse filters/cursor/page size and render text/JSON | Read-only; does not invoke the record API |
| Lifecycle owners | Decide when an observed execution/decision starts, ends, or is meaningful | Own actual event truth and execution/cardinality mapping |
| `usage_log` projection | Add execution/correlation IDs to applicable new Usage records | Existing `kind`, count, and #184 aggregation remain unchanged |
| `cheaper-workflow` lifecycle observer | Emit History-neutral Job transition events from the engine state machine | Does not depend on CLI History types and cannot change workflow outcome |
| Hook diagnostic evidence | Preserve typed evaluator/route result before native Hook rendering | Pure data only; no storage side effect and no Hook output change |
| Process correlation carrier | Carry trace/parent hints through managed child spawn after `env_clear` | Explicit environment values only; receiver always treats them as unverified hints |
| Catalog history identity projection | Expose validated provider/model identity fields from the resolved catalog object | Invalid IDs omit only History identity; catalog acceptance and execution resolution are unchanged |

Catalog provider/model fields must enter history through a typed resolved identity projection containing only the catalog provider ID and model ID. At the projection boundary, validate these projected strings against the accepted 1–128 ASCII allowlist and reject URL markers (`://`, `?`, `#`, `@`) without applying a generic URI parser to IDs. This is a History-only projection check: it does not reject catalog definitions or alter model resolution. `base_url`, endpoint, raw `--model`/environment input, and display label are never projection inputs. Official IDs such as `amazon.nova-2-lite-v1:0` remain intact. An invalid custom identity is omitted from History, while model resolution and execution behavior remain unchanged. Pre-resolution errors have no resolved model/provider fields.

## Data and Control Flow

### Identity and record lifecycle

1. At each execution boundary, create a local UUID v4 `executionId`. A root gets a local `traceId` unless a valid inherited trace hint is present; inherited hints carry `contextProvenance:"inherited-unverified"`. An actual `parentExecutionId` is populated only when this process's lifecycle code directly owns the parent-child relation. An incoming parent carrier is stored as `parentExecutionHintId`, never as a verified edge.
2. Before execution starts, create a unique pending directory below `<userConfigDir>/history/`, write the complete bounded `started.json`, and rename the directory to its final `<executionId>` name. If any step fails, drop the History write and proceed with the product operation. The started record is published before work begins.
3. Completion writes one immutable `terminal.json` with `create_new` into the existing final directory. It verifies a complete matching started record and never creates/recreates a directory. First terminal wins. Missing or invalid terminal leaves a valid started record visible as incomplete/unknown.
4. One-shot Hook/Route decisions similarly write a `decision.json` inside a pending directory then publish by rename. The `recordedAt` is its decision time. Decisions do not fabricate a Worker or Route run.
5. Retention runs only before started/decision writes. It accounts for all visited entries against count/byte/time budgets, removes pending directories older than 24 hours, and removes a final directory as a 24-hour orphan only when both `started.json` and `decision.json` are missing or invalid. A valid start with a missing/corrupt terminal is retained as incomplete subject to ordinary 30-day/count/byte retention. A late terminal after deletion is dropped.

`started.json`, `decision.json`, and `terminal.json` each contain one schema-versioned JSON object and required UTC `recordedAt`. The execution's list ordering timestamp is its start timestamp; a decision-only row uses its decision timestamp. Query sorts by `(recordedAt DESC, executionId DESC)` and uses that pair for keyset cursors.

### Command start guards

History start belongs to the CLI command boundary, before any fallible operation that the spec requires to be observable. A small `ExecutionRecorder` guard owns the generated execution context and best-effort start/finish calls; its errors are swallowed at the boundary. The central command result path attempts one terminal record for normal return and typed error return. A process kill naturally leaves only `started.json`.

| Command boundary | Start point | Terminal/decision behavior | Usage ownership |
| --- | --- | --- | --- |
| `cheaper run <worker>` | After command syntax/config-dir selection, before worker/model/input/policy resolution | Every returned worker resolution, setup, runtime, or execution error closes as failed; force-kill leaves incomplete | Add same execution ID only when existing recorder emits `kind:"run"` |
| `cheaper workflow run` | Before workflow load/compile/bind/engine setup | Workflow execution guard closes on each returned result; Job events handled by observer below | Workflow envelope and Job-owned Worker work add no standalone `kind:"run"` |
| `cheaper route run` / dispatch | Record pure resolver outcome at decision boundary; create Route execution `started` only after route is resolved runnable and immediately before invoking it | no-match/no-grant/no-pin/not-runnable is a `route-decision` decision file without `started.json`; actual run gets terminal on return | Only actual Route run uses existing `kind:"route"` row and same ID |
| `cheaper route logs <runId>` | Before reading/fetching the referenced Route log | read success, missing log, permission, decode, or I/O error closes terminal; kill leaves incomplete | Successful existing `kind:"route-refetch"` row gets its own same execution ID |
| Hook/Route decision | After pure evaluator returns a meaningful typed result, before native response formatting | One-shot `decision.json`; logging failure does not affect native verdict | No Usage row |

`ExecutionRecorder` is held by dispatch/result orchestration, not `run_worker`, so it includes pre-usage failures. It passes the same context into `build_usage_record`/`build_route_record`/`build_route_refetch_record` when those existing builders are called. `RunCtx` may carry the IDs to internal code, but the dispatch guard remains owner of terminal-on-return handling. Parser syntax failures before a command is identified are outside this command execution boundary.

### Workflow Job lifecycle boundary

`cheaper-workflow` owns Job state transitions but must not depend on CLI History modules. Add an optional `WorkflowJobObserver` interface to the engine ports using only workflow-domain values (`job_id`, job kind, dependency IDs, typed transition/status/reason, timestamps, observed metrics). Emit `Started` at the state transition before executor dispatch and `Terminal` from the centralized terminal transition path for completed, failed, cancelled, timeout, skipped, blocked, and admission failure states. This covers transitions that do not call the executor.

The CLI supplies an observer implementation scoped to the Workflow execution. It maps each logical `job_id` to a fresh UUID v4 `executionId` and uses the Workflow guard ID as the actual `parentExecutionId`. `job_id` remains the display/config identity; it is never reused as execution ID. On `Started`, the observer publishes `started.json`. On a terminal event with a prior start, it publishes `terminal.json`; a terminal-only skip/blocked event publishes a `decision.json` with `kind:"job"`, status, reason, dependencies, and Workflow parent. The observer attaches Worker identity/metrics available in the Job outcome and does not call standalone Worker usage recording. Observer persistence errors are ignored by engine execution.

The observer is a callback at the transition owner, not reconstructed from final `WorkflowOutcome`; therefore skipped/blocked Job transitions and earlier starts survive even if later jobs fail or the process exits. `WorkflowExecutionContext` receives the observer from the CLI run boundary without importing any History schema/type into `cheaper-workflow`. The observer is called exactly once when each Job enters running and once when it reaches a terminal state; if a terminal status is emitted without a prior running event, it creates a decision-only Job record. The engine event contains the logical `job_id`, job kind, dependency IDs, transition, finite status/reason, event time, and available typed metrics. The CLI observer maps `job_id` to execution UUID and captures the enclosing Workflow UUID. Invoke callbacks after the engine commits a state transition and outside its state lock. The CLI observer's per-run `job_id -> executionId` map lock is held only for lookup/insert, never during file I/O. Event callbacks are synchronous, bounded, and best-effort; storage latency/failure is isolated by the CLI adapter and cannot abort engine state transitions.

### Typed Hook and Route evaluation boundary

`hook::routing_policy` returns a typed `HookDecisionEvidence` before `hook::targets` renders `HookResponse`. It has finite evaluator reason/outcome enums, optional typed measurements (`fileBytes`, threshold, measured tokens/null and measurement provenance), optional typed Route evaluation (`matched`, existing viability, finite no-match/no-grant/no-pin/not-runnable/rewrite/fallback reason), canonical evaluator verdict, and final native verdict. It contains no path, free-form reason, or payload. Target-specific native formatting consumes this object and must retain existing stdout/stderr/exit byte contracts.

The persisted schema uses `evaluatorVerdict` and `nativeVerdict` together for newly written `hook-decision` events; generic `verdict` remains for non-Hook decisions and is not duplicated on new Hook events. Native verdict is the finite target outcome `allow`, `deny`, or `rewrite`. `fallbackReason` is an optional `ReasonCode` and is set only when a finite evaluator Rewrite was rendered as a non-Rewrite native fallback. `hookCacheProvenance` uses `leader-evaluated`, `cache-hit`, `fallback`, or `bypass`. These fields are optional additive fields in this branch's initial schema v1: new readers accept older events with the fields absent; older binaries are not expected to read events emitted after this feature is introduced.

Hook Search reasons are projected only from observed evaluator branches, with no policy behavior change or raw error storage:

| Authoritative Search branch | History `reasonCode` | Other persisted evidence |
| --- | --- | --- |
| No candidates, out of scope, or empty materialized result | `search.no_candidate` | Evaluator verdict only |
| Measured bytes or tokens below configured threshold | `search.below_threshold` | Available bounded byte/token measurement and threshold |
| Measured tokens exceed configured threshold and delegation is advised | `search.threshold_exceeded_delegate_advised` | Evaluator verdict, measured tokens, threshold |
| Search scan cap reached | `search.scan_limited` | Only metrics actually measured before the cap |
| Tokenization, timeout, or scan failure with fail-open result | `search.measurement_unavailable` | Null measurement only when source explicitly reports no measurement |

If the evaluator does not provide enough typed evidence to distinguish a row, omit `reasonCode`; never infer from a free-form error or command/query content. Cache hits/followers retain `cache-hit` in in-process evidence but do not write duplicate History records. A leader-written record keeps `leader-evaluated`; fallback and bypass records retain their corresponding provenance.

The resolver-facing result must retain detail before the current `Option<Verdict>` fallthrough: an unselected route arm returns a typed reason and viability to the Hook decision owner, while existing route execution/resolution API wire values remain unchanged. A fallthrough read evaluation may yield its own typed evidence; persist only meaningful decisions. The pure evidence result is reusable by side-effect-free route diagnostics such as the later #185 utility; persistence is performed only by the authoritative Hook command owner.

`HookEvalCache` stores and returns the typed evidence together with provenance (`leader-evaluated`, `cache-hit`, `fallback`, `bypass`). Only the leader that computed and published a deduplicated result writes the decision record; cache hits/followers do not create duplicate records. A cache bypass/fallback writes only if it produced a meaningful typed result. Cache identity excludes raw path/input from persisted data; cache storage keeps only existing cache state and typed decision projection. Unsupported Rewrite records evaluator Rewrite and native fallback Deny as separate fields.

### Managed child carrier boundary

The CLI creates a plain correlation carrier (canonical trace UUID and current execution UUID) and supplies it to runtime/tool execution context. `cheaper-tools` treats these as explicit optional environment values, not ambient variables. The process, command, and spawn preparation paths that intentionally call `env_clear()` add these two entries from the supplied carrier after clearing and alongside their existing explicit environment. No grants or user environment allowlist is changed. The child creates its own execution ID and stores received values only as unverified hints; no child environment value may set a verified parent edge. Arbitrary external programs spawned by a tool are not observed as executions; if they forward the hint to another cheaper process, that receiver still marks it unverified.

### Recommended implementation ownership

| Slice owner | Primary paths | Responsibility |
| --- | --- | --- |
| Store and query | `crates/cheaper-cli/src/history/`, `args.rs`, `cli.rs`, `text_format.rs` | Schema, publication, terminal first-writer, retention, bounded reads, cursor codec, history CLI |
| Standalone Worker and Usage IDs | `crates/cheaper-cli/src/run.rs`, `cli.rs`, `usage_log.rs` | Start before resolution, terminal on result, applicable Usage linkage; supply current execution correlation to managed tools |
| Workflow and Jobs | `crates/cheaper-workflow/src/engine.rs`, `crates/cheaper-cli/src/workflow_run.rs` | History-neutral lifecycle observer, shared Job UUID map, skip/blocked terminal-only decisions, Workflow parent relation, reuse the issued Job UUID as the child carrier |
| Route and Hook decisions | `crates/cheaper-cli/src/route/`, `hook/routing_policy.rs`, `hook/targets.rs`, `hook/eval_cache.rs` | Typed Route/Hook evidence, target-native verdict projection, route run/refetch guards, cache dedup semantics; pass actual Route identity into routed Workflow context |
| Managed child propagation | `crates/cheaper-runtime/src/run/boundary.rs`, `run/mod.rs`, `crates/cheaper-tools/src/invocation.rs`, `registry.rs`, `tools/exec/`, `tools/proc/`, `command/`, `process/` | Explicit typed carriers after `env_clear`, no CLI History dependency in tools/runtime |
| Catalog identity projection | `crates/cheaper-runtime/src/catalog/` and CLI History projection adapter | Validated resolved provider/model identity only; omit invalid history identity without rejecting catalog/model execution |
| User docs and evidence | `docs/guides/`, focused integration tests, `docs/designs/` | Usage, privacy, lifecycle and cross-platform contract tests |

### Event ownership and Usage mapping

| Lifecycle owner | History representation | Usage row |
| --- | --- | --- |
| Standalone `run` | One Worker execution with start and optional terminal | Existing applicable `kind:"run"`, same execution ID |
| Workflow invocation | Workflow execution envelope | No independent `kind:"run"` |
| Workflow Job | Job execution; its Worker identity/metrics are projected onto Job | No independent Worker `kind:"run"` for the same work |
| Route decision | One decision execution for meaningful resolver outcome | None |
| Route run | One Route execution, preserving route `runId` and `logRef`; child Workflow/Job IDs when applicable | Applicable `kind:"route"`, same Route execution ID |
| Route log refetch | One independent refetch execution referring to source only by `logRef` | Applicable `kind:"route-refetch"`, own ID |
| Hook decision | One decision record with evaluator outcome and native verdict/advice distinction | None |
| `count-tokens` | Outside Issue #192 History | Existing `kind:"count"` unchanged |

Same `traceId` does not deduplicate rows. A History write and Usage append are independent best-effort attempts; one missing side does not synthesize the other. #184 continues to join refetch rows only by its existing in-window `route` plus nonempty `logRef` ownership rule.

### Decision and failure semantics

| Observation | Stored evidence | Do not infer |
| --- | --- | --- |
| Hook Deny with delegation advice | Hook decision, typed reason, advice verdict | Worker started, skipped, or never started |
| Hook Rewrite emitted | Evaluator Rewrite plus target-native verdict (including fallback Deny when unsupported) | Host performed a route run |
| Route resolver produces no-match/no-grant/no-pin/not-runnable | Typed Route decision outcome/viability, even when Hook policy falls through to another evaluator | Resolver outcome is lost in generic Allow or Route execution started |
| Route resolver chooses a route | Route decision and viability | Route execution started |
| Route execution starts | Route started record and existing `runId`/`logRef` mapping | External host action after response |
| Process killed after start | Valid start without terminal; CLI reports incomplete/unknown | Success or a synthetic failure reason |
| Store unavailable/corrupt/write denied | No or partial History evidence | Product result/error modified by History failure |

Reason, event, status, verdict, viability, and error code use finite typed enums. Worker/provider/model/workflow/job/route IDs and route `logRef` use their specified validators. Raw values are never copied as a fallback when projection fails. Metric conversion is checked and nonnegative with the accepted field caps; an invalid metric omits only that field and preserves the otherwise-valid event. A serialized event over 8 KiB is dropped as a whole. `exitCode` is a separate signed `i64` JSON integer field and does not use metric validation.

### Query and pagination

- Grammar: `cheaper history list [--since <Nm|Nh>] [--worker <id>] [--kind <kind>] [--status <status>] [--limit <1..500>] [--cursor <opaque>] [--format text|json]`; `cheaper history show <execution-uuid> [--format text|json]`; `cheaper history trace <trace-uuid> [--limit <1..500>] [--cursor <opaque>] [--format text|json]`. Default format is text; list/trace limit defaults to 50. `--since` is a relative minutes/hours window local to History; this does not change `usage --since` grammar.
- The cursor is an opaque base64url encoding of version 1 JSON `{ "recordedAt": <UTC RFC3339>, "executionId": <UUIDv4> }`; decode is length capped at 512 bytes and validates both fields. It encodes no path, offset, or authority. Unsupported/malformed versions fail as invalid CLI input before any file access.
- List and trace JSON shape is `{ "records": [...], "scanComplete": true|false, "nextCursor": <string|null> }`; trace additionally includes `traceId`. Text output prints the same records and, when incomplete, a visible `incomplete scan; narrow filters or restart query` footer. `nextCursor` is null unless scan completed and additional matching rows exist.
- Show reads only the requested UUID directory and at most the three known event files. Each file reader consumes at most 8 KiB + 1 byte; over-limit, malformed, or truncated files are ignored while other valid files remain visible, and the JSON response marks `recordState:"incomplete"` or `"corrupt"`. JSON shape is `{ "record": <object|null>, "recordState": "complete"|"incomplete"|"corrupt"|"not-found" }`; missing ID returns `record:null` with `recordState:"not-found"` and does not create storage. Text form states `not found` or `incomplete/corrupt` without exposing raw file contents.
- Filters are applied to projected fields.
- Query never creates the store or performs retention. A missing store returns an empty result.
- A complete scan may return a keyset `nextCursor` for `(recordedAt, executionId)`. If 20,000 directories, 128 MiB event data, or 500 ms is reached first, return available rows with `scanComplete:false`, omit `nextCursor`, and clearly label the result incomplete. This bounds work without promising continuation past unscanned entries.
- Pagination is guaranteed when the retained dataset can be fully scanned within budget. Concurrent additions may not appear on later pages; restarting at the first page includes them. Query memory is limited to the requested page, bounded path metadata, and one event object at a time. `show` reads at most three known files with a per-file 8 KiB + 1 byte cap.

## Non-Functional Constraints

- Store default path is `<userConfigDir>/history/`; lazy creation on writer path only.
- Event object ≤8 KiB. Retention targets 30 days, 10,000 execution directories, and 128 MiB logical event bytes. Sweep and query each stop at 20,000 entries, 128 MiB, or 500 ms. These are best-effort targets, not hard cross-process quotas.
- No shared append lock, descriptor held for execution lifetime, per-event fsync, or external logging. Event files use exclusive creation; execution directories are unique UUIDs.
- History recording failure must not change Hook verdict/output/exit, Worker result, Route result, Workflow state, or policy behavior.
- Windows and POSIX path behavior plus custom config directories must be covered. Pending publication and final record names share a filesystem parent for rename.

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| Many files make scans expensive | Count/time/byte budgets, bounded retention, explicit incomplete output |
| Retention races an active execution | No handle/lock; late terminal cannot recreate deleted record; product operation remains unaffected |
| Caller forges correlation carrier | Separate inherited hint/provenance from actual parent edge; never use for grants/policy |
| Partial event files | Validate per-file object; preserve valid start; first terminal wins; do not synthesize state |
| Catalog identity projection accidentally stores endpoint | Typed projection receives only provider ID/model ID; test separate `base_url` never appears |
| One side of Usage/History recording fails | Independent best-effort write; no guessed repair/backfill |

## Deferred Design Concerns

- Changing retention values requires measured fixture evidence and an ADR update; current accepted limits remain the contract for this implementation.
- A database/indexed backend is deferred until representative complete-scan fixtures show the bounded JSON design cannot meet the stated scan limits.

## Verification Notes

- [ ] Test pending-directory publication, pre-start crash cleanup, and writer failure without altering the wrapped operation.
- [ ] Test valid started plus partial/missing terminal survives orphan cleanup and remains incomplete; test invalid start and decision-only orphan cleanup after 24 hours.
- [ ] Test duplicate terminal writers, late terminal after retention, and concurrent execution IDs do not overwrite each other.
- [ ] Test standalone Worker start before worker/model/input/policy resolution, including unknown worker and setup failure; test Route decision-before-run, actual Route start only after Runnable resolution, and refetch start before log reads.
- [ ] Test workflow engine emits exactly one start and terminal event per executed Job, terminal-only decision for skip/blocked/admission failures, distinct UUID from logical `job_id`, and actual Workflow parent UUID.
- [ ] Test Hook typed evidence for byte fast path, threshold deny, route no-match/no-grant/no-pin/not-runnable, supported Rewrite, and unsupported Rewrite fallback without changing target-native output. Verify cache leader records once, follower/hit does not duplicate, and fallback/bypass preserves typed outcome.
- [ ] Test child command/process env after `env_clear` receives explicit trace/parent carriers; child execution creates its own UUID and labels received IDs unverified.
- [ ] Test hook/route decision ownership and Worker-vs-Workflow-Job-vs-Route Usage cardinality against the ownership table.
- [ ] Test carrier provenance: an inherited parent hint remains unverified even if a matching ID exists; actual in-process parent remains an edge.
- [ ] Test official catalog ID `amazon.nova-2-lite-v1:0` is projected while `base_url`, raw model input, endpoint, and display label are absent. Invalid custom IDs omit only History identity and do not change catalog resolution or execution.
- [ ] Test metric zero/max/max+1, negative, fraction, NaN/infinity, and signed `exitCode`; invalid metrics omit only the field while preserving a valid event, and signed exit code is preserved.
- [ ] Test decision-only ordering, complete-scan keyset paging, and `scanComplete:false` with no `nextCursor` at each budget boundary; test cursor wire format and malformed cursor rejection before storage reads.
- [ ] Test `show` reads at most three files and each file at most 8 KiB + 1 byte; oversize/truncated files do not prevent other valid files from being shown.
- [ ] Test JSON/text shapes for complete/incomplete/corrupt/not-found, including visible incomplete scan indication.
- [ ] Test query against missing/corrupt/partial store performs no filesystem mutation, Usage write, retention, or execution.
- [ ] Run focused integration tests and `mise run check` per the approved plan.

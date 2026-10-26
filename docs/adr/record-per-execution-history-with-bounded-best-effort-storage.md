---
id: "ADR-034MEdx0Fka9B0j6O5yPOf"
type: "adr"
status: "accepted"
title: "Record per-execution history with bounded best-effort storage"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/execution-history-issue-192.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/execution-history-issue-192.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/execution-history-issue-192.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/usage-analysis-scope-issue-184.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Record per-execution history with bounded best-effort storage

## Context and Problem Statement

Issue #192 requires history for usage-free failures, workflow jobs, route runs, and decisions. `usage.jsonl` is a separate aggregate contract and `route-logs` stores large command output. Its current append helper has no cross-process lock; putting history into that file would mix data and retain its concurrent-write risks. History writes must not change the operation being recorded, and queries must not create or mutate storage.

## Decision Drivers

* Do not add a runtime dependency or modify `usage.jsonl` aggregation/cardinality.
* Support simultaneous cheaper processes without interleaving records or waiting on a shared hot-path lock.
* Keep reads bounded and side-effect free; preserve crash-visible started records as incomplete/unknown.
* Bound retained data and query cost without shared append locks or open files held for an execution's lifetime.
* Store classified, allowlisted data only; never persist raw prompt/input/argv/env/tool text/credentials/error bodies.

## Considered Options

* One shared `history.jsonl` with a cross-process lock and recovery protocol.
* One uniquely created JSONL segment per CLI process/session; only that process appends to it.
* One execution directory with immutable started and terminal decision files, created by the execution owner.
* SQLite or another embedded indexed database.

## Decision Outcome

Chosen option: "one directory per execution with immutable event files", because each execution ID is locally generated and owns its directory, each write opens a file only for that event, and no execution holds a descriptor or shared append lock while running. A started file remains useful if the process dies; a separate terminal file cannot overwrite it or race another execution's records. `usage.jsonl` remains unchanged except for optional IDs.

### Consequences

* Good, because `create_new` starts a unique execution directory and immutable `started.json`/`decision.json`/`terminal.json` files; different executions never append to the same file.
* Good, because a crash after a complete start leaves it visible as incomplete/unknown; a malformed terminal file never replaces the valid start.
* Good, because terminal writers never recreate a directory removed by retention and first terminal file wins; a late or duplicate terminal write is dropped.
* Good, because readers open existing files only; absent store remains absent and no query performs retention, Usage writes, or execution.
* Bad, because History creates multiple filesystem entries per execution and list/trace scans cost grow with retained records; caps and scan limits must be measured.
* Bad, because per-execution storage is best-effort: retention can expire an old running execution, and concurrent starts can temporarily exceed the retention target.
* Bad, because a partial started file is unusable; a partial terminal file is ignored and the execution remains incomplete. No recovery fabricates a success.

### Record and identity contract

* The store is lazy: only a started execution or meaningful decision write may create `<userConfigDir>/history/`; `history list/show/trace` never creates it.
* A locally generated UUID v4 `executionId` names one execution directory. An execution can have `started.json` and one `terminal.json`; a one-shot Hook/Route decision has `decision.json`. Each file contains one bounded JSON object with `schemaVersion`, required UTC `recordedAt`, `executionId`, optional `traceId`, optional actual `parentExecutionId`, optional `parentExecutionHintId` and provenance, kind, status/event, and optional finite `reasonCode`.
* A started record is published before work begins. The terminal writer opens only an existing directory with a valid started record and creates `terminal.json` with `create_new`; it never creates/recreates the directory. The first terminal write wins. A duplicate terminal, malformed terminal, or late terminal after retention is dropped; the reader reports incomplete/unknown rather than replacing the first status.
* A process kill during a terminal write leaves the complete started record plus an invalid/missing terminal. The reader ignores the invalid terminal only and shows the execution as incomplete. A partial started/decision file is ignored as an unrecorded best-effort write.
* Publish a started execution from a uniquely named pending directory: create it, write complete `started.json`, then rename to the final UUID directory before work begins. Readers ignore pending directories. A bounded writer sweep removes pending directories older than 24 hours; pending entries count against the sweep budget. For final UUID directories, 24-hour orphan cleanup applies only when neither a valid `started.json` nor a valid `decision.json` exists. Validity requires a complete schema, matching directory `executionId`, and expected event type. A valid start remains a retained incomplete execution even if its terminal file is missing or corrupt; it is subject only to normal 30-day/count/byte retention. This prevents a started record from being deleted as a pre-publication orphan.
* Trace/parent hints use `CHEAPER_TRACE_ID` and `CHEAPER_PARENT_EXECUTION_ID`; parse only canonical UUID v4 values. The current process always creates its own `executionId`. Managed child processes receive parent and trace values explicitly, but environment carriers are forgeable and are always stored/displayed as unverified hints. Only a parent relation established by the current process lifecycle may populate `parentExecutionId`; incoming parent values use `parentExecutionHintId` with `contextProvenance:"inherited-unverified"`. Hints never authorize policy/grants and are never inferred from names, paths, `logRef`, or timing. Invalid/missing hints start an unrelated root. Hook native output remains unchanged, so it cannot carry the later host action.

### Field allowlist

* `kind` is one of `worker`, `workflow`, `job`, `route`, `hook-decision`, `route-decision`, or `route-refetch`. `status` is one of `running`, `completed`, `failed`, `cancelled`, `timeout`, `skipped`, or `blocked`. `event` is `started`, `decision`, or `terminal`. `verdict` is one of `allow`, `deny-delegation-advised`, `rewrite-emitted`, `not-routed`, or `unknown`; `viability` uses the existing Route Viability enum. `errorCode` uses existing `cheaper_core::errors::ErrorCode` values, never error text.
* `reasonCode` is a versioned finite enum, initially: `read.below_threshold`, `read.byte_fast_path`, `read.threshold_exceeded_delegate_advised`, `read.measurement_limited`, `read.measurement_failed_fail_open`, `read.no_candidate`, `read.out_of_scope`, `search.no_candidate`, `shell.no_candidate`, `route.no_match`, `route.no_grant`, `route.no_pin`, `route.not_runnable`, `route.rewrite_emitted`, `route.rewrite_unsupported_fallback_deny`, `workflow.job_dependency_failed`, and `run.worker_resolution_failed`. Adding a reason is an explicit schema change; callers cannot provide free-form values.
* `workerId`, `workflowId`, and `routeId` are validated identifiers: 1–128 ASCII bytes from `[A-Za-z0-9._:/-]`. `providerId` and `modelId` come only from a typed resolved catalog identity projection that selects the provider identifier and model `id`; never project `base_url`, endpoint, raw `--model`/environment input, or display label. The History-specific identity projection validates only these projected fields against the same 1–128 ASCII allowlist and rejects URL marker forms (`://`, `?`, `#`, `@`) without a generic URI parser, preserving official IDs such as `amazon.nova-2-lite-v1:0`. This does not validate/reject catalog definitions or change catalog composition, model resolution, or execution. Invalid custom identity is omitted from History. Pre-resolution failure has no model/provider identity.
* `executionId`, `traceId`, `parentExecutionId`, and `usageRecordRef.executionId` are canonical UUID v4 strings. Route `logRef` must pass the existing route-log ID validator. IDs are not free-form.
* Metric fields use checked finite nonnegative integer serialization (`0 <= value <= cap`): `durationMs` ≤ 315,360,000,000 (10 years); `turns` and `toolCalls` ≤ 1,000,000; `fileBytes` ≤ 1,099,511,627,776 (1 TiB); `threshold` and `measuredTokens` ≤ 1,000,000,000. Negative, fractional, non-finite, overflowed, or over-cap values are omitted, never cast or clamped. `measuredTokens` is JSON null only when no token measurement occurred. `exitCode` is separate from nonnegative metrics: preserve an observed signed integer in `i64` range as a JSON integer; omit missing/signal, fractional, or out-of-range values. This retains the source sign and does not conflate process status with cheaper's own CLI error code. Each serialized event is at most 8 KiB; event data that exceeds the limit is omitted and the write is dropped rather than truncated into misleading JSON.
* Do not persist full paths, basenames, command/argv text, Hook stdin, input JSON, prompts, tool arguments/output, raw environment, API keys, credential values, or error messages. Hook decision records may include only the decision kind, typed verdict/reason, `fileBytes`, configured threshold, and measured token count/null. Route records may include validated route ID, exit/error code, and strict opaque `logRef`.

The Usage linkage is fixed as follows (not by `traceId` or parent traversal):

| History owner | Usage row | Link rule |
| --- | --- | --- |
| Standalone Worker execution | `kind:"run"` when the current recorder would emit one | Same `executionId`; pre-usage failures have no Usage row |
| Workflow and each Job | No new standalone `kind:"run"` row; Workflow envelopes remain excluded | Worker identity/metrics belong to its Job record |
| Route run (including route-owned exec/postprocess usage) | `kind:"route"` | Same Route execution ID; `runId`/`logRef` stays separate |
| Route log refetch | `kind:"route-refetch"` | Own refetch ID; existing `logRef` names the source Route |
| Hook/Route decision | None | No usage row |
| `count-tokens` | Existing `kind:"count"` unchanged and outside History | No ID addition |

#184 continues to aggregate each usage row independently; IDs do not filter, deduplicate, backfill, or join refetch rows. History write failure and Usage append failure are independent best-effort failures.

### Query and retention budgets

* `history list` defaults to 50 rows and accepts at most 500; `show <execution-id>` reads only that ID's directory; `trace <trace-id>` scans bounded retained records. Text/JSON filters may include since, kind, worker, and status.
* List/trace order by `recordedAt` descending, then `executionId` descending. The keyset cursor contains that pair. Each execution uses its started-event timestamp; a decision-only record uses its decision timestamp. A page is a live view: records created after a page may be absent from later pages; restart at the first page to include new records. Existing immutable IDs do not move between pages. Return `nextCursor` only after a complete scan; if a scan limit is reached, return rows already found with `scanComplete:false`, no `nextCursor`, and text output must say the result is incomplete. Pagination is guaranteed only when the retained dataset can be fully scanned within the stated budget.
* List/trace scan at most 20,000 execution directories, 128 MiB of event data, or 500 ms, whichever is reached first. When a limit is reached, return rows already found with `scanComplete:false`; text output must say the result is incomplete. Memory is bounded to page results plus at most 20,000 directory names and one 8 KiB event object.
* Writer-side best-effort retention targets 30 days, 10,000 execution directories, and 128 MiB of logical event-file bytes. A sweep is triggered before a new started/decision write, deletes expired/oldest records, and scans at most 20,000 entries (including pending directories), 128 MiB, or 500 ms. It removes pending directories older than 24 hours and final directories older than 24 hours only when both `started.json` and `decision.json` are missing or invalid; a valid `started.json` or `decision.json` is not an orphan even if another event file is invalid. Query never sweeps. If a sweep exhausts its budget before it can establish capacity, skip this History write.
* Retention targets are not a hard cross-process quota: simultaneous writers may temporarily exceed them. Each event remains size bounded; later writer sweeps remove excess. Retention may delete an execution older than policy even if its owner has not finished. A late terminal write never recreates the deleted directory, and the original execution result is unaffected.

### Confirmation

Approval record (2026-10-26): fresh A/B reviewers, separate Validator, and separate Judge completed the bounded review protocol; confirmed blockers were corrected and the final targeted Judge reported no remaining confirmed P0/P1 blocker. This review-protocol verdict is the user's explicitly authorized approval substitute. The ADR is accepted for planning. After implementation, contract tests must prove concurrent creates do not overwrite, a valid started file survives a truncated terminal, duplicate terminals are first-writer-wins, a retention deletion is not undone by a late terminal writer, query is read-only, and scan/page/retention budgets return explicit incomplete results when exhausted.

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/history/` (identity/context, typed schema, per-execution store, query/retention), `args.rs`, `cli.rs`, Worker/Workflow/Route/Hook lifecycle owners, `usage_log.rs` (additive ID projection only), `text_format.rs`, focused contract tests, user guide, and doc indexes.
* Patterns to follow: existing `<userConfigDir>` resolution, per-event `create_new` files, workspace `uuid` v4, best-effort recorders, typed `RunAbortReason`/Route Viability, route log TTL/count/byte retention conventions.
* Patterns to avoid: modifying `usage.jsonl` aggregation or putting History rows in it; reusing route `runId` as execution ID; Worker run rows for Workflow Job-owned Worker work; treating Hook Deny as Worker start; inferring missing parent/trace; query-time directory creation/cleanup/refetch writes/execution; raw payload persistence; mutable shared append or a lock on Hook hot path.
* Dependencies or configuration: no new dependency. Generate `executionId` and root `traceId` with workspace `uuid` v4. Optional explicit `CHEAPER_TRACE_ID` and `CHEAPER_PARENT_EXECUTION_ID` inputs are validated correlation hints only; managed children receive explicit propagation. Default store is `<userConfigDir>/history/`; `CHEAPER_LOG` does not control History.

## Verification

* [ ] Multi-process event file create, partial terminal, duplicate terminal, retention race, and crash tests pass on Windows and POSIX; absent terminal status remains unknown.
* [ ] History list/show/trace use read-only bounded scans, default page 50/max 500, keyset cursor `(recordedAt, executionId)`, and report `scanComplete=false` without a continuation cursor on 20,000-entry, 128 MiB, or 500 ms scan budget; missing store stays missing. Decision-only records participate in stable ordering. Pagination is guaranteed for datasets fully scanned within budget.
* [ ] Writer-side retention targets 30 days, 10,000 execution directories, and 128 MiB logical event bytes; each event is <=8 KiB. Sweep scans at most 20,000 directories/128 MiB/500 ms. These are best-effort targets, not a cross-process hard cap; concurrent creators may exceed them temporarily, and new history writes stop when a bounded sweep cannot establish capacity.
* [ ] Worker/job/route/decision ID ownership and usage mappings match the spec; additive usage IDs leave #184 aggregation/refetch semantics unchanged.
* [ ] Actual parent edges are distinct from inherited-unverified hints in text and JSON, including trace output. Provider/model values are projected only from resolved catalog identity, with endpoint/raw-input/display-label sources excluded and official colon-bearing IDs preserved. Metric values enforce nonnegative field caps; `exitCode` follows its separate integer range/omit contract. Windows/custom config dir, orphan cleanup, retention/query budgets, and Hook hot-path overhead meet design limits.
* [ ] `mise run check` passes; independent review finds no unresolved confirmed P0/P1.

## Pros and Cons of the Options

### Option 1: Shared locked JSONL

* Good, because one file has low directory-entry overhead and supports a simple ordered stream.
* Bad, because every event requires cross-process synchronization and stale-lock recovery; a lock can block the high-frequency Hook path.

### Option 2: Per-process JSONL segments

* Good, because each segment has one writer and readers can ignore a malformed tail.
* Bad, because a long-lived process holds an active file through execution; retention cannot safely remove it, while crash residue needs liveness/age recovery and file count grows per process.

### Option 3: Per-execution event files (chosen)

* Good, because each event has its own path; readers can preserve a valid started record when a terminal event is partial, and retention has no long-lived writer handle to unlink.
* Bad, because it creates more directories/files; bounded enumerate/scan/page and best-effort retention are required.

### Option 4: Embedded database

* Good, because indexes and transactional paging simplify large histories.
* Bad, because it adds database lifecycle/migration/locking work and likely a dependency; no current measurement demonstrates bounded segmented JSONL is insufficient.

## More Information

- [Issue #192](https://github.com/scarecrow173/cheaper/issues/192)
- `../specs/execution-history-issue-192.md`
- `../specs/usage-analysis-scope-issue-184.md` — usage ID compatibility and aggregate ownership.
- `../designs/usage-grouped-route-error-analysis-issue-184.md` — report behavior and route/refetch join constraints.
- `../../crates/cheaper-cli/src/usage_log.rs` — existing best-effort `usage.jsonl` behavior.
- `../../crates/cheaper-cli/src/route/log_store.rs` — exclusive create and retention precedent.

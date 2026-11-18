---
name: verify-workflow-scheduler
description: Exhaustive scheduling-safety verification for the cheaper-workflow DAG engine — every 4-node acyclic needs-graph crossed with when-skip variants and max_jobs caps is run against the real engine and compiler. Run this before merging any change to crates/cheaper-workflow/src/engine.rs scheduler logic (readiness, transitions, when evaluation, concurrency) or crates/cheaper-workflow/src/compiler.rs dependency-graph construction. A pass covers the bounded sweep only, not the whole system.
---

# Verify workflow scheduler invariants

**Property (plain words):** the engine never starts a job before every
job it depends on — via `needs` or `$from` inside `when` — has reached
a terminal state; a job whose `when` is false never runs and its
dependents still run; no more than `max_jobs` jobs run at once; every
job reaches exactly one terminal state. The compiler accepts a
definition iff its dependency graph is acyclic.

**The harm if it breaks:** a dependent reads a producer's output that
does not exist yet (silent wrong results or a late INVALID_OUTPUT), a
skipped job's dependent never runs (missing work), or the engine
exceeds the caller's concurrency budget (resource thrash).

## Run

```bash
mise exec -- cargo test -p cheaper-workflow --test exhaustive
```

Two tests, ~10 s total, deterministic, no network/model/grants:

- `every_acyclic_dag_respects_needs_skip_and_concurrency` — all 64
  `needs`-subset DAGs over 4 jobs (edges only point backwards, so every
  shape is acyclic by construction) × `when` on j2 binding `$from` j0
  evaluated true/false × `max_jobs` 2 and 4 = **256 real engine
  executions** through `run_workflow_with_observer` with mock tool
  executors.
- `every_cyclic_variant_is_rejected_at_compile` — each shape mutated
  with a mutual `needs` pair must fail `compile()` = 128 rejections.

## Coverage record

- **Verified code:** `crates/cheaper-workflow/src/engine.rs` — the
  admission loop (`remaining == 0` readiness gate, `when` evaluation,
  `finish_job` terminal bookkeeping), the `Ready → execute` dispatch
  with the `max_jobs` cap; `crates/cheaper-workflow/src/compiler.rs` —
  dep-edge construction (`needs`, `$from` in `when`/`input`) and the
  `workflow dependency cycle` check.
- **Check:** `crates/cheaper-workflow/tests/exhaustive.rs` — the only
  file the next agent edits when the invariants themselves change.
- **Sync commit:** `8784a0f` — the last commit at which this check was
  confirmed to describe the code above. If `git diff 8784a0f --
  crates/cheaper-workflow/src/engine.rs crates/cheaper-workflow/src/compiler.rs`
  is non-empty, re-read the diff, confirm the check still covers the
  changed behavior, and only then move the sync commit forward.

## Interpreting a failure

Failures print the counterexample in the code's own terms:
`edges=[j0->j2,...] when_j2=eq(j0.text,"MISMATCH") max_jobs=2` plus the
violated invariant — e.g. `j2 started at 5 before j0 terminal at 7` or
`overlap 3 exceeded max_jobs`.

- **Real bug** — the transition log violates an invariant on a DAG the
  compiler accepted. The bug is in `engine.rs` (or `compiler.rs` if a
  cyclic shape was accepted). Reproduce on the printed case, fix at the
  scheduler/compiler boundary, rerun.
- **Modeling error** — the check's assumptions drifted from the API:
  transition enum variants renamed, `WorkflowRunOptions` fields
  changed, executor contract moved, `when` semantics intentionally
  changed. Fix `tests/exhaustive.rs` to match the new contract, rerun,
  then update the sync commit. If the intended semantics genuinely
  changed (e.g. skipped jobs now cascade), update the invariant in the
  doc comment at the top of that file too.

## Known failure modes (proven, not hypothetical)

- Removing the `remaining == 0` readiness gate: jobs start before
  producers → their `$from` bindings fail closed at admission, and the
  run fails before the ordering assertion — still caught (the engine
  refuses to read unproduced data rather than corrupting it). A subtler
  ordering bug that dodges binding failure hits the explicit
  started-before-terminal assertion instead.
- Raising the concurrency cap (`running >= max_jobs` → `>= 8.0`):
  `overlap 3 exceeded max_jobs` — caught directly.

## Scope

Covers: acyclic DAG scheduling order, `needs` and `$from`-in-`when`
edges, condition-skip propagation, `max_jobs` concurrency, exactly-once
terminal coverage, cycle rejection at compile.

Does NOT cover: timeout/abort paths, resource-lease contention beyond
admission, worker-kind jobs (mock covers `tool` kind; the scheduler
treats kinds uniformly), DAGs larger than 4 nodes (ordering induction
over shapes larger than the bound is argued, not enumerated — the bound
already exercises every distinct predecessor/skip/overlap pattern), or
the `outputs` materialization layer beyond what `when`-skip exercises.

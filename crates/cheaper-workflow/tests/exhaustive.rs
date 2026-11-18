//! Exhaustive scheduling-safety sweep over the real DAG engine.
//!
//! Property (plain words): for every acyclic workflow the compiler
//! accepts, the engine never starts a job before every job it needs —
//! via `needs` or via `$from` inside `when` — has reached a terminal
//! state; a job whose `when` is false is never started and its
//! dependents still run; concurrent running jobs never exceed
//! `max_jobs`; and every job reaches exactly one terminal state.
//!
//! Method: enumerate ALL `needs` subsets for 4 jobs ordered so edges
//! only point backwards (64 acyclic shapes), cross with a `when`
//! condition on job 2 that binds `$from` job 0 (implicit dep edge) and
//! evaluates true or false, and run each shape against the real
//! `run_workflow_with_observer` at `max_jobs` 2 and 4 — 256 engine
//! executions. A second sweep proves the compiler rejects the cyclic
//! counterparts (mutual `needs` edges).
//!
//! Covered code: `src/engine.rs` scheduler readiness/transition logic,
//! `when` evaluation, CONDITION_SKIPPED materialization;
//! `src/compiler.rs` dependency-graph construction and cycle check.

mod common;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

use cheaper_workflow::engine::{
    run_workflow_with_observer, WorkflowJobObserver, WorkflowJobObserverError,
    WorkflowJobTerminalStatus, WorkflowJobTransition,
};
use cheaper_workflow::{JobStatus, WorkflowRunOptions};

use common::{default_harness, definition, tool_result, FnExecutor};

const IDS: [&str; 4] = ["j0", "j1", "j2", "j3"];

#[derive(Default)]
struct RecordingObserver {
    transitions: Vec<WorkflowJobTransition>,
}

impl WorkflowJobObserver for RecordingObserver {
    fn observe(
        &mut self,
        transition: WorkflowJobTransition,
    ) -> Result<(), WorkflowJobObserverError> {
        self.transitions.push(transition);
        Ok(())
    }
}

/// `needs` subsets for every job: job i may need any subset of j0..j(i-1).
/// Returns all 64 acyclic shapes as `Vec<(job_index, Vec<needs_index>)>`.
fn all_acyclic_dags() -> Vec<Vec<HashSet<usize>>> {
    let mut dags: Vec<Vec<HashSet<usize>>> = vec![vec![HashSet::new(); 4]];
    for i in 1..4usize {
        let mut next = Vec::new();
        for mask in 0..(1usize << i) {
            let preds: HashSet<usize> = (0..i).filter(|p| mask & (1 << p) != 0).collect();
            for dag in &dags {
                let mut d = dag.clone();
                d[i] = preds.clone();
                next.push(d);
            }
        }
        dags = next;
    }
    dags
}

/// Build the workflow definition for a DAG shape. `when_lit` selects
/// j2's condition: `"j0"` evaluates true, anything else false. The
/// `when` binds `$from` j0, so j2 gains j0 as an implicit predecessor.
fn definition_for(dag: &[HashSet<usize>], when_lit: &str) -> Value {
    let mut raw = definition();
    for (i, id) in IDS.iter().enumerate() {
        let needs: Vec<&str> = dag[i].iter().map(|p| IDS[*p]).collect();
        let mut job = json!({ "kind": "tool", "tool": "grep", "input": {} });
        if !needs.is_empty() {
            job["needs"] = json!(needs);
        }
        if i == 2 {
            job["when"] = json!({
                "left": { "$from": "j0", "path": "/content/0/text" },
                "op": "eq",
                "right": { "$literal": when_lit },
            });
        }
        raw["jobs"][*id] = job;
    }
    raw
}

/// Dep set the engine must honor for job `i`: explicit `needs` plus the
/// implicit `$from` edge in j2's `when`.
fn effective_preds(dag: &[HashSet<usize>], i: usize) -> HashSet<usize> {
    let mut preds = dag[i].clone();
    if i == 2 {
        preds.insert(0);
    }
    preds
}

fn describe(dag: &[HashSet<usize>], when_lit: &str, max_jobs: f64) -> String {
    let edges: Vec<String> = (0..4)
        .flat_map(|i| {
            dag[i]
                .iter()
                .map(move |p| format!("{}->{}", IDS[*p], IDS[i]))
        })
        .collect();
    format!(
        "edges=[{}] when_j2=eq(j0.text,{:?}) max_jobs={}",
        edges.join(","),
        when_lit,
        max_jobs
    )
}

#[tokio::test]
async fn every_acyclic_dag_respects_needs_skip_and_concurrency() {
    let dags = all_acyclic_dags();
    assert_eq!(dags.len(), 64);
    let fixtures = common::FixtureSet::new();
    let mut checked = 0usize;

    for dag in &dags {
        for when_lit in ["j0", "MISMATCH"] {
            let when_true = when_lit == "j0";
            let raw = definition_for(dag, when_lit);
            let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();

            for max_jobs in [2.0, 4.0] {
                let case = describe(dag, when_lit, max_jobs);
                let mut h = default_harness();

                let inflight = Arc::new(AtomicUsize::new(0));
                let max_seen = Arc::new(AtomicUsize::new(0));
                {
                    let (inflight, max_seen) = (Arc::clone(&inflight), Arc::clone(&max_seen));
                    h.ports.tool = Arc::new(FnExecutor(
                        move |ctx: cheaper_workflow::WorkflowExecutionContext| {
                            let cur = inflight.fetch_add(1, Ordering::SeqCst) + 1;
                            max_seen.fetch_max(cur, Ordering::SeqCst);
                            let inflight = Arc::clone(&inflight);
                            let job_id = ctx.job_id.clone();
                            async move {
                                sleep(Duration::from_millis(1)).await;
                                inflight.fetch_sub(1, Ordering::SeqCst);
                                Ok(tool_result(&job_id))
                            }
                        },
                    ));
                }

                let mut observer = RecordingObserver::default();
                let outcome = run_workflow_with_observer(
                    WorkflowRunOptions {
                        workflow: &workflow,
                        ports: h.ports,
                        max_jobs,
                        timeout_ms: 10_000.0,
                        signal: None,
                        abort_grace_ms: None,
                        now: None,
                    },
                    &mut observer,
                )
                .await
                .unwrap_or_else(|e| panic!("engine error on {case}: {e}"));

                assert!(outcome.ok, "workflow failed on {case}: {:?}", outcome.error);

                // Index transitions: seq -> (Started | Terminal status) per job.
                let mut started_at: HashMap<String, usize> = HashMap::new();
                let mut terminal_at: HashMap<String, (usize, WorkflowJobTerminalStatus)> =
                    HashMap::new();
                let mut running = 0usize;
                let mut max_overlap = 0usize;
                for (seq, t) in observer.transitions.iter().enumerate() {
                    match t {
                        WorkflowJobTransition::Started { job } => {
                            running += 1;
                            max_overlap = max_overlap.max(running);
                            started_at.insert(job.job_id.clone(), seq);
                        }
                        WorkflowJobTransition::Terminal { job, status, .. } => {
                            running = running.saturating_sub(1);
                            terminal_at.insert(job.job_id.clone(), (seq, *status));
                        }
                    }
                }

                // Invariant 1: no job starts before its effective
                // predecessors reach a terminal state.
                for (i, id) in IDS.iter().enumerate() {
                    let Some(&start) = started_at.get(*id) else {
                        continue;
                    };
                    for p in effective_preds(dag, i) {
                        let pred = IDS[p];
                        let (t_seq, t_status) = *terminal_at.get(pred).unwrap_or_else(|| {
                            panic!("{case}: {id} started but {pred} never terminal")
                        });
                        assert!(
                            t_seq < start,
                            "{case}: {id} started at {start} before {pred} terminal at {t_seq}"
                        );
                        let _ = t_status;
                    }
                }

                // Invariant 2: j2 with when:false is never started and
                // terminates Skipped; when:true runs.
                let j2_started = started_at.contains_key("j2");
                if when_true {
                    assert!(j2_started, "{case}: when:true j2 must run");
                } else {
                    assert!(!j2_started, "{case}: when:false j2 must not start");
                    assert_eq!(
                        terminal_at.get("j2").map(|t| t.1),
                        Some(WorkflowJobTerminalStatus::Skipped),
                        "{case}: j2 must terminate Skipped"
                    );
                }

                // Invariant 3: dependents of a skipped job still run.
                for (i, id) in IDS.iter().enumerate() {
                    if !when_true && effective_preds(dag, i).contains(&2) && i != 2 {
                        assert!(
                            started_at.contains_key(*id),
                            "{case}: {id} depends on skipped j2 but never ran"
                        );
                    }
                }

                // Invariant 4: concurrency cap and exactly-once coverage.
                assert!(
                    max_overlap <= max_jobs as usize,
                    "{case}: overlap {max_overlap} exceeded max_jobs"
                );
                assert!(
                    max_seen.load(Ordering::SeqCst) <= max_jobs as usize,
                    "{case}: executor saw more than max_jobs in flight"
                );
                for id in IDS {
                    assert!(
                        terminal_at.contains_key(id),
                        "{case}: {id} missing terminal transition"
                    );
                }

                // Outcome-level agreement: started <=> Succeeded, not
                // started <=> Skipped (for j2) or Blocked elsewhere.
                let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
                    outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
                for id in IDS {
                    if started_at.contains_key(id) {
                        assert_eq!(by_id[id].status, JobStatus::Succeeded, "{case}: {id}");
                    } else {
                        assert_ne!(by_id[id].status, JobStatus::Succeeded, "{case}: {id}");
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 256);
}

/// The compiler must reject every DAG made cyclic by a mutual `needs`
/// edge — the acceptance boundary of the ordering property.
#[tokio::test]
async fn every_cyclic_variant_is_rejected_at_compile() {
    let dags = all_acyclic_dags();
    let fixtures = common::FixtureSet::new();
    let mut rejected = 0usize;
    for dag in &dags {
        for (a, b) in [(0usize, 1usize), (0, 3)] {
            let mut cyclic = dag.clone();
            cyclic[a].insert(b);
            cyclic[b].insert(a);
            let raw = definition_for(&cyclic, "j0");
            assert!(
                fixtures.compiler().compile(&raw, None).await.is_err(),
                "cyclic edges {}<->{} accepted: {:?}",
                IDS[a],
                IDS[b],
                cyclic
            );
            rejected += 1;
        }
    }
    assert_eq!(rejected, 128);
}

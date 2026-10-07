//! Port of `packages/tool-runtime/src/execution-metrics.ts` (T2-2).
//!
//! `ExecutionMetrics` は invocation 全体または owner 単位の集計 snapshot。
//! 時刻は monotonic ms(`f64`)、`overlap_ms` は speedup / token savings を
//! 意味しない(共有 ADR D3)。`max_concurrency` は履歴の再生ではなく
//! lifecycle が観測した high-water 値を呼び出し側が渡す。

use std::collections::BTreeMap;

use serde::Serialize;

use crate::operation::{OperationRecord, OperationState};

/// `Record<OperationState, number>` の struct 表現。serde の key は
/// TS の state 文字列(`queued`/`running`/.../`timed_out`)と一致する。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct OperationStateCounts {
    pub queued: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
}

impl OperationStateCounts {
    fn increment(&mut self, state: OperationState) {
        match state {
            OperationState::Queued => self.queued += 1,
            OperationState::Running => self.running += 1,
            OperationState::Completed => self.completed += 1,
            OperationState::Failed => self.failed += 1,
            OperationState::Cancelled => self.cancelled += 1,
            OperationState::TimedOut => self.timed_out += 1,
        }
    }
}

/// `ExecutionMetrics`。field 名・意味は TS 版と同一。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ExecutionMetrics {
    pub operations: u64,
    #[serde(rename = "byState")]
    pub by_state: OperationStateCounts,
    /// `toolName` が無い宣言操作は `kind` に集計する。
    #[serde(rename = "byTool")]
    pub by_tool: BTreeMap<String, u64>,
    #[serde(rename = "maxConcurrency")]
    pub max_concurrency: u64,
    #[serde(rename = "operationMs")]
    pub operation_ms: f64,
    #[serde(rename = "activeMs")]
    pub active_ms: f64,
    #[serde(rename = "overlapMs")]
    pub overlap_ms: f64,
    #[serde(rename = "queueMs")]
    pub queue_ms: f64,
}

/// `collectExecutionMetrics`。`snapshot_at` は呼び出し側の clock で
/// 取得した monotonic ms、`observed_max_concurrency` は観測済みの
/// high-water(0 以上の実測値。ここで再計算しない)。
pub fn collect_execution_metrics(
    records: &[OperationRecord],
    snapshot_at: f64,
    observed_max_concurrency: u64,
) -> ExecutionMetrics {
    let mut by_state = OperationStateCounts::default();
    let mut by_tool = BTreeMap::<String, u64>::new();
    let mut intervals: Vec<(f64, f64)> = Vec::new();
    let mut operation_ms = 0.0f64;
    let mut queue_ms = 0.0f64;
    for record in records {
        by_state.increment(record.state);
        let tool = record
            .tool_name
            .clone()
            .unwrap_or_else(|| record.kind.clone());
        *by_tool.entry(tool).or_insert(0) += 1;
        let queue_end = (record
            .started_at
            .or(record.finished_at)
            .unwrap_or(snapshot_at))
        .min(snapshot_at);
        queue_ms += (queue_end - record.created_at).max(0.0);
        let Some(started_at) = record.started_at else {
            continue;
        };
        let end = record.finished_at.unwrap_or(snapshot_at).min(snapshot_at);
        if end <= started_at {
            continue;
        }
        operation_ms += end - started_at;
        intervals.push((started_at, end));
    }

    intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut active_ms = 0.0f64;
    let mut covered_until = f64::NEG_INFINITY;
    for (start, end) in intervals {
        active_ms += (end - start.max(covered_until)).max(0.0);
        covered_until = covered_until.max(end);
    }
    ExecutionMetrics {
        operations: records.len() as u64,
        by_state,
        by_tool,
        max_concurrency: observed_max_concurrency,
        operation_ms,
        active_ms,
        overlap_ms: operation_ms - active_ms,
        queue_ms,
    }
}

#[cfg(test)]
mod tests {
    //! `execution-metrics.test.ts` 対応。OperationManager 経由で record を
    //! 作り、fake clock で deterministically に集計する。

    use std::sync::{Arc, Mutex};

    use cheaper_core::errors::{CheaperError, ErrorCode};
    use tokio::sync::Notify;

    use super::*;
    use crate::operation::{AbortReason, OperationManager, OperationManagerOptions};

    #[derive(Clone)]
    struct ManualClock(Arc<Mutex<f64>>);

    impl ManualClock {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(0.0)))
        }
        fn set(&self, v: f64) {
            *self.0.lock().unwrap() = v;
        }
        fn clock(&self) -> crate::operation::Clock {
            let inner = Arc::clone(&self.0);
            Arc::new(move || *inner.lock().unwrap())
        }
    }

    fn manager_with(clock: ManualClock) -> OperationManager {
        OperationManager::new(OperationManagerOptions {
            clock: Some(clock.clock()),
            ..Default::default()
        })
        .unwrap()
    }

    fn operation() -> crate::operation::CreateOperationOptions {
        crate::operation::CreateOperationOptions {
            owner_id: "a".to_string(),
            kind: "tool".to_string(),
            tool_name: Some("read".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn zero_snapshot_and_all_six_states() {
        let clock = ManualClock::new();
        let m = manager_with(clock);
        assert_eq!(
            m.execution_metrics(None),
            ExecutionMetrics {
                operations: 0,
                by_state: OperationStateCounts::default(),
                by_tool: BTreeMap::new(),
                max_concurrency: 0,
                operation_ms: 0.0,
                active_ms: 0.0,
                overlap_ms: 0.0,
                queue_ms: 0.0,
            }
        );
    }

    #[test]
    fn measures_serial_versus_overlapping_execution() {
        for overlap in [false, true] {
            let clock = ManualClock::new();
            let m = manager_with(clock.clone());
            let first = m.create(operation()).unwrap();
            clock.set(10.0);
            m.start(&first.id).unwrap();
            if !overlap {
                clock.set(30.0);
                m.complete(&first.id, serde_json::Value::Null).unwrap();
            }
            clock.set(if overlap { 10.0 } else { 35.0 });
            let second = m.create(operation()).unwrap();
            clock.set(if overlap { 20.0 } else { 45.0 });
            m.start(&second.id).unwrap();
            if overlap {
                clock.set(40.0);
                m.complete(&first.id, serde_json::Value::Null).unwrap();
            }
            clock.set(if overlap { 50.0 } else { 65.0 });
            m.complete(&second.id, serde_json::Value::Null).unwrap();
            clock.set(100.0);
            let metrics = m.execution_metrics(None);
            let by_state = OperationStateCounts {
                completed: 2,
                ..Default::default()
            };
            assert_eq!(
                metrics,
                ExecutionMetrics {
                    operations: 2,
                    by_state,
                    by_tool: BTreeMap::from([("read".to_string(), 2)]),
                    max_concurrency: if overlap { 2 } else { 1 },
                    operation_ms: if overlap { 60.0 } else { 40.0 },
                    active_ms: 40.0,
                    overlap_ms: if overlap { 20.0 } else { 0.0 },
                    queue_ms: 20.0,
                }
            );
        }
    }

    #[test]
    fn keeps_owner_local_union_and_maximum_separate_from_invocation_totals() {
        let clock = ManualClock::new();
        let m = manager_with(clock.clone());
        let a = m.create(operation()).unwrap();
        m.start(&a.id).unwrap();
        clock.set(10.0);
        let b = m
            .create(crate::operation::CreateOperationOptions {
                tool_name: Some("grep".to_string()),
                owner_id: "b".to_string(),
                ..operation()
            })
            .unwrap();
        m.start(&b.id).unwrap();
        clock.set(20.0);
        m.complete(&a.id, serde_json::Value::Null).unwrap();
        clock.set(30.0);
        m.complete(&b.id, serde_json::Value::Null).unwrap();
        let a_metrics = m.execution_metrics(Some("a"));
        assert_eq!(a_metrics.operations, 1);
        assert_eq!(a_metrics.max_concurrency, 1);
        assert_eq!(a_metrics.active_ms, 20.0);
        let b_metrics = m.execution_metrics(Some("b"));
        assert_eq!(b_metrics.operations, 1);
        assert_eq!(b_metrics.max_concurrency, 1);
        assert_eq!(b_metrics.active_ms, 20.0);
        let all = m.execution_metrics(None);
        assert_eq!(all.operations, 2);
        assert_eq!(all.max_concurrency, 2);
        assert_eq!(all.operation_ms, 40.0);
        assert_eq!(all.active_ms, 30.0);
        assert_eq!(all.overlap_ms, 10.0);
    }

    #[test]
    fn observes_zero_duration_concurrency() {
        for overlap in [false, true] {
            let clock = ManualClock::new();
            let m = manager_with(clock);
            let a = m.create(operation()).unwrap();
            m.start(&a.id).unwrap();
            if !overlap {
                m.complete(&a.id, serde_json::Value::Null).unwrap();
            }
            let b = m.create(operation()).unwrap();
            m.start(&b.id).unwrap();
            if overlap {
                m.complete(&a.id, serde_json::Value::Null).unwrap();
            }
            m.complete(&b.id, serde_json::Value::Null).unwrap();
            let metrics = m.execution_metrics(None);
            assert_eq!(metrics.max_concurrency, if overlap { 2 } else { 1 });
            assert_eq!(metrics.operation_ms, 0.0);
            assert_eq!(metrics.active_ms, 0.0);
            assert_eq!(metrics.overlap_ms, 0.0);
        }
    }

    #[test]
    fn counts_failed_queued_cancelled_timed_out_and_pending_wait() {
        let clock = ManualClock::new();
        let m = manager_with(clock.clone());
        let queued = m.create(operation()).unwrap();
        let timeout = m
            .create(crate::operation::CreateOperationOptions {
                tool_name: Some("grep".to_string()),
                ..operation()
            })
            .unwrap();
        let failed = m
            .create(crate::operation::CreateOperationOptions {
                tool_name: Some("write".to_string()),
                ..operation()
            })
            .unwrap();
        m.create(crate::operation::CreateOperationOptions {
            kind: "process".to_string(),
            tool_name: None,
            ..operation()
        })
        .unwrap();
        clock.set(5.0);
        m.start(&failed.id).unwrap();
        clock.set(10.0);
        m.cancel(&queued.id, AbortReason::cancelled()).unwrap();
        m.cancel(
            &timeout.id,
            AbortReason::from(CheaperError::new(ErrorCode::Timeout, "timeout")),
        )
        .unwrap();
        clock.set(15.0);
        m.fail(&failed.id, AbortReason::msg("failed")).unwrap();
        clock.set(20.0);
        let metrics = m.execution_metrics(None);
        let by_state = OperationStateCounts {
            queued: 1,
            cancelled: 1,
            timed_out: 1,
            failed: 1,
            ..Default::default()
        };
        assert_eq!(
            metrics,
            ExecutionMetrics {
                operations: 4,
                by_state,
                by_tool: BTreeMap::from([
                    ("read".to_string(), 1),
                    ("grep".to_string(), 1),
                    ("write".to_string(), 1),
                    ("process".to_string(), 1),
                ]),
                max_concurrency: 1,
                operation_ms: 10.0,
                active_ms: 10.0,
                overlap_ms: 0.0,
                queue_ms: 45.0,
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn clamps_running_to_snapshot_and_tracks_detached_late_settle() {
        let clock = ManualClock::new();
        let m = OperationManager::new(OperationManagerOptions {
            clock: Some(clock.clock()),
            abort_grace_ms: Some(30.0),
            ..Default::default()
        })
        .unwrap();
        let a = m.create(operation()).unwrap();
        m.start(&a.id).unwrap();
        clock.set(20.0);
        m.cancel(
            &a.id,
            AbortReason::from(CheaperError::new(ErrorCode::Timeout, "timeout")),
        )
        .unwrap();
        let metrics = m.execution_metrics(None);
        assert_eq!(metrics.by_state.running, 1);
        assert_eq!(metrics.operation_ms, 20.0);

        clock.set(30.0);
        m.drain(None, None).await.unwrap();
        assert_eq!(
            m.get(&a.id).unwrap().settlement,
            crate::operation::Settlement::Unsettled
        );
        clock.set(50.0);
        let detached = m.execution_metrics(None);
        assert_eq!(detached.by_state.running, 1);
        assert_eq!(detached.operation_ms, 50.0);
        assert_eq!(detached.active_ms, 50.0);
        clock.set(80.0);
        m.complete(&a.id, serde_json::Value::Null).unwrap();
        clock.set(100.0);
        let metrics = m.execution_metrics(None);
        assert_eq!(metrics.by_state.timed_out, 1);
        assert_eq!(metrics.operation_ms, 80.0);
        assert_eq!(metrics.active_ms, 80.0);
        // detach 前の snapshot は変化しない(independent object)。
        assert_eq!(detached.operation_ms, 50.0);
        assert!(m.create(operation()).is_err());
        assert_eq!(m.execution_metrics(None).operations, 1);
    }

    #[test]
    fn preserves_prototype_like_tool_names_and_independent_snapshots() {
        let clock = ManualClock::new();
        let m = manager_with(clock);
        for tool_name in ["__proto__", "constructor", "read"] {
            let a = m
                .create(crate::operation::CreateOperationOptions {
                    tool_name: Some(tool_name.to_string()),
                    ..operation()
                })
                .unwrap();
            m.start(&a.id).unwrap();
            m.complete(&a.id, serde_json::Value::Null).unwrap();
        }
        let snap = m.execution_metrics(None);
        assert_eq!(snap.by_tool["__proto__"], 1);
        assert_eq!(snap.by_tool["constructor"], 1);
        assert_eq!(snap.by_tool["read"], 1);
        let again = m.execution_metrics(None);
        assert_eq!(again.operations, 3);
        assert_eq!(again.by_state.completed, 3);
        assert_eq!(again.by_tool["read"], 1);
    }

    /// `execution-metrics.test.ts` の invocation-clock ケース。invocation 層は
    /// T2-7 のため、ここでは同じ構成(共有 clock + maxOperations=1 の
    /// scheduler + 2 owner)を primitive から組み立てて queueMs を pin する。
    #[tokio::test(flavor = "current_thread")]
    async fn uses_shared_clock_and_records_real_scheduler_wait() {
        use crate::resources::ResourceManager;
        use crate::scheduler::{OperationDeclaration, ToolScheduler};
        use cheaper_core::resources::types::ClaimMode;
        use cheaper_core::workspace::paths::SymlinkPolicy;

        let clock = ManualClock::new();
        let now = clock.0.clone();
        let manager = manager_with(clock.clone());
        let scheduler = ToolScheduler::new(
            manager.clone(),
            Some(&serde_json::json!({ "maxOperations": 1 })),
            Some(ResourceManager::new("/workspace", SymlinkPolicy::Deny)),
        )
        .unwrap();

        let release = Arc::new(Notify::new());
        let first_started = Arc::new(Notify::new());

        let first = {
            let scheduler = scheduler.clone();
            let release = release.clone();
            let first_started = first_started.clone();
            tokio::spawn(async move {
                scheduler
                    .run(
                        crate::operation::CreateOperationOptions {
                            owner_id: "first".to_string(),
                            kind: "tool".to_string(),
                            tool_name: Some("ls".to_string()),
                            ..Default::default()
                        },
                        OperationDeclaration::new(ClaimMode::Shared),
                        move |_| {
                            let release = release.clone();
                            let first_started = first_started.clone();
                            async move {
                                first_started.notify_one();
                                release.notified().await;
                                Ok::<serde_json::Value, AbortReason>(serde_json::json!({
                                    "content": []
                                }))
                            }
                        },
                    )
                    .await
            })
        };
        // wait for first to be running
        first_started.notified().await;
        *now.lock().unwrap() = 5.0;
        let second = {
            let scheduler = scheduler.clone();
            let now = now.clone();
            tokio::spawn(async move {
                scheduler
                    .run(
                        crate::operation::CreateOperationOptions {
                            owner_id: "second".to_string(),
                            kind: "tool".to_string(),
                            tool_name: Some("ls".to_string()),
                            ..Default::default()
                        },
                        OperationDeclaration::new(ClaimMode::Shared),
                        move |_| {
                            let now = now.clone();
                            async move {
                                *now.lock().unwrap() = 30.0;
                                Ok::<serde_json::Value, AbortReason>(serde_json::json!({
                                    "content": []
                                }))
                            }
                        },
                    )
                    .await
            })
        };
        // second は queued のまま待機させる。flush で prepare/admission を回す。
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        *now.lock().unwrap() = 10.0;
        let second_metrics = manager.execution_metrics(Some("second"));
        assert_eq!(second_metrics.by_state.queued, 1);
        assert_eq!(second_metrics.queue_ms, 5.0);
        *now.lock().unwrap() = 20.0;
        release.notify_waiters();
        let (a, b) = tokio::join!(first, second);
        a.unwrap().unwrap();
        b.unwrap().unwrap();
        let metrics = manager.execution_metrics(None);
        assert_eq!(metrics.operations, 2);
        assert_eq!(metrics.max_concurrency, 1);
        assert_eq!(metrics.operation_ms, 30.0);
        assert_eq!(metrics.active_ms, 30.0);
        assert_eq!(metrics.overlap_ms, 0.0);
        assert_eq!(metrics.queue_ms, 15.0);
    }
}

//! Port of `packages/workflow/src/dag.ts`.

use std::collections::{HashMap, HashSet};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use serde_json::{json, Map, Value};

use crate::types::DependencyEdge;

/// `compileDag` の戻り値。
#[derive(Debug, Clone, PartialEq)]
pub struct DagResult {
    /// 決定的 topological order(Kahn + code-unit sort の ready queue)。
    pub order: Vec<String>,
    /// dedupe・sort 済みの semantic edges。
    pub edges: Vec<DependencyEdge>,
}

/// `compileDag`。全 sort は JS `Array#sort` 規則(UTF-16 code unit)を
/// `code_unit_compare` で再現する。
pub fn compile_dag(
    job_ids: &[String],
    dependencies: &[DependencyEdge],
) -> Result<DagResult, CheaperError> {
    let mut ids: Vec<String> = job_ids.to_vec();
    ids.sort_by(|a, b| code_unit_compare(a, b));
    let mut outgoing: HashMap<&str, HashSet<&str>> =
        ids.iter().map(|id| (id.as_str(), HashSet::new())).collect();
    let mut indegree: HashMap<&str, usize> = ids.iter().map(|id| (id.as_str(), 0usize)).collect();
    for DependencyEdge { from, to } in dependencies {
        if !outgoing.contains_key(from.as_str()) || !outgoing.contains_key(to.as_str()) {
            return Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown dependency: {from} -> {to}"),
            ));
        }
        if from == to {
            return Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("self dependency: {from}"),
            ));
        }
        let children = outgoing.get_mut(from.as_str()).expect("checked above");
        if !children.contains(to.as_str()) {
            children.insert(to.as_str());
            *indegree.get_mut(to.as_str()).expect("checked above") += 1;
        }
    }
    let mut edges = Vec::new();
    for from in &ids {
        let mut targets: Vec<&str> = outgoing[from.as_str()].iter().copied().collect();
        targets.sort_by(|a, b| code_unit_compare(a, b));
        for to in targets {
            edges.push(DependencyEdge {
                from: from.clone(),
                to: to.to_string(),
            });
        }
    }
    let mut ready: Vec<&str> = ids
        .iter()
        .map(|s| s.as_str())
        .filter(|id| indegree[id] == 0)
        .collect();
    let mut order: Vec<String> = Vec::new();
    while let Some(id) = (!ready.is_empty()).then(|| ready.remove(0)) {
        order.push(id.to_string());
        // `outgoing` の Set 反復順は不定のため code-unit sort で固定する
        // (`ready.sort()` により ready 側は常に整列済みで、push 順が
        // 最終 order に残る差分は子側の走査順。TS は Set 挿入順=
        // 構築時の辺列挙順、こちらは code-unit 順に揃える — どちらでも
        // `ready.sort()` 後の順は同じだが、同時に ready 化する兄弟の
        // push 順を deterministic にするため)。
        let mut children: Vec<&str> = outgoing[id].iter().copied().collect();
        children.sort_by(|a, b| code_unit_compare(a, b));
        for child in children {
            let remaining = indegree[child] - 1;
            *indegree.get_mut(child).expect("present") = remaining;
            if remaining == 0 {
                ready.push(child);
            }
        }
        ready.sort_by(|a, b| code_unit_compare(a, b));
    }
    if order.len() != ids.len() {
        return Err(
            CheaperError::new(ErrorCode::InvalidRequest, "workflow dependency cycle").with_details(
                {
                    let mut details = Map::new();
                    details.insert(
                        "jobs".to_string(),
                        Value::Array(
                            ids.iter()
                                .filter(|id| indegree[id.as_str()] > 0)
                                .map(|id| json!(id))
                                .collect(),
                        ),
                    );
                    details
                },
            ),
        );
    }
    Ok(DagResult { order, edges })
}

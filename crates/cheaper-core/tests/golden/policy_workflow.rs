use cheaper_core::policy::workflow::resolve_workflow_policy;

use crate::common::*;

#[test]
fn workflow_cases() {
    let f = fixture("policy/workflow.json");
    for case in cases(&f) {
        let name = case_name(case);
        let produced = match resolve_workflow_policy(opt_input(case, "input")) {
            Ok(p) => Ok(serde_json::to_value(&p).unwrap()),
            Err(e) => Err(cheaper_err(&e, &[])),
        };
        let mut expected = case["expected"].clone();
        if case["rustDropUnknownArtifactKeys"].as_bool().unwrap_or(false) {
            // Documented divergence: TS `resolveWorkflowPolicy` spreads
            // unknown `artifacts` keys into the output; the Rust typed
            // `WorkflowArtifactPolicy` drops them (unreachable through
            // the schema-validated pipeline). Strip them here.
            const KNOWN: [&str; 4] = [
                "maxArtifactBytes",
                "maxTotalBytes",
                "maxPreviewBytes",
                "maxReadBytes",
            ];
            if let Some(a) = expected
                .get_mut("ok")
                .and_then(|ok| ok.get_mut("artifacts"))
                .and_then(|a| a.as_object_mut())
            {
                a.retain(|k, _| KNOWN.contains(&k.as_str()));
            }
        }
        check(&name, &expected, produced);
    }
}

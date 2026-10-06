use serde_json::{json, Value};
use std::path::Path;

use cheaper_core::resources::normalization::{normalize_resource_manifest, ResourceError};
use cheaper_core::resources::predicates::{
    claim_contains, claims_conflict, manifests_conflict, resource_contains, resource_key,
    resources_overlap,
};
use cheaper_core::resources::types::{
    ClaimMode, Resource, ResourceClaim, ResourceManifest, ResourceScope,
};

use crate::common::*;

/// Predicate cases bind `<ABS>`/`_<FSROOT>` to a synthetic absolute base
/// (`golden-pred-base` under cwd, matching the dump). `resourceKey`
/// lowercases its identity on win32 → a lowercased scrub needle too.
struct Pred {
    needles: Vec<(String, String)>,
}

fn pred() -> Pred {
    let abs = abs_pred_base();
    let fsroot = Path::new(&abs)
        .ancestors()
        .last()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Pred {
        needles: vec![
            ("<ABS>".to_string(), abs.clone()),
            ("<FSROOT>".to_string(), fsroot),
            // `identity_key` lowercases the whole path on win32.
            ("<ABS>".to_string(), abs.to_lowercase()),
        ],
    }
}

fn scope(v: &Value) -> ResourceScope {
    match v.as_str() {
        Some("subtree") => ResourceScope::Subtree,
        _ => ResourceScope::Exact,
    }
}

fn mode(v: &Value) -> ClaimMode {
    match v.as_str() {
        Some("exclusive") => ClaimMode::Exclusive,
        _ => ClaimMode::Shared,
    }
}

fn resource(p: &Pred, v: &Value) -> Resource {
    let v = unscrub(v, &p.needles[..2]);
    Resource::workspace(native_path(v["path"].as_str().unwrap()), scope(&v["scope"]))
}

fn claim(p: &Pred, v: &Value) -> ResourceClaim {
    let v = unscrub(v, &p.needles[..2]);
    ResourceClaim {
        resource: resource(p, &v["resource"]),
        mode: mode(&v["mode"]),
    }
}

fn manifest(p: &Pred, v: &Value) -> ResourceManifest {
    let claims = |key: &str| -> Vec<ResourceClaim> {
        v.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|c| claim(p, c)).collect())
            .unwrap_or_default()
    };
    ResourceManifest {
        reads: claims("reads"),
        writes: claims("writes"),
        produces: claims("produces"),
    }
}

#[test]
fn resource_cases() {
    let f = fixture("resources.json");
    for case in cases(&f) {
        if !axis_ok(case) {
            continue;
        }
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        let produced: Result<Value, Value> = match op {
            "normalize" => {
                let Some(c) = case_dir(case) else {
                    // Symlink creation unavailable on this host — mirrors
                    // the dump skipping the case.
                    continue;
                };
                let input = unscrub(&case["input"], &c.needles);
                let input_ref = if case["omitted"].as_bool().unwrap_or(false) {
                    None
                } else {
                    Some(&input)
                };
                match normalize_resource_manifest(
                    &c.root,
                    input_ref,
                    symlink_policy(&case["policy"]),
                ) {
                    Ok(m) => Ok(json!({
                        "manifest": scrub(&serde_json::to_value(&m).unwrap(), &c.needles)
                    })),
                    Err(ResourceError::Invalid(e)) => Err(cheaper_err(&e, &c.needles)),
                    Err(ResourceError::Escape(e)) => Err(escape_err(&e, &c.needles)),
                }
            }
            "resourceContains" => {
                let p = pred();
                Ok(json!(resource_contains(
                    &resource(&p, &case["a"]),
                    &resource(&p, &case["b"]),
                )))
            }
            "claimPredicates" => {
                let p = pred();
                let a = claim(&p, &case["a"]);
                let b = claim(&p, &case["b"]);
                Ok(json!({
                    "contains": claim_contains(&a, &b),
                    "conflict": claims_conflict(&a, &b),
                }))
            }
            "predicatesBundle" => {
                let p = pred();
                let r = |path: &str, s: &str| resource(&p, &json!({"path": path, "scope": s}));
                let cl = |path: &str, s: &str, m: &str| {
                    claim(
                        &p,
                        &json!({"resource": {"path": path, "scope": s}, "mode": m}),
                    )
                };
                Ok(json!({
                    "overlap": resources_overlap(&r("<ABS>/a", "subtree"), &r("<ABS>/a/b", "exact")),
                    "overlapSymmetric": resources_overlap(&r("<ABS>/a/b", "exact"), &r("<ABS>/a", "subtree")),
                    "conflict": claims_conflict(&cl("<ABS>/a", "subtree", "exclusive"), &cl("<ABS>/a/b", "exact", "shared")),
                    "conflictSymmetric": claims_conflict(&cl("<ABS>/a/b", "exact", "shared"), &cl("<ABS>/a", "subtree", "exclusive")),
                    "siblingConflict": claims_conflict(&cl("<ABS>/a", "subtree", "exclusive"), &cl("<ABS>/ab", "subtree", "exclusive")),
                }))
            }
            "resourceKeys" => {
                let p = pred();
                let ka = resource_key(&resource(&p, &case["a"]));
                let kb = resource_key(&resource(&p, &case["b"]));
                Ok(json!({
                    "keyA": scrub(&json!(ka), &p.needles),
                    "keyB": scrub(&json!(kb), &p.needles),
                }))
            }
            "manifestsConflict" => {
                let p = pred();
                let read = manifest(&p, &case["a"]);
                let write = manifest(&p, &case["b"]);
                let produce = manifest(&p, &case["c"]);
                let disjoint = manifest(&p, &case["d"]);
                let empty = manifest(&p, &case["e"]);
                Ok(json!({
                    "readVsRead": manifests_conflict(&read, &read),
                    "readVsWriteParent": manifests_conflict(&read, &write),
                    "readVsProduceParent": manifests_conflict(&read, &produce),
                    "disjoint": manifests_conflict(&read, &disjoint),
                    "emptyVsRead": manifests_conflict(&empty, &read),
                }))
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}

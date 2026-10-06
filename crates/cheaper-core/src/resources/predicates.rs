//! Port of `packages/core/src/resources/predicates.ts`.

use crate::nodepath;
use crate::resources::types::{
    ClaimMode, Resource, ResourceClaim, ResourceManifest, ResourceScope,
};

/// win32 identities are case-insensitive → lowercase for comparison.
fn identity_key(identity: &str) -> String {
    if cfg!(windows) {
        identity.to_lowercase()
    } else {
        identity.to_string()
    }
}

/// `resourceKey`: comparison key for normalized resources
/// (`JSON.stringify([kind, identityKey(path), scope])` shape).
pub fn resource_key(resource: &Resource) -> String {
    serde_json::json!([
        resource.kind(),
        identity_key(resource.path()),
        resource.scope().as_str()
    ])
    .to_string()
}

/// `resourceContains`: subtree covers itself and descendants; exact
/// covers only the same exact identity.
pub fn resource_contains(container: &Resource, contained: &Resource) -> bool {
    let parent = identity_key(container.path());
    let child = identity_key(contained.path());
    if container.scope() == ResourceScope::Exact {
        return contained.scope() == ResourceScope::Exact && parent == child;
    }
    let descendant_prefix = if parent.ends_with(nodepath::SEP) {
        parent.clone()
    } else {
        format!("{parent}{}", nodepath::SEP)
    };
    parent == child || child.starts_with(&descendant_prefix)
}

pub fn resources_overlap(left: &Resource, right: &Resource) -> bool {
    resource_contains(left, right) || resource_contains(right, left)
}

/// `claimContains`: exclusive covers shared/exclusive; shared covers
/// shared only.
pub fn claim_contains(container: &ResourceClaim, contained: &ResourceClaim) -> bool {
    (container.mode == ClaimMode::Exclusive || contained.mode == ClaimMode::Shared)
        && resource_contains(&container.resource, &contained.resource)
}

pub fn claims_conflict(left: &ResourceClaim, right: &ResourceClaim) -> bool {
    (left.mode == ClaimMode::Exclusive || right.mode == ClaimMode::Exclusive)
        && resources_overlap(&left.resource, &right.resource)
}

pub fn manifests_conflict(left: &ResourceManifest, right: &ResourceManifest) -> bool {
    let mut left_claims = left.reads.iter().chain(&left.writes).chain(&left.produces);
    let right_claims: Vec<&ResourceClaim> = right
        .reads
        .iter()
        .chain(&right.writes)
        .chain(&right.produces)
        .collect();
    left_claims.any(|a| right_claims.iter().any(|b| claims_conflict(a, b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(path: &str, scope: ResourceScope, mode: ClaimMode) -> ResourceClaim {
        ResourceClaim {
            resource: Resource::workspace(path, scope),
            mode,
        }
    }

    fn root(p: &str) -> String {
        if cfg!(windows) {
            format!("C:\\{p}").replace('/', "\\")
        } else {
            format!("/{p}")
        }
    }

    #[test]
    fn subtree_contains_descendants_and_self() {
        let parent = Resource::workspace(root("a"), ResourceScope::Subtree);
        assert!(resource_contains(
            &parent,
            &Resource::workspace(root("a/b/c"), ResourceScope::Exact)
        ));
        assert!(resource_contains(
            &parent,
            &Resource::workspace(root("a"), ResourceScope::Subtree)
        ));
        assert!(!resource_contains(
            &parent,
            &Resource::workspace(root("ab"), ResourceScope::Exact)
        ));
    }

    #[test]
    fn exact_contains_only_same_exact_identity() {
        let a = Resource::workspace(root("a"), ResourceScope::Exact);
        assert!(resource_contains(
            &a,
            &Resource::workspace(root("a"), ResourceScope::Exact)
        ));
        // An exact container does not cover a subtree claim on the same path.
        assert!(!resource_contains(
            &a,
            &Resource::workspace(root("a"), ResourceScope::Subtree)
        ));
        assert!(!resource_contains(
            &a,
            &Resource::workspace(root("a/b"), ResourceScope::Exact)
        ));
    }

    #[test]
    fn claim_contains_mode_rules() {
        let subtree = claim(&root("a"), ResourceScope::Subtree, ClaimMode::Exclusive);
        let child_shared = claim(&root("a/b"), ResourceScope::Exact, ClaimMode::Shared);
        let child_exclusive = claim(&root("a/b"), ResourceScope::Exact, ClaimMode::Exclusive);
        assert!(claim_contains(&subtree, &child_shared));
        assert!(claim_contains(&subtree, &child_exclusive));
        let shared = claim(&root("a"), ResourceScope::Subtree, ClaimMode::Shared);
        assert!(claim_contains(&shared, &child_shared));
        assert!(!claim_contains(&shared, &child_exclusive));
    }

    #[test]
    fn conflicts_require_an_exclusive_side() {
        let a = claim(&root("a"), ResourceScope::Subtree, ClaimMode::Exclusive);
        let b = claim(&root("a/b"), ResourceScope::Exact, ClaimMode::Shared);
        assert!(claims_conflict(&a, &b));
        let s1 = claim(&root("a"), ResourceScope::Subtree, ClaimMode::Shared);
        let s2 = claim(&root("a/b"), ResourceScope::Exact, ClaimMode::Shared);
        assert!(!claims_conflict(&s1, &s2));
    }

    #[test]
    fn manifests_conflict_checks_all_buckets() {
        let left = ResourceManifest {
            reads: vec![claim(&root("a"), ResourceScope::Subtree, ClaimMode::Shared)],
            ..Default::default()
        };
        let right = ResourceManifest {
            writes: vec![claim(
                &root("a/b"),
                ResourceScope::Exact,
                ClaimMode::Exclusive,
            )],
            ..Default::default()
        };
        assert!(manifests_conflict(&left, &right));
        let far = ResourceManifest {
            writes: vec![claim(
                &root("z"),
                ResourceScope::Exact,
                ClaimMode::Exclusive,
            )],
            ..Default::default()
        };
        assert!(!manifests_conflict(&left, &far));
    }
}

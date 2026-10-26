use std::env;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TRACE_ID_ENV: &str = "CHEAPER_TRACE_ID";
const PARENT_EXECUTION_ID_ENV: &str = "CHEAPER_PARENT_EXECUTION_ID";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextProvenance {
    InheritedUnverified,
}

impl ContextProvenance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InheritedUnverified => "inherited-unverified",
        }
    }
}

/// IDs owned by one local execution, kept separate from unverified process hints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionIdentity {
    execution_id: Uuid,
    trace_id: Uuid,
    parent_execution_id: Option<Uuid>,
    parent_execution_hint_id: Option<Uuid>,
    has_inherited_unverified_context: bool,
}

impl ExecutionIdentity {
    /// Creates a root execution with locally generated v4 execution and trace IDs.
    pub fn root() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_execution_id: None,
            parent_execution_hint_id: None,
            has_inherited_unverified_context: false,
        }
    }

    /// Creates a local execution while treating carrier values only as hints.
    /// Invalid or non-canonical values are ignored.
    pub fn from_inherited(trace_hint: Option<&str>, parent_hint: Option<&str>) -> Self {
        let trace_id = trace_hint.and_then(parse_canonical_v4);
        let parent_execution_hint_id = parent_hint.and_then(parse_canonical_v4);
        let has_inherited_unverified_context =
            trace_id.is_some() || parent_execution_hint_id.is_some();

        Self {
            execution_id: Uuid::new_v4(),
            trace_id: trace_id.unwrap_or_else(Uuid::new_v4),
            parent_execution_id: None,
            parent_execution_hint_id,
            has_inherited_unverified_context,
        }
    }

    /// Reads the explicit correlation carrier for the current process.
    /// Environment values never create a verified parent edge.
    pub fn from_process_environment() -> Self {
        let trace_hint = env::var(TRACE_ID_ENV).ok();
        let parent_hint = env::var(PARENT_EXECUTION_ID_ENV).ok();
        Self::from_inherited(trace_hint.as_deref(), parent_hint.as_deref())
    }

    /// Creates a lifecycle-owned child relation within this process.
    pub fn child_of(parent: &Self) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            trace_id: parent.trace_id,
            parent_execution_id: Some(parent.execution_id),
            parent_execution_hint_id: None,
            has_inherited_unverified_context: false,
        }
    }

    pub const fn execution_id(&self) -> Uuid {
        self.execution_id
    }

    pub const fn trace_id(&self) -> Uuid {
        self.trace_id
    }

    pub const fn parent_execution_id(&self) -> Option<Uuid> {
        self.parent_execution_id
    }

    pub const fn parent_execution_hint_id(&self) -> Option<Uuid> {
        self.parent_execution_hint_id
    }

    pub const fn context_provenance(&self) -> Option<ContextProvenance> {
        if self.has_inherited_unverified_context {
            Some(ContextProvenance::InheritedUnverified)
        } else {
            None
        }
    }

    pub const fn has_inherited_unverified_context(&self) -> bool {
        self.has_inherited_unverified_context
    }
}

fn parse_canonical_v4(value: &str) -> Option<Uuid> {
    let parsed = Uuid::parse_str(value).ok()?;
    (parsed.get_version_num() == 4 && parsed.hyphenated().to_string() == value).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::ExecutionIdentity;

    #[test]
    fn history_identity_issues_local_v4_ids() {
        let identity = ExecutionIdentity::root();

        assert_eq!(identity.execution_id().get_version_num(), 4);
        assert_eq!(identity.trace_id().get_version_num(), 4);
        assert_ne!(identity.execution_id(), identity.trace_id());
        assert_eq!(identity.parent_execution_id(), None);
        assert_eq!(identity.parent_execution_hint_id(), None);
    }

    #[test]
    fn history_identity_keeps_inherited_parent_as_unverified_hint() {
        let trace_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let identity = ExecutionIdentity::from_inherited(
            Some(&trace_id.to_string()),
            Some(&parent_id.to_string()),
        );

        assert_ne!(identity.execution_id(), parent_id);
        assert_eq!(identity.trace_id(), trace_id);
        assert_eq!(identity.parent_execution_id(), None);
        assert_eq!(identity.parent_execution_hint_id(), Some(parent_id));
        assert!(identity.has_inherited_unverified_context());
    }

    #[test]
    fn history_identity_rejects_noncanonical_and_non_v4_hints() {
        let v1 = Uuid::from_u128(0x00000000000010008000000000000001);
        let identity =
            ExecutionIdentity::from_inherited(Some("{not-a-canonical-id}"), Some(&v1.to_string()));

        assert_eq!(identity.trace_id().get_version_num(), 4);
        assert_ne!(identity.trace_id(), v1);
        assert_eq!(identity.parent_execution_hint_id(), None);
        assert!(!identity.has_inherited_unverified_context());
    }

    #[test]
    fn history_identity_child_uses_only_explicit_lifecycle_parent() {
        let parent = ExecutionIdentity::root();
        let child = ExecutionIdentity::child_of(&parent);

        assert_ne!(child.execution_id(), parent.execution_id());
        assert_eq!(child.trace_id(), parent.trace_id());
        assert_eq!(child.parent_execution_id(), Some(parent.execution_id()));
        assert_eq!(child.parent_execution_hint_id(), None);
        assert!(!child.has_inherited_unverified_context());
    }
}

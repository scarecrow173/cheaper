//! Port of `packages/core/src/resources/types.ts`.

use serde::{Deserialize, Serialize};

/// `ResourceScope`: `"exact" | "subtree"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceScope {
    Exact,
    Subtree,
}

impl ResourceScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Subtree => "subtree",
        }
    }
}

/// `ResourceClaim["resource"]`: `{kind: "workspace", path, scope}`.
/// `path` is the canonical absolute real path (no glob, no display path).
/// `#[serde(tag = "kind")]` reproduces the TS wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Resource {
    #[serde(rename = "workspace")]
    Workspace { path: String, scope: ResourceScope },
}

impl Resource {
    pub fn workspace(path: impl Into<String>, scope: ResourceScope) -> Self {
        Self::Workspace {
            path: path.into(),
            scope,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::Workspace { path, .. } => path,
        }
    }

    pub fn scope(&self) -> ResourceScope {
        match self {
            Self::Workspace { scope, .. } => *scope,
        }
    }

    pub fn kind(&self) -> &'static str {
        "workspace"
    }
}

/// `ClaimMode`: `"shared" | "exclusive"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClaimMode {
    Shared,
    Exclusive,
}

impl ClaimMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Exclusive => "exclusive",
        }
    }
}

/// `ResourceClaim`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceClaim {
    pub resource: Resource,
    pub mode: ClaimMode,
}

/// `ResourceManifest`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub reads: Vec<ResourceClaim>,
    pub writes: Vec<ResourceClaim>,
    pub produces: Vec<ResourceClaim>,
}

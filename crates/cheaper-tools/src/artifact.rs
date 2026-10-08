//! Port of `packages/tool-runtime/src/artifact.ts`.
//!
//! Artifact identity/provenance は plain JSON data。workspace path の意味を
//! 持たない。ここは port のみを定義し、store 実体は workflow 側
//! (Phase 4)が実装する。TS の sync interface をそのまま同期 trait と
//! して移植する(`read`/`put` は TS でも非 async)。実装が失敗し得る
//! (上限超過・store 未到達等)ため、Rust 側は `Result<_, ToolError>` を
//! 返す(TS の throw 相当)。

use serde::{Deserialize, Serialize};

use crate::ToolError;

/// `ArtifactProducer`。Artifact identity/provenance は plain JSON data。
/// workspace path の意味を持たない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactProducer {
    pub job_id: String,
    pub name: String,
}

/// `ArtifactReference`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReference {
    pub artifact: String,
    pub run_id: String,
    pub producer: ArtifactProducer,
    /// 保存した source byte 数。preview の byte 数ではない。
    pub bytes: u64,
    pub preview: String,
    pub truncated: bool,
}

/// `ArtifactSource`(`{kind:"artifact", reference}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSource {
    pub reference: ArtifactReference,
}

/// `WorkspaceSource`(`{kind:"workspace", path}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSource {
    pub path: String,
}

/// `ReadSource`。consumer が明示 dispatch する。artifact URI を workspace
/// resolve してはならない。wire shape は `{kind:"artifact"|"workspace",...}`
/// の discriminated union。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReadSource {
    #[serde(rename = "artifact")]
    Artifact(ArtifactSource),
    #[serde(rename = "workspace")]
    Workspace(WorkspaceSource),
}

/// `ArtifactReadRequest`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReadRequest {
    /// source byte 基準。0-indexed、EOF と同値まで許可。
    pub offset: u64,
    /// 正数。invocation の maxReadBytes を超える要求は拒否する。
    pub max_bytes: u64,
}

/// `ArtifactReadResult`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReadResult {
    /// 明示 bounded materialization の copy。返却後は caller 所有。
    pub data: Vec<u8>,
    pub offset: u64,
    pub next_offset: u64,
    pub eof: bool,
}

/// invocation runtime が借用する port(`ArtifactReader`)。
/// store/cache/lifetime の所有権は持たない。
pub trait ArtifactReader: Send + Sync {
    fn read(
        &self,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError>;
}

/// `ArtifactPut["data"]`(`string | Uint8Array`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactData {
    Text(String),
    Bytes(Vec<u8>),
}

impl From<String> for ArtifactData {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for ArtifactData {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<Vec<u8>> for ArtifactData {
    fn from(b: Vec<u8>) -> Self {
        Self::Bytes(b)
    }
}

/// `ArtifactPut`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPut {
    pub producer: ArtifactProducer,
    pub data: ArtifactData,
}

/// `ArtifactPublisher`。scope は借用する。producer.job_id は runtime
/// owner が固定する。
pub trait ArtifactPublisher: Send + Sync {
    fn max_artifact_bytes(&self) -> u64;
    fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError>;
}

/// `ArtifactPorts`。`Clone` は invocation → worker scope への port 共有用。
#[derive(Clone, Default)]
pub struct ArtifactPorts {
    pub reader: Option<std::sync::Arc<dyn ArtifactReader>>,
    pub publisher: Option<std::sync::Arc<dyn ArtifactPublisher>>,
}

/// `ArtifactMetadata["availability"]`(`"available" | "unavailable"`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactAvailability {
    Available,
    Unavailable,
}

/// `ArtifactMetadata`(`Omit<ArtifactReference,"preview">` + optional
/// preview + availability)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactMetadata {
    pub artifact: String,
    pub run_id: String,
    pub producer: ArtifactProducer,
    pub bytes: u64,
    pub truncated: bool,
    pub availability: ArtifactAvailability,
    /// `project()` の runtime key 順(`{…, availability, preview?}`)に
    /// 揃えるため struct では末尾に置く(type 宣言順ではなく
    /// object literal の挿入順が wire shape)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

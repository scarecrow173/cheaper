//! Port of `packages/workflow/src/artifacts.ts`.
//!
//! D4/D7: filesystem を持たず、1 invocation が所有する in-memory
//! bounded store。`AbortSignal` は `cheaper-tools::operation` の
//! first-reason port を再利用する。

use std::collections::HashSet;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::MAX_SAFE_INTEGER_U64 as MAX_SAFE_INTEGER;
use cheaper_tools::artifact::{
    ArtifactAvailability, ArtifactMetadata, ArtifactPut, ArtifactReadRequest, ArtifactReadResult,
    ArtifactReader, ArtifactReference, ArtifactSource,
};
use cheaper_tools::operation::{AbortListenerId, AbortReason, AbortSignal};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

// Re-export the port surface the TS module re-exports.
pub use cheaper_tools::artifact::{
    ArtifactData, ArtifactPorts, ArtifactPublisher, ReadSource, WorkspaceSource,
};

/// `ArtifactStoreOptions`。cap の供給元は owning invocation。
pub struct ArtifactStoreOptions {
    pub max_artifact_bytes: u64,
    pub max_total_bytes: u64,
    pub max_preview_bytes: u64,
    pub max_read_bytes: u64,
    pub signal: Option<AbortSignal>,
}

/// `ArtifactProjectionOptions`。
#[derive(Debug, Clone, Copy, Default)]
pub struct ArtifactProjectionOptions {
    /// `includePreview !== false` が既定 — `None`/`Some(true)` で preview
    /// を含める。
    pub include_preview: Option<bool>,
}

// `Number.isSafeInteger(cap)` — u64 では整数は常に整数なので
// 2^53-1 上限だけが有効な制約。
struct Entry {
    reference: ArtifactReference,
    data: Vec<u8>,
}

struct Inner {
    /// `Map` 相当 — `metadata()` は put 挿入順を返す。
    entries: Vec<(String, Entry)>,
    /// `Set<string>` 相当(`JSON.stringify([jobId, name])`)。
    producers: HashSet<String>,
    bytes: u64,
    closed: bool,
    aborted: bool,
}

/// `^artifact://(${UUID})/(${UUID})$` — lowercase hex の厳密 format。
fn parse_artifact_uri(artifact: &str) -> Option<(&str, &str)> {
    let rest = artifact.strip_prefix("artifact://")?;
    let (run_id, id) = rest.split_once('/')?;
    // `rest` に余分な `/` があれば拒否(`split_once` 後の id 側検査で
    // 自動的に失敗するが、UUID part 数の一致として明示する)。
    fn is_uuid(s: &str) -> bool {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return false;
        }
        let lens = [8, 4, 4, 4, 12];
        parts.iter().zip(lens).all(|(p, n)| {
            p.len() == n
                && p.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    }
    if is_uuid(run_id) && is_uuid(id) {
        Some((run_id, id))
    } else {
        None
    }
}

/// `project`。
fn project(
    reference: &ArtifactReference,
    availability: ArtifactAvailability,
    options: ArtifactProjectionOptions,
) -> ArtifactMetadata {
    ArtifactMetadata {
        artifact: reference.artifact.clone(),
        run_id: reference.run_id.clone(),
        producer: reference.producer.clone(),
        bytes: reference.bytes,
        truncated: reference.truncated,
        availability,
        preview: if options.include_preview != Some(false) {
            Some(reference.preview.clone())
        } else {
            None
        },
    }
}

/// `projectArtifactMetadata` — final envelope 向け。cleanup 後の
/// reference は metadata であり取得可能性を宣言しない。
pub fn project_artifact_metadata(
    reference: &ArtifactReference,
    options: ArtifactProjectionOptions,
) -> ArtifactMetadata {
    project(reference, ArtifactAvailability::Unavailable, options)
}

/// `decoder.decode(chunk, {stream})` 相当: 不完全な multibyte tail は
/// flush しない(stream=true)、不正 byte 列は U+FFFD に置換する。
fn decode_utf8_streaming(bytes: &[u8], stream: bool) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    loop {
        match std::str::from_utf8(rest) {
            Ok(s) => {
                out.push_str(s);
                break;
            }
            Err(e) => {
                out.push_str(std::str::from_utf8(&rest[..e.valid_up_to()]).expect("valid prefix"));
                match e.error_len() {
                    // invalid sequence → replacement char。
                    Some(len) => {
                        out.push('\u{FFFD}');
                        rest = &rest[e.valid_up_to() + len..];
                    }
                    // incomplete tail — stream が保持(flush しない)か、
                    // 真の EOF で replacement を置く。
                    None => {
                        if !stream {
                            out.push('\u{FFFD}');
                        }
                        break;
                    }
                }
            }
        }
    }
    out
}

/// `boundedPreview`。decoded text 全文を作らず最大 cap source bytes
/// だけを decode する。
fn bounded_preview(data: &[u8], cap: u64) -> (String, bool) {
    let cap = usize::try_from(cap).unwrap_or(usize::MAX);
    let streaming = data.len() > cap;
    let text = decode_utf8_streaming(&data[..data.len().min(cap)], streaming);
    let mut preview = String::new();
    let mut bytes = 0usize;
    for ch in text.chars() {
        let size = ch.len_utf8();
        if bytes + size > cap {
            break;
        }
        preview.push(ch);
        bytes += size;
    }
    // `data.length > cap || preview.length < text.length` — preview は
    // text の prefix のため `preview != text` と同値。
    (preview.clone(), streaming || preview != text)
}

/// `store.stats`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactStoreStats {
    pub bytes: u64,
    pub artifacts: u64,
    pub closed: bool,
}

/// `RunArtifactStore`。
pub struct RunArtifactStore {
    run_id: String,
    limits: Limits,
    inner: Arc<Mutex<Inner>>,
    registration: Mutex<Option<(AbortSignal, AbortListenerId)>>,
}

#[derive(Debug, Clone, Copy)]
struct Limits {
    max_artifact_bytes: u64,
    max_total_bytes: u64,
    max_preview_bytes: u64,
    max_read_bytes: u64,
}

impl RunArtifactStore {
    /// `new RunArtifactStore(options)`。cap 検証 → signal abort 検査 →
    /// listener 登録の順(TS constructor と同じ)。
    pub fn new(options: ArtifactStoreOptions) -> Result<Arc<Self>, CheaperError> {
        let limits = Limits {
            max_artifact_bytes: options.max_artifact_bytes,
            max_total_bytes: options.max_total_bytes,
            max_preview_bytes: options.max_preview_bytes,
            max_read_bytes: options.max_read_bytes,
        };
        for (field, cap) in [
            ("maxArtifactBytes", limits.max_artifact_bytes),
            ("maxTotalBytes", limits.max_total_bytes),
            ("maxPreviewBytes", limits.max_preview_bytes),
            ("maxReadBytes", limits.max_read_bytes),
        ] {
            if cap > MAX_SAFE_INTEGER || (field == "maxReadBytes" && cap == 0) {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("invalid artifact cap: {field}"),
                ));
            }
        }
        let inner = Arc::new(Mutex::new(Inner {
            entries: Vec::new(),
            producers: HashSet::new(),
            bytes: 0,
            closed: false,
            aborted: false,
        }));
        let store = Arc::new(Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            limits,
            inner,
            registration: Mutex::new(None),
        });
        if let Some(signal) = options.signal {
            if signal.is_aborted() {
                return Err(CheaperError::new(
                    ErrorCode::Aborted,
                    "artifact invocation aborted",
                ));
            }
            let weak: Weak<Mutex<Inner>> = Arc::downgrade(&store.inner);
            let id = signal.on_abort(move |_reason: &AbortReason| {
                if let Some(inner) = weak.upgrade() {
                    let mut guard = inner.lock().unwrap();
                    guard.aborted = true;
                    // `this.#onAbort` → `dispose()`。signal/listener は
                    // registration 側が既に drain 済み(once)なので
                    // entries の clearing だけを行う。
                    guard.closed = true;
                    guard.entries.clear();
                    guard.producers.clear();
                    guard.bytes = 0;
                }
            });
            *store.registration.lock().unwrap() = Some((signal, id));
        }
        Ok(store)
    }

    /// `store.reader` — `{read: (s, r) => this.read(s, r)}` 相当の
    /// borrowed reader handle。
    pub fn reader(self: &Arc<Self>) -> Arc<dyn ArtifactReader> {
        Arc::new(ReaderHandle {
            run_id: self.run_id.clone(),
            limits: self.limits,
            inner: Arc::clone(&self.inner),
        })
    }

    /// `store.runId`。
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// `store.stats`。
    pub fn stats(&self) -> ArtifactStoreStats {
        let inner = self.inner.lock().unwrap();
        ArtifactStoreStats {
            bytes: inner.bytes,
            artifacts: inner.entries.len() as u64,
            closed: inner.closed,
        }
    }

    /// `store.maxArtifactBytes`。
    pub fn max_artifact_bytes(&self) -> u64 {
        self.limits.max_artifact_bytes
    }

    /// `put({producer, data})`。`#assertOpen` → producer/data 検証 →
    /// duplicate → cap の順(TS と同じ検証順序)。
    pub fn put(&self, put: ArtifactPut) -> Result<ArtifactReference, CheaperError> {
        let ArtifactPut { producer, data } = put;
        let bytes: &[u8] = match &data {
            ArtifactData::Text(s) => s.as_bytes(),
            ArtifactData::Bytes(b) => b,
        };
        let bytes_len = bytes.len() as u64;
        let mut inner = self.inner.lock().unwrap();
        assert_open(&inner)?;
        if producer.job_id.is_empty() || producer.name.is_empty() {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "invalid artifact value or producer",
            ));
        }
        let producer_key = json!([producer.job_id, producer.name]).to_string();
        if inner.producers.contains(&producer_key) {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "duplicate artifact producer/name",
            ));
        }
        if bytes_len > self.limits.max_artifact_bytes
            || bytes_len > self.limits.max_total_bytes.saturating_sub(inner.bytes)
        {
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                "artifact storage limit exceeded",
            )
            .with_details({
                let mut details = Map::new();
                details.insert("bytes".to_string(), json!(bytes_len));
                details.insert("storedBytes".to_string(), json!(inner.bytes));
                details
            }));
        }
        // `Buffer.from(data)` — owned copy。
        let copy: Vec<u8> = match data {
            ArtifactData::Text(s) => s.into_bytes(),
            ArtifactData::Bytes(b) => b,
        };
        let (preview, truncated) = bounded_preview(&copy, self.limits.max_preview_bytes);
        let reference = ArtifactReference {
            artifact: format!("artifact://{}/{}", self.run_id, uuid::Uuid::new_v4()),
            run_id: self.run_id.clone(),
            producer: producer.clone(),
            bytes: bytes_len,
            preview,
            truncated,
        };
        inner.entries.push((
            reference.artifact.clone(),
            Entry {
                reference: reference.clone(),
                data: copy,
            },
        ));
        inner.producers.insert(producer_key);
        inner.bytes += bytes_len;
        Ok(reference)
    }

    /// `read(source, request)`。
    pub fn read(
        &self,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, CheaperError> {
        let inner = self.inner.lock().unwrap();
        Self::read_via_inner(&inner, &self.run_id, &self.limits, source, request)
    }

    /// `metadata(options)` — put 挿入順。
    pub fn metadata(&self, options: ArtifactProjectionOptions) -> Vec<ArtifactMetadata> {
        let inner = self.inner.lock().unwrap();
        inner
            .entries
            .iter()
            .map(|(_, e)| project(&e.reference, ArtifactAvailability::Available, options))
            .collect()
    }

    /// `toJSON`。
    pub fn to_json(&self) -> Value {
        let inner = self.inner.lock().unwrap();
        json!({
            "runId": self.run_id,
            "closed": inner.closed,
            "bytes": inner.bytes,
            "artifacts": inner
                .entries
                .iter()
                .map(|(_, e)| {
                    serde_json::to_value(project(
                        &e.reference,
                        ArtifactAvailability::Available,
                        ArtifactProjectionOptions::default(),
                    ))
                    .unwrap_or(Value::Null)
                })
                .collect::<Vec<_>>(),
        })
    }

    /// `dispose()` — abort clear は tool drain と独立。
    pub fn dispose(&self) {
        let registration = self.registration.lock().unwrap().take();
        let mut inner = self.inner.lock().unwrap();
        if inner.closed {
            // TS は早期 return — ただし listener 除去は通る
            // (`removeEventListener` は closed 後も呼ばれない: TS も
            // `if (this.#closed) return` で skip するため同じ)。
            return;
        }
        inner.closed = true;
        if let Some((signal, id)) = registration {
            signal.remove_listener(id);
        }
        inner.entries.clear();
        inner.producers.clear();
        inner.bytes = 0;
    }

    /// `read` と同じ open 検査を port impl からも使う。
    fn read_via_inner(
        inner: &Inner,
        run_id: &str,
        limits: &Limits,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, CheaperError> {
        assert_open(inner)?;
        let reference = &source.reference;
        match parse_artifact_uri(&reference.artifact) {
            Some((id, _)) if id == run_id && reference.run_id == run_id => {}
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidInput,
                    "invalid or foreign artifact identity",
                ))
            }
        }
        let entry = inner
            .entries
            .iter()
            .find(|(key, _)| key == &reference.artifact)
            .map(|(_, e)| e);
        let Some(entry) = entry else {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "unknown or inconsistent artifact reference",
            ));
        };
        if reference.producer.job_id != entry.reference.producer.job_id
            || reference.producer.name != entry.reference.producer.name
            || reference.bytes != entry.reference.bytes
            || reference.preview != entry.reference.preview
            || reference.truncated != entry.reference.truncated
        {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "unknown or inconsistent artifact reference",
            ));
        }
        let len = entry.data.len() as u64;
        if request.offset > MAX_SAFE_INTEGER
            || request.offset > len
            || request.max_bytes > MAX_SAFE_INTEGER
            || request.max_bytes == 0
        {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                "invalid artifact read range",
            ));
        }
        if request.max_bytes > limits.max_read_bytes {
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                "artifact read limit exceeded",
            ));
        }
        let offset = request.offset as usize;
        let next_offset = offset + (request.max_bytes as usize).min(entry.data.len() - offset);
        Ok(ArtifactReadResult {
            data: entry.data[offset..next_offset].to_vec(),
            offset: request.offset,
            next_offset: next_offset as u64,
            eof: next_offset == entry.data.len(),
        })
    }
}

fn assert_open(inner: &Inner) -> Result<(), CheaperError> {
    if inner.closed {
        return Err(CheaperError::new(
            if inner.aborted {
                ErrorCode::Aborted
            } else {
                ErrorCode::InvalidRequest
            },
            "artifact store is closed",
        ));
    }
    Ok(())
}

/// `ArtifactPublisher` — TS の `store` 自体が publisher port(`maxArtifactBytes`
/// getter + `put`)を実装するのと同じ。`ArtifactPorts.publisher` へ
/// `Arc<RunArtifactStore>` を直接渡せる seam。
impl ArtifactPublisher for RunArtifactStore {
    fn max_artifact_bytes(&self) -> u64 {
        self.limits.max_artifact_bytes
    }

    fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError> {
        RunArtifactStore::put(self, value).map_err(|e| Box::new(e) as ToolError)
    }
}

/// `store.reader` が返す frozen `{read}` object。dispose 後の read は
/// `closed` flag 経由で即失敗する。
struct ReaderHandle {
    run_id: String,
    limits: Limits,
    inner: Arc<Mutex<Inner>>,
}

impl ArtifactReader for ReaderHandle {
    fn read(
        &self,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError> {
        let inner = self.inner.lock().unwrap();
        RunArtifactStore::read_via_inner(&inner, &self.run_id, &self.limits, source, request)
            .map_err(|e| Box::new(e) as ToolError)
    }
}

/// `withArtifactStore` — engine 向け所有 seam。consumer 自体の drain は
/// 行わない。`run` が返す error はそのまま伝播する。
pub async fn with_artifact_store<T, Fut>(
    options: ArtifactStoreOptions,
    run: impl FnOnce(Arc<RunArtifactStore>) -> Fut,
) -> Result<T, ToolError>
where
    Fut: Future<Output = Result<T, ToolError>>,
{
    let signal = options.signal.clone();
    let store = RunArtifactStore::new(options).map_err(|e| Box::new(e) as ToolError)?;
    let result = run(Arc::clone(&store)).await;
    // `finally { store.dispose() }` — dispose は必ず走る。
    store.dispose();
    // `run()` の throw は abort 検査より先に伝播する(TS の try 本体内
    // で `await run` が先に来るため)。
    let value = result?;
    if signal.is_some_and(|s| s.is_aborted()) {
        return Err(Box::new(CheaperError::new(
            ErrorCode::Aborted,
            "artifact invocation aborted",
        )));
    }
    Ok(value)
}

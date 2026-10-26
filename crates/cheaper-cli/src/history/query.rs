use std::cmp::Ordering;
use std::fs;
use std::io;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::schema::{
    ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent, RecordedAt, ValidatedIdentifier,
    MAX_EVENT_BYTES,
};
use super::store::{HistoryStore, DECISION_FILE, STARTED_FILE, TERMINAL_FILE};

const MAX_SCAN_EXECUTIONS: usize = 20_000;
const MAX_SCAN_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SCAN_TIME: Duration = Duration::from_millis(500);
const MAX_PAGE_SIZE: usize = 500;
const MAX_CURSOR_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordState {
    Complete,
    Incomplete,
    Corrupt,
    NotFound,
}

#[derive(Debug, Clone, Default)]
pub struct HistoryFilter {
    pub kind: Option<ExecutionKind>,
    pub status: Option<ExecutionStatus>,
    pub worker_id: Option<ValidatedIdentifier>,
    pub since: Option<RecordedAt>,
}

impl HistoryFilter {
    pub fn with_worker_id(mut self, value: &str) -> Option<Self> {
        self.worker_id = Some(ValidatedIdentifier::new(value)?);
        Some(self)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListRecord {
    pub event: HistoryEvent,
    pub status: ExecutionStatus,
    pub record_state: RecordState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub records: Vec<HistoryListRecord>,
    pub scan_complete: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDetail {
    pub started: Option<HistoryEvent>,
    pub decision: Option<HistoryEvent>,
    pub terminal: Option<HistoryEvent>,
    pub state: RecordState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCursor {
    recorded_at: RecordedAt,
    execution_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CursorWire {
    version: u8,
    recorded_at: String,
    execution_id: String,
}

impl HistoryCursor {
    pub fn decode(value: &str) -> Option<Self> {
        if value.len() > MAX_CURSOR_BYTES {
            return None;
        }
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        let wire: CursorWire = serde_json::from_slice(&bytes).ok()?;
        let recorded_at = RecordedAt::parse_utc(&wire.recorded_at)?;
        let id = Uuid::parse_str(&wire.execution_id).ok()?;
        (wire.version == 1
            && id.get_version_num() == 4
            && id.hyphenated().to_string() == wire.execution_id)
            .then_some(Self {
                recorded_at,
                execution_id: wire.execution_id,
            })
    }

    fn from_event(event: &HistoryEvent) -> Self {
        Self {
            recorded_at: event.recorded_at().clone(),
            execution_id: event.execution_id().to_owned(),
        }
    }

    fn encode(&self) -> String {
        let wire = CursorWire {
            version: 1,
            recorded_at: self.recorded_at.as_str().to_owned(),
            execution_id: self.execution_id.clone(),
        };
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&wire).expect("cursor fields serialize"))
    }
}

pub struct HistoryQuery<'a> {
    store: &'a HistoryStore,
}

impl<'a> HistoryQuery<'a> {
    pub fn new(store: &'a HistoryStore) -> Self {
        Self { store }
    }

    pub fn list(
        &self,
        filter: &HistoryFilter,
        limit: usize,
        cursor: Option<&HistoryCursor>,
    ) -> HistoryPage {
        self.scan(filter, limit, cursor, None)
    }

    pub fn trace(
        &self,
        trace_id: &str,
        limit: usize,
        cursor: Option<&HistoryCursor>,
    ) -> HistoryPage {
        if !canonical_v4(trace_id) {
            return empty_page(true);
        }
        self.scan(&HistoryFilter::default(), limit, cursor, Some(trace_id))
    }

    pub fn show(&self, execution_id: &str) -> HistoryDetail {
        let Some(id) = parse_canonical_v4(execution_id) else {
            return not_found();
        };
        let dir = self.store.root_dir().join(id.to_string());
        let Ok(metadata) = fs::symlink_metadata(&dir) else {
            return not_found();
        };
        if !metadata.file_type().is_dir() {
            return not_found();
        }
        let started = self
            .store
            .read_event(id, STARTED_FILE)
            .filter(|event| event.event() == ExecutionEvent::Started);
        let decision = self
            .store
            .read_event(id, DECISION_FILE)
            .filter(|event| event.event() == ExecutionEvent::Decision);
        let mut terminal = None;
        if let Some(started_event) = started.as_ref() {
            terminal = self.store.read_event(id, TERMINAL_FILE).filter(|event| {
                event.event() == ExecutionEvent::Terminal
                    && started_event.same_lifecycle_identity(event)
            });
        }
        let has_any_event_file = [STARTED_FILE, DECISION_FILE, TERMINAL_FILE]
            .iter()
            .any(|name| event_file_size(&dir.join(name)).is_ok_and(|size| size.is_some()));
        let state = if started.is_some() {
            if terminal.is_some() {
                RecordState::Complete
            } else {
                RecordState::Incomplete
            }
        } else if decision.is_some() {
            RecordState::Complete
        } else if has_any_event_file {
            RecordState::Corrupt
        } else {
            RecordState::NotFound
        };
        HistoryDetail {
            started,
            decision,
            terminal,
            state,
        }
    }

    fn scan(
        &self,
        filter: &HistoryFilter,
        limit: usize,
        cursor: Option<&HistoryCursor>,
        trace_id: Option<&str>,
    ) -> HistoryPage {
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        let read_dir = match fs::read_dir(self.store.root_dir()) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return empty_page(true),
            Err(_) => return empty_page(false),
        };
        let began = Instant::now();
        let mut scanned_bytes = 0u64;
        let mut scan_complete = true;
        let mut records: Vec<HistoryListRecord> = Vec::with_capacity(limit.saturating_add(1));

        for (directories, entry) in read_dir.enumerate() {
            if directories >= MAX_SCAN_EXECUTIONS || began.elapsed() >= MAX_SCAN_TIME {
                scan_complete = false;
                break;
            }
            let Ok(entry) = entry else {
                scan_complete = false;
                break;
            };
            let Ok(file_type) = entry.file_type() else {
                scan_complete = false;
                break;
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(id) = parse_canonical_v4(&name) else {
                continue;
            };
            let dir = entry.path();
            let started =
                match scanned_event(self.store, id, &dir, STARTED_FILE, &mut scanned_bytes) {
                    Ok(event) => event.filter(|event| event.event() == ExecutionEvent::Started),
                    Err(()) => {
                        scan_complete = false;
                        break;
                    }
                };
            let decision =
                match scanned_event(self.store, id, &dir, DECISION_FILE, &mut scanned_bytes) {
                    Ok(event) => event.filter(|event| event.event() == ExecutionEvent::Decision),
                    Err(()) => {
                        scan_complete = false;
                        break;
                    }
                };
            let terminal =
                match scanned_event(self.store, id, &dir, TERMINAL_FILE, &mut scanned_bytes) {
                    Ok(event) => event.filter(|event| event.event() == ExecutionEvent::Terminal),
                    Err(()) => {
                        scan_complete = false;
                        break;
                    }
                };
            if began.elapsed() >= MAX_SCAN_TIME {
                scan_complete = false;
                break;
            }
            let Some(event) = started.as_ref().or(decision.as_ref()) else {
                continue;
            };
            if trace_id.is_some_and(|wanted| event.trace_id() != Some(wanted)) {
                continue;
            }
            let terminal = terminal.filter(|terminal| {
                started
                    .as_ref()
                    .is_some_and(|started| started.same_lifecycle_identity(terminal))
            });
            let state = if started.is_some() {
                if terminal.is_some() {
                    RecordState::Complete
                } else {
                    RecordState::Incomplete
                }
            } else {
                RecordState::Complete
            };
            let status = terminal
                .as_ref()
                .map_or(event.status(), HistoryEvent::status);
            if !matches_filter(event, terminal.as_ref(), status, filter)
                || cursor.is_some_and(|cursor| !after_cursor(event, cursor))
            {
                continue;
            }
            let record = HistoryListRecord {
                event: event.clone(),
                status,
                record_state: state,
            };
            insert_sorted_bounded(&mut records, record, limit + 1);
        }

        let has_more = scan_complete && records.len() > limit;
        records.truncate(limit);
        let next_cursor = if has_more {
            records
                .last()
                .map(|record| HistoryCursor::from_event(&record.event).encode())
        } else {
            None
        };
        HistoryPage {
            records,
            scan_complete,
            next_cursor,
        }
    }
}

fn scanned_event(
    store: &HistoryStore,
    id: Uuid,
    directory: &std::path::Path,
    filename: &str,
    scanned_bytes: &mut u64,
) -> Result<Option<HistoryEvent>, ()> {
    let Some(size) = event_file_size(&directory.join(filename)).map_err(|_| ())? else {
        return Ok(None);
    };
    *scanned_bytes = scanned_bytes.saturating_add(size);
    if *scanned_bytes > MAX_SCAN_BYTES {
        return Err(());
    }
    if size > MAX_EVENT_BYTES as u64 {
        return Ok(None);
    }
    Ok(store.read_event(id, filename))
}

fn event_file_size(path: &std::path::Path) -> io::Result<Option<u64>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(Some(metadata.len())),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn matches_filter(
    event: &HistoryEvent,
    terminal: Option<&HistoryEvent>,
    status: ExecutionStatus,
    filter: &HistoryFilter,
) -> bool {
    filter.kind.is_none_or(|kind| kind == event.kind())
        && filter.status.is_none_or(|wanted| wanted == status)
        && filter.worker_id.as_ref().is_none_or(|worker| {
            event.worker_id() == Some(worker.as_str())
                || terminal.and_then(HistoryEvent::worker_id) == Some(worker.as_str())
        })
        && filter
            .since
            .as_ref()
            .is_none_or(|since| event.recorded_at().sort_key() >= since.sort_key())
}

fn after_cursor(event: &HistoryEvent, cursor: &HistoryCursor) -> bool {
    (event.recorded_at().sort_key(), event.execution_id())
        < (cursor.recorded_at.as_str(), cursor.execution_id.as_str())
}

fn insert_sorted_bounded(
    records: &mut Vec<HistoryListRecord>,
    record: HistoryListRecord,
    max: usize,
) {
    let index = records
        .binary_search_by(|existing| compare_records(existing, &record))
        .unwrap_or_else(|index| index);
    records.insert(index, record);
    if records.len() > max {
        records.pop();
    }
}

fn compare_records(left: &HistoryListRecord, right: &HistoryListRecord) -> Ordering {
    right
        .event
        .recorded_at()
        .sort_key()
        .cmp(left.event.recorded_at().sort_key())
        .then_with(|| right.event.execution_id().cmp(left.event.execution_id()))
}

fn parse_canonical_v4(value: &str) -> Option<Uuid> {
    let id = Uuid::parse_str(value).ok()?;
    (id.get_version_num() == 4 && id.hyphenated().to_string() == value).then_some(id)
}

fn canonical_v4(value: &str) -> bool {
    parse_canonical_v4(value).is_some()
}

fn empty_page(scan_complete: bool) -> HistoryPage {
    HistoryPage {
        records: Vec::new(),
        scan_complete,
        next_cursor: None,
    }
}

fn not_found() -> HistoryDetail {
    HistoryDetail {
        started: None,
        decision: None,
        terminal: None,
        state: RecordState::NotFound,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;

    use crate::history::identity::ExecutionIdentity;
    use crate::history::schema::{
        ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent, RecordedAt,
    };
    use crate::history::store::{HistoryStore, TERMINAL_FILE};

    use super::{HistoryCursor, HistoryFilter, HistoryQuery, RecordState};

    #[test]
    fn history_query_keeps_missing_store_read_only() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let query = HistoryQuery::new(&store);

        assert!(
            query
                .list(&HistoryFilter::default(), 10, None)
                .scan_complete
        );
        assert!(!store.root_dir().exists());
        assert_eq!(
            query.show("2f1f7fd9-4c5b-4dc3-b225-1f207a133215").state,
            RecordState::NotFound
        );
        assert!(!store.root_dir().exists());
    }

    #[test]
    fn history_query_ignores_pending_publications() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let event = HistoryEvent::new(
            &identity,
            ExecutionKind::Worker,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        );
        let pending = store.root_dir().join(".pending-crashed-writer");
        fs::create_dir_all(&pending).unwrap();
        fs::write(
            pending.join("started.json"),
            event.to_bounded_json().unwrap(),
        )
        .unwrap();

        let page = HistoryQuery::new(&store).list(&HistoryFilter::default(), 10, None);
        assert!(page.scan_complete);
        assert!(page.records.is_empty());
        assert!(pending.exists());
    }

    #[test]
    fn history_query_reports_valid_start_as_incomplete_when_terminal_is_corrupt() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let start = HistoryEvent::new(
            &identity,
            ExecutionKind::Worker,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        );
        assert!(store.write_started(&start));
        fs::write(
            store
                .root_dir()
                .join(identity.execution_id().to_string())
                .join(TERMINAL_FILE),
            b"{truncated",
        )
        .unwrap();

        let detail = HistoryQuery::new(&store).show(&identity.execution_id().to_string());
        assert_eq!(detail.state, RecordState::Incomplete);
        assert_eq!(detail.started.unwrap().status(), ExecutionStatus::Running);
        assert!(detail.terminal.is_none());
    }

    #[test]
    fn history_query_pages_equal_timestamps_in_execution_id_order() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let timestamp = RecordedAt::parse_utc("2026-10-27T10:00:00.000Z").unwrap();
        let mut expected = Vec::new();
        for _ in 0..3 {
            let identity = ExecutionIdentity::root();
            let start = HistoryEvent::new_at(
                &identity,
                ExecutionKind::Worker,
                ExecutionEvent::Started,
                ExecutionStatus::Running,
                timestamp.clone(),
            );
            expected.push(identity.execution_id().to_string());
            assert!(store.write_started(&start));
        }
        expected.sort_by(|left, right| right.cmp(left));

        let query = HistoryQuery::new(&store);
        let first = query.list(&HistoryFilter::default(), 2, None);
        assert!(first.scan_complete);
        assert_eq!(
            first
                .records
                .iter()
                .map(|r| r.event.execution_id())
                .collect::<Vec<_>>(),
            expected[..2].iter().map(String::as_str).collect::<Vec<_>>()
        );
        let cursor = HistoryCursor::decode(first.next_cursor.as_deref().unwrap()).unwrap();
        let second = query.list(&HistoryFilter::default(), 2, Some(&cursor));
        assert!(second.scan_complete);
        assert_eq!(second.records.len(), 1);
        assert_eq!(second.records[0].event.execution_id(), expected[2]);
        assert!(second.next_cursor.is_none());
    }

    #[test]
    fn history_query_scan_budget_returns_incomplete_without_cursor() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        for index in 0..20_001 {
            fs::create_dir(store.root_dir().join(format!("foreign-{index}"))).unwrap();
        }

        let page = HistoryQuery::new(&store).list(&HistoryFilter::default(), 10, None);
        assert!(!page.scan_complete);
        assert!(page.next_cursor.is_none());
    }
}

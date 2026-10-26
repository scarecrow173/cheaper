use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use super::schema::{ExecutionEvent, HistoryEvent, RecordedAt, MAX_EVENT_BYTES};

pub const STARTED_FILE: &str = "started.json";
pub const DECISION_FILE: &str = "decision.json";
pub const TERMINAL_FILE: &str = "terminal.json";

const PENDING_PREFIX: &str = ".pending-";
const MAX_RETAINED_EXECUTIONS: usize = 10_000;
const MAX_RETAINED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SWEEP_ENTRIES: usize = 20_000;
const MAX_SWEEP_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SWEEP_TIME: Duration = Duration::from_millis(500);
const ORPHAN_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const RETENTION_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Debug, Clone)]
pub struct HistoryStore {
    root_dir: PathBuf,
}

#[derive(Debug)]
struct RetainedDirectory {
    path: PathBuf,
    execution_id: String,
    modified: SystemTime,
    order_key: String,
    bytes: u64,
    expired: bool,
    orphan: bool,
}

impl HistoryStore {
    /// Resolves the History directory under the user's config directory without creating it.
    pub fn new(user_config_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: user_config_dir.as_ref().join("history"),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Publishes a started event through a private directory and same-parent rename.
    pub fn write_started(&self, event: &HistoryEvent) -> bool {
        self.publish_once(event, ExecutionEvent::Started, STARTED_FILE)
    }

    /// Publishes a one-shot decision event through a private directory and rename.
    pub fn write_decision(&self, event: &HistoryEvent) -> bool {
        self.publish_once(event, ExecutionEvent::Decision, DECISION_FILE)
    }

    /// Adds the first terminal event to an existing, valid started execution.
    pub fn write_terminal(&self, event: &HistoryEvent) -> bool {
        if event.event() != ExecutionEvent::Terminal {
            return false;
        }
        let Some(bytes) = event.to_bounded_json() else {
            return false;
        };
        let execution_id = match Uuid::parse_str(event.execution_id()) {
            Ok(id)
                if id.get_version_num() == 4
                    && id.hyphenated().to_string() == event.execution_id() =>
            {
                id
            }
            _ => return false,
        };
        let execution_dir = self.root_dir.join(execution_id.to_string());
        let Ok(directory_metadata) = fs::symlink_metadata(&execution_dir) else {
            return false;
        };
        if !directory_metadata.file_type().is_dir() {
            return false;
        }
        let Some(started) = read_event_file(&execution_dir.join(STARTED_FILE)) else {
            return false;
        };
        if started.event() != ExecutionEvent::Started || !started.same_lifecycle_identity(event) {
            return false;
        }
        write_exclusive(&execution_dir.join(TERMINAL_FILE), &bytes).is_ok()
    }

    pub(crate) fn read_event(&self, execution_id: Uuid, name: &str) -> Option<HistoryEvent> {
        if !matches!(name, STARTED_FILE | DECISION_FILE | TERMINAL_FILE) {
            return None;
        }
        let execution_dir = self.root_dir.join(execution_id.to_string());
        if !fs::symlink_metadata(&execution_dir)
            .ok()?
            .file_type()
            .is_dir()
        {
            return None;
        }
        read_event_file(&execution_dir.join(name))
            .filter(|event| event.execution_id() == execution_id.hyphenated().to_string())
    }

    fn publish_once(&self, event: &HistoryEvent, expected: ExecutionEvent, filename: &str) -> bool {
        if event.event() != expected {
            return false;
        }
        let Some(bytes) = event.to_bounded_json() else {
            return false;
        };
        let execution_id = match Uuid::parse_str(event.execution_id()) {
            Ok(id)
                if id.get_version_num() == 4
                    && id.hyphenated().to_string() == event.execution_id() =>
            {
                id
            }
            _ => return false,
        };
        let final_dir = self.root_dir.join(execution_id.to_string());
        if fs::symlink_metadata(&final_dir).is_ok() {
            return false;
        }
        if !self.sweep_before_write(bytes.len() as u64)
            || fs::create_dir_all(&self.root_dir).is_err()
        {
            return false;
        }

        let mut pending_dir = None;
        for _ in 0..4 {
            let candidate = self
                .root_dir
                .join(format!("{PENDING_PREFIX}{}", Uuid::new_v4()));
            match fs::create_dir(&candidate) {
                Ok(()) => {
                    pending_dir = Some(candidate);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return false,
            }
        }
        let Some(pending_dir) = pending_dir else {
            return false;
        };
        let result = write_exclusive(&pending_dir.join(filename), &bytes)
            .and_then(|()| fs::rename(&pending_dir, &final_dir));
        if result.is_err() {
            let _ = fs::remove_dir_all(&pending_dir);
            return false;
        }
        true
    }

    fn sweep_before_write(&self, incoming_bytes: u64) -> bool {
        self.sweep_at_with_capacity(SystemTime::now(), 1, incoming_bytes)
    }

    #[cfg(test)]
    fn sweep_at(&self, now: SystemTime) -> bool {
        self.sweep_at_with_capacity(now, 0, 0)
    }

    fn sweep_at_with_capacity(
        &self,
        now: SystemTime,
        incoming_count: usize,
        incoming_bytes: u64,
    ) -> bool {
        let read_dir = match fs::read_dir(&self.root_dir) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
            Err(_) => return false,
        };
        let started_at = Instant::now();
        let cutoff_orphan = now.checked_sub(ORPHAN_AGE).unwrap_or(UNIX_EPOCH);
        let cutoff_retention = now.checked_sub(RETENTION_AGE).unwrap_or(UNIX_EPOCH);
        let retention_cutoff = RecordedAt::from_system_time(cutoff_retention)
            .as_str()
            .to_owned();
        let mut scanned_bytes = 0u64;
        let mut retained = Vec::new();
        let mut old_pending = Vec::new();

        for (visited, entry) in read_dir.enumerate() {
            if visited >= MAX_SWEEP_ENTRIES || started_at.elapsed() >= MAX_SWEEP_TIME {
                return false;
            }
            let Ok(entry) = entry else { return false };
            let Ok(file_type) = entry.file_type() else {
                return false;
            };
            if !file_type.is_dir() {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                return false;
            };
            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.starts_with(PENDING_PREFIX) {
                for filename in [STARTED_FILE, DECISION_FILE, TERMINAL_FILE] {
                    let path = entry.path().join(filename);
                    let Ok(metadata) = fs::symlink_metadata(&path) else {
                        continue;
                    };
                    if metadata.file_type().is_file() {
                        scanned_bytes = scanned_bytes.saturating_add(metadata.len());
                        if scanned_bytes > MAX_SWEEP_BYTES {
                            return false;
                        }
                    }
                }
                if modified <= cutoff_orphan {
                    old_pending.push(entry.path());
                }
                continue;
            }
            let Ok(id) = Uuid::parse_str(&name) else {
                continue;
            };
            if id.get_version_num() != 4 || id.hyphenated().to_string() != name {
                continue;
            }

            let mut bytes = 0u64;
            for filename in [STARTED_FILE, DECISION_FILE, TERMINAL_FILE] {
                let path = entry.path().join(filename);
                let Ok(metadata) = fs::symlink_metadata(&path) else {
                    if path.exists() {
                        return false;
                    }
                    continue;
                };
                if !metadata.file_type().is_file() {
                    continue;
                }
                bytes = bytes.saturating_add(metadata.len());
                scanned_bytes = scanned_bytes.saturating_add(metadata.len());
                if scanned_bytes > MAX_SWEEP_BYTES {
                    return false;
                }
            }
            let started = read_event_file(&entry.path().join(STARTED_FILE)).filter(|event| {
                event.event() == ExecutionEvent::Started && event.execution_id() == name
            });
            let decision = read_event_file(&entry.path().join(DECISION_FILE)).filter(|event| {
                event.event() == ExecutionEvent::Decision && event.execution_id() == name
            });
            let valid_lifecycle = started.is_some() || decision.is_some();
            let order_key = started
                .as_ref()
                .or(decision.as_ref())
                .map(|event| event.recorded_at().sort_key().to_owned())
                .unwrap_or_else(|| RecordedAt::from_system_time(modified).as_str().to_owned());
            let expired = valid_lifecycle && order_key < retention_cutoff;
            let orphan = !valid_lifecycle && modified <= cutoff_orphan;
            retained.push(RetainedDirectory {
                path: entry.path(),
                execution_id: name.into_owned(),
                modified,
                order_key,
                bytes,
                expired,
                orphan,
            });
        }

        for path in old_pending {
            let _ = fs::remove_dir_all(path);
        }
        retained.retain(|entry| {
            if entry.expired || entry.orphan {
                fs::remove_dir_all(&entry.path).is_err()
            } else {
                true
            }
        });

        retained.sort_by(|left, right| {
            left.order_key
                .cmp(&right.order_key)
                .then_with(|| left.execution_id.cmp(&right.execution_id))
                .then_with(|| left.modified.cmp(&right.modified))
        });
        let mut total_count = retained.len();
        let mut total_bytes = retained.iter().map(|entry| entry.bytes).sum::<u64>();
        for entry in retained {
            if total_count.saturating_add(incoming_count) <= MAX_RETAINED_EXECUTIONS
                && total_bytes.saturating_add(incoming_bytes) <= MAX_RETAINED_BYTES
            {
                break;
            }
            if fs::remove_dir_all(&entry.path).is_ok() {
                total_count = total_count.saturating_sub(1);
                total_bytes = total_bytes.saturating_sub(entry.bytes);
            }
        }
        started_at.elapsed() < MAX_SWEEP_TIME
            && total_count.saturating_add(incoming_count) <= MAX_RETAINED_EXECUTIONS
            && total_bytes.saturating_add(incoming_bytes) <= MAX_RETAINED_BYTES
    }
}

fn write_exclusive(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)
}

fn read_event_file(path: &Path) -> Option<HistoryEvent> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_EVENT_BYTES as u64 {
        return None;
    }
    let file = File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_EVENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    HistoryEvent::from_bounded_json(&bytes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::{Duration, SystemTime};

    use tempfile::tempdir;

    use crate::history::identity::ExecutionIdentity;
    use crate::history::schema::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent};

    use super::{HistoryStore, DECISION_FILE, STARTED_FILE, TERMINAL_FILE};

    fn event(
        identity: &ExecutionIdentity,
        event: ExecutionEvent,
        status: ExecutionStatus,
    ) -> HistoryEvent {
        HistoryEvent::new(identity, ExecutionKind::Worker, event, status)
    }

    #[test]
    fn history_store_lazily_publishes_started_event_without_pending_visibility() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let execution_dir = store.root_dir().join(identity.execution_id().to_string());

        assert!(!store.root_dir().exists());
        assert!(store.write_started(&event(
            &identity,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        assert!(execution_dir.join(STARTED_FILE).is_file());
        assert!(!execution_dir.join(TERMINAL_FILE).exists());
        assert_eq!(fs::read_dir(store.root_dir()).unwrap().count(), 1);
        assert!(HistoryEvent::from_bounded_json(
            &fs::read(execution_dir.join(STARTED_FILE)).unwrap()
        )
        .is_some());
    }

    #[test]
    fn history_store_terminal_is_first_writer_wins() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let start = event(&identity, ExecutionEvent::Started, ExecutionStatus::Running);
        assert!(store.write_started(&start));

        let failed = event(&identity, ExecutionEvent::Terminal, ExecutionStatus::Failed);
        let completed = event(
            &identity,
            ExecutionEvent::Terminal,
            ExecutionStatus::Completed,
        );
        assert!(store.write_terminal(&failed));
        assert!(!store.write_terminal(&completed));

        let terminal = store
            .read_event(identity.execution_id(), TERMINAL_FILE)
            .unwrap();
        assert_eq!(terminal.status(), ExecutionStatus::Failed);
    }

    #[test]
    fn history_store_terminal_requires_a_valid_started_record_and_never_recreates_it() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let start = event(&identity, ExecutionEvent::Started, ExecutionStatus::Running);
        assert!(!store.write_terminal(&event(
            &identity,
            ExecutionEvent::Terminal,
            ExecutionStatus::Completed,
        )));
        assert!(store.write_started(&start));

        let execution_dir = store.root_dir().join(identity.execution_id().to_string());
        fs::remove_dir_all(&execution_dir).unwrap();
        assert!(!store.write_terminal(&event(
            &identity,
            ExecutionEvent::Terminal,
            ExecutionStatus::Completed,
        )));
        assert!(!execution_dir.exists());
    }

    #[test]
    fn history_store_publishes_decision_only_records_atomically() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let decision = event(
            &identity,
            ExecutionEvent::Decision,
            ExecutionStatus::Skipped,
        );

        assert!(store.write_decision(&decision));
        let execution_dir = store.root_dir().join(identity.execution_id().to_string());
        assert!(execution_dir.join(DECISION_FILE).is_file());
        assert!(!execution_dir.join(STARTED_FILE).exists());
        assert!(HistoryEvent::from_bounded_json(
            &fs::read(execution_dir.join(DECISION_FILE)).unwrap()
        )
        .is_some());
    }

    #[test]
    fn history_store_drops_writes_when_store_parent_is_unwritable() {
        let temp = tempdir().unwrap();
        let blocked = temp.path().join("is-a-file");
        fs::write(&blocked, b"keep").unwrap();
        let store = HistoryStore::new(&blocked);

        assert!(!store.write_started(&event(
            &ExecutionIdentity::root(),
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        assert_eq!(fs::read(blocked).unwrap(), b"keep");
    }

    #[test]
    fn history_store_keeps_valid_start_and_deletes_old_orphan_directories() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        assert!(store.write_started(&event(
            &identity,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        let valid_dir = store.root_dir().join(identity.execution_id().to_string());
        fs::write(valid_dir.join(TERMINAL_FILE), b"{ truncated").unwrap();

        let decision_identity = ExecutionIdentity::root();
        assert!(store.write_decision(&HistoryEvent::new(
            &decision_identity,
            ExecutionKind::RouteDecision,
            ExecutionEvent::Decision,
            ExecutionStatus::Skipped,
        )));
        let decision_dir = store
            .root_dir()
            .join(decision_identity.execution_id().to_string());

        let orphan_id = uuid::Uuid::new_v4();
        let orphan_dir = store.root_dir().join(orphan_id.to_string());
        fs::create_dir(&orphan_dir).unwrap();
        fs::write(orphan_dir.join(STARTED_FILE), b"{ truncated").unwrap();
        let pending_dir = store.root_dir().join(".pending-crashed-writer");
        fs::create_dir(&pending_dir).unwrap();
        fs::write(pending_dir.join(STARTED_FILE), b"partial").unwrap();

        assert!(store.sweep_at(SystemTime::now() + Duration::from_secs(48 * 60 * 60)));
        assert!(
            valid_dir.exists(),
            "a valid start remains retained when its terminal is corrupt"
        );
        assert!(
            decision_dir.exists(),
            "a valid decision is not a 24-hour orphan"
        );
        assert!(!orphan_dir.exists());
        assert!(!pending_dir.exists());
    }

    #[test]
    fn history_store_late_terminal_does_not_recreate_a_retained_record() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        let identity = ExecutionIdentity::root();
        let expired = HistoryEvent::new_at(
            &identity,
            ExecutionKind::Worker,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
            crate::history::schema::RecordedAt::parse_utc("2020-01-01T00:00:00.000Z").unwrap(),
        );
        assert!(store.write_started(&expired));
        let execution_dir = store.root_dir().join(identity.execution_id().to_string());

        assert!(store.sweep_at(SystemTime::now()));
        assert!(!execution_dir.exists());
        assert!(!store.write_terminal(&event(
            &identity,
            ExecutionEvent::Terminal,
            ExecutionStatus::Completed,
        )));
        assert!(!execution_dir.exists());
    }

    #[test]
    fn history_store_allows_independent_concurrent_execution_publications() {
        let temp = tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let workers = (0..8)
            .map(|_| {
                let root = root.clone();
                thread::spawn(move || {
                    let store = HistoryStore::new(root);
                    let identity = ExecutionIdentity::root();
                    assert!(store.write_started(&event(
                        &identity,
                        ExecutionEvent::Started,
                        ExecutionStatus::Running,
                    )));
                    identity.execution_id()
                })
            })
            .collect::<Vec<_>>();
        let ids = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        let store = HistoryStore::new(root);

        for id in ids {
            assert!(store.read_event(id, STARTED_FILE).is_some());
        }
    }
}

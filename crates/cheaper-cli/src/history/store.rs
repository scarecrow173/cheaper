use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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
// A shared marker amortizes scans across processes. Its age also bounds crash recovery.
const SWEEP_LEASE_INTERVAL: Duration = Duration::from_secs(2);
const SWEEP_LOCK_WAIT: Duration = Duration::from_millis(10);
const SWEEP_LEASE_FILE: &str = ".sweep-lease";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SweepMarkerState {
    Pending,
    Success,
    Failed,
}

struct SweepMarker {
    state: SweepMarkerState,
    marked_at: SystemTime,
    fresh: bool,
    remaining_count: usize,
    remaining_bytes: u64,
}

impl SweepMarkerState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

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
        // Publish only after a bounded sweep proves the new event fits capacity.
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
        let now = SystemTime::now();
        self.sweep_before_write_with_capacity_at(now, 1, incoming_bytes)
    }

    fn sweep_before_write_with_capacity_at(
        &self,
        now: SystemTime,
        incoming_count: usize,
        incoming_bytes: u64,
    ) -> bool {
        match fs::symlink_metadata(&self.root_dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
            Err(_) => return false,
            Ok(metadata) if !metadata.file_type().is_dir() => return false,
            Ok(_) => {}
        }

        let Some(mut lease) = self.try_lock_sweep_lease() else {
            return false;
        };

        if let Some(marker) = read_sweep_marker(&mut lease, now) {
            if marker.fresh && marker.state == SweepMarkerState::Success {
                if marker.remaining_count >= incoming_count
                    && marker.remaining_bytes >= incoming_bytes
                {
                    return write_sweep_marker_with_capacity(
                        &mut lease,
                        SweepMarkerState::Success,
                        marker.marked_at,
                        marker.remaining_count - incoming_count,
                        marker.remaining_bytes - incoming_bytes,
                    );
                }
                // The prior sweep did not reserve enough room for this incoming
                // event. Re-sweep under the same cross-process lease before admitting it.
            } else if marker.fresh {
                return false;
            }
        }
        if !write_sweep_marker_with_capacity(&mut lease, SweepMarkerState::Pending, now, 0, 0) {
            return false;
        }

        let capacity = self.sweep_at_with_capacity(now, incoming_count, incoming_bytes);
        let marker_written = write_sweep_marker_with_capacity(
            &mut lease,
            if capacity.is_some() {
                SweepMarkerState::Success
            } else {
                SweepMarkerState::Failed
            },
            SystemTime::now(),
            capacity.map_or(0, |(count, _)| count),
            capacity.map_or(0, |(_, bytes)| bytes),
        );
        capacity.is_some() && marker_written
    }

    fn try_lock_sweep_lease(&self) -> Option<File> {
        let lease_path = self.root_dir.join(SWEEP_LEASE_FILE);
        let Ok(lease) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lease_path)
        else {
            return None;
        };
        // The OS releases this lock if the process exits. Keeping the inode stable
        // avoids stale-marker removal races between concurrent sweep contenders.
        let deadline = Instant::now() + SWEEP_LOCK_WAIT;
        loop {
            match lease.try_lock() {
                Ok(()) => break,
                Err(_) if Instant::now() < deadline => std::thread::yield_now(),
                Err(_) => return None,
            }
        }
        Some(lease)
    }

    #[cfg(test)]
    fn sweep_at(&self, now: SystemTime) -> bool {
        self.sweep_at_with_capacity(now, 0, 0).is_some()
    }

    fn sweep_at_with_capacity(
        &self,
        now: SystemTime,
        incoming_count: usize,
        incoming_bytes: u64,
    ) -> Option<(usize, u64)> {
        let read_dir = match fs::read_dir(&self.root_dir) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Some((MAX_RETAINED_EXECUTIONS, MAX_RETAINED_BYTES));
            }
            Err(_) => return None,
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
                return None;
            }
            let Ok(entry) = entry else { return None };
            let Ok(file_type) = entry.file_type() else {
                return None;
            };
            if !file_type.is_dir() {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                return None;
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
                            return None;
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
                        return None;
                    }
                    continue;
                };
                if !metadata.file_type().is_file() {
                    continue;
                }
                bytes = bytes.saturating_add(metadata.len());
                scanned_bytes = scanned_bytes.saturating_add(metadata.len());
                if scanned_bytes > MAX_SWEEP_BYTES {
                    return None;
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
        (started_at.elapsed() < MAX_SWEEP_TIME
            && total_count.saturating_add(incoming_count) <= MAX_RETAINED_EXECUTIONS
            && total_bytes.saturating_add(incoming_bytes) <= MAX_RETAINED_BYTES)
            .then(|| {
                (
                    MAX_RETAINED_EXECUTIONS - total_count - incoming_count,
                    MAX_RETAINED_BYTES - total_bytes - incoming_bytes,
                )
            })
    }
}

fn read_sweep_marker(file: &mut File, now: SystemTime) -> Option<SweepMarker> {
    const MAX_MARKER_BYTES: usize = 96;
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut contents = String::new();
    (&mut *file)
        .take(MAX_MARKER_BYTES as u64 + 1)
        .read_to_string(&mut contents)
        .ok()?;
    if contents.len() > MAX_MARKER_BYTES {
        return None;
    }
    let mut parts = contents.split(':');
    let state = parts.next()?;
    let timestamp = parts.next()?;
    let state = match state {
        "pending" => SweepMarkerState::Pending,
        "success" => SweepMarkerState::Success,
        "failed" => SweepMarkerState::Failed,
        _ => return None,
    };
    let millis = timestamp.parse::<u64>().ok()?;
    let marked_at = UNIX_EPOCH.checked_add(Duration::from_millis(millis))?;
    let remaining_count = parts.next()?.parse::<usize>().ok()?;
    let remaining_bytes = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let fresh = now
        .duration_since(marked_at)
        .is_ok_and(|age| age < SWEEP_LEASE_INTERVAL);
    Some(SweepMarker {
        state,
        marked_at,
        fresh,
        remaining_count,
        remaining_bytes,
    })
}

fn write_sweep_marker_with_capacity(
    file: &mut File,
    state: SweepMarkerState,
    now: SystemTime,
    remaining_count: usize,
    remaining_bytes: u64,
) -> bool {
    let timestamp = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    file.set_len(0)
        .and_then(|()| file.seek(SeekFrom::Start(0)).map(|_| ()))
        .and_then(|()| {
            write!(
                file,
                "{}:{timestamp}:{remaining_count}:{remaining_bytes}",
                state.as_str()
            )
        })
        .and_then(|()| file.flush())
        .is_ok()
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
    use std::fs::{self, OpenOptions};
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime};

    use tempfile::tempdir;

    use crate::history::identity::ExecutionIdentity;
    use crate::history::schema::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent};

    use super::{
        write_sweep_marker_with_capacity, HistoryStore, SweepMarkerState, DECISION_FILE,
        MAX_RETAINED_BYTES, MAX_RETAINED_EXECUTIONS, STARTED_FILE, SWEEP_LEASE_FILE, TERMINAL_FILE,
    };

    fn event(
        identity: &ExecutionIdentity,
        event: ExecutionEvent,
        status: ExecutionStatus,
    ) -> HistoryEvent {
        HistoryEvent::new(identity, ExecutionKind::Worker, event, status)
    }

    fn read_marker(store: &HistoryStore, now: SystemTime) -> Option<(SweepMarkerState, bool)> {
        let mut file = fs::File::open(store.root_dir().join(SWEEP_LEASE_FILE)).ok()?;
        super::read_sweep_marker(&mut file, now).map(|marker| (marker.state, marker.fresh))
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
    fn history_store_successful_sweep_authorizes_write_and_cooldown_reuses_result() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        let marker_path = store.root_dir().join(SWEEP_LEASE_FILE);
        let first = ExecutionIdentity::root();
        assert!(store.write_started(&event(
            &first,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        let first_marker = fs::read_to_string(&marker_path).unwrap();
        assert_eq!(
            read_marker(&store, SystemTime::now()),
            Some((SweepMarkerState::Success, true))
        );

        let second = ExecutionIdentity::root();
        assert!(store.write_started(&event(
            &second,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        let second_marker = fs::read_to_string(marker_path).unwrap();
        let first_parts = first_marker.split(':').collect::<Vec<_>>();
        let second_parts = second_marker.split(':').collect::<Vec<_>>();
        assert_eq!(first_parts[0], second_parts[0]);
        assert_eq!(first_parts[1], second_parts[1]);
        assert_eq!(
            first_parts[2].parse::<usize>().unwrap(),
            second_parts[2].parse::<usize>().unwrap() + 1
        );
        assert!(first_parts[3].parse::<u64>().unwrap() > second_parts[3].parse::<u64>().unwrap());
    }

    #[test]
    fn history_store_fresh_success_marker_rechecks_changed_incoming_capacity() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        let now = SystemTime::now();
        let mut lease = store.try_lock_sweep_lease().unwrap();
        assert!(write_sweep_marker_with_capacity(
            &mut lease,
            SweepMarkerState::Success,
            now,
            0,
            0,
        ));
        drop(lease);

        assert!(!store.sweep_before_write_with_capacity_at(
            now + Duration::from_millis(1),
            MAX_RETAINED_EXECUTIONS + 1,
            MAX_RETAINED_BYTES + 1,
        ));
        assert_eq!(
            read_marker(&store, now + Duration::from_millis(1)).map(|(state, _)| state),
            Some(SweepMarkerState::Failed)
        );
        let identity = ExecutionIdentity::root();
        assert!(!store
            .root_dir()
            .join(identity.execution_id().to_string())
            .exists());
    }

    #[test]
    fn history_store_failed_sweep_marks_failed_and_skips_history_write() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        let now = SystemTime::now();

        assert!(!store.sweep_before_write_with_capacity_at(now, MAX_RETAINED_EXECUTIONS + 1, 0,));
        assert_eq!(
            read_marker(&store, SystemTime::now()),
            Some((SweepMarkerState::Failed, true))
        );

        let identity = ExecutionIdentity::root();
        let execution_dir = store.root_dir().join(identity.execution_id().to_string());
        assert!(!store.write_started(&event(
            &identity,
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
        assert!(!execution_dir.exists());

        let decision_identity = ExecutionIdentity::root();
        let decision_dir = store
            .root_dir()
            .join(decision_identity.execution_id().to_string());
        assert!(!store.write_decision(&event(
            &decision_identity,
            ExecutionEvent::Decision,
            ExecutionStatus::Skipped,
        )));
        assert!(!decision_dir.exists());
    }

    #[test]
    fn history_store_expired_failed_sweep_state_retries_and_can_succeed() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        let now = SystemTime::now();
        let marker_path = store.root_dir().join(SWEEP_LEASE_FILE);
        let mut marker = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&marker_path)
            .unwrap();
        assert!(write_sweep_marker_with_capacity(
            &mut marker,
            SweepMarkerState::Failed,
            now,
            0,
            0,
        ));
        drop(marker);

        assert!(store.sweep_before_write_with_capacity_at(now + Duration::from_secs(3), 1, 0));
        assert_eq!(
            read_marker(&store, SystemTime::now()).map(|(state, _)| state),
            Some(SweepMarkerState::Success)
        );
    }

    #[test]
    fn history_store_contention_skips_for_all_marker_states() {
        for state in [
            SweepMarkerState::Success,
            SweepMarkerState::Pending,
            SweepMarkerState::Failed,
        ] {
            let temp = tempdir().unwrap();
            let store = HistoryStore::new(temp.path());
            fs::create_dir_all(store.root_dir()).unwrap();
            let marker_path = store.root_dir().join(SWEEP_LEASE_FILE);
            let mut marker = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&marker_path)
                .unwrap();
            let now = SystemTime::now();
            assert!(write_sweep_marker_with_capacity(
                &mut marker,
                state,
                now,
                0,
                0
            ));
            let lock = store.try_lock_sweep_lease().unwrap();

            assert!(
                !store.sweep_before_write_with_capacity_at(now + Duration::from_secs(1), 1, 0),
                "contention admission for {state:?} marker"
            );
            drop(lock);
        }
    }

    #[test]
    fn history_store_unknown_oversized_or_overflowing_marker_does_not_authorize_contention() {
        for marker_contents in ["success:18446744073709551615".to_owned(), "x".repeat(65)] {
            let temp = tempdir().unwrap();
            let store = HistoryStore::new(temp.path());
            fs::create_dir_all(store.root_dir()).unwrap();
            let marker_path = store.root_dir().join(SWEEP_LEASE_FILE);
            fs::write(&marker_path, marker_contents).unwrap();
            let lock = store.try_lock_sweep_lease().unwrap();
            let identity = ExecutionIdentity::root();
            let execution_dir = store.root_dir().join(identity.execution_id().to_string());

            assert!(!store.write_started(&event(
                &identity,
                ExecutionEvent::Started,
                ExecutionStatus::Running,
            )));
            assert!(!execution_dir.exists());
            drop(lock);
            assert!(store.write_started(&event(
                &identity,
                ExecutionEvent::Started,
                ExecutionStatus::Running,
            )));
            assert!(execution_dir.exists());
        }
    }

    #[test]
    fn history_store_cross_process_lease_is_released_when_holder_exits() {
        const ROOT_ENV: &str = "CHEAPER_HISTORY_LEASE_CHILD_ROOT";
        const READY_ENV: &str = "CHEAPER_HISTORY_LEASE_CHILD_READY";
        const RELEASE_ENV: &str = "CHEAPER_HISTORY_LEASE_CHILD_RELEASE";

        if let Some(root) = std::env::var_os(ROOT_ENV) {
            let store = HistoryStore::new(root);
            let Some(mut lease) = store.try_lock_sweep_lease() else {
                std::process::exit(10);
            };
            if !write_sweep_marker_with_capacity(
                &mut lease,
                SweepMarkerState::Pending,
                SystemTime::now(),
                0,
                0,
            ) {
                std::process::exit(12);
            }
            let ready = std::env::var_os(READY_ENV).expect("child ready path");
            let release = std::env::var_os(RELEASE_ENV).expect("child release path");
            fs::write(ready, b"locked").expect("write child ready marker");
            let deadline = Instant::now() + Duration::from_secs(2);
            while !Path::new(&release).exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            // Deliberately bypass Rust drops; the OS must release the file lock.
            std::process::exit(if Path::new(&release).exists() { 0 } else { 11 });
        }

        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        let ready = temp.path().join("lease-child-ready");
        let release = temp.path().join("lease-child-release");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("history::store::tests::history_store_cross_process_lease_is_released_when_holder_exits")
            .arg("--nocapture")
            .env(ROOT_ENV, temp.path())
            .env(READY_ENV, &ready)
            .env(RELEASE_ENV, &release)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let ready_deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() && Instant::now() < ready_deadline {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("lease child exited before acquiring lock: {status}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !ready.exists() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("lease child did not acquire lock before timeout");
        }

        let now = SystemTime::now();
        assert!(!store.sweep_before_write_with_capacity_at(now + Duration::from_secs(1), 1, 0,));
        fs::write(&release, b"exit").unwrap();

        let exit_deadline = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= exit_deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("lease child did not exit before timeout");
            }
            thread::sleep(Duration::from_millis(10));
        };
        assert!(status.success());
        assert!(store.sweep_before_write_with_capacity_at(now + Duration::from_secs(3), 1, 0,));
    }

    #[test]
    fn history_store_skips_write_when_sweep_lease_is_unavailable() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());
        fs::create_dir_all(store.root_dir()).unwrap();
        fs::create_dir(store.root_dir().join(SWEEP_LEASE_FILE)).unwrap();

        assert!(!store.write_started(&event(
            &ExecutionIdentity::root(),
            ExecutionEvent::Started,
            ExecutionStatus::Running,
        )));
    }

    #[test]
    fn history_store_read_does_not_create_a_missing_event_directory() {
        let temp = tempdir().unwrap();
        let store = HistoryStore::new(temp.path());

        assert!(store
            .read_event(ExecutionIdentity::root().execution_id(), STARTED_FILE)
            .is_none());
        assert!(!store.root_dir().exists());
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
    fn history_store_concurrent_publications_are_independent_when_admitted() {
        let temp = tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let workers = (0..8)
            .map(|_| {
                let root = root.clone();
                thread::spawn(move || {
                    let store = HistoryStore::new(root);
                    let identity = ExecutionIdentity::root();
                    let published = store.write_started(&event(
                        &identity,
                        ExecutionEvent::Started,
                        ExecutionStatus::Running,
                    ));
                    (identity.execution_id(), published)
                })
            })
            .collect::<Vec<_>>();
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        let store = HistoryStore::new(root);
        let mut ids = std::collections::HashSet::new();
        let mut published_count = 0;

        for (id, published) in outcomes {
            assert!(ids.insert(id), "concurrent identities must remain unique");
            published_count += usize::from(published);
            assert_eq!(
                store.read_event(id, STARTED_FILE).is_some(),
                published,
                "a skipped best-effort publication must not leave a partial event"
            );
        }
        assert!(published_count > 0, "at least one writer must be admitted");
    }
}

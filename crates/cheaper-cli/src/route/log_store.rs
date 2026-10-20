//! `route::log_store` — `<userConfigDir>/route-logs/` への routed
//! 実行 log 保存(design §route log store、spec R28-R31)。
//!
//! - 命名: `<runId>.log`。runId は `<unix-ms>-<pid>-<hex>` の run
//!   単位 unique id で厳格 `^[0-9]+-[0-9]+-[0-9a-f]+$`。実行中は
//!   `<runId>.log.tmp` へ streaming write、完了時に rename(途中
//!   file は最終名で現れない — `fs_util::atomic_write` と同じ
//!   temp+rename 意味論の streaming 版)。
//! - unix `0600` — command output は secret を含みうるため
//!   credentials.json と同じ保護水準。
//! - retention sweep(`route run` 起動時・best-effort):
//!   `*.tmp`(実行中 log)を除外し (a) TTL 30 日、(b) 最大
//!   200 file、(c) 合計 256 MiB を古い順削除で強制。
//! - read は `RouteLogReader`(`ArtifactReader` impl)— `<id>` →
//!   `<route-logs>/<id>.log` の変換は runId regex 検証を必ず通し、
//!   runId-addressed scoped read が arbitrary `*.log` read へ
//!   拡大しない(R31/C10)。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_tools::artifact::{
    ArtifactReadRequest, ArtifactReadResult, ArtifactReader, ArtifactSource,
};
use cheaper_tools::ToolError;
use serde_json::{json, Map};

/// runId の厳格形式 `^[0-9]+-[0-9]+-[0-9a-f]+$` — `<unix-ms>-<pid>-<hex>`。
/// `route logs <id>`・`RouteLogReader` の両方がこの検証を共有する。
pub fn is_valid_run_id(id: &str) -> bool {
    let mut parts = id.split('-');
    let (Some(ms), Some(pid), Some(hex), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    !ms.is_empty()
        && ms.bytes().all(|b| b.is_ascii_digit())
        && !pid.is_empty()
        && pid.bytes().all(|b| b.is_ascii_digit())
        && !hex.is_empty()
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// retention 上限(design default 確定値)。
const TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// `route-logs/` store。`dir` 注入で test 可能な純粋 fs 操作。
#[derive(Debug, Clone)]
pub struct RouteLogStore {
    dir: PathBuf,
}

impl RouteLogStore {
    /// canonical 配置 — `<userConfigDir>/route-logs/`(secret を
    /// 含みうるため workspace 外)。
    pub fn new(user_config_dir: &Path) -> Self {
        Self {
            dir: user_config_dir.join("route-logs"),
        }
    }

    /// test・直接注入用。
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `<runId>.log` の file path — runId 検証は caller 責務
    /// (`read_log`/`RouteLogReader` が `is_valid_run_id` を通す)。
    fn log_path(&self, run_id: &str) -> PathBuf {
        self.dir.join(format!("{run_id}.log"))
    }

    /// runId 採番 + `<runId>.log.tmp` streaming writer を開く。
    /// unix `0600` で作成するため write 中の log も owner-only。
    pub fn begin(&self) -> Result<RouteLogWriter, CheaperError> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| store_io_error("route-logs dir の作成に失敗しました", &self.dir, e))?;
        let run_id = new_run_id();
        let tmp = self.dir.join(format!("{run_id}.log.tmp"));
        let final_path = self.log_path(&run_id);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(&tmp)
            .map_err(|e| store_io_error("route log の作成に失敗しました", &tmp, e))?;
        Ok(RouteLogWriter {
            file,
            tmp,
            final_path,
            run_id,
        })
    }

    /// retention sweep — `*.tmp` 除外で (a) TTL・(b) count・
    /// (c) size を古い順に削除。best-effort: 個別 entry の stat/
    /// remove 失敗は skip し、dir 走査自体の失敗のみ error。
    pub fn sweep(&self) -> io::Result<usize> {
        self.sweep_with(TTL, MAX_FILES, MAX_BYTES)
    }

    /// 上限を注入する sweep — test 用 seam。`sweep` は定数を渡す
    /// 薄い wrapper。
    fn sweep_with(&self, ttl: Duration, max_files: usize, max_bytes: u64) -> io::Result<usize> {
        let mut files: Vec<(u64, u64, PathBuf)> = Vec::new(); // (mtime secs, bytes, path)
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if is_absent(&e) => return Ok(0),
            Err(e) => return Err(e),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "tmp") {
                continue; // 実行中 log は sweep 対象外。
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push((mtime, meta.len(), path));
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let ttl_secs = ttl.as_secs();
        let ttl_removed = files
            .iter()
            .filter(|(mtime, _, _)| now.saturating_sub(*mtime) > ttl_secs)
            .count();
        // (a) TTL 超過を先に落とす。
        files.retain(|(mtime, _, path)| {
            if now.saturating_sub(*mtime) > ttl_secs {
                let _ = fs::remove_file(path);
                false
            } else {
                true
            }
        });
        // 古い順にソート — (b)(c) は古い方から削除する。
        files.sort_by_key(|(mtime, _, _)| *mtime);
        let mut removed = ttl_removed;
        // (b) count 上限。
        while files.len() > max_files {
            let (_, _, path) = files.remove(0);
            let _ = fs::remove_file(&path);
            removed += 1;
        }
        // (c) 合計 size 上限。
        let mut total: u64 = files.iter().map(|(_, bytes, _)| *bytes).sum();
        while total > max_bytes && !files.is_empty() {
            let (_, bytes, path) = files.remove(0);
            let _ = fs::remove_file(&path);
            total = total.saturating_sub(bytes);
            removed += 1;
        }
        Ok(removed)
    }

    /// `route logs <id>` の bounded read — runId 検証 +
    /// `{data, offset, nextOffset, eof}` の ArtifactReader contract。
    /// offset は byte 基準 0-indexed で EOF と同値まで許可する。
    pub fn read_log(
        &self,
        run_id: &str,
        offset: u64,
        max_bytes: u64,
    ) -> Result<ArtifactReadResult, ToolError> {
        read_bounded(&self.dir, run_id, offset, max_bytes)
    }

    /// `ArtifactPorts.reader` 注入用の `ArtifactReader`。
    pub fn reader(&self) -> RouteLogReader {
        RouteLogReader {
            dir: self.dir.clone(),
        }
    }
}

/// `<unix-ms>-<pid>-<hex>` の runId 採番 — `fs_util::temp_path` の
/// pid+nanos 規約と同型の衝突耐性(nanos 同一時は counter で分離)。
pub(crate) fn new_run_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{}-{:x}",
        now.as_millis(),
        std::process::id(),
        now.as_nanos() ^ ((seq as u128) << 32)
    )
}

fn is_absent(e: &io::Error) -> bool {
    cheaper_core::errors::is_absent_file_error(e)
}

/// `<runId>.log.tmp` streaming writer。`finish` で rename して初めて
/// 最終名が現れる。
pub struct RouteLogWriter {
    file: File,
    tmp: PathBuf,
    final_path: PathBuf,
    run_id: String,
}

impl RouteLogWriter {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn tmp_path(&self) -> &Path {
        &self.tmp
    }

    /// tmp → `<runId>.log` へ rename(flush 後に移す)。rename 先が
    /// Windows で置換可能なのは `MoveFileExW`+REPLACE 相当 — rename
    /// 対象は存在しない(`create_new` の tmp が唯一 writer)。
    pub fn finish(mut self) -> io::Result<PathBuf> {
        self.file.flush()?;
        fs::rename(&self.tmp, &self.final_path)?;
        Ok(self.final_path)
    }
}

impl Write for RouteLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// `ArtifactPorts.reader` 用 — artifact ref の `run_id` を runId
/// regex で検証してから `<route-logs>/<runId>.log` を bounded
/// read する(runId-addressed scoped read。生 file path 経路は
/// 提供しない)。
pub struct RouteLogReader {
    dir: PathBuf,
}

impl ArtifactReader for RouteLogReader {
    fn read(
        &self,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError> {
        read_bounded(
            &self.dir,
            &source.reference.run_id,
            request.offset,
            request.max_bytes,
        )
    }
}

/// 共有 bounded read — runId 検証 → open → seek → read。
fn read_bounded(
    dir: &Path,
    run_id: &str,
    offset: u64,
    max_bytes: u64,
) -> Result<ArtifactReadResult, ToolError> {
    if !is_valid_run_id(run_id) {
        return Err(invalid_run_id(run_id));
    }
    let path = dir.join(format!("{run_id}.log"));
    let mut file = File::open(&path).map_err(|e| -> ToolError {
        if is_absent(&e) {
            Box::new(
                CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("route log not found: {run_id}"),
                )
                .with_details(Map::from_iter([("runId".to_string(), json!(run_id))])),
            )
        } else {
            Box::new(store_io_error("route log の read に失敗しました", &path, e))
        }
    })?;
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if offset > len {
        return Err(Box::new(
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("route log offset beyond EOF: {offset} > {len}"),
            )
            .with_details(Map::from_iter([
                ("runId".to_string(), json!(run_id)),
                ("offset".to_string(), json!(offset)),
            ])),
        ));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| -> ToolError {
            Box::new(store_io_error("route log の seek に失敗しました", &path, e))
        })?;
    let mut buf = vec![0u8; (max_bytes as usize).min((len - offset) as usize)];
    file.read_exact(&mut buf).map_err(|e| -> ToolError {
        Box::new(store_io_error("route log の read に失敗しました", &path, e))
    })?;
    let next_offset = offset + buf.len() as u64;
    Ok(ArtifactReadResult {
        data: buf,
        offset,
        next_offset,
        eof: next_offset >= len,
    })
}

fn invalid_run_id(run_id: &str) -> ToolError {
    Box::new(
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("invalid route run id: {run_id}"),
        )
        .with_details(Map::from_iter([("runId".to_string(), json!(run_id))])),
    )
}

fn store_io_error(message: &str, file: &Path, e: io::Error) -> CheaperError {
    let mut details = Map::new();
    details.insert("file".to_string(), json!(file.display().to_string()));
    details.insert(
        "errno".to_string(),
        json!(cheaper_core::errors::io_errno_display(
            &file.display().to_string(),
            &e
        )),
    );
    CheaperError::new(ErrorCode::InternalError, format!("{message}: {e}")).with_details(details)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, RouteLogStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = RouteLogStore::new(tmp.path());
        (tmp, store)
    }

    #[test]
    fn run_id_format_and_validation() {
        let id = new_run_id();
        assert!(is_valid_run_id(&id), "run_id: {id}");
        for valid in ["1-2-a", "1700000000000-42-deadbeef", "0-0-0"] {
            assert!(is_valid_run_id(valid), "valid: {valid}");
        }
        for invalid in [
            "../x",
            "a/b",
            "a\\b",
            "..",
            "a-1",
            "1-2",
            "1-2-",
            "x-2-a",
            "1-x-a",
            "1-2-A",   // uppercase hex
            "1-2-g",   // non-hex
            "1-2-a-3", // 4 segments
            "-1-2-a",
            "1-2-a.log",
        ] {
            assert!(!is_valid_run_id(invalid), "invalid: {invalid}");
        }
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn begin_writes_tmp_then_finish_renames() {
        let (_tmp, store) = store();
        let mut writer = store.begin().unwrap();
        let run_id = writer.run_id().to_string();
        assert!(is_valid_run_id(&run_id));
        writer.write_all(b"line1\nline2\n").unwrap();
        // 実行中は `.tmp` のみ存在し最終名は現れない。
        assert!(store.dir().join(format!("{run_id}.log.tmp")).exists());
        assert!(!store.dir().join(format!("{run_id}.log")).exists());
        let final_path = writer.finish().unwrap();
        assert_eq!(final_path, store.dir().join(format!("{run_id}.log")));
        assert!(!store.dir().join(format!("{run_id}.log.tmp")).exists());
        #[cfg(unix)]
        assert_eq!(mode(&final_path), 0o600);
        // read_log で内容が取れる。
        let result = store.read_log(&run_id, 0, 1024).unwrap();
        assert_eq!(result.data, b"line1\nline2\n");
        assert!(result.eof);
    }

    #[test]
    fn read_log_is_bounded_and_offset_aware() {
        let (_tmp, store) = store();
        let mut writer = store.begin().unwrap();
        let run_id = writer.run_id().to_string();
        writer.write_all(b"0123456789").unwrap();
        writer.finish().unwrap();
        let page = store.read_log(&run_id, 4, 3).unwrap();
        assert_eq!(page.data, b"456");
        assert_eq!(page.offset, 4);
        assert_eq!(page.next_offset, 7);
        assert!(!page.eof);
        let tail = store.read_log(&run_id, 7, 100).unwrap();
        assert_eq!(tail.data, b"789");
        assert!(tail.eof);
        // EOF と同値の offset は空 result で許可、超過は拒否。
        assert!(store.read_log(&run_id, 10, 8).unwrap().data.is_empty());
        assert!(store.read_log(&run_id, 11, 8).is_err());
        // 不正 runId・absent log。
        assert!(store.read_log("../escape", 0, 8).is_err());
        assert!(store.read_log("1-2-a", 0, 8).is_err());
    }

    #[test]
    fn artifact_reader_shares_run_id_validation() {
        let (_tmp, store) = store();
        let mut writer = store.begin().unwrap();
        let run_id = writer.run_id().to_string();
        writer.write_all(b"payload").unwrap();
        writer.finish().unwrap();
        let reader = store.reader();
        let source = |id: &str| ArtifactSource {
            reference: cheaper_tools::artifact::ArtifactReference {
                artifact: "route-log".to_string(),
                run_id: id.to_string(),
                producer: cheaper_tools::artifact::ArtifactProducer {
                    job_id: "exec".to_string(),
                    name: "routed-exec".to_string(),
                },
                bytes: 7,
                preview: String::new(),
                truncated: false,
            },
        };
        let request = ArtifactReadRequest {
            offset: 0,
            max_bytes: 64,
        };
        let result = reader.read(&source(&run_id), &request).unwrap();
        assert_eq!(result.data, b"payload");
        // traversal id は read を拒否する。
        assert!(reader.read(&source("../../etc/passwd"), &request).is_err());
        assert!(reader.read(&source("1-2-a.log"), &request).is_err());
    }

    #[test]
    fn sweep_enforces_ttl_count_size_and_skips_tmp() {
        let (_tmp, store) = store();
        fs::create_dir_all(store.dir()).unwrap();
        let write_log = |name: &str, bytes: usize| -> PathBuf {
            let path = store.dir().join(name);
            fs::write(&path, vec![b'x'; bytes]).unwrap();
            path
        };
        // in-flight tmp は sweep 対象外。
        let mut writer = store.begin().unwrap();
        let inflight = writer.run_id().to_string();
        writer.write_all(b"inflight").unwrap();
        // (a) TTL: mtime を過去へ戻した file のみ消える。
        let fresh = write_log("fresh.log", 8);
        let old = write_log("old.log", 8);
        let past = SystemTime::now() - Duration::from_secs(3600);
        fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(past)
            .unwrap();
        let removed = store
            .sweep_with(Duration::from_secs(60), usize::MAX, u64::MAX)
            .unwrap();
        assert_eq!(removed, 1);
        assert!(fresh.exists());
        assert!(!old.exists());
        // (b) count: 古い方から max_files を超えた分が消える。
        let a = write_log("a.log", 8);
        let b = write_log("b.log", 8);
        let c = write_log("c.log", 8);
        let removed = store.sweep_with(Duration::MAX, 2, u64::MAX).unwrap();
        assert_eq!(removed, 2); // fresh + a (mtime 昇順で古い 2 件)
        assert!(b.exists() || c.exists());
        // (c) size: 合計が max_bytes を超える分を古い順に削る。
        for path in [&a, &b, &c, &fresh] {
            let _ = fs::remove_file(path);
        }
        let big = write_log("big.log", 128);
        let small = write_log("small.log", 8);
        let removed = store.sweep_with(Duration::MAX, usize::MAX, 100).unwrap();
        assert_eq!(removed, 1); // 古い方(small か big — mtime 順)を削除
        assert!(big.exists() != small.exists());
        // tmp は条件を全て満たしても残る。
        assert!(store.dir().join(format!("{inflight}.log.tmp")).exists());
    }
}

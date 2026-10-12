//! Plugin catalog snapshot file
//! （`<userConfigDir>/providers/<id>/catalog.json`）の envelope と read/write。
//!
//! plugin `refreshModels` の結果（`ModelDefinitionJson` を verbatim）を
//! runtime 所有で永続化し、後続 process の catalog 構築へ注入する
//! （issue #81）。path 組立・I/O・envelope 検証をこの module に閉じ込め、
//! caller（`runtime.rs`・`auth/service.rs`）は `Result` のみを扱う。
//! resolved `CatalogModel` は保存しない — `api`/`baseUrl` fallback・
//! defaults 解決は compose 層の責務。

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::schema::ModelDefinitionJson;

/// envelope schema version。`ModelDefinitionJson` の schema 変更
/// （field 追加・削除）は本 version の bump を必須とする — 旧 version
/// file は `load_plugin_catalog` の検証で無視され、次回 login refresh
/// で再生成される。同一 version 内での field 追加は行わない。
pub const PLUGIN_CATALOG_FILE_VERSION: u32 = 1;

/// `catalog.json` envelope。unknown key は同 version 内の前方互換として
/// 読み飛ばす（serde の default 挙動）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogFile {
    pub version: u32,
    /// provider id — cross-provider の file 取り違え・手動配置の誤適用を
    /// 防ぐため load 時に照合する。
    pub provider: String,
    /// epoch ms。v1 では診断用途のみで TTL 判定に使わない。
    pub fetched_at: i64,
    /// plugin `refresh_models` が返した `ModelDefinitionJson`（verbatim）。
    pub models: Vec<ModelDefinitionJson>,
}

/// snapshot を読む。file 不在 → `Ok(None)`（初回は常に不在で error で
/// ない）。その他の io error・malformed JSON・version 不一致・provider
/// 不一致・`models` 空 → `Err`（file 全体を無視 — caller は bootstrap
/// fallback + `record_error`）。通過時は `Ok(Some(file))`。
pub fn load_plugin_catalog(
    path: &Path,
    provider_id: &str,
) -> Result<Option<PluginCatalogFile>, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Failed to read plugin catalog: {e}\n\nFile: {}",
                path.display()
            ));
        }
    };
    let file: PluginCatalogFile = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse plugin catalog: {e}\n\nFile: {}",
            path.display()
        )
    })?;
    if file.version != PLUGIN_CATALOG_FILE_VERSION {
        return Err(format!(
            "Unsupported plugin catalog version {} (expected {})\n\nFile: {}",
            file.version,
            PLUGIN_CATALOG_FILE_VERSION,
            path.display()
        ));
    }
    if file.provider != provider_id {
        return Err(format!(
            "Plugin catalog provider mismatch: expected \"{provider_id}\", found \"{}\"\n\nFile: {}",
            file.provider,
            path.display()
        ));
    }
    if file.models.is_empty() {
        return Err(format!(
            "Plugin catalog has an empty models list\n\nFile: {}",
            path.display()
        ));
    }
    Ok(Some(file))
}

/// temp+rename atomic write。`create_dir_all` → 同 dir の temp file
/// （`.<file name>.tmp-<pid>-<nanos>`）へ write → `std::fs::rename` で
/// 置換する。**remove-first はしない** — `std::fs::rename` は Windows
/// でも `MoveFileExW` + `MOVEFILE_REPLACE_EXISTING` semantics で既存
/// target を置き換える。`CredentialStore::write_file` の
/// `#[cfg(windows)]` 事前 remove は継承 workaround であり複製しない。
/// 失敗時は temp file を best-effort で remove して `Err` を返す
/// （caller は `record_error`）。
pub fn write_plugin_catalog(path: &Path, file: &PluginCatalogFile) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| {
        format!(
            "Failed to create plugin catalog directory: {e}\n\nFile: {}",
            path.display()
        )
    })?;
    let body = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Failed to serialize plugin catalog: {e}"))?;
    crate::fs_util::atomic_write(
        path,
        &body,
        &crate::fs_util::AtomicWriteOptions {
            windows_remove_first: false,
            #[cfg(unix)]
            unix_mode: None,
        },
    )
    .map_err(|e| {
        format!(
            "Failed to write plugin catalog: {e}\n\nFile: {}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_def(id: &str) -> ModelDefinitionJson {
        serde_json::from_str(&format!(
            r#"{{"id": "{id}", "name": "Model {id}", "api": "anthropic-messages"}}"#
        ))
        .unwrap()
    }

    fn sample_file(models: Vec<ModelDefinitionJson>) -> PluginCatalogFile {
        PluginCatalogFile {
            version: PLUGIN_CATALOG_FILE_VERSION,
            provider: "fake".to_string(),
            fetched_at: 1_760_000_000_000,
            models,
        }
    }

    #[test]
    fn write_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("providers")
            .join("fake")
            .join("catalog.json");
        let file = sample_file(vec![sample_def("m1"), sample_def("m2")]);
        write_plugin_catalog(&path, &file).unwrap();
        let loaded = load_plugin_catalog(&path, "fake").unwrap().unwrap();
        assert_eq!(loaded, file);
        // envelope は camelCase で書かれる（`fetchedAt`）。
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"version\": 1"), "{body}");
        assert!(body.contains("\"fetchedAt\": 1760000000000"), "{body}");
        // None field は省略される（models.json convention）。
        assert!(!body.contains("baseUrl"), "{body}");
    }

    #[test]
    fn load_returns_none_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        assert_eq!(load_plugin_catalog(&path, "fake").unwrap(), None);
    }

    #[test]
    fn malformed_json_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load_plugin_catalog(&path, "fake").is_err());
    }

    #[test]
    fn version_mismatch_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        let mut file = sample_file(vec![sample_def("m1")]);
        file.version = PLUGIN_CATALOG_FILE_VERSION + 1;
        std::fs::write(&path, serde_json::to_string(&file).unwrap()).unwrap();
        let err = load_plugin_catalog(&path, "fake").unwrap_err();
        assert!(err.contains("version"), "{err}");
    }

    #[test]
    fn provider_mismatch_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        write_plugin_catalog(&path, &sample_file(vec![sample_def("m1")])).unwrap();
        let err = load_plugin_catalog(&path, "other").unwrap_err();
        assert!(err.contains("provider"), "{err}");
    }

    #[test]
    fn empty_models_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        write_plugin_catalog(&path, &sample_file(vec![])).unwrap();
        assert!(load_plugin_catalog(&path, "fake").is_err());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        let mut value: serde_json::Value =
            serde_json::to_value(sample_file(vec![sample_def("m1")])).unwrap();
        value["futureKey"] = serde_json::json!({"nested": true});
        std::fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
        let loaded = load_plugin_catalog(&path, "fake").unwrap().unwrap();
        assert_eq!(loaded.models.len(), 1);
    }

    #[test]
    fn write_replaces_existing_file_without_remove_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.json");
        write_plugin_catalog(&path, &sample_file(vec![sample_def("old")])).unwrap();
        let updated = sample_file(vec![sample_def("new")]);
        write_plugin_catalog(&path, &updated).unwrap();
        let loaded = load_plugin_catalog(&path, "fake").unwrap().unwrap();
        assert_eq!(loaded, updated);
        // temp file が残らない。
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
    }
}

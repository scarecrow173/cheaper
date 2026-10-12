//! `CHEAPER_TEST_FAKE_STREAM` seam(TS `cli.ts::loadTestStreamFn` 相当)。
//!
//! TS 版は module を動的 import して `default export` を streamFn に
//! 変換するが、Rust 版は **fixture JSON path** 形式を取る:
//! `CHEAPER_TEST_FAKE_STREAM` が指す JSON file を
//! [`cheaper_runtime::dev::fixture::RunFixture`] として load し、
//! fake provider / scripted session を組み立てる素材として run 経路
//! (T3)へ渡す。
//!
//! activation 規則(TS と同一):
//! - `NODE_ENV === "test"` でないと無効
//! - env 未設定・空文字なら無効
//!
//! fixture load 失敗は INVALID_REQUEST として呼び出し側へ伝播する
//! (TS で module import / shape 検証が throw する経路に相当)。

use cheaper_core::config::dirs::EnvLookup;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::dev::fixture::RunFixture;
use serde_json::{json, Map};
use std::path::Path;

/// `loadTestStreamFn` 相当。有効なら load 済み fixture を返す。
/// `None` = fake stream 無効(通常経路)。
pub fn load_test_stream(env: EnvLookup<'_>) -> Result<Option<RunFixture>, CheaperError> {
    if env("NODE_ENV").as_deref() != Some("test") {
        return Ok(None);
    }
    let spec = env("CHEAPER_TEST_FAKE_STREAM").unwrap_or_default();
    if spec.is_empty() {
        return Ok(None);
    }
    let fixture = RunFixture::load(Path::new(&spec)).map_err(|e| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!(
                "CHEAPER_TEST_FAKE_STREAM fixture load failed: {}",
                e.message()
            ),
        )
        .with_details(Map::from_iter([("spec".to_string(), json!(spec))]))
        .with_cause(e)
    })?;
    Ok(Some(fixture))
}

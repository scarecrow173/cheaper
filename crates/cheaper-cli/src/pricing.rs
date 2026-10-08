//! `loadPricingSnapshot`(`packages/cli/src/pricing.ts`)の port。
//!
//! `<userConfigDir>/pricing.json` を user 提供の pricing snapshot として
//! fail-soft に読む。形式は core の bundled snapshot(`PricingSnapshot`:
//! `{version, updatedAt, models: {"<provider>/<id>": {input, output, ...}}}`)
//! と同一で、core `resolve_pricing` の snapshot 引数にそのまま渡せる
//! bridge(ADR-034rzI0Yqw3L8AFM9diZct の snapshot contract に整合)。
//! file 不在・parse 失敗・shape 不正は `None`。entry 単位の不正はその
//! entry のみ skip する。

use std::path::Path;

use cheaper_core::model::pricing::{PricingSnapshot, PricingSnapshotEntry};
use serde_json::Value;

pub const PRICING_FILENAME: &str = "pricing.json";

/// `isNonNegativeNumber`: `typeof === "number" && Number.isFinite(v)
/// && v >= 0`。JSON parse 由来の number は常に有限だが、`Value::from`
/// 経由の非有限を塞ぐため is_finite も見る。
fn non_negative_number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite() && *n >= 0.0)
}

/// `loadPricingSnapshot`。`PricingSnapshot` の `version`/`updated_at` は
/// core の bundled const 向けに `&'static str` で宣言されているため、
/// file 由来の文字列は `Box::leak` で bridge する(process 寿命の
/// 読み取り専用データ;golden test helper と同じ手法)。
pub fn load_pricing_snapshot(user_config_dir: &Path) -> Option<PricingSnapshot> {
    let raw = std::fs::read_to_string(user_config_dir.join(PRICING_FILENAME)).ok()?;
    let parsed: Value = serde_json::from_str(&raw).ok()?;
    let obj = parsed.as_object()?;
    let version = obj.get("version")?.as_str()?;
    let updated_at = obj.get("updatedAt")?.as_str()?;
    let models = obj.get("models")?.as_object()?;

    // `Object.entries` 順 = preserve_order の挿入順を維持する
    // (`resolve_pricing` の first-match 走査が insertion order に依存)。
    let entries: Vec<(String, PricingSnapshotEntry)> = models
        .iter()
        .filter_map(|(key, e)| {
            let e = e.as_object()?;
            let input = non_negative_number(e.get("input")?)?;
            let output = non_negative_number(e.get("output")?)?;
            Some((
                key.clone(),
                PricingSnapshotEntry {
                    input,
                    output,
                    cache_read: e.get("cacheRead").and_then(non_negative_number),
                    cache_write: e.get("cacheWrite").and_then(non_negative_number),
                    updated_at: e.get("updatedAt").and_then(Value::as_str).map(String::from),
                },
            ))
        })
        .collect();
    Some(PricingSnapshot {
        version: Box::leak(version.to_string().into_boxed_str()),
        updated_at: Box::leak(updated_at.to_string().into_boxed_str()),
        models: entries,
    })
}

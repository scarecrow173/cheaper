---
id: "DESIGN-034CM9o41G2FjaP5cV125c"
type: "design"
status: "approved"
title: "Plugin catalog snapshot persistence and user models merge"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/plugin-catalog-persistence-and-user-models.md"
    - "../adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
  implemented-by:
    - "../plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  depends-on:
    - "./provider-only-registration-layer.md"
    - "./devin-local-provider-plugin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./pi-state-paths-under-userconfigdir-pi.md"
    - "./auth-command.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/plugin-catalog-persistence-and-user-models.md"
    - "../adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
    - "../discovery/plugin-catalog-refresh-persistence-and-user-models.md"
  derived-by:
    - "../plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
    - "../test-specs/plugin-catalog-persistence-and-user-model-merge.md"
  verifies: []
  verified-by: []
  references:
    - "../impl/ir/phase-3-port-runtime-adapter-providers.md"
    - "../tasks/phase-3-port-runtime-adapter-providers.md"
  defers: []
  deferred-by: []
---

# Plugin catalog snapshot persistence and user models merge

## Context

SPEC-034CKp34AcjKtAYjnfjXU4 / ADR-034CKnwOWyYssJ1VZR9spo（issue #81・#82）
の実装設計。upstream の決定（Option A + γ + 3A）を再検討せず、そのまま
コード構造へ落とす。

- **Defect 1**: `AuthService::login` 成功後の `refresh_plugin_catalog`
  （`crates/cheaper-runtime/src/auth/service.rs:464`）が `devin models
  list` の結果を `CatalogStore::refresh_plugin_models` →
  `dynamic_models`（in-memory `RwLock`、`catalog/mod.rs:780`）にのみ
  保持する。`cheaper` は command ごとの one-shot process のため別
  process の `run`/`models` には bootstrap `fallback_models()` 4 件
  しか届かない。
- **Defect 2**: `compose_provider`（`catalog/mod.rs:591`）で
  `apply_extension` が `plugin.models` 非空なら model list を丸ごと
  replace し、user の `providers.<id>.models[]` を silent に破棄する。
  さらに plugin provider は `base_models` が空で、`api`/`baseUrl` を
  欠く user def は `model_from_json` error → compose error で provider
  ごと drop する。
- **surfacing**: `CatalogStore::errors`/`record_error`（`catalog/mod.rs:867`・
  `876`）は CLI 表面に露出しておらず、non-builtin provider の drop が
  `unknown provider` にしか見えない。

## Scope

### In Scope

- `<userConfigDir>/providers/<id>/catalog.json` への runtime-owned
  snapshot 永続化（write = `refresh_plugin_catalog`、read =
  `Runtime::create`）。
- plugin provider における user `models[]` の additive merge（compose・
  refresh 両経路で同一規則）と `api`/`baseUrl` fallback chain。
- `model_header_sources` の refresh 時再構築。
- `CatalogStore::errors()` の CLI stderr surfacing（`models`・`run`・
  `auth status`・`auth login`、`--quiet` で抑制）。

### Out of Scope

- `run`/`models`/runtime 起動時の lazy/TTL refresh（refresh 契機は
  login のみ。`fetchedAt` は診断用に保持）。
- logout・credential 無効化時の snapshot 削除（stale snapshot は次回
  login refresh で自己修復）。
- pi file（`<userConfigDir>/pi/`、`models-store.json`）の読み書き。
- `CredentialStore::write_file` の `#[cfg(windows)]` 事前 remove
  workaround の是正（本 writer へ複製しないのみ。修正は別 chain）。
- Windows devin-CLI child env 隔離（issue #80、別 chain）。
- envelope `warnings` field 等の構造化 surface（別 chain へ defer）。
- `cheaper workflow run` 等、spec が列挙しない command への warning
  追加（`create_runtime` を共有するが emit 責務は command 層に置く）。

## Data Format

### `<userConfigDir>/providers/<providerId>/catalog.json`

```json
{
  "version": 1,
  "provider": "devin",
  "fetchedAt": 1760000000000,
  "models": [ { "id": "swe-2", "name": "SWE-2", ... } ]
}
```

- `models[]` の要素は `ModelDefinitionJson`（plugin `refresh_models`
  の返却形）を verbatim 保存する。resolved `CatalogModel` は保存
  しない（`api`/`base_url` fallback・defaults 解決は compose 層の
  責務 — bootstrap/persisted/refresh の 3 経路で同一の
  `extension_models` 変換を通すための不変条件）。
- `fetchedAt` は epoch ms。v1 では診断用途のみで TTL 判定に使わない。
- unknown key は同 version 内の前方互換として読み飛ばす。
- `ModelDefinitionJson` の schema 変更（field 追加・削除）は envelope
  `version` bump を必須とし、旧 version file は検証で無視され次回
  login refresh で再生成される。同一 version 内での field 追加は
  行わない。

## Components and Boundaries

### `crates/cheaper-runtime/src/catalog/persisted.rs`（新設）

snapshot file の envelope 型と read/write を所有する module
（`catalog` module 配下。`pub mod persisted`）。path 組立・I/O・
envelope 検証をここに閉じ込め、caller（`runtime.rs`・`service.rs`）は
`Result` のみを扱う。

```rust
pub const PLUGIN_CATALOG_FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogFile {
    pub version: u32,
    pub provider: String,
    pub fetched_at: i64,
    pub models: Vec<ModelDefinitionJson>,
}

/// file 不在 → Ok(None)。検証失敗 → Err(msg)（caller が record_error）。
pub fn load_plugin_catalog(path: &Path, provider_id: &str)
    -> Result<Option<Vec<ModelDefinitionJson>>, String>;

/// temp+rename atomic write。失敗 → Err(msg)（caller が record_error）。
pub fn write_plugin_catalog(
    path: &Path, provider_id: &str,
    models: &[ModelDefinitionJson], fetched_at_ms: i64,
) -> Result<(), String>;
```

`load_plugin_catalog` の検証（全て file 全体を無視し Err を返す —
caller は bootstrap fallback + `record_error`）:

1. `read_to_string` が `NotFound` → `Ok(None)`（silent。初回は常に
   不在で error ではない）。その他の io error → Err。
2. JSON parse / deserialize 失敗 → Err（malformed / schema error）。
3. `version != PLUGIN_CATALOG_FILE_VERSION` → Err。
4. `provider != provider_id` → Err（cross-provider の file 取り違え・
   手動配置の誤適用を防ぐ）。
5. `models` が空 → Err（write 側 guard と対称 — 空 file は自己矛盾）。
6. 通過 → `Ok(Some(models))`。

`write_plugin_catalog`:

1. `fs::create_dir_all(path.parent())`。
2. `PluginCatalogFile { version: 1, provider, fetched_at, models }` を
   `serde_json::to_string_pretty`。
3. 同 dir の temp file（`.catalog.json.tmp-<pid>-<nanos>`、
   `CredentialStore::write_file` の命名 pattern と同型）へ write。
4. `std::fs::rename` で置換。**remove-first はしない** —
   `std::fs::rename` は Windows でも既存 target を置き換える
   （`MoveFileExW` + `MOVEFILE_REPLACE_EXISTING` semantics）。
   `CredentialStore::write_file` の `#[cfg(windows)]` 事前 remove
   （`auth/store.rs:278-281`）は継承 workaround であり複製しない。
5. 失敗時は temp file を best-effort で remove して Err。

### `crates/cheaper-runtime/src/catalog/schema.rs`

persist のため `Serialize` derive を追加する（いずれも `mod.rs` ではなく
`catalog/schema.rs` 内の定義）:

- `ModelDefinitionJson`（`schema.rs:436`）
- `ModelCostJson`（`schema.rs:408`）・`ModelCostTierJson`（`schema.rs:397`）
- `ThinkingLevelMap`（`schema.rs:354`）・`ModelInput`（`schema.rs:346`）

`Option` field に `skip_serializing_if = "Option::is_none"` を揃えて
出力を clean に保つ（`ThinkingLevelMap` には既に付いている — Serialize
derive 追加で有効化される）。`ProviderConfigJson`・`ModelOverrideJson`
等の persist しない型には追加しない（YAGNI）。

**accepted limitation — `Option<Option<String>>` round-trip**:
`ThinkingLevelMap` の各 level は `Option<Option<String>>` で、明示 null
（`Some(None)`）と key 不在（`None`）を区別する。serde は deserialize
時に JSON `null` を外側 `None` へ畳むため、serialize→read で
`Some(None)` が `None` へ縮退する（再 serialize では key 自体が落ちる）。
機能差は現時点で無い — 消費側（`resolve_model_uid`・
`preferred_default`・mapped-level 計数、`cheaper-provider-devin/src/
models.rs:156-188,243-250,479`）は `Some(None)` と `None` を同じく
unmapped として扱い、両者を区別するのは map 構築時（`family_to_models`
の `is_none` 判定、`models.rs:213,225`）のみで persist 前に完結する。
また persist guard (c) の等価比較は in-memory defs 同士
（`plugin_bootstrap_defs` vs refresh `definitions`）で行い round-trip
後の snapshot を比較しないため、guard 判定にも影響しない。

### `crates/cheaper-runtime/src/plugin.rs`

`ProviderPluginContext` に path 組立 helper を追加し、`plugin_dir`
規約を一箇所に集約する（表現ドリフト回避 — `runtime.rs`・
`service.rs` の双方が参照する）:

```rust
impl ProviderPluginContext {
    /// plugin catalog snapshot — `<userConfigDir>/providers/<id>/catalog.json`。
    /// runtime 所有（plugin の `plugin_dir` 配下に置くが plugin 自身は
    /// 読み書きしない）。
    pub fn catalog_path(&self, plugin_id: &str) -> PathBuf {
        self.plugin_dir(plugin_id).join("catalog.json")
    }
}
```

### `crates/cheaper-runtime/src/catalog/mod.rs`

#### `CatalogStore::build` — snapshot 注入用の第 4 引数

```rust
pub fn build(
    user_models: Option<ModelsJsonFile>,
    user_error: Option<String>,
    plugin_registrations: &[PluginProviderRegistration],
    persisted_plugin_models: &BTreeMap<String, Vec<ModelDefinitionJson>>,
) -> Result<CatalogStore, CheaperError>
```

provider ごとの loop で `persisted_plugin_models.get(id)` を
`compose_provider` へ渡す。**`registration.models` は差し替えない**
（normative — `refresh_plugin_catalog` の persist guard 比較基準を
注入前 bootstrap に固定するため。注入方式を registration 差し替えに
すると post-snapshot process で基準が snapshot 自身になり、guard (c)
が機能不全になる）。

#### `compose_provider` — plugin provider の順序変更

現行（`mod.rs:591-764`）: `apply_models_json`（provider-level 検証 +
base overlay + per-def `config.models` 解決、`mod.rs:409-473`）→
`apply_extension` → `modelOverrides`。

`apply_models_json` は実質 3 つの責務を持つ:

- (a) provider-level 検証: `config.oauth.is_some() &&
  config.base_url.is_none()` → Err（`mod.rs:414-418`）と「must specify
  baseUrl/headers/compat/modelOverrides/models」検証（`mod.rs:419-437`）。
- (b) base-model overlay: base models への `baseUrl`/`compat` merge
  （`mod.rs:438-452`）。
- (c) per-def `config.models` 解決: `model_from_json` を `?` 伝播する
  loop（`mod.rs:453-471`）— def 1 件の失敗が provider 全体を落とす。

plugin provider（`plugin.is_some()`）では (c) だけを merge stage へ
defer し、(a)・(b) は従来どおり走らせる。(a)・(b) は
`apply_models_json` から共有 helper（`validate_provider_config` /
`apply_provider_overlay` 相当 — 命名は既存 convention に従う）へ
抽出し、`apply_models_json` 内の custom-provider 経路と plugin
provider の compose 経路の双方から呼ぶ。`"baseUrl" is required when
"oauth" is set.` や「must specify …」の error 文字列を経路間で
複製しないための構造（表現ドリフト回避）。`apply_models_json`
自身は helper 呼出 + (c) の loop に再構成される。

plugin provider での検証 semantics は今日と完全に同一（zero
divergence）— `config.oauth.is_some() && config.base_url.is_none()`
の場合のみ Err であり、`{"oauth":"radius","baseUrl":...}` は検証を
通過して `radius_gateway` 判定（`mod.rs:667`・`mod.rs:745-748`）
により今日どおり `RADIUS_API_KEY` env auth となる。この動作は既知
の quirk（plugin が auth を所有する provider に config 宣言 `oauth`
が効く）としてそのまま維持する — 本 chain の scope 外であり、
`config.oauth.is_some()` の無条件 Err 化のような新規 fatal class
は導入しない。「must specify …」検証も plugin provider で**緩めず
維持する** — 実効 no-op の config entry は custom と同じく user
error であり、plugin が api/baseUrl を供給してもその意味は変わら
ない。

plugin provider の順序:

1. **base list**: `base_models.to_vec()` に上記共有 helper の
   (a)・(b) を適用（plugin provider では base_models が実質空のため
   overlay は no-op、検証のみ実効）。per-def `config.models` 解決は走らせない
   — merge stage が扱うため `?` 伝播させると provider drop 経路が
   残る。`plugin_bootstrap_defs`・`plugin_user_defs`・`config_api`・
   `config_base_url` はこの時点で `ProviderEntry` へ保持する。
2. **`apply_extension`**: `persisted` があればそれを、無ければ
   `plugin.models` を extension defs として使う（signature に
   `persisted: Option<&[ModelDefinitionJson]>` を追加し、内部の
   `extension_models` 呼び出しに
   `persisted.unwrap_or(plugin.models.as_slice())` を渡す）。
   `plugin.models` が空でも persisted defs があれば
   `extension_models` が走る。persisted は reader が非空を保証する
   ため、effective defs が空のときのみ従来の baseUrl-override 分岐。
3. **merge stage（新規）**: `merge_user_model_defs` を post-extension
   list に対して走らせる（`plugin.models` 空否に依らず全 plugin
   provider で適用）。
4. **`modelOverrides`**: 最終層（不変）。

```rust
/// plugin provider の user `models[]` additive merge — compose と
/// refresh で共有する唯一の merge 実装（DRY）。post-extension list の
/// id と衝突しない def のみ `model_from_json` で解決して末尾へ追加
/// する（plugin 定義順 → user 追加順の file 順）。衝突・解決不能は
/// その def のみ skip + `errors` へ記録し、provider・plugin models は
/// 残す。
fn merge_user_model_defs(
    provider_id: &str,
    models: &mut Vec<CatalogModel>,
    user_defs: &[ModelDefinitionJson],
    fallback_api: Option<&str>,
    fallback_base_url: Option<&str>,
    errors: &mut Vec<String>,
)
```

- 衝突判定: `models`（merge 時点の post-extension list）に `def.id`
  が存在 → skip + `errors.push`。authoritative なのは plugin 定義
  id だけでなく post-extension list の**全 id** — plugin が builtin
  provider id を register した場合（`CatalogStore::build` は builtin
  id の plugin registration に bundled `base_models` を供給する）
  builtin base model の id も含み得る。衝突した user def は skip +
  記録され、list 側の model を shadow しない。
- fallback chain: `api` = `definition.api → config.api → plugin.api`、
  `baseUrl` = `definition.baseUrl → config.baseUrl → plugin.base_url
  → find_model_defaults`。`model_from_json` が `provider_api`/
  `provider_base_url`/`defaults` の 3 引数を既に持つため、呼び出し側で
  `config.api.or(plugin.api)`・`config.base_url.or(plugin non-empty
  base_url)` を合成して渡す。`find_model_defaults` の参照先は merge
  時点の post-extension list（既存 `models`、追加済み user def も
  含む）。
- 非空 filter は `merge_user_model_defs` **内部**に置く（単一 site）:
  `model_from_json` は `Some("")` の base_url を verbatim に受け入れ、
  `extension_models` は `plugin_base_url.is_empty()` で guard する
  （`mod.rs:531-541`）。fallback 合成時に
  `.filter(|s: &&str| !s.is_empty())`（`config.base_url` 側も同様）
  を helper 内で適用すれば、compose・refresh の両 merge 経路で同一の
  非空判定になる — caller 側の各経路で個別に filter すると drift する。
- 非 fatal error の出口: `compose_provider` に `errors: &mut
  Vec<String>` 引数を追加し、merge stage は bare message を push する。
  `CatalogStore::build` が compose 結果に関わらず各 message を
  `format!("Provider \"{id}\": {msg}")` で errors へ集約する（fatal
  `Err` の prefix 規則と同じ）。

#### `ProviderEntry` — refresh 再評価用の raw 保持 field

```rust
/// plugin provider の raw user `models[]` defs（refresh 時に同一 merge
/// を再実行するため resolved 形ではなく definition のまま保持。
/// `model_overrides` と同じ構造）。plugin provider 以外は空。
plugin_user_defs: Vec<ModelDefinitionJson>,
/// merge fallback chain の config 層（`config.api`/`config.baseUrl`）。
config_api: Option<String>,
config_base_url: Option<String>,
/// `register()` が emit した `registration.models` の不変 copy —
/// persist guard (c) の比較基準（snapshot 注入の影響を受けない）。
pub(crate) plugin_bootstrap_defs: Vec<ModelDefinitionJson>,
```

visibility: `plugin_user_defs`・`config_api`・`config_base_url` は
`refresh_plugin_models`（`catalog/mod.rs` 内）だけが読むため
module-private field で足りる。`plugin_bootstrap_defs` は
`persist_plugin_catalog`（`auth/service.rs`、別 module）が guard (c)
の比較基準として読むため `pub(crate)` とする（accessor method は
single-use のため YAGNI）。

refresh 時の plugin-level fallback は既存 field で足りる:
`entry.plugin_api`（=`plugin.api`）と `entry.base_url`（plugin
provider では `plugin.base_url` — `compose_provider` の base_url 決定
が plugin 優先のため）。

#### `refresh_plugin_models` — merge 再実行 + header source 再構築

現行（`mod.rs:958-985`）の `extension_models` → `model_overrides`
再適用の間に merge を挟み、header source も再構築する:

1. `extension_models(provider_id, definitions,
   entry.plugin_api.as_deref().unwrap_or_default(),
   entry.base_url.as_deref().unwrap_or_default(),
   &self.models_of(entry))`（不変 — `plugin_api`/`plugin_base_url`
   は `&str` 引数のため `Option<String>` field は `as_deref` で渡す）。
2. `merge_user_model_defs(provider_id, &mut models,
   entry.plugin_user_defs.as_slice(),
   entry.config_api.as_deref().or(entry.plugin_api.as_deref()),
   entry.config_base_url.as_deref().or(entry.base_url.as_deref()),
   &mut errs)` — compose と同一 helper・同一 fallback chain。
   errs は `Provider "<id>": ` prefix 付きで `self.record_error`。
   非空 filter は helper 内部の単一 site に集約するため、call site
   では `as_deref()` のみで filter しない。
3. `model_overrides` 再適用（不変 — merge 後・最終層。
   `entry.model_overrides.as_ref()` で `Option<&BTreeMap<..>>`）。
4. `collect_model_header_sources(models, entry.model_overrides.as_ref(),
   Some(entry.plugin_user_defs.as_slice()), definitions)` で header
   map を再構築（`user_defs` は `Option<&[ModelDefinitionJson]>` —
   `&Vec<T>` ではなく `as_slice()` で渡す）。
5. `replace_plugin_models` で `dynamic_models` へ反映。

これにより衝突判定は refresh ごとに再評価される: compose 時に post-
extension list の id と衝突して skip された user def は、refresh で
plugin catalog がその id を落とした場合に effective になる（逆方向
も同規則 — id 復活時は post-extension list 側優先 + error 記録）。

#### `model_header_sources` — 集約 helper + dynamic 保持

compose 時の inline 構築（`mod.rs:616-654`）を helper 化して refresh
経路と共有する:

```rust
/// `rawModelHeaders` 相当の per-model source merge（template 未解決）。
/// precedence（後勝ち）: modelOverrides.headers < models[].headers
/// < plugin def headers。
fn collect_model_header_sources(
    models: &[CatalogModel],
    overrides: Option<&BTreeMap<String, ModelOverrideJson>>,
    user_defs: Option<&[ModelDefinitionJson]>,
    plugin_defs: &[ModelDefinitionJson],
) -> BTreeMap<String, Map<String, Value>>
```

- compose 時の `plugin_defs` は effective defs（persisted ??
  `plugin.models`）を渡す — persisted snapshot 由来の `headers` も
  request へ届く。
- `dynamic_models` の value を `Vec<CatalogModel>` から
  `{ models: Vec<CatalogModel>, model_header_sources: BTreeMap<..> }`
  の struct（`RefreshedCatalog` 等）へ変更し、refresh 時に header map
  も一緒に差し替える。`models_of` は `.models` を返すだけの調整。
- 読み取り側: `CatalogStore::model_headers(provider_id, model_id)` を
  追加し、refresh 済み source を優先・無ければ `entry.model_headers`
  へ fallback する（`ProviderEntry::model_headers` は compose-time
  source の accessor として残す。現在の request 経路に caller は無い
  が、今後の request header 解決は store accessor を経由させる）。

### `crates/cheaper-runtime/src/runtime.rs`

`Runtime::create`（`runtime.rs:41-74`）で plugin registration ごとに
snapshot を読み、`CatalogStore::build` へ注入する:

```rust
let mut persisted: BTreeMap<String, Vec<ModelDefinitionJson>> = BTreeMap::new();
let mut snapshot_errors: Vec<String> = Vec::new();
for reg in &registrations {
    // `catalog` seam（refreshModels）を持たない plugin は snapshot を
    // 生成し得ない — 残存 file があっても適用しない（plugin が seam を
    // drop した後の stale snapshot が provider を shadow するのを防ぐ）。
    if reg.catalog.is_none() {
        continue;
    }
    let path = context.catalog_path(&reg.id);
    match persisted::load_plugin_catalog(&path, &reg.id) {
        Ok(Some(defs)) => { persisted.insert(reg.id.clone(), defs); }
        Ok(None) => {}
        Err(e) => snapshot_errors.push(format!("Provider \"{}\": {e}", reg.id)),
    }
}
let catalog = CatalogStore::build(user_models, user_error, &registrations, &persisted)?;
for e in snapshot_errors {
    catalog.record_error(e);
}
```

- read error は `build` 後に `record_error`（`errors` は `Mutex` のため
  post-build で記録できる。build 引数を増やして internal error vec に
  混ぜるより、既存の runtime error channel を使う方が単純）。
- 検証失敗・file 不在は `registration.models`（bootstrap）へ
  fallback — `persisted` に entry が無ければ `compose_provider` は
  従来どおり `plugin.models` を使う。
- removed plugin の orphan file は読まない — read loop は registration
  単位で回るため、registration に存在しない provider id の
  `catalog.json` は読み取り対象にすらならない（削除もしない）。

### `crates/cheaper-runtime/src/auth/service.rs`

`refresh_plugin_catalog`（`service.rs:464-486`）で
`catalog.refresh_plugin_models` が `Ok(())` を返したときのみ、
persist guard を全て満たす場合に `write_plugin_catalog` を呼ぶ:

```rust
Ok(Some(definitions)) => {
    match catalog.refresh_plugin_models(&entry.id, &definitions) {
        // in-memory 反映に成功した defs のみ persist — 変換不能な defs を
        // 書くと自己修復不能な snapshot になる（下記）。
        Ok(()) => self.persist_plugin_catalog(
            catalog, entry, credential.is_some(), &definitions,
        ),
        Err(e) => catalog.record_error(format!("Provider \"{}\": {e}", entry.id)),
    }
}
```

persist を `refresh_plugin_models` の `Ok(())` 継続に限定する理由:
`extension_models` 変換に失敗する defs を snapshot 化すると、次回
boot で `Runtime::create` が同じ defs を注入して compose が再度
`extension_models` で失敗 → provider ごと drop する。すると
`auth login <id>` が `require_provider`（`service.rs:252-260`）で
`unknown provider` に落ち、login 経由の refresh による自己修復が
不可能になる（復旧には `catalog.json` の手動削除が必要）。

`persist_plugin_catalog`（`pub(crate)` — guard を login 経路なしに
unit test 可能にするため分離。integration test（`tests/`）からは
`pub(crate)` が見えないため、guard 単体の test は `service.rs` 内の
`#[cfg(test)]` inline module に置く）:

```rust
pub(crate) fn persist_plugin_catalog(
    &self,
    catalog: &CatalogStore,
    entry: &ProviderEntry,
    has_credential: bool,
    definitions: &[ModelDefinitionJson],
)
```

- guard (a): `has_credential`（call site で `credential.is_some()` —
  `refresh_models` に渡した credential が `Some`）。
- guard (b): `definitions` が非空。空なら write skip +
  `record_error`（観測性のため。空 list を persist すると reader 側の
  検証で必ず失敗する自己矛盾的 file になる）。
- guard (c): `definitions != entry.plugin_bootstrap_defs`
  （`ModelDefinitionJson` の `PartialEq` 等価比較。credential 有りでも
  `models_from_catalog` が fallback を返す経路がある — 例: 空
  `families`・変換後 0 件）。
- 全条件を満たす場合のみ `self.ctx.catalog_path(&entry.id)` へ
  `write_plugin_catalog`（`fetched_at` は `SystemTime` epoch ms）。
  write 失敗は `record_error` のみ — login を失敗させない（既存 F13
  契約: refresh/persist 失敗は login 結果を変えない）。

### `crates/cheaper-cli` — stderr surfacing

`CatalogStore::errors()` の各 error を `warning: <msg>\n` 行として
stderr へ出力する共通 helper を `cli.rs` に置く。sink は
`&mut dyn FnMut(&str)` とする — `auth.rs::execute_with` は
`CliWrite` を持たず `terminal: &mut dyn AuthTerminal`
（`write(&mut self, &str)`、`auth.rs:91-103`）しか受け取らないため、
`CliWrite` 型の引数では auth 側から呼べない。closure sink にすれば
`io.stderr.write_str` と `AuthTerminal::write` の双方を同一 helper で
流せ、test では scripted `AuthTerminal` の capture seam に warning が
そのまま落ちる:

```rust
/// spec Req 4 — catalog composition/persistence error の user-facing
/// surface。`--quiet` で抑制。envelope `result` shape・exit code 不変。
/// sink は `warning: <msg>\n` の行を受け取る closure
/// （`io.stderr.write_str` または `AuthTerminal::write` を wrap）。
pub(crate) fn emit_catalog_warnings(
    errors: &[String],
    sink: &mut dyn FnMut(&str),
    quiet: bool,
)
```

呼び出し点:

- `models.rs`: `create_runtime` 直後（`models.rs:78` の
  `let runtime = crate::run::create_runtime(...)` と `models.rs:79`
  の fallible `runtime.get_available(None)?` の間）— sink は
  `&mut |s: &str| io.stderr.write_str(s)`。emit が fallible consumer
  より先なので後続の `?` で warning が欠落しない。
- `run.rs`: `create_runtime` 直後（`run.rs:404`）— `run_worker`
  前・同様に後続の fallible 処理より先。sink は同上。なお fixture
  `auth_logins` の `AuthService::login` 失敗は `create_runtime` 内で
  `?` 伝播して emit 点前に return する（`run.rs:121-131`）が、
  fixture 専用経路のため warning 未 emit は受容する。
- `auth.rs`: `execute_with`（`auth.rs:525-617`）で**2 回** emit
  する。action match 内は全分岐 `?` 伝播（`auth.rs:544,548,576,595,
  601` 等）するため、match 後の emit は error 経路で走らない。また
  dropped plugin provider への `auth status <id>`・`auth login <id>`
  は `require_provider`（`service.rs:252-260`）が
  `unknown provider` を返し、記録済みの compose error が見えないまま
  終了する — spec Req 4 の禁止事項。そこで:

  1. **early emit**: `execute_with` 先頭、`catalog()`/`auth()` 取得
     直後・action dispatch の前に
     `emit_catalog_warnings(&catalog.errors(),
     &mut |s: &str| terminal.write(s), args.quiet)` を呼び、emit 済み
     件数を `let mut emitted = catalog.errors().len();` で保持する。
     これで action 成否・`require_provider` の成否に関わらず
     compose-time error（snapshot load error・provider drop 等）は
     stderr の `warning:` 行として必ず出る（dropped plugin では
     `warning:` 行 + `unknown provider` の envelope error が併存）。
  2. **post-login emit**: `auth login` arm で `auth.login(...)` が
     `Ok` を返した直後（`describe_provider` 再評価 `auth.rs:599-601`
     より前）に `let errs = catalog.errors();` →
     `emit_catalog_warnings(&errs[emitted.min(errs.len())..], …)`
     で差分だけ emit し `emitted` を更新する。`refresh_plugin_catalog`
     は `AuthService::login` 内で credential 永続化後・`Ok` 返却前に
     必ず走る（`service.rs:452`）ため、login `Ok` 時点で refresh/
     persist error は全て記録済み — error return の場合は refresh が
     走っていないので emit 不要。cursor で emit 済み分を除外するため
     compose error の再出力も起きない。

  status/logout は early emit のみで足りる（command 中に catalog error
  を追加する経路を持たない）。`--quiet` は early・post-login 双方を
  抑制する。

  emit 重複: `auth login` では同一の merge 衝突 message が compose 時
  と refresh 時 re-merge で 2 回 `errors` に記録され、warning 行と
  しても 2 行出得る。emit 側で dedup はしない — 記録の事実をそのまま
  出す accepted stderr noise とする（login 時のみ）。

gate は各 args struct の `quiet` field（`args.rs` — `ModelsCliArgs`/
`AuthCliArgs`/`RunCliArgs` いずれも `quiet: bool` 所持）で、既存の
`model_line`・`usage-log:` と同じ mechanism。

## Data and Control Flow

### login 経路（write）

```text
AuthService::login → store.set(credential)
  → refresh_plugin_catalog(catalog, entry)
      → plugin_catalog.refresh_models(ctx, credential)   // devin models list
      → Ok(Some(defs)) → catalog.refresh_plugin_models   // in-memory 反映
      → Ok(()) → persist_plugin_catalog                  // guard a/b/c
          → write_plugin_catalog(ctx.catalog_path(id))   // temp+rename
        （refresh_plugin_models Err → record_error のみ・persist しない）
```

### runtime 起動経路（read）

```text
Runtime::create
  → plugin register → registrations
  → per registration（reg.catalog.is_none() は skip）:
      load_plugin_catalog(catalog_path(id))
      → Ok(Some(defs)) → persisted[id] = defs
      → Err(msg)       → snapshot_errors += msg（build 後 record_error）
  → CatalogStore::build(models.json, user_error, registrations, persisted)
      → compose_provider: extension defs = persisted ?? plugin.models
      → plugin provider: provider-level 検証（oauth 等）+ base overlay
        → apply_extension → merge_user_model_defs → overrides
```

### compose の provider 分岐

| provider kind | user `models[]` の解決 | def error |
| --- | --- | --- |
| builtin / custom | `apply_models_json` 内（従来どおり） | fatal → provider drop（builtin は base fallback） |
| plugin | merge stage（post-extension list へ additive） | per-def skip + `record_error`（provider 存続） |

## Error Paths

| 地点 | 条件 | 記録形 |
| --- | --- | --- |
| `Runtime::create` | snapshot malformed / version 不一致 / provider 不一致 / `models` 空 / io error | `record_error("Provider \"<id>\": <msg>")` |
| `compose_provider`（plugin） | `config.oauth.is_some() && config.base_url.is_none()`（`{"oauth":…}` + baseUrl 無し — 従来と同一検証。`{"oauth":"radius","baseUrl":…}` は従来どおり radius env auth） | fatal Err → provider drop + `errors` |
| `merge_user_model_defs`（compose） | user def id が post-extension list と衝突 | `errors.push` → `Provider "<id>": <msg>` |
| `merge_user_model_defs`（compose） | `model_from_json` 失敗（baseUrl 解決不能・非正 contextWindow 等。`baseUrl:""` def も fallback 非空 filter で解決不能扱い） | 同上 |
| `refresh_plugin_models` merge 再実行 | 同上の 2 ケース（同一 helper・同一非空規則） | `record_error` |
| `refresh_plugin_catalog` | `refresh_models` Err | `record_error`（既存） |
| `refresh_plugin_catalog` | `refresh_plugin_models` Err | `record_error`（既存）— persist しない |
| `persist_plugin_catalog` | 空 definitions（guard b） | write skip + `record_error` |
| `persist_plugin_catalog` | `write_plugin_catalog` 失敗 | `record_error` |

CLI は `catalog.errors()` を `warning:` 行で stderr へ emit する
（`--quiet` で抑制）。envelope `result` shape・`schemaVersion`・exit
code は不変。

## Non-Functional Constraints

- `run`/`models`/`Runtime::create` から subprocess・network を呼ばない
  （offline・低遅延契約。read path は file I/O のみ）。
- `<userConfigDir>/pi/` 配下の file を一切読み書きしない。
- snapshot write は atomic（temp+rename、remove-first 無し）。失敗は
  best-effort の `record_error` で login/起動を壊さない。
- 新規依存なし（`serde`/`serde_json`/`std::fs` 既存）。env var・
  config key の追加なし。

## Risks and Trade-offs

- snapshot は login 間で stale になり得る（受容済み — endpoint 側の
  廃止 model は catalog 解決を通過して provider error として失敗し、
  `MODEL_TIER_UNAVAILABLE` に戻らない。`fetchedAt` で診断可能）。
- merge は port 済み `apply_extension` の replace semantics との乖離
  （defect 修正として正当化済み — upstream TS parity は vendored
  source 不在のため未検証前提）。
- `CatalogStore::build` の signature 変更は既存 test（`tests/catalog.rs`
  `tests/auth.rs` の `&[]` 呼び出し）を一括更新する機械的変更を伴う。
- guard (c) の残余リスク: live refresh が正当に bootstrap list と
  等価な defs を返す一方で、より rich な stale `catalog.json` が
  残存するケースでは write が skip され stale snapshot が持続する
  （自己修復は非対称）。bootstrap 等価な refresh 応答は
  `models_from_catalog` の fallback-on-empty-flaky 経路（空
  `families`・変換後 0 件 → `fallback_models()`）と区別できず、
  その場合に snapshot を上書きすると正常な catalog を破壊する —
  residual staleness の方が害が小さいため受容する。

## Deferred Design Concerns

- 起動時 lazy/TTL refresh | one-shot process には background refresh
  の実装先が無く、subprocess 依存を起動経路へ持ち込まない判断 | 鮮度
  要件が高まった時点で別 ADR | stale snapshot のまま運用される。
- envelope `warnings` field（構造化 surface） | envelope 不変契約
  （spec Req 6）との整合設計が別途必要 | machine consumer が catalog
  error を parse したくなった時点で別 chain | 当面は stderr テキスト
  のみ。

## Verification Notes

既存 seam: fake `PluginCatalog`/`PluginAuth`（`tests/plugin.rs` の
`FakeCatalog`/`FakeAuth`）、`DevinProviderPlugin::with_runner`（fake
`DevinCliRunner`）、`CHEAPER_CONFIG_DIR` + tempfile（runtime・CLI test
共通）、`CatalogStore::build` 直接呼び出し、CLI in-process `run_cli`
+ `VecSink` stderr capture。

- [ ] `catalog/persisted.rs` unit test: write→load round-trip
      （envelope `version:1`/`provider`/`fetchedAt`/`models`）、
      malformed・version 不一致・provider 不一致・空 `models` → Err、
      不在 → `Ok(None)`、unknown key 読み飛ばし。
- [ ] 永続化 e2e（fake plugin + tempdir）: login →
      `providers/fake/catalog.json` 生成 → **新** `Runtime::create` で
      persisted model が `get_supported_model`/`get_available` に見える
      （AC: 別 process `models`/`run` の根幹）。
- [ ] persist guard: 空 definitions・bootstrap 等価 definitions
      （`fake` plugin で `refresh_models` が registration.models と同
      list を返す注入）、credential `None`（`persist_plugin_catalog`
      を直接呼ぶ）で既存 `catalog.json` を上書きしない。
- [ ] merge: `providers.fake.models[]` の新 id が catalog に残り
      resolution 可能 / `api`・`baseUrl` 省略 def が plugin 宣言値で
      解決 / post-extension list の id 衝突 → list 側優先 + error
      記録 / invalid def → その def のみ skip + provider・plugin
      models 存続 / `plugin.models` 空の fake provider でも merge
      が効く。
- [ ] refresh merge 一本化: `refresh_plugin_models` 後も user-added
      model が保持 / compose 時に衝突 skip された user def が refresh
      で post-extension list から id 消滅後に effective / refresh で
      id 復活時は list 側優先 + error 記録。
- [ ] header 再構築: refreshed defs の `headers` が refresh 後の
      `model_header_sources`（`CatalogStore::model_headers`）に反映。
- [ ] CLI surfacing: malformed `catalog.json` 配置で `models`・
      `auth status`・`auth login`（fake runner で refresh 失敗注入
      し同一 run で emit 確認）・`run`（fixture seam）の stderr に
      `warning:` 行 / `--quiet` で抑制 / envelope `result` shape・
      exit code 不変。`execute_with` の emit は scripted
      `AuthTerminal::write` 経由で assert する（sink seam）。
- [ ] logout は `catalog.json` を削除せず、次回 `Runtime::create` が
      同 snapshot を引き続き使う（spec freshness AC — logout 後も
      persisted model が catalog に残る）。
- [ ] refresh 失敗 defs の persist 抑止: fake `refresh_models` が
      `extension_models` 変換不能な defs（baseUrl 解決不能等）を返す
      と `refresh_plugin_models` Err → `record_error` され、
      `catalog.json` が**書かれない・既存 file を上書きしない**
      （persist が `Ok(())` 継続に限定されていることの検証）。
- [ ] plugin provider の config `{"oauth":"radius"}`（baseUrl 無し）:
      compose で今日と同じ `oauth ⇒ baseUrl` 検証が Err → provider
      drop + `errors` 記録（無条件 Err 化しない —
      `{"oauth":"radius","baseUrl":…}` は従来の radius env auth
      semantics のまま検証を通過する）。
- [ ] fake plugin `base_url:""` + baseUrl 無し user def: compose と
      `refresh_plugin_models` の**両経路**で def skip + `errors`/
      `record_error`（空 base_url の model が残らない — helper 内
      非空 filter の検証）。
- [ ] `auth status <dropped-plugin>`: compose error が stderr の
      `warning:` 行として emit され、`unknown provider` の envelope
      error と併存する（early emit が action 失敗を踏まない検証）。
- [ ] 既存回帰: builtin/custom compose・provider drop 契約・
      `mise run check`（cargo test/clippy/fmt + check-doc-ids）green。

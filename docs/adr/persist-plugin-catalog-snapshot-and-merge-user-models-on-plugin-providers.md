---
id: "ADR-034CKnwOWyYssJ1VZR9spo"
type: "adr"
status: "proposed"
title: "plugin live catalog は providers/<id>/catalog.json に永続化し user models[] は plugin provider で additive merge にする"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/81"
    - "https://github.com/scarecrow173/cheaper/issues/82"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
  depends-on:
    - "./provider-only-registration-via-explicit-bundled-plugin-contract.md"
    - "./migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
    - "./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
    - "../specs/plugin-catalog-persistence-and-user-models.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/plugin-catalog-refresh-persistence-and-user-models.md"
  derived-by:
    - "../designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
    - "../plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  verifies: []
  verified-by:
    - "../test-specs/plugin-catalog-persistence-and-user-model-merge.md"
  references:
    - "../tasks/phase-3-port-runtime-adapter-providers.md"
    - "../impl/ir/phase-3-port-runtime-adapter-providers.md"
    - "../designs/devin-local-provider-plugin.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# plugin live catalog は providers/<id>/catalog.json に永続化し user models[] は plugin provider で additive merge にする

## Context and Problem Statement

`devin` provider（bundled plugin、`crates/cheaper-provider-devin`）の実 endpoint smoke で判明した 2 つの相乗する defect（issue #81・#82、上流 discovery `DISC-o3GbG6FWEBRFcbNBaslikh`、実測記録は `docs/tasks/phase-3-port-runtime-adapter-providers.md`）を解消する設計を固定する。

- **Defect 1（issue #81）**: plugin の live catalog refresh（`DevinPluginCatalog::refresh_models` → `devin models list --format json`）の結果が login process の in-memory state にのみ保持される。refresh の呼び出し経路は `AuthService::login` 成功後の `refresh_plugin_catalog`（`crates/cheaper-runtime/src/auth/service.rs:452` 起動、`464-486` 本体）のみで、結果は `CatalogStore::refresh_plugin_models` → `replace_plugin_models` が `dynamic_models: RwLock<BTreeMap>`（`crates/cheaper-runtime/src/catalog/mod.rs:780`）へ入れる。**永続化経路は存在しない**。`cheaper` CLI は command ごとに別 process で、`Runtime::create`（`crates/cheaper-runtime/src/runtime.rs:41-74`）は起動ごとに bundled snapshot + `<userConfigDir>/models.json` + plugin registration から catalog を再構築するため、別 process の `run`/`models` は bootstrap `fallback_models()` 4 件（`crates/cheaper-provider-devin/src/plugin.rs:238`）しか見えず、実 endpoint が公開する 100+ variant（`swe-2` family 含む）は `resolve_explicit_model`（`crates/cheaper-core/src/model/resolve.rs:272-285`）で `MODEL_TIER_UNAVAILABLE` になる。
- **Defect 2（issue #82）**: `compose_provider`（`catalog/mod.rs:591-613`）は `apply_models_json` → `apply_extension` → `modelOverrides` の順で合成するが、`apply_extension`（`catalog/mod.rs:475-506`）は `plugin.models` が非空なら model list を**丸ごと replace** し、user の `providers.<id>.models[]` 追加を silently 破棄する。`modelOverrides` は既存 id の per-field mutate のみ（`catalog/mod.rs:607-613`）で新規追加は不能。さらに plugin provider は builtin meta・`base_models` を持たないため、user definition が `api`/`baseUrl` を欠くと `model_from_json` が error → compose error で **provider ごと drop** する（`catalog/mod.rs:841-855`、builtin 以外は fallback 無し）。どちらの経路でも user `models[]` は最終 catalog に到達せず、replace 経路では catalog error にも記録されない。

2 つの defect は相乗する: Defect 1 で live catalog が `run` から見えない以上、自然な回避策は user `models[]` への手動追加だが Defect 2 がそれを塞ぎ、plugin provider の model 追加経路が完全に閉じている。片方だけの修正では回避不能・診断不能な状態が残るため同一 work chain で扱う。

決めるべきこと: (a) refresh 済み plugin catalog を process 境界を越えて `run`/`models` へ届ける方法（所在・形式・鮮度）、(b) plugin provider における user `models[]` の merge 規則と invalid def の扱い、(c) catalog error の user-facing surfacing（spec Req 4）。

## Decision Drivers

- `cheaper` CLI は command ごとの one-shot process（ADR `one-shot-json-first-cli`）— login process の refresh 結果を `run`/`models` process へ届けるには永続化が必須で、in-memory の延長では解決しない。
- `run`/`models` は offline かつ低遅延でなければならない — `devin` CLI subprocess + network への新規依存を起動経路に持ち込めない（`devin models list` は timeout 20s の child process、`crates/cheaper-provider-devin/src/models.rs:440-469`）。
- `<userConfigDir>` は cheaper の user-scope state の canonical root（credentials.json・models.json・usage.jsonl）。pi 由来 file（`auth.json`/`pi/` 以下、`PI_CODING_AGENT_DIR`）は一切読まない方針が migrate ADR で維持されている。plugin 固有 state は `<userConfigDir>/providers/<id>/` が既存の境界（`ProviderPluginContext::plugin_dir`、`crates/cheaper-runtime/src/plugin.rs:48-50`、devin の `providers/devin/data/`）。
- 外部 contract 不変: envelope `schemaVersion:1`・exit code・`models` exit 0 contract・`models.json` schema。
- TS 前身（Pi `ModelRuntime`）は refresh 済み catalog を `models-store.json` で永続化し後続 process が読んでいた。Rust port はこれを v1 非実装として落とした（`docs/impl/ir/phase-3-port-runtime-adapter-providers.md:533`）— 本 ADR は pi file を読むのではなく、同等機能を cheaper-owned file として再実装する。
- 表現ドリフト禁止（AGENTS.md）: bootstrap models・persisted snapshot・live refresh の 3 経路で同一の変換規則（`extension_models`）を通すため、永続化するのは resolved `CatalogModel` ではなく plugin 出力の `ModelDefinitionJson`。
- silent drop 禁止: user config が捨てられる経路は catalog error（TS `compositionErrors` 相当、`CatalogStore::errors`/`record_error`）へ記録して診断可能にする。
- refresh（login 後の差し替え）経路でも merge 規則が共有されなければ、永続化と merge を入れても login 後 refresh で user `models[]` が再び消失する（`extension_models` は compose/refresh の両経路で共有済み、DRY）。

## Considered Options

### 決定 1 — refresh 済み plugin catalog の process 間受け渡し（issue #81）

- **A: `<userConfigDir>/providers/<id>/catalog.json` へ runtime-owned snapshot を永続化する**（採用）。login 成功後の `refresh_plugin_catalog` が適用した `ModelDefinitionJson` list を `{version, provider, fetchedAt, models}` envelope で atomic write し、`Runtime::create` が読んで plugin の extension model source として注入する。credential/models.json と同 layer で `pi/` 非読取方針・`plugin_dir` 境界と整合し、offline/低遅延の run 契約を壊さない。refresh 契機は既存の login seam のみ（TTL/read 時 refresh は v1 非採用）。
- **B: runtime 起動時・`models`/`run` 時に credential があれば lazy refresh**。常に最新を得られるが、process ごとに `devin` subprocess + network を spawn し、timeout（最大 20s）・offline 環境で `run` が遅延・失敗し得る。one-shot CLI の起動経路に network 依存を持ち込むため不採用。
- **C: 永続 snapshot + TTL/stale 時 refresh の hybrid**。鮮度は改善するが、stale 判定時に (b) と同じ subprocess 依存が読み取り経路へ入り、one-shot process では「background refresh」の実装先が無い。v1 では不採用 — `fetchedAt` を schema に含めておき、必要になれば別 ADR で追加できる形に留める。
- **D: plugin 自身が `plugin_dir` に永続化する（plugin-owned）**。`register` 時に plugin が persisted defs を読んで `models:` に差し込めば runtime contract 変更不要だが、永続化 policy（path・envelope・atomicity・鮮度）が plugin ごとに複製される。refresh seam（`refresh_plugin_catalog`）は既に runtime が所有しており、pi でも永続化は ModelRuntime 側の責務だった — policy の single authoritative implementation は runtime 側に置く（DRY）。

### 決定 2 — plugin provider における user `models[]` の merge 規則（issue #82）

- **α: user `models[]` を plugin extension list へ upsert merge（id 衝突は user 優先）**。追加は可能になるが、`models[]`（定義の全置換）と `modelOverrides`（per-field mutate）の責務が重複し、user def が plugin catalog id を silently shadow する — login refresh 後に挙動が変わり得て予測しにくい。不採用。
- **β: replace 維持（TS parity）+ 破棄を catalog error で明示**。現行 contract 最小変更で診断可能性だけ回復するが、user の追加経路は依然存在せず issue #82 を解消しない（Defect 1 修正後も、login 間に endpoint が追加した id へ user が手を出せない）。不採用。
- **γ: additive merge — plugin 未定義 id の user def のみ追加し、plugin 定義 id は plugin が authoritative**（採用）。「`models[]` は追加、`modelOverrides` は既存 model の mutate」という責務分離を維持したまま追加経路を開く。port 済み `apply_extension`（`catalog/mod.rs:477-506`）の丸ごと replace との乖離だが、replace 自体が issue #82 の defect であり merge への変更は defect 修正として独立に正当化される。vendored TS source は repo に存在しないため、upstream TS `applyExtension` が同じ replace semantics かは未検証（port コメント由来の hypothesis として扱う）。

### 付随 defect — invalid user def による provider 丸ごと drop

plugin provider で user def が `api`/`baseUrl` を欠くと `model_from_json` が error → compose error で provider ごと drop する経路を同時に直す（in scope）。

- **採用**: plugin provider の user `models[]` def は `apply_models_json` 内で resolve しない（`model_from_json` の error が `?` で伝播し provider ごと drop する現行経路を塞ぐ — `catalog/mod.rs:600`・`466`）。`apply_extension` 適用後の model list に対して走る merge stage で各 def を解決し、`api`/`baseUrl` の fallback chain は `definition → config.api/baseUrl → plugin.api/base_url → find_model_defaults` とする。それでも解決不能な def や検証 error は **その def のみ skip + catalog error 記録**で provider・plugin models を残す。plugin provider の `compose` error による provider drop は auth 欠落等の fatal のみに限定する。custom（plugin 非登録）provider は従来どおり def error が fatal — provider kind による semantics の divergence を明示する。
- **却下**: 現状維持（drop + error 記録）。error は記録されるが plugin bootstrap models まで全消失し、user `models[]` 追加の試行が provider を壊す — merge 導入後は最悪の失敗形になる。

### 決定 3 — catalog error の user-facing surface（spec Req 4）

`CatalogStore::errors`/`record_error`（`catalog/mod.rs:777`・`876`）は現状 CLI 表面に露出していない。spec Req 4 は compose/skip/discard error のユーザー観測可能性を要求するため、surface をここで決める。

- **A: stderr diagnostic（採用）**。`cheaper models`・`cheaper run`・`cheaper auth status`・`cheaper auth login` が runtime 構築後に `catalog.errors()`（`runtime.catalog()`、`runtime.rs:82`）を読み、各 error を `warning: <msg>` 行として stderr へ出力する（`warning: Provider "devin": ...` 形 — error は `Provider "<id>": <msg>` 形式で記録済み、`catalog/mod.rs:831`・`service.rs:475,480`）。`auth login` を surface に含めるのは、login 成功後の `refresh_plugin_catalog` が記録する refresh/persist 失敗の `record_error` が同一 one-shot process の終了とともに消え、他 command の surface では観測不能になるため — login 自身がその run 中に記録された catalog error を同じ `warning:` 行で emit する。`--quiet`（global bool option、`crates/cheaper-cli/src/args.rs:253`・各 args struct、HELP は "Suppress stderr diagnostics"）で抑制し、`run` の `model_line`・`usage-log:` と同じ mechanism を再利用する。envelope `result` shape・`schemaVersion`・exit code は一切変えない。stdout envelope 契約と診断出力が分離済みで、parent agent の parsing に影響しない最小変更。
- **B: envelope への `warnings` field 追加**。構造化され機械可読だが `result` shape の変更を伴い、envelope 不変契約（spec Req 6）との整合設計が別途必要になる。**明示的に別 chain へ defer する**（必要になった時点で別 ADR）。
- **C: exit code / error envelope で報告**。catalog error は非 fatal 設計（provider drop 後も catalog は残り `models` は exit 0）なので、exit code を変えると `models` exit 0 contract を壊す。不採用。

## Decision Outcome

Chosen options: 決定 1 = **A**（runtime-owned persisted snapshot）、決定 2 = **γ**（additive merge、plugin ids authoritative）+ 付随 defect の per-def error 化、決定 3 = **A**（catalog error を stderr diagnostic として emit）。

確定した決定事項:

- **snapshot file**: `<userConfigDir>/providers/<providerId>/catalog.json`。`ProviderPluginContext::plugin_dir`（`plugin.rs:48-50`）と同一の境界で、runtime（plugin ではなく）が所有する。envelope は `{ "version": 1, "provider": "<id>", "fetchedAt": <epoch ms>, "models": [ModelDefinitionJson, ...] }`。`fetchedAt` は v1 では診断用途のみで TTL 判定には使わない。
- **envelope 検証（load 時）**: file 不在は silent fallback（初回は常に不在で error ではない）。以下は **file 全体を無視して bootstrap `registration.models` へ fallback + `record_error`** する: (a) malformed JSON / schema error、(b) `version` 不一致、(c) `provider` field が directory の plugin id と不一致（cross-provider の file 取り違え・手動配置を誤適用しない）、(d) `models` が空。unknown key は同 version 内の前方互換のため読み飛ばす。**versioning 規則**: `ModelDefinitionJson` への field 追加・削除を含む schema 変更は envelope `version` の bump を必須とし、旧 version の file は (b) で無視され次回 login refresh で再生成される。同一 version 内での field 追加は行わない。
- **永続化対象**: `ModelDefinitionJson` list（`PluginCatalog::refresh_models` の返却形）を verbatim 保存する。resolved `CatalogModel` や合成済み model は保存しない — `api`/`base_url` fallback・defaults 解決は compose 層の責務であり、bootstrap/persisted/refresh の 3 経路が同一の `extension_models` 変換を通る。
- **write 契機 + persist guard**: `refresh_plugin_catalog`（login 成功後、`service.rs:464-486`）で `catalog.refresh_plugin_models` が `Ok` のとき、適用した同じ definitions を保存する — ただし **次の 3 条件をすべて満たす場合に限る**: (a) その refresh が credential 有りで実行されたこと（`refresh_models` 呼び出しに渡した credential が `Some`）、(b) 適用した definitions list が空でないこと、(c) 返却 definitions が **plugin 宣言の bootstrap model list** と等しくないこと（`ModelDefinitionJson` は `PartialEq` derive 済み — `schema.rs:434`。等価比較で判定する）。(b) について: `refresh_plugin_models` は空 definitions を `Ok(())` で bootstrap 維持として扱い（`catalog/mod.rs:966-968`）、reader 側も空 `models` を file 無視 + `record_error` と定めているため、空 list を persist すると次回起動で必ず検証失敗する自己矛盾的な file になる — write は skip とし、観測性のため `record_error` してよい。(c) の比較基準は **`register()` が emit した `registration.models` の、snapshot 注入前の不変 copy** と定義する — persisted defs を `registration.models` へ差し替えて注入する実装では、post-snapshot login process の同 field が snapshot 自身になるため、refresh が bootstrap 等価の definitions を返しても基準（snapshot）と非等値で persist が通り、live snapshot が bootstrap で上書きされる。credential `Some` でもこの経路は実在する — `models_from_catalog` は空 `families`・変換後 0 件でも `fallback_models()` 4 件を返す（`models.rs:422-435`）。この曖昧さを排するため、**injection は `registration.models` を差し替えず別 source（`CatalogStore::build` への別引数）として渡すことを normative とする**（read 契機参照）— `registration.models` は常に plugin 宣言の bootstrap list であり、guard の比較基準と bootstrap fallback baseline が一意に保たれる。guard が必要な理由: `DevinPluginCatalog::refresh_models` は credential `None` で `Ok(Some(fallback_models()))`（4 件 bootstrap）を返す（`crates/cheaper-provider-devin/src/plugin.rs:172-175`）ため、無条件 persist だと login process の credential read 失敗だけで live 100+ model の snapshot が bootstrap で上書きされる。また `models_from_catalog` は devin で空を返さない（`None`・空 `families`・変換後 0 件のいずれでも `fallback_models()`、`models.rs:422-435`）ので (b) の空 skip だけではこの経路を防げず、(c) の等価比較が要る。write 失敗・refresh 失敗は login/refresh 結果を変えず `record_error` する。catalog は secret を含まないため `0600` は要しない。
- **atomic write**: 同 dir temp file + `std::fs::rename` で置換し、**remove-first はしない**。`std::fs::rename` は Windows でも既存 target を置き換える（`MoveFileExW` + `MOVEFILE_REPLACE_EXISTING` semantics）— remove→rename の gap は crash window で非 atomic になる。`CredentialStore::write_file`（`store.rs:243-292`）は `#[cfg(windows)]` で target 事前 remove を行う（`store.rs:278-281`、コメントは「Windows の MoveFile 系は target 存在で失敗」と記述）が、これは現行 `std::fs::rename` の挙動と食い違う継承 workaround であり本 writer へは複製しない（credential 側の修正は本 ADR の scope 外）。
- **read 契機**: `Runtime::create` が各 plugin registration に対し `<userConfigDir>/providers/<id>/catalog.json` を読み、上記 envelope 検証を通過した場合のみそれを plugin の extension model source として `CatalogStore::build` へ渡す。injection は **`registration.models` を差し替えない別 source（`CatalogStore::build` への別引数）として行う**（normative — persist guard の比較基準を注入前 bootstrap に固定するため、`registration.models` は常に `register()` が emit した plugin 宣言 bootstrap のまま残す）。解釈は必ず `apply_extension`/`extension_models` 単一路径を通す。検証失敗・file 不在は `registration.models`（bootstrap）へ fallback する。
- **refresh 契機は login のみ**: `run`/`models`/runtime 起動時の lazy/TTL refresh は実装しない。credential 無しの refresh（devin は `Ok(Some(fallback_models()))`）は in-memory では bootstrap と同内容で無害だが、persist guard により snapshot も上書きしない。logout・credential 無効化時に snapshot を削除しない（unconfigured provider は `get_available` の configured filter で見えず、stale snapshot は次回 login refresh で自己修復する）。
- **user `models[]` merge（plugin provider のみ・`plugin.models` の空否に依らず適用）**: plugin registration を持つ provider では、user `config.models` の各 def を `apply_models_json` 内で resolve せず raw `ModelDefinitionJson` のまま merge stage へ回す。merge stage は `apply_extension`（persisted defs 注入時は `extension_models`）適用後の model list に対して走り、plugin 定義 id と衝突しない user def を `model_from_json` で解決して末尾へ追加する（plugin 定義順 → user 追加順の file 順）。衝突 def は plugin 定義を優先して skip + `record_error` する。merge は「plugin defs が自 id を authoritative に持ち、user は残りの id を追加する」規則であり extension の replace が走ったかに依らない — `plugin.models` が空で `apply_extension` が replace しない provider でも同一規則を適用する。custom（plugin 非登録）provider では従来どおり `apply_models_json` 内で def を resolve し、def error は provider drop の fatal のまま — provider kind による semantics の divergence を明示する。
- **fallback chain（plugin provider の user def）**: merge stage の `model_from_json` 呼び出しでは provider-level fallback を合成して渡す。`api` = `definition.api → config.api → plugin.api`（`PluginProviderRegistration.api`）、`baseUrl` = `definition.baseUrl → config.baseUrl → plugin.base_url → find_model_defaults`（参照先は merge 時点の post-extension model list）。解決不能・検証 error はその def のみ skip + `record_error`。provider 自体・plugin models は残す。
- **refresh 経路の merge 一本化（spec Req 3）**: `ProviderEntry` は plugin provider の user `config.models` def を raw `ModelDefinitionJson` list として保持する（resolved 形ではなく — refresh 後の plugin set との衝突を再評価できるようにする。`model_overrides` の保持と同じ構造、`catalog/mod.rs:175`）。`refresh_plugin_models` は refreshed definitions を `extension_models` で解決した後、**compose と同一の additive merge を再実行**し（保持した user def を同一 fallback chain で resolve、id 衝突は skip + `record_error`）、最後に `model_overrides` を再適用する。これにより衝突判定が refresh ごとに再評価される: compose 時に plugin id と衝突して skip された user def は、refresh で plugin catalog がその id を落とした場合に effective になる（逆に有効だった user def が plugin 定義 id に取って代わられた場合は plugin 側が勝ち catalog error に記録される）。
- **`model_header_sources` の refresh 再構築**: compose 時の `model_header_sources`（`catalog/mod.rs:616-654`）は registration `plugin.models`・`config.models`・`modelOverrides` のみを source にしており、`refresh_plugin_models` で差し替わった defs の `headers` はそのままでは request に届かない。refresh 時に refreshed defs（+ merge 済み user def + overrides）から同じ precedence で再構築する（`dynamic_models` 側に header map も保持する形か entry 更新かは実装詳細）。
- **catalog error の surfacing（決定 3A / spec Req 4）**: `cheaper models`・`cheaper run`・`cheaper auth status`・`cheaper auth login` は runtime 構築後に `catalog.errors()` を読み、各 error を `warning: <msg>` 行（`warning: Provider "devin": ...` 形）として stderr へ出力する。`auth login` は自身の run 中に `refresh_plugin_catalog` が記録した refresh/persist 失敗を含む（これらは login process 終了とともに消えるため、login 自身が emit しなければ観測経路がない）。`--quiet` で抑制し、envelope `result` shape・`schemaVersion`・exit code は不変（envelope `warnings` field 等の構造化 surface のみ明示的に別 chain へ defer）。
- **不変**: `modelOverrides` の per-field mutate 契約、`models[]` headers/`model_header_sources` の merge precedence（`rawModelHeaders` spread 順）、builtin/custom provider の merge 規則と provider drop 契約、envelope `schemaVersion:1`、exit code、`<userConfigDir>/pi/` 非読取はすべて維持する。custom（user config）provider での invalid def による provider drop は既存契約のままとする（plugin provider にのみ per-def skip を導入）。

### Consequences

- Good, because login 1 回で `devin models list` の live catalog が全後続 process の `run`/`models`/`resolve` から見え、`swe-2` family 等の endpoint 公開 model が `MODEL_TIER_UNAVAILABLE` ではなく選択可能になる（issue #81 の解消）。
- Good, because `providers.devin.models[]` に新 id を書くだけで plugin provider へ model を追加できるようになり、`api`/`baseUrl` 省略でも plugin 宣言値で解決される（issue #82 の解消）。
- Good, because user config の破棄・shadowing・def 単位の invalid がすべて catalog error へ記録され、silent drop がなくなる。
- Good, because `run`/`models` の起動経路に subprocess/network 依存を持ち込まず、offline で snapshot がそのまま使える。
- Bad, because snapshot は login 間で stale になり得る — endpoint が model を追加/廃止しても再 login（または将来の TTL refresh 実装）まで反映されない。`fetchedAt` で診断は可能。なお stale entry の廃止 model は catalog 解決を通過して endpoint で通常の error mapping（provider error）として失敗する — `MODEL_TIER_UNAVAILABLE` に戻るわけではない。
- Bad, because merge への変更は port 済みコードで観測される `apply_extension` の replace semantics との乖離であり、記録・正当化が必要になる（upstream TS `provider-composer.ts` との parity 自体は vendored source 不在のため未検証の前提 — ただし merge は defect 修正として parity の有無に依らず正当化される）。
- Bad, because `ModelDefinitionJson` 系型は現在 `Deserialize` のみで、persist には `Serialize` derive の追加が必要（schema.rs の型群）。
- Good, because catalog error channel（`CatalogStore::errors`/`record_error`）が stderr diagnostic（`warning:` 行、`--quiet` 抑制可）として `models`/`run`/`auth status`/`auth login` から観測可能になり（login 中の refresh/persist 失敗も同一 run で emit される）、spec Req 4 を envelope 非変更で満たす。envelope `warnings` field 等の構造化 surface のみ明示的に別 chain へ defer する。
- Neutral, because IR の「`models-store.json` / `withRemoteCatalog` は v1 非実装」判断は、pi file の読み取りではなく cheaper-owned snapshot の導入として一部改訂される（pi/`models-store.json` は引き続き読まない）。

### Confirmation

- fake `PluginCatalog`（`crates/cheaper-runtime/tests/plugin.rs` の refresh 注入 seam）+ 一時 `CHEAPER_CONFIG_DIR` で、login → `<dir>/providers/<id>/catalog.json` 生成 → 新 `Runtime::create` で persisted model が `get_model`/`get_available` に見えることを確認。
- persisted file 破損・version 不一致・`provider` 不一致・空 `models` で bootstrap fallback + catalog error 記録を確認。
- persist guard: credential 無し refresh、空 definitions、および注入前 bootstrap `models` と等価な definitions（persisted snapshot が注入済みの process で bootstrap 等価の返却をする経路を含む）では既存 `catalog.json` を上書きしないことを確認。
- plugin provider の user `models[]` merge: 新 id 追加・`api`/`baseUrl` 省略時の plugin 値 fallback・id 衝突 skip + error 記録・invalid def skip + provider 存続を unit test で確認（`plugin.models` 空の fake provider でも merge が効くことを併せて確認）。
- `refresh_plugin_models` 後に user-added model が保持されること、compose 時に plugin id 衝突で skip された user def が refresh で plugin id 消滅後に effective になることを確認。
- refresh で差し替わった defs の `headers` が `model_header_sources` 再構築を経て request に届くことを確認。
- catalog error が `models`/`run`/`auth status`/`auth login` の stderr に `warning:` 行で出ること（`auth login` は login 中の refresh/persist 失敗の `record_error` が同一 run で emit されることを含む）、`--quiet` で出ないこと、envelope `result` shape が不変であることを CLI test で確認。
- 既存 compose 回帰: builtin/custom provider の catalog compose test と differential golden が変化なしで green。

## Implementation Plan

- Affected paths:
  - `crates/cheaper-runtime/src/catalog/schema.rs` — persisted file 用の `PluginCatalogFile`（`version`/`provider`/`fetched_at`/`models`）serde 型を追加するか `catalog/persisted.rs` を新設。`ModelDefinitionJson`・`ModelCostJson`・`ModelCostTierJson`・`ThinkingLevelMap`・`ModelInput` に `Serialize` derive を追加する。
  - `crates/cheaper-runtime/src/catalog/mod.rs` — plugin provider では `apply_models_json` が user `config.models` を resolve せず raw def を merge stage へ回すよう分岐する（custom provider は従来どおり `apply_models_json` 内で fatal resolve）。merge stage は `apply_extension` 後の model list に対し、`plugin.models` 空否に依らず全 plugin provider で走り、fallback chain（`config.api/baseUrl → plugin.api/base_url → find_model_defaults`、参照先は post-extension list）で `model_from_json` する。衝突/invalid は def 単位で skip + error 集約（compose の error 集約を `(ProviderEntry, Vec<String>)` 相当または `errors: &mut Vec<String>` 引数へ拡張）。`ProviderEntry` に raw user defs（`ModelDefinitionJson` list）を保持する field を追加し、`refresh_plugin_models` で同じ additive merge + `modelOverrides` 再適用 + `model_header_sources` 再構築を行う。
  - `crates/cheaper-runtime/src/runtime.rs` — `Runtime::create` で plugin registration ごとに `providers/<id>/catalog.json` を load し、envelope 検証（version・`provider`・`models` 非空）通過時のみ extension model source として `CatalogStore::build` へ別引数で注入する（`registration.models` は差し替えず plugin 宣言 bootstrap のまま — persist guard の比較基準を注入前 bootstrap に固定するため）。検証失敗・parse 失敗は bootstrap fallback + `record_error`。
  - `crates/cheaper-runtime/src/auth/service.rs` — `refresh_plugin_catalog` で persist guard（credential `Some` で実行された refresh・definitions 非空・definitions が注入前の plugin 宣言 bootstrap `models` と非等価）を満たす場合のみ `providers/<id>/catalog.json` へ atomic write。空 definitions・bootstrap 等価は write skip（空は `record_error` してよい）。失敗は `record_error` のみ。
  - `crates/cheaper-runtime/src/plugin.rs` — `plugin_dir` 既存。必要なら `catalog_path(plugin_id)` helper を `ProviderPluginContext` に追加して path 組み立てを一箇所に集約する。
  - `crates/cheaper-cli/src/models.rs`・`run.rs`・`auth.rs` — runtime 構築後に `runtime.catalog().errors()` を読み、各 error を `warning: <msg>` で `io.stderr` へ出力する（`args.quiet` で抑制）。`auth.rs` は `auth status` に加え `auth login` も対象（login 中の refresh/persist 失敗の `record_error` を同一 process で emit）。envelope・exit code は不変。
  - persisted file 書き込みは `CredentialStore::write_file`（`crates/cheaper-runtime/src/auth/store.rs:248-292`）の temp+rename pattern を参考にするが、その `#[cfg(windows)]` 事前 remove（`store.rs:278-281`）は複製しない — `std::fs::rename`/`MoveFileExW` は Windows でも既存 target を置き換えるため remove→rename の非 atomic window を持ち込まない。共通化するなら runtime 内に小さな atomic-write helper を置き、credential 側の remove-first 解消は別途行ってよい（表現ドリフト回避）。
- Patterns to follow: `extension_models`（bootstrap/refresh 両経路で共有済み）を persisted/merge 経路でも唯一の definition→model 変換とする。`CredentialStore` の atomic write・best-effort 記録（usage.jsonl の contract と同じ思想）。catalog error は `record_error`/`errors.push` の `compositionErrors` 相当 channel に統一し、CLI 表面への露出は `io.stderr` の `warning:` 行 + `--quiet` gate とする（`run` の `model_line`・`usage-log:` と同じ mechanism）。
- Patterns to avoid: resolved `CatalogModel` を file に保存すること（fallback/defaults 解決の二重化・schema 連鎖変更）。`run`/`models`/`Runtime::create` から `devin` CLI や network を呼ぶこと。plugin crate（`cheaper-provider-devin`）側に永続化 policy を実装すること（runtime 所有）。pi の `models-store.json`/`pi/` 配下の file を読むこと。
- Dependencies or configuration: 新規依存なし（`serde`/`serde_json` 既存）。env var・config key の追加なし。
- Migration steps: 既存ユーザーへの破壊的変更なし。file 不在は bootstrap fallback、login 成功が初回の snapshot 生成点。credential・models.json は不変。

## Verification

- [ ] login（fake runner / 実 `devin` CLI）後に `<userConfigDir>/providers/<id>/catalog.json` が `version:1` envelope で atomic に生成され、別 process の `Runtime::create` で persisted model が `get_model`/`get_available`/`resolve_explicit_model` 経由で到達可能になることを確認（issue #81 の再現条件: login → 別 process `cheaper models`/`run --model devin/<refreshed-id>`）。
- [ ] catalog.json 欠落・version 不一致・`provider` 不一致・schema error・空 `models` の各ケースで bootstrap `registration.models` へ fallback し、欠落以外は catalog error に記録されることを unit test で確認。
- [ ] credential 無し refresh、空 definitions、または bootstrap `models` と等価な definitions が既存 `catalog.json` snapshot を上書きしないこと（persist guard — 比較基準は注入前の plugin 宣言 bootstrap list で、persisted snapshot 注入後の process で bootstrap 等価の返却が persist されないことも併せて確認）を unit test で確認。
- [ ] `providers.devin.models[]` に plugin 未定義 id を追加すると catalog に現れ、`api`/`baseUrl` 省略 def が plugin 宣言値で解決されること、plugin 定義 id との衝突 def が plugin 優先 + catalog error になることを unit test で確認。`plugin.models` 空の fake plugin provider でも同じ merge が効くことを併せて確認。
- [ ] plugin provider で invalid user def（baseUrl 解決不能・非正 contextWindow）がその def のみ skip + catalog error となり、provider・plugin models が存続することを unit test で確認。
- [ ] `refresh_plugin_models` による live 差し替え後も user-added model が保持され、compose 時に plugin id 衝突で skip された user def が refresh で plugin id 消滅後に effective になること、refresh で plugin 定義 id になった場合は plugin 優先になることを unit test で確認。
- [ ] refresh で差し替わった defs の `headers` が `model_header_sources` 再構築を経て `model_headers` 経由で request に届くことを unit test で確認。
- [ ] catalog error が `models`/`run`/`auth status`/`auth login` の stderr に `warning:` 行として観測可能で（`auth login` は login 中の refresh/persist 失敗の `record_error` が同一 run で emit されることを含む）、`--quiet` で抑制され、envelope `result` shape・`schemaVersion`・exit code が不変であることを CLI test で確認。
- [ ] refresh/persist の失敗が login を失敗させず catalog error に記録されること（既存 F13 契約の維持）。
- [ ] builtin/custom provider の merge 規則・provider drop 契約・`models`/`run`/`auth` の envelope `schemaVersion:1`・exit code が変化していないことを既存 test + parity golden で確認。
- [ ] `mise run check`（cargo test/clippy/fmt + `node scripts/check-doc-ids.mjs` 等）が green。

## Pros and Cons

### 決定 1 Option A: runtime-owned persisted snapshot（採用）

- Good, because login 1 回の refresh が process 境界を越えて全 command に効き、offline でも動く。file 契約は `credentials.json`/`models.json`/`usage.jsonl` と同 layer で一貫する。
- Good, because `providers/<id>/catalog.json` は `plugin_dir` の既存境界に乗り、provider 追加・隔離・削除が自然に拡張できる。
- Bad, because login 間は stale になり得る（TTL/手動 refresh を v1 に入れない判断の裏面）。
- Bad, because 新しい file 契約（schema evolution の責任）を 1 つ増やす。

### 決定 1 Option B: lazy refresh at startup（不採用）

- Good, because 常に最新 catalog で、新 file 契約が要らない。
- Bad, because `run`/`models` ごとに `devin` subprocess（timeout 20s）+ network が走り、offline/timeout で起動が遅延・失敗し得る — one-shot CLI 契約と両立しない。

### 決定 1 Option C: persisted + TTL/background hybrid（v1 不採用）

- Good, because 鮮度と offline 耐性を両立し得る。
- Bad, because one-shot process には background refresh の実装先がなく、TTL refresh は stale 時に Option B の問題を読み取り経路へ再導入する。`fetchedAt` を残して将来拡張余地に留める。

### 決定 1 Option D: plugin-owned persistence（不採用）

- Good, because runtime contract を変えず plugin 内に閉じられる。
- Bad, because path/envelope/atomicity/鮮度の policy が plugin ごとに複製され、refresh seam を所有する runtime との責務分割が不自然になる。

### 決定 2 Option γ: additive merge + plugin api/baseUrl fallback + per-def error（採用）

- Good, because plugin provider への追加経路が開き、「`models[]` = 追加・`modelOverrides` = mutate」の責務分離が保たれる。invalid def が provider を壊さない。
- Bad, because port 済み実装で観測される `apply_extension` の replace semantics との乖離（defect 修正として正当化済み — upstream TS parity は未検証の前提）。merge 規則が 1 つ増える。

### 決定 2 Option α: upsert merge（不採用）

- Good, because 最大の自由度（plugin 定義 id も user が書き換えられる）。
- Bad, because `models[]`/`modelOverrides` の責務が重複し、user def が plugin id を silently shadow して refresh 後の挙動が予測しにくい。

### 決定 2 Option β: replace 維持 + catalog error（不採用）

- Good, because 現行 contract 最小変更で診断可能性は回復する。
- Bad, because user の追加経路が残り issue #82 を解消しない — Defect 1 修正と併せても追加不能な構造が残る。

### 決定 3 Option A: stderr diagnostic（採用）

- Good, because envelope・exit code 契約を変えずに spec Req 4 の観測可能性を満たし、既存の stderr diagnostic mechanism（`--quiet`・`model_line`・`usage-log:`）を再利用できる。
- Bad, because stderr は非構造化テキストで、machine consumer が error を parse したい場合は後続の envelope `warnings` field（defer 済み）を待つ必要がある。

### 決定 3 Option B: envelope `warnings` field（defer）

- Good, because catalog error が構造化され parent agent が機械可読に扱える。
- Bad, because `result` shape の変更であり、envelope 不変契約との整合設計が必要になる — v1 の scope に入れず別 chain で扱う。

## More Information

- 上流 discovery: `DISC-o3GbG6FWEBRFcbNBaslikh`（`docs/discovery/plugin-catalog-refresh-persistence-and-user-models.md`）— 2 defect の実コード・実測検証と判断軸。
- GitHub issue #81 / #82 — それぞれ Defect 1 / Defect 2 の report。
- smoke 記録: `docs/tasks/phase-3-port-runtime-adapter-providers.md`（login → catalog refresh → fallback 4 件のみの観測）。
- IR: `docs/impl/ir/phase-3-port-runtime-adapter-providers.md` — F13（`plugin_catalog` handle + login 後 refresh）と `models-store.json`/`withRemoteCatalog` v1 非実装の記録。本 ADR は後者を cheaper-owned snapshot として一部改訂する。
- spec: `docs/specs/plugin-catalog-persistence-and-user-models.md`（SPEC-034CKp34AcjKtAYjnfjXU4）— 要件本体（本 ADR はその設計決定を固定する）。`relations.related` で相互リンク。
- 制約元 ADR: `provider-only-registration-via-explicit-bundled-plugin-contract.md`（plugin contract）、`migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`（`<userConfigDir>/pi/` 非読取・Pi state 非移行）、`devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md`（plugin 隔離 dir・`devin models list` 委譲）、`pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md`（superseded だが pi file 非読取方針の由来）。
- 実コード: `crates/cheaper-runtime/src/catalog/mod.rs`・`src/auth/service.rs`・`src/plugin.rs`・`src/runtime.rs`・`src/auth/store.rs`、`crates/cheaper-provider-devin/src/plugin.rs`・`src/models.rs`、`crates/cheaper-core/src/model/resolve.rs`。

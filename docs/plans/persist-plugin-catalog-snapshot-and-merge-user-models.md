---
id: "PLAN-034COkvLdOXz9j7HzLFGEK"
type: "plan"
status: "approved"
title: "Persist plugin catalog snapshot and merge user models on plugin providers"
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
    - "docs/specs/plugin-catalog-persistence-and-user-models.md"
    - "docs/designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
  implemented-by:
    - "docs/tasks/persist-plugin-catalog-and-merge-user-models.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-catalog-persistence-and-user-models.md"
    - "docs/designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
    - "docs/adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
    - "docs/discovery/plugin-catalog-refresh-persistence-and-user-models.md"
  derived-by:
    - "docs/tasks/persist-plugin-catalog-and-merge-user-models.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-catalog-persistence-and-user-model-merge.md"
  references: []
  defers: []
  deferred-by: []
---

# Persist plugin catalog snapshot and merge user models on plugin providers Implementation Plan

## Goal

approved design `DESIGN-034CM9o41G2FjaP5cV125c` をそのままコードへ落とす:
plugin `refreshModels` の結果を `<userConfigDir>/providers/<id>/catalog.json`
へ永続化し後続 process の catalog 構築で使えるようにする(issue #81)、
plugin provider に対する user `providers.<id>.models[]` を additive merge
で残す(issue #82)、`CatalogStore::errors()` を `warning:` 行として
`models`/`run`/`auth status`/`auth login` の stderr へ surface する
(spec Req 4)。

## Scope

1 つの実装ストリーム(catalog schema/compose → persist → CLI surface の
shared critical path を持つ単一変更)。分割しない。境界は design の
Scope に一致する。

- **In scope**: `catalog.json` の runtime-owned snapshot(write =
  `refresh_plugin_catalog`、read = `Runtime::create`)、plugin provider での
  user `models[]` additive merge(compose・refresh 両経路同一規則)+
  `api`/`baseUrl` fallback chain、`model_header_sources` の refresh 時
  再構築、catalog error の CLI stderr surfacing(`--quiet` で抑制)。
- **Out of scope**: lazy/TTL refresh、logout 時の snapshot 削除、
  `<userConfigDir>/pi/` の読み書き、`CredentialStore::write_file` の
  `#[cfg(windows)]` 事前 remove workaround の是正、Windows devin-CLI
  child env 隔離(issue #80)、envelope `warnings` field 等の構造化
  surface、spec が列挙しない command への warning 追加。

## Upstream Documents

- `docs/specs/plugin-catalog-persistence-and-user-models.md`
  (SPEC-034CKp34AcjKtAYjnfjXU4 — Req 1-6・Acceptance Criteria)
- `docs/adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md`
  (ADR-034CKnwOWyYssJ1VZR9spo — Option A + γ + 3A、envelope 検証・
  persist guard・merge 規則・surfacing の確定決定)
- `docs/discovery/plugin-catalog-refresh-persistence-and-user-models.md`
  (DISC-o3GbG6FWEBRFcbNBaslikh — defect の実測 evidence)

## Design Inputs

- `docs/designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md`
  (approved) — 本 plan の唯一の構造源。`Components and Boundaries` の各
  module 契約、`Error Paths` 表、`Verification Notes` の test 一覧をそのまま
  実装順序・検証へ写像する。
- 設計上の normative 注意:
  - snapshot 注入は `registration.models` を差し替えない — persist guard
    (c) の比較基準(`plugin_bootstrap_defs`)を注入前 bootstrap に固定する
    ため、`CatalogStore::build` の別引数で渡す。
  - persist は `refresh_plugin_models` が `Ok(())` を返した場合のみ —
    変換不能 defs の snapshot 化は自己修復不能にする。
  - atomic write は temp+rename、remove-first しない
    (`CredentialStore::write_file` の Windows 事前 remove は複製しない)。
  - plugin provider での provider-level 検証(oauth ⇒ baseUrl・
    「must specify …」)は今日と同一 semantics のまま維持する
    (緩めない・fatal class を増やさない)。

## File Map

### `cheaper-runtime`

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-runtime/src/catalog/schema.rs` | `ModelInput`・`ThinkingLevelMap`・`ModelCostTierJson`・`ModelCostJson`・`ModelDefinitionJson` に `Serialize` derive 追加、Option field の `skip_serializing_if` 整備 | `ProviderConfigJson`・`ModelOverrideJson` には付けない(YAGNI) |
| `crates/cheaper-runtime/src/catalog/persisted.rs` | 新設。`PluginCatalogFile` envelope + `load_plugin_catalog`/`write_plugin_catalog`(temp+rename atomic) | `catalog` module 配下 `pub mod persisted` |
| `crates/cheaper-runtime/src/catalog/mod.rs` | `CatalogStore::build` 第 4 引数、`compose_provider` の plugin 分岐再構成、`merge_user_model_defs`・`collect_model_header_sources` helper、`ProviderEntry` 新 field、`refresh_plugin_models` re-merge + header 再構築、`dynamic_models` → `RefreshedCatalog` 化、`CatalogStore::model_headers` | 本変更の主戦場 |
| `crates/cheaper-runtime/src/plugin.rs` | `ProviderPluginContext::catalog_path` helper | `plugin_dir` 規約の集約(表現ドリフト回避) |
| `crates/cheaper-runtime/src/runtime.rs` | `Runtime::create` で snapshot load → `build` 注入 + `record_error` | read error は build 後に記録 |
| `crates/cheaper-runtime/src/auth/service.rs` | `persist_plugin_catalog`(`pub(crate)`、guard a/b/c)+ `refresh_plugin_catalog` への配線 | guard test は inline `#[cfg(test)]` |
| `crates/cheaper-runtime/tests/plugin.rs` | FakeCatalog/FakeAuth seam での永続化・merge・refresh 再評価 integration test + `FakePlugin` の `base_url`/`models` 可変化 | `CHEAPER_CONFIG_DIR` ではなく `Runtime::create` へ tempdir を渡す既存 pattern。`FakePlugin::register` は `base_url: "http://fake.local"`・`models: [fake-1]` を hardcode する(`tests/plugin.rs:108-134`)ため、design-mandated test(変換不能 defs の非 persist・`plugin.models` 空での merge・`base_url:""` の両経路 skip)には `base_url`/`models` を受け取る constructor/builder が必要。現在の構築は全て struct literal(`tests/plugin.rs:141-149`・`184-190`・`plugin_with_catalog` `279-290`)なので、field 追加は 3 site を壊す — `FakePlugin::new(auth, catalog, base_url, models)` 等の constructor + default 値で既存 test を追随させる |
| `crates/cheaper-runtime/tests/catalog.rs`・`tests/auth.rs` | `CatalogStore::build` 新 signature への機械的追随(`&BTreeMap::new()`)+ merge 系 test | 既存呼び出し一括更新 |

### `cheaper-cli`

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/cli.rs` | `emit_catalog_warnings(errors, sink: &mut dyn FnMut(&str), quiet)` 共通 helper | sink は closure(`io.stderr.write_str`・`AuthTerminal::write` 両用) |
| `crates/cheaper-cli/src/models.rs` | `create_runtime` 直後に emit(`get_available` の `?` より先) | `models.rs:78` 付近 |
| `crates/cheaper-cli/src/run.rs` | `create_runtime` 直後に emit(`run_worker` より先) | `run.rs:404` 付近 |
| `crates/cheaper-cli/src/auth.rs` | `execute_with` で early emit + `auth login` Ok 後の差分 emit(cursor 方式) | `auth.rs:525-617` |
| `crates/cheaper-cli/tests/auth.rs`・`tests/models.rs`・`tests/run.rs` | `warning:` 行・`--quiet` 抑制・envelope 不変の stderr assertion | `VecSink`/`ScriptedTerminal`/`run_cli` seam |

## Implementation Sequence

### Phase A — persistence plumbing

- [ ] 1. `schema.rs` の `ModelInput`・`ThinkingLevelMap`・`ModelCostTierJson`・`ModelCostJson`・`ModelDefinitionJson` に `Serialize` を derive し、Option field に `skip_serializing_if = "Option::is_none"` を揃える(`ThinkingLevelMap` は既存 attribute が有効化される)
- [ ] 2. `cargo check -p cheaper-runtime` で compile を確認する
- [ ] 3. `catalog/persisted.rs` を新設し `PluginCatalogFile`・`PLUGIN_CATALOG_FILE_VERSION`・`load_plugin_catalog`・`write_plugin_catalog` を実装する(load: `NotFound` → `Ok(None)`、parse/version/provider 不一致/空 `models` → `Err`; write: `create_dir_all` + `.catalog.json.tmp-<pid>-<nanos>` temp + `fs::rename`、remove-first 無し・失敗時 temp cleanup)。`catalog/mod.rs` に `pub mod persisted` を追加する
- [ ] 4. `persisted.rs` の unit test を書く(write→load round-trip・malformed・version 不一致・provider 不一致・空 `models` → Err・不在 → `Ok(None)`・unknown key 読み飛ばし)を `cargo test -p cheaper-runtime persisted` で緑にする
- [ ] 5. `plugin.rs` に `ProviderPluginContext::catalog_path` を追加する
- [ ] 6. Phase A を commit する(`feat: add plugin catalog snapshot persistence module`)

### Phase B — catalog merge machinery

- [ ] 7. `ProviderEntry` に `plugin_user_defs`・`config_api`・`config_base_url`(module-private)と `plugin_bootstrap_defs`(`pub(crate)`)を追加し、`compose_provider` で値を埋める
- [ ] 8. `apply_models_json` から provider-level 検証(a)と base-model overlay(b)を共有 helper(`validate_provider_config`・`apply_provider_overlay` 相当)へ抽出し、custom-provider 経路の挙動不変を既存 test で確認する(refactor のみ)
- [ ] 9. `merge_user_model_defs` helper を実装する(post-extension list と衝突しない user def のみ `model_from_json` で解決して末尾追加。fallback chain: `api` = def → config → plugin、`baseUrl` = def → config → plugin → `find_model_defaults`。非空 filter は helper 内の単一 site。衝突・解決不能は per-def skip + `errors.push`)
- [ ] 10. `apply_extension` に `persisted: Option<&[ModelDefinitionJson]>` を追加し、effective defs `persisted.unwrap_or(&plugin.models)` を `extension_models` へ渡す。空 guard は `plugin.models.is_empty()` ではなく effective defs へ移す — `plugin.models` が空でも persisted snapshot があれば `extension_models` が走り、effective defs が空のときのみ従来の baseUrl-override 分岐(`catalog/mod.rs:482-494`)を踏む(design `compose_provider` 項の順序 2 と同一規則)
- [ ] 11. `compose_provider` の plugin provider 分岐を再構成する(検証+overlay → `apply_extension` → `merge_user_model_defs` → `modelOverrides`)。signature に `persisted: Option<&[ModelDefinitionJson]>`(step 10 の `apply_extension` へそのまま渡す)と `errors: &mut Vec<String>` の 2 引数を追加し、merge error は bare message で `errors` へ push する
- [ ] 12. `CatalogStore::build` に第 4 引数 `persisted_plugin_models: &BTreeMap<String, Vec<ModelDefinitionJson>>` を追加し、provider loop から `persisted_plugin_models.get(id)` と `errors` vec を `compose_provider` へ渡す。builtin fallback の recursive 呼び出し(`catalog/mod.rs:848` の `compose_provider(id, Some(meta), &base_models, None, None)`)にも新引数を forward する(`plugin: None` のため merge は走らないが、signature を揃え `errors` は同一 vec を共有する)。merge error は `Provider "<id>": ` prefix で `errors` へ集約する — `compose_provider` が後段で fatal Err を返して provider drop される場合も含め、compose 結果の成否に関わらず集約する(fatal Err の prefix 規則と同じ)
- [ ] 13. `CatalogStore::build` の全 call site(`runtime.rs`・`tests/catalog.rs`・`tests/auth.rs` の `&[]` 呼び出し)を新 signature に追随する
- [ ] 14. compose 時の inline header 構築(`mod.rs:616-654`)を `collect_model_header_sources` helper へ抽出し、compose は effective defs(persisted ?? `plugin.models`)を渡す
- [ ] 15. `dynamic_models` の value を `{ models, model_header_sources }` の struct(`RefreshedCatalog` 等)へ変更し `models_of` を調整、`CatalogStore::model_headers(provider_id, model_id)`(refresh 済み優先・`entry.model_headers` fallback)を追加する
- [ ] 16. `refresh_plugin_models` に merge 再実行(`merge_user_model_defs` 同一 fallback chain・errs は `Provider "<id>": ` prefix で `record_error`)と header map 再構築を挟み、`replace_plugin_models` で model+header を一緒に差し替える
- [ ] 17. `cargo test -p cheaper-runtime` で既存 test の回帰が無いことを確認し Phase B を commit する(`feat: merge user model definitions into plugin provider catalogs`)

### Phase C — persist integration

- [ ] 18. `service.rs` に `persist_plugin_catalog`(`pub(crate)`)を実装する(guard a: `has_credential`、b: definitions 非空 — 空は skip + `record_error`、c: `definitions != entry.plugin_bootstrap_defs`; 全通過で `write_plugin_catalog(ctx.catalog_path(&entry.id), …)`、write 失敗は `record_error` のみ)
- [ ] 19. `refresh_plugin_catalog` を `refresh_plugin_models` の `Ok(())` 継続でのみ `persist_plugin_catalog` を呼ぶ形へ改める(`Err` → `record_error` のみ、persist しない)
- [ ] 20. `persist_plugin_catalog` の guard 単体 test を `service.rs` の `#[cfg(test)]` inline module に書く(空 definitions・bootstrap 等価 definitions・`has_credential=false` で既存 `catalog.json` を上書きしない)。`ProviderEntry` は private field(`model_header_sources`・`model_overrides`・step 7 の module-private field)を持つため `service.rs` から literal 構築できない — test は fake `PluginProviderRegistration` を渡した `CatalogStore::build(...)` で catalog を組み立て `catalog.provider(id)` で entry を取得する
- [ ] 21. `runtime.rs` の `Runtime::create` に snapshot load loop を追加する(`reg.catalog.is_none()` は skip、`load_plugin_catalog` の `Ok(Some)` → persisted map、Err → `snapshot_errors` → build 後 `record_error`)
- [ ] 22. Phase C を commit する(`feat: persist refreshed plugin catalogs and load them at runtime create`)

### Phase D — CLI surfacing

- [ ] 23. `cli.rs` に `emit_catalog_warnings` を実装する(`warning: <msg>\n` を sink closure へ、`quiet` で抑制)
- [ ] 24. `models.rs`(`create_runtime` 直後・`get_available` より先)と `run.rs`(`create_runtime` 直後・`run_worker` より先)に emit を挿入する
- [ ] 25. `auth.rs::execute_with` に early emit(catalog/auth 取得直後・action dispatch 前、emit 済み件数 cursor 保持)と `auth login` `Ok` 後の差分 emit(`errs[emitted..]`)を追加する
- [ ] 26. Phase D を commit する(`feat: surface catalog errors as stderr warnings in models, run, and auth commands`)

### Phase E — behavior tests

- [ ] 27. `tests/plugin.rs` に永続化 e2e を追加する(login → `providers/fake/catalog.json` 生成 → 別 `Runtime::create` で persisted model が `get_supported_model`/`get_available` に見える。`extension_models` 変換不能 defs は persist されない。logout で `catalog.json` を消さないことに加え、logout 後の `Runtime::create` も同 snapshot を注入し続け persisted model が catalog に残ることを assert する)
- [ ] 28. merge の integration test を追加する(新 id 追加・`api`/`baseUrl` 省略の plugin 値 fallback・post-extension list 衝突 skip+error・invalid def skip+provider 存続・`plugin.models` 空でも merge が効く・refresh 後の衝突再評価・refresh header 再構築・空 `base_url` filter 両経路・oauth-without-baseUrl 検証維持)。`plugin.models` 空・`base_url:""` の case は `FakePlugin` の `base_url`/`models` 可変化(File Map 参照)を先行させる
- [ ] 29. CLI surfacing test を追加する(malformed `catalog.json` で `models`/`auth status`/`auth login`/`run` の stderr に `warning:` 行、`--quiet` 抑制、envelope `result` shape・exit code 不変、`execute_with` の early emit は `ScriptedTerminal` capture で assert)。`auth login` の post-login 差分 emit も必須検証とする — fake plugin の `FakeCatalog` で `auth.login` 中の refresh error を注入し、login `Ok` 後の `errs[emitted..]` emit が同一 run で `warning:` 行を `ScriptedTerminal` capture へ出すことを `execute_with` 経由で検証する(`run_cli` は非 TTY で対話 prompt 拒否により refresh 到達前に失敗するためこの経路では検証できない — `tests/auth.rs:343-380` の既存挙動)
- [ ] 30. `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli` と `mise run check` を通し Phase E を commit する(`test: cover plugin catalog persistence and user model merge`)

## Dependencies and Risks

- **依存**: 上流 doc は全て確定済み(design approved)。外部依存の追加なし
  (`serde`/`serde_json`/`std::fs`/`tempfile` は既存)。
- **Critical path**: Phase A → B → C → D → E の直列。B の `ProviderEntry`
  field と `build` signature は C(runtime/service)と D より前に必要。
- **Risk 1 — `CatalogStore::build` signature 変更の波及**(data-loss 無し・
  compile error で機械検出可): `tests/catalog.rs`・`tests/auth.rs`・
  `runtime.rs` の呼び出し一括更新で対処。
- **Risk 2 — merge helper の fallback filter drift**: `config.base_url` 空
  文字列扱いを compose/refresh 両 call site で複製すると規則がずれる。
  非空 filter は `merge_user_model_defs` 内部の単一 site に閉じ込める
  (design normative)。
- **Risk 3 — persist guard (c) の residual staleness**: live refresh が
  bootstrap 等価 defs を返し stale `catalog.json` が残るケースで write が
  skip される(ADR/design で受容済み。`fetchedAt` で診断可能)。
- **Risk 4 — `Option<Option<String>>` round-trip 縮退**: `ThinkingLevelMap`
  の明示 null が serialize→load で key 不在へ畳まれる(design の accepted
  limitation。消費側は等価扱いのため機能差なし)。
- **Risk 5 — emit 順序**: `models`/`run` では emit を後続 fallible 処理より
  先に置かないと warning が欠落する。call site の位置を design どおりに
  固定し test で assert する。
- **Risk 6 — `emit_catalog_warnings` の対象範囲**: emit は
  `catalog.errors()` の全件を出すため、snapshot 由来の error だけでなく
  models.json parse error 等の既存 error class も新たに stderr の
  `warning:` 行として surface する(error class の追加ではなく surface の
  新設)。既存 CLI test が green のままなのは clean tempdir で catalog
  error が 0 件になるためであり、error を仕込んだ fixture では warning
  行が stderr assertion に影響し得る点を留意する。

## Task Breakdown

実行単位は `docs/tasks/persist-plugin-catalog-and-merge-user-models.md`
(単一 task doc、内部で T1-T6 に順序付け)。Phase との対応:

| ID | 範囲 | 内容 | 依存 |
| --- | --- | --- | --- |
| T1 | schema + persisted.rs | step 1-6 | — |
| T2 | catalog merge machinery | step 7-17 | T1(主に同一 `catalog/` file 群の変更文脈・作業順序の共有 — Serialize derive の実 consumer は T3 の persist であり T2 自身は serialize しない) |
| T3 | persist integration | step 18-22 | T1 + T2(`plugin_bootstrap_defs`・`catalog_path`・`write_plugin_catalog`) |
| T4 | CLI surfacing | step 23-26 | T2(`errors` 集約の確定後 — emit は `catalog.errors()` 読み出しのみで T3 に依存しない) |
| T5 | runtime + integration tests | step 27-29 | T3 + T4 |
| T6 | full verification | step 30 | 全 task |

## Verification

- [ ] `mise exec -- cargo test -p cheaper-runtime persisted` が `persisted.rs` unit test(round-trip・各 Err 分岐・`Ok(None)`)を緑にする
- [ ] `mise exec -- cargo test -p cheaper-runtime` で既存 catalog/auth/plugin test が全パス(`build` signature 追随後)
- [ ] fake plugin + tempdir で login → `<dir>/providers/fake/catalog.json` 生成 → **新** `Runtime::create` で persisted model が `get_supported_model`/`get_available` に見える(spec AC: 別 process `models`/`run` の根幹)
- [ ] persist guard: 空 definitions・bootstrap 等価 definitions・`has_credential=false` で既存 `catalog.json` を上書きしない
- [ ] merge: user def の新 id が catalog に残り resolution 可能 / `api`・`baseUrl` 省略 def が plugin 値で解決 / id 衝突 → list 優先 + error 記録 / invalid def → per-def skip + provider 存続 / `plugin.models` 空でも merge 有効
- [ ] `refresh_plugin_models` 後も user-added model が保持され、衝突判定が refresh ごとに再評価される(plugin id 消滅 → user def effective、id 復活 → list 優先 + error)
- [ ] refreshed defs の `headers` が `CatalogStore::model_headers` に反映される
- [ ] malformed `catalog.json` で `models`・`auth status`・`auth login`・`run` の stderr に `warning:` 行が出る、`--quiet` で抑制、envelope `result` shape・exit code 不変(`auth status <dropped-plugin>` では `warning:` 行と `unknown provider` envelope error が併存)
- [ ] `auth login` の post-login 差分 emit: fake plugin の `FakeCatalog` で `auth.login` 中の refresh error を注入し、login `Ok` 後の `errs[emitted..]` emit が同一 run の `ScriptedTerminal` capture に `warning:` 行として出ることを `execute_with` で検証する(`run_cli` 非 TTY 経路は refresh 到達前に失敗するため不適)
- [ ] `mise run check`(cargo test/clippy/fmt + check-doc-ids)が緑

## Rollback Strategy

- 全変更は `crates/` 内の Rust コード + test に留まり、外部 contract
  (envelope・exit code・`models.json` schema・plugin trait)は不変。
  `git revert` で撤去可能。
- `catalog.json` は新規 file で、削除しても bootstrap fallback へ戻るだけ
  (破壊的 migration 無し)。rollback 後に残存 file があっても旧コードは
  読まない。
- persist guard 誤判定(正当な refresh が persist されない)が発覚した場合は
  `catalog.json` の手動削除 + 再 login で回復する。恒久対応は guard 条件を
  `service.rs` のみで修正可能(単一 site)。

## Open Questions / Gaps

- `RefreshedCatalog` の型名・`collect_model_header_sources` の
  signature 詳細は実装時に既存命名 convention に合わせる(design は
  `RefreshedCatalog` 等と例示)。
- 抽出する共有 helper の命名(`validate_provider_config`・
  `apply_provider_overlay` 相当)は既存 `apply_models_json` の構造に
  倣って実装時に確定する。
- emit の stderr noise(login 時に同一 merge 衝突 message が 2 行出得る)
  は design で dedup しないことが確定済み — 追加判断不要。

## Self-Review

- [x] Every spec requirement maps to a task(Req 1/2 → T1+T3+T5、Req 3/5 → T2+T5、Req 4 → T4+T5、Req 6 → 不変確認として Verification に明示)
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: 直列 critical path(T1→T2→T3/T4→T5→T6)のため inline
  execution を基本とする。T1(schema+persisted.rs)と T4(CLI helper)は
  並行可能だが、依存の少ない順に同一 agent で進める方が衝突が少ない。
- Independent work streams: T4(CLI surfacing)は T2 の `errors` 集約確定後、
  T3 と並行可。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 本 plan は review gate 待ち(`status: draft`)。task doc は plan 承認を
  前提条件として先に作成済み(TASK-DOC-GATE-001 は graph delegate の
  指示により同時生成 — plan の status が `approved` になってから task を
  開始する)。
- レビュー重点 1(normative 遵守): snapshot 注入が `registration.models`
  を差し替えないこと、persist が `refresh_plugin_models` の `Ok(())`
  継続に限定されること、atomic write が remove-first しないこと。
- レビュー重点 2(merge 規則): fallback chain・衝突判定の対象
  (post-extension list 全 id)・非空 filter の単一 site 化が design と
  一致しているか。
- レビュー重点 3(surfacing): `execute_with` の early emit + post-login
  差分 emit の cursor 方式が design どおりか、emit が fallible 処理より
  先に置かれているか。
- レビュー重点 4(検証維持): plugin provider の provider-level 検証
  (oauth ⇒ baseUrl・「must specify …」)が custom と同一 semantics の
  まま残っているか。
- 上流 doc: spec `SPEC-034CKp34AcjKtAYjnfjXU4`・ADR
  `ADR-034CKnwOWyYssJ1VZR9spo`・design `DESIGN-034CM9o41G2FjaP5cV125c`。

---
id: "TASK-034COkOggSVxC84DkUUem5"
type: "task"
status: "todo"
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
    - "docs/plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  implemented-by: []
  depends-on:
    - "docs/plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-catalog-persistence-and-user-model-merge.md"
  references: []
  defers: []
  deferred-by: []
---

# Persist plugin catalog snapshot and merge user models on plugin providers

## Work

plan `docs/plans/persist-plugin-catalog-snapshot-and-merge-user-models.md`
(PLAN-034COkvLdOXz9j7HzLFGEK)の実行単位。plugin `refreshModels` の結果を
`<userConfigDir>/providers/<id>/catalog.json` へ永続化し(issue #81)、
plugin provider の user `models[]` を additive merge で残し(issue #82)、
catalog error を `warning:` 行として stderr へ surface する。

plan の Implementation Sequence Phase A-E が本 task の作業単位。順序と
依存関係は以下のとおり(各項目は semantic commit の単位):

- **T1 — schema derives + persisted.rs**(plan step 1-6):
  `catalog/schema.rs` の `ModelInput`/`ThinkingLevelMap`/
  `ModelCostTierJson`/`ModelCostJson`/`ModelDefinitionJson` に `Serialize`
  derive(`skip_serializing_if` 整備)→ `catalog/persisted.rs` 新設
  (`PluginCatalogFile` envelope・`load_plugin_catalog`・temp+rename
  atomic `write_plugin_catalog`)→ persisted unit test(round-trip・
  malformed/version/provider 不一致・空 models → Err、不在 → `Ok(None)`)
  → `plugin.rs` の `ProviderPluginContext::catalog_path`。
- **T2 — catalog merge machinery**(plan step 7-17、T1 に依存):
  `ProviderEntry` 新 field(`plugin_user_defs`/`config_api`/
  `config_base_url`/`plugin_bootstrap_defs`)→ `apply_models_json` の
  検証(a)/overlay(b) helper 抽出 → `merge_user_model_defs` →
  `apply_extension` の `persisted` param → `compose_provider` の plugin
  分岐再構成 + `errors: &mut Vec<String>` → `CatalogStore::build` 第 4
  引数 `persisted_plugin_models` + 全 call site 追随(`runtime.rs`・
  `tests/catalog.rs`・`tests/auth.rs`)→ `collect_model_header_sources`
  helper 化 → `dynamic_models` の `RefreshedCatalog` 化 +
  `CatalogStore::model_headers` → `refresh_plugin_models` の re-merge +
  header 再構築。
- **T3 — persist integration**(plan step 18-22、T1+T2 に依存):
  `service.rs` の `persist_plugin_catalog`(guard a/b/c)+
  `refresh_plugin_catalog` で `refresh_plugin_models` `Ok(())` 継続のみ
  persist → guard 単体 test(`service.rs` inline `#[cfg(test)]`)→
  `runtime.rs` の snapshot load loop(`reg.catalog.is_none()` skip・
  Err → build 後 `record_error`)。
- **T4 — CLI surfacing**(plan step 23-26、T2 の `errors` 集約確定後で
  T3 と並行可): `cli.rs` の `emit_catalog_warnings`(closure sink) →
  `models.rs`/`run.rs` の `create_runtime` 直後 emit → `auth.rs::
  execute_with` の early emit + post-login 差分 emit(cursor 方式)。
- **T5 — behavior tests**(plan step 27-29、T3+T4 に依存):
  `tests/plugin.rs` の永続化 e2e(login → `catalog.json` → 別
  `Runtime::create`、変換不能 defs 非 persist、logout で snapshot 残存)
  + merge/refresh 再評価 test → CLI surfacing test(malformed
  `catalog.json` の `warning:` 行・`--quiet` 抑制・envelope 不変、
  `ScriptedTerminal` capture)。
- **T6 — full verification**(plan step 30、全 task に依存):
  `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli` と
  `mise run check` の緑化。

依存関係: T1 → T2 → T3、T2 → T4、T3+T4 → T5 → T6。T4 は T3 と並行可。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/persist-plugin-catalog-snapshot-and-merge-user-models.md`
  (本 task が implement する plan — File Map・Implementation Sequence・
  Verification の正)
- `docs/specs/plugin-catalog-persistence-and-user-models.md`
  (SPEC-034CKp34AcjKtAYjnfjXU4 — Req 1-6・Acceptance Criteria)
- `docs/adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md`
  (ADR-034CKnwOWyYssJ1VZR9spo — Option A + γ + 3A の確定決定)
- `docs/designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md`
  (DESIGN-034CM9o41G2FjaP5cV125c、approved — module 契約・Error Paths・
  Verification Notes)
- `docs/test-specs/plugin-catalog-persistence-and-user-model-merge.md`
  (TSPEC-034COkGnjlrMwzT5pEaMzV — 本 task が satisfy する保証)

## Dependencies

- plan `PLAN-034COkvLdOXz9j7HzLFGEK` の `status` が `approved` 以上に
  なることが本 task の着手条件(TASK-DOC-GATE-001。plan は review gate
  待ちの `draft` で作成済み — gate 通過後に開始する)。
- 外部依存・他 task への依存は無い(design の upstream design は全て
  実装済み)。

## Affected Files

- `crates/cheaper-runtime/src/catalog/schema.rs` — `Serialize` derive
- `crates/cheaper-runtime/src/catalog/persisted.rs` — 新設
- `crates/cheaper-runtime/src/catalog/mod.rs` — compose/merge/refresh 再構成
- `crates/cheaper-runtime/src/plugin.rs` — `catalog_path`
- `crates/cheaper-runtime/src/runtime.rs` — snapshot load
- `crates/cheaper-runtime/src/auth/service.rs` — `persist_plugin_catalog`
- `crates/cheaper-cli/src/cli.rs` — `emit_catalog_warnings`
- `crates/cheaper-cli/src/models.rs`・`run.rs`・`auth.rs` — emit call site
- `crates/cheaper-runtime/tests/plugin.rs`・`catalog.rs`・`auth.rs`、
  `crates/cheaper-cli/tests/auth.rs`・`models.rs`・`run.rs` — test

## Done When

- [ ] plan の Implementation Sequence step 1-30 が全て完了し、各 Phase
      の semantic commit が積まれている
- [ ] 新規・既存 test が全て green(`persisted.rs` unit・guard・永続化
      e2e・merge/refresh 再評価・CLI surfacing・既存回帰)
- [ ] `catalog.json` 経由で別 `Runtime::create` が persisted model を
      解決し、user `models[]` が silent discard されず、catalog error が
      `warning:` 行で stderr に出る(TSPEC-034COkGnjlrMwzT5pEaMzV の
      Rules/Examples を満たす)
- [ ] envelope `schemaVersion: 1`・`result` shape・exit code・
      `models.json` schema・`<userConfigDir>/pi/` 非読取が不変

## Verification

- [ ] `mise exec -- cargo test -p cheaper-runtime` exits 0
      (persisted unit・guard・plugin merge・refresh test を含む)
- [ ] `mise exec -- cargo test -p cheaper-cli` exits 0
      (surfacing stderr assertion・envelope parity を含む)
- [ ] `mise run check` exits 0(cargo test/clippy/fmt + check-doc-ids)
- [ ] `grep -rn "catalog.json" crates/cheaper-runtime/src/plugin.rs` で
      file 名の組み立てが `ProviderPluginContext::catalog_path` の単一
      site に閉じていること、`persisted.rs` が path ではなく envelope
      契約(`PluginCatalogFile`・version 検証)のみを所有していることを確認

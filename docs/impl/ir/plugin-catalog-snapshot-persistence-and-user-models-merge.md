---
id: "IMPL-034CSeJIDJx6Es0oHOERBj"
type: "impl"
status: "completed"
title: "Plugin catalog snapshot persistence and user-models merge"
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
    - "docs/tasks/persist-plugin-catalog-and-merge-user-models.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Plugin catalog snapshot persistence and user-models merge

## Summary

plugin `refreshModels` の live catalog 結果を
`<userConfigDir>/providers/<id>/catalog.json` へ永続化し(issue #81)、
plugin provider の user `models[]` を additive merge し(issue #82)、
catalog error を `warning:` 行として stderr に surface する実装。
one-shot CLI プロセスでも login 時 refresh の catalog が参照可能に
なり、`devin models list` が返す `swe-2-*` 等の live model が
後続の `models`/`run` から解決できる。

## Context

- task `TASK-034COkOggSVxC84DkUUem5` / plan `PLAN-034COkvLdOXz9j7HzLFGEK`
  (approved)/ spec `SPEC-034CKp34AcjKtAYjnfjXU4` / ADR
  `ADR-034CKnwOWyYssJ1VZR9spo` / design
  `DESIGN-034CM9o41G2FjaP5cV125c` (approved)を実装。
- issues #81(plugin catalog 非永続)#82(plugin provider で user
  `models[]` が silent discard)。

## Implementation

### Changes

- `catalog/schema.rs`: `ModelInput`/`ThinkingLevelMap`/`ModelCostTierJson`/
  `ModelCostJson`/`ModelDefinitionJson` に `Serialize` derive +
  `skip_serializing_if`。
- `catalog/persisted.rs`(新設): `PluginCatalogFile` envelope
  (`{version:1, provider, fetchedAt, models[]}`)、`load_plugin_catalog`
  (不在 `Ok(None)`、malformed/version/provider 不一致・空 → `Err`)、
  `write_plugin_catalog`(temp+rename atomic、remove-first 無し)。
- `plugin.rs`: `ProviderPluginContext::catalog_path`。
- `catalog/mod.rs`: `ProviderEntry` に `plugin_user_defs`/`config_api`/
  `config_base_url`/`plugin_bootstrap_defs`/`extension_base_models` を
  追加。`apply_models_json` から `validate_provider_config` +
  `apply_provider_overlay` を抽出(plugin provider でも verbatim 適用)。
  `merge_user_model_defs`(additive、post-extension list の id が
  authoritative、per-def skip+error、非空 filter は helper 内単一 site)。
  `apply_extension` に `persisted` param(effective defs で empty 判定)。
  `CatalogStore::build` 第 4 引数。`RefreshedCatalog` で refresh 時に
  model+header を atomic swap。refresh でも同一 merge を再実行し、
  `extension_base_models` を defaults pool に使う。
- `auth/service.rs`: `persist_plugin_catalog`(guard: credential 有り・
  非空・bootstrap と非等価)+ `refresh_plugin_models` の `Ok` 継続のみ
  persist(変換不能 defs の persist → provider drop → 自己修復不可を防止)。
- `runtime.rs`: `reg.catalog.is_some()` な registration のみ snapshot
  load、Err は `record_error`。
- `cheaper-cli`: `emit_catalog_warnings`(`&mut dyn FnMut(&str)` sink)。
  `models`/`run` は `create_runtime` 直後、`auth` は `execute_with` 冒頭の
  early emit(`?` error path でも到達)+ login 後の cursor 差分 emit。
  `--quiet` で抑制。envelope/exit code 不変。

### Decisions During Implementation

- `load_plugin_catalog` は `PluginCatalogFile` envelope 全体を返す
  (design sketch の `Vec<ModelDefinitionJson>` からの意図的変更 —
  `fetched_at` を保持し将来の診断に備える)。
- 衝突 skip された user def の `headers` が残存 plugin model に
  注入される leak を修正 — merge は `Vec<&ModelDefinitionJson>` の
  merged defs を返し、header source にはそれのみを渡す
  (review finding、compose/refresh 両経路)。
- refresh 時 `extension_models` の defaults pool を compose 時の
  pre-extension base と同一化 — user def の `baseUrl` 借用 →
  persist → 次回 boot で provider drop の経路を遮断(review finding)。
- collision message は `Provider "<id>":` prefix 済みになるため
  message 本体から provider 名を除いた。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli` — 全 pass
  (runtime 129 + cli 全 suite、新規テスト 21+ 件)。
- `mise run check` — 全緑(biome/fmt/clippy -D warnings/version/
  doc-ids/cargo test --workspace/node tests 138/bundle smoke/
  check-apm-payload)。
- review-protocol: design 2 cycle + plan 1 cycle + implementation
  2 cycle、全 finding 修正済み・verdict ACCEPT WITH NON-BLOCKING NOTES。

## Risks

- guard (c) の非対称性: live refresh が bootstrap 等価を返す場合は
  stale snapshot が残る(fallback clobber 防止とのトレードオフ、
  design で accepted)。
- `{"oauth":"radius","baseUrl":...}` on plugin provider は従来どおり
  `RADIUS_API_KEY` env auth になる quirk を保持(検証は custom と
  verbatim 同一、新 fatal 無し)。
- orphan temp file(`.catalog.json.tmp-*`)は crash 時に残存し得る
  (reader は `catalog.json` のみ参照、無害)。

## Follow-ups

- issue #80: Windows Devin CLI isolation(本 chain のスコープ外)。
- `workflow run`/`usage --compare-model` の runtime 経路は catalog
  warning を emit しない(design で command 層限定として deferred)。

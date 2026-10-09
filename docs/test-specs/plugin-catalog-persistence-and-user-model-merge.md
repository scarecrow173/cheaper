---
id: "TSPEC-034COkGnjlrMwzT5pEaMzV"
type: "test-spec"
status: "approved"
title: "plugin catalog persistence and user model merge guarantees"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
  derived-by: []
  verifies:
    - "docs/specs/plugin-catalog-persistence-and-user-models.md"
    - "docs/adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin catalog persistence and user model merge guarantees

## Purpose

SPEC-034CKp34AcjKtAYjnfjXU4 / ADR-034CKnwOWyYssJ1VZR9spo(issue #81・#82)
の 2 defect 修正が守るべき振る舞いを固定する: (1) login で refresh した
plugin live catalog が `<userConfigDir>/providers/<id>/catalog.json` 経由で
**別 process** の catalog 構築に届くこと、(2) plugin provider の user
`providers.<id>.models[]` が silent に破棄されないこと、(3) catalog
error が stderr diagnostic として観測可能であること。

引退条件: snapshot 永続化方式(Option A)・merge 規則(γ)・surfacing
(3A)のいずれかが再設計された場合。`catalog.json` の envelope schema
が変わる場合は `version` bump 規則と共に本 spec の検証事例を見直す。

## Feature

plugin provider catalog の snapshot 永続化、user `models[]` additive
merge、および catalog error の stderr surface。

## Rules

### snapshot 永続化(issue #81 / spec Req 1-2)

- `refreshModels` が `Ok(Some(definitions))` を返した login は、
  `catalog.refresh_plugin_models` が `Ok(())` のときに限り
  `<userConfigDir>/providers/<id>/catalog.json` へ
  `{version:1, provider, fetchedAt, models:[ModelDefinitionJson]}` を
  atomic write(temp+rename、remove-first 無し)する。`refresh_models` が
  `Err`/`Ok(None)`、または `refresh_plugin_models` が `Err` のときは
  write しない(変換不能 defs の snapshot 化は自己修復を不可能にする)。
- write は persist guard (a) credential 有り refresh、(b) definitions
  非空、(c) `definitions != registration.models`(注入前 bootstrap)の
  3 条件を全て満たす場合のみ走る。空 definitions は skip +
  `record_error`。
- `Runtime::create` は `catalog` seam を持つ registration のみ snapshot
  を読む(`reg.catalog.is_none()` の残存 file は適用しない)。file 不在は
  silent `Ok(None)`。malformed・version 不一致・provider 不一致・空
  `models`・io error は file 全体を無視して bootstrap `plugin.models` へ
  fallback + `record_error`。unknown key は読み飛ばす。
- logout・credential 無効化時に snapshot は削除しない(stale snapshot は
  次回 login refresh で自己修復)。refresh 契機は login のみ —
  `run`/`models`/`Runtime::create` から subprocess/network を呼ばない。

### user `models[]` additive merge(issue #82 / spec Req 3・5)

- plugin provider の user `models[]` def は `apply_models_json` の
  per-def resolve loop を通らず、extension 適用後の model list に対する
  merge stage で `model_from_json` 解決される。`plugin.models` が空でも
  同一規則で merge が走る。
- merge は additive: post-extension list に `def.id` が存在しない def のみ
  末尾へ追加(plugin 定義順 → user 追加順)。衝突は list 側(plugin/
  builtin base 含む)が authoritative で、user def は skip + error 記録
  (`Provider "<id>": <msg>` 集約)。
- fallback chain: `api` = `definition.api → config.api → plugin.api`、
  `baseUrl` = `definition.baseUrl → config.baseUrl → plugin.base_url →
  find_model_defaults(post-extension list)`。空文字列 fallback は
  helper 内部の単一 site で filter する。解決不能・検証 error の def は
  per-def skip + error 記録で、provider・plugin models は残る。
- `refresh_plugin_models` は compose と同一 merge を再実行する —
  衝突判定は refresh ごとに再評価され、plugin catalog から消えた id の
  user def は effective になり、復活した id では list 側が優先される。
  `model_overrides` は merge 後の最終層として再適用される。
- refreshed defs の `headers` は `model_header_sources` 再構築を経て
  `CatalogStore::model_headers` へ届く。
- plugin provider の provider-level 検証(`oauth` 宣言時の `baseUrl`
  必須・「must specify …」)は custom provider と同一 semantics を維持
  する(緩めない)。custom provider の invalid def による provider drop
  契約は不変。

### catalog error surfacing(spec Req 4)

- `cheaper models`・`cheaper run`・`cheaper auth status`・
  `cheaper auth login` は `catalog.errors()` の各 error を
  `warning: <msg>` 行として stderr へ出力する。`--quiet` で抑制し、
  envelope `result` shape・`schemaVersion`・exit code は不変。
- `auth` 系は action dispatch 前に compose-time error を emit する
  (early emit)ため、dropped provider への `auth status <id>` でも
  `warning:` 行と `unknown provider` envelope error が併存する。
- `auth login` は login 中に `refresh_plugin_catalog` が記録した
  refresh/persist error を同一 run で差分 emit する(post-login emit)。

## Examples

- Given: fake plugin(`FakeCatalog`/`FakeAuth`、`tests/plugin.rs` seam)と
  tempdir userConfigDir。When: `auth().login("fake", oauth)` 成功後、
  別 `Runtime::create` を同じ dir で構築。Then: `catalog.json` が
  `providers/fake/catalog.json` に存在し、新 runtime で refreshed model
  が `get_supported_model`/`get_available` に見える。
- Given: fake `refresh_models` が bootstrap `registration.models` と等価な
  defs を返す(または空 defs、credential `None` の `persist_plugin_catalog`
  直接呼び出し)。Then: 既存 `catalog.json` を上書きしない。
- Given: fake `refresh_models` が `extension_models` 変換不能な defs
  (baseUrl 解決不能)を返す。Then: `refresh_plugin_models` が Err →
  `record_error` され `catalog.json` が書かれない・既存 file を上書きしない。
- Given: `providers.fake.models[]` に plugin 未定義 id の def(api/
  baseUrl 省略)。Then: plugin 宣言値 fallback で解決され catalog に残り、
  model resolution で選択可能。
- Given: user def の id が post-extension list と衝突、または def が
  invalid。Then: その def のみ skip + `errors` 記録、provider・plugin
  models は存続(silent discard なし)。
- Given: compose 時に plugin id と衝突して skip された user def。
  When: refresh で plugin catalog がその id を落とす。Then: re-merge で
  user def が effective になる(逆方向は list 優先 + error 記録)。
- Given: refreshed defs が `headers` を持つ。Then: refresh 後の
  `model_headers` に反映される。
- Given: `base_url:""` を返す fake plugin と baseUrl 無しの user def。
  When: compose と `refresh_plugin_models` の両経路で merge を走らせる。
  Then: 両経路で def が skip + `errors`/`record_error` に記録され、空
  `base_url` の model は catalog に残らない(helper 内の非空 filter が
  両経路で同一に効くことの検証)。
- Given: malformed `catalog.json` を配置。When: `cheaper models`・
  `auth status`・`auth login`・`run`。Then: stderr に `warning:` 行、
  `--quiet` で抑制、envelope `result` shape・exit code 不変。
  `auth login` では login 中の refresh 失敗注入(`FakeCatalog.fail`)が
  同一 run で emit される。
- Given: logout 済み provider。Then: `catalog.json` は残り、次回
  `Runtime::create` が同 snapshot を使い続ける。
- Given: plugin provider config `{"oauth":"radius"}`(baseUrl 無し)。
  Then: 従来どおり検証 Err → provider drop + `errors` 記録
  (`{"oauth":"radius","baseUrl":…}` は radius env auth のまま通過)。

## Guarantees

- login 1 回で取得した live catalog が以降の任意 process の catalog に
  届く( offline ・低遅延契約を壊さない — read path は file I/O のみ)。
- user `models[]` の破棄・shadowing・def 単位 invalid が全て catalog
  error として記録され、silent discard の経路が残らない。
- merge・fallback・衝突規則が compose 時と refresh 時で同一実装
  (`merge_user_model_defs`・`extension_models` 共有)に閉じる。
- persist guard により bootstrap 等価・空・credential 無し refresh が
  既存 snapshot を破壊しない。
- catalog error が `models`/`run`/`auth status`/`auth login` の stderr
  で観測可能(無報告の `unknown provider` 経路が残らない)かつ
  envelope/exit code 不変。
- `<userConfigDir>/pi/` 配下の file を一切読み書きしない。

## Non-goals

- lazy/TTL refresh・stale 判定による再取得(鮮度は `fetchedAt` の診断
  用途のみ)。
- logout/credential 無効化時の snapshot 削除。
- envelope `warnings` field 等の構造化 surface(別 chain defer)。
- `cheaper workflow run` 等 spec 列挙外 command への warning。
- `<userConfigDir>/pi/` の file 読み取り・import、`PI_CODING_AGENT_DIR`
  の参照。
- 実 endpoint の smoke 検証(別 chain の opt-in smoke で扱う。本 spec の
  検証は fake CLI/stub で代替する)。
- upstream TS `applyExtension` との parity 検証(vendored source 不在の
  ため未検証前提 — merge は defect 修正として正当化済み)。

## Risk

- この保証が落ちると issue #81/#82 が再発する: login 成果が process を
  跨がず live catalog の model が `MODEL_TIER_UNAVAILABLE` で到達不能に
  戻り、user `models[]` 追加が silent discard で失われる。persist guard
  の欠落は bootstrap 等価 refresh が正常 snapshot を上書きして自己修復
  不能にする。surfacing の欠落は provider drop が `unknown provider`
  にしか見えず診断不能になる。merge 規則の compose/refresh drift は
  login 後に user def が再び消失する。

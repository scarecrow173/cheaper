---
id: "DISC-o3GbG6FWEBRFcbNBaslikh"
type: "discovery"
status: "resolved"
title: "Plugin catalog refresh persistence and user model additions"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "docs/tasks/phase-3-port-runtime-adapter-providers.md"
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
  related:
    - "docs/adr/pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
    - "docs/designs/devin-local-provider-plugin.md"
    - "docs/impl/ir/phase-3-port-runtime-adapter-providers.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/plugin-catalog-persistence-and-user-models.md"
    - "../adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
    - "../designs/plugin-catalog-snapshot-persistence-and-user-models-merge.md"
    - "../plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Plugin catalog refresh persistence and user model additions

## Exploration Goal

`devin` provider(bundled plugin、`crates/cheaper-provider-devin`)の実
endpoint smoke(issue #81・#82、smoke 記録は
`docs/tasks/phase-3-port-runtime-adapter-providers.md`)で判明した 2 つの
関連 defect を整理し、bug-fix work chain の設計判断に必要な事実と未決
事項を確定することが目的。

- **Defect 1(issue #81)**: plugin live catalog(`devin models list
  --format json` 経由の `DevinPluginCatalog::refresh_models`)の結果が
  login process の in-memory state にのみ保持され、永続化されない。
  別 process の `run`/`models` は bootstrap `fallback_models()` 4 件
  しか見えず、実 endpoint が公開する 100+ variant(`swe-2` family 含む)
  はすべて `MODEL_TIER_UNAVAILABLE` で到達不能。
- **Defect 2(issue #82)**: `compose_provider` で `plugin.models` が非空の
  場合、user の `providers.<id>.models[]` 追加が `apply_extension` の
  丸ごと replace で silently 破棄される。`modelOverrides` は既存 id の
  mutate のみで新規追加は不能であり、plugin provider へ user が model
  を追加する経路が存在しない。drop は catalog error にも記録されず
  診断不能。

## Key Issues and Assumptions

以下、全件を実コード・実測で検証済み(confirmed)。

### Defect 1: live catalog refresh が process 内に閉じる

- refresh seam は `PluginCatalog::refresh_models`
  (`crates/cheaper-runtime/src/plugin.rs:105-114`)。devin 実装は
  `DevinPluginCatalog::refresh_models`
  (`crates/cheaper-provider-devin/src/plugin.rs:162-179`)で、credential
  ありなら `load_cli_catalog`(`src/models.rs:440-469`、
  `devin models list --format json`、timeout 20s)→ `models_from_catalog`
  (`src/models.rs:422-435`)、credential 無しは `fallback_models()`。
- 呼び出し経路は `AuthService::login` 成功後の
  `refresh_plugin_catalog`(`crates/cheaper-runtime/src/auth/service.rs:452`
  起動、`464-486` 本体)のみ。結果は `CatalogStore::refresh_plugin_models`
  → `replace_plugin_models`(`src/catalog/mod.rs:947-985`)で
  `dynamic_models: RwLock<BTreeMap>`(`catalog/mod.rs:780`)へ入る。
  **in-memory のみで永続化経路は存在しない**。
- `Runtime::create`(`crates/cheaper-runtime/src/runtime.rs:41-74`)は
  process 起動ごとに bundled snapshot + `<userConfigDir>/models.json`
  + plugin registration から catalog を再構築する。`cheaper` CLI は
  command ごとに別 process のため、login process の refresh 結果は
  `run`/`models` へ一切届かない。
- 観測挙動: `cheaper auth login devin` 後も別 process の
  `cheaper models`/`cheaper run --model devin/<id>` は bootstrap の
  `fallback_models()` 4 件(claude-opus-5 / claude-fable-5 /
  gpt-5.6-sol / swe-1-7、`models.rs:292-418`)のみを見る。実 endpoint
  の 100+ variant(`swe-2` family 含む)は `resolve_explicit_model`
  (`crates/cheaper-core/src/model/resolve.rs:272-285`)で
  `MODEL_TIER_UNAVAILABLE` になる。smoke 記録(task doc Verification 欄)
  の「catalog refresh → 実 model 4 件」もこの fallback 4 件と一致し、
  refresh 結果が非永続であることと整合する。
- TS 前身(Pi `ModelRuntime`)は refresh 済み catalog を pi
  `models-store.json` 経由で永続化していた。Rust port は
  `<userConfigDir>/pi/` を意図的に読まない
  (`runtime.rs:7-8`、ADR `pi-persistent-state-userconfigdir-pi-…`)が、
  代替の永続化経路を持たない。IR 記録でも `models-store.json` /
  remote catalog refresh(`withRemoteCatalog`)は v1 非実装と明記
  (`docs/impl/ir/phase-3-port-runtime-adapter-providers.md:533`)。

### Defect 2: plugin provider への user `models[]` が silently 破棄される

- `compose_provider`(`crates/cheaper-runtime/src/catalog/mod.rs:591-613`)
  は `apply_models_json`(600 行、user overlay merge)→
  `apply_extension`(604 行)→ `modelOverrides`(607-613 行)の順。
- `apply_extension`(`catalog/mod.rs:475-506`)は `plugin.models` が非空
  なら `extension_models` が **model list を丸ごと replace する**
  (475-476 行コメントどおり)。このとき user `models[]` 追加済みの
  入力 list は `find_model_defaults` の参照先にしかならず、結果には
  残らない。devin は `models: fallback_models()`(`plugin.rs:238`)で
  常に非空 → 常に replace が走る。
- `modelOverrides` は `overrides.get(&model.id)` で既存 model の per-field
  mutate のみ(`catalog/mod.rs:607-613`、`apply_model_override` 257-298)。
  新規 id の追加はできない。
- さらに plugin provider は builtin meta が無く `base_models` が空の
  ため、user definition が `api`/`baseUrl` を欠くと `model_from_json`
  (`catalog/mod.rs:325-406` — fallback 参照は `config.api`/
  `config.base_url` と `base_models` のみで、plugin の `api`/`base_url`
  は見ない)が error → compose error で provider ごと drop
  (`CatalogStore::build` 841-855、builtin 以外は fallback 無し)。
  definition を valid(`api`/`baseUrl` 付き)にしても、今度は plugin
  models で silently replace される。どちらの経路でも user `models[]`
  は最終 catalog に到達せず、replace 経路では catalog error も記録
  されない(undiagnosable)。
- 同じ `extension_models` は `refresh_plugin_models`
  (`catalog/mod.rs:958-985`)からも呼ばれるため、仮に永続化を入れても
  refresh 適用で user `models[]` が再び消失する規則が共有されている。

### 2 defect の相乗

Defect 1 で live catalog が `run` から見えない以上、自然な回避策は
user `providers.devin.models[]` への手動追加だが、Defect 2 がそれを
塞ぐ。合わせて plugin provider の model 追加経路が完全に閉じており、
一方だけの修正では回避不能・診断不能な状態が残るため、同一 work
chain で扱う。

## Alternatives and Comparison

設計方針は未確定。判断軸だけを整理する。

### Defect 1 — 永続化の所在と鮮度

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `<userConfigDir>` 直下に cheaper-owned の新 file(例: catalog cache) | credential/models.json と同 layer、`pi/` 非読取方針と整合 | file 契約を新設する必要 | maybe |
| `<userConfigDir>/providers/<id>/` 配下(plugin 隔離 dir) | plugin 固有 state の既存境界(`plugin_dir`)に整合、provider 追加に拡張しやすい | runtime が plugin 固有 dir 規約を知る必要 | maybe |
| 永続化せず run/models 時に lazy refresh | 常に最新 | 起動ごとに `devin` subprocess + network、offline/timeout で run が遅延・失敗し得る | no(無条件には不可、TTL refresh の一形態としてはあり得る) |

形式(resolved `CatalogModel` 保存 vs `ModelDefinitionJson` 保存)・
鮮度契機(login 時のみ vs TTL/stale 時 refresh)・credential 無効化後の
扱いは Open Questions へ。

### Defect 2 — user `models[]` の merge vs replace

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| plugin models と user `models[]` を merge(upsert) | user が plugin provider へ model を追加できる、issue #82 の根本解消 | TS `applyExtension` 丸ごと replace との parity 乖離の可能性、merge 規則の新設計 | maybe |
| replace 維持(TS parity)+ drop を catalog error で明示 | 現行 contract 最小変更、診断可能性だけ回復 | user 追加経路は依然無し(issue #81 の回避策にならない) | maybe |
| 現状維持 | 変更なし | 2 defect 相乗で追加経路ゼロのまま | no |

## Tentative Conclusions and Hypotheses

- (confirmed) 2 defect は実コード・実測で再現する。`dynamic_models` は
  in-memory `RwLock` のみで永続化無し、`apply_extension` は plugin
  models 非空で丸ごと replace、`modelOverrides` は追加不能。
- (confirmed) 2 つは相乗し、plugin provider の model 追加経路が
  完全に存在しない。片方だけの修正では実効が限定的。
- (hypothesis) 永続化は `<userConfigDir>` 配下の cheaper-owned file が
  筋(pi file 非読取の方針と整合)だが、所在・形式・鮮度は未決。
- (hypothesis) `apply_extension` の丸ごと replace は pi
  `provider-composer.ts` の port 注記として書かれているため TS parity
  の意図的 contract の可能性があるが、TS 側でも plugin provider への
  user 追加が不可能だったかは未確認 — merge へ変える判断は parity
  確認が前提。

## Open Questions

1. live catalog の永続化先・形式・鮮度: `<userConfigDir>` 直下 vs
   `providers/<id>/` 配下、resolved `CatalogModel` vs
   `ModelDefinitionJson` の保存粒度、refresh 契機(login のみ /
   TTL / run 時 lazy refresh)、logout・credential 無効化時の cache
   破棄規則。IR の「`models-store.json` / `withRemoteCatalog` は v1
   非実装」判断(offline v1 規則)をどこまで維持・改訂するか。
2. `providers.<id>.models[]` の plugin provider への追加は merge が
   正しいのか — TS `applyExtension` の replace 仕様は parity 対象の
   意図的 contract か、TS 側の制約の移植か(要: pi
   `provider-composer.ts` の実挙動確認)。merge にする場合、plugin
   宣言 `api`/`baseUrl` を user definition の fallback に使うか —
   現状 `model_from_json` は config/base_models のみを見て plugin の
   `api`/`base_url` を参照しないため、plugin provider で user model
   定義が `api`/`baseUrl` 必須になっている点も併せて決める。
3. silent drop の surfacing: user `models[]` が plugin models で
   置換・消失する経路を catalog error(`compositionErrors` 相当)として
   記録するか、記録するなら compose 時 error / `models` 出力 warning
   のどの層で。
4. `refresh_plugin_models` の合成規則: 永続化と merge を入れた場合、
   login 後 refresh で差し替わる list にも user `models[]` を保持する
   規則を compose 時と一本化するか(DRY — `extension_models` は両経路で
   共有済み)。

## Promotion Candidates

- [ ] spec-doc: plugin catalog persistence + user `models[]` merge
      contract(issue #81/#82 の fix 仕様)
- [ ] task/plan: 本 discovery を source とする bug-fix work chain

## Sources and Relations

- GitHub issue #81 / #82 — それぞれ Defect 1 / Defect 2 の report
- `docs/tasks/phase-3-port-runtime-adapter-providers.md` — devin 実
  endpoint smoke 記録(login → catalog refresh → fallback 4 件のみ、
  `run --model devin/swe-1-7` 成功、Windows child env 隔離の残課題)
- `docs/adr/pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md`
  — `<userConfigDir>/pi/` 非参照方針(永続化先選定の制約)
- `docs/designs/devin-local-provider-plugin.md` — plugin contract の設計
- `docs/impl/ir/phase-3-port-runtime-adapter-providers.md` — F13
  (`plugin_catalog` handle + login 後 refresh)と
  `models-store.json` v1 非実装の実装記録
- 実コード: `crates/cheaper-runtime/src/catalog/mod.rs`・
  `src/auth/service.rs`・`src/plugin.rs`・`src/runtime.rs`、
  `crates/cheaper-provider-devin/src/plugin.rs`・`src/models.rs`、
  `crates/cheaper-core/src/model/resolve.rs`

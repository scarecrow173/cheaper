---
id: "SPEC-034CKp34AcjKtAYjnfjXU4"
type: "spec"
status: "proposed"
title: "Plugin catalog persistence and user model additions"
created: "2026-10-09"
updated: "2026-10-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/81"
    - "https://github.com/scarecrow173/cheaper/issues/82"
    - "../tasks/phase-3-port-runtime-adapter-providers.md"
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
    - "../plans/persist-plugin-catalog-snapshot-and-merge-user-models.md"
  depends-on:
    - "../adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./devin-local-bundled-provider.md"
    - "./pi-state-under-cheaper-config-dir.md"
    - "../designs/devin-local-provider-plugin.md"
    - "../adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md"
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
    - "../impl/ir/phase-3-port-runtime-adapter-providers.md"
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# Plugin catalog persistence and user model additions

## What Should Be Built

plugin provider の catalog に関する 2 つの defect(issue #81・#82)を修正し、
「login で取得した live catalog が別 process から使えること」と「plugin
provider への user `models.json` 定義が silent に消失しないこと」を実現
する。

- **plugin catalog の永続化(issue #81)**: plugin `refreshModels`
  (devin では `devin models list --format json` 委譲)の結果を、login
  process の in-memory state だけでなく `<userConfigDir>` 配下の
  cheaper-owned な永続 state へ保存し、以降の任意の新規 process
  (`run`/`models` 等)の catalog 構築で利用できるようにする。
  `<userConfigDir>/pi/` 配下の pi file は読み書きしない(既存方針維持)。
- **user `models[]` の no-silent-discard(issue #82)**: plugin provider
  に対する user `providers.<id>.models[]` 定義が catalog 合成で
  無報告に破棄されないようにする。plugin 定義との merge(追加を選択
  可能にする)か、決定論的な catalog error による明示的拒否かは ADR で
  決定するが、「silent discard だけは許容しない」が本 spec の要求。
- **silent drop の surface**: non-builtin provider が catalog
  composition error で drop される経路(現状は user から観測不能で
  `unknown provider` にしか見えない)を、stderr diagnostic
  (`warning:` 行、`--quiet` で抑制)として診断可能にする。

本 spec は要件のみを規定する。surfacing は stderr diagnostic を
要件として固定し(Req 4)、永続化先・形式・鮮度・merge 規則の
設計判断は並走する ADR(ADR-034CKnwOWyYssJ1VZR9spo)に委譲する。

## Why Now

- issue #81: `cheaper auth login devin` 成功後も、別 process の
  `cheaper run`/`cheaper models` は bootstrap `fallback_models()` の
  4 件しか見えず、実 endpoint が公開する 100+ variant(`swe-2` family
  含む)はすべて `MODEL_TIER_UNAVAILABLE` で到達不能。実 endpoint smoke
  (`docs/tasks/phase-3-port-runtime-adapter-providers.md`)で再現済み。
  TS 前身(Pi `ModelRuntime`)は refresh 済み catalog を
  `models-store.json` で永続化していたが、Rust port は pi file を
  意図的に読まず代替経路を持たない(IR で v1 非実装と明記済み)。
- issue #82: `compose_provider` で `plugin.models` が非空の場合
  `apply_extension` が model list を丸ごと replace するため、user の
  `models[]` 追加は結果に残らず catalog error にも記録されない。
  `modelOverrides` は既存 id の mutate のみで新規追加は不能であり、
  plugin provider へ user が model を追加する経路が存在しない。
- 2 defect は相乗する: live catalog が見えない以上の自然な回避策
  (user `models[]` への手動追加)を Defect 2 が塞ぐため、片方だけの
  修正では回避不能・診断不能な状態が残る。同一 work chain で扱う。

## Users and Value

- **cheaper 利用者 / parent agent**: `auth login devin` 後に実 endpoint
  の全 model(`swe-2` 等)を通常の model/tier resolution で利用できる。
  login の成果が process を跨いで有効になる。
- **cheaper 利用者(設定編集者)**: plugin provider への `models[]` 追加が
  機能するか、少なくとも失敗理由が分かる。設定が無言で無効化される
  ことがなくなる。
- **cheaper 開発者 / 将来の plugin provider 作者**: catalog drop が
  catalog error として観測可能になり、plugin catalog 合成規則が
  compose 時・refresh 時で一本化される。

## Scope

### In Scope

- plugin provider の refreshed catalog の永続化と、新規 process での
  利用経路(所在・形式・鮮度・logout/credential 無効化時の扱いを含む
  contract 定義)。
- plugin provider に対する user `models.json` (`providers.<id>.models[]`
  ・`modelOverrides`)の合成 semantics の定義と修正 — compose 時と
  login 後 refresh 時で同一規則とすること。
- plugin 宣言 `api`/`baseUrl` を user model 定義の fallback として
  参照するかの決定(現状は plugin 値を見ず、省略時に provider ごと
  drop する)。
- catalog composition error / silent drop の surfacing(non-builtin
  provider の drop がユーザーから観測可能であること)。

### Out of Scope

- plugin contract(`CheaperProviderPlugin`・`refreshModels` seam)自体の
  再設計。
- `cheaper auth login` の UX・flow の変更。
- 新規 provider や provider 機能の追加。
- Windows での devin CLI child env 隔離の問題(issue #80)— 別 chain で
  追跡済み。
- `<userConfigDir>/pi/` の pi file(`models-store.json` 等)の読み取り・
  import — pi file 非参照方針を維持する。

## Requirements

1. **永続化**: plugin `refreshModels` が model list を返した場合、その
   結果は `<userConfigDir>` 配下の cheaper-owned な state へ保存され、
   以降に起動する任意の process の catalog 構築で利用できること。
   `<userConfigDir>/pi/` 配下の file は読み書きしない。
2. **鮮度 semantics**: persisted catalog の refresh 契機・stale 判定・
   logout/credential 無効化時の扱い・破損/欠損時の fallback 挙動を
   ADR で定義し、定義どおりに動作すること。少なくとも login 成功時の
   refresh 結果が後続 process に届くこと。
3. **no silent discard**: plugin provider の user `providers.<id>.models[]`
   定義は、compose 時・plugin refresh 適用時の両経路で silent に
   破棄されないこと。最終 catalog に残る(merge)か、決定論的な
   catalog error として記録される(reject)かは ADR で決定するが、
   どちらの規則でも両経路で一貫すること。merge を採る場合、user
   定義と plugin 定義 id の衝突判定は plugin catalog refresh の
   たびに同一 merge 規則で再評価されること(compose 時に衝突で
   無効だった定義が、refresh で plugin 側の id が消えた後に
   有効化され得る)。
4. **surfacing**: non-builtin provider が catalog composition /
   persistence error で drop・skip・file 無視される場合、その error
   が stderr diagnostic として観測可能であること。`cheaper models`・
   `cheaper run`・`cheaper auth status`・`cheaper auth login` は
   catalog error を `warning:` 行として stderr へ出力する(`--quiet`
   で抑制、envelope `result` shape・exit code は不変)。
   `auth login` は login 中の catalog refresh/persist 失敗を含む
   (これらの `record_error` は login process の終了とともに消える
   ため、login 自身が emit しないと観測経路がない)。無報告のまま
   `unknown provider` になる経路を残さない。
5. **plugin `api`/`baseUrl` fallback**: plugin provider の user model
   定義で `api`/`baseUrl` を省略した場合の挙動(plugin 宣言値への
   fallback を許すか明示 error とするか)を ADR で決定し、「省略 →
   provider ごとの silent drop」という現状を解消すること。
6. **不変 contract**: stdin/stdout envelope `schemaVersion: 1`・exit
   code・`models.json` の schema・plugin contract の外部形・
   `auth login` の exit contract は変更しない。

## Acceptance Criteria

- [ ] `cheaper auth login devin` 完了後、**別 process** の
      `cheaper models --provider devin` が live catalog の model
      (`swe-2` 等、bootstrap fallback 4 件以外)を一覧する。
      実 endpoint 依存の検証は opt-in smoke で行い、通常 test は
      fake CLI/stub で代替する。
- [ ] `cheaper run --model devin/<live catalog のみに存在する model>`
      が catalog 解決を通過する(`MODEL_TIER_UNAVAILABLE` で失敗
      しない。endpoint 側の正当な error は許容)。
- [ ] user `providers.<pluginId>.models[]` の定義が (a) 最終 catalog に
      残り `models` 出力・model resolution で選択可能、または (b)
      決定論的な catalog error として surface される — silent discard
      が存在しないことが test で検証される。
- [ ] plugin `refreshModels` 適用後も user `models[]`/`modelOverrides`
      の扱いが初回 compose 時と同一規則(衝突の再評価を含む)である
      ことが test で検証される — 例: compose 時に plugin id と衝突
      して skip された user 定義が、refresh で plugin catalog から
      その id が消えた後に catalog へ反映される。
- [ ] non-builtin provider の catalog composition/persistence error が
      `cheaper models`/`run`/`auth status`/`auth login` の stderr に
      `warning:` 行として出力されることが stderr assertion で検証
      される(`--quiet` で抑制されること、envelope `result` shape が
      不変であることを併せて確認)。`auth login` では login 中の
      refresh/persist 失敗が同一 run で emit されることも確認する。
- [ ] persisted catalog の鮮度 semantics(refresh 契機・stale 判定・
      logout/credential 無効化時・破損時 fallback)が ADR で定義され、
      定義どおりの挙動が test で検証される。
- [ ] `<userConfigDir>/pi/` 配下の file を一切読み書きしない。
- [ ] 既存 test が全て green であり、新挙動を覆う test が追加されて
      いる。

## Deferred Design Concerns

- 永続化 file の所在(`<userConfigDir>` 直下 vs `providers/<id>/` 配下)・
  形式(resolved model vs definition 粒度)・鮮度(login のみ vs TTL/lazy)
  | 設計判断は本 spec ではなく並走 ADR の責務 | ADR 作成時 | 未決のまま
  実装に入ると file 契約が暗黙に確定し後戻りコストが高い。
- user `models[]` の merge vs replace(TS `applyExtension` parity の
  確認を前提とする)・plugin `api`/`baseUrl` fallback の是非 | 同上 |
  ADR 作成時 | merge 規則を暗黙に決めると TS parity との乖離が
  未検証のまま残る。
- catalog error の surface 層(`models` warning vs `auth status` vs
  明示 error) | 同上 | ADR 作成時 | 層を誤ると診断可能性の目的を
  満たさない。
- run/models 起動時の lazy refresh(常に最新化) | 起動ごとの subprocess
  + network コストと offline/timeout 耐性のトレードオフがあり v1 必須
  ではない | 永続化方式の確定後、必要になった時点で再検討 | 放置しても
  永続化 + login 時 refresh で要件は満たせる。

## Sources and Relations

- GitHub issue
  [#81](https://github.com/scarecrow173/cheaper/issues/81) /
  [#82](https://github.com/scarecrow173/cheaper/issues/82) — 実 endpoint
  smoke で報告された 2 defect。
- 上流 discovery:
  `docs/discovery/plugin-catalog-refresh-persistence-and-user-models.md`
  (`DISC-o3GbG6FWEBRFcbNBaslikh`)— 全 defect の実コード・実測検証済み
  の evidence、option 比較、Open Questions を記録
  (`relations.derives-from`)。
- `docs/tasks/phase-3-port-runtime-adapter-providers.md` — devin 実
  endpoint smoke 記録(login → catalog refresh → fallback 4 件のみ)。
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  — `<userConfigDir>/pi/` 非参照・Pi state 非移行方針(`depends-on`:
  永続化先選定の制約として維持する。同方針の由来である superseded
  `pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md`
  を supersede した現行 ADR)。
- `docs/designs/devin-local-provider-plugin.md` /
  `docs/specs/devin-local-bundled-provider.md` — plugin contract と
  devin provider の設計(「live catalog を必ず優先」は既存要求)。
- `docs/impl/ir/phase-3-port-runtime-adapter-providers.md` — F13
  (plugin catalog refresh seam)と `models-store.json` v1 非実装の
  実装記録。本 spec はその非実装判断の改訂を含む。
- 実装選択の決定は ADR
  `docs/adr/persist-plugin-catalog-snapshot-and-merge-user-models-on-plugin-providers.md`
  (ADR-034CKnwOWyYssJ1VZR9spo)に委譲する(`relations.related` で
  相互リンク済み)。

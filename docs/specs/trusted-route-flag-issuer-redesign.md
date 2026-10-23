---
id: "SPEC-034LwdoInh3ie1KPlSjkJl"
type: "spec"
status: "proposed"
title: "trusted route flag issuer redesign — 宣言層 provenance trust・project 層 `trusted` 宣言の拒否"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/168"
    - "../discovery/trusted-route-flag-issuer-redesign-issue-168.md"
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
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/trusted-route-flag-issuer-redesign-issue-168.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../designs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../designs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../specs/unified-exec-grant-model.md"
    - "../tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/configuration.md"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign — 宣言層 provenance trust・project 層 `trusted` 宣言の拒否

## What Should Be Built

route の trust 判定を「宣言値（route entry の `trusted: true`
field）」から「**宣言層 provenance**（builtin/user 層で宣言された
route = trusted、project 層で宣言された route = untrusted）」へ
移す再設計。`trusted` flag の発行主体を project 自己申告から
operator/user 層（= 宣言層そのもの）へ移し、route trust 判定を
grant `scope`・`grants`/`substitute` 層制約と同じ「trust は宣言
内容ではなく宣言主体が決める」単一モデルへ揃える。

構成要素は 3 つ + 整合記述:

1. **trust = 宣言層 provenance** — 3 層 merge（builtin → user →
   project）の shadowing gate が `route.trusted` 宣言値ではなく
   `route.layer` を読む。builtin/user 層 route は現行どおり下位層を
   無条件 shadow し、project 層 route は `match.command` 衝突時に
   **`trusted` 宣言の有無・値に関わらず常に下位層へ負け**て
   `suppressed` provenance へ記録される。`trusted: true` による
   project 層の shadow 経路は廃止する。
2. **project 層の `trusted` key presence を fail-loud 拒否** —
   `grants`/`substitute`（#166 O7a・#167 O3a）と同じ宣言層 gate・
   同じ error class（`InvalidRuntimePolicy` + `schema_error` —
   `"{label}.trusted: not declarable in the project layer"`）で、
   project 層 route entry が `trusted` key を含むことを値の形状・
   値に関わらず拒否する。builtin/user 層の `trusted` key は従来
   どおり受理される（bool 形状検査・inert — その層では元来意味を
   持たない documented 現状維持）。
3. **`Route.trusted` field の除去** — `route.trusted` の唯一の
   consumer は merge の shadowing gate（`routes.rs:201`）であり、
   provenance 判別へ置き換わるため field を struct から削除する。
   `SuppressedRoute`・`InvalidRouteGroup`・`RouteLayer`・
   `EffectiveRoutes`・`route resolve` 出力 shape・`kind:"route"`
   record shape は変更しない。
4. **grant 参照 scope との整合** — route trust（宣言層 provenance）
   と grant `scope`（authoring 層 provenance: project task 由来は
   `scope:"project"`、trusted 層宣言は `scope:"trusted"` 既定 +
   `scope:"project"` 明示は自己弱体化）は同一モデルの 2 面であり、
   route の grant 参照（`grant`/`grants[]`/`substitute.grant`）が
   `scope=="trusted" && !fixed` の grant のみを解決する
   `is_route_eligible` 単一 seam は不変であることを文書化する。

## Why Now

- #166（`grants`）・#167（`substitute`）で「exec 委譲面の
  privileged 宣言は宣言層 provenance で判別し、project 層
  presence を fail-loud 拒否する」規約が確立済み —
  `routes.rs:476-480`・`:510-519` の code comment が
  「`trusted:true` は project 自己申告の shadowing gate であり
  判別に使えない — 判別子は宣言層 provenance」と明記する時点で、
  flag 自身が唯一残った project 自己申告 privilege になっている。
- grant `scope` provenance（#165 M3 — project task →
  `scope:"project"`、route grant 参照は trusted-scoped のみ解決）
  と同一の信頼モデルへ route trust を揃えることで、route 側は
  「誰が宣言したか」・grant 側は「誰が authoring したか」が同じ
  層境界を共有する。
- 親 design（DESIGN-034KuDkWOmNXiBddEQ4XWK）の issue 分解 6/7
  であり、deps（#163・#166・#167）は全て完了済み。残存リスク
  「project が自ら `trusted` を宣言できる」は本 issue の完了を
  もって解消する。
- `route.trusted` の consumer は現行で merge gate のみと実測済み
  （viability・grant 解決・record は trusted/layer を読まない）
  — flag の意味論を層 provenance へ畳む変更の blast radius が
  merge 単一点に限定されている。

## Users and Value

- **operator/user（trusted 層作者）** — workspace checkout が
  `trusted: true` を書くだけで trusted 層 route の `match.command`
  prefix を乗っ取る自己申告経路が構造的に閉じる。自分が宣言した
  route は project 側の宣言では二度と置き換わらない。
- **project 作者** — 非衝突 prefix の route 宣言・発火・`grant:`
  anchor 参照は従来どおり書ける（新規 prefix route は trust 不要で
  宣言可能 — D7 の残存用途）。衝突時は `suppressed` provenance
  として `route resolve`・`kind:"route"` record に報告され、
  黙って無視される経路は変わらず存在しない。
- **cheaper 保守者** — trust 判別子が「宣言層」の 1 つに統一
  され、flag 判別と provenance 判別の二重経路が解消する。
  `Route` の意味論から privilege flag が消え、`Route.layer`
  provenance が trust・診断・target.path 層起点解決を担う単一
  source になる。

## Scope

### In Scope

- merge（`route::routes::load`）の project 分岐を「衝突 →
  `suppressed` provenance 記録・非衝突 → 有効化」の無条件形へ
  変更 — `route.trusted` 読取りの除去
- `validate_route` の project 層 `trusted` key presence 拒否 —
  `grants`/`substitute` と同じ gate 位置・error class・
  presence-first 判別
- `Route.trusted` field の削除と、`KNOWN_KEYS` の `trusted`
  処遇（builtin/user 層は受理・bool 検証・inert を維持）
- `SuppressedRoute` doc comment の semantics 追従（displaced
  下位層 route の記録 arm は到達不能になり、「衝突で負けた
  project route」のみを運ぶ）
- 関連 test の更新（`routes.rs` の merge/layer-gate test、
  integration `route.rs`・`route_run.rs` の `trusted` 前提 arm、
  test constructor の `trusted` field 除去）
- docs 追従 — `specs/pretooluse-route-resolver-and-workflow-delegation.md`
  の R1 field 一覧・R7 層間 trust 記述、`guides/parent-agent-integration.md`
  の route 宣言層説明、`resources/routes.yaml` の shape 記述
- route trust と grant `scope` の整合記述（下記 R5）

### Out of Scope

- **operator/user 側 consent registry**（trusted 層が project
  route の shadow を個別に許可する宣言機構 — ADR O1b）— 需要未
  確認のため YAGNI。D7 で却下済みの user-level opt-in list と
  同型であり、将来の需要が確立した時点で trusted 層宣言機構と
  して追加可能（forward-compatible — 本 spec が拒否するのは
  project 層の `trusted` key であり、trusted 層側の新規宣言機構
  は別 schema として後付けできる）
- project route の新規 prefix 宣言権・`grant:` anchor 宣言権の
  変更 — project 層 route は従来どおり非衝突 prefix で宣言・発火
  する（不変）
- grant `scope` モデル・`is_route_eligible`・`project_grants` の
  変更 — route grant 参照の eligibility は #165/#166 確定のまま
- `suppressed`/`invalid`/`route resolve` 出力 shape・
  `kind:"route"` record shape・`schemaVersion` の変更
- builtin/user 層の inert `trusted` key の schema からの完全除去
  — routes.yaml `version` bump が必要な schema 破壊であり本
  issue では行わない（受理・inert を維持。Deferred Design
  Concerns 参照）
- Option E（exec-less route）— 引き続き親 chain の open 事項
- 文書正直性 sweep・migration guide 本文 — issue 7（#169）が
  担う。本 spec は破壊的変更の告知材料を供給するのみ

## Requirements

- R1. route の trust 判定を宣言層 provenance とすること —
  `layer ∈ {Builtin, User}` の route は同一 `match.command` で
  下位層を無条件 shadow する（現行不変 — user が builtin を
  無条件 shadow する規約も不変）。`layer == Project` の route
  は `match.command` が下位層 route と衝突した場合、`trusted`
  宣言の有無・値に関わらず常に負けて `suppressed` provenance
  へ記録され、非衝突時は現行どおり有効化される。merge の
  project 分岐は `route.trusted` を一切読まないこと。
- R2. project 層 routes.yaml（`<project_dir>/routes.yaml`）の
  route entry が `trusted` key を含む場合、値の形状・値
  （`true`/`false`）に関わらず fail-loud route-load error
  （`InvalidRuntimePolicy` — `schema_error` 経由、violation
  メッセージ `"{label}.trusted: not declarable in the project
  layer"`）で拒否すること。判定は `grants`/`substitute` と同じ
  宣言層 provenance gate（`layer == RouteLayer::Project &&
  entry.contains_key(...)`）・同じ error class・同じ
  presence-first 規約（untrusted 層の宣言は内容を評価しない）
  で行い、層 gate 群は `grants`/`substitute` 拒否と同じ検証
  位置（required field 解読後・該当 key の値形状検査前）に
  置くこと。builtin/user 層の `trusted` key は従来どおり受理
  し、bool 以外の値は従来どおり `schema_error` とすること
  （受理層での型検査規約は不変）。`KNOWN_KEYS` への `trusted`
  登録は維持する（reject は層 gate で行い、未知 key 拒否とは
  別規約とする）。
- R3. `Route.trusted` field を削除すること — 唯一の consumer
  は merge の shadowing gate であり provenance へ置き換わる。
  `SuppressedRoute`・`InvalidRouteGroup`・`RouteLayer`・
  `EffectiveRoutes`・`RouteResolution`・`route resolve` 出力
  shape・`kind:"route"` record shape は変更しない。merge で
  displaced 下位層 route が二度と発生しないため、`suppressed`
  の displaced 記録 arm は到達不能になる — `SuppressedRoute`
  の semantics は「衝突で負けた project route の provenance」
  へ追従記述すること（reporting 経路・観測義務は不変）。
- R4. 次を test で検証すること:
  - project 層 entry の `trusted` 宣言（`true`/`false` 両方）が
    `InvalidRuntimePolicy` で拒否される（presence-first —
    値が非 bool 等の形状不正でも層 gate が先に拒否する）。
    builtin/user 層の `trusted` 宣言は受理される
  - `trusted` を持たない project route が同一 `match.command`
    で builtin/user route に負け、`suppressed` provenance が
    `route resolve` 出力・`kind:"route"` record に到達する
    （現行維持の回帰固定）
  - builtin/user route が下位層を無条件 shadow する現行規約に
    回帰がない
  - 非衝突の project route は従来どおり有効化・発火する
    （`grant:` anchor・byte-matched mode の viability・
    expected_input byte 一致・exit mirror は不変）
- R5. route trust と grant 参照 scope の整合を文書化すること:
  route の trust は宣言層 provenance（builtin/user 層宣言 =
  trusted）、grant の `scope` は authoring 層 provenance
  （project task 由来は `scope:"project"`、trusted 層宣言は
  `scope:"trusted"` 既定で `scope:"project"` 明示は自己弱体化）
  であり、両者は「trust は宣言値ではなく宣言主体が決める」
  単一モデルの route 側・grant 側の 2 面であること。route の
  grant 参照（`grant` anchor・`grants[]` member・
  `substitute.grant`）が `scope=="trusted" && !fixed` の
  grant のみを解決する `is_route_eligible` 単一 seam は本
  変更で不変であり、route の trust 判定と route が使える
  grant の scope 判定は同一の層境界を共有すること。
- R6. 次の不変条件を保持すること:
  - **routing ≠ authorization**（D2）— viability・grant
    eligibility・`expected_input`・record 経路は
    `route.trusted`/`route.layer` を trust 判別として読まない
    （`route.layer` は target.path 層起点解決と診断のための
    provenance として残る）。trust が影響するのは merge
    precedence のみ。
  - **hook 経路の fail-open** — `load_or_none` は `trusted`
    presence 拒否を含むあらゆる load 失敗を `None`（route
    arm 不活性）へ畳む現行規約不変。project 層 `trusted`
    宣言は fail-open でその workspace の project routing
    全体を不活性化する（fail-closed な schema 強制と
    fail-open な hook 合成の既存責務分離）。
  - **`suppressed` provenance の報告義務** — 衝突で負けた
    project route は `route resolve` `suppressed` field と
    `kind:"route"` record が報告する（静かに無視しない —
    #167 で確立した displaced 記録と同じ規約で、負けた側を
    記録する）。
  - **fail-loud load** — 未知 key・required 欠落・schema
    違反・層 gate 違反は file 全体を config error にする
    現行規約不変。project 層 `trusted` presence は schema
    error として他の層 gate と同じ fail 経路を辿る。

## Acceptance Criteria

- [ ] `trusted` flag の発行主体が project 自己申告ではなく
      宣言層 provenance（builtin/user 層 = operator/user 層）で
      判定される設計が、spec（本書）と ADR
      （`docs/adr/trusted-route-flag-issuer-redesign.md`）に
      決定・記録されている — issue AC1
- [ ] project 層 routes.yaml の route entry が `trusted` key を
      含むと `InvalidRuntimePolicy` の route-load error で拒否
      され、`trusted: true` を宣言した project route が下位層
      route を shadow できない（宣言経路自体が load 時に閉じる）
      ことが test で確認できる。`trusted` 非宣言の project
      route は衝突時に `suppressed` provenance として報告され
      続ける — issue AC2
- [ ] spec R5 と ADR が、route trust（宣言層 provenance）と
      grant `scope`（authoring 層 provenance — route grant 参照
      は `scope=="trusted" && !fixed` のみ解決、#166 grant-set・
      #167 substitute の `is_route_eligible` 単一 seam）の整合
      を文書化している — issue AC3
- [ ] `Route.trusted` field と merge の `route.trusted` 読取りが
      除去され、builtin/user 層の shadowing・同一層 dup `invalid`
      隔離・`route resolve` 出力 shape・record shape・hook
      fail-open に回帰がない（既存 route/hook/run test 群 pass
      — `trusted` 前提の test arm は本 spec に合わせて更新）
- [ ] `trusted` を含む project routes.yaml が load 失敗する破壊
      的変更が spec Scope/AC と #169 migration guide 向け材料と
      して明記されている（修復 = 当該 key の削除 1 行）

## Deferred Design Concerns

- operator/user 側 consent registry（project route shadow の
  個別許可） | project-tuned workflow で trusted 層 route を
  置き換える正当用途が顕在化した場合の復元経路 | operator が
  workflow を `<userConfigDir>/` へ vendor して user 層 route を
  宣言する代替は可能（`target.path` の層起点制約で user 層は
  project file を直接指せない）だが、需要が確立した時点で
  trusted 層宣言機構として別 schema で追加可能 | D7 で却下済み
  の「user-level opt-in list」を provenance 時代の形に再設計
  する必要がある（consent key・staleness semantics の設計未着手）
- builtin/user 層の inert `trusted` key の完全削除 | 現行は
  受理・bool 検証・inert（documented 現状維持） | routes.yaml
  `version` bump の機会に schema から除去可能 | inert key を
  残す表現 drift は小さいが、schema としての正直性は version
  bump 時に回収するのが自然
- `SuppressedRoute` semantics の狭義化 | displaced 記録 arm が
  到達不能になった — doc comment 追従で対応済み | merge
  semantics に新たな置換経路が将来追加された場合に provenance
  の意味を再評価 | 現時点では「負けた project route」のみを
  運ぶ報告経路として不変

## Sources and Relations

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/168
  （権限モデル再設計 6/7 — body は
  `docs/tasks/file-issue-trusted-flag-issuer-redesign.md`
  TASK-034KvxFX6oCd4TlEowM1kd に canonical 保存）
- 上流 discovery:
  `../discovery/trusted-route-flag-issuer-redesign-issue-168.md`
  （DISC-034L44O8jYRDhvhM16gDiM — 実測接点・選択肢比較）
- 親 chain: `../specs/permission-model-responsibility-redesign.md`
  （SPEC-034KuqPkTK2b4mHsykH9A5 — R4 信頼境界機構の再定義）・
  `../designs/permission-model-responsibility-redesign.md`
  （DESIGN-034KuDkWOmNXiBddEQ4XWK — issue 分解 6/7・M3 grant
  scope・残存リスク記述）・
  `../discovery/permission-model-responsibility-redesign.md`
- flag の現行定義（再記述対象 — cross-chain ため references
  止まり）:
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （SPEC-034J7SxWvGL3oSbeBpo8a3 — R1 field 一覧・R7 層間
  trust）・
  `../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  （ADR-034J8eZHNPqARi25HLFOL6 — D7 で `trusted:true` を
  explicitness mechanism として確定・user-level opt-in list
  却下理由）
- 層 provenance 先例:
  `../specs/route-grant-set-projection.md`
  （SPEC-034KTMfXYD5ZlGZCRb8Paj — `grants` project 層拒否
  O7a）・`../adr/route-grant-set-projection.md`・
  `../specs/substitute-exec-route-mode.md`
  （SPEC-034KXdoK2bMG3gQtIj8l6h — `substitute` project 層拒否
  O3a）・`../adr/substitute-exec-route-mode.md`
- grant scope モデル: `../specs/unified-exec-grant-model.md`
  （`scope: trusted|project` provenance・`is_route_eligible`
  単一 seam）・`../adr/enforce-declared-contracts-over-content-audit.md`
  （宣言契約強制の責務モデル）
- user-facing 記述（実装時に追従）:
  `../guides/parent-agent-integration.md`・
  `../guides/configuration.md`・`crates/cheaper-cli/resources/routes.yaml`

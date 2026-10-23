---
id: "TSPEC-034L59Ghpuk8XWUN8qgXJu"
type: "test-spec"
status: "approved"
title: "trusted route flag issuer redesign の検証契約 (issue #168)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/168"
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
    - "docs/plans/trusted-route-flag-issuer-redesign.md"
    - "docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/test-specs/substitute-exec-route-mode.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
  derived-by: []
  verifies:
    - "docs/specs/trusted-route-flag-issuer-redesign.md"
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route_cmd.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign の検証契約 (issue #168)

## Purpose

issue #168(#151 実装分解 6/7)で確定した「route の trust 判定を
宣言値(`Route.trusted` flag)から宣言層 provenance(`layer ∈
{Builtin, User}` = trusted、`layer == Project` = untrusted)
へ移す再設計」を、merge semantics・層 gate schema・field
削除・suppressed provenance・fail-open・grant `scope` 整合・
破壊的変更告知の各面で固定する。
SPEC-034LwdoInh3ie1KPlSjkJl の R1-R6・Acceptance Criteria
(AC1-AC5)+ issue AC1-AC3 と
DESIGN-034L4tqYwtc8q88WEIXPh9 の Verification Notes(11 項目)
が authoritative。

保つべき意図は 4 つ。(1) **判別子の単一化**: trust 判別に
使うのは宣言層 provenance 1 つ — merge が `route.trusted`
宣言値を読む経路・merge 外の trust 分岐の新設は禁止。(2)
**presence-first の fail-closed**: untrusted 層(project)の
`trusted` 宣言は値の形状・値(`true`/`false`/非 bool)を評価
せず presence で `schema_error` → `InvalidRuntimePolicy` へ
倒す — 「`false` か absent のみ許容」という incoherent な
意味論を作らない(O2d 不採用)。(3) **観測義務の維持**:
衝突で負けた project route は `suppressed` provenance として
`route resolve` 出力と `kind:"route"` record が報告する —
静かに無視しない(報告経路・shape は不変、運ぶのは「負けた
project route」のみ)。(4) **失敗方向の両経路安全性**:
schema 強制は fail-closed(file が load error で明示的に
止まる)、hook 合成は fail-open(`load_or_none` が load 失敗を
`None` へ畳み route arm 不活性・command は直接実行継続) —
既存の責務分離を維持する。

退役条件: route trust の判別が宣言層 provenance から別机制へ
変わる・`suppressed` provenance の reporting 契約が変わる・
project 層への宣言権境界(`grants`/`substitute`/`trusted`
層 gate 群)が変わるまで存続する。builtin/user 層の inert
`trusted` key が routes.yaml `version` bump で除去されても
(design §Deferred)本規約の層 gate 構造は変わらない。

## Feature

merge(`route::routes::load`)の project 分岐は
`route.trusted` を一切読まない無条件形 — `match.command`
衝突時は project route が常に負けて `suppressed` provenance
へ記録され、非衝突時は有効化される(displaced 下位層 route
の記録 arm は到達不能で除去)。`validate_route` は project
層 route entry の `trusted` key presence を `passthroughEnv`
parse 後・`trusted` bool 形状検査前に `"{label}.trusted:
not declarable in the project layer"` の `schema_error` →
`InvalidRuntimePolicy` で拒否し(`grants`/`substitute` 層
gate と同じ判別・位置・error class)、builtin/user 層の
`trusted` は受理・bool 検証・inert を維持する(値は `Route`
に保持しない — field は削除済み)。`Route.trusted` field は
除去され、`SuppressedRoute`・`InvalidRouteGroup`・
`RouteLayer`・`EffectiveRoutes`・`RouteResolution`・
`route resolve` 出力 shape・`kind:"route"` record shape は
不変。hook 経路では `load_or_none` が presence 拒否を含む
load 失敗を `None` へ畳み全層の route arm を不活性化する。

## Rules

- Rule: **merge trust = 宣言層 provenance — project 分岐の
  無条件形**(spec R1・ADR O1a — `routes.rs` unit test)
  - merge の `RouteLayer::Project` 分岐は `route.trusted`
    を一切読まない — `effective.contains_key(&route.command)`
    のみで分岐し、衝突時は project route 自身を
    `suppressed.push(SuppressedRoute{name, command})` へ、
    非衝突時は `order.push` + `insert` へ進む。
  - displaced 下位層 route を `suppressed` へ記録する arm
    (#167 が導入した `insert` 戻り値経路)は到達不能のため
    除去される — 新無条件形では project route の `insert`
    は非衝突時にのみ走り、戻り値は常に `None`。unreachable
    arm を防御的に残さない。
  - builtin/user 分岐は文面不変 — user は builtin を無条件
    shadow する現行規約(同一 trusted principal の宣言
    override — silent のまま、suppressed 記録なし)。
  - 層 gate で `trusted` presence が load 時に閉じるため、
    merge が flag を読む経路は構造的に存在しない(trust は
    `route.layer` が決める — O1a)。

- Rule: **project 層 `trusted` presence 拒否 —
  presence-first fail-loud**(spec R2・issue AC2・ADR O2a —
  `routes.rs` unit `project_layer_trusted_is_rejected` +
  `route.rs` integration `resolve_project_trusted_rejected`)
  - project 層(`<project_dir>/routes.yaml`)の route entry
    が `trusted` key を含む場合、`validate_route` は**値の
    形状検査より先に** presence で `schema_error(file,
    "{label}.trusted: not declarable in the project
    layer")` → `InvalidRuntimePolicy` を返す fail-loud
    route-load error — `details.{path, reason:"schema",
    violation}` が file と entry(`routes[N].trusted`)を
    名指しする。
  - `trusted: true`・`trusted: false`・非 bool 値(例:
    `trusted: str`)の 3 形とも同じ violation で拒否される
    — untrusted 層の宣言は内容を評価しない(`grants`
    O7a/`substitute` O3a と同じ fail-closed・同じ宣言層
    provenance 判別)。値が非 bool 等の形状不正でも層 gate
    が先に拒否する(presence-first)。
  - gate 位置は `grants`/`substitute` gate と同じ「該当 key
    の値形状検査直前」規約 — `passthroughEnv` parse 後・
    `trusted` bool 検査前。層 gate 群の発火順は schema 記述順
    (grants → substitute → trusted)で、複数 gated key を
    持つ entry は先に来る gate の violation を報告する
    (逐次 fail-loud)。判別に新規 param は不要 —
    `layer: RouteLayer` は `validate_route` の既存 param。
  - `trusted` key を含む project routes.yaml は **file 全体**
    load 失敗 — file 内の非衝突 route も不活性化する
    (破壊的変更)。`route resolve` 経路では
    `INVALID_RUNTIME_POLICY`・exit 3 になる
    (`resolve_project_trusted_rejected` —
    `resolve_project_substitute_rejected` と同型の pin)。
  - 「`trusted: true` を宣言した project route が下位層を
    shadow する」は**発生しない** — 宣言経路自体が load
    時に閉じる(「宣言しても判定を変えられない」ではなく
    「申告を受理しない」で issue AC2 を最強形で満たす)。

- Rule: **builtin/user 層 `trusted` 受理・inert** —
  `KNOWN_KEYS` 維持(spec R2 — `routes.rs` unit)
  - builtin/user 層の `trusted: true`・`trusted: false` 宣言
    は受理される(documented 現状維持 — その層では元来意味を
    持たない)。非 bool 値は従来どおり `{label}.trusted:
    not a boolean` の `schema_error` → `InvalidRuntimePolicy`
    (受理層の型検査規約不変)。
  - parse は bool 形状検査のみへ縮小し、値は `Route` に保持
    しない(`if let Some` 形で「検査するが使わない」—
    `let _ =` や discard binding は使わない)。値が `Route`
    に載らないことは `Route.trusted` field 削除後の compile
    が証明する。
  - `KNOWN_KEYS` の `"trusted"` 登録は維持される —
    `KNOWN_KEYS` は全層共通のため project 層の拒否は
    unknown-key 規約ではなく明示の層 gate で行う(O2c
    不採用: builtin/user の inert 宣言まで一律に壊す
    gratuitous な破壊を避ける)。`trusted` の typo(`trustd`
    等)は従来どおり unknown key として route-load error
    (fail-loud 規約不変)。

- Rule: **`Route.trusted` field 削除・shape 不変**(spec R3 —
  compile + `usage_log.rs` record shape pin)
  - `Route` struct から `trusted` field(doc comment ごと)
    が削除され、全 `Route` literal(`validate_route` 内・
    test helper `resolve.rs`/`routing_policy.rs`・routes.rs
    unit literal)から `trusted` 指定が除去される —
    `PartialEq`/`Clone`/`Debug` derive は不変。
  - `Route.layer` が trust 判定・診断・`target.path` 層起点
    解決(`run.rs` file_root)の単一 provenance source になる
    — `trusted` の値を別 field・別表現へ移さない(semantic
    input として消滅、表現 drift なし)。
  - `SuppressedRoute`・`InvalidRouteGroup`・`RouteLayer`・
    `EffectiveRoutes`・`RouteResolution`・`route resolve`
    出力 shape・`kind:"route"` record shape(`schemaVersion`)
    は不変 — `usage_log.rs` の record shape pin test
    (:298-353)と `route.rs` の出力 shape assertion が
    無改訂で pass する契約。

- Rule: **suppressed provenance — 衝突で負けた project route
  のみを運ぶ**(spec R3/R4/R6 — `routes.rs` unit + `route.rs`/
  `route_run.rs` integration)
  - `trusted` 非宣言の project route が同一 `match.command`
    で builtin/user route に負けたとき、負けた project route
    名が `suppressed` provenance へ記録され、`route resolve`
    出力 `suppressed` field(`route_cmd.rs`)と
    `kind:"route"` record(`run.rs` → `usage_log.rs`)へ
    既存経路で到達する — Option 型・shape 不変(観測義務:
    静かに無視しない)。
  - `SuppressedRoute` semantics は「衝突で負けた project
    route」へ狭義化される — displaced 下位層 route が
    `suppressed` に載る経路は二度と発生せず、「`suppressed`
    が下位層 route 名を報告する」観測は存在しない(test
    側の assert も追従: `merge_records_displaced_lower_layer`
    rewrite・route_run untrusted arm のみ残す)。
  - consumer は不変: `resolve` の `suppressed()` lookupは
    `.find(is_prefix)` で負けた project route 名を拾い、
    best-match 経路・no-match 経路とも既存どおり。

- Rule: **builtin/user 無条件 shadow・同一層 dup 隔離の回帰
  無し**(spec R4 — `routes.rs` unit)
  - user が builtin を無条件 shadow する現行規約に回帰がない
    — rewrite 後の `user_shadows_builtin_and_project_never_
    shadows` で builtin/user silent shadow(project
    suppressed 記録なし)を pin。
  - 同一層内の同一 `match.command` 重複は `invalid` bucket
    へ隔離され resolve 対象から除外される現行規約
    (`same_layer_duplicates_are_quarantined_as_invalid`)は
    無改訂で pass。

- Rule: **非衝突 project route の有効化不変**(spec R4 —
  `routes.rs` unit + 既存 integration 群)
  - 非衝突 `match.command` の project route は従来どおり
    有効化・発火する — `grant:` anchor 宣言・byte-matched /
    substitute mode の viability・`expected_input` byte 一致・
    exit mirror は不変(routes.rs unit の非衝突 arm + 既存
    `route.rs`/`route_run.rs` test 群が無改訂で pass)。

- Rule: **hook 経路 fail-open — project 層違反で全層 route
  arm 不活性**(spec R6 — `tests/integration/hook.rs`
  integration test)
  - `load_or_none`(`route/supply.rs`)は `trusted` presence
    拒否を含むあらゆる load 失敗を `None`(route arm 不活性)
    へ畳む現行規約不変 — fail-closed な schema 強制と
    fail-open な hook 合成の責務分離を維持する。
  - 違反 file は **project 層** `<workspace>/.cheaper/
    routes.yaml` に置くこと — 既存の hook-route fixture
    (`routed_shell_fixture` — routes.yaml/policy/workflow を
    `f.cfg` = user 層へ配置)は layer gate の対象外となる
    ため再使用不可(builtin/user 層の `trusted` は inert
    受理で違反にならない)。
  - user 層に valid な route があっても、project 層 file の
    load 失敗で **builtin/user/project 全層の route arm が
    不活性化**し、routed だった command は直接実行経路
    (unrouted eval — 例: route arm 非発火で Dedup record を
    publish する deny 側経路)へ落ちて継続する。strict 経路
    (`route resolve`/`route run` prepare)は error をそのまま
    返し fail-open しない — 両経路の非対称を pin。
  - 症状は debug log のみで `route resolve` 実行まで可視化
    されない — migration guide(#169)で明記する材料として
    本契約を固定する。

- Rule: **grant `scope` seam 不変・route/grant trust の整合
  文書化**(spec R5・issue AC3 — docs 確認 + code 不変 pin)
  - route trust(宣言層 provenance)と grant `scope`(authoring
    層 provenance — project task → `scope:"project"`、trusted
    層宣言は `scope:"trusted"` 既定・`scope:"project"` 明示は
    自己弱体化)は「trust は宣言値ではなく宣言主体が決める」
    単一モデルの 2 面であり、route grant 参照(`grant`
    anchor・`grants[]` member・`substitute.grant`)が
    `scope=="trusted" && !fixed` のみを解決する
    `is_route_eligible` 単一 seam(`policy/exec.rs`)は
    **predicate・comment ともに変更なし**。
  - route 側の「誰が宣言したか」と grant 側の「誰が
    authoring したか」が同じ層境界を共有する記述が
    `guides/parent-agent-integration.md`(grant 参照記述)へ
    反映されている — code 変更を伴わない文書化契約。
  - project partial schema の `command` 節除外(grant
    authoring は trusted 層のみ)も不変。

- Rule: **routing ≠ authorization・fail-loud load・shape 不変
  の回帰無し**(spec R6・spec AC4 — 既存 route/hook/run
  test 群)
  - viability・grant eligibility・`expected_input`・record
    経路は `route.trusted`/`route.layer` を trust 判別として
    読まない(D2)— `route.layer` は target.path 層起点解決と
    診断の provenance として残る。trust が影響するのは
    merge precedence のみ。
  - 未知 key・required 欠落・schema 違反・層 gate 違反は
    file 全体を config error にする fail-loud 規約不変 —
    project 層 `trusted` presence は他の層 gate と同じ
    `schema_error` → `InvalidRuntimePolicy` 経路を辿る。
  - `route_cmd` 出力 shape・`payload` schema・`workflow_run`
    gate・`usage_log`/`usage_report`・`policy/exec.rs`・
    `policy/load.rs`・`tools/*`・`hook/targets.rs`・
    `routed-exec.workflow.json`・`tests/integration/
    usage_log.rs`・`proc_invocation.rs` は無改訂で維持される
    — `trusted` 前提 arm の更新済み test を除き既存
    route/hook/run test 群が pass(spec AC4 — 更新対象は削除
    ではなく「presence 拒否検証 + 負け側 suppressed 回帰
    固定」への置き換え)。

- Rule: **破壊的変更の告知材料**(spec AC5・design §Deferred
  — doc 確認)
  - `trusted` key を含む project routes.yaml が file 全体
    load 失敗する破壊的変更が spec Scope/AC と design
    §Deferred(operator migration path — 修復 = 当該 key の
    削除 1 行、`trusted: true` で shadow を意図していた場合は
    workflow を `<userConfigDir>/` へ vendor し user 層 route
    を宣言)として明記されている — #169 migration guide へ
    渡せる記述。
  - `project routes.yaml` を userConfigDir へ丸ごとコピー
    すれば `trusted: true` は user 層で inert 受理されるため
    意図した shadowing が無改修で復活する migration 経路が
    design §Risks に記録されている。

- Rule: **full pipeline**(`mise run check` 全緑 +
  `node scripts/check-doc-ids.mjs` + `audit_docs` 0 error —
  design Verification Notes 末尾・spec AC4)

## Examples

- Example: project routes.yaml の route entry に
  `trusted: true` → `load` が `InvalidRuntimePolicy`
  (violation `routes[0].trusted: not declarable in the
  project layer`、file・entry 名指し)で file 全体を拒否。
  `trusted: false`・`trusted: str`(非 bool)でも同じ
  violation — presence-first で値を見ない。`route resolve`
  経路では exit 3(spec R2・issue AC2・design VN1/VN4)。
- Example: builtin/user 層 routes.yaml の `trusted: true`・
  `trusted: false` は受理され `Route` 構築に進む(値は保持
  されない)。`trusted: yes` のような非 bool 値は
  `{label}.trusted: not a boolean` の `schema_error`
  (spec R2・design VN2)。
- Example: user 層 route `[cargo, test]`(grant: user-grant)+
  `trusted` 非宣言の project route `[cargo, test]` →
  effective は user route、`suppressed` が project route 名
  `proj-test` を記録し、`route resolve` 出力・
  `kind:"route"` record の `suppressed` に乗る
  (spec R4・design VN3 — `merge_records_displaced_lower_layer`
  rewrite 後は project-lose 記録のみ assert)。
- Example: builtin/user 層間の shadow(user → builtin)は同一
  trusted principal の宣言 override として silent のまま —
  `suppressed` は空(spec R4・design VN5 —
  `user_shadows_builtin_and_project_never_shadows`)。
- Example: `trusted` 非宣言の project route が非衝突
  `match.command`(`[make, build]` 等)を宣言 → 通常有効化・
  `suppressed` 空・従来どおり発火(spec R4・design VN6)。
- Example: `trusted` key + `grants` key を併記した project
  entry → 層 gate 発火順は schema 記述順で grants gate が先
  — violation は `routes[0].grants: not declarable …`
  (`project_layer_grants_are_rejected` の併記 arm comment
  追従対象 — spec R2)。
- Example: workspace `.cheaper/routes.yaml` に `trusted: true`
  を含む file + user 層に valid な `cat` route + grant policy
  → hook firing は `load_or_none` で route arm 不活性へ落ち、
  `cat big.txt` は routed rewrite せず unrouted eval
  (dedup record を publish する経路)で処理される
  (spec R6・design VN8 — 違反 file は project 層)。
- Example: `Route { … }` literal に `trusted:` を残すと
  compile error — field 削除の churn 完結は compiler が網羅
  (`grep -rn "Route {" crates/` sanity + `route.trusted`・
  `.trusted` 残存 grep — spec R3・design VN7)。
- Example: route の grant 参照(`grant` anchor・`grants[]`
  member・`substitute.grant`)は `is_route_eligible`
  (`scope=="trusted" && !fixed`)のみを解決 — `scope:"project"`
  の grant 名参照は viability `NoGrant` へ fold(#166/#167
  規約不変 — spec R5・design VN9)。

## Guarantees

- 実装者が design を再解釈しなくても、層 gate の位置・
  presence-first 規約・error 文言・merge 無条件形・field
  削除 churn・suppressed semantics 狭義化・fail-open 責務
  分離・grant `scope` 整合の全項目が unit + integration +
  既存 test 群で機械的に検証できる。
- project 層の `trusted` 自己申告経路は load 時に構造的に
  閉じ、「untrusted checkout が `trusted: true` を書くだけで
  trusted 層 route の `match.command` prefix を乗っ取る」
  唯一の宣言経路が消える(issue AC2 の最強形)。
- trust 判別子が宣言層 provenance 1 つに統一され、merge が
  `route.trusted` を読む経路・merge 外の trust 分岐・
  `trusted` 値の別表現移行が存在しないことが compile +
  grep sanity で pin される。
- `trusted: false`・非 bool 値を含む全形が同一 violation で
  拒否され、形状不正で gate を迂回する経路がないことが pin
  される(presence-first)。
- builtin/user 層の `trusted:` 受理・inert と `KNOWN_KEYS`
  維持が pin され、受理層を壊さない境界(O2c 不採用の理由)
  が保たれる。
- 衝突で負けた project route が `route resolve` 出力と
  `kind:"route"` record に報告され続け、「負けた側を記録
  する」観測義務が trust 判定変更後も維持される — shape・
  報告経路は不変。
- hook 経路の fail-open が project 層違反 file で pin され、
  schema fail-closed・hook fail-open の責務分離が regression
  test で固定される。
- 破壊的変更の症状(全件 load 失敗・hook では全層 routing
  不活性)と修復(key 削除 1 行・user 層 vendor 経路)が
  docs/Deferred として記録され、#169 migration guide の材料に
  なる。

## Non-goals

- operator/user 側 consent registry(O1b — project route
  shadow の個別許可宣言機構)— 需要未確認で YAGNI。
  forward-compat は保持するが本 spec は検証しない。
- builtin/user 層の inert `trusted` key の schema 完全除去
  (routes.yaml `version` bump が必要 — Deferred)。
- `suppressed`/`invalid`/`route resolve` 出力 shape・
  `kind:"route"` record shape・`schemaVersion` の変更検証
  (不変規約 — 変わらないことの pin のみ)。
- viability・grant eligibility・`expected_input`・exit
  mirror・`is_route_eligible`/`project_grants`/
  `check_exec_execution`/`pin_exec_grant(s)` の挙動検証
  (全て不変 — 既存 test 群の無改訂 pass で担保)。
- project route の新規 prefix 宣言権・`grant:` anchor 宣言権
  の変更(不変 — 従来どおり非衝突 prefix で宣言・発火)。
- Option E(exec-less route)・文書正直性 sweep・migration
  guide 本文(#169 が担う)。
- 検証コマンド・実行結果の記録 — task の `## Verification`
  と impl record の責務。

## Risk

- 層 gate を値形状検査の後・または unknown-key 経路で実装
  すると、presence-first の fail-closed 契約(非 bool も
  `not declarable` で拒否)と `KNOWN_KEYS` 維持(builtin/user
  inert 受理)の両方が崩れる — gate 位置・文言・
  `project_layer_trusted_is_rejected` の 3 形 pin が防衛線。
- `Route.trusted` field を残したまま merge のみ変更すると
  「merge は読まないが field だけ残る」表現 drift になり
  trust 判別子の単一化が未完に終わる — field 削除 +
  literal churn + grep 残存確認が必要。
- displaced 記録 arm を防御的に残すと「project が insert で
  置換し得る」誤読を構造に残し、将来の merge 変更時に
  `suppressed` semantics が再び曖昧になる — 到達不能 arm は
  code からも消す規約。
- `suppressed` semantics の狭義化に test 追従が伴わないと、
  「`suppressed` が下位層 route 名を報告する」はずの旧
  assert が「もう発生しない観測」を要求して fail する —
  rewrite 対象(merge_records_displaced_lower_layer・
  resolve_project_trusted_rejected・route_run untrusted arm)
  の pin が必要。
- hook fail-open test が既存 `routed_shell_fixture`(user 層
  `f.cfg`)を再利用して `trusted` を書くと、user 層受理で
  違反にならず「fail-open しない」誤った pin になる —
  違反 file は project 層 `<workspace>/.cheaper/routes.yaml`
  に置く必須条件。
- 破壊的変更の告知が docs に届かないと、hook 経路の routing
  不活性が debug log のみで不可視のまま operator に届く —
  症状(`route resolve` まで不可視)と診断・修復手順の明記が
  T3 の責務。
- `Route.layer` への provenance 集約を docs/comment が追随
  しないと、「project は `trusted: true` で shadow する」と
  いう stale 記述が file header・`SuppressedRoute` doc・
  guides・pretooluse spec に残り次の変更を誤誘導する —
  comment 追随 site の全件更新が必要。

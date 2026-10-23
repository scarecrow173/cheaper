---
id: "ADR-034LT2o68CgcOinMEs8PZ8"
type: "adr"
status: "proposed"
title: "trusted route flag issuer redesign — trust = 宣言層 provenance・project 層 `trusted` presence 拒否"
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
  depends-on:
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/trusted-route-flag-issuer-redesign.md"
    - "../specs/route-grant-set-projection.md"
    - "../specs/substitute-exec-route-mode.md"
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "./unified-exec-grant-model.md"
    - "./enforce-declared-contracts-over-content-audit.md"
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
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../designs/substitute-exec-route-mode.md"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../specs/unified-exec-grant-model.md"
    - "../tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/configuration.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# trusted route flag issuer redesign — trust = 宣言層 provenance・project 層 `trusted` presence 拒否

## Context and Problem Statement

route の trust は現行 `Route.trusted` bool で表される — project
層 route にのみ意味を持つ optional field で（`routes.rs:85-87`、
`routes.rs:627-633` で bool として parse）、読む consumer は
3 層 merge（builtin → user → project）の shadowing gate
（`routes.rs:201`）のみである。merge 規則: builtin/user 層
route は同一 `match.command` で下位層を無条件 shadow し、
project 層 route は `trusted: true` のときのみ下位層を shadow
する — `trusted: true` で置換された displaced 下位層 route は
`suppressed` provenance へ記録され（`routes.rs:210-218`）、
trust 不在で衝突した project route 自身も `suppressed` へ記録
される（`routes.rs:201-205`・`resolve.rs:221-228` が
`route resolve` 出力と `kind:"route"` record へ届ける）。

問題はこの flag が **project 自身の宣言で効く**ことである。
ADR-034J8eZHNPqARi25HLFOL6 D7 は `trusted: true` を
「explicitness の mechanism であり consent の mechanism では
ない」と明確に位置付け、残存リスクを「(a) routing≠authorization
で実行は trusted grant の argv match が要る (b) project 新規
prefix route は trust 無しで宣言可能 (c) `.cheaper/routes.yaml`
は review される artifact (d) override は provenance 報告される」
で限定とした。しかし権限モデル再設計（#151 系列）で「trust は
宣言内容ではなく**宣言主体（層 provenance）**が決める」モデルが
確立し、この flag は唯一残った project 自己申告 privilege に
なった:

- grant `scope` — `ExecGrantScope::Trusted|Project`
  （`crates/cheaper-core/src/policy/exec.rs:42-45`）。project
  層宣言 task は `scope:"project"` へ畳まれ（`load.rs:248-257`
  で merge 前に task 名集合を捕捉 → `exec.rs:513-517` で
  provenance 畳み込み）、route の grant 参照は
  `scope=="trusted" && !fixed` のみを解決する
  `is_route_eligible` 単一 seam（`exec.rs:114-116`）。
  trusted 層の `scope:"project"` 明示宣言は自己弱体化としてのみ
  意味を持つ（`exec.rs:612-615`）。
- `grants`（#166 O7a）・`substitute`（#167 O3a）— project 層
  entry の key presence を `InvalidRuntimePolicy` で fail-loud
  拒否（`routes.rs:481-486`・`:520-525`）。判別子は宣言層
  provenance であり、code comment が「`trusted:true` は project
  自己申告の shadowing gate であり判別に使えない」と明記済み。
- `route.rewrite.approvalBypass` — policy `route` 節は
  builtin/user のみ authoring 可で、project policy.yaml の
  `route` key は `InvalidRuntimePolicy`（D14 と同型の層境界）。
- `command` policy 節 — project partial schema が節ごと除外
  （`load.rs:242-247` — grant authoring は trusted 層のみ）。

この経路が残る限り、untrusted な checkout の routes.yaml が
`trusted: true` を書くだけで trusted 層（builtin/user）route の
`match.command` prefix を乗っ取れる — 実行面は grant argv 契約が
制約するが、**どの route がその prefix を所有するか**の判定を
project に委ねているのが issue #168 の是正対象。

実測の consumer 確認（本ブランチ HEAD）: `route.trusted` を
読むのは merge gate のみ（`.trusted` の使用は `routes.rs:201`
と parse `:632` のみ）。viability（`resolve.rs:278-348`）・
grant 解決（`is_route_eligible`/`project_grants`）・record・
`route run` は `route.trusted` を読まない。`route.layer` は
`run.rs:366-369` で target.path の層起点解決にのみ使われ、
trust 判別とは別概念 — **trust が分岐するのは merge precedence
の一点だけ**であり、flag の意味論を層 provenance へ畳む変更の
blast radius は merge + schema gate に閉じる。

## Decision Drivers

- **trust 判定主体は operator/user 層** — project 自己申告の
  解消（issue AC1）。「宣言値としての trusted」を「宣言層としての
  trusted」へ置き換える。
- **判別子の単一化** — `grants`/`substitute`・grant `scope`・
  `route` policy 節・`command` 節除外と同じ「宣言層 provenance +
  project 層 presence 拒否」規約へ揃え、flag 判別と provenance
  判別の二重経路を残さない。
- **project route の宣言権は保全** — 新規 prefix route は trust
  無しで宣言可能（D7 (b) の残存用途）。変わるのは「下位層を
  shadow する privilege」のみであり、非衝突 route・`grant:`
  anchor 宣言・発火は不変。
- **fail-loud schema vs EXISTING key の互換** —
  `grants`/`substitute` は NEW key で拒否に互換 cost が無かった
  が、`trusted` は routes.yaml v1 から受理されている EXISTING
  key。presence 拒否は `trusted: false` を含む既存 file も全て
  壊す — 破壊的変更系列（#169 migration guide が収集）内での
  許容性を評価する必要がある。
- **`suppressed` provenance の観測義務** — 負けた route を黙って
  無視しない（D7・#167 で確立）。trust 判定が変わっても報告
  経路は維持する。
- **YAGNI・forward-compat** — consent registry のような追加機構
  は需要が確立するまで作らない。ただし今日の拒否が将来の
  trusted 層側 consent 宣言を構造的に妨げない形にする。

## Considered Options

### O1 trust 判定主体（issuer mechanism）

- **O1a 宣言層 provenance = trust** — `layer ∈ {Builtin, User}`
  の route が trusted、`layer == Project` の route は untrusted
  とし、merge gate が `route.trusted` 宣言値ではなく宣言層を
  読む（実質、project 分岐は「衝突 → `suppressed`」の無条件
  形）。`trusted` flag は semantic input として消滅し、trust
  判定の主体は「宣言がどの層に置かれたか」= operator/user 層
  そのものになる。`grants`/`substitute`・grant `scope`・
  approvalBypass・`command` 節除外と同一の判別子で、route
  trust と grant scope が「誰が宣言したか」の同一層境界を
  共有する。project 層が下位層を shadow する経路は構造的に
  消える（flag 申告も registry も要らない — 判定は宣言位置
  に畳み込まれる）。
- **O1b operator/user 側 consent registry** — trusted 層 file
  （例: `<userConfigDir>/routes.yaml` や別 file）が「この
  project route（名・command・hash 等で key 化）は shadow
  可」を明示宣言する consent 機構。D7 で却下済みの
  user-level opt-in list と同型で、却下理由はそのまま残る:
  第二 config surface・staleness semantics（name/path-keyed
  は後続編集まで信任・hash-keyed は正当な編集の度に失効）・
  route file と分離した trust 宣言 file の split-brain。consent
  としては強いが、(i) 需要が未確認で YAGNI、(ii) `route.trusted`
  を消して別機構を新設する二段の変更になる、(iii) consent
  対象（route 宣言そのもの vs shadow privilege）の設計が
  未着手。project 側が `trusted` を書けない現行形を維持する
  だけでも consent の構造的余地は残る（trusted 層側宣言機構
  として後付け可能 = forward-compatible）。
- **O1c flag を残し宣言層を builtin/user に制限** — flag 自体は
  存続させ、「trusted 層のみ宣言可」とする案。動作は O1a と
  同一（builtin/user は flag によらず無条件 shadow で flag は
  inert、project 層の宣言は拒否）— 差異は `trusted` key の
  schema 処遇（受理継続 vs 拒否）だけであり、O1 の issuer
  軸ではなく O2 の key 処遇軸へ fold する。
- **O1d signed/approved artifact list** — O1b の暗号化版
  （operator が署名・approve した route 目録）。key 管理・
  revocation・offline 検証の全コストを乗せて consent を得る
  案。脅威モデル上の要求も需要も未確立で、YAGNI の範囲を
  大きく超える。

### O2 project 層 `trusted` key の処遇（ignore vs reject）

- **O2a presence を fail-loud 拒否** — project 層 entry の
  `trusted` key presence を `"{label}.trusted: not declarable
  in the project layer"` として `schema_error` →
  `InvalidRuntimePolicy` へ倒す（`grants`/`substitute` と同じ
  gate 位置・error class・presence-first 規約 — untrusted 層の
  宣言は内容を評価しないため `trusted: false` も拒否）。
  `trusted` key は trust 申告そのものであり、untrusted 層は
  「申告を置くこと自体」ができない — issue AC2（宣言しても
  判定を変えられない）を「申告経路が load 時に閉じる」まで
  強める最強の形。builtin/user 層の `trusted` は受理・inert
  を維持（その層では元来意味を持たない documented 現状 —
  schema 上の trust 申告は trusted 層でのみ置ける、という
  「申告権 = 層」の読みと一致）。
- **O2b ignore（受理・破棄）** — `KNOWN_KEYS` に `trusted` を
  残し bool 検証後に値を discard。load 破壊はゼロで、semantic
  変化は「`trusted: true` を持つ衝突 route が shadow しなく
  なる」一点に限定される（`suppressed` provenance 記録で
  観測可能）。ただし file が `trusted: true` と宣言しながら
  system が一切意味を持たない **zombie key を恒久に残す** —
  D7 が「silent override」を explicit 宣言へ変換した方向を、
  「explicit 宣言が silent に inert 化する」逆向きに復活
  させる。`grants`/`substitute` で確立した「privileged 宣言は
  presence 拒否」の統一規約からの carve-out にもなり、
  「この key は書けるが何もしない」という表現 drift を
  schema に残す。
- **O2c `KNOWN_KEYS` から除去（全層 unknown-key 拒否）** —
  key を schema から削除し「field は存在しない」に倒す最も
  正直な形だが、user 層・builtin 層の `trusted:` 宣言（その
  層では元来 inert で、意味を持たないものを書いていただけの
  file）まで一律に load error にする — 意味の無かった層の
  宣言を壊す gratuitous な破壊を伴い、得るものは「key が
  全層で死んだ」対称性のみ。
- **O2d `trusted: true` のみ拒否（`false` 受理）** — 「宣言
  するなら効果の無い値のみ可」という規約は presence-first
  discipline（untrusted 層の宣言内容を評価しない）と矛盾し、
  「`trusted` は `false` か absent のみ許容」という意味論
  自体が incoherent（`false` を書く意味が存在しない）。

**ignore vs reject の互換性評価**: O2a は `trusted` key を含む
project routes.yaml を**全件 load 失敗**にする — file 内の
非衝突 route も全て不活性化し、hook 経路では fail-open で
その workspace の project routing 全体が落ち、strict 経路
（`route resolve`/`route run`）は `INVALID_RUNTIME_POLICY`。
これは `grants`/`substitute`（NEW key — 拒否で壊れる既存
file が存在しなかった）には無かった実質 cost。緩和材料:
(a) 対象は `trusted` を書いた file のみ = まさに除去対象機能の
利用者であり、無関係な file への波及は無い (b) 破壊的変更系列の
#169 migration guide で告知でき、修復は当該 key の削除 1 行
(c) 失敗方向は fail-closed — file が黙って意味を失うのでは
なく load error で明示的に止まり、作者が変更に気付ける
(d) 「`trusted` を書いた file」は shadowing を意図した file で
あり、ignore でも semantic が変わる（shadow しなくなる）以上、
「壊れるなら loud に壊れる」方が fail-loud 哲学と整合する。
O2b を選ぶ理由は「既存 file の load 互換」だけだが、これは
`trusted: true` の file が黙って shadow を失う形（loud な
破壊を避けて silent な semantic 変化を選ぶ）— 本 repo が
route file 全体に適用してきた fail-loud 規約と逆向き。

## Decision Outcome

採用: **O1a + O2a**。

- route の trust は**宣言層 provenance** — `layer ∈ {Builtin,
  User}` の route は同一 `match.command` で下位層を無条件
  shadow（現行不変）、`layer == Project` の route は `trusted`
  宣言に関わらず下位層を shadow しない。merge の project
  分岐は「衝突 → `suppressed` provenance 記録・非衝突 →
  有効化」の無条件形になり、`route.trusted` を一切読まない。
- project 層 routes.yaml の `trusted` key presence は
  `"{label}.trusted: not declarable in the project layer"` で
  `InvalidRuntimePolicy` + `schema_error` へ倒す（`grants`/
  `substitute` と同じ gate 位置・error class・presence-first —
  `trusted: false` も拒否）。builtin/user 層の `trusted` は
  受理・bool 検証・inert を維持。`KNOWN_KEYS` 登録は維持
  （拒否は層 gate で行い、未知 key とは別規約）。
- `Route.trusted` field を削除（唯一の consumer は merge
  gate であり provenance へ置き換わる）。`SuppressedRoute`・
  `RouteLayer`・`EffectiveRoutes`・`route resolve` 出力
  shape・`kind:"route"` record shape は不変 — merge で
  displaced 下位層 route が二度と発生しないため
  `suppressed` は「衝突で負けた project route」のみを運ぶ
  （displaced 記録 arm は到達不能。doc comment 追従）。
- project route の非衝突 prefix 宣言・`grant:` anchor 宣言・
  発火・viability・expected_input・exit mirror は不変。
- 破壊的変更: `trusted` key を含む project routes.yaml は
  全件 load 失敗（#169 migration guide 向け材料 — 修復は
  当該 key の削除 1 行）。
- O1b consent registry は不採用（YAGNI + D7 却下理由の存続）
  — 将来、operator が project route の shadow を個別許可する
  需要が確立した場合、trusted 層側の新規宣言機構として追加
  可能（本決定は project 層 `trusted` key を閉じるだけであり
  forward-compat を損なわない）。
- route trust（宣言層 provenance）と grant `scope`（authoring
  層 provenance — project task → `scope:"project"`、trusted
  層 grant → `scope:"trusted"` 既定・`scope:"project"` 明示は
  自己弱体化）は「trust は宣言値ではなく宣言主体が決める」
  単一モデルの 2 面であり、route grant 参照の
  `is_route_eligible`（`scope=="trusted" && !fixed`）単一
  seam は不変。

### Consequences

* Good, because trust 判別子が「宣言層 provenance」1 つに
  統一され、flag 判別と provenance 判別の二重経路が消える
  — `grants`/`substitute`・grant `scope`・`route` policy 節・
  `command` 節除外と同じ規約が route trust にも適用され、
  「trust は宣言値ではなく宣言主体が決める」モデルが route・
  grant の両面で閉じる。
* Good, because project 層の `trusted` 宣言経路自体が load
  時に閉じ、untrusted checkout が trusted 層 route の
  `match.command` prefix を乗っ取る唯一の宣言経路が構造的に
  消える — issue AC2 を「宣言しても判定を変えられない」では
  なく「申告を受理しない」まで強める。
* Good, because `Route.trusted` field の削除で `Route` の
  意味論から privilege flag が消え、`Route.layer` provenance
  が trust・診断・target.path 層起点解決の単一 source になる
  — merge gate が読むのは layer のみで、判別の重複表現が無い。
* Good, because 失敗方向が両経路で安全側: presence 拒否は
  fail-closed（file が load error で明示的に止まる）、衝突は
  trusted 層 route が勝つ（project route は `suppressed` として
  報告され silent ではない）。
* Good, because `suppressed` provenance・`route resolve` 出力・
  `kind:"route"` record の reporting 経路は shape 不変で、
  「負けた route を黙って無視しない」規約は trust 判定変更
  後も維持される。
* Bad, because `trusted` key を含む**既存の** project
  routes.yaml は全件 load 失敗する（`trusted: false` の inert
  宣言・file 内の非衝突 route を含む）— `grants`/`substitute`
  の NEW key には無かった EXISTING key の互換破壊。#169
  migration guide で告知し、修復は当該 key の削除 1 行だが、
  hook 経路では fail-open でその workspace の project routing
  全体が不活性化する（route arm が落ちて command は直接実行
  される — 実行自体は継続）。
* Bad, because project 層が下位層 route を shadow する正当
  用途（D7 が残した「project-tuned workflow で routed command
  を tuned する」経路）が構造的に消える — operator が同等を
  実現するには workflow を `<userConfigDir>/` へ vendor して
  user 層 route を宣言する必要がある（`target.path` の層起点
  制約で user 層は project file を直接指せない）。
* Bad, because builtin/user 層の `trusted` key は受理・inert の
  まま残る（zombie ではあるが、その層では元来 inert の
  documented 現状維持 — routes.yaml `version` bump 時の
  schema 除去へ defer）。
* Bad, because `route.trusted` を前提にした既存 test arm
  （merge test の trusted shadow arm・integration の displaced
  provenance 記録 test）は新規約への書き換えが要る —
  「project `trusted` が効く」前提の検証は「presence 拒否」
  検証へ置き換わる。

### Confirmation

- spec（`docs/specs/trusted-route-flag-issuer-redesign.md`）の
  Acceptance Criteria が実装 AC。
- issue #168 の Acceptance Criteria（発行主体の設計決定・記録 /
  project 層 `trusted` 宣言が trust 判定を変えられないことの
  test 確認 / grant 参照 scope との整合文書化）が spec R1-R6
  と AC で全て覆われていること。
- discovery（`docs/discovery/trusted-route-flag-issuer-redesign-issue-168.md`
  の file:line inventory）が merge gate・schema gate・
  suppressed 報告・grant scope seam の全接点を網羅している
  こと。

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/route/routes.rs`
  （`Route.trusted` field 削除・merge project 分岐の無条件化・
  `validate_route` への `trusted` 層 gate 追加・`SuppressedRoute`
  doc comment 追従）、`crates/cheaper-cli/src/route/resolve.rs`
  （test constructor の `trusted` field 除去）、
  `crates/cheaper-cli/src/hook/routing_policy.rs`（test
  constructor 同上）、`crates/cheaper-cli/tests/integration/
  {route,route_run}.rs`（`trusted` 前提 arm の新規約化）、
  `crates/cheaper-cli/src/route/routes.rs` 内 unit test（merge・
  layer-gate test）、`crates/cheaper-cli/resources/routes.yaml`
  （shape 記述）、`docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （R1 field 一覧・R7 層間 trust の追従記述）、
  `docs/guides/parent-agent-integration.md`・
  `docs/guides/configuration.md`（route 宣言層説明）。
* Patterns to follow: `grants`/`substitute` の project 層
  presence 拒否（`routes.rs:481-486`・`:520-525` —
  `layer == RouteLayer::Project && entry.contains_key` を
  `schema_error`/`InvalidRuntimePolicy` へ倒す判別・gate 位置・
  `"{label}.<key>: not declarable in the project layer"`
  文言）、grant `scope` の authoring 層 provenance
  （`exec.rs:513-517`・`:612-615`）、`load_or_none` の
  fail-open 合成（`supply.rs:138-152` — load 失敗を `None`
  へ畳む既存規約に presence 拒否も乗る）。
* Patterns to avoid: `route.trusted` 宣言値を判別子として読む
  経路の復活・merge 外での trust 分岐の新設（viability・
  eligibility・record は layer を読まない）・`KNOWN_KEYS` から
  の `trusted` 除去（builtin/user 層の inert 受理を壊さない —
  層 gate で拒否する）・`suppressed`/`invalid`/resolve 出力
  shape 変更・consent registry の先行実装（YAGNI）。
* Dependencies or configuration: 新規 dep なし。#166 の
  `suppressed` provenance・`grants` 層 gate、#167 の
  `substitute` 層 gate・displaced 記録、#165 の
  `ExecGrantScope`/`is_route_eligible` provenance を前提。

## Verification

* [ ] project 層 routes.yaml の `trusted` key presence（`true`・
      `false`・非 bool — presence-first で値を見ない）が
      `InvalidRuntimePolicy` の route-load error で拒否され、
      builtin/user 層の `trusted` 宣言は受理される
* [ ] `trusted` を宣言した project route が下位層 route を
      shadow できない（宣言経路が load 時に閉じる）こと、
      `trusted` 非宣言の project route が同一 `match.command`
      で builtin/user route に負けて `suppressed` provenance
      として `route resolve` 出力・`kind:"route"` record に
      到達することが test で確認できる
* [ ] builtin/user route が下位層を無条件 shadow する現行規約・
      同一層 dup `invalid` 隔離に回帰がない
* [ ] 非衝突の project route が従来どおり有効化・発火する
      （`grant:` anchor・viability・expected_input byte 一致・
      exit mirror 不変）
* [ ] `Route.trusted` field・merge の `route.trusted` 読取りが
      除去され、`Route`/`SuppressedRoute`/resolve 出力/record
      の shape が不変
* [ ] hook 経路の fail-open — `trusted` presence 拒否を含む
      load 失敗が route arm 不活性へ畳まれ、command 直接実行
      経路は継続する
* [ ] spec R5 の記述どおり、route trust（宣言層）と grant
      `scope`（authoring 層）の整合が文書化され、route grant
      参照が `scope=="trusted" && !fixed` のみ解決する
      `is_route_eligible` 単一 seam に変更がない
* [ ] 既存 route/hook/run test 群が pass（`trusted` 前提 arm
      は新規約へ更新 — 削除ではなく拒否検証・suppressed
      回帰固定へ置き換え）

## Pros and Cons of the Options

### O1a 宣言層 provenance = trust（採用）

* Good, because trust 判別子が `grants`/`substitute`・grant
  `scope`・approvalBypass・`command` 節除外と同一の「宣言層
  provenance」に統一され、「trust は宣言主体が決める」単一
  モデルが route・grant の両面で閉じる。issuer は追加機構
  なしに宣言位置へ畳み込まれる。
* Good, because `Route.trusted` を semantic input として消せ、
  merge gate が読むのは `route.layer` のみ — `Route` の
  意味論から privilege flag が消えて表現 drift が無い。
* Good, because project route の非衝突宣言権・`grant:` anchor・
  発火は不変で、失うのは「下位層を shadow する privilege」
  のみ — 変化面が flag の唯一の semantics に限定される。
* Bad, because project 層の shadowing 用途が構造的に消え、
  「project-tuned workflow で trusted 層 route を置き換える」
  正当用途は operator 側の vendor（user 層 route + workflow
  の `<userConfigDir>/` 配置）を要する — `target.path` の
  層起点制約で user 層は project file を直接指せない。

### O1b operator/user 側 consent registry

* Good, because consent としては最も強く、「この project route
  は shadow 可」を operator が個別に許可できる — D7 が
  「consent としてはより強い」と認めた設計。
* Bad, because 第二 config surface・staleness semantics
  （name/path-keyed は後続編集まで信任・hash-keyed は正当な
  編集の度に失効）・route file と分離した trust 宣言 file の
  split-brain — D7 の却下理由がそのまま残り、需要が未確認
  のまま新機構を作る YAGNI 違反。将来の復元経路は trusted
  層宣言機構として残る（forward-compat）ため、需要確立時に
  別 schema で追加する方が誠実。

### O1c flag 維持・宣言層を builtin/user に制限

* Good, because flag を消さない見た目の保守性。
* Bad, because 動作は O1a と同一（builtin/user は flag によらず
  無条件 shadow で flag は inert）で、差異は `trusted` key の
  schema 処遇だけ — issuer 軸の選択ではなく O2 の key 処遇
  軸の話であり、独立した option としての設計内容を持たない。

### O1d signed/approved artifact list

* Good, because operator の approve を暗号的に検証できる最強形。
* Bad, because key 管理・revocation・offline 検証の全コストを
  乗せる割に、脅威モデル上の要求も需要も未確立 — YAGNI の
  範囲を大きく超える。

### O2a presence を fail-loud 拒否（採用）

* Good, because `grants`/`substitute` と同じ gate・error
  class・presence-first 規約で、「privileged 宣言は project
  層で受理しない」統一規約に flag 自身も収まる — `trusted`
  key は trust 申告そのものであり、untrusted 層は申告を置く
  こと自体ができない読みと一致。
* Good, because issue AC2 を「宣言しても判定を変えられない」
  ではなく「申告経路が load 時に閉じる」まで強め、fail
  方向は fail-closed（loud に止まる）で file 作者が変更に
  気付ける。「黙って意味を失う file」を残さない。
* Bad, because `trusted` を含む**既存の** project routes.yaml
  は全件 load 失敗する（`trusted: false` の inert 宣言・
  file 内の非衝突 route を含む）— EXISTING key への presence
  拒否は NEW key には無かった互換破壊であり、hook 経路では
  fail-open でその workspace の project routing 全体が不活性
  化する。#169 migration guide で告知・1 行削除で修復。

### O2b ignore（受理・破棄）

* Good, because load 破壊がゼロで、semantic 変化は「`trusted:
  true` を持つ衝突 route が shadow しなくなる」一点に限定
  される — `suppressed` provenance 記録で喪失は観測可能。
* Bad, because file が `trusted: true` と宣言しながら system
  が一切意味を持たない zombie key を恒久に残し、D7 が解消
  した「silent override」を「explicit 宣言が silent に inert
  化する」逆向きで復活させる。`grants`/`substitute` の
  presence 拒否統一規約からの carve-out で、「書けるが何も
  しない key」という表現 drift を schema に残す — 本 repo
  の fail-loud 哲学と逆向きに silent な semantic 変化を選ぶ
  ことになる。

### O2c `KNOWN_KEYS` から除去（全層拒否）

* Good, because「field は存在しない」という schema 正直性は
  最大。
* Bad, because user 層・builtin 層の inert `trusted:` 宣言
  （その層では元来意味を持たないものを書いていただけの
  file）まで一律に load error にする gratuitous な破壊を伴い、
  得るものは「key が全層で死んだ」対称性のみ — 意味の無い
  層の宣言を壊す cost に見合わない。

### O2d `trusted: true` のみ拒否（`false` 受理）

* Bad, because presence-first discipline（untrusted 層の宣言
  内容を評価しない）と矛盾し、「`false` か absent のみ許容」
  という意味論自体が incoherent — `false` を書く意味が存在
  しない key の半端な残し方になる。

## More Information

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/168
  （権限モデル再設計 6/7 — body は
  `docs/tasks/file-issue-trusted-flag-issuer-redesign.md`
  TASK-034KvxFX6oCd4TlEowM1kd に canonical 保存）
- 上流 discovery:
  `../discovery/trusted-route-flag-issuer-redesign-issue-168.md`
- spec: `../specs/trusted-route-flag-issuer-redesign.md`
- flag の現行定義（本決定で書き換わる部分 — cross-chain ため
  references 止まり）:
  `./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  （ADR-034J8eZHNPqARi25HLFOL6 — D7 で `trusted:true` を
  explicitness mechanism として確定・user-level opt-in list
  却下理由が O1b 評価の土台）・
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （SPEC-034J7SxWvGL3oSbeBpo8a3 — R1 field 一覧・R7 層間
  trust）
- 層 provenance 先例: `./route-grant-set-projection.md`
  （ADR-034KTMnPFDF25x710LFSKY — `grants` project 層拒否
  O7a）・`./substitute-exec-route-mode.md`
  （ADR-034KXk8h1AtwCkf2cQrb7m — `substitute` project 層拒否
  O3a・「`trusted:true` は判別に使えない」の確立）
- grant scope モデル: `./unified-exec-grant-model.md`
  （`ExecGrantScope` provenance・`is_route_eligible` 単一
  seam）・`./enforce-declared-contracts-over-content-audit.md`
  （宣言契約強制の責務モデル）
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  （issue 分解 6/7・M3 grant scope・残存リスク記述）・
  `../specs/permission-model-responsibility-redesign.md`
- consent registry（O1b）は不採用 — 需要確立時に trusted 層
  宣言機構として追加可能（本決定は project 層 `trusted` key
  を閉じるのみで forward-compat を損なわない）

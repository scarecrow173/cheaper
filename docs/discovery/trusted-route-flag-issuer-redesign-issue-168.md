---
id: "DISC-034L44O8jYRDhvhM16gDiM"
type: "discovery"
status: "resolved"
title: "trusted route flag issuer redesign issue-168"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/trusted-route-flag-issuer-redesign.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../designs/route-grant-set-projection.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../specs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../designs/substitute-exec-route-mode.md"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../specs/unified-exec-grant-model.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/configuration.md"
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign issue-168

## Exploration Goal

GitHub issue #168（権限モデル再設計 6/7: trusted route flag
発行主体の再設計 D4）の実装検討。`trusted:true` を project
自身が宣言できる現状の再設計 — trust 判定主体を operator/user
層へ移す方式の決定と実装。route trust model は grant 参照
scope（trusted-scoped grant のみ解決 — M3/#166・substitute
#167）と直に交差するため依存完了後の着手。depends on
#163/#166/#167（全完了済み）。

### Scope（issue 本文）

- `trusted` route flag の発行主体を operator/user 層へ移す
  方式を決定し実装する。project 自身が `trusted:true` を
  宣言できる現状を解消する。
- route の trust model を grant 参照 scope（trusted-scoped
  grant のみを解決 — M3 / #166）と整合させて設計する。
- 残存リスクの扱い: 本 issue 完了までは現行と同じ曝露
  （project が自ら `trusted` を宣言できる）が残る — 悪化は
  しないが放置もしない。

### briefing で解く論点（issue/依頼が明示した設計問い）

1. 発行主体 mechanism — (a) 層 provenance = trust（project
   層 route は無条件 untrusted・`trusted` field を
   ignored/rejected）、(b) operator/user 側の許可 list、
   (c) flag 維持 + builtin/user 限定宣言、(d) signed/approved
   list。
2. project `trusted` を無効化した時に壊れるもの — trusted
   shadowing に依存する既存 project route（破壊的変更 →
   #169 migration guide 記録）。merge `suppressed` semantics
   は displaced route を記録し続けるか。
3. 互換性 — project 層 `trusted` を ignore（silent no-op）
   にするか fail-loud reject にするか（`grants`/`substitute`
   は presence 拒否だが NEW key であり、`trusted` は
   EXISTING accepted key — `KNOWN_KEYS`/schema 契約を含めて
   評価）。
4. 「trust」が意味するもの — `route.trusted` を読む code
   path（shadowing だけか、viability 等他でも使うか）。

## Key Issues and Assumptions

briefing 実測（本ブランチ HEAD 時点）で確定した論点:

1. **`Route.trusted` の唯一の consumer は merge の shadowing
   gate** — `routes.rs:85-87` で field 宣言（project 層のみ
   意味を持つ・builtin/user は inert の documented 現状）、
   `:627-633` で bool parse（inert でも parse は許容）、
   `KNOWN_KEYS :388-398` に `"trusted"` 登録（`:397`）。
   `.trusted` の読取りは全 crate で `routes.rs:201` のみ
   （merge の project 分岐 `if effective.contains_key(...) &&
   !route.trusted`）。viability・grant 解決・record・
   `route run` は `route.trusted` を読まない — **trust が
   分岐するのは merge precedence の一点だけ**（論点 4 確定:
   shadowing only）。

2. **merge 規則の現行形** — `routes.rs:182-222` が builtin →
   user → project の層順で merge。builtin/user 分岐
   （`:192-197`）は無条件 insert（user は builtin を無条件
   shadow）。project 分岐（`:200-220`）は「衝突 && !trusted →
   自身を `suppressed` 記録（`:201-205`）」「衝突 && trusted
   → insert + displaced 下位層 route を `suppressed` 記録
   （`:210-218`）」「非衝突 → insert」の 3 形。`SuppressedRoute`
   （`:121-129` — `{name, command}`）は `resolve.rs:221-228`
   が `route resolve` 出力 `suppressed` field と
   `kind:"route"` record へ届ける。`route.layer` は trust と
   別概念で `run.rs:366-369` が target.path の層起点解決に
   使う（`RouteLayer` enum `:31-36`、層 path 組立 `:161-177`）。

3. **層 provenance 判別の先例がコードに確立済み** —
   `grants`（#166 O7a、`routes.rs:481-486`）・`substitute`
   （#167 O3a、`:520-525`）は project 層 entry の key
   presence を `schema_error` → `InvalidRuntimePolicy` で
   fail-loud 拒否（presence-first — untrusted 層の宣言は
   内容を評価しない）。両 gate の code comment が
   「`trusted:true` は project 自己申告の shadowing gate で
   あり判別に使えない — 判別子は宣言層 provenance」と明記
   （`:476-480`・`:510-519`）— flag 自身が唯一残った
   自己申告 privilege。

4. **grant `scope` = authoring 層 provenance** —
   `ExecGrantScope::Trusted|Project`（`exec.rs:42-45`）。
   project 層宣言 task は `load.rs:248-257` で merge 前に
   task 名集合を捕捉 → `exec.rs:513-517` で `scope:"project"`
   へ畳み込み、trusted 層宣言 grant は既定 `scope:"trusted"`
   で `scope:"project"` 明示は自己弱体化（`:612-615`）。
   project partial schema は `command` 節ごと除外
   （`load.rs:242-247` — grant authoring は trusted 層のみ）。
   route grant 参照（`grant`/`grants[]`/`substitute.grant`）は
   `is_route_eligible`（`scope=="trusted" && !fixed` —
   `exec.rs:114-116`）の単一 seam で判定 — route trust と
   grant scope は同一層境界を共有するモデルの 2 面。

5. **ignore vs reject の非対称** — `grants`/`substitute` は
   NEW key で presence 拒否の互換 cost がゼロだったが、
   `trusted` は routes.yaml v1 から受理される EXISTING key。
   presence 拒否（O2a）は `trusted` を含む project
   routes.yaml を全件 load 失敗にする（`trusted:false` の
   inert 宣言・file 内の非衝突 route を含む — hook 経路は
   `load_or_none` `supply.rs:138-152` で fail-open に畳まれ
   project routing 全体が不活性化、strict 経路は
   `INVALID_RUNTIME_POLICY`）。ignore（O2b）は load 破壊
   ゼロだが zombie key（file が `trusted:true` と書きながら
   system が意味を持たない）を恒久に残す — D7 が解消した
   「silent override」を「explicit 宣言の silent inert 化」
   として逆方向に復活させる。

6. **D7 の却下済み consent 機構** —
   ADR-034J8eZHNPqARi25HLFOL6 D7（`docs/adr/pretooluse-…:
   280-319`）は `trusted:true` を「explicitness の mechanism
   であり consent ではない」と位置付け、user-level opt-in
   list（consent としては強いが第二 config surface・staleness
   semantics が割に合わない）と strict precedence（project
   tuned workflow の正当用途を排除）を却下済み。本 issue は
   「self-declared」の残存リスクを発行主体移行で解消する
   もので、consent registry（O1b）は需要未確認で YAGNI —
   ただし trusted 層宣言機構としての後付け余地（forward-
   compat）は残す。

7. **test 接点** — `routes.rs:800`（`user_shadows_builtin_
   and_project_needs_trusted` — trusted shadow arm）、
   `:1025-1031`・`:1252-1258`（`trusted:true` + `grants`/
   `substitute` の判別非効 test — 新規約では trusted presence
   自体が拒否）、`tests/integration/route.rs:285-336`
   （`resolve_project_trusted_displacement_reports_suppressed`）、
   `route_run.rs:1386-1455`（`suppressed_project_provenance_
   reaches_route_record` の trusted arm）、test constructor
   （`resolve.rs:434`・`routing_policy.rs:2090`）が
   `Route.trusted` field 依存。

## Line Reference 実測表（接点一覧）

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `Route.trusted` field | `crates/cheaper-cli/src/route/routes.rs:85-87` | `bool` — project 層のみ意味・builtin/user inert |
| merge project 分岐 | `routes.rs:198-220` | `trusted:true` で下位層 shadow + displaced 記録 |
| `.trusted` 読取り | `routes.rs:201` | 全 crate で唯一の consumer |
| `trusted` parse | `routes.rs:627-633` | bool 検証・全層受理（inert でも parse） |
| `KNOWN_KEYS` | `routes.rs:388-398` | `"trusted"` 登録（`:397`） |
| `grants` 層拒否 | `routes.rs:481-486` | presence-first `schema_error`/`InvalidRuntimePolicy` |
| `substitute` 層拒否 | `routes.rs:520-525` | 同上（`grants` と同型 gate） |
| `SuppressedRoute` | `routes.rs:121-129` | `{name, command}` — merge 敗北 provenance |
| `RouteLayer` | `routes.rs:31-36` | `Builtin`/`User`/`Project` — load 順 `:161-177` |
| `suppressed` 報告 | `resolve.rs:221-228`, `:246`, `:266` | `route resolve` `suppressed` field → record |
| `route.layer` 用途 | `run.rs:366-369` | target.path 層起点解決（trust とは別概念） |
| `is_route_eligible` | `policy/exec.rs:114-116` | `scope=="trusted" && !fixed` 単一 seam |
| `ExecGrantScope` | `policy/exec.rs:42-45` | `Trusted`|`Project` — authoring 層 provenance |
| project task provenance | `policy/load.rs:248-257`, `exec.rs:513-517` | merge 前 task 名集合捕捉 → `scope:"project"` |
| `command` 節除外 | `policy/load.rs:242-247` | `ProjectPartial` schema — grant authoring は trusted 層 |
| hook fail-open | `route/supply.rs:138-152` | `load_or_none` — load 失敗を `None`(route arm 不活性)へ |

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| O1a 宣言層 provenance = trust | `grants`/`substitute`・grant `scope`・approvalBypass・`command` 節除外と同一判別子・`Route.trusted` を消せる（consumer は merge gate のみ）・issuer が機構なしに宣言位置へ畳み込まれる | project の shadowing 用途が構造的に消える（operator は user 層 route + workflow vendor で代替 — `target.path` 層起点で直接参照不可） | **採用** |
| O1b operator/user consent registry | consent 最強・個別許可 | D7 却下理由の存続（第二 config surface・staleness・split-brain）・需要未確認で YAGNI・trusted 層宣言機構として後付け可能で今日不要 | 不採用（forward-compat 残置） |
| O1c flag 維持 + builtin/user 限定 | flag を消さない見た目の保守性 | 動作は O1a と同一（trusted 層では flag inert）— issuer 軸ではなく key 処遇（O2）の話で独立した設計を持たない | 不採用（O2 へ fold） |
| O1d signed/approved list | 暗号的 approve 検証 | key 管理・revocation・offline 検証の全コスト — 脅威モデル・需要とも未確立 | 不採用 |
| O2a project `trusted` presence 拒否 | `grants`/`substitute` と同 gate・error class・presence-first — 「申告を置くこと自体」を untrusted 層ができない・AC2 を最強に満たす・fail-closed で作者が気付ける | `trusted` を含む既存 project routes.yaml 全件 load 失敗（`trusted:false`・非衝突 route を含む — EXISTING key ゆえの互換破壊、#169 で告知・1 行削除で修復） | **採用** |
| O2b ignore（受理・破棄） | load 破壊ゼロ・semantic 変化は shadowing 喪失一点（suppressed で観測可） | zombie key — file が `trusted:true` と書きながら system が意味を持たない・presence 拒否統一規約からの carve-out・「silent override」解消の逆向き復活 | 不採用 |
| O2c `KNOWN_KEYS` 除去（全層拒否） | 「field は存在しない」schema 正直性最大 | user/builtin 層の inert 宣言（元来意味なし）まで一律に壊す gratuitous な破壊 | 不採用 |
| O2d `trusted:true` のみ拒否 | — | presence-first（内容を評価しない）と矛盾・「`false` か absent のみ許容」の意味論が incoherent | 不採用 |

## Tentative Conclusions and Hypotheses

- **決定**: O1a + O2a（採用 root は
  `docs/adr/trusted-route-flag-issuer-redesign.md`
  ADR-034LT2o68CgcOinMEs8PZ8、spec は
  `docs/specs/trusted-route-flag-issuer-redesign.md`
  SPEC-034LwdoInh3ie1KPlSjkJl）。
- trust 判定: `layer ∈ {Builtin, User}` = trusted。merge の
  project 分岐は「衝突 → `suppressed` 記録・非衝突 → 有効化」
  の無条件形 — `route.trusted` を一切読まない。`Route.trusted`
  field は削除（consumer は merge gate のみ）。
- key 処遇: project 層 `trusted` presence は
  `"{label}.trusted: not declarable in the project layer"`
  → `InvalidRuntimePolicy` + `schema_error`（`grants`/
  `substitute` と同 gate 位置・presence-first — `false` も
  拒否）。builtin/user 層は受理・bool 検証・inert 維持、
  `KNOWN_KEYS` 登録は維持（層 gate で拒否 — 未知 key とは
  別規約）。
- `suppressed` semantics: project route は二度と下位層を
  置換しないため displaced 記録 arm は到達不能 —
  `suppressed` は「衝突で負けた project route」のみを運ぶ
  （shape・reporting 経路不変、doc comment 追従）。
- 整合: route trust（宣言層 provenance）と grant `scope`
  （authoring 層 provenance）は同一層境界を共有する「trust
  は宣言主体が決める」モデルの 2 面。route grant 参照の
  `is_route_eligible`（`scope=="trusted" && !fixed`）単一
  seam は不変。
- 破壊的変更: `trusted` を含む project routes.yaml は load
  失敗（修復 = key 削除 1 行）+ project 層の shadowing 用途
  消滅（operator は user 層 route + workflow vendor で代替）
  — #169 migration guide 向け材料。

## Open Questions

- operator/user 側 consent registry（O1b）— project route の
  shadow を operator が個別許可する機構。需要が確立した時点で
  trusted 層宣言機構として別 schema で追加可能（本決定は
  project 層 `trusted` key を閉じるのみで forward-compat を
  損なわない）。D7 却下理由（staleness・split-brain）の再設計
  が前提。
- builtin/user 層の inert `trusted` key の完全削除 — 現行は
  受理・inert（documented 現状維持）。routes.yaml `version`
  bump の機会に schema から除去するのが自然（本 issue では
  行わない — spec Out of Scope・Deferred Design Concerns）。
- `SuppressedRoute` の displaced 記録 arm 到達不能化に伴う
  semantics 狭義化 — doc comment 追従で対応。merge semantics
  に新たな置換経路が将来追加された場合に再評価。

## Promotion Candidates

- [x] spec-doc needed: yes → `docs/specs/trusted-route-flag-issuer-redesign.md`
      （SPEC-034LwdoInh3ie1KPlSjkJl — provenance trust・
      presence 拒否・field 削除・整合記述の intent）
- [x] adr-doc needed: yes → `docs/adr/trusted-route-flag-issuer-redesign.md`
      （ADR-034LT2o68CgcOinMEs8PZ8 — O1 issuer・O2 key 処遇
      の決定記録）
- [ ] design-doc needed: no — 依頼は briefing artifacts のみ
      （spec + ADR + resolved discovery）。実装設計は別
      design flow で扱う。

## Sources and Relations

- issue #168: https://github.com/scarecrow173/cheaper/issues/168
  （権限モデル再設計 6/7 — body は
  `docs/tasks/file-issue-trusted-flag-issuer-redesign.md`
  TASK-034KvxFX6oCd4TlEowM1kd に canonical 保存）
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  （DESIGN-034KuDkWOmNXiBddEQ4XWK — issue 分解 6/7・M3
  grant scope・残存リスク記述）・
  `../specs/permission-model-responsibility-redesign.md`
  （SPEC-034KuqPkTK2b4mHsykH9A5 — R4 信頼境界機構の再定義）・
  `../discovery/permission-model-responsibility-redesign.md`
- flag の現行定義（cross-chain — references 止まり）:
  `../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  （ADR-034J8eZHNPqARi25HLFOL6 — D7 で `trusted:true` を
  explicitness mechanism として確定・user-level opt-in list
  却下理由）・
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （SPEC-034J7SxWvGL3oSbeBpo8a3 — R1 field 一覧・R7 層間
  trust）
- 層 provenance 先例（cross-chain — references 止まり）:
  `../specs/route-grant-set-projection.md`・
  `../adr/route-grant-set-projection.md`・
  `../designs/route-grant-set-projection.md`・
  `../discovery/route-grant-set-projection-issue-166.md`
  （#166 — `grants` project 層拒否 O7a・suppressed 記録）・
  `../specs/substitute-exec-route-mode.md`・
  `../adr/substitute-exec-route-mode.md`・
  `../designs/substitute-exec-route-mode.md`・
  `../discovery/substitute-exec-route-mode-issue-167.md`
  （#167 — `substitute` project 層拒否 O3a・displaced 記録）
- grant scope モデル: `../specs/unified-exec-grant-model.md`
  （`ExecGrantScope` provenance・`is_route_eligible` 単一
  seam — #165 M3）・
  `../adr/enforce-declared-contracts-over-content-audit.md`
  （宣言契約強制の責務モデル）
- user-facing 記述（実装時に追従）:
  `../guides/parent-agent-integration.md`（route 宣言層説明）・
  `../guides/configuration.md`（grant scope・route 宣言）

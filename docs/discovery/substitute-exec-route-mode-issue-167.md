---
id: "DISC-034KWGjJMiSKU5wBxcznho"
type: "discovery"
status: "resolved"
title: "substitute-exec route mode issue-167"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
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
    - "../specs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
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
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../specs/unified-exec-grant-model.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../tasks/file-issue-substitute-exec-route-mode.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# substitute-exec route mode issue-167

## Exploration Goal

GitHub issue #167（権限モデル再設計 5/7: substitute-exec route
mode）の実装検討。route が `substitute:{grant,argv}` を宣言
できる substitute-exec mode — 非 exec routing deferred sketch
（ADR-034KtTzOJGqmzmBW3SwGp5）の採用要素（Option D）。precheck
の strict さを保ったまま「宣言通り実行する command」を検査
する。depends on #166（grant-set 投影・record split・viability
分割 = 完了済み）。

### Scope（issue 本文）

- route `substitute:{grant,argv}` 宣言を追加。
- viability の検査対象を `substitute.{grant,argv}` へ retarget
  （原 argv でなく substitute を `check_command_execution` で
  検査）。
- `expected_input`（`route/run.rs` の
  `{name:grant,args:exec_args}`）を substitute mode では
  `{name:substitute.grant,args:substitute.argv}` へ再 anchor、
  exec-input byte 一致（D11）を substitute argv へ適用。
- exit mirror は「実行した command の exit」（substitute 宣言
  あり → substitute exit）。spec R15 は再記述済み。
- grant anchor 規約: `expected_input.name` = `substitute.grant`、
  lifecycle bound は `substitute.grant` 宣言値、usage record
  route 帰属は `route.grant` anchor、`executed` field が
  substitute argv を記録。
- **Option E（exec-less route）は範囲外** — open 事項として
  明示（author 宣言 exit の信用 = 成功偽装 hole 未解消。
  route `grant` field は firing gate として留保される設計だけ
  確定）。

### briefing で解く論点（issue/依頼が明示した設計問い）

1. `substitute` 宣言を trusted 層（builtin/user）限定にする
   か — argv 置換で route が typed command を実行しなくなる
   ため、`route.grant` argv-match のセキュリティ推論が乗り
   移るか。
2. `substitute` が `grants` member set に含まれるか、
   firing gate/anchor が `route.grant` のままか
   `substitute.grant` へ替わるか。
3. 正確な schema 形状と default。
4. substitute viability が #166 の追加 member 検査とどう
   異なるか（member は job argv 未確定で argv 照合を遅延、
   substitute は宣言 argv 確定済みで実実行ペア = 完全検査）。
5. Option E の正確な境界。

## Key Issues and Assumptions

briefing 実測（本ブランチ HEAD 時点）で確定した論点:

1. **schema 接点 — `substitute` は新規 optional nested
   object**。`Route` struct（`routes.rs:37-62`）は `name`/
   `command`/`target_path`/`grant`/`grants`/`token_threshold`/
   `passthrough_env`/`trusted`/`layer`。`KNOWN_KEYS`
   （`:334-343`）に `substitute` は未登録で未知 key は
   fail-loud reject。`grants` の project 層拒否
   （`:421-431` — presence を形状検査より先に
   `schema_error`/`InvalidRuntimePolicy` へ倒す）が層制約の
   先例で、`substitute` も同じ位置・同じ error class で
   拒否する。

2. **viability retarget 接点 — anchor 完全検査が substitute
   ペアへ**。`resolve.rs:276-383` の no-grant 分岐は
   `route.grant` lookup + `is_route_eligible` filter
   （`:312-320`）→ `pin_exec_grant`（`:321-330`）→
   `ExecToolOptions` 構築（`:331-345`）→
   `check_exec_execution(&options, &route.grant, &out.exec_args)`
   （`:346-348`）。substitute 宣言のある route ではこの完全
   検査の対象を `{substitute.grant, substitute.argv}` へ
   向け、`route.grant` は member loop（`:349-377` — 存在 +
   eligibility + pin、argv 照合なし）と同じ検査へ降格する。
   原 argv の `route.grant` 照合は行わない — 実行されない
   command の認可を調べる形骸化検査（deferred sketch・親
   design 採否表「弱めるのは (a) substitute retarget、(b)
   set 内非 exec grant は member 検査」の (a)+(b) に一致 —
   substitute mode の `route.grant` はまさに「set 内の非
   exec grant」）。

3. **member 検査との質的差異** — `grants[]` member を呼ぶ
   job の argv は route 解決時点で未確定（workflow 内で
   構築）のため argv 照合を job 実行時へ遅延させるのに対し、
   `substitute.argv` は route 宣言値で確定済み・実際に実行
   されるペアそのもの。argv 照合が route 解決時に**意味を
   持つ**ため完全 `check_exec_execution` を向ける — これが
   論点 4 の答え。

4. **layer 制約 — `substitute` は trusted 層限定が妥当**。
   byte-matched mode の `route.grant` は `expected_input`
   byte 一致で「実行される exec = typed command」に拘束さ
   れるため project 層でも `grant:` anchor 宣言が許されて
   きた（#166 O7a の argv-match 保護）。substitute mode は
   typed command を一切実行しない — typed command は作者
   制御の `match.command` prefix に退化し、実行対象は宣言
   `{substitute.grant, substitute.argv}` になる。project 層
   受理は「untrusted authoring が任意 typed prefix に対し
   任意 trusted grant を宣言 argv で起動する」能力であり、
   `grants` member（投影へ入るだけ）より強い**実実行
   `{grant,argv}` の選択権** — O4c/O7a が塞いだ審査迂回を
   再発させる。`trusted: true` は shadowing gate で判別に
   使えない（O7a 同型）。判別子は宣言層 provenance。

5. **expected_input/inputs/deadline 接点** —
   `RouteExecution::new`（`run.rs:148-182`）が
   `expected_input = {"name": grant, "args": exec_args}`
   （`:176`）を作り、caller は `:374-382` で
   `prepared.route.grant` + `prepared.resolution.exec_args`
   を渡す。substitute mode では substitute ペアを渡すだけで
   D11 gate（`workflow_run.rs:467-484` — `job_id=="exec" &&
   tool=="command"` のみ byte-match、不一致で
   `exec-input-mismatch`/`PolicyViolation`）は機構無変更で
   効く。inputs（`:384-399`）は `grant`/`execArgs` を実行
   ペアへ、`argv`/`args`/`executable` は原 command identity
   として維持 — builtin `routed-exec.workflow.json` は
   `{name:$input.grant, args:$input.execArgs}` bind のため
   無変更で両 mode に動く。deadline（`run.rs:325-332` +
   grant_timeout lookup `:356-363` — 現行 `route.grant` の
   timeout 由来）は substitute mode では `substitute.grant`
   の宣言値へ追従（親 design「lifecycle bound は
   expected_input が指す実行 grant」）。

6. **projection 接点 — set へ `substitute.grant` を加える**。
   `project_grants(&route.grant_set())`（`run.rs:121-123`、
   `Route::grant_set()` `routes.rs:64-75` =
   `{grant} ∪ grants`）。substitute mode の有効 set は
   `{route.grant} ∪ grants ∪ {substitute.grant} ∪ {fixed}` —
   `substitute.grant` が投影に無いと exec job の
   `resolve_exec_grant` が policy 解決失敗で落ちるため必須。
   `route.grant` は投影 member として残し「投影名は全て
   viability 検証済み」の不変を維持（投影判定位置・全件
   eligible 必須規約は不変）。

7. **record 接点 — `{command, executed}` split が前提整備
   済み**。`build_route_record`（`usage_log.rs:369-422`）は
   `route`（宣言 `name` — `route.grant` anchor の宣言
   identity）+ `commandHash`（原 command — env-prefix 値を
   含み得るため hash のまま）+ `executed:{grant,argv}`
   （`:392-398` — captured `details.{grant,argv}` 由来の
   実測値）。`RouteRecordInput`（`:332-364`）の
   `executed_grant`/`executed_argv` は `finish()`（
   `run.rs:480-504`）が captured exec result details から
   取るため、substitute mode では自動的に substitute ペアを
   記録する — shape・`schemaVersion: 1`・新規 field なし・
   timing（captured result 必須）不変。`exitCode`/`ok` は
   mirror した substitute exit 由来。

8. **stdin-unsupported 分類の対象** — `is_interactive`
   （`resolve.rs:164-173` + `INTERACTIVE_COMMANDS` `:113-158`
   + flag list `:160-162`、適用 `:378-381` で
   `(executable, exec_args)` を評価）は typed 側を維持
   （「interactive typed command は routing で置き換えない」
   invariant）しつつ、substitute 宣言のある route では
   substitute ペア（pin 解決 executable + 宣言 argv）にも
   適用 — stdin 非接続で実行されるのは substitute であり、
   interactive substitute は grant timeout まで hang してから
   失敗するため resolve 時点で `stdin-unsupported` に倒す。

9. **Option E 境界 — routed-exec convention は不変**。
   `validate_routed_convention`（`workflow_run.rs:897-945`）
   は unconditional `exec` job + `outputs.execResult` whole
   binding を必須とし、substitute mode もこの convention の
   内側（exec job 必須 = 実行するのは substitute）で動く。
   Option E（exec job 不要・`outputs.exitCode` 宣言値
   mirror・「何も実行しない」route）は author 宣言 exit を
   信用する成功偽装 hole が未解決・product 需要未確認で、
   親 design が open と確定 — 本 issue では実装しない。

## Side-by-side Inventory（実測 file:line — 本ブランチ HEAD 時点）

### route schema / load

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `Route` struct | `crates/cheaper-cli/src/route/routes.rs:37-62` | `substitute` field 無し |
| `KNOWN_KEYS` | `routes.rs:334-343` | `substitute` 未登録・未知 key fail-loud |
| `grants` 層制約先例 | `routes.rs:421-431` | project 層 `grants` key presence → `schema_error`/`InvalidRuntimePolicy` |
| `grant_set()` | `routes.rs:64-75` | `{grant} ∪ grants` — substitute 追加対象 |
| `validate_route` 層 param | `routes.rs:327-333` | `layer` を受け判別可能 |
| built-in 例 + shape 記述 | `crates/cheaper-cli/resources/routes.yaml` | `grants` 記述のみ・`substitute` 無し |

### viability / resolve

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `Viability` enum | `resolve.rs:20-42` | `Runnable`/`NoGrant`/`MissingTarget`/`InvalidRoute`/`UnsafeCommand`/`StdinUnsupported` — 新規値なし方針 |
| viability 優先順 | `resolve.rs:276-383` | unsafe-command → invalid-route → missing-target → no-grant → member → stdin-unsupported |
| anchor 完全検査 | `resolve.rs:305-348` | `is_route_eligible` + `pin_exec_grant` + `ExecToolOptions` + `check_exec_execution(route.grant, out.exec_args)` — retarget 対象 |
| member 級検査 | `resolve.rs:349-377` | 存在 + `is_route_eligible` + pin のみ — `route.grant` 降格先 |
| `is_interactive` | `resolve.rs:164-173` | `(executable, exec_args)` 評価 — substitute ペア適用対象 |
| `RouteResolution` | `resolve.rs:83-107` | `grant` = `route.grant`（anchor echo）·shape 不変 |

### run / workflow / record

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `expected_input` 構築 | `route/run.rs:176` | `{"name": grant, "args": exec_args}` — substitute ペア再 anchor 対象 |
| `RouteExecution::new` caller | `route/run.rs:374-382` | `prepared.route.grant` + `resolution.exec_args` |
| deadline/grant_timeout | `route/run.rs:325-332`, `:356-363` | `route.grant` timeout 由来 → substitute.grant へ追従 |
| routed workflow inputs | `route/run.rs:384-399` | `grant`/`execArgs` 実行ペア・`argv`/`args`/`executable` 原 identity |
| `project_grants` 呼出 | `route/run.rs:121-123` | `route.grant_set()` 投影・ineligible で `denied("no-grant")` |
| `finish()` exit/executed | `route/run.rs:467-474`, `:480-504` | captured `details.{exitCode,grant,argv}` 由来 |
| D11 byte-match gate | `workflow_run.rs:467-484` | `job_id=="exec" && tool=="command"` のみ — 機構不変 |
| `route_context` attach | `workflow_run.rs:521` | 同 gate 条件 |
| `validate_routed_convention` | `workflow_run.rs:897-945` | unconditional `exec` job + `outputs.execResult` — Option E 境界 |
| builtin workflow | `resources/workflows/routed-exec.workflow.json:17` | `{name:$input.grant, args:$input.execArgs}` — 両 mode で無変更 |
| record shape | `usage_log.rs:332-364`, `:369-422` | `{route, commandHash, executed:{grant,argv}, …}` schemaVersion 1 |

### policy seam（変更不要）

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `is_route_eligible` | `crates/cheaper-core/src/policy/exec.rs:111-116` | `scope=="trusted" && !fixed` — `route.grant`/`grants[]`/`substitute.grant` 同一 seam（comment で substitute 参照を明示済み） |
| `project_grants` | `policy/exec.rs:124-134` | set 全件 eligible 必須・`{named} ∪ {fixed}` を残す |
| `check_exec_execution`/`pin_exec_grant` | `crates/cheaper-tools/src/tools/exec/mod.rs:163-211`, `:275-312` | shared predicate seam — retarget 後も同一呼出し |
| `authorize_args` | `exec/mod.rs:319-333` | argv pattern 照合 |

## Line Reference Drift 訂正表（issue/design 引用 → 実測）

| 引用 | 実測 | 差分 |
| --- | --- | --- |
| issue/design「`expected_input`（`route/run.rs:176`）」 | `route/run.rs:176` | **drift なし** — `RouteExecution::new` 内の `json!` 行 |
| design 採否表「argv precheck（`resolve.rs:304-335`）」 | `resolve.rs:305-348`（`check_exec_execution` 呼出し `:346-348`） | #166 member loop 追加による行 drift（構造不変） |
| design「grant-set 投影 `{route.grant} ∪ grants ∪ {fixed}`」 | `policy/exec.rs:124-134` | 実測一致 — substitute では set に `substitute.grant` を加える |
| sketch「`check_command_execution`」 | `check_exec_execution` | #165 で rename 済み（issue 本文も sketch 名を踏襲） |

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| O1a `substitute:{grant,argv}` nested object | `match`/`target`/`output` と同型・ペア atomic 検証・nested 未知 key 拒否が既存規約に乗る | top-level flat 規約に 1 段深い optional object が増える | **採用** |
| O1b flat field 対（`substituteGrant`/`substituteArgv`） | top-level flat に揃う | 相互 required 規約を新設・partial 宣言を弾く構造が弱い | 不採用 |
| O1c `substitute: "<command string>"` | 直感的 | tokenize/quote/metachar を schema へ持ち込み argv 列契約と表現 drift | 不採用 |
| O1d mode-keyed schema（`mode: substitute`） | mode 排他が明示 | YAGNI（mode 2 種）・presence が switch になる・既存 routes.yaml 影響大 | 不採用 |
| O2a 完全検査を substitute へ retarget・`route.grant` member 級 | 「実行する command」だけ完全検査・「set 内非 exec grant」と同一規則・形骸化検査を除去 | typed argv の発火拘束が `match.command` prefix まで後退（authorization-of-intent 消滅 — 用途上意図的） | **採用** |
| O2b dual check（原 argv も `route.grant` 照合維持） | 発火条件が最保守 | 実行されない command の形骸化検査を復活・「認可したくない command を置き換える」用途を破壊 | 不採用 |
| O2c `route.grant` 無検査・投影外 | 実装最小 | 「投影名は viability 検証済み」不変が壊れ・member job が `route.grant` を呼べず・attribution phantom | 不採用 |
| O3a `substitute` は trusted 層限定（project presence を fail-loud） | 実実行 `{grant,argv}` の選択権を trusted authoring に閉じ・O4c/O7a と同じ判別子（宣言層 provenance）・同じ error class | project route は substitute を宣言不可 — builtin/user 層へ移す必要 | **採用** |
| O3b project 層受理 + 文書化 | schema uniform・実装最小 | 「argv は grant pattern が bound」は「どの grant をどの宣言 argv で起動するか」の選択権を制約せず confused-deputy hole 再発 | 不採用 |
| O3c project 条件付き受理（原 argv が `route.grant` 認可前提） | 発火条件を狭める | substitute ペア選択権は依然 project 作者 — hole の本体を閉じず規約だけ複雑化 | 不採用 |
| O4a `expected_input` = substitute ペア・inputs の `grant`/`execArgs` = 実行ペア | builtin workflow が binding 無変更で両 mode 動作・「exec job input = expected_input」単一規約維持 | `grant` input の値が mode で分岐（意味は「実行 grant」で統一） | **採用** |
| O4b 専用 inputs（`substituteGrant`/`substituteArgv`）追加 | 宣言/実行 anchor を別 input で表現 | builtin workflow が mode 別 binding を要・input 語彙が分かれて表現 drift | 不採用 |
| O4c workflow 宣言 substitute | — | workflow-declared grants と同型の信頼境界迂回・byte-match の意味が反転 | 不採用 |
| O5a record shape 不変（`route` + `commandHash` + `executed`） | #166 record split がそのまま substitute を表現・schemaVersion 1・reader/集計不変 | firing-gate 認可 grant は宣言 join 経由でしか遡れない（現行規約と同じ制約） | **採用** |
| O5b firing-gate anchor の明示 field 追加 | 時点性の anchor が record に凍結 | 「実測 + route identity」規約へ宣言 echo を混ぜる・需要未確立（YAGNI — additive で後付け可） | 不採用 |
| O5c record anchor を `substitute.grant` へ | — | `route` 帰属が宣言軸から外れ per-route 集計が乖離・「実行されなかった command を帰属にしない」目的は `executed` が担う | 不採用 |
| O6a typed 側維持 + substitute ペアにも適用 | 「interactive typed command は routing しない」invariant を mode 共通で保ち・stdin 非接続で実行される substitute 側リスクも resolve 時点で塞ぐ | interactive substitute 宣言・interactive typed command は不発に倒れる面が増える | **採用** |
| O6b substitute のみへ retarget | 「実行されるものだけ検査」と見た目上一致 | interactive typed command の intercept を可能にする invariant 緩和 | 不採用 |
| O6c typed 側のみ維持 | 変更最小 | interactive substitute が grant timeout まで hang してから失敗 — resolve 時点で分類可能なのに未検査 | 不採用 |
| O7a 投影 set = `{route.grant} ∪ grants ∪ {substitute.grant} ∪ {fixed}` | exec job 解決の必須前提・「投影名は viability 検証済み」不変維持・`route.grant` member として残る | `route.grant`（実行しない grant）も投影に残り半径が僅かに広い | **採用** |
| O7b substitute mode で `route.grant` を投影外 | 投影面最小化 | member 非対称・set 検査から anchor が外れる複雑化・attribution が投影外の名を指す矛盾 | 不採用 |
| O8a Option E は open 事項として残置 | author 宣言 exit 成功偽装 hole を前提判断なしに開けない・convention 不変 | lint/analysis route の真需要確認時に別 issue + revisit | **採用** |
| O8b Option E まで本 issue で実装 | — | measured → author-declared の信頼モデル変更は別の大きな判断・未決で実装は hole を開ける | 不採用 |
| O8c Option E を恒久的に不採用 | — | 需要確認時の revisit 経路を閉じる | 不採用 |

## Tentative Conclusions and Hypotheses

- **決定**: O1a + O2a + O3a + O4a + O5a + O6a + O7a + O8a
  （採用 root は `docs/adr/substitute-exec-route-mode.md`、
  spec は `docs/specs/substitute-exec-route-mode.md`）。
- schema: `substitute?: {grant: <non-empty string>, argv:
  [<non-empty string>, …]}` — `argv` は executable を含まない
  args 列（pin が executable を供給）、`argv: []` 合法
  （executable 単独起動）、nested 未知 key・非 object・
  key 欠落は fail-loud。`grant`（単数）は required のまま
  （発火 gate・attribution anchor・投影 member・Option E 側の
  firing gate 確定設計と整合）。`substitute.grant` は
  `route.grant` と同名可（Option B 相当の同一 grant argv
  差替）・`grants[]` と重複可（投影 dedupe）。
- 層制約: `substitute` は trusted 層（builtin/user）限定 —
  project 層 `substitute` key presence を `validate_route` で
  `schema_error`/`InvalidRuntimePolicy`（`grants` と同じ
  error class・presence-first 判定・`{label}.substitute`
  文言）へ倒す。`grant:` 単数・byte-matched route は
  project 層で従来どおり有効。
- viability: substitute 宣言のある route では anchor 完全
  検査（存在 + `is_route_eligible` + `pin_exec_grant` +
  `ExecToolOptions` + `check_exec_execution` argv 照合）の
  対象を `substitute.{grant,argv}` へ retarget、`route.grant`
  は member 級検査（`:349-377` と同じ）へ降格。原 argv
  照合なし（形骸化検査の除去 — 論点 4: member の「job
  argv 未確定ゆえ遅延」とは異なり substitute argv は宣言
  確定済み・実実行ペアのため完全検査が意味を持つ）。
  substitute 側・`route.grant`・member のいずれの失敗も
  `Viability::NoGrant` fold（enum・出力 shape 不変）。
- 投影: `{route.grant} ∪ grants ∪ {substitute.grant} ∪
  {fixed}`（`route.grant_set()` の拡張または run 側で加算 —
  設計詳細）。投影位置・全件 eligible 必須規約不変。
- exec-input: `expected_input = {name: substitute.grant,
  args: substitute.argv}`。D11 gate（`workflow_run.rs:467-484`）
  機構不変。inputs の `grant`/`execArgs` は実行ペア、
  `argv`/`args`/`executable` は原 identity 維持 — builtin
  workflow 無変更。
- lifecycle: deadline（`run.rs:325-332`）・exec job の
  timeout/killGrace/exitCodes/cwd/env は `substitute.grant`
  宣言値。routed-exec context（exec_cwd・extra_env・
  accept_any_exit_code・log_tee）は exec job に従来どおり。
  member job は自身 grant bound（#166 規約不変）。
- exit: captured `details.exitCode`（substitute の exit）を
  mirror — spec R15 の「実行した command の exit」。
- record: shape 不変（`route` 宣言 identity + `commandHash`
  原 command + `executed:{substitute.grant, substitute.argv}`
  実測 — `details` 由来で自動的に substitute ペアを記録）。
  attribution anchor は宣言側 `route.grant`。
- stdin: typed 側 `is_interactive` を維持 + substitute ペア
  （pin executable + 宣言 argv）へ適用。
- Option E: 範囲外 — routed-exec convention（unconditional
  `exec` job + `outputs.execResult`）不変・exec job 不要
  route・`outputs.exitCode` mirror・「何も実行しない」route
  を作らない。open 事項として親 chain へ残置。

## Open Questions

- **Option E（exec-less route）の採否** — 親 chain の open
  事項。author 宣言 exit（`outputs.exitCode` mirror・固定
  mapping）を信用する成功偽装 hole の未解決・lint-only/
  analysis route の product 需要未確認。採否は需要確認と
  exit-code contract の信頼モデル設計が揃った時点の別 issue
  で判断（route `grant` field は firing gate として留保
  される設計だけ確定済み）。本 issue では実装しない。
- `route resolve` 出力への substitute echo — 本 briefing で
  は「載せない（R11 shape 維持）」とするが、substitute
  mode では `grant` output が anchor（`route.grant`）を指し
  「何が実行されるか」が見えない。発火診断需要が確立した
  時点で再検討（spec Deferred Design Concerns に記録）。
- usage record への firing-gate anchor grant 明示 field —
  record は実測 + route identity の規約を維持としたが、
  routes.yaml 編集で `route.grant` が改名された場合に
  時点性の anchor が `route` 名経由の宣言 join でしか
  遡れない。監査需要が確立したら additive field として
  別途追加（O5b — spec Deferred Design Concerns に記録）。
- substitute argv への typed args 転送・補間 — 宣言凍結を
  崩すため不採用としたが、「prefix は substitute 化しつつ
  末尾 args は typed のまま」の需要が確立した時点で
  byte-match 対象を宣言形に再設計する別 issue で検討。
- `grants[]` member の interactive screening — substitute
  ペアは main exec のため screening 対象としたが member
  job は非検査のまま（known semantics — 自身 grant bound
  で動く別経路）。member の interactive 失敗が運用問題
  化した場合に別途検討。
- `trusted` flag 発行主体の再設計（issue 6）— `substitute`
  の層制約は宣言層 provenance で判別するため現行 `trusted`
  flag とは独立だが、発行主体設計が変わると route 層の
  trust 判定に波及する可能性は残る。

## Promotion Candidates

- [x] spec-doc needed: yes → `docs/specs/substitute-exec-route-mode.md`
      （SPEC-034KXdoK2bMG3gQtIj8l6h — `substitute` schema・
      viability retarget・anchor 規約・Option E 境界の intent）
- [x] adr-doc needed: yes → `docs/adr/substitute-exec-route-mode.md`
      （ADR-034KXk8h1AtwCkf2cQrb7m — O1-O8 の決定記録）
- [ ] design-doc needed: no — 依頼は briefing artifacts のみ
      （spec + ADR + resolved discovery）。実装設計は別
      design flow で扱う。

## Sources and Relations

- issue #167: https://github.com/scarecrow173/cheaper/issues/167
  （権限モデル再設計 5/7 — body は
  `docs/tasks/file-issue-substitute-exec-route-mode.md`
  TASK-034KvxFptiaMDpvi8xe9dM に canonical 保存）
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  （DESIGN-034KuDkWOmNXiBddEQ4XWK — deferred sketch 採否表・
  「substitute ペアへの retarget」・anchor 規約・Option E
  open 確定・issue 分解 5/7）・
  `../specs/permission-model-responsibility-redesign.md`
  （SPEC-034KuqPkTK2b4mHsykH9A5）
- 直接の前提（別 chain — references 止まり）:
  `../specs/route-grant-set-projection.md`
  （SPEC-034KTMfXYD5ZlGZCRb8Paj — `grants` schema・viability
  anchor/member 分割・`{command, executed}` record split・
  O7a 層制約判別子）・`../adr/route-grant-set-projection.md`
  （ADR-034KTMnPFDF25x710LFSKY）・
  `../designs/route-grant-set-projection.md`・
  `../discovery/route-grant-set-projection-issue-166.md`
- deferred sketch: `../adr/defer-non-exec-routing-to-permission-model-redesign.md`
  （ADR-034KtTzOJGqmzmBW3SwGp5 — Option D 採用候補・
  「原 argv ではなく宣言された substitute へ向け直す」・
  Option B の argv 変換用途・Option E open）
- 前提 seam: `../specs/unified-exec-grant-model.md`（R9 —
  route 参照は `scope=="trusted" && !fixed` のみ解決する
  単一 seam `is_route_eligible`）
- 親 ADR: `../adr/enforce-declared-contracts-over-content-audit.md`
  （ADR-034KuqZelggTIb8Z8K9QB8 — 宣言契約強制の責務モデル）
- refine 対象 spec（後続で substitute 記述を追記）:
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （SPEC-034J7SxWvGL3oSbeBpo8a3 — R1 route field・R12
  viability・R34 record。R15「実行した command の exit」は
  再記述済み）
- record additive 規約: `../guides/usage.md`

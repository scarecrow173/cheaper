---
id: "SPEC-034KXdoK2bMG3gQtIj8l6h"
type: "spec"
status: "approved"
title: "substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・anchor 切替"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
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
    - "../adr/substitute-exec-route-mode.md"
    - "./route-grant-set-projection.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/substitute-exec-route-mode-issue-167.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../designs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../specs/unified-exec-grant-model.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../guides/usage.md"
    - "../guides/parent-agent-integration.md"
  defers: []
  deferred-by: []
---

# substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・anchor 切替

## What Should Be Built

route が `substitute:{grant,argv}` を宣言できる **substitute-exec
mode** を routes.yaml schema・viability・routed 実行・usage record
へ導入する。typed command は routing の発火契機（match prefix）に
留まり、実際に実行されるのは route 宣言の substitute ペア
（`substitute.grant` が pin する executable + `substitute.argv`
固定 args）である。非 exec routing deferred sketch
（ADR-034KtTzOJGqmzmBW3SwGp5）の採用要素 — Option D
（substituted exec）の実装であり、precheck の strict さを保った
まま検査対象を「実行されない原 command」から「宣言通り実行する
command」へ retarget する。#166（grant-set 投影・record split・
viability 分割）の機構をそのまま土台とする。

構成要素は 5 つ:

1. **`substitute:{grant,argv}` 宣言の schema 追加** — route entry
   の optional object。`grant`（参照 grant 名）と `argv`（実行
   args 列 — executable を含まない command tool input の `args`
   形）の required ペア。宣言は trusted 層（builtin/user）の
   routes.yaml 限定 — project 層は fail-loud 拒否。
2. **viability retarget** — substitute 宣言のある route では
   anchor 完全検査（存在 + `is_route_eligible` + pin +
   `check_exec_execution` argv 照合）の対象を
   `substitute.{grant,argv}` へ向け直す。原 argv の認可検査は
   実行されない command の形骸化検査になるため行わない。
   `route.grant` は member 級検査（存在 + eligibility + pin）へ
   降格し、`grants[]` member の検査は不変。
3. **exec-input anchor 切替** — `expected_input` を
   `{name:substitute.grant, args:substitute.argv}` へ再 anchor
   し、exec-input byte 一致（D11）を substitute argv へ適用する。
   routed workflow は declaration 値の substitute ペアを inputs
   経由で受け取り、exec job がそれを byte 一致で呼ぶ。
4. **lifecycle・exit・record の anchor 規約** — workflow deadline
   / exec job lifecycle bound は `substitute.grant` の宣言値、
   exit mirror は実行した command（= substitute）の exit、usage
   record の route 帰属は `route.grant` anchor を維持しつつ
   `executed` field が実測 substitute ペアを記録する。
5. **宣言層境界** — substitute route は typed command を実行
   しないため、`route.grant` を typed command へ拘束していた
   byte-match 保護は substitute ペアへ移行しない。typed command
   は作者制御の match prefix でしかないため、`substitute` の
   authoring は trusted 層限定とする（`grants` の O7a と同型・
   さらに強い根拠）。

## Why Now

- deferred sketch で確定した安全境界（substitute への retarget・
  「実行した command の exit」mirror・record split）が #166 で
  実装済みの基盤（grant-set 投影・`{command, executed}` record・
  viability の anchor/member 分割）へそのまま乗る。`executed`
  field は substitute mode 対応のために先行分離済み。
- spec R15（`docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`）
  は route-run exit を「実行した command の exit」へ再記述済みで、
  substitute mode はその規約の正当化実装である。
- substitute mode がない限り、route は「typed command をそのまま
  実行する」mode に固定され、`cargo build` → `cargo check`・
  破壊的 command → dry-run といった semantic substitution の
  用途（ADR-034KtTzOJGqmzmBW3SwGp5 が「真に未充足」と識別した
  領域）が実現できない。
- Option E（exec-less route）は引き続き open 事項 — 本 spec は
  substitute mode の受け入れ境界を固定し、Option E と誤認される
  設計（workflow 宣言 exit・exec job 不要）を明示的に排除する。

## Users and Value

- **route 作者（operator/user layer）** — typed command を、それを
  実行せずにより安全・軽量な宣言済み実行へ置き換える route を
  書ける（例: `cargo build` → `cargo check`、変更系 → dry-run）。
  substitute の実行面は grant の argv pattern が bound するため、
  宣言できる置換は grant 認可内に閉じる。
- **parent agent / hook consumer** — substitute mode でも viability
  の strict さ（実 exec の完全事前検査）が保たれ、route 不発時は
  従来どおり原 command が直接実行される。
- **cheaper 保守者** — authorization は引き続き
  `check_exec_execution` の単一 seam。「実行されない command」
  の認可を調べる形骸化検査を viability から排除できる。

## Scope

### In Scope

- routes.yaml route entry への `substitute?: {grant, argv}` 追加
  （schema・validation・`Route` struct・fixture・
  `resources/routes.yaml` の shape 記述）
- `substitute` 宣言の層制約 — trusted 層（builtin/user）限定。
  project 層 routes.yaml の `substitute` key presence を
  fail-loud route-load error で拒否
- viability の retarget — substitute 宣言のある route で anchor
  完全検査を substitute ペアへ向け直し、`route.grant` は
  member 級検査（存在 + eligibility + pin）へ降格
- `route.grant` の substitute mode での役割再定義 —
  required 維持・発火 gate/attribution anchor・投影 member。
  原 argv への argv 照合は行わない
- 投影 set への `substitute.grant` 追加 — effective set =
  `{route.grant} ∪ grants ∪ {substitute.grant} ∪ {fixed}`
- `expected_input` の substitute ペア再 anchor と、routed
  workflow inputs が実行ペア（`grant`/`execArgs`）を供給する
  契約 — builtin routed-exec workflow が両 mode で使い回せる形
- workflow deadline / exec job lifecycle bound の substitute
  grant 由来化
- stdin-unsupported 分類への substitute ペア評価の追加
  （typed 側の既存分類は不変）
- `kind:"route"` record の anchor 規約の固定（`route` 帰属 +
  `commandHash` 原 command + `executed` 実測 substitute ペア）
- 上記に伴う test・docs 更新

### Out of Scope

- **Option E（exec-less route）** — exec job を持たない
  routed workflow・`outputs.exitCode` 等の workflow 宣言値の
  exit mirror・何も実行しない route を routing 対象にする
  一切の形。author 宣言 exit を信用する設計（成功偽装 hole）
  は未解決のままでは実装しない — 本 issue では明示的に範囲外
  （open 事項として記録）
- **workflow 宣言の substitute** — substitute ペアの出所は
  routes.yaml の route 宣言のみ。workflow job が substitute
  argv を組み立てたり grant を選んだりする経路は作らない
  （exec-input byte 一致が構造的に拒否する）
- **typed argv の substitute への転送** — `substitute.argv` は
  宣言時点の固定値。typed args を substitute argv へ埋め込む
  展開・補間は行わない
- **viability enum・`route resolve` 出力 shape の変更** —
  substitute 失敗は `no-grant` へ fold。substitute 情報の
  resolve echo は行わない（診断需要が確立した時点で再検討）
- **record schemaVersion の bump・新規 record field** —
  `{command, executed}` split（#166）のまま。`executed` が
  substitute ペアを記録するのは実測 echo であり新 field は要らない
- **interactive member screening との再評価** — `grants[]`
  member の interactive 非検査（known semantics）は本 spec で
  変えない
- **`trusted` flag 発行主体の再設計** — issue 6 の設計論点

## Requirements

- R1. routes.yaml の route entry に optional な
  `substitute: {grant: <name>, argv: [<arg>, …]}` を追加する
  こと。
  - `substitute` は object で `grant`・`argv` の 2 key を
    required とし、nested object の未知 key は `match`/
    `target`/`output` と同じ fail-loud 規約で拒否する。
    `substitute` が object でない場合も route-load error。
  - `substitute.grant` は `command.grants`（unified
    `policy.exec.grants`）の既存 grant 名への non-empty
    string 参照 — route は grant を生成しない。declared
    executable は grant 宣言側が供給する。
  - `substitute.argv` は string 配列で、command tool input
    の `args` と同じく executable を含まない実行 args 列と
    する（executable は `substitute.grant` の pin が供給）。
    `argv: []` は executable 単独起動を明示宣言する合法値。
    各要素は non-empty string。
  - `substitute.grant` は `route.grant` と同名でもよい —
    同一 grant 内の argv 差替（deferred sketch Option B の
    「同一 grant で args を変える」用途を route 宣言形で
    実現する形）は合法な substitute mode として扱う。
    `substitute.grant` を `grants[]` にも列挙する宣言は
    冗長だが合法（投影 set は dedupe する）。
  - `grant`（単数）は substitute 宣言の有無に関わらず
    required のまま — 発火 gate・record attribution
    anchor・投影 member の anchor field として維持する
    （Option E 側でも `grant` は firing gate として留保
    される確定設計と一致）。
  - `KNOWN_KEYS` への `"substitute"` 追加を含め、未知 key
    拒否の fail-loud schema 規約（`validate_route`）を
    維持すること。
- R2. `substitute` を宣言できるのは trusted 層
  （builtin/user）の routes.yaml のみとし、project 層
  （`<project_dir>/routes.yaml`）の route entry が
  `substitute` key を含む場合は、値の形状に関わらず
  fail-loud route-load error（`InvalidRuntimePolicy` —
  `grants` の O7a と同じ error class・同じ宣言層
  provenance 判別・形状検査より先に presence で拒否）で
  拒否すること。
  - 根拠: byte-matched mode では `route.grant` の実行は
    `expected_input` byte 一致で typed command（user が
    打鍵した argv）へ拘束されるため、project 層でも
    `grant:` anchor の宣言が許されてきた（R7/O7a の
    argv-match 保護）。substitute mode は typed command を
    **一切実行しない** — typed command は作者制御の
    `match.command` prefix に退化し、実行対象は route 宣言
    の `{substitute.grant, substitute.argv}` になる。
    project 層が substitute を宣言できると、untrusted
    authoring が「typed command とは独立に、任意の trusted
    grant を任意の（pattern 適合）宣言 argv で起動する」
    能力を得る — `grants` member 密輸として O7a が塞いだ
    confused-deputy hole を、member（callable なだけ）より
    強い「実実行 pair の選択権」として再発させる。
    `trusted: true` は shadowing gate であり判別に使えない
    （O7a と同じ結論）。
- R3. substitute 宣言のある route の viability は、anchor
  完全検査の対象を `substitute.{grant, argv}` へ retarget
  すること。
  - retarget 後の検査は byte-matched anchor と同じ完全検査:
    `substitute.grant` の存在 + `is_route_eligible`
    （`scope=="trusted" && !fixed` — `route.grant`・
    `grants[]` member・`substitute.grant` 全てに同じ単一
    seam）+ `pin_exec_grant` + `ExecToolOptions` 構築 +
    `check_exec_execution(substitute.grant, substitute.argv)`
    の argv pattern 照合 + cwd/pin verify の事前経路。
  - `substitute.argv` は route 解決時点で確定済みの宣言値
    であるため、argv 照合は #166 member の「job 実行時へ
    遅延」とは質的に異なり、route 解決時の完全検査が
    **意味を持つ** — 「宣言通り実行する command」を
    検査する precheck の strict さを維持する。
  - substitute 側検査の失敗（未存在・project-scope・
    fixed・pin 不能・argv pattern 不一致）は全て
    `Viability::NoGrant` へ fold すること（新規 viability
    値なし・route 不発 → 原 command 直接実行の fail-safe）。
  - 原 argv（typed command）の `route.grant` argv pattern
    照合は substitute mode では行わない — 実行されない
    command の認可を調べる形骸化検査であり、かつ
    「typed command が grant 認可済みのときのみ発火」は
    destructive→dry-run 等の中核用途（認可したくない
    command こそ置き換えたい）を破壊する。
- R4. substitute mode での `route.grant` は member 級検査
  （存在 + `is_route_eligible` + pin 解決可能性 — `grants[]`
  member と同じ検査位置・同じ `NoGrant` fold）とすること。
  - `route.grant` は発火 gate・record attribution anchor・
    投影 member として残り、routed workflow から member
    job として呼び出せる。viability の完全検査対象は
    substitute ペアに一本化する。
- R5. route run 時の policy 投影を
  `{route.grant} ∪ grants ∪ {substitute.grant} ∪
  {fixed grants}` とすること（substitute.grant が投影に
  含まれなければ exec job の `resolve_exec_grant` が失敗
  するため必須）。投影判定位置・「set 内 1 件でも
  ineligible なら `denied("no-grant")`」の fail 規約は #166
  のままとする。routed workflow が到達できる exec 面は
  「宣言 set の argv union + fixed」に閉じたまま。
- R6. `expected_input` を substitute mode では
  `{"name": substitute.grant, "args": substitute.argv}` へ
  再 anchor すること。
  - `job_id == "exec" && tool == "command"` の job への
    byte-match gate（`exec-input-mismatch` /
    `PolicyViolation`）の機構・文言・分類は不変 — 照合
    対象値が substitute ペアへ替わるだけ。
  - routed workflow inputs は**実行ペア**を供給する:
    `grant`・`execArgs` input は substitute mode では
    `substitute.grant`・`substitute.argv`（byte-matched
    mode では従来どおり `route.grant`・原 exec_args）を
    運ぶ。`argv`・`args`・`executable` input は原 command
    の identity（worker 入力・診断）として原値を維持する
    — builtin `routed-exec.workflow.json` が binding を
    変えずに両 mode で動作するための契約。
  - workflow 側が substitute ペアを変更・再構成する経路は
    存在しない（byte-match が宣言値以外を拒否 — substitute
    は route 宣言時点で凍結される）。
- R7. `route run` の process exit code は**実行した
  command**（substitute mode では substitute command）の
  exit code を mirror すること — captured exec result の
  `details.exitCode` を mirror する機構自体は変更しない
  （spec R15 は再記述済み）。`accept_any_exit_code` による
  非 0 透過も不変。
- R8. lifecycle bound は実行 grant（substitute mode では
  `substitute.grant`）の宣言値に従うこと:
  - workflow 全体の deadline は `substitute.grant` の
    `timeout` から導出する（現行 `route.grant` timeout
    由来の規約を実行 grant へ追従）。
  - exec job の `timeout`/`killGrace`/`exitCodes`/`cwd`/
    `env` は `command::execute` が named grant（=
    `substitute.grant`）の宣言値を読む既存規約 — routed-
    exec context（`exec_cwd` = payload cwd・passthrough
    `extra_env`・`accept_any_exit_code`・route log
    `log_tee`）は exec job に従来どおり適用される。
  - `grants[]` member job は従来どおり自身 grant の宣言
    bound で動き、route context は付かない（#166 規約
    不変）。
- R9. `kind:"route"` usage record の anchor 規約を維持する
  こと:
  - `route` field（route 宣言 identity）と `commandHash`
    （原 command の一方向 hash — env-prefix 値を含み得る
    ため hash のまま）は不変。**route 帰属は `route.grant`
    anchor** — route の発火 gate 認可 grant は
    `substitute.grant` ではなく `route.grant` のままと
    する（grant set・substitute は実行範囲の宣言であり
    attribution anchor を構成しない）。
  - `executed:{grant, argv}` は実測値 — captured exec
    result の `details.{grant,argv}` 由来であり、
    substitute mode では自動的に
    `{substitute.grant, substitute.argv}` を記録する。
    record shape・`schemaVersion: 1`・additive 規約・
    record timing（captured result 必須）は #166 のまま、
    新規 field は追加しない。
  - `exitCode`・`ok` は mirror した substitute exit 由来。
- R10. stdin-unsupported 分類は、typed 側の既存評価
  （`is_interactive(原 executable, 原 exec_args)`）を
  **維持**しつつ、substitute 宣言のある route では
  substitute ペアにも同じ interactive 評価を適用すること。
  substitute 側の executable 名判定は**宣言 basename
  （`substitute.grant` の `executable` 宣言値）と pin 解決済み
  basename の union** で行う — pin 解決名のみでは
  `python3` → `python3.11` のような canonical 名 suffix で
  回避されるため宣言名でも検査し、宣言名のみでは shim が
  実体を偽装できるため解決名でも検査する。
  - typed 側を維持するのは「interactive な typed command
    は routing で置き換えない」不変条件の保存 — 原
    command が stdin 前提なら substitute 宣言の有無に
    関わらず routing しない。
  - substitute 側を追加するのは、stdin 非接続で実行される
    のは substitute の方であり、interactive な substitute
    は発火後に grant timeout まで hang してから失敗する
    ため、resolve 時点で `stdin-unsupported` に倒す。
  - `grants[]` member の非検査（known semantics）は変え
    ない — member job は自身 grant bound で動く別経路。
- R11. **Option E（exec-less route）は本 spec の範囲外**と
  し、routing 対象となる routed workflow は引き続き
  routed-exec convention（unconditional `exec` job +
  `outputs.execResult` whole binding —
  `validate_routed_convention`）を満たすこと。exec job
  不要の route・
  `outputs.exitCode` 等の workflow 宣言値 exit mirror・
  「何も実行しない」route は作らない — author 宣言 exit を
  信用する成功偽装 hole が未解決のため、Option E は親
  chain の open 事項として残置する（route `grant` field が
  firing gate として留保される設計だけは確定済み）。
- R12. 次の不変条件を保持すること:
  - **routing ≠ authorization**（D2）— viability は
    `check_exec_execution`・`pin_exec_grant`・
    `is_route_eligible` への委譲のみを行い、route 層に
    内容審査を持たない。substitute argv は grant の argv
    pattern が bound するため、route 作者は grant 認可外の
    実行を宣言できない。
  - **fail-safe fold** — substitute 側・`route.grant`・
    member のいずれの失敗も `Viability::NoGrant` で
    route 不発 → 原 command 直接実行。
  - **hook fail-open** — `load_or_none` 経路の規約は不変。
    `substitute` の schema error は `grants` と同じく load
    error として fail-open に fold される。
  - **semantic substitution は trusted 層 opt-in で受容** —
    親 agent は substitute の exit/output を原 command の
    ものとして解釈する。これは routing 層が新たに開く
    意味差であり、trusted authoring 限定（R2）の主要根拠。
  - **宣言凍結** — substitute ペアは route 宣言値で凍結
    され、typed argv・payload・workflow が変更できない
    （byte-match gate）。
  - `execDenyPatterns` の適用点は `substitute.grant`・
    `route.grant`（member 級）にも一律に効く — deny される
    grant は ineligible → `NoGrant`。

## Acceptance Criteria

- [ ] `substitute: {grant, argv}` を宣言した trusted 層 route
      が schema 受理され、`substitute` が object でない・
      `grant`/`argv` 欠落・nested 未知 key・非 string
      要素の各場合に fail-loud route-load error となる
      （`argv: []` は受理される）
- [ ] project 層 routes.yaml の route entry が `substitute`
      key を含むと fail-loud route-load error
      （`InvalidRuntimePolicy` — file・entry 名指しの
      details）で拒否され、builtin/user 層の `substitute`
      宣言は受理される。project 層の `grant:` 単数宣言・
      byte-matched route は従来どおり有効
- [ ] substitute 宣言のある route で、`substitute.argv` が
      `substitute.grant` の argv pattern に一致しない・
      `substitute.grant` が未存在・project-scope・fixed・
      pin 不能の各場合に viability が `no-grant` となり
      route が発火しない（新規 viability 値なし）
- [ ] substitute 宣言のある route で、原 argv が
      `route.grant` の argv pattern に一致**しない**場合も
      （substitute ペアが健全な限り）viability が
      `runnable` となり発火する — 原 argv 照合を行わない
      retarget の回帰固定。`route.grant` が未存在・
      ineligible・pin 不能の場合は `no-grant`
- [ ] substitute route の routed workflow で、exec job が
      `{name: substitute.grant, args: substitute.argv}`
      を実行し、expected_input が同じ substitute ペアを
      要求して `exec-input-mismatch` gate を通る
      （builtin routed-exec workflow が無変更で動作）。
      substitute ペアと異なる name/args の `job_id=="exec"`
      job は `exec-input-mismatch` で失敗する
- [ ] substitute command の exit code（0・非 0）が
      `route run` の process exit code としてそのまま
      mirror される
- [ ] workflow deadline・exec job lifecycle が
      `substitute.grant` の宣言 `timeout`/`killGrace`/
      `exitCodes`/`cwd`/`env` に従う（`route.grant` の
      宣言値ではない）
- [ ] `kind:"route"` record が `route`（route 名 —
      `route.grant` anchor の宣言 identity）・
      `commandHash`（原 command）・`executed`:
      `{grant: substitute.grant, argv: substitute.argv}`
      を持ち、`schemaVersion: 1`・新規 field 無しで
      `cheaper usage` 集計が従来どおり動作する
- [ ] typed 側 interactive（原 executable が interactive
      list）の typed command は substitute 宣言があっても
      `stdin-unsupported` で不発し、interactive な
      executable を持つ `substitute.grant` または
      interactive flag を含む `substitute.argv` の route は
      `stdin-unsupported` で不発する
- [ ] substitute route が `grants[]` member と併用でき、
      `input.name` が member 名の job は従来どおり動作し、
      投影外の grant 名は拒否される
- [ ] exec job を持たない・`outputs.execResult` を持たない
      workflow は substitute 宣言の有無に関わらず routed
      convention 違反で拒否される（Option E 不実装の
      構造的確認）
- [ ] `substitute` を持たない既存 route（byte-matched・
      grants 宣言）の viability・exec-input・exit mirror・
      record 挙動に回帰がない（既存 route/hook/run test
      群 pass）

## Deferred Design Concerns

- `route resolve` 出力への substitute echo | R11 出力 shape
  を維持したため substitute mode の診断が `route`/`grant`
  field だけでは読めない | substitute mode の発火診断需要が
  確立した時点で検討 | `grant` output は anchor（route.grant）
  のままで substitute 実行対象が見えない
- usage record への anchor grant 明示 field | `route` 名経由の
  declaration join に委ねる現行構造では、record 書込み後に
  routes.yaml の `grant` が編集されると firing-gate 認可
  grant が record から遡れない | route 監査で時点性の anchor
  が必要になった時点で additive field として検討 | record は
  実測 + route identity の規約を現行維持とした
- Option E（exec-less route）の採否 | 親 chain の open 事項 —
  author 宣言 exit を信用する成功偽装 hole が未解決で
  product 需要も未確認 | lint-only/analysis route の需要が
  確認され、exit-code contract の設計が固まった時点で
  再検討 | `grant` field は firing gate として留保される
  設計だけ確定（R11）
- substitute argv への typed args 転送・補間 | 宣言凍結を
  崩すため現行は不採用 | 「prefix は substitute 化しつつ
  末尾 args は typed のまま」の需要が確立した時点で、
  byte-match 対象を宣言形に再設計する別 issue で検討
- interactive member screening との整合 | substitute ペアは
  main exec のため screening 対象としたが、`grants[]`
  member は非検査のまま | member の interactive 失敗が運用
  問題化した場合に member 側 screening を別途検討

## Sources and Relations

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/167
  （権限モデル再設計 5/7）
- 上流 discovery:
  `../discovery/substitute-exec-route-mode-issue-167.md`
  （DISC-034KWGjJMiSKU5wBxcznho — 実測接点・選択肢比較）
- 親 chain: `../specs/permission-model-responsibility-redesign.md`
  （SPEC-034KuqPkTK2b4mHsykH9A5 — R6/R8）・
  `../designs/permission-model-responsibility-redesign.md`
  （DESIGN-034KuDkWOmNXiBddEQ4XWK — deferred sketch 採否
  表・grant anchor 規約・issue 分解 5/7）
- 直接の前提: `../specs/route-grant-set-projection.md`
  （SPEC-034KTMfXYD5ZlGZCRb8Paj — viability 分割・set 投影・
  `{command, executed}` record split）・
  `../adr/route-grant-set-projection.md`
  （ADR-034KTMnPFDF25x710LFSKY）・
  `../designs/route-grant-set-projection.md`
  （DESIGN-034KUgF5QB3WuzFG6PmVvr — member 検査・投影・
  anchor 規約の実装形）
- deferred sketch:
  `../adr/defer-non-exec-routing-to-permission-model-redesign.md`
  （ADR-034KtTzOJGqmzmBW3SwGp5 — Option D 採用・Option E
  継続 open）
- 前提 seam: `../specs/unified-exec-grant-model.md`（R9 —
  `substitute.grant` も `scope=="trusted" && !fixed` のみ
  解決する route 境界・`is_route_eligible` 単一 seam）
- 親 ADR: `../adr/enforce-declared-contracts-over-content-audit.md`
  （ADR-034KuqZelggTIb8Z8K9QB8 — 宣言契約強制の責務モデル）
- refine 対象 spec（本 chain からの refines ではなく後続
  更新先）: `./pretooluse-route-resolver-and-workflow-delegation.md`
  （SPEC-034J7SxWvGL3oSbeBpo8a3 — R1 route field・R12
  viability・R34 record への substitute 記述追記先。R15
  exit mirror の「実行した command」再記述は済）

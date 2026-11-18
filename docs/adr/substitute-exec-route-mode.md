---
id: "ADR-034KXk8h1AtwCkf2cQrb7m"
type: "adr"
status: "accepted"
title: "substitute-exec route mode — substitute:{grant,argv}・viability retarget・実行 anchor 切替"
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
  depends-on:
    - "./route-grant-set-projection.md"
    - "./defer-non-exec-routing-to-permission-model-redesign.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/substitute-exec-route-mode.md"
    - "../specs/route-grant-set-projection.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "./enforce-declared-contracts-over-content-audit.md"
    - "./unified-exec-grant-model.md"
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
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../specs/unified-exec-grant-model.md"
    - "../guides/usage.md"
    - "../guides/parent-agent-integration.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# substitute-exec route mode — substitute:{grant,argv}・viability retarget・実行 anchor 切替

## Context and Problem Statement

route は現行 byte-matched mode のみを持つ: `match.command`
prefix が typed argv に一致し、`route.grant` が原 argv（typed
command）を `check_exec_execution` で認可できるときのみ発火
し、routed workflow の `exec` job が**原 command そのもの**を
`{name: route.grant, args: 原 exec_args}` で実行する
（`resolve.rs:312-348`・`route/run.rs:176`・
`workflow_run.rs:467-484` の expected_input byte 一致 — D11）。

しかし routing の用途には「typed command とは別の、宣言済みの
安全な command を実行したい」semantic substitution がある
（`cargo build` → `cargo check`、変更系 → dry-run、軽量
linter）。ADR-034KtTzOJGqmzmBW3SwGp5 は non-exec routing の
spectrum を評価し、**Option D（substituted exec — exec job 必須
だが route 宣言の substitute `{grant,argv}` を実行）を採用
候補として権限モデル再設計（#151）へ defer** した。親 design
（DESIGN-034KuDkWOmNXiBddEQ4XWK）がその採否を確定
（substitute exec・「実行した command の exit」mirror・record
split 採用、Option E exec-less は open）し、issue 5/7 として
本 issue へ分解済み。

#166（grant-set 投影）で基盤は揃った: route が宣言する grant
set の投影・viability の anchor/member 分割・`{command,
executed}` record split は、substitute mode が「原 command」と
「実 exec」の分岐を記録するための前提整備として先に入って
いる。`is_route_eligible`（`scope=="trusted" && !fixed`）は
`route.grant`・`grants[]`・`substitute.grant` の 3 参照を同じ
単一 seam で判定する設計が確立済み（#165 R9・`policy/exec.rs`
の comment で substitute 参照を明示）。

残る論点は substitute mode 固有の設計: 宣言 schema の形・
viability の検査対象と `route.grant` の substitute mode での
役割・`substitute` を宣言できる層・`expected_input`/lifecycle/
exit/record の anchor 切替規約・Option E との境界。

## Decision Drivers

- **「実行する command」を検査する** — deferred sketch の
  確定スケッチは viability の `check_command_execution` を
  「原 argv ではなく宣言された substitute」へ向け直す形を採用
  候補とし、親 design が採用を確定した。byte-matched mode で
  原 argv 照合が意味を持つのは「原 argv = 実行 argv」（D11）
  だからであり、substitute mode で原 argv を照合しても実行
  されない command の認可を調べる**形骸化**になる。
- **byte-match 保護は typed command へ乗り移らない** —
  byte-matched mode の `route.grant` は `expected_input`
  byte 一致で「実行される exec = typed command」へ拘束され、
  project 層が `grant:` を宣言しても実行対象は user が打鍵
  した argv のまま。substitute mode はこの等式を壊す:
  typed command は作者制御の `match.command` prefix に退化
  し、実行される `{grant, argv}` は route 宣言値になる。
  「typed command への拘束」に依存した project 層
  authorability の根拠は substitute に届かない。
- **routing ≠ authorization（D2）・confused-deputy 半径の宣言
  境界** — route 層は authorization を委譲呼出しするだけで
  grant を生成しない。`substitute.argv` は `substitute.grant`
  の argv pattern が bound し、投影外の grant は routed
  workflow から解決不能 — confused-deputy 半径は「宣言
  set の argv union + fixed」に閉じたまま。
- **anchor は 2 系統を分離する** — 親 design の anchor 規約:
  「実行される grant」（expected_input・lifecycle bound・
  exit mirror）は実行 grant `substitute.grant` へ anchor
  し、「route 帰属（route × firing gate の認可 grant）」は
  `route.grant` に anchor する。両者を混ぜない。
- **fail-safe fold** — substitute 側・`route.grant`・member
  のいずれの欠陥も route 不発 → 原 command 直接実行。
  「route 発火 → exec 失敗 → 原 command 不実行」の後退を
  許さない。
- **宣言凍結** — substitute ペアは route 宣言値で固定。
  typed argv 補間・payload・workflow 側の組み立てを許すと
  byte-match gate の意味が崩れ、project workflow が実行
  argv を authoring できる経路が復活する。
- **Option E は open のまま** — author 宣言 exit を mirror
  する設計は成功偽装を routing 層 opt-in で受容する前提
  判断が要り、product 需要も未確認。substitute（Option D）
  の実装に Option E を紛れ込ませない。

## Considered Options

### O1 `substitute` 宣言の schema 形状

- **O1a `substitute: {grant, argv}` nested object** — `match:
  {command}`・`target: {kind,path}`・`output: {tokenThreshold}`
  と同じ compound-value-as-object 規約。`grant`・`argv` の
  required ペアとして atomic に検証でき、nested 未知 key は
  既存の fail-loud 規約で拒否。`argv` は command tool input
  の `args` と同じ executable を含まない args 列（executable
  は `substitute.grant` の pin が供給）。
- **O1b flat field 対（`substituteGrant:`・`substituteArgv:`）** —
  route の top-level key は flat 規約でもある（`grant`・
  `grants`・`passthroughEnv`）ため flat は一見整合するが、
  2 field が独立に存在し得る partial 宣言（`substituteGrant`
  のみ）を schema で必ず相互 required にする規約を新設する
  必要があり、ペアの atomic 性が nested object より弱い。
- **O1c `substitute: "<command string>"`（free-form）** —
  `substitute: "cargo check --all-targets"` のように文字列
  宣言する形。tokenize 規約・quote・shell metachar の扱いを
  routes.yaml schema 側へ持ち込み、routing 層が「shell を
  経由しない argv 列」という実行契約と表現 drift する。
- **O1d mode-keyed schema（`mode: substitute` + `exec:
  {grant,argv}` 等）** — mode 列挙で byte-matched/substitute
  を排他切替する形。現状の mode は 2 種のみで YAGNI、
  `substitute` presence がそのまま mode switch として機能
  し、`mode` field は既存 routes.yaml 全件への新規 required
  key か optional 複雑化のどちらかを強いる。

### O2 viability の検査対象と `route.grant` の substitute mode での役割

- **O2a anchor 完全検査を substitute ペアへ retarget、
  `route.grant` は member 級検査へ降格** — substitute 宣言の
  ある route では `substitute.grant` × `substitute.argv` に
  anchor と同じ完全検査（存在 + `is_route_eligible` + pin +
  `ExecToolOptions` + `check_exec_execution` の argv 照合）を
  向け、`route.grant` は `grants[]` member と同じ検査
  （存在 + eligibility + pin 解決）を受ける。原 argv の
  `route.grant` 照合は行わない — substitute mode で原 argv
  は「実行されない command」であり、その認可は形骸化検査。
  親 design の条件付き弱化列挙（(a) substitute retarget・
  (b) set 内非 exec grant は member 検査）と一致し、
  substitute mode の `route.grant` はまさに「set 内の非
  exec grant」になる。
- **O2b dual check — 原 argv の `route.grant` 照合を維持し
  つつ substitute ペアも完全検査** — 「typed command が
  grant 認可済みのときのみ発火」（authorization-of-intent）
  を残す案。発火条件は最も保守的だが、(i) 実行されない
  command の認可を viability で調べる形骸化を復活させ、
  (ii) 「認可したくない command こそ置き換えたい」用途
  （変更系 → dry-run）を根幹で破壊する — substitute を
  掛けるには typed command を認可する trusted grant の宣言
  が必要になり、その grant は `workflow run --grant` で
  直接呼べる実行面になる。authorization-of-intent を
  firing gate として残す設計は Option E（実行物が無い
  mode で `grant` が残る唯一の意味）向けに確定した言語
  であり、Option D には適用しない。
- **O2c `route.grant` は何も検査しない・投影からも外す** —
  substitute mode で `route.grant` を純 metadata 化する案。
  しかし `route.grant` を投影外にすると member job が
  `route.grant` を呼べなくなる非対称が生まれ、
  `project_grants` の「set 内全件 eligible 必須」検査からも
  外れるため「投影名は全て viability 検証済み」の不変が
  壊れる。record attribution が viability 未検証の grant 名を
  指す phantom になる。

### O3 `substitute` 宣言の layer 制約（authoring trust boundary）

- **O3a `substitute` は trusted 層（builtin/user）限定 —
  project 層の key presence を fail-loud route-load error で
  拒否** — O7a（#166 `grants`）と同じ判別子: 宣言層
  provenance（`layer == RouteLayer::Project`）を `validate_route`
  で `schema_error`（`InvalidRuntimePolicy`）へ倒す。
  `route.grant` 単数・`match`/`target`/`output`/`passthroughEnv`/
  `trusted` は従来どおり project 層で宣言可。`trusted: true`
  は project 自己申告の shadowing gate であり判別に使えない
  （O7a と同じ結論）。
- **O3b project 層でも受理（schema uniform + 文書化）** —
  「substitute argv は grant の argv pattern が bound する
  ため実害は限定的」と見なす案。しかし substitute mode は
  byte-match 保護が typed command へ乗り移らないため、
  project 層が「任意の typed prefix に対して、任意の trusted
  grant を、（pattern 適合の）任意の宣言 argv で起動する
  route」を書ける — `grants` member が「投影へ入るだけ」
  だったのに対し、substitute は「実際に実行される
  `{grant, argv}` の選択権」であり、O4c/O7a が塞いだ
  confused-deputy hole を**さらに強い形**で再発させる。
  grant pattern bound は「どの grant を・どの宣言 argv で
  起動するか」の選択権を制約しない（O7b と同じ推論）。
- **O3c project 層でも「原 argv が `route.grant` を満たす
  route のみ」substitute 可（条件付き受理）** — O2b の
  authorization-of-intent 維持を project 層の前提条件に
  する案。発火条件は狭まるが、substitute ペアの選択権は
  依然 project 作者のまま — 「typed command が認可済み
  であること」は「何を実行するか」への制約にならず、
  hole の本体（実行 `{grant,argv}` の authoring 権）は
  閉じない。schema/semantic の複雑さだけが増える。

### O4 `expected_input`・routed workflow inputs の anchor

- **O4a `expected_input = {name: substitute.grant, args:
  substitute.argv}`、inputs の `grant`/`execArgs` は実行
  ペアを運ぶ** — `RouteExecution::new` に渡す grant/argv を
  substitute ペアへ切替え、routed workflow inputs も
  `grant = substitute.grant`・`execArgs = substitute.argv`
  を供給する。builtin `routed-exec.workflow.json` は exec
  job を `{name: $input.grant, args: $input.execArgs}` に
  bind するため、両 mode で binding 無変更のまま
  `job_id == "exec"` の input が substitute ペアと byte
  一致する。`argv`・`args`・`executable` input は原
  command identity（analyze prompt・診断）として原値を
  維持。
- **O4b 専用 inputs（`substituteGrant`/`substituteArgv`）
  を追加し `grant` input は `route.grant` のまま** —
  宣言 anchor と実行 anchor を別 input で表す案。
  `grant` input の意味が mode で分岐する事態は避けられるが、
  builtin workflow が mode 別 binding（または `when` 切替）
  を要し、「exec job input = expected_input」という単一
  規約を 2 系統の input 語彙へ分ける表現 drift を生む。
- **O4c workflow 宣言の substitute — routed workflow 側が
  substitute argv を組み立てる** — deferred sketch が明示
  拒否した workflow-declared grants と同型。expected_input
  byte 一致が宣言値以外を拒否するため構造的に成立しない
  が、「workflow が exec input を提案して route が受理する」
  経路を設計として許すと byte-match の意味が反転する。
  不採用。

### O5 usage record の anchor・shape

- **O5a record shape 不変 — `route`（宣言 identity =
  `route.grant` anchor）+ `commandHash`（原 command）+
  `executed:{grant,argv}`（実測 substitute ペア）** —
  `executed` は captured exec result の `details.{grant,argv}`
  由来の実測値であり、substitute mode では自動的に
  `{substitute.grant, substitute.argv}` を記録する
  （`command::execute` が named grant・実 argv を記録する
  既存規約）。`route` field は宣言 identity で、その route
  の firing gate 認可 grant は `route.grant` — attribution
  anchor は宣言側に残る。schemaVersion 1・additive 規約・
  record timing（captured result 必須）不変。
- **O5b record へ firing-gate anchor grant の明示 field を
  追加（例 `grant`/`anchorGrant`）** — substitute mode で
  `executed.grant ≠ route.grant` となるため、record だけ
  では firing-gate 認可 grant が `route` 名 → routes.yaml
  宣言 join 経由でしか遡れない。書込み時点の anchor を
  record へ凍結する案（`suppressed` provenance と同型の
  宣言状態 capture）。観測性は上がるが、#166 の record
  contract は「実測 + route identity」を規約としており
  宣言 echo は含めない設計。routes.yaml 編集後の時点性
  join 欠落は hypothetical であり YAGNI — 監査需要が確立
  したら additive field として別途追加できる。
- **O5c record anchor を `substitute.grant` へ再 anchor** —
  実行した grant を attribution 主体にする案。「実行され
  なかった command」を帰属にしない目的（record split の
  動機）は `executed` field が担っており、`route` 帰属は
  「どの route が発火したか」であり実行 grant ではない。
  route 宣言を外れて実測 grant へ帰属を移すと
  `cheaper usage` の per-route 集計が宣言軸と乖離する。

### O6 stdin-unsupported 分類の対象

- **O6a typed 側を維持 + substitute ペアにも適用** —
  typed 側の `is_interactive(原 executable, 原 exec_args)`
  分類は「interactive な typed command は routing で
  置き換えない」invariant として維持（byte-matched と同じ
  発火条件を substitute mode でも保つ）。加えて
  substitute ペア（`substitute.grant` の宣言 basename・
  pin 解決 basename の union + `substitute.argv`）に同じ
  分類を適用する — stdin 非接続で spawn されるのは
  substitute であり、interactive な substitute は発火後に
  grant timeout まで hang してから失敗するため、resolve
  時点で `stdin-unsupported` に倒す。pin 解決名のみでは
  `python3` → `python3.11` のような canonical 名 suffix で
  回避されるため、宣言名との union で両側を検査する。
- **O6b substitute ペアのみへ retarget（typed 側を外す）** —
  「実行されるものだけ検査する」原則を interactive 分類へも
  適用する案。しかし typed 側を外すと「`vim` を打った
  user が `cat` の結果を受け取る」routing が可能になり、
  interactive typed command を intercept する substitute
  route を作者が意図せず・あるいは意図して・書けるように
  なる — byte-matched で成立していた「interactive 入力は
  routing しない」invariant を緩める方向は採らない。
- **O6c typed 側のみ維持（substitute 側は未検査）** —
  substitute argv は trusted 作者の宣言値であり interactive
  宣言は作者責任・grant timeout が bound すると見なす案。
  失敗は bounded だが「発火 → hang → timeout 失敗」は
  「原 command 直接実行」より悪い UX であり、resolve 時点で
  分類可能な情報（pin executable + 宣言 argv）は揃って
  いる — 検査しない理由がない。

### O7 投影 set への `substitute.grant` の扱い

- **O7a 有効 set = `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}`** — `substitute.grant` を
  投影へ加える。`substitute.grant` が投影に無ければ exec
  job の `resolve_exec_grant` が policy 解決失敗で落ちる
  ため必須。`route.grant` は投影 member として残り
  （member job から呼べる・attribution anchor）。投影判定
  は既存 `project_grants` の位置・全件 `is_route_eligible`
  必須の規約を維持 — viability で両名とも検証済みの set を
  渡すため「投影名は viability 検証済み」の不変が成立する。
- **O7b substitute mode では `route.grant` を投影から外す** —
  実行しない grant を投影しない最小化案。confused-deputy
  半径は僅かに狭まるが、`route.grant` を member job が
  呼べない非対称・`project_grants` の set 全件検査から
  anchor が外れる規約複雑化・record attribution が投影外の
  名を指す矛盾を伴い、得る安全性は substitute argv が
  既に宣言固定であることを考えると薄い。

### O8 Option E（exec-less route）の扱い

- **O8a 本 issue では実装せず open 事項として残置** —
  routed-exec convention（unconditional `exec` job +
  `outputs.execResult` whole binding — `validate_routed_
  convention`）を不変とし、exec job 不要の route・
  `outputs.exitCode` の workflow 宣言値 mirror・「何も
  実行しない」route を作らない。author 宣言 exit を信用
  する設計は成功偽装を routing 層 opt-in で受容する前提
  判断が要り、親 design が「product 需要未確認」で open
  と確定済み。`grant` field は Option E でも firing gate
  として留保される確定設計だけ意識して required を維持。
- **O8b Option E まで本 issue で実装** — substitute mode と
  「exec を持たない route」を同一 schema 拡張で実装する案。
  exit contract（measured → author-declared）の信頼モデル
  変更は substitute よりはるかに大きく、未決判断を前提に
  実装すると成功偽装 hole を開ける。
- **O8c Option E を恒久的に不採用と確定** — open ではなく
  reject に倒す案。lint-only/analysis route の需要が将来
  確認された場合の revisit 経路を残す方が誠実で、採否の
  判断材料がまだ揃っていない。

## Decision Outcome

採用: O1a + O2a + O3a + O4a + O5a + O6a + O7a + O8a。

- routes.yaml route entry に optional な `substitute:
  {grant, argv}` を追加（`match`/`target`/`output` と同型の
  nested object・required ペア・nested 未知 key 拒否・
  `argv` は executable を含まない args 列で `[]` 可）。
- `substitute` の宣言は trusted 層（builtin/user）限定 —
  project 層 routes.yaml の `substitute` key presence を
  `validate_route` で `schema_error`（`InvalidRuntimePolicy`、
  `grants` と同じ error class・形状検査より先に presence
  で拒否）へ倒す。`grant:` 単数は従来どおり project 層で
  宣言可。
- viability: substitute 宣言のある route では anchor 完全
  検査の対象を `substitute.{grant, argv}` へ retarget し、
  `route.grant` は member 級検査（存在 + `is_route_eligible`
  + pin — `grants[]` と同じ）へ降格。原 argv の argv 照合は
  行わない。全失敗は `Viability::NoGrant` fold（新規
  viability 値・出力 shape 変更なし）。
- 投影 set = `{route.grant} ∪ grants ∪ {substitute.grant} ∪
  {fixed}`。
- `expected_input = {name: substitute.grant, args:
  substitute.argv}` — D11 byte-match の機構・文言・
  `job_id=="exec" && tool=="command"` gate は不変。routed
  workflow inputs の `grant`/`execArgs` は実行ペアを運び、
  `argv`/`args`/`executable` は原 command identity として
  原値を維持（builtin routed-exec workflow は両 mode で
  無変更で動作）。
- lifecycle bound（workflow deadline・exec job の
  timeout/killGrace/exitCodes/cwd/env）は `substitute.grant`
  の宣言値。exit mirror は captured `details.exitCode` =
  substitute の exit をそのまま（spec R15 の「実行した
  command の exit」）。
- record は `route` + `commandHash` + `executed` —
  attribution anchor は `route.grant`（宣言側）、`executed`
  が実測 substitute ペア。shape・schemaVersion 不変・
  新規 field なし。
- stdin-unsupported 分類は typed 側を維持しつつ
  substitute ペア（pin 解決 executable + 宣言 argv）にも
  適用。
- Option E（exec-less route・workflow 宣言 exit mirror）は
  本決定の範囲外で open 事項として残置 —
  `validate_routed_convention` は変更しない。

### Consequences

* Good, because viability が「宣言通り実行する command」だけを
  検査する構造になり、実行されない command の認可を調べる
  形骸化検査が消える。precheck の strict さ（完全検査）は
  byte-matched と同じレベルで維持される。
* Good, because `substitute.argv` は宣言時点で確定するため
  argv 照合が route 解決時に意味を持ち、#166 member の
  「argv 未確定ゆえ job 実行時遅延」とは質的に異なる完全
  事前検査になる — substitute 側欠陥で発火後に失敗する
  経路が resolve 時点で塞がれる。
* Good, because `substitute` を trusted 層限定にすることで、
  「typed command とは独立に trusted grant を宣言 argv で
  起動する」選択権が trusted authoring に閉じ、project 層
  routes.yaml が exec 委譲面を authoring する経路は `grants`
  と同じ構造的に閉じた形で塞がれる。
* Good, because builtin `routed-exec.workflow.json` が両 mode
  で無変更動作し、「exec job input = expected_input」の
  単一規約と D11 byte-match が mode に関わらず同じ強制点
  として機能する。
* Good, because record split の前提整備がそのまま効き、
  「route が発火した（route.grant 認可 anchor）」と「実際に
  実行した `{grant, argv}`」が record 上で分岐を持つ —
  substitute 実装に record schema 変更が要らない。
* Good, because `route.grant` が firing gate/attribution
  anchor・投影 member として残り、Option E 採用時にも
  「firing gate の認可 grant」として意味を持つ形（親 design
  確定）と整合する。
* Bad, because substitute mode で原 argv は `route.grant` の
  argv pattern に照合されなくなり、「typed command が認可
  済みのときのみ発火」の authorization-of-intent は
  substitute mode では消える — typed argv は `match.command`
  prefix までしか発火を拘束しない（作者制御）。この緩和は
  semantic substitution の中核用途（認可したくない command
  を置き換える）を可能にする意図的な trade-off であり、
  substitute 宣言を trusted 層限定にした理由そのもの。
* Bad, because 親 agent は substitute の exit/output を原
  command のものとして解釈する — semantic substitution は
  routing 層が新たに開く意味差であり、user が打鍵した
  command と実際に走った command が異なることを trusted
  authoring opt-in で受容する（spec R15 再記述済みの帰結）。
* Bad, because `route resolve` 出力は substitute 宣言を
  echo せず、`grant` field は anchor（`route.grant`）を
  指す — substitute mode の発火診断で「何が実行されるか」
  が resolve 出力だけでは見えない（`--json` diagnostics 改善
  は後続検討）。
* Bad, because interactive な executable を pin する
  `substitute.grant`・interactive flag を含む
  `substitute.argv` の route は `stdin-unsupported` で不発
  し、interactive な typed command は substitute 宣言があって
  も routing されない — typed・substitute 双方の分類で
  不発に倒れる面が増える（意図的な保守方向）。
* Bad, because routes.yaml schema・`Route` struct・viability
  分岐（anchor/member 分岐を substitute で再編）・
  `RouteExecution::new`/`grant_timeout`/inputs の実行ペア
  切替・fixture・routes.yaml shape 記述・test を一括で
  触る変更面が広い（破壊的 schema 変更ではないが）。
* Bad, because `kind:"route"` record だけでは firing-gate
  認可 grant が `route` 名経由の宣言 join に依存する —
  routes.yaml 編集で `route.grant` が改名されると時点性の
  anchor が record から遡れない（現行 record 規約と同じ
  制約 — 監査需要が確立したら additive field で拡張）。

### Confirmation

- spec の受入条件（`docs/specs/substitute-exec-route-mode.md`
  の Acceptance Criteria）が実装 AC。
- issue #167 の Acceptance Criteria（substitute schema・
  viability retarget・expected_input 再 anchor・exit
  mirror・record・Option E 範囲外明示）が spec の R1-R11
  と AC で全て覆われていること。
- discovery（`docs/discovery/substitute-exec-route-mode-issue-167.md`
  の file:line inventory）が schema・viability・projection・
  run 経路・record の全接点を網羅していること。

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/route/{routes,resolve,
  run}.rs`（`Route` struct・`KNOWN_KEYS`/`validate_route` への
  `substitute` 追加と project 層拒否・viability の substitute
  retarget と `route.grant` member 化・`RouteExecution::new`/
  `grant_timeout`/inputs の実行ペア切替）、
  `crates/cheaper-cli/resources/routes.yaml`（shape 記述）、
  route/hook/run 関連 integration test・fixture。projection 側
  (`policy/exec.rs`)・workflow_run D11 gate・`usage_log.rs`
  record builder は仕組み変更なし（`grant_set()` の set に
  substitute.grant を含める形のみ）。
* Patterns to follow: `is_route_eligible` の単一 seam
  （`route.grant`・`grants[]`・`substitute.grant` 同一判定）、
  member 検査の `resolve.rs:349-377` 構造・NoGrant fold、
  `validate_route` の `grants` 層制約（presence 優先・
  `schema_error`/`InvalidRuntimePolicy` — `routes.rs:421-431`
  と同型の `{label}.substitute` error）、`RouteExecution::
  new` の `{name,args}` expected_input 生成・
  `project_grants` の投影位置（viability 通過後）。
* Patterns to avoid: viability enum 値の追加・workflow 宣言の
  substitute/exec・typed argv の substitute argv への補間・
  project 層 `substitute` 受理・`route.grant` の optional 化・
  record schemaVersion bump・Option E の実装（exec job 不要
  route・workflow 宣言 exit mirror）。
* Dependencies or configuration: 新規 dep なし。#166 の
  `grant_set()`/`grants` member 検査/`{command, executed}`
  record・#165 の `ResolvedExecPolicy`/`is_route_eligible`/
  `pin_exec_grant`/`check_exec_execution` を前提。

## Verification

* [ ] `substitute:{grant,argv}` が schema 受理され、object
      以外・`grant`/`argv` 欠落・nested 未知 key・非 string
      要素で fail-loud route-load error（`argv: []` は受理）
* [ ] project 層 routes.yaml の `substitute` key presence が
      `InvalidRuntimePolicy` の route-load error で拒否され、
      builtin/user 層は受理 — `route.grant` 単数・byte-
      matched route は project 層で従来どおり有効
* [ ] substitute argv が `substitute.grant` pattern 外・
      grant 未存在・project-scope・fixed・pin 不能の各場合に
      viability `no-grant` fold（新規 viability 値なし）
* [ ] 原 argv が `route.grant` pattern と不一致でも
      substitute ペアが健全なら `runnable` で発火（retarget
      の回帰固定）・`route.grant` 欠陥は `no-grant`
* [ ] exec job が `{substitute.grant, substitute.argv}` を
      実行し expected_input byte 一致を通る（builtin
      workflow 無変更）・不一致は `exec-input-mismatch`
* [ ] substitute exit（0・非 0）が `route run` exit code に
      mirror される
* [ ] workflow deadline・exec job lifecycle が
      `substitute.grant` 宣言値由来
* [ ] `kind:"route"` record が `route` + `commandHash` +
      `executed:{substitute.grant, substitute.argv}` を持ち
      schemaVersion 1・`cheaper usage` 集計回帰なし
* [ ] interactive typed command（substitute 宣言ありでも）・
      interactive substitute ペアの route が
      `stdin-unsupported` で不発
* [ ] exec job・`outputs.execResult` を持たない workflow が
      substitute 宣言の有無に関わらず routed convention 違反
      で拒否（Option E 不実装の構造的確認）
* [ ] `substitute` 無しの既存 route（byte-matched・
      `grants` 宣言）の挙動に回帰なし

## Pros and Cons of the Options

### O1a `substitute:{grant,argv}` nested object（採用）

* Good, because compound value を object で宣言する既存
  schema 規約と一致し、ペアの required/atomic 検証と
  nested 未知 key 拒否が `match`/`target`/`output` と同じ
  fail-loud 構造に乗る。
* Bad, because route の top-level flat field 群に唯一の
  optional nested object が増え、schema 説明が 1 段深くなる。

### O1b flat field 対

* Good, because 既存 top-level flat 規約に揃う。
* Bad, because 相互 required 規約を新設する必要があり、
  partial 宣言を schema で弾く構造が nested object より弱い。

### O1c command-string 宣言

* Good, because 見た目が直感的。
* Bad, because tokenize/quote/metachar の扱いを schema へ
  持ち込み、「shell 非経由の argv 列」契約と表現 drift
  する。

### O1d mode-keyed schema

* Good, because mode 排他が明示的。
* Bad, because YAGNI（mode は 2 種）・`substitute` presence
  がそのまま switch になる・既存 routes.yaml への新規 key
  影響が大きい。

### O2a retarget + `route.grant` member 級（採用）

* Good, because 「実行する command」だけを完全検査し、
  `route.grant` を「set 内の非 exec grant」と同じ規則へ
  畳む — 親 design の条件付き弱化列挙（(a)+(b)）と 1:1
  対応し、第二の検査規則を作らない。
* Bad, because substitute mode で原 argv は grant pattern に
  照合されず、「typed command が認可済みのときのみ発火」
  の発火条件が `match.command` prefix まで後退する
  （semantic substitution の中核用途を可能にする意図的
  trade-off — trusted 層限定が埋め合わせ）。

### O2b dual check

* Good, because 発火条件が最も保守的（typed command 認可
  済み + substitute 認可済み）。
* Bad, because 実行されない command の認可を調べる形骸化
  検査を復活させ、「認可したくない command こそ置き換え
  たい」用途を根幹で破壊する — substitute するには typed
  command を認可する trusted grant が要り、その grant は
  `workflow run --grant` で直接呼べる実行面になる。

### O2c `route.grant` 無検査・投影外

* Good, because 実装が最小。
* Bad, because 「投影名は viability 検証済み」不変が壊れ、
  member job が `route.grant` を呼べず、attribution が
  未検証 grant 名を指す phantom になる。

### O3a trusted 層限定（採用）

* Good, because byte-match 保護が typed command へ乗り移ら
  ない substitute mode で、「実実行 `{grant,argv}` の
  authoring 権」を trusted 層に閉じ、O4c/O7a が塞いだ
  confused-deputy hole を宣言側でも構造的に閉じる。判別子
  は宣言層 provenance で `grants` と同じ error class・
  同じ presence-first 判定。
* Bad, because project 層 route は substitute を宣言できず、
  semantic substitution を使いたい project は builtin/user
  層への route 宣言を要する（route 自身の宣言・`grant:`
  単数は project 層のまま可）。

### O3b project 層受理

* Good, because schema uniform で実装差分が最小。
* Bad, because 「member を投影へ含める」以上の「実実行
  `{grant,argv}` の選択権」を untrusted 層へ渡し、
  「argv は grant pattern が bound」の緩和は選択権自体を
  制約しない — 文書化は検出不能な悪用を防げない。

### O3c project 条件付き受理（原 argv 認可前提）

* Good, because 発火条件を狭めた形で project へ開く。
* Bad, because 「typed command が認可済み」は「何を実行
  するか」への制約にならず、substitute ペアの選択権は
  project 作者のまま — hole の本体を閉じずに規約だけ
  複雑化する。

### O4a inputs に実行ペア（採用）

* Good, because builtin workflow が binding 無変更で両
  mode に動き、「exec job input = expected_input」の単一
  規約を保つ — `grant`/`execArgs` が「実行される grant・
  argv」を指す意味は byte-matched mode と同じ。
* Bad, because `grant` input の値が mode で分岐する
  （byte-matched では `route.grant`、substitute では
  `substitute.grant`）— 意味規約としては「実行 grant」で
  統一されるため実害はない。

### O4b 専用 substitute inputs

* Good, because 宣言 anchor と実行 anchor を別 input で
  表せる。
* Bad, because builtin workflow が mode 別 binding を要し、
  input 語彙が「宣言系/実行系」で分かれて表現 drift
  を生む。

### O4c workflow 宣言 substitute

* Bad, because workflow-declared grants と同型の信頼境界
  迂回で、byte-match gate の意味（宣言値以外を拒否）が
  反転する — 構造的に不成立。

### O5a record shape 不変（採用）

* Good, because `{command, executed}` split が substitute
  mode をそのまま表現し、record schemaVersion・reader・
  集計に変更を持ち込まない。attribution anchor は宣言側
  （`route` 宣言 + `route.grant`）に残り、実測分岐は
  `executed` が記録する。
* Bad, because firing-gate 認可 grant は `route` 名 →
  routes.yaml 宣言 join 経由でしか遡れず、書込み後の宣言
  編集で時点性の anchor が欠落し得る（現行 record 規約と
  同じ制約）。

### O5b 明示 anchor field 追加

* Good, because firing-gate 認可 grant を record へ凍結し、
  routes.yaml 編集後も時点性の anchor が遡れる。
* Bad, because 「実測 + route identity」の record 規約へ
  宣言 echo を混ぜ、需要未確立の field を YAGNI に追加
  する — 必要なら additive で後付けできる。

### O5c record anchor を `substitute.grant` へ

* Bad, because `route` 帰属が宣言軸（どの route が発火
  したか）から実測 grant へずれ、`cheaper usage` の
  per-route 集計が宣言軸と乖離する — 「実行されなかった
  command」を帰属にしない目的は `executed` が既に担う。

### O6a typed + substitute 双方に適用（採用）

* Good, because「interactive な typed command は routing
  で置き換えない」invariant を mode に関わらず保ちつつ、
  stdin 非接続で実際に実行される substitute の interactive
  リスクを resolve 時点で塞ぐ。分類可能な情報は両側に
  揃っている。
* Bad, because interactive な substitute executable/argv の
  route が不発になり、interactive な typed command も
  substitute 宣言があっても routing されない — 不発方向が
  増える（保守方向として意図的）。

### O6b substitute のみ

* Good, because 「実行されるものだけ検査」原則と見た目上
  一致する。
* Bad, because interactive な typed command を substitute
  route が intercept 可能になり、「interactive 入力は
  routing しない」invariant を緩める。

### O6c typed のみ

* Good, because 変更が最小。
* Bad, because interactive な substitute が grant timeout
  まで hang してから失敗する経路を残し、resolve 時点で
  分類可能なのに検査しない。

### O7a 投影 set へ `substitute.grant` を追加（採用）

* Good, because exec job の `resolve_exec_grant` 前提を
  満たし、`route.grant` を member として残すことで
  「投影名は全て viability 検証済み」の不変を維持する —
  投影判定位置・全件 eligible 必須の規約は既存のまま。
* Bad, because `route.grant`（実行しない grant）も投影に
  残り、confused-deputy 半径が最小化形より僅かに広い
  （member job から呼べる — ただし投影名は全て宣言済み・
  viability 検証済み）。

### O7b substitute mode で `route.grant` を投影外

* Good, because 投影面が最小化される。
* Bad, because member job が `route.grant` を呼べない
  非対称・`project_grants` の set 検査から anchor が外れる
  規約複雑化・record attribution が投影外の名を指す矛盾
  を伴い、得る安全性は薄い。

### O8a Option E は open 事項として残置（採用）

* Good, because author 宣言 exit を信用する成功偽装 hole
  を前提判断なしに開けず、substitute（Option D）の採用
  決定を独立して進められる。routed-exec convention は
  不変で、Option E を誤って紛れ込ませる形（exec job
  不要 route・`outputs.exitCode` mirror）を本 spec が
  明示的に排除する。
* Bad, because lint-only/analysis route の真の需要が早期に
  確認された場合に本 ADR の revisit と別 issue が要る。

### O8b Option E まで実装

* Bad, because measured → author-declared の exit 信頼
  モデル変更は substitute より大きく、未決判断を前提に
  実装すると成功偽装 hole を開ける。

### O8c Option E を恒久的に不採用

* Bad, because lint-only/analysis route の需要が将来確認
  された場合の revisit 経路を閉じ、採否の判断材料がまだ
  揃っていない段階で恒久決定をするのは誠実でない。

## More Information

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/167
- 上流 discovery: `../discovery/substitute-exec-route-mode-issue-167.md`
- spec: `../specs/substitute-exec-route-mode.md`
- 前提 ADR: `./route-grant-set-projection.md`（#166 —
  grant-set 投影・viability anchor/member 分割・`{command,
  executed}` record split — 本 ADR はその機構の直上に
  乗る）、`./defer-non-exec-routing-to-permission-model-redesign.md`
  （Option D 採用 sketch — 本 ADR はその採用要素の実装
  決定）、`./unified-exec-grant-model.md`（`is_route_eligible`
  単一 seam）、`./enforce-declared-contracts-over-content-audit.md`
  （宣言契約強制の責務モデル）
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  （anchor 規約・採否表・issue 分解 5/7）・
  `../specs/permission-model-responsibility-redesign.md`
- Option E（exec-less route）は引き続き open — 本決定に
  含めない。`route.grant` field は Option E でも firing
  gate として留保される確定設計と整合する形で required
  を維持した。

## Refine 注記（stdin 分類の tri-state 化）

O6a の「両側分類」は維持されるが、分類器は `is_interactive`
boolean から `classify_interaction`（`Batch`/`Approval`/
`Structural` の ordered tri-state）へ拡張された。typed 側・
substitute 宣言名・pin 解決名の union から **最悪階層** を採る
規約は不変: `Structural` → `StdinUnsupported`、`Approval` →
`ApprovalRequired`（hook rewrite は emit されるが承認 bypass
決定は併記されない）、`Batch` → `Runnable`。script 付き
interpreter 起動（`python check.py` 等）は `Batch` として
routable になり、bare REPL・UI app・stdin program のみが
`StdinUnsupported` に残る。

---
id: "TSPEC-034L2MB0m2NOWZsyuTHsjA"
type: "test-spec"
status: "approved"
title: "substitute-exec route mode の検証契約 (issue #167)"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
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
  related:
    - "docs/plans/substitute-exec-route-mode.md"
    - "docs/discovery/substitute-exec-route-mode-issue-167.md"
    - "docs/test-specs/route-grant-set-projection.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/route-run-delegated-exec.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "../designs/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/test-specs/route-usage-records.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
    - "docs/adr/substitute-exec-route-mode.md"
  derived-by: []
  verifies:
    - "docs/specs/substitute-exec-route-mode.md"
    - "docs/designs/substitute-exec-route-mode.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
    - "crates/cheaper-tools/src/tools/exec/mod.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
  defers: []
  deferred-by: []
---

# substitute-exec route mode の検証契約 (issue #167)

## Purpose

issue #167(#151 実装分解 5/7)で確定した「route が typed command
を実行せず宣言済みの substitute ペア `{substitute.grant,
substitute.argv}` を実行する substitute-exec mode」を、
declaration schema・宣言層制約・有効 set/実行ペア供給・
viability retarget・`route.grant` member 級降格・interactive
union 分類・投影境界・実行 anchor 切替・exit mirror・record
attribution・merge suppressed provenance・Option E 境界の
12 面で固定する。SPEC-034KXdoK2bMG3gQtIj8l6h の R1-R12・
Acceptance Criteria(AC1-AC12)と
DESIGN-034L0RxGVVTOqtaUUThowk の Verification Notes が
authoritative。

保つべき意図は 4 つ。(1) **「実行する command」を検査する
precheck の strict さ**: substitute mode でも実行ペアには
byte-matched anchor と同じ完全検査(存在 + `is_route_eligible`
+ pin + `check_exec_execution` argv 照合)が掛かり、実行され
ない原 argv の形骸化検査は除去される。(2) **fail-safe fold**:
substitute 側・member 級 `route.grant`・`grants[]` member の
いずれの失敗も `Viability::NoGrant` 一種へ畳み(新規 viability
値なし)、route 不発 → 原 command 直接実行。(3) **宣言層境界**:
`substitute` は trusted 層(builtin/user)限定 — byte-match が
typed command へ課していた拘束は substitute に乗り移らず、
project 層への「実実行 `{grant,argv}` 選択権」委譲は O7a が
塞いだ confused-deputy hole の再発として load 時に拒否する。
(4) **2 系統 anchor の分離**: 実行 anchor(`expected_input`・
deadline・lifecycle・`executed` echo)は `substitute.grant`、
firing gate・record attribution anchor は `route.grant` の
まま — `route`/`commandHash`/`schemaVersion:1` は不変。

退役条件: `substitute` 宣言の層制約・viability retarget 構造・
投影 set 規則・record `executed` 契約が変わるまで存続する。
Option E(exec-less route)が採用されても本規約(routed-exec
convention 必須・実行した command の exit mirror)は変わらない。

## Feature

routes.yaml route entry の optional `substitute: {grant, argv}`
(trusted 層限定)を `Route.substitute` へ parse し、presence を
mode switch とする。viability は anchor 完全検査の対象を
`Route::exec_pair` 経由の実行ペア(substitute mode は
`{substitute.grant, substitute.argv}`、byte-matched は
`{route.grant, 原 exec_args}`)へ retarget し、substitute
mode の `route.grant` は member ループと同じ検査(存在 +
`is_route_eligible` + pin)へ降格する。stdin-unsupported 分類は
typed 側を維持しつつ substitute ペアへ宣言 executable 名と
pin 解決名の union で適用する。route run 時は
`project_grants(&route.grant_set())` が `{route.grant} ∪
grants ∪ {substitute.grant} ∪ {fixed}` を投影し、
`expected_input`・deadline・inputs `grant`/`execArgs` は実行
ペアへ再 anchor、`argv`/`args`/`executable` は原 command
identity を維持する。process exit は実行した command
(substitute)の exit を mirror し、`kind:"route"` record は
`route`(宣言 identity)+ `commandHash`(原 command)+
`executed`(実測 substitute ペア)を `schemaVersion:1` のまま
記録する。merge は project `trusted:true` route が下位層
route を置き換えた際 displaced route を `suppressed`
provenance へ記録する。

> （issue #169 注記 — #168 で `Route.trusted` field は削除・merge は無条件化。project 層 route は下位層を shadow せず、同名衝突は常に project 側が負けて `suppressed` provenance へ記録される（`SuppressedRoute` semantics は「衝突で負けた project route」へ狭義化）。project 層 `trusted` key presence は値不問で `InvalidRuntimePolicy`（strict 経路は load error・hook 経路は fail-open で route arm 不活性）。builtin/user 層の `trusted:` key は受理されるが inert）

## Rules

- Rule: **declaration schema — `substitute:{grant,argv}`**
  (spec R1・ADR O1a — `routes.rs` unit test)
  - `substitute: {grant: <name>, argv: [<arg>, …]}` の parse
    で `Route.substitute` が `Some(RouteSubstitute)` になる。
    `grant`・`argv` の 2 key は required — 片方の欠落は
    `schema_error` → `InvalidRuntimePolicy`。
  - `substitute` が object でない(非 object)→ `"{label}.
    substitute: not an object"`。nested 未知 key →
    `"{label}.substitute: unknown key: {key}"` —
    `match`/`target`/`output` と同じ fail-loud 規約。
  - `substitute.grant` は non-empty string のみ検証 —
    `command.grants` の既存 grant 名への参照で、未存在・
    project-scope・fixed 名の宣言自体は legal(失敗は
    viability `NoGrant` へ fold — `grant` 単数と同じ「存在/
    eligibility は viability へ委譲」規約、第二の検証規則を
    作らない)。
  - `substitute.argv` は string 配列で executable を含まない
    実行 args 列(command tool input `args` と同形)。`argv:
    []` は executable 単独起動の明示宣言として受理。各要素は
    non-empty string — 非 array・非 string 要素・空 string
    要素は `InvalidRuntimePolicy` + `details.{path,
    reason:"schema", violation}` で file・entry 名指しの
    route-load error。
  - `grant`(単数)は substitute の有無に関わらず required の
    まま(firing gate・record attribution anchor・投影
    member)。`substitute` の typo(`substitut` 等)は
    `KNOWN_KEYS` への `"substitute"` 追加のもとで unknown
    key 拒否(fail-loud 規約不変)。
  - `substitute.grant == route.grant`(同一 grant の argv
    差替)・`substitute.grant ∈ grants[]`(冗長だが `grant_set()`
    が dedupe)はともに合法 — 拒否規約を持たない。

- Rule: **宣言層制約 — trusted 層限定**(spec R2・ADR O3a —
  `routes.rs` unit + `route.rs` integration)
  - project 層(`<project_dir>/routes.yaml`)の route entry が
    `substitute` key を含む場合、`validate_route` が**形状検査
    より先に** presence で `schema_error("{label}.substitute:
    not declarable in the project layer")` →
    `InvalidRuntimePolicy` を返す fail-loud route-load error
    — 値が malformed でも presence が先に拒否される
    (untrusted 層の宣言は内容を評価しない fail-closed、
    `grants` O7a と同型の判別子・同じ宣言層 provenance 判別)。
  - builtin/user 層の `substitute` 宣言は受理される。project
    層の `grant:` 単数宣言・byte-matched route は従来どおり
    有効。`trusted: true` 宣言は判別に使わない(project 自己
    申告の shadowing gate — 判別子は宣言層 provenance)。
    > （issue #169 注記 — project 層 `trusted` は判別以前に presence 自体が `InvalidRuntimePolicy` で拒否される（#168 — `{label}.trusted: not declarable in the project layer`、値不問・strict 経路は error・hook 経路は fail-open で route arm 不活性）。builtin/user 層の `trusted:` key は受理されるが inert — `Route.trusted` field は削除済み）
  - `load` の厳密経路(`route resolve`/`route run`)は error
    をそのまま返し、hook の `load_or_none` は `None` fold で
    route arm 不活性へ落ちる(既存 fail-open 規約)。

- Rule: **`Route::grant_set()`/`Route::exec_pair()` — 有効
  set・実行ペアの単一 owner**(spec R5・ADR O2a/O7a —
  `routes.rs` unit test)
  - `grant_set()` が `{route.grant} ∪ grants ∪
    {substitute.grant}` の `BTreeSet` を返す — substitute 無し
    なら従来どおり `{route.grant} ∪ grants`。
    `substitute.grant == route.grant`・`substitute.grant ∈
    grants[]` は dedupe で単一要素になる。
  - `exec_pair(typed_args)` が substitute mode で
    `(substitute.grant, substitute.argv)`、byte-matched で
    `(route.grant, typed_args)` を返す — viability 完全検査の
    対象・`expected_input`・inputs `grant`/`execArgs`・
    deadline 導出が全てこの anchor に従う単一の供給点
    (firing/attribution anchor の `route.grant` とは別系統)。
    「実行ペア選択」を viability と run 配線で二度書かない。

- Rule: **viability retarget — 実行ペアの完全検査**(spec R3・
  ADR O2a — `resolve.rs` unit + `route.rs` integration)
  - substitute 宣言のある route で anchor 完全検査の対象は
    `route.exec_pair(&out.exec_args)` が選ぶ実行ペア —
    `substitute.grant` の存在 + `is_route_eligible`
    (`scope=="trusted" && !fixed` の単一 seam)+
    `pin_exec_grant` + `ExecToolOptions` 構築 +
    `check_exec_execution(substitute.grant, substitute.argv)`
    の argv pattern 照合。`ExecToolOptions` の構築内容は
    byte-matched と同一。
  - `substitute.argv` が `substitute.grant` の argv pattern
    に不一致・`substitute.grant` が未存在・`scope:"project"`・
    `fixed`・pin 不能(unresolvable・`execDenyPatterns` 衝突)・
    cwd prepare・pin verify 失敗のいずれでも
    `Viability::NoGrant` — 新規 viability 値なし、`route
    resolve` 出力は理由を区別しない。
  - **原 argv の `route.grant` argv pattern 照合は行わない**
    — 実行されない command の形骸化検査の除去が retarget の
    本質(「認可したくない command こそ置き換えたい」用途の
    ため、typed command の認可は発火条件ではない)。

- Rule: **`route.grant` の member 級降格**(spec R4・ADR O2a —
  `resolve.rs` unit)
  - substitute 宣言のある route で原 argv が `route.grant` の
    argv pattern に一致**しない**場合も、substitute ペアが
    健全なら `runnable`(retarget の回帰固定 — AC4)。
  - `route.grant` は `grants[]` member と同じ位置・同じ検査
    (存在 + `is_route_eligible` + `pin_exec_grant`、argv 照合
    なし、`check_exec_execution`/`verify_pinned_executable`/
    cwd prepare は呼ばない)で評価される — 未存在・
    ineligible・pin 不能は `NoGrant` fold(member 失敗と区別
    不能、`tracing::debug!` に member 名)。
  - `route.grant` は firing gate・record attribution
    anchor・投影 member・member job からの呼出し対象として
    残る(完全検査の対象ではなくなるだけ)。

- Rule: **stdin 分類は両側 tri-state — 宣言名∪解決名の
  union で最悪階層**(spec R10・ADR O6a — `resolve.rs` unit)
  - typed 側維持: 原 executable/原 exec_args が `Structural`
    なら substitute 宣言の有無に関わらず `StdinUnsupported`
    (「stdin 依存 typed command は routing で置き換えない」
    invariant)。
  - substitute 側追加: `classify_interaction` を **宣言 basename**
    (`command_name(&grant.executable)`)と **pin 解決済み
    basename**(`command_name(&pinned)`)の union +
    `substitute.argv` へ適用 — いずれかが `Structural` なら
    `StdinUnsupported`、`Approval` なら `ApprovalRequired`、
    全て `Batch` なら `Runnable`(最悪階層を採る)。
  - union の両側を pin: (a) 宣言名が list 内で pin 解決先が
    canonical 名ずらし(`python3`→`python3.11`、`vim`→
    `vim.basic`、`vi`→`nvi` 相当の fixture)でも
    `StdinUnsupported`、(b) 宣言名が list 外でも pin 解決
    済み実体名が list 内なら `StdinUnsupported`(shim 名での
    回避を塞ぐ)。
  - `grants[]` member は screening 対象外のまま(known
    semantics — member job は自身 grant bound で動く別経路)。

- Rule: **投影境界 — `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}`**(spec R5・ADR O7a —
  `run.rs` prepare test + `proc_invocation` parity 回帰)
  - `route run` prepare は `project_grants(&route.
    grant_set())` を呼び、set 内 1 件でも未存在/ineligible
    なら `denied("no-grant")` `PolicyViolation`(viability
    通過後の二重防御 — #166 規約不変)。
  - 投影後 `supply.policy.exec.grants` は宣言 set + fixed に
    限定され、undeclared 名を `input.name` に指定した job は
    `resolve_exec_grant` で `PolicyViolation`。`execDenyPatterns`
    は `substitute.grant`・member 級 `route.grant` の
    `pin_exec_grant` に uniform に効く(deny される grant は
    ineligible → `NoGrant`)。
  - `substitute.grant` が投影に含まれなければ exec job の
    `resolve_exec_grant` が失敗する — set への追加は必須
    条件として pin(`prepare_projects_grant_set` 系 test に
    substitute 含有 case)。

- Rule: **実行 anchor — `expected_input`・inputs・deadline・
  lifecycle**(spec R6/R8・ADR O4a — `route_run.rs`
  integration + `run.rs` unit)
  - `expected_input = {"name": substitute.grant, "args":
    substitute.argv}` — byte-match gate(`job_id=="exec" &&
    tool=="command"` のみ適用)・`reason:"exec-input-mismatch"`
    `PolicyViolation`・文言は不変。照合対象値が substitute
    ペアへ替わるだけ。
  - routed workflow inputs の `grant`/`execArgs` は実行ペアを
    運び、`argv`/`args`/`executable` は原 command identity
    を維持 — builtin `routed-exec.workflow.json` が binding
    無変更(`{name: $input.grant, args: $input.execArgs}`)で
    両 mode に動作する契約。
  - `job_id == "exec"` で `route.grant`・原 exec_args・member
    名・投影外名を指定する job は `exec-input-mismatch` で
    失敗する。
  - workflow deadline は `substitute.grant` の宣言 `timeout`
    から導出(`deadline_ms` は不変、`grant_timeout` lookup 名が
    実行ペア名)。exec job の `timeout`/`killGrace`/
    `exitCodes`/`cwd`/`env` は `command::execute` が named
    grant(=`substitute.grant`)の宣言値を読む既存規約 —
    `route_context`(exec_cwd=payload cwd・passthrough
    `extra_env`・`accept_any_exit_code`・`log_tee`)は mode
    共通で適用。
  - `grants[]` member job は従来どおり自身 grant の宣言
    bound(`route_context` 非適用) — workflow deadline は実行
    grant timeout のみ由来で max-of-set 化はしない(known
    semantics)。

- Rule: **exit mirror — 実行した command の exit**(spec R7 —
  `route_run.rs` integration)
  - substitute command の exit code(0・非 0)が `route run`
    の process exit code としてそのまま mirror される —
    captured `details.exitCode` を mirror する機構・
    `accept_any_exit_code` による非 0 透過は不変。

- Rule: **record attribution — `route`/`route.grant` anchor
  + 実測 `executed`**(spec R9・ADR O5a — `route_run.rs`
  record case + `usage_log` shape 回帰)
  - `kind:"route"` record は `route`(route 宣言名 —
    `route.grant` anchor の宣言 identity)・`commandHash`
    (原 `payload.command` の `DefaultHasher` hex — env-prefix
    値を含み得るため hash のまま)を不変で持つ。
  - `executed:{grant, argv}` は実測 echo — substitute mode
    では `{grant: substitute.grant, argv: substitute.argv}`。
    `schemaVersion: 1`・additive 規約・record timing(captured
    result 必須)は #166 のまま、新規 field なしで
    `cheaper usage` 集計が従来どおり動く。
  - `exitCode`・`ok` は mirror した substitute exit 由来。
    member job の実行は `kind:"route"` record に直接載らない
    (#166 規約不変)。

- Rule: **member 併用と `job_id != "exec"` 規約**(spec
  AC10・R5 — `route_run.rs` integration)
  - substitute route が `grants[]` member と併用でき、
    `input.name` が member 名の `job_id != "exec"` job は
    従来どおり動作する(expected_input gate 非適用・
    `route_context: None`・member grant の宣言 argv pattern・
    lifecycle/cwd/env/exitCodes)。
  - `input.name = route.grant`・`input.name = substitute.
    grant` の member job(`job_id != "exec"`)は投影内にある
    ため到達できる — substitute.grant への到達は member
    規約(argv pattern 照合のみ)で pin。
  - `job_id == "exec"` で member 名を指定する job は
    `exec-input-mismatch` で失敗(authoring 規約の pin)。

- Rule: **merge suppressed provenance — project `trusted:true`
  置換の displaced 記録**(design §(a)・§Risks —
  `routes.rs` unit + `route_run.rs` integration)
  - project `trusted:true` route が同一 `match.command` で
    下位層(builtin/user)route を置き換えたとき、displaced
    route を `suppressed` provenance へ記録する(`insert`
    戻り値利用 — `SuppressedRoute` semantics は「merge 敗北
    route」へ拡張、shape `{name, command}` 不変)。新規追加
    では `insert` が `None` を返し no-op。
  - builtin/user 層間の shadow(user → builtin)は同一 trusted
    principal の宣言 override として silent のまま。記録
    対象は untrusted 境界をまたぐ project 層の `trusted:true`
    置換のみ。
  - consumer は不変: `resolve` の `suppressed()` は
    `.find(is_prefix)` で displaced route 名を拾い、
    `RouteResolution.suppressed`・`route resolve` 出力・
    `kind:"route"` record の `suppressed` へ既存経路で乗る
    (新規 field・shape 変更なし)。
  - 旧 semantics を pin する 2 test を更新:
    `user_shadows_builtin_and_project_needs_trusted`
    (`routes.rs` unit — `suppressed.is_empty()` assert が
    displaced user route 名を返すように反転)と
    `suppressed_project_provenance_reaches_route_record`
    (`route_run.rs` — trusted branch が resolve 出力・record
    の `suppressed` を assert するように反転)。
  > （issue #169 注記 — #168 で merge の `trusted` 置換 arm 自体が除去・merge は無条件化。project 層 route は下位層を shadow せず同名衝突は常に project 側が負けて `suppressed` へ記録（`SuppressedRoute` semantics は「衝突で負けた project route」へ狭義化）。project 層 `trusted` key presence は load 時 `InvalidRuntimePolicy`（strict）/ fail-open（hook 経路）。上記 2 test は `project_layer_trusted_is_rejected`・`project_trusted_violation_disables_route_arm_fail_open` 系へ置換済み）

- Rule: **match precedence interplay(known semantics の
  pin)**(design §Risks・Verification Notes review P1-1 —
  `resolve.rs` unit + `routing_policy.rs` route eval)
  - longest `match.command` の best 1 件のみが viability
    評価される — non-runnable best(`NoGrant`/
    `MissingTarget`/`InvalidRoute`)から shorter match への
    fall-through はない。project 層が substitute route より
    長い `match.command` を持つ non-runnable route を宣言した
    とき、route arm は rewrite せず typed command が直接実行
    される(現行 precedence を pin — substitute 固有の hole
    ではなく byte-matched でも同じ非対称)。
  - project `trusted:true` route が builtin/user substitute
    route を同一 `match.command` で shadow するとき、
    effective set は project route を採用し `suppressed`
    provenance が displaced route 名を記録する
    (`route resolve` 出力 `suppressed` field で pin)。
    > （issue #169 注記 — #168 でこの分岐は到達不能: project 層 `trusted` key presence は load 時 `InvalidRuntimePolicy` で拒否（strict）/ fail-open（hook）。同名衝突は常に project 側が負けて `suppressed` へ記録される — project route が effective set に採用される経路は存在しない）

- Rule: **Option E 構造的境界 — routed convention 不変**
  (spec R11・ADR O8a — `route_run.rs` integration)
  - exec job を持たない・`outputs.execResult` whole binding
    を持たない workflow は substitute 宣言の有無に関わらず
    routed convention 違反(`InvalidRuntimePolicy` —
    `reason:"missing-exec-job"`/`"exec-job-shape"`/
    `"execResult-output"`)で拒否される
    (`validate_routed_convention` 不変の確認)。
  - `route resolve` 出力 shape 不変 — substitute echo は
    載らず、`grant` field は `route.grant` anchor を指す
    (R11 — `route.rs` integration の shape assertion)。

- Rule: **fail-open・byte-matched 回帰無し**(spec R12・
  AC12 — 既存 route/hook/run test 群)
  - `load_or_none` が route/policy load 失敗(substitute
    schema error を含む)で `None` を維持し hook arm が不活性
    へ落ちる。`supply::load` 厳密経路は error をそのまま
    返す。
  - substitute 宣言 route が hook eval 経路
    (`evaluate_route_arm`)でも同じ resolve を通り runnable
    時のみ rewrite される。
  - `substitute` を持たない既存 route(byte-matched・
    `grants` 宣言)の viability・exec-input・exit mirror・
    record 挙動に回帰がない — 既存 `resolve.rs`/`routes.rs`/
    `run.rs` unit・`route.rs`/`route_run.rs`/`usage_log.rs`/
    `hook.rs`/`proc_invocation.rs`(cheaper-tools)test 群が
    無改訂で pass。**例外は displaced-trusted provenance の
    2 assert 更新のみ**(上記 merge suppressed Rule)。
    > （issue #169 注記 — #168 で当該 2 test は merge 無条件化（project 衝突敗北の `suppressed` 記録）へ再 pin 済み。「displaced-trusted provenance」の旧 semantics pin は現行 test 群の記述ではない）
  - `route_cmd` 出力 shape・`payload` schema・`workflow_run`
    gate・`usage_log`/`usage_report`・`policy/exec.rs`・
    `tools/*`・`routed-exec.workflow.json`・`hook/targets.rs`
    は無改訂で維持される。

- Rule: **full pipeline**(`mise run check` 全緑 +
  `node scripts/check-doc-ids.mjs` + `audit_docs` 0 error —
  design Verification Notes 末尾)

## Examples

- Example: user 層 route `[cargo, build]` が
  `substitute: {grant: cargo-check, argv: [check, --workspace]}`
  を宣言 → schema 受理、viability は `cargo-check` grant の
  pin + `check --workspace` の argv pattern 照合を完全検査し
  `runnable`。typed `cargo build --release` の原 argv は
  `route.grant`(cargo)の pattern に照合されないまま発火
  (spec AC1・AC4)。
- Example: project routes.yaml の route entry に
  `substitute: {grant: x, argv: []}` → `load` が
  `InvalidRuntimePolicy`(violation `…substitute: not
  declarable in the project layer`、file・entry 名指し)で
  file 全体を拒否 — 値が `{grant: 1}` と malformed でも
  presence が先に拒否される。builtin/user 層の同じ宣言は
  受理。project 層の `grant:` 単数・byte-matched route は
  受理(spec AC2)。
- Example: `substitute.grant` が policy に無い・
  `scope:"project"`・`fixed:true`・pin 不能・
  `substitute.argv` が argv pattern 不一致 → いずれも
  viability `no-grant` で route 不発 → 原 command 直接実行
  (新規 viability 値なし — spec AC3)。
- Example: substitute 宣言のある route で `route.grant` が
  未存在・`scope:"project"`・`fixed`・pin 不能 → `no-grant`
  (member 級降格 — spec AC4)。
- Example: `substitute.grant` の宣言 executable が `python3`
  で pin 解決先が `python3.11`(canonical 名ずらし)→ bare
  argv では宣言名側で `stdin-unsupported`、positional argv
  `["x.py"]` では `runnable`。宣言名が `mypython`
  (list 外)でも pin 解決済み実体名が `python3.11` なら
  bare argv で解決名側 `stdin-unsupported`。`substitute.argv`
  に `docker -it` 等の interactive flag 含有でも同じ
  `stdin-unsupported`(spec AC9)。
- Example: substitute route の routed workflow(builtin
  `routed-exec.workflow.json` 無変更)で exec job が
  `{name: substitute.grant, args: substitute.argv}` を実行し
  `expected_input` gate を通る。`job_id == "exec"` で
  `route.grant`・原 exec_args・member 名を指定する job は
  `exec-input-mismatch`(spec AC5)。
- Example: substitute command が exit 7 で終了 → `route run`
  の process exit code が 7(`accept_any_exit_code` で bounded
  job error にならず透過 — spec AC6)。
- Example: `substitute.grant` の `env`/`timeout` と
  `route.grant` の `env`/`timeout` が異なる宣言で、
  envdump 系 fixture が substitute grant の env を子へ届け、
  deadline が substitute grant の timeout + 120s に従う
  (spec AC7 — `run.rs` unit の deadline 選出 + `route_run.rs`)。
- Example: `kind:"route"` record が `route`(宣言名)・
  `commandHash`(原 command)・`executed:{grant:
  substitute.grant, argv: substitute.argv}` を `commandHash`
  直後に持ち、`schemaVersion: 1`・新規 field 無しで
  `cheaper usage` の route bucket 集計が動く(spec AC8)。
- Example: `grants: [fmt]` + `substitute` を宣言した route で
  `input.name = "fmt"` の `job_id != "exec"` job が動作し、
  `input.name = "undeclared"` は `PolicyViolation`、
  `input.name = substitute.grant` の member job は member
  規約で到達できる(spec AC10・design Verification Notes の
  AC gap 項目)。
- Example: project `trusted:true` route `[git,push]` が builtin
  substitute route `[git,push]`(dry-run 化)を shadow →
  effective set は project route、`suppressed` provenance が
  displaced builtin route 名を記録し `route resolve` 出力・
  `kind:"route"` record の `suppressed` に乗る(design §(a)
  merge 拡張)。
  > （issue #169 注記 — 現行では project 層 `trusted` key presence は load 時拒否（strict）/ fail-open（hook）でこの宣言自体が到達不能。`trusted` 非宣言の project route は同名衝突で builtin/user に負けて `suppressed` へ記録される — project route が effective set に採用される経路は存在しない（#168））
- Example: project 層が `[git,push,origin]` + ineligible
  `grant:` の route を宣言 → longest-match の best のみ評価
  で non-runnable、route arm は `None` fallthrough で typed
  `git push origin main` が直接実行される(known semantics
  pin — shorter match への fall-through なし)。
- Example: `outputs.execResult` を持たない・`job_id == "exec"`
  の job を持たない workflow を target に持つ substitute
  route → `InvalidRuntimePolicy`(`missing-exec-job`/
  `execResult-output`)で拒否(Option E 不実装の構造的確認
  — spec AC11)。

## Guarantees

- 実装者が design を再解釈しなくても、schema parse・層制約
  error・`grant_set`/`exec_pair` union・viability retarget・
  member 級降格・interactive union・投影境界・実行 anchor・
  exit mirror・record `executed`・merge suppressed
  provenance・Option E 境界の全項目が unit + integration +
  parity test で機械的に検証できる。
- substitute mode でも実行ペアに byte-matched と同レベルの
  完全事前検査が掛かり、「実行されない command」の形骸化
  検査は viability から排除される。
- substitute 側・member 級 `route.grant`・`grants[]` member
  の失敗は全て `NoGrant` 一種へ畳まれ、新規 viability 値・
  `route resolve` 出力 shape・viability enum が増えない。
- 宣言権が trusted 層に限定され、project 層の `substitute`
  presence が形状検査より先に fail-loud 拒否される非対称
  境界が pin される(`grant:` 単数・byte-matched route は
  project で宣言可)。
- record `executed` が substitute ペアの実測 echo で
  `schemaVersion:1`・`route`/`commandHash` anchor を侵さず、
  pipeline(`finish`/`build_route_record`/`usage_report`)は
  code 不変のまま substitute ペアを運ぶことが e2e で pin
  される。
- project `trusted:true` による trusted substitute route の
  silent displacement が `suppressed` provenance で追跡可能に
  なり、`route resolve`・`kind:"route"` record の両面に
  手がかりが残る。
  > （issue #169 注記 — #168 で `suppressed` semantics は「衝突で負けた project route」へ狭義化。project `trusted:true` 宣言は load 時拒否のため displacement 経路は存在せず、追跡対象は同名衝突で負けた project route の記録となる）
- Option E(exec-less route)の構造的境界が convention 検証で
  pin され、author 宣言 exit を信用する設計が混入しない。

## Non-goals

- Option E(exec-less route)— exec job を持たない routed
  workflow・`outputs.exitCode` 等の workflow 宣言値 exit
  mirror・「何も実行しない」route(親 chain の open 事項 —
  構造的拒否の pin のみ本 spec の対象)。
- `route resolve` 出力への substitute echo・viability 値の
  追加(R11 出力 shape 不変 — 診断需要は Deferred)。
- usage record の schemaVersion bump・新規 field・
  firing-gate anchor grant 明示 field(O5a/O5b — 監査需要は
  Deferred)。
- typed args の substitute argv への転送・補間(宣言凍結 —
  別 issue)。
- `grants[]` member の interactive screening・member argv の
  route 解決時照合(known semantics — #166 O2a 不変)。
- non-runnable best match からの shorter match への
  fall-through(match precedence 全体の別設計 — known
  semantics として pin のみ)。
- `is_route_eligible`・`project_grants`・`check_exec_execution`・
  `pin_exec_grant`・`authorize_args`・`RouteExecution::new`・
  `validate_routed_convention` の signature/predicate 変更
  (全て不変 — substitute も同じ seam を通る)。
- builtin/user 層間 shadow の suppressed 記録(同一 trusted
  principal の宣言 override — 記録対象は project 層置換のみ)。
  > （issue #169 注記 — #168 で `suppressed` は「衝突で負けた project route」のみを運ぶ semantics へ狭義化。builtin/user 層間 shadow が記録対象外なのは維持）
- 検証コマンド・実行結果の記録 — task の `## Verification`
  と impl record の責務。

## Risk

- member ループ位置へ `route.grant` を降格する際に anchor
  完全検査の対象を `exec_pair` へ替え忘れる・または原 argv
  照合を残すと、「実行されない command の形骸化検査」が残存
  し destructive→dry-run の中核用途を破壊する — retarget・
  原 argv 非照合・member 級降格の 3 点を pin で固定。
- interactive 分類を宣言名のみにすると canonical 名ずらし
  (`python3`→`python3.11`)で、解決名のみにすると shim 名で
  系統的に回避される — union 両側の pin が必要。
- project 層 `substitute` を受理・または形状検査より後に
  presence を見ると、byte-match 保護の届かない
  confused-deputy hole(実実行 `{grant,argv}` 選択権)が再発
  する — presence 先出し + `InvalidRuntimePolicy` の pin が
  防衛線。
- `grant_set()` に `substitute.grant` を入れ忘れると exec
  job の `resolve_exec_grant` が投影 policy 解決失敗で落ち、
  宣言した substitute route が必ず `no-grant` で不発になる
  — 投影含有 case の pin が必要。
- merge の `suppressed` 記録を untrusted project 敗北だけに
  残すと、project `trusted:true` が trusted substitute を
  silent に置き換えて追跡不能になる — displaced 記録 +
  consumer 不変の pin が必要(旧 semantics pin 2 test の
  assert 更新漏れは compile でなく assertion 失敗として顕在化
  するため design Churn 一覧の site で網羅)。
  > （issue #169 注記 — #168 で `Route.trusted` field 削除・merge 無条件化によりこの risk は現行構造では成立しない: project `trusted` 宣言は load 時拒否・同名衝突は常に project 敗北で `suppressed` 記録。`suppressed` semantics は「衝突で負けた project route」へ狭義化済み）
- `expected_input`・deadline・inputs が `route.grant`・原
  exec_args に anchor されたままだと、exec job が宣言と異なる
  ペアを要求/実行して `exec-input-mismatch` で常に失敗するか
  誤った lifecycle bound が掛かる — `exec_pair` 単一供給点
  からの anchor 化と e2e pin が必要。
- Option E 的設計(exec job 不要・workflow 宣言 exit)が
  substitute 実装に紛れ込むと author 宣言 exit を信用する
  成功偽装 hole が開く — convention 拒否の回帰 pin で構造的
  に排除。

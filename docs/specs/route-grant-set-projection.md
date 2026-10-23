---
id: "SPEC-034KTMfXYD5ZlGZCRb8Paj"
type: "spec"
status: "approved"
title: "route grant-set projection — grants 宣言・viability 条件緩和・usage record split"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
    - "../discovery/route-grant-set-projection-issue-166.md"
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
    - "../adr/route-grant-set-projection.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
    - "./substitute-exec-route-mode.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/route-grant-set-projection-issue-166.md"
  derived-by:
    - "../designs/route-grant-set-projection.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../designs/unified-exec-grant-model.md"
    - "../specs/unified-exec-grant-model.md"
    - "../adr/unified-exec-grant-model.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# route grant-set projection — grants 宣言・viability 条件緩和・usage record split

## What Should Be Built

route が宣言する **exec grant 集合**（`grants: [name, …]`）を routed
workflow の exec 委譲面へ投影し、あわせて route/hook 側の
audit-dependency 整理・viability の条件緩和・route usage record の
attribution split を行う。#165 の unified `ExecGrant` モデル上で、
route 1 件が委譲できる grant を `{route.grant}` 単体から
**宣言された set** へ一般化する。

構成要素は 4 つ:

1. **route/hook audit-dependency cleanup** — viability の
   unsafe-command 分岐が依存する `argv_shell_re`（metachar 走査）は
   #163 で exec/policy 層から除去された内容審査の残存経路。
   route 層からも除去し、authorization への委譲構造を完成させる。
2. **route grant-set 宣言と投影** — routes.yaml の route entry に
   optional な `grants: [name, …]` を追加し（宣言は trusted 層の
   routes.yaml 限定 — project 層は fail-loud 拒否）、route run
   時に `{route.grant} ∪ grants ∪ {fixed grants}` を policy へ
   投影する（現行 `project_grant` は `{route.grant} ∪ {fixed}`
   のみ）。
3. **viability の条件緩和** — firing gate である `route.grant` は
   現行どおり byte-matched の argv 事前検査を維持し、set の
   追加メンバー（non-exec grant）は存在確認 + pin 解決のみを
   route 解決時に行い、argv 照合は job 実行時へ遅延する。
4. **usage record の `{command, executed}` split** — route record が
   保持する `command`（= `commandHash`、原 command のハッシュ）を
   据え置き、実 exec argv を記録する `executed` field を追加して、
   「要求された command」と「実際に実行された exec」の attribution
   を分離する。

## Why Now

- #163/#164 で exec/policy 層の内容審査が除去・opt-in 化され、
  route viability に残った `argv_shell_re` 分岐が「内容審査に依存する
  最後の mandatory 経路」として親 spec（permission-model
  responsibility redesign R6/AC）で整理対象に明示された。
- #165 で `command.grants`/`process.tasks` が単一 `ExecGrant` へ統合
  され、`is_route_eligible`（`Trusted && !fixed`）という単一の
  route 参照判定 seam が確立した。set 投影はこの seam を再利用
  できる（#165 IR が将来拡張として `grants[]` を明示）。
- routed workflow が実行したい job は `route.grant` が pin する
  executable だけとは限らない。現行の単一 grant 投影では
  routed workflow 内の追加 command job が `route.grant` しか
  呼べず、補助 executable（例: 整形・集計）を route から委譲
  できない。宣言 set 投影で confused-deputy 半径を
  「宣言された grant の argv union」に閉じたまま拡張する。
- 将来の substitute mode（issue #167）で「原 command」と
  「実 exec」が分岐する前に、usage record の attribution を
  `{command, executed}` へ分離しておくと record schema の
  前方互換が取れる。

## Users and Value

- **route 作者（operator/user layer）** — 1 route で複数の
  trusted grant を workflow へ委譲できる。補助 executable の
  ために route を増やす必要がない。
- **parent agent / hook consumer** — viability が内容審査へ
  依存しなくなり、`git log -G 'a|b'` のように quoted metachar を
  含む正当な argv で route が誤って不発になる偽陰性が消える。
- **cheaper 保守者** — authorization の単一実装
  （`check_exec_execution` + `is_route_eligible`）への委譲構造が
  完成し、route 層に内容審査の複製が残らない。

## Scope

### In Scope

- routes.yaml route entry への `grants?: [name, …]` 追加
  （schema・validation・`Route` struct・fixture・
  `resources/routes.yaml` の shape 記述）
- `grants` 宣言の層制約 — trusted 層（builtin/user）限定。
  project 層 routes.yaml の `grants` key presence を
  fail-loud route-load error で拒否（anchor `route.grant`
  単数は project 層でも引き続き宣言可）
- `project_grant` の set 版への一般化と、route run 時の
  `{route.grant} ∪ grants ∪ {fixed}` 投影
- viability の条件緩和 — `route.grant` は byte-matched 完全検査、
  追加 set メンバーは existence + eligibility + pin 解決のみ
- `argv_shell_re` 依存分岐の除去（route viability の metachar
  走査 + 定義本体。crates 内 consumer は route viability のみ）
- route usage record への `executed` field 追加
  （`{grant, argv}` — captured exec result details 由来）
- `expected_input`・lifecycle bounds・exit mirror の anchor 規約の
  明文化（route.grant = 発火 gate 兼 exec anchor として不変）

### Out of Scope

- **substitute mode**（`substitute.grant` による exec 差替 —
  issue #167。record split はその前提整備だが substitute 自体は
  本 spec に含めない）
- **workflow 宣言の grant** — grant set の出所は routes.yaml の
  route 宣言のみ。workflow 側から grant を要求する経路は作らない
  （confused-deputy 半径の宣言境界を route に固定するため）
- **非 exec workflow target の routing** — script/read 等の
  非 exec 委譲は deferred sketch（ADR-034KtTzOJGqmzmBW3SwGp5）
  側の論点であり本 spec では扱わない
- usage record schemaVersion の bump・`commandHash` の
  削除・後方互換 reader — `executed` は additive field のみ
- fixed grant を set メンバーとして宣言する機能 — fixed grant は
  既に無条件で投影される。`grants` 内の fixed 名は ineligible 扱い
- project-scope grant の set メンバー化 — `is_route_eligible`
  （trusted-scope のみ）をそのまま維持する

## Requirements

- R1. routes.yaml の route entry に optional な `grants: [name, …]`
  を追加すること。`grant`（単数）は required のまま据え置き、
  route の発火 gate・exec anchor・record attribution anchor と
  しての意味を変えないこと。有効 set は
  `{route.grant} ∪ grants` であり、`grants` 内の `route.grant`
  自身の重複・メンバー間重複は冪等に dedupe する（順序は
  意味を持たない）。`grants: []` は未指定と同値。宣言数上限は
  `passthroughEnv` 等の既存宣言配列と同じく課さない。
  未知 key の拒否を含む既存の fail-loud schema 規約
  （`KNOWN_KEYS`・`validate_route`）を維持すること。
  なお `grants` を宣言できるのは trusted 層（builtin/user）の
  routes.yaml のみとし、project 層（`<project_dir>/routes.yaml`）
  の route entry が `grants` key を含む場合は fail-loud な
  route-load error で拒否すること — member は anchor と違い
  typed command への argv-match で拘束されず、untrusted 層への
  member 宣言権委譲は project policy が `command` section を
  持てない既存の信頼境界（ADR O4c/O7a）と矛盾する。
  anchor `route.grant` 単数は従来どおり project 層でも宣言可。
- R2. set メンバーの eligibility は `is_route_eligible`
  （`scope == Trusted && !fixed`）で一律に判定すること。
  未存在・project-scope・fixed の名前はいずれも ineligible と
  し、route.grant と同じ判定 seam を再利用して第二の
  eligibility 規則を作らないこと。
- R3. viability は次の条件付き構造にすること:
  (a) `route.grant` — 現行どおり完全検査（存在 + eligibility +
      pin 解決 + `check_exec_execution` による argv 照合 +
      cwd/pin verify の事前経路）。byte-matched mode の
      「argv mismatch → route 不発 → 原 command が直接実行
      される」挙動を維持する。
  (b) 追加 set メンバー — 存在 + `is_route_eligible` +
      pin 解決のみを route 解決時に行う。argv 照合は job
      実行時の `check_exec_execution` へ遅延させる
      （direct `workflow run --grant` と同じ検査位置）。
  (c) set メンバーのいずれかが失敗した場合の分類は
      `Viability::NoGrant` 一種に統一すること（fail-safe —
      宣言された委譲面が壊れている route は発火させず、
      原 command 直接実行へ倒す）。viability enum・
      優先順位・`route resolve` 出力 shape（R11）は変更しない。
- R4. route run 時の policy 投影を `{route.grant} ∪ grants ∪
  {fixed grants}` に拡張すること。投影判定は現行
  `project_grant` と同じ位置（viability 通過後・workflow
  context 構築前）で行い、全 set 名に `is_route_eligible` を
  適用し、1 件でも ineligible なら `denied("no-grant")` で
  fail すること。routed workflow の tool scope / command tool
  name 解決は投影後 policy のみを見るため、exec 委譲面は
  宣言 set の argv union + 既存 fixed grant に閉じること。
- R5. viability の unsafe-command 分岐から `argv_shell_re`
  （metachar 走査）を除去すること。`input.unsafe_command`
  （caller 付与 flag）と `CWD_CHANGING_COMMANDS` による
  unsafe-command 分類・`Viability::StdinUnsupported` は維持する。
  crates 内の唯一の consumer が外れた時点で `argv_shell_re`
  定義自体を `policy/exec.rs` から削除し、#163 で「grant 側は
  metachar を拒否する」と書かれた経緯の stale comment を
  正すこと。
- R6. route usage record に `executed` field を追加すること:
  - `executed = {grant: <name>, argv: [<args>]}` —
    captured exec result の `details.grant`/`details.argv` 由来の
    実測値。byte-matched mode では `grant = route.grant`・
    `argv = 原 exec_args` と一致する。
  - `command`/`commandHash` field は現行のまま維持する
    （`payload.command` の hash — 原 command の identity。
    env-prefix 値を含み得るため hash のまま）。
  - `executed` は record が書かれる全経路で付与する
    （record timing は現行どおり captured exec result 必須）。
  - schemaVersion は `1` のまま — additive optional field とし、
    `usage` 集計・既存 reader への互換影響を持たないこと。
- R7. anchor 規約を維持・明文化すること:
  - `expected_input = {"name": route.grant, "args": exec_args}`
    を変更しない（`job_id == "exec"` && `tool == "command"`
    の job への byte match gate も現行どおり）。
  - routed `exec` job の lifecycle bounds（timeout/killGrace/
    exitCodes）は `route.grant` の宣言値に従う。
  - set メンバーを呼ぶ追加 job は `input.name` に自身の
    grant 名を指定し、その grant の宣言 lifecycle・
    `cwd`/`env`/`exitCodes`・executable pin が適用される。
    member job には routed-exec context（route の
    `passthroughEnv` capture・`exec_cwd` override・
    `accept_any_exit_code`・route log tee）は適用されず、
    `route_context: None` の通常 `command` job として実行
    される（`workflow_run.rs:467-470,521` — context は
    `job_id == "exec" && tool == "command"` のみに付く）。
  - member を呼ぶ `command` job は `job_id != "exec"` で宣言
    すること — `job_id == "exec" && tool == "command"` の
    job は expected_input byte-match gate が
    `{name: route.grant, args: exec_args}` を要求するため、
    member 名を指定すると `exec-input-mismatch` で失敗する
    （workflow authoring 規約）。
  - route usage record の attribution anchor は `route` +
    `route.grant`（`executed.grant` は実測 echo）のまま。
- R8. 次の不変条件を保持すること:
  - **routing ≠ authorization**（D2）— viability は
    `check_exec_execution` への委譲と `is_route_eligible`
    判定のみを行い、route 層に内容審査を持たない。
  - hook 経路の fail-open — route/target 読み込みは
    `load_or_none` で失敗時は route arm 不活性のまま通常
    tool path へ落ちる。
  - `execDenyPatterns` の適用点は set メンバーにも一律に
    効く（pin 適用点 + scope pin で deny される member は
    ineligible → `NoGrant`）。
  - exec-input byte match・exit mirror・routed-exec
    convention（`outputs.execResult` 全 binding + unconditional
    `exec` job）の検証は変更しない。

## Acceptance Criteria

- [ ] `grants: [a, b]` を宣言した route で発火した routed
      workflow 内の command job が `a`・`b` を `input.name` に
      指定して実行でき、未宣言名を指定した job は policy
      解決失敗で拒否される
- [ ] project 層 routes.yaml の route entry が `grants` key を
      含むと fail-loud route-load error で拒否され（該当 file
      全体が load 失敗）、builtin/user 層の `grants` 宣言は
      受理される。project 層の `route.grant` 単数宣言は
      従来どおり有効
- [ ] `route.grant` の argv pattern と hook payload argv が
      不一致のとき viability が `no-grant` となり route が
      発火せず原 command が直接実行される（byte-matched
      挙動の回帰無し）
- [ ] `grants` のメンバーが未存在・project-scope・fixed の
      いずれかの場合、viability が `no-grant` となり route が
      発火しない（新規 viability 値を追加しない）
- [ ] `grants` メンバーの argv pattern に一致しない argv を
      指定した job が job 実行時の `check_exec_execution` で
      拒否される（route 解決時には発火を止めない）
- [ ] quoted metachar を含む argv（例: `git log -G 'a|b'`）で
      route が `unsafe-command` と誤分類されず `runnable`
      となる（`argv_shell_re` 依存の除去）
- [ ] `argv_shell_re` 定義と import が削除され、
      `grep argv_shell_re` が crates 内に残らない
- [ ] route usage record が `executed:{grant, argv}` を持ち、
      `command`/`commandHash` が不変で、schemaVersion が
      `1` のまま、`cheaper usage` 集計が従来どおり動作する
- [ ] `expected_input` の `name` が `route.grant` のまま
      `exec-input-mismatch` gate を通り、追加 job が自身の
      grant の lifecycle bounds で実行される
- [ ] `load_or_none` 経路・`unsafe-command`（caller flag /
      cwd-changing）・`stdin-unsupported` の既存分類が回帰
      していない（既存 route/hook テスト群 pass）

## Deferred Design Concerns

- substitute mode での `expected_input`/`executed` anchor 切替 |
  issue #167 の設計範囲 | substitute.grant 導入時に本 spec の
  R6/R7 anchor 規約を再評価する | 先回りして substitute 前提の
  field を入れると byte-matched mode の意味論が曖昧になる
- `route resolve` 出力への `grants` echo | 診断用途はあるが
  R11 出力 shape を広げる確立した需要が無い | 失敗 member の
  特定手段が必要になった時点で検討 | viability が no-grant の
  理由が「route.grant 失敗 vs member 失敗」で区別できない
- captured 前の exec job 失敗経路の record 欠落 | 現行仕様の
  まま据え置き（route record は captured result 必須）|
  route 観測性の別 issue で扱う | exec 不発 route の集計盲區が
  残る

## Sources and Relations

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/166
- 上流 discovery: `../discovery/route-grant-set-projection-issue-166.md`
  （実測 inventory・選択肢比較・open questions）
- 親 spec/design: `../specs/permission-model-responsibility-redesign.md`・
  `../designs/permission-model-responsibility-redesign.md`
  （R6「viability は authorization を呼ぶが内容審査を呼ばない」・
  issue 4 分解 — grant-set 投影・viability 整理・record split）
- 前提 IR: `../impl/ir/unified-exec-grant-model-issue-165.md`
  （`is_route_eligible` seam・`project_grant` の `{route.grant} ∪
  {fixed}` 挙動・将来 `grants[]` 拡張の明示）、
  `../impl/ir/exec-contract-content-audit-removal-issue-163.md`
  （`argv_shell_re` 残存は route viability のみ・issue 4 整理対象）
- 判定 ADR: `../adr/route-grant-set-projection.md`
- refine 対象 spec: `./pretooluse-route-resolver-and-workflow-delegation.md`
  （grant semantics・record 契約・routed-exec convention の
  更新先）

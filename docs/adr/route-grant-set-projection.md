---
id: "ADR-034KTMnPFDF25x710LFSKY"
type: "adr"
status: "proposed"
title: "route grant-set projection — grants 宣言・viability 条件緩和・record split"
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
  depends-on:
    - "./unified-exec-grant-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/route-grant-set-projection.md"
    - "./defer-non-exec-routing-to-permission-model-redesign.md"
    - "./enforce-declared-contracts-over-content-audit.md"
  refines:
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
  refined-by: []
  derives-from:
    - "../discovery/route-grant-set-projection-issue-166.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/unified-exec-grant-model.md"
    - "../specs/unified-exec-grant-model.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# route grant-set projection — grants 宣言・viability 条件緩和・record split

## Context and Problem Statement

route/hook 経路には #163 で exec/policy 層から除去された
内容審査（content audit）の残存経路がある: route viability の
unsafe-command 分岐が `argv_shell_re`（NUL・CR・LF・shell
metachar・`$(` の走査）で argv を検査している（実測
`crates/cheaper-cli/src/route/resolve.rs:287-291`、定義
`crates/cheaper-core/src/policy/exec.rs:144-150`）。#163 実施後、
grant 側の authorization（`check_exec_execution` → `authorize_args`）
は metachar を拒否しないため、viability が authorization より
**厳しい**状態が残っており、quoted metachar を含む正当な argv
（`git log -G 'a|b'` 等）で route が `unsafe-command` と誤分類
されて不発する偽陰性がある（routing ≠ authorization、D2 違反
状態）。

同時に #165 の unified `ExecGrant` 移行で `project_grant` は
`{route.grant} ∪ {fixed grants}` しか残さず（実測
`crates/cheaper-core/src/policy/exec.rs:123-130`）、routed
workflow 内の追加 command job が `route.grant` 以外を
`input.name` に指定できない。route 1 件が委譲できる exec
面を宣言された set へ拡張し、かつ confused-deputy 半径を
宣言された grant の argv union に閉じる必要がある。

最後に、route usage record は現行 `commandHash`（原 command
文字列のハッシュ）のみを保持し、「要求された command」と
「実際に実行された exec」の attribution が同一 field に潰れて
いる。将来の substitute mode（#167）で実 exec が原 command
から分岐する前に、record を `{command, executed}` へ分離する
必要がある。

未決の主要論点: set 投影の宣言形状・eligibility 判定・viability
緩和の境界・失敗時分類・record field 形状・`argv_shell_re`
の扱い。

## Decision Drivers

- **D2 routing ≠ authorization**（親 design 不変条件）—
  viability は authorization を呼ぶが内容審査を呼ばない構造
  （親 spec R6/AC「routing viability が grant/allowlist 検査を
  呼ぶが内容審査を呼ばない」）。
- **byte-matched firing の保持** — 現行挙動では argv mismatch
  → viability `no-grant` → route 不発 → 原 command が直接実行
  される。`route.grant` の argv 事前検査を無条件に弱めると
  「route 発火 → routed exec 失敗 → 原 command が実行されない」
  回帰になる。
- **confused-deputy 半径の宣言境界** — route から見える exec
  面は route 宣言が唯一の出所とし、workflow 宣言の grant を
  混ぜない。
- **fail-safe fold** — 宣言された委譲面が壊れている route
  （member 未存在・ineligible）は発火させず原 command 直接
  実行へ倒す（fail-open は load 経路、fail-closed は exec 面の
  既存二層を維持）。
- **単一 seam の再利用** — `is_route_eligible`・
  `check_exec_execution`・`pin_exec_grant` の既存 seam を
  再編せず、第二の eligibility/authorization 規則を作らない。
- **record additive 互換** — `usage.jsonl` は additive-only 規約
  （`docs/guides/usage.md`）で、schemaVersion bump・consumer
  変更を伴わない拡張とする。
- **KISS/YAGNI** — substitute mode(#167)・非 exec workflow
  target は対象外とし、現行 byte-matched mode に必要な最小
  構造のみを決める。

## Considered Options

### O1 set 投影の宣言形状

- **O1a `grant`（required）+ `grants?: [name, …]`（追加 field）**
  — 有効 set = `{route.grant} ∪ grants`。`grant` は発火 gate・
  exec anchor・record attribution anchor として不変、`grants`
  は追加委譲面のみを担う。
- **O1b `grants: [name, …]` 単一 field へ置換（`route.grant` も
  member 必須）** — set を 1 field で表現し anchor を
  `grants[0]` または別 anchor field で指定。宣言重複を
  schema で防げるが、既存 routes.yaml 全件の移行と、
  「発火 grant」と「委譲先」の意味差を 1 配列内で表す
  規約（位置依存または anchor field 追加）が必要。
- **O1c `grant` field を string-or-array の union 型へ拡張** —
  `grant: cargo-test` / `grant: [a, b]`。field 数は増えないが
  型が二値化し、anchor（先頭?全員?）の意味規約を別途定義
  する必要があり、schema 検証・serde 表現・文書の全てで
  union 説明が要る。

### O2 viability 緩和の境界

- **O2a `route.grant` は完全検査・追加 member は存在 +
  eligibility + pin のみ** — `route.grant` は現行経路
  （`pin_exec_grant` + `ExecToolOptions` 構築 +
  `check_exec_execution(exec_args)`）を維持し byte-matched
  firing を保持。追加 member は executable が pin 解決
  できることだけ確認し、argv 照合は job 実行時
  （`command::execute` 内の `check_exec_execution`）へ
  遅延する — direct `workflow run --grant` と同一の検査位置。
- **O2b 全 member に完全検査（argv 照合込み）** — member
  も `route.grant` と同じ `check_exec_execution` を掛ける。
  しかし member を呼ぶ job の argv は route 解決時点では
  未確定（workflow 内で構築される）ため、照合対象 argv が
  存在せず、route.grant 用 argv（原 command 由来）に対して
  無関係な member の pattern を試すことになる — 意味的に
  誤った検査であり実質採用不能。
- **O2c member は何も検査しない** — route.grant だけ検査し
  発火させ、member の欠陥は job 実行時の解決失敗に任せる。
  最も緩いが、宣言の typo・policy 削除で「route 発火 →
  routed exec 失敗 → 原 command 不実行」という O2a が防ぐ
  回帰を許してしまい、fail-safe fold に反する。

### O3 set member 失敗時の分類

- **O3a `Viability::NoGrant` へ統一 fold** — member の
  未存在・project-scope・fixed・pin/deny 失敗は全て
  `no-grant` とし、route は不発、原 command が直接実行される。
  viability enum・優先順位・`route resolve` 出力 shape に変更なし。
- **O3b 新規 viability 値（例 `IneligibleSetMember`）** —
  失敗理由を区別できるが、enum 追加は wire 契約変更であり、
  hook 側の挙動はどちらも「route 不発」で同一。診断情報は
  `route resolve --json` の diagnostics 経路で補えば足り、
  新規 enum 値の正当化には至らない。
- **O3c `InvalidRoute` へ fold** — route 宣言自体の欠陥と
  みなす案。`InvalidRoute` は現行「route 宣言・target 解決の
  malformed」専用の分類であり、grant 解決失敗を混ぜると
  「policy 側の状態」なのに「route が壊れている」と報告する
  誤 attribution になる。

### O4 set member の eligibility 判定

- **O4a `is_route_eligible`（`Trusted && !fixed`）を一律適用**
  — route.grant と同一 seam。project-scope grant は route が
  参照しない（#165 O7/M3）、fixed grant は無条件で投影済み
  のため member 宣言は ineligible 扱いで失敗として fold。
- **O4b fixed grant も member に許す（vacuous member）** —
  fixed は既に投影されるため宣言しても無害だが、author の
  「これを書かないと使えない」という誤認を助長し、fail-loud
  schema 規約上「書いても何もしない宣言」を容認するのは
  不親切。未存在扱いで no-grant となる方が早期検出になる。
- **O4c project-scope grant も許す** — project policy が自身を
  route 委譲面へ入れられると、project layer が trusted layer の
  審査を迂回して exec 面を広げられる。D2 の trust boundary
  を侵すため不採用。

### O5 `argv_shell_re` の扱い

- **O5a 分岐と定義を両方除去** — unsafe-command 分岐から
  metachar 走査を外し（`input.unsafe_command` flag・
  `CWD_CHANGING_COMMANDS`・`StdinUnsupported` は維持）、
  定義本体も `policy/exec.rs` から削除。crates 内 consumer は
  route viability のみ（`grep` 実測）。
- **O5b `[\0\r\n]` のみ残す縮小版** — 「exec 不可能な byte」
  だけ残す案。しかし NUL/CR/LF は tokenize 段階
  （`tokenize_command`）で既に拒否され viability へ到達
  しないため、縮小版は到達不能コードを残すことになる。
- **O5c 現行維持** — metachar 走査を残す。authorization と
  viability が diverge したまま（quoted metachar の偽陰性が
  残り、#163 の「内容審査を route に残さない」意図に反する）。

### O6 usage record の field 形状・attribution anchor

- **O6a `command`/`commandHash` 据え置き + `executed: {grant,
  argv}` 追加** — `commandHash`（`payload.command` の hash）は
  そのまま「要求された command」の identity として維持し、
  `executed` に実測 `{grant, argv}`（captured exec result の
  `details.grant`/`details.argv`）を載せる。schemaVersion 1 の
  additive field。byte-matched mode では `executed.grant =
  route.grant`、`executed.argv = 原 exec_args`。
- **O6b `executed` を argv 文字列（join）で記録** —
  `executed: "git log -G 'a|b'"`。record が小さいが
  round-trip 不能（quote 復元・空白含み arg の境界喪失）で、
  `{name, args}` 構造の exec 契約と表現 drift する。
- **O6c `executed` を `{grant, argvHash}` で記録** —
  `commandHash` と対称に hash 化する案。`command` が hash
  なのは env-prefix 値（`FOO=secret cmd`）を含み得るためだが、
  exec argv は env-prefix を持てず（prepare が拒否）、
  `route-logs/` が同じ `<userConfigDir>` に raw 出力を保存
  している現状では hash 化の保護意味が薄く、観測性だけが
  失われる。
- **O6d `command` を raw 文字列へ変更** — env-prefix 値を
  record へ書くことになり、`usage_log` 既存テストが明示的に
  防いでいる secret 混入経路を復活させる。不採用。

## Decision Outcome

採用: O1a + O2a + O3a + O4a + O5a + O6a。

- routes.yaml に `grants?: [name, …]` を optional 追加し、
  `grant` は required のまま発火 gate / exec anchor /
  attribution anchor として据え置く。有効 set は
  `{route.grant} ∪ grants`。
- viability は `route.grant` のみ byte-matched 完全検査
  （`check_exec_execution` 経由）を維持し、追加 member は
  存在 + `is_route_eligible` + `pin_exec_grant` のみ確認。
  member の argv 照合は job 実行時へ遅延する。
- member 失敗は `Viability::NoGrant` に fold。enum・出力
  shape・hook 側挙動は不変。
- eligibility は `is_route_eligible` 一律（trusted-scope・
  `!fixed` のみ。fixed は vacuous・project-scope は trust
  boundary 侵害のため member 不可）。
- `argv_shell_re` 分岐と定義を除去（`input.unsafe_command`・
  `CWD_CHANGING_COMMANDS`・`StdinUnsupported` 維持）。
- route record は `command`/`commandHash` 据え置き +
  `executed:{grant, argv}` additive 追加（schemaVersion 1 維持、
  `route` フィールドの attribution anchor 不変）。

### Consequences

* Good, because: viability が「authorization への委譲」だけに
  なり、#163 が残した内容審査経路が route 層からも消えて
  D2（routing ≠ authorization）が回復する。quoted metachar
  argv の route 不発という偽陰性が解消される。
* Good, because: routed workflow が宣言された補助 executable
  を呼べるようになり、exec 委譲面は依然として「宣言 set の
  argv union + 既存 fixed grant」に閉じる（confused-deputy
  半径は宣言境界のまま拡大しない）。
* Good, because: `route.grant` の byte-matched firing が保持
  され、argv mismatch 時に原 command が直接実行される
  既存挙動に回帰がない。member 失敗も `no-grant` fold で
  原 command 直接実行へ倒れる fail-safe を保つ。
* Good, because: `executed` が実測 `{grant, argv}` を記録
  するため、将来の substitute mode で「原 command と実
  exec の分岐」を既存 record shape で表現できる。
* Bad, because: `grants` 宣言数だけ route 解決時の pin
  解決コストが増える（PATH 走査 × member 数 — 宣言数に
  上限を設けないため operator 責任）。member が増えるほど
  「どの member が no-grant の原因か」の診断が難しくなる
  （`route resolve` の diagnostics 改善は後続）。
* Bad, because: routes.yaml schema・`Route` struct・
  fixture・`project_grant` 呼び出し側・viability 分岐・
  usage record builder・関連 test を一括で触る必要があり、
  route 周辺の変更面が広い（破壊的 schema 変更ではないが）。
* Bad, because: `executed` の追加で record 行が僅かに大きく
  なり、旧 reader で `executed` を読めない（additive なので
  破壊ではないが、新 field の情報は旧 tool では観測不能）。

### Confirmation

- spec の受入条件（`docs/specs/route-grant-set-projection.md`
  の Acceptance Criteria）が実装 AC。
- discovery inventory（`docs/discovery/route-grant-set-projection-issue-166.md`
  の file:line 表）が viability・schema・record の全接点を
  網羅していること。

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/route/{routes,resolve,
  run}.rs`（`Route` struct・`KNOWN_KEYS`/`validate_route`・
  unsafe-command 分岐・viability member 検査・
  `project_grant` 呼出・record 呼出）、
  `crates/cheaper-core/src/policy/exec.rs`（`project_grant` set
  版一般化・`argv_shell_re` 定義削除）、
  `crates/cheaper-cli/src/usage_log.rs`（route record builder の
  `executed` field）、`crates/cheaper-cli/resources/routes.yaml`
  （shape 記述）、route/usage 関連 integration test・fixture。
* Patterns to follow: `is_route_eligible` の単一 seam 再利用、
  `project_grant` の投影位置（viability 通過後・workflow
  context 構築前）、`check_exec_execution` の shared
  predicate seam、`validate_route` の fail-loud schema 規約、
  `command`/`commandHash` の hash 記録規約。
* Patterns to avoid: viability enum 値の追加・route 層への
  内容審査 regex の再導入・member の argv 事前照合・
  `grant` field の union 型化・workflow 宣言の grant 受理・
  record schemaVersion bump。
* Dependencies or configuration: 新規 dep なし。#165 の
  `ResolvedExecPolicy`/`is_route_eligible`/`project_grant`・
  `pin_exec_grant`・`check_exec_execution` を前提。#167
  （substitute mode）の実装は本 ADR 適用後。

## Verification

* [ ] `grants` 宣言の route が `{route.grant} ∪ grants ∪
      {fixed}` を投影し、未宣言 grant 名を `input.name` に
      指定した job が policy 解決失敗で拒否される
* [ ] `route.grant` の argv mismatch で viability `no-grant` →
      route 不発 → 原 command 直接実行（byte-matched 回帰無し）
* [ ] member 未存在 / project-scope / fixed で viability
      `no-grant` へ fold（新規 enum 値なし）
* [ ] member argv mismatch は route 解決時に止めず job
      実行時に拒否（検査位置が direct `workflow run --grant`
      と同一）
* [ ] quoted metachar argv で route が `runnable`（unsafe-
      command 偽陰性の解消）・`argv_shell_re` 削除済み
* [ ] `input.unsafe_command`・`CWD_CHANGING_COMMANDS`・
      `stdin-unsupported` の分類が維持
* [ ] route record に `executed:{grant, argv}` が載り、
      `command`/`commandHash`/schemaVersion 不変・
      `cheaper usage` 集計が回帰無し
* [ ] `expected_input.name = route.grant` で `exec-input-
      mismatch` gate が通り、追加 job が自身 grant の
      lifecycle bounds で実行される

## Pros and Cons of the Options

### O1a `grant` + `grants` 追加 field（採用）

* Good, because `grant` の発火 gate / anchor 意味を変えず
  後方互換（`grants` 無しの既存 routes.yaml は全件有効）で、
  「発火」と「委譲先」の意味差が field 名に載る。
* Bad, because field が 2 つに増え、`grant` が `grants` に
  含まれるかどうかの重複規約（本 ADR では冪等 dedupe）を
  明記する必要がある。

### O1b `grants` 単一 field 置換

* Good, because set を 1 か所で宣言でき anchor を除けば
  宣言が簡潔。
* Bad, because 全既存 routes.yaml の破壊的変更になり、
  anchor 表現（位置依存 or 別 field）の新規規約が要る。

### O1c `grant` union 型

* Good, because field 数が増えない。
* Bad, because string-or-array の union は schema・serde・
  文書の全てで説明コストが高く、anchor 意味規約が依然必要。

### O2a anchor 完全検査 + member は存在/pin のみ（採用）

* Good, because byte-matched firing を保持しつつ、job-supplied
  argv の検査を実行時の正当な位置（`check_exec_execution`）
  に置ける。member 欠陥時に原 command 直接実行へ倒れる。
* Bad, because member の argv が実際に合うかは job 実行まで
  分からず、route 解決時点での「使える保証」は executable
  の存在までに限定される（direct `workflow run --grant` と
  同等の保証レベル）。

### O2b 全 member 完全検査

* Good, because 見た目上は最も厳格。
* Bad, because member を呼ぶ job の argv が route 解決時点で
  未確定であり、照合対象が無い — 検査として成立しない。

### O2c member 無検査

* Good, because 実装が最小。
* Bad, because 宣言 typo・policy 削除で「route 発火 →
  routed exec 失敗 → 原 command 不実行」の回帰を許す。

### O3a `NoGrant` fold（採用）

* Good, because wire 契約・enum・hook 挙動が不変で、
  fail-safe（route 不発 → 原 command 実行）に倒れる。
* Bad, because 「route.grant 失敗」と「member 失敗」の
  区別が viability 値だけでは付かない。

### O3b 新規 viability 値

* Good, because 失敗理由を構造的に表現できる。
* Bad, because enum 追加は wire 契約変更で、hook 側挙動は
  どちらも「不発」であり診断改善以外の利益が無い。

### O3c `InvalidRoute` fold

* Good, because 「宣言が壊れている」印象を与える。
* Bad, because grant 解決失敗は policy 状態の問題であり、
  route 宣言 malformed と混同すると誤診を誘導する。

### O4a `is_route_eligible` 一律（採用）

* Good, because route.grant と同一 seam で第二規則を生まず、
  trust boundary が #165 と一致する。
* Bad, because fixed grant を書くと fail になる（author が
  「書けば使える」と誤認した場合に fail-loud になるが、
  これは早期検出として意図的）。

### O4b fixed 許可（vacuous member）

* Good, because 「既に投影されている fixed」を書いても
  失敗しない。
* Bad, because 無意味な宣言を容認し fail-loud schema 規約に
  反する。

### O4c project-scope 許可

* Bad, because project layer が trusted layer の審査を迂回
  して route 委譲面を広げられ、D2 の trust boundary を侵す。

### O5a `argv_shell_re` 分岐+定義除去（採用）

* Good, because 内容審査の route 層残存が消え、
  authorization との diverge（偽陰性）が解消される。
* Bad, because `route resolve` の unsafe-command 分類が
  現行より狭くなる（metachar を含む argv で `runnable` と
  報告されるようになる — ただし authorization は別途 job
  実行時に効く）。

### O5b `[\0\r\n]` 縮小版

* Good, because 定義を小さく残せる。
* Bad, because tokenize 段階で既に拒否される到達不能コード
  を残すことになる。

### O5c 現行維持

* Good, because 変更なし。
* Bad, because D2 違反と偽陰性が残る（本 issue の動機）。

### O6a `command` 据え置き + `executed:{grant,argv}`（採用）

* Good, because 「要求」と「実測」の attribution が分離し、
  schemaVersion 1 の additive で将来の substitute mode に
  そのまま乗る。
* Bad, because record 行が大きくなり、`executed` を読めるのは
  新 reader のみ。

### O6b `executed` 文字列 join

* Good, because 最小の表現。
* Bad, because round-trip 不能・exec 契約 `{name,args}` と
  表現 drift。

### O6c `executed` を `{grant, argvHash}`

* Good, because `commandHash` と対称で record が小さい。
* Bad, because exec argv は env-prefix を持てず hash の
  保護意味が薄く、`route-logs/` に raw が既にある現状では
  観測性だけ失う。

### O6d `command` raw 文字列化

* Bad, because env-prefix 値の secret 混入経路を復活させ、
  既存 `usage_log` テストの防衛対象と衝突する。

## More Information

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/166
- 上流 discovery: `../discovery/route-grant-set-projection-issue-166.md`
- spec: `../specs/route-grant-set-projection.md`
- 前提 ADR: `./unified-exec-grant-model.md`（O7 route
  eligibility・O8 `project_grant` 投影の `{route.grant} ∪
  {fixed}`）、`./enforce-declared-contracts-over-content-audit.md`
  （enforce-declared-contracts 原則）、
  `./defer-non-exec-routing-to-permission-model-redesign.md`
  （非 exec routing sketch の deferred 判断 — 本 ADR はその
  受け入れ判断の一部）
- substitute mode（#167）は本 ADR の `executed` field が
  前提となるが、substitute.grant 自体は本決定に含めない。

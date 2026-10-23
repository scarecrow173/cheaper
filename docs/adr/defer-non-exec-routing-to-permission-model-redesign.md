---
id: "ADR-034KtTzOJGqmzmBW3SwGp5"
type: "adr"
status: "accepted"
title: "routed-exec convention を唯一の routing mode とし non-exec routing は権限モデル再設計 (#151) へ defer する"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/160"
    - "https://github.com/scarecrow173/cheaper/issues/151"
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
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "./command-grant-trust-boundary-and-managed-process-reuse.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "./per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "./invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "./enforce-declared-contracts-over-content-audit.md"
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "../designs/documentation-honesty-sweep-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/non-exec-routing-design-space.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# routed-exec convention を唯一の routing mode とし non-exec routing は権限モデル再設計 (#151) へ defer する

## Context and Problem Statement

issue #160(#147 からの split)は、routing が**原 command を実行しない**
workflow(non-exec routing — 例: `cargo build` を lint-only workflow へ
route)をサポートすべきかを問う。現行実装は routed-exec convention を
構造的に強制する:

- `validate_routed_convention`(`workflow_run.rs:897`)が `exec` job
  (`kind:tool, tool:command, when` 無し)と `outputs.execResult:
  {$from:exec}` の whole binding を必須とする。
- `exec-input-mismatch`(`workflow_run.rs:465-482`)が exec job の input
  を `{name:<route.grant>, args:<原 argv>}` の byte-exact 一致に固定
  する。
- `route run` は resolved policy の `command.grants` を route grant
  1 件へ投影する(`route/run.rs:121` → `command.rs:74` C9 —
  project 層 workflow が他の user grant を呼ぶ confused deputy を防ぐ)。
- `route run` の exit code は `execResult.details.exitCode` の mirror
  (D11/R15)。

これらは全て承認済みの設計決定(D11/R15 exit mirror・C9
confused-deputy・D2 routing≠authorization)の帰結である。問題は、
non-exec routing を認める場合に (a) execResult 無しの exit-code
mapping contract、(b) grant argv-allowlist の audit 前提(現行は原
command に対して照合)の再設計、(c) routing≠authorization 境界との整合
(#151 と interlock し得る)の 3 点をどう扱うかである。issue は明示的な
exit 条件として「現行 convention(exec 必須 + 任意の追加 job/worker は
自由)で十分なら close してよい」を定めている。

## Decision Drivers

* **routing≠authorization(D2)の維持**: route は `command.grants` の
  既存名を参照するだけで grant を生成しない。実行される全 command が
  trusted-layer grant の argv pattern に照合される構造を崩さない。
* **confused-deputy 半径の最小化(C9)**: single-grant projection
  (`project_grant`、`command.rs:74-80`)が束縛するのは `command`
  tool surface のみ — `process` policy は無投影のまま serialize
  され(`workflow_run.rs:244-248`)、`process` section は
  `policy_project_partial_schema` が project 層へ許すため、
  project 層の routed workflow は今日でも project/user 宣言の任意
  `process` task と worker job(read/grep/find/ls + 任意 prompt)を
  呼び出せる。つまり exec 必須 + grant 必須が真に保護するのは
  deputy の完全封じ込めではなく、(a) firing gate(原 argv が grant
  認可済みのときのみ route が発火)、(b) 原 command の実実行保証、
  (c) exit-code の真正性、(d) usage record の attribution の 4 点
  である。workflow 自身に grant authoring 力を与える形は取らない
  方針は維持する。
* **exit-code contract の semantic 保全(D11/R15)**: 親 agent は
  rewrite 済み command の exit code を原 command の exit として解釈
  する。parent に見える output body は今日でも workflow 宣言の
  `result` output が供給し得る(`finish()` は clean 時に
  `outcome.result.result` を優先返却 — `route/run.rs:527-537`)
  ため、実測が保証される signal は exit-code mirror のみである。
  Option E の実質的 delta は exit code を measured から
  author-declared へ移す点に narrowing されるが、親 agent が exit
  code で成否を gate する以上これは依然 real hole であり、mirror
  対象が workflow 内部状態へ変わるとこの前提が揺らぐ。
* **audit attribution**: `kind:"route"` usage record は route × 原
  command × grant の attribution を前提にする。「実行されなかった
  command」を `command` として記録する不整合を許容しない。
* **二重設計の回避**: 認可・audit contract は issue #151(権限モデル
  再設計)の構成要素である。standalone に先に設計すると同じ境界を二度
  引き直すことになる。

## Considered Options

* Option A: 現状維持 — routed-exec convention のみを supported mode
  とし、non-exec routing は提供しない(issue #160 を close)。
* Option B: argv-modifying exec — exec job は必須・同一 grant だが、
  workflow が grant argv pattern 内で args を変換可能にする
  (`cargo test` → `cargo test --quiet`)。
* Option C: grant-set routing — route が `grants:[name,…]` を宣言し、
  投影を複数 grant へ拡張。exec 必須は維持しつつ追加 command job が
  declared set 内の他 grant を使用可能にする。
* Option D: substituted exec — exec job は必須だが route 宣言の任意
  grant/argv を実行可能にし、exit mirror は substitute の exitCode を
  mirror する。
* Option E: full non-exec — exec job 不要。pure-analysis/lint-only
  route を許容し、exit code は `outputs.exitCode` 等の workflow 宣言値
  または固定 mapping で供給する。

## Decision Outcome

Chosen option: "Option A — 現状維持"。issue #160 は「現行 convention
で十分」として close する。non-exec routing(Option B–E の spectrum)
は採用しない — 将来需要が確認された場合は issue #151 の権限モデル
再設計の中で再検討する(defer)。

理由:

1. **現行 convention は shipped use case を完全に覆う**。routing の
   product 目的は巨大 output を持つ command の delegation への自動
   置換(原 command を実行・output を capture・condense worker で縮約)
   であり、exec 必須 + 自由な追加 job/worker/when/outputs で表現済み。
   Option B–E が追加で開く用途(argv 変換・別 command・analysis-only)
   は現時点で限定的 — 出力最小化 flag(B)は condense worker が事後
   縮約で目的の大半を達成しており、analysis-only / worker-only の
   workflow は `cheaper workflow run` で既に実行可能(non-exec
   routing が足すのは hook 経由の自動 interception という利便性
   のみ)。真に未充足なのは substituted/deferred exec(`cargo
   build` → `cargo check`・破壊的 command → dry-run 等)と
   author-declared exit code の系統であり、(C–E) の代替実行・非実行は
   semantic substitution(親 agent が「原 command が走った」と解釈
   する契約を破る)の新しい信頼負担を伴う。
2. **non-exec のコスト主体は workflow 側ではなく認可・audit contract
   側**。viability の grant×原 argv 事前検査・single-grant projection・
   usage record の attribution は全て「route grant が原 command を認可
   する」前提に組み込まれており、これらを正しく緩めることは権限モデル
   の再設計(= #151 の領域)である。今 standalone に実装すると
   `project_grant`・route schema・viability・record contract を一度
   変更し、#151 で再度設計し直す二重コストになる。
3. **安全な認可モデルの形は確定済みで、#151 への handoff 材料になる
   (下記 Deferred design sketch)**。却下の方向ではなく「正しい境界で
   設計する時期を #151 に合わせる」判断である。

### Deferred design sketch(#151 で採用する場合の検討入力)

non-exec routing を将来解禁する場合、認可モデルは次の形が安全境界と
なる(discovery の分析を確定スケッチとして記録):

* **grant 投影は route-declared grant *set* へ拡張する** —
  `project_grant` を複数名へ一般化し、route が `grants:[name,…]` を
  宣言した集合だけを policy へ投影する。workflow の `input.name` 参照は
  その set 内に限られるため、confused-deputy 半径は declared grant
  argv union に明示的に閉じる(D2 を維持)。
* **workflow-declared grants は採用しない** — project 層 workflow が
  executable/argv を authoring できる形は trusted-layer-only の grant
  境界(ADR-034EMgaatFhsXjR97kH14c)の迂回であり、D2/C9 の構造保証
  を壊す。grant 名は常に `command.grants` の trusted 宣言への参照で
  あること。
* **viability の事前検査は「存在・pin 解決可能な grant set」へ弱め、
  argv 照合は実行時の command tool に委譲する** — 原 argv への
  `check_command_execution` は「実行されない command」の認可を調べる
  形骸化を避ける。実 argv 照合は direct `workflow run --grant` 経路と
  同じく job 実行時の責務とする。
* **(variant)事前検査を弱めない hybrid 形も併記する** —
  substituted exec(Option D)向けに、route 宣言が substitute
  `{grant, argv}` を明示し、viability の `check_command_execution`
  を原 argv ではなく**宣言された substitute** へ向け直す路線も
  ある。precheck の strict さは維持されたまま、検査対象が「実行
  されない command」から「宣言通り実行する command」へ移るだけ
  であり、上記の弱める路線と排他ではない。
* **exit-code mapping は「実行した command の exit」を mirror する
  (Option D 形)を第一候補とする** — D11 mirror を substitute へ
  読み替えるだけで機構を維持できる。exec を持たない Option E は
  `outputs.exitCode` の workflow 宣言値 mirror か固定 mapping が必要
  で、author による成功偽装を route 層 opt-in で受容する設計判断が
  前提になる。なお Option E でも route の `grant` field は
  authorization-of-intent(firing gate — 原 argv が grant 認可済みの
  ときのみ route が発火する条件)として留保可能であり、無意味化
  はしない。
* **usage record は grant set + 実 exec argv を記録する形へ拡張する** —
  `command`(原 command 文字列)と実際に実行された argv を別 field で
  記録し、「実行されなかった command」を `command` attribution として
  記録しない。

### Scope and Non-Goals

* `validate_routed_convention`・`exec-input-mismatch`・single-grant
  projection・exit-code mirror の現行動作は変更しない(本 ADR は現状
  を確認する記録であり、コード変更を伴わない)。
* Option B–E の spec/design は本 ADR の scope 外 — #151 の権限モデル
  再設計の中で扱う。
* hook verdict の新 mode(rewrite でも Allow でもない応答)や、route
  宣言以外の delegation 形態は対象外。
* non-exec routing は既存の coverage gap を救済しない — command
  deny-list(`command_denied_executable_re`、`command.rs:160-189` の
  find/sed/awk/tar/docker/python*/node/ssh 等)は grant executable
  自体を恒久拒否するため、これら heavy-output command は grant を
  持てず routing mode に関わらず `no-grant` で unroutable のまま。
  grant-env sanitization(`minimal_env` — ambient credential を落とし
  `passthroughEnv`/env-prefix のみ通過)も同様に不変である。

### Consequences

* Good, because routing≠authorization・confused-deputy 半径・exit-code
  mirror・audit attribution の承認済み不変条件がそのまま維持され、
  issue #160 は明示的な exit 条件に従って close できる。
* Good, because deferred design sketch が #151 の権限モデル再設計へ
  具体的な入力(route-declared grant set・set 投影・実 exec argv の
  audit・substitute mirror)を供給し、再検討時の出発点を固定する。
* Bad, because argv 変換(B)や補助 command(C)のような小さく安全な
  拡張も #151 まで待つことになり、lint-only route 等の需要が早期に
  確認された場合は本 ADR の revisit が必要になる。

### Confirmation

* `node scripts/check-doc-ids.mjs` は pass する。
* issue #160 は本 ADR を根拠に close する。#151 の検討時に本書の
  Deferred design sketch を入力として参照する。

## Refine 注記（issue #169）

権限モデル再設計（#151 分解 #163–#168）により、本 doc の以下の記述は現行の真実と異なる。本 ADR は決定時点の記録として本文は保持する。

- `:175-209` — Deferred design sketch（grant-set 投影・viability 弱化・substitute retarget・exit mirror・record split の「#151 で採用する場合」の検討入力、採否未確定の体裁）→ 採否は親 design で確定済み: grant-set 投影**採用**（#166 — `project_grant` → route 宣言 `grants:[name,…]`、trusted-scoped grant のみ解決）・viability**条件付き弱化**（#166 — byte-matched mode の argv precheck は維持、set 内非 exec grant は存在 + pin のみ）・usage record `{command, executed}` split**採用**（#166）・substitute-exec mode**採用**（#167 — exit mirror は「実行した command」の exit）・workflow-declared grants**不採用**・Option E exec-less は引き続き**open**。
- `:225-229` — 「command deny-list（`command_denied_executable_re`、`command.rs:160-189`）は grant executable 自体を恒久拒否するため、heavy-output command は routing mode に関わらず `no-grant` で unroutable のまま」→ #163/#164 で falsify 済み。builtin deny-list は除去され deny は opt-in `security.execDenyPatterns`（既定 `[]`）宣言時のみ効くため、既定ではこれらの executable も grant が宣言・pin 可能なら route-eligible である（`no-grant` は deny 該当ではなく grant 非宣言・非 eligible・pin 不能・argv 不一致の viability 結果）。`command_denied_executable_re`・`command.rs:160-189` への参照自体も機構除去済みの stale コード参照。

governing: [ADR-034L6ihibvbjmSnKmSffWH](./documentation-honesty-sweep-and-migration-guide.md)（canonical 文言・訂正方式）・[SPEC-034L6i3XmfCaWRHAiw8TTT](../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md) R5・[migration guide](../guides/migration.md)

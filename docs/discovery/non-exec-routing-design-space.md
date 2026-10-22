---
id: "DISC-034KtQqavnAfVl6CqAVaN3"
type: "discovery"
status: "resolved"
title: "non-exec routing 設計検討 (issue #160)"
created: "2026-10-22"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
  derived-by:
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/payload.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
  defers: []
  deferred-by: []
---

# non-exec routing 設計検討 (issue #160)

## Exploration Goal

issue #147 で導入された routing は routed-exec convention を強制する —
routed workflow は `exec` job(`kind:tool, tool:command, when 無し`)を
必須とし、その `execResult.details.exitCode` を `route run` の exit code
へ mirror する。issue #160 はこの convention を緩めて、**原 command を
実行しない workflow**(non-exec routing、例: `cargo build` を lint-only
workflow へ route)をサポートすべきかを決める。issue 自身が明記する
exit 条件:「現行 convention の範囲(exec 必須 + 任意の追加
job/worker は自由)で十分なら本 issue は close してよい」。

## Key Issues and Assumptions

- 現行 convention が routed workflow に許す範囲の正確な把握 —
  「exec 必須」以外に何が固定され、何が自由か。
- authorization 前提 — viability の `no-grant` 判定は route grant の
  argv-allowlist を**原 argv**に照合する(C9/confused-deputy 対策の一環)。
  workflow が**別の command**を実行するなら、この audit 前提が崩れる。
- exit-code mapping — `route run` の exit code は
  `execResult.details.exitCode` の mirror(D11/R15)。execResult を持た
  ない workflow では mirror 対象が存在しない。
- routing≠authorization 境界(D2)と issue #151(権限モデル再設計)の
  interlock — grant モデル自体が再設計対象なら、non-exec の認可モデルは
  その中で設計するのが自然か。

## 現行 convention の調査結果(何が固定で何が自由か)

コード調査で確認した構造的事实:

- **route 宣言**(`route/routes.rs:307-459`)— `name`・`match.command`
  (argv prefix)・`target:{kind:"workflow",path}`(kind は `"workflow"`
  固定、`:378-386`)・`grant`(**必須 field**、`:399` — route は grant を
  生成せず `command.grants` の既存名を参照)・`output.tokenThreshold`・
  `passthroughEnv`・`trusted`(project 層 override の explicitness 機構)。
- **viability**(`route/resolve.rs:272-341`)— 固定優先順
  `unsafe-command → invalid-route → missing-target → no-grant →
  stdin-unsupported → runnable`。`no-grant` 判定は route grant の pin
  解決 + `check_command_execution(&options, &route.grant,
  &out.exec_args)` で、**原 argv**(`exec_args = argv[1..]`)が grant の
  argv-allowlist に適合することを要求する。つまり rewrite は「user が
  実際に打った command が grant で認可済み」のときのみ発火する。
- **single-grant projection**(`route/run.rs:121` +
  `policy/command.rs:74-81`、design C9)— `route run` は revalidation
  後、resolved policy の `command.grants` を **route grant 1 件だけ**へ
  投影する。routed workflow 内の全 `command` tool job(exec・追加を
  問わず)が `input.name` で参照できる grant は route grant のみ —
  project 層 workflow が他の user grant を勝手に呼ぶ confused deputy を
  構造的に防ぐ。
- **convention 検証**(`workflow_run.rs:897-940`
  `validate_routed_convention`)— `jobs.exec` は
  `{kind:"tool", tool:"command", "when" key 無し}`、`outputs.execResult`
  は `{$from:"exec", path:""}` の whole-job binding を要求。
- **exec-input-mismatch**(`workflow_run.rs:465-482`)— exec job の
  input は `{name:<route.grant>, args:<execArgs>}` と byte-exact 一致が
  必須。**workflow は実行される command の grant・argv を一切変更
  できない**。
- **route context**(`route/run.rs:220-229`)— exec job の command
  tool scope は `extra_env`(env-prefix + allowlist ambient)・
  `exec_cwd`(原 cwd)・`accept_any_exit_code:true`・`log_tee`(全量
  output → `<userConfigDir>/route-logs/`)付きで構築される。
- **exit-code mirror**(`route/run.rs:450-474`)— `route_exec.captured()`
  (exec job の `CanonicalToolResult`)の `details.exitCode` を process
  exit code として mirror。captured 無し = cheaper error(infra 失敗と
  command 失敗の domain 分離、D11/R15)。
- **audit**(`route/run.rs:574-590` `record_route`)— `kind:"route"`
  usage record は `{route, command(原 command 文字列), originalTokens,
  returnedTokens, worker, durationMs, logRef, postprocessFailures,
  exitCode, ok}` を記録し、「route grant が原 command を認可した」という
  attribution を前提にする。

**routed workflow が今日できること**: exec job(完全固定 input)に加え
て、任意の `kind:worker` job(builtin `analyze` の condense worker 等 —
meta の model/usage は `run.rs:553-566` が収穫)、`tools.allow` 内の任意
`kind:tool` job、`when` 条件、`needs` 依存、追加 outputs を自由に持て
る。exec 後の job 失敗は `postprocessFailures` として記録され exit
code を変えない(R16)。追加の `command` tool job(exec 以外の
job_id)は byte-exact input check の対象外 — 同 check は
`ctx.job_id == "exec" && ctx.job.name == "command"` に filter
される(`workflow_run.rs:468`)。したがって non-exec `command` job
は **grant に pin された同一 executable を、grant argv pattern に
適合する任意の argv で実行できる**。制約は「同一 executable の
み・`route_context` 非適用(`extra_env`・`exec_cwd`・`log_tee`・
`accept_any_exit_code` は exec job 専用、`workflow_run.rs:516`)」
であり、実行不能ではない。異なる executable や他 grant を使う
実行は single-grant projection が閉じる。

## Alternatives and Comparison

non-exec は二値ではなく spectrum として整理する:

| Option | 内容 | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| A. 現状維持 | routed-exec convention のみを supported mode とする | 認可・exit mirror・audit の前提が全て成立・実証済み。追加コスト 0。issue の明示的 close 条件を満たす | argv 変換・別 command・analysis-only は構造的に表現不能 | **yes(推奨)** |
| B. argv-modifying exec | exec job は必須・同一 grant だが、workflow が grant argv pattern 内で args を変換可(`cargo test` → `cargo test --quiet`) | grant argv allowlist がそのまま認可境界。exit mirror(D11)は不変。出力最小化 flag 付与という正当用途を開く | viability の原 argv 事前検査と実実行 argv が乖離(TOCTOU 的隙間 — 実実行は command tool が grant に照合するため認可上は安全だが、audit record `command` と実 argv が食い違い record 拡張が要る)。exec-input-mismatch の撤廃は workflow author(project 層可)へ flag 選択権を渡す = 原 command との意味等価性が宣言者責任へ移る | maybe — #151 で再検討 |
| C. grant-set routing | route が `grants:[name,…]` を宣言、投影を複数 grant へ拡張。exec 必須は維持し追加 command job が declared set 内の他 grant を使用可 | 補助 command(失敗時 triage・state probe)を開く。confused-deputy 半径は declared grant argv union に明示限定され、D2(routes は参照、grants は trusted layer が authorize)を維持 | `project_grant` の集合化・route schema 拡張・viability の set 事前検査・audit record の grant set 記録が要る。依然 exec 必須で厳密な non-exec 要求は満たさない | maybe — #151 の grant モデルと一体設計すべき |
| D. substituted exec | exec job は必須だが route 宣言の任意 grant/argv を実行可(例: `cargo build` → `cargo clippy`)。exit mirror は substitute の exitCode を mirror | D11 mirror を「実行した command の exit」に読み替えるだけで mirror 機構はそのまま使える | route grant の原 argv 事前検査が形骸化(原 command は走らない)。親 agent が見る exit/output は substitute の意味論 — semantic substitution は機械検証不能で route author 責任へ移る。usage record が「実行されなかった command」を `command` として記録する不整合 | maybe — #151 で再検討 |
| E. full non-exec | exec job 不要。pure-analysis(例: `git diff` → worker が file 直接解析)・lint-only route | 最も広い表現力 | exit code の自然な source が無い → `outputs.exitCode` の workflow 宣言値 mirror(author による成功偽装を route 層 opt-in で受容するしかない)か固定 mapping が必要。`grant` 必須 schema・viability 前提・usage record の attribution が全て変わる | no — #151 で扱うなら E として |

**workflow-declared grants は全 option で却下**: project 層 workflow が
executable/argv を authoring できる形は trusted-layer-only の grant 境界
(ADR-034EMgaatFhsXjR97kH14c)の迂回であり、D2/C9 の構造保証を壊す。
認可名は常に `command.grants` の trusted 宣言への参照であり続ける必要
がある。

## 認可モデルの評価(option ごとの audit 前提)

現行の audit 前提は「route grant が**原 argv**を事前認可し、workflow は
その argv を byte-exact で実行した」こと。non-exec でこの前提が崩れる
方向は 2 つある:

- **実行される command が変わる**(B/C/D)— 認可の実体は各 command
  job の `input.name` → grant argv pattern 照合(command tool が実行時に
  検査)で担保できるが、**viability 事前検査の意味が変わる**: 原 argv
  への `check_command_execution` は「実行されない command」の認可を
  調べる形骸化した check になる。安全な代替は route-declared grant
  **set** の投影 + set 各 grant の存在・pin 解決の事前検査(実 argv 照合
  は実行時の command tool に委譲 — direct `workflow run` の `--grant`
  経路と同じ責務分割)。audit record は grant set + 実 exec argv を記録
  する形へ拡張が必要。
- **process が一切走らない**(E)— grant 照合対象が無く、`grant` 必須
  field 自体が意味を失う。exit code も合成値になる。routing の目的
  (command の delegation への置換)から外れ、「hook が別応答を返す」
  別 feature として設計する方が自然。

## Issue #151(権限モデル再設計)との interlock

non-exec routing の実装コストの主体は workflow 側ではなく**認可・audit
contract** 側にある: grant-set 宣言・投影・事前検査・usage record の
attribution・exit-code mapping convention。これらは全て #151 が再設計
する権限モデルの構成要素であり、standalone に先に実装すると同じ境界を
二度設計し直すことになる。逆に #151 後なら grant-set 宣言が権限モデル
側の自然な概念として済む可能性が高い。したがって non-exec routing は
#151 の**検討材料として引き渡す**のが正しい境界と判断する。

## Tentative Conclusions and Hypotheses

- **[confirmed]** 現行 convention は「原 command を必ず 1 回、原
  argv・原 cwd で実行しその exit を mirror する」ことを構造的に保証
  している。自由度は exec 周辺の追加 job/worker/条件/outputs にあり、
  command 実行面は route grant + 原 argv に完全固定される(single-grant
  projection + exec-input-mismatch)。
- **[confirmed]** non-exec routing の真の blocker は workflow 表現力で
  はなく authorization・audit contract: viability 事前検査・single-grant
  projection・usage record の全てが「route grant が原 command を認可
  する」前提に組み込まれている。
- **[confirmed]** 安全な認可モデルの形は既に見える: route-declared
  grant set への投影拡張(D2 を維持)+ 実 exec argv の record 記録 +
  exec-required を維持した substitute mirror。workflow-declared grant は
  trusted-layer 境界を壊すため却下。
- **[confirmed]** 現行 convention は shipped use case(巨大 output の
  condense delegation)に十分であり、B–E が追加で開く価値は現時点では
  限定的(出力最小化 flag は condense worker で目的の大半を達成済み)。
- **[推奨]** **Option A: 現行 convention で十分として issue #160 は
  close** し、non-exec routing(B–E の spectrum と認可モデル sketch を
  含む)は issue #151 の権限モデル再設計へ defer する。決定を durable
  に残すため ADR-034KtTzOJGqmzmBW3SwGp5 を作成済み。

## Open Questions

- #151 の権限モデル再設計で grant-set 宣言・per-job grant 参照が自然に
  表現できるか(#151 側の検討入力として本書を引き渡す)。
- argv-modifying exec(Option B)のみ先行解禁する価値 — condense
  worker で output 縮約は実現済みのため優先度は低いと判断。
- pure-analysis route(E)が本当に routing の責務か、hook verdict の別
  mode(rewrite ではなく別応答)の方が自然か — #151 後に revisit。

## Promotion Candidates

- [ ] spec-doc needed: なし — build しない決定であり spec は不要。
- [x] adr-doc needed: 「routed-exec convention を唯一の routing mode
  とし、non-exec routing は権限モデル再設計(#151)へ defer」の決定
  記録 → `../adr/defer-non-exec-routing-to-permission-model-redesign.md`
  (ADR-034KtTzOJGqmzmBW3SwGp5)として作成済み。

## Sources and Relations

- 一次情報: `crates/cheaper-cli/src/route/{routes,resolve,run,payload,
  supply}.rs`、`crates/cheaper-cli/src/workflow_run.rs`、
  `crates/cheaper-core/src/policy/command.rs`、
  `crates/cheaper-cli/resources/workflows/routed-exec.workflow.json`。
- 設計根拠: `../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
  (C9 single-grant projection・exit mirror 節)、
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (R11/R12/R15/R16/R32)、
  `../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  (D2 routing≠authorization・D11 exit-code propagation・D13)。
- issue: #160(本検討)、#151(権限モデル再設計 — defer 先)、#147(親
  epic、routed-exec convention の起源)。

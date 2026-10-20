---
id: "ADR-034J8eZHNPqARi25HLFOL6"
type: "adr"
status: "proposed"
title: "PreToolUse auto-routing via route resolver and workflow delegation"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/142"
    - "https://github.com/scarecrow173/cheaper/issues/143"
    - "https://github.com/scarecrow173/cheaper/issues/144"
    - "https://github.com/scarecrow173/cheaper/issues/145"
    - "https://github.com/scarecrow173/cheaper/issues/146"
    - "https://github.com/scarecrow173/cheaper/issues/147"
    - "https://github.com/scarecrow173/cheaper/issues/148"
    - "https://github.com/scarecrow173/cheaper/issues/149"
    - "https://github.com/scarecrow173/cheaper/issues/150"
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
    - "./command-grant-trust-boundary-and-managed-process-reuse.md"
    - "./dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "./compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
  derived-by:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../specs/shell-read-large-file-blocking.md"
    - "../specs/invocation-scoped-command-grants.md"
    - "../specs/policy-gated-command-tool.md"
    - "../specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "../specs/delegation-usage-measurement-log.md"
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
    - "../specs/run-scoped-artifact-store.md"
    - "../specs/canonical-token-counter.md"
    - "../specs/workflow-file-interface.md"
    - "../specs/workflow-contract-compiler.md"
    - "./conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "./usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "./measured-offload-tool-output-token-estimated-savings-pricing-seam.md"
    - "./keep-artifact-storage-invocation-local-and-bounded.md"
    - "./invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/grant.rs"
    - "crates/cheaper-cli/src/policy.rs"
    - "crates/cheaper-cli/src/embedded.rs"
    - "crates/cheaper-cli/src/run.rs"
    - "crates/cheaper-cli/src/workflow_run.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/proc/mod.rs"
    - "crates/cheaper-tools/src/artifact.rs"
    - "crates/cheaper-workflow/src/engine.rs"
    - "crates/cheaper-workflow/src/schema.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "crates/cheaper-core/src/config/dirs.rs"
    - "crates/cheaper-core/src/fs_util.rs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# PreToolUse auto-routing via route resolver and workflow delegation

## Context and Problem Statement

cheaper の delegation は現状 manual opt-in(親 agent が `cheaper run` /
worker alias を明示呼出)に依存しており、実測で adoption が致命的に低い
(DISC-034J7LwMEdPR5HxFkgOivZ): Devin 238 session で hook 発火は 12
session のみ(<1%)。一方で tool output は frontier context へ 5 週間で
1.43GB 流入し(exec 262MB / edit echo 758MB / read 362MB)、Codex では
>10KB の tool output が全 output bytes の 64% を占める。発火した
session では 13.6M tokens の流入を実際に block しており機構の効果は実証
済み — 足りないのは coverage であり、巨大 output が発生する時点で自動的に
delegation へ倒す PreToolUse rewrite が必須となる。

issue #142 で確定した設計判断(14 項目)と spec(SPEC-034J7SxWvGL3oSbeBpo8a3、
R1-R40)は存在するが、以下は contract ではなく「どう構成するか」の decision
record を必要とする: routed 実行の architecture 形状(orchestrator を新設する
か)、routing 宣言と認可の境界、hook rewrite の verdict 形と target 別 staged
rollout、前提 primitive としての workflow `when` / command `exitCodes` /
route log store + scoped read、route 層解決と project override の trust
mechanism、env passthrough 境界、`kind:"route"` usage record、dedup の
cache 範囲、exit-code 伝播 contract、workflow 起動の process topology、
builtin routes の出荷形態。本 ADR はこれらを D1-D13 として確定し、要件の
受け入れ境界は sibling spec に残す。

## Decision Drivers

* **fail-safe rewrite(deciding criterion)**: routing 層の不備が親 agent の
  command 実行を壊してはならない。rewrite は verdict のみで実行を伴わず、
  成立しない場合は常に passthrough(Allow)に倒す。deny でも silent な exec
  skip でもない。
* **認可は trusted layer にのみ存在する既存境界の継承**: `command.grants`
  は builtin / user / invocation の trusted layer でのみ authoring 可能で
  project layer は schema 層で構造的に排除される
  (ADR-034EMgaatFhsXjR97kH14c)。routing はこの境界を迂回・複製・弱化して
  はならない。
* **reuse over duplication**: workflow engine・command tool・
  `tokenize_shell`・ArtifactReader の positioned read・usage.jsonl の
  best-effort append・dedup の verdict cache を拡張で再利用し、並立する
  第二経路(専用 orchestrator・汎用 file read grant・実行 dedup)を作らない。
* **determinism / 非対話**: hook 評価は bounded で interactive prompt を
  発行できない。`when` は LLM・外部 I/O を介さない純粋評価、trust は
  file-based の明示宣言、rewrite 応答は上流 approval flow を温存する形
  (`updatedInput` のみ)に限定する。
* **coverage が目的**: 発端は manual opt-in の coverage 不足である。安全な
  既定値を維持しつつ、宣言(routes)は広く受け入れ、認可(grants)だけが
  opt-in である構成を取る。
* **secrets hygiene**: routed 実行の全量 output は secret を含みうるため
  workspace 配下に置かず、参照は runId-addressed の scoped 機構に限定する。

## Considered Options

本 ADR は複数の関連決定を含む。D1(architecture 形状)が primary であり、
D2-D13 はその帰結・前提 primitive・個別の mechanism 選択である。各決定の
選択肢は対応する Decision 節と `Pros and Cons of the Options` に記録する。

* Option A: 専用 `kind:exec` orchestrator を新設し、routed 実行を workflow
  経由ではなく専用 executor として実装する。
* Option B: caller-side two-phase orchestration — workflow schema を変更
  せず、hook / `route run` 側が command 実行→閾値判定→worker 呼出の段階を
  自前で制御する。
* Option C: workflow 拡張 — routed 実行を登録済み workflow の起動として
  表現し、`when`・`exitCodes`・scoped log read を workflow/tool 層の汎用
  primitive として追加する。

## Decision Outcome

Chosen option: "Option C — workflow extension", because command tool・engine・
admission・cancel・usage の機構をそのまま再利用でき、routed 実行が要求する
`when`(conditional worker)や `exitCodes`(非 0 exit の result 化)は汎用
primitive として user workflow にも開放できる唯一の形であるため(issue #142
で確定、discovery Option A 採択を確定)。Option A は同じ機構の二重実装に
なり、Option B は orchestration が caller へ分散して user workflow から
表現不能になる(実質的な orchestrator を workflow の外に生やす形)。

以下、個別決定を D2-D13 として確定する。

### D1 — routed 実行は workflow 起動として表現し、専用 orchestrator を持たない

`route run` が起動する routed 実行は `target.path` が指す登録済み
workflow の起動である(command tool による原 argv 実行 + output capture +
token 計測 + `when` 付き conditional worker の構成)。`kind:exec` のような
workflow を介さない executor・専用 orchestrator は設けない。route は
「argv prefix → 登録済み workflow + grant 参照」の宣言であって、実行
semantics を持つ新 tool kind ではない。

### D2 — routing ≠ authorization: routes は宣言のみ、認可は `command.grants`

route の `grant` field は `command.grants` の既存 grant 名への参照であり、
route は grant を生成しない。実行 viability は実 argv に対する grant match
だけが決め、project 層は route を定義できても grant を authoring できない
(trusted-layer-only 境界、ADR-034EMgaatFhsXjR97kH14c をそのまま継承)。
grant が成立しない route match は non-runnable(`no-grant`)であり rewrite
せず従来挙動へ fallback する — project が持ち込んだ route 定義はそれだけ
では実行不能であり、routing 層が新しい認可経路にならないことが構造的に
保証される。builtin routes も auto-granted な grant を伴わずに出荷する
(出荷形態は D13)。

### D3 — `Verdict::Rewrite{command}` + target 別 staging、`updatedInput` のみ

`Verdict`(`routing_policy.rs:116`)に `Rewrite{command}` variant を追加し、
wire 表現は既存 internally-tagged serde 規約(`Allow` →
`{"decision":"allow"}`)に倣う `{"decision":"rewrite","command":"…"}` とする。
Rewrite は command が runnable の場合のみ発行し、render は `updatedInput`
のみを emit して `permissionDecision` を書かない — 上流の approval flow を
温存する(discovery Option E 却下: `permissionDecision:"allow"` 併記は
approval bypass)。

* 先行: **claude + devin**。claude は `hookSpecificOutput.updatedInput`、
  devin は同等の input-rewrite field で exit 0。
* **windsurf は N/A**: rewrite 非対応のため従来どおり Allow fallback。
* **codex / cursor / copilot は issue #150 へ defer**: codex は rewrite に
  `permissionDecision:"allow"` が必須で approval を素通しする、cursor は
  preToolUse event と allow-skips-approval の検証が要る、copilot は応答
  contract 未検証 — いずれも approval-bypass 検証なしに Rewrite verdict を
  発行しない。
* Rewrite が成立しない全てのケース(non-match・non-runnable viability・
  rewrite-safe gate 拒否)は `Allow`(passthrough、原 command 不変更)に
  倒す。

### D4 — workflow `when` は generic deterministic conditional job semantics

`when` は caller 側 two-phase orchestration(Option B 却下 — orchestrator
の外部分散)ではなく、workflow job の optional key として engine が評価する
汎用 conditional job 機構とする。

* predicate は `$input`/`$from` 値への deterministic な純粋評価で、operator
  集合は `eq|ne|gt|lt|exists` に限定する(LLM・外部 I/O を介さない)。
* when-false の job は terminal state `skipped` へ遷移し、dependency
  failure 由来の skip(現行 `DEPENDENCY_FAILED`、`engine.rs:649-658`)とは
  outcome 上で区別可能な diagnostic を持つ。condition skip は
  `dependency_failed` を dependents へ伝播させず、それ単独で workflow を
  failure にしない(`engine.rs:1051-1060` の「非 Succeeded = workflow
  error」集約から condition-skip を除外)。
* condition-skipped job の dependents は自動 skip されず自身の `when` を
  評価する(`when` が無ければ従来どおり実行)。skipped producer への
  `$from` 参照は `null` に materialize し、outputs の `$from` binding が
  skipped producer を指す場合も `null` — output schema が `null` を許容
  しない場合は従来どおり `InvalidOutput`(skip 可能性の宣言は author の
  責務)。
* `when` 内の `$from` 参照は input binding と同じく compiler の依存 edge
  推論に参加し、dependency failed/cancelled の job に依存する job は
  `when` 評価に進まず従来の `DEPENDENCY_FAILED` skip を維持する。

### D5 — `command.grants.<name>.exitCodes` は grant-level opt-in(default `[0]`)

非 0 exit を result data として返すのは grant-level opt-in のみとする —
`command.grants.<name>.exitCodes:[<int>,…]`(default `[0]`)を grant schema
(`crates/cheaper-core/src/policy/command.rs:78-101`)へ追加する。exit code
が集合内なら非 0 でも Err にせず result data `{state, exitCode, stdout,
stderr}`(既存 details shape、`tools/command/mod.rs:86-110`)として返し、
集合外は従来どおり Err。

* **grant-level を選ぶ理由**: 「どの exit が result か」は実行内容ではなく
  認可対象の contract であり、tool input / task / per-call に置くと
  untrusted model input または workflow author が error/result 境界を動か
  せる。grant(trusted layer 宣言)に置くのが責務として正しい。
* spawn 失敗・timeout・abort は `exitCodes` にかかわらず Err のまま —
  process が正常終了した場合の非 0 code のみを result 化し、infra
  failure を result data に変換しない(spec R19/R20 を ratify)。

### D6 — route log store は `<userConfigDir>/route-logs/`、read は scoped 機構のみ

* stdout/stderr 全量を `<userConfigDir>/route-logs/` 配下の runId-unique
  名の file へ tee する(secret を含みうるため workspace 外 —
  ADR-034r05iHrZUE6WljM1ZMiX 系の userConfigDir 分離と同じ論理)。全量
  capture は command tool の `maxOutputBytes` cap と独立で、親へ返す
  compact result は cap 済み + `truncated` flag を持つ。
* 実行中は temp 名へ streaming 書き込み、完了時に `atomic_write`
  (temp+rename、`crates/cheaper-core/src/fs_util.rs:34`)で最終名へ —
  crash 中途の不完全 file は最終名で現れない。
* retention は size 上限・count 上限・TTL の 3 条件で best-effort に強制
  (default 値は design)。
* 読み出しは `cheaper route logs <id>` が担い、offset/maxBytes の bounded
  read とする。参照 contract は `ArtifactReader` / canonical read
  (`tools/artifact.rs:82-88` の positioned bounded read)と揃え、
  `count_tokens`・bulk-reader・`route logs` が 1 つの参照 contract を共有
  する形とする。**generic な workspace 外 file grant は拒否**
  (discovery Option D 却下 — 境界が広すぎる)。log は runId-addressed の
  scoped 機構経由でのみ読め、生の file path による workspace 外 read 経路
  を提供しない。scoped read の正確な形(`artifact://` URI 拡張か
  invocation-scoped read か)は design で確定する。
* log 書き込み・retention の storage failure は原 command の exit
  status・result を失わない(postprocessFailure / `logRef` 欠落へ縮退)。

### D7 — route 層解決は builtin < user < project、project override の trust は route 上の `trusted: true`

* route 宣言の definition 解決は builtin(binary embedded resource)→
  user(`<userConfigDir>/routes.yaml`)→ project
  (`<workspace>/.cheaper/routes.yaml`)の層 precedence とし、同一
  `match.command` prefix では上位層が下位層を shadow する(user は
  builtin を無条件で shadow 可能)。merge 規則は policy layer と同型の
  責務分離に従うが、route に invocation layer は設けない(hook 評価は
  firing 単位で bounded、invocation-scoped route の供給元が存在しない)。
* **project override の trust mechanism は project route 上の明示 field
  `trusted: true` とする**。project 層の route が user/builtin 層 route と
  同一 `match.command` prefix を持つ場合、`trusted: true` がある場合のみ
  project route が有効(shadow する)となり、無い場合はその prefix では
  inactivate されて上位層 route が勝つ。抑制された project route は
  `route resolve` が viability / 診断として報告する(静かに無視しない)。
* `trusted: true` は **explicitness の mechanism であり consent の mechanism
  ではない**: project 自身が宣言できるため、効果は「silent override を
  禁止する」不変条件(spec R7)を explicit・auditable な宣言へ変換すること
  にある。self-declared であっても残存リスクは限定的である —
  (a) routing≠authorization(D2)により実行には引き続き trusted grant の
  argv match が必須で、route は実行対象 command を広げられない、
  (b) project 層の新規 prefix route は trust 無しで既に宣言可能であり、
  override に求める追加防御は「user が既に route した prefix だけ」に
  限られる、(c) `.cheaper/routes.yaml` は通常 review される project
  artifact であり `trusted: true` は diff で可視、(d) override は
  `route resolve` と `kind:"route"` record で provenance を報告する。
* 却下: user-level opt-in list(trusted layer に project 単位の override
  許可を持つ)— consent としてはより強いが、user config が project 内
  artifact を path/name/hash で参照する第二 config surface と staleness
  semantics(path-keyed は後続変更まで信任、hash-keyed は正当な編集の度に
  無効化)を新設する必要があり、上記の限定的な残存リスクに対して割に
  合わない。却下: override 経路を一切持たない(strict precedence のみ)—
  project 側 workflow で routed command を tuned する正当用途を構造的に
  排除し、spec R7 が前提とする「明示 trust 規則の存在」と矛盾する。却下:
  project 内の独立 trust 宣言 file — flag が route file と分離して
  split-brain になり、per-route field の explicitness を失う。
* `trusted: true` は project 層 route にのみ意味を持つ field とし、将来
  user-side consent が必要になった場合は「`trusted: true` + user 側の
  許可」に強化可能な forward-compatible な形とする(schema は変更せず
  追加条件で拡張できる)。

### D8 — env passthrough は `passthroughEnv` allowlist のみ、stdin/TTY は除外

routed 実行へ引き継ぐ env は route の `passthroughEnv` allowlist に列挙
された env 名の hook 評価時 capture 値のみとし、allowlist 外の hook/親
process env は引き継がない(子 env は `minimal_env` + grant 宣言 `env` +
allowlist capture 値で構成 — `tools/proc/mod.rs:164`)。passthrough 値は子
process の実行 env のみに使い、**route log・usage record・workflow の
worker 入力へ載せない**(spec R32 の不変条件 — secret 値が observability
経路へ漏れないことを構造的に保証)。stdin/TTY 依存の command は routing
対象外(`stdin-unsupported` viability)— `route run` は子 process の
stdin を親へ接続しない。hook env ≠ shell env である caveat は
user-facing doc に明記する。

* 却下: 全 env passthrough(secret 漏洩)、env 非継承のみ(実行互換性が
  不十分 — PATH/HOME 系を必要とする command が壊れる)。

### D9 — usage record は `kind:"route"` の独立 contract

routed 実行は `<userConfigDir>/usage.jsonl` へ `kind:"route"` の 1 行
JSONL record として継続記録する — field は `{route, commandHash,
originalTokens, returnedTokens, reductionRatio, worker:{model, usage},
durationMs:{exec, analyze, total}, logRef, refetches,
postprocessFailures}`(spec R34)。既存 `kind:"run"`/`"count"` と同じ
best-effort append 規約(記録失敗は exit code・stdout を変えない、
`usage_log.rs:179`+)に従い、workspace・`.cheaper/` へは書かない。

* **savings contract と分離する理由**: `originalTokens`/`returnedTokens`
  の delta は frontier context への流入削減の実測値であり、worker run の
  estimated savings(`cheaper usage` の `directlyComparable` 母数、
  pricing seam 経由)とは意味が異なる — token delta ≠ billing savings。
  `kind:"route"` record は estimated savings 集計へ混入しない
  (measured offload と estimated savings の分離は
  ADR-034rzGnXqru47VgHNL16cS と同じ構造)。

### D10 — dedup は verdict のみ共有、`VariantInputs::Shell` に `routes.yaml` hash を追加

hook eval dedup の `VariantInputs::Shell`(`eval_cache.rs:314`)に effective
`routes.yaml` 集合の content hash を追加し、route 定義変更で cached
verdict が自然に invalidate されるようにする。**dedup が共有するのは
verdict(rewrite 後 command を含む評価結果)のみであり、command の実行
そのものは dedup しない** — 各 firing は独立して rewrite され、実行は
firing 毎に 1 回行われる(二重実行防止ではない)。

### D11 — exit-code propagation: `route run` の exit = 原 command の exit、再実行なし

`route run` の process exit code は原 command の exit code(非 0 を含め
透過)とする。route 自体の infra failure(re-resolve 再失敗・payload 不正・
target 読み込み不可等、command が未実行の失敗)は cheaper error code で
返し、command の exit code と混同しない。revalidation 失敗時は原 command
を実行せず cheaper error で終了し、routing 層内で raw command を fallback
実行しない(grant 境界の迂回を防ぐ)。post-processing(capture・token 計測・
worker 呼出・usage/log 記録)の失敗は原 command を**再実行しない** —
`postprocessFailures` として記録し、command の exit code・取得済み
output の扱いを変えない。

### D12 — `route run` は workflow を in-process で起動する

`route run` は `cheaper run` と同じく workflow engine を in-process で
compose する(injected executors・invocation-owned resources、
ADR-034AtVcQU6kREPV6w4aHs4 の composition 形)。nested `cheaper run`
subprocess は却下 —

* exit-code contract(D11)は原 command の exit の mirror であり、
  subprocess の exit code(workflow の成否)を proxy すると contract が
  壊れる。
* process hop が増えると signal/process-tree 伝播(親の中断・kill が子
  process 木へ届く contract)と cwd 復元の責務が曖昧になる。
* usage/log 記録が 2 invocation に分散し、`kind:"route"` record の単一
  記録面を失う。

discovery の open question(in-process vs subprocess)をここで確定する。
engine が既に in-process embedding 用に設計されているため追加の分離機構は
不要であり、workflow のどの job が失敗しても `route run` 自身の process
model は一つである。

### D13 — builtin routes は enabled で出荷する(activation は grant で実質 opt-in)

builtin routes(binary embedded resource)は enabled のまま出荷する —
builtin route は auto-granted な grant を伴わないため、user が参照先の
grant を定義しない限り viability は `no-grant` で rewrite は発火しない
(D2 の structural fail-safe)。spec の deferred concern「enabled 出荷か
opt-in 出荷か」をここで確定する。

* opt-in gate(別の enabled flag・配布切替)を重ねる理由が存在しない —
  activation は既に grant requirement で gate されており、route 宣言が
  embedded されているだけでは挙動は変わらない。
* coverage が feature の目的である以上、grant を定義した時点で builtin
  route が即動く形が manual opt-in の教訓と整合する。

### Scope and Non-Goals

* Codex / Cursor / Copilot への Rewrite verdict 発行(approval-bypass 検証
  が必要 — issue #150)。windsurf は rewrite 非対応のため恒久的に
  Allow fallback。
* 複合 shell command(pipe・chain・redirect・metachar 含む)への routing —
  rewrite-safe gate は単一 segment のみ許容する現行境界を維持し、需要が
  確認された時点で revisit する。
* 汎用 workspace 外 file access grant(log 読み出しは runId-addressed の
  scoped 機構のみ、正確な形は design)。
* `route run` payload の encoding・log retention の default 値・巨大 log
  の token 計測方法(scoped read の URI 形と併せて design)。
* Claude `updatedInput` silent-ignore bug(#81340)の version 検出と
  doctor verb(実装時または follow-up)。
* viability precedence の固定順(`unsafe-command → invalid-route →
  missing-target → no-grant → stdin-unsupported → runnable`)は spec R12
  の spec-level detail であり、本 ADR は変更しない。
* worker から更に route/worker を呼ぶ recursive delegation。

### Consequences

* Good, because routed 実行が既存の workflow engine・command tool・
  dedup・usage.jsonl の単一保守面に乗り、`when`・`exitCodes`・scoped
  log read が routing 以外の user workflow からも使える汎用 primitive
  になる(orchestrator の二重実装を回避)。
* Good, because routing が認可を新設しないため、project 層が route を
  持ち込んでも grant 不成立なら構造的に実行不能 — 安全弁が policy の
  既存 trust 境界と同型になる。
* Good, because `updatedInput` のみの rewrite が upstream approval を
  温存し、routing 層の不備・未対応 target は全て passthrough に倒れる
  fail-safe になる。
* Good, because `trusted: true` が silent override を explicit な宣言へ
  変換し、resolve と usage record が override の provenance を報告する。
* Good, because `<userConfigDir>/route-logs/` + scoped read が secret
  を workspace 外に保ち、generic file grant を開けずに事後参照を実現
  する。
* Bad, because workflow schema/engine(`when`・skip materialization・
  dep inference)と command grant schema(`exitCodes`)に拡張が必要で、
  前提 primitive(#144/#145/#146)が routed execution(#147)より先に
  landed する必要がある(実装順序は issue map の依存関係どおり)。
* Bad, because `trusted: true` は self-declared であり malicious project
  が同じ flag を書ける — consent ではなく explicitness の mechanism
  であること、残存リスクの境界(grant・新規 prefix の非制限・provenance
  報告)を user-facing doc に明記する必要がある。
* Bad, because claude/devin 以外の target(codex/cursor/copilot)では
  approval-bypass 検証が済むまで routing coverage が得られず、windsurf
  では rewrite が恒久的に効かない。
* Bad, because `route run` が exit code を mirror する関係上、route の
  infra failure と command の非 0 exit が呼出側から見て別 domain に
  分かれる(cheaper error code vs 原 exit code)ことを docs に明記する
  必要がある。

### Confirmation

* `route resolve` が viability enum と shadowing provenance を報告し、
  project route が `trusted: true` 無しに同一 prefix の user/builtin
  route を shadow しないことが test で固定される。
* runnable → `updatedInput` のみ(permissionDecision 無し)、
  non-runnable / non-match → Allow、windsurf・codex・cursor・copilot で
  Rewrite verdict を発行しないことが test で固定される。
* `route run` が原 command の exit code(0・非 0)を返し、revalidation
  失敗時に command を実行せず cheaper error で終了し、post-processing
  失敗時に再実行しないことが test で固定される。
* `when` の skip≠failure・dependents の非伝播・`$from`→`null`・
  `DEPENDENCY_FAILED` 不変、`exitCodes` の集合内 result 化と infra
  failure の Err 維持が test で固定される。
* `mise run check` が pass すること。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-cli/src/hook/routing_policy.rs`
    - `Verdict`(:116)に `Rewrite{command}` を追加し、Shell routing が
      route 解決→viability→Rewrite/Allow を発行する経路を組む。
      rewrite-safe gate は `tokenize_shell`(:1093)+ cwd-changing set
      (:1309)+ `command_name` self-bypass(:1294)を再利用する。
  - `crates/cheaper-cli/src/hook/targets.rs`
    - claude の `updatedInput` render(`hookSpecificOutput` 内、
      `pretooluse_render` :492 系)と devin の input-rewrite field
      (`devin_render` :545 系)を追加 — いずれも `permissionDecision`
      を emit しない。codex/cursor/copilot/windsurf では Rewrite
      verdict を発行しない target gate を render/normalize 層に置く。
  - `crates/cheaper-cli/src/hook/eval_cache.rs`
    - `VariantInputs::Shell`(:314)に effective `routes.yaml` 集合の
      content hash を追加する。
  - `crates/cheaper-cli/src/route*.rs`(新規)
    - `route resolve`(dry-run:`--command`/`--argv`、viability enum・
      shadowing provenance 報告)と `route run`(structured payload・
      re-resolve + revalidate・exit-code mirror・log capture・usage
      記録)、`route logs <id>`(bounded read)。routes.yaml loader・層
      解決・`trusted: true` 評価もここ(または専用 module)。
  - `crates/cheaper-cli/resources/`
    - builtin `routes.yaml` と routed-exec 用の embedded workflow
      resource。`embedded.rs` の materialize 経路に載せる。
  - `crates/cheaper-cli/src/usage_log.rs`
    - `kind:"route"` record の構築(R34 field 群、best-effort append)。
  - `crates/cheaper-cli/src/run.rs`・`workflow_run.rs`・`cli.rs`・
    `args.rs`・`main.rs`
    - `route` subcommand の wiring と workflow の in-process 起動
      (`cheaper run` の composition と同じ injected-executor 形)。
  - `crates/cheaper-core/src/policy/command.rs`
    - `command.grants.<name>.exitCodes` schema 追加(default `[0]`)。
  - `crates/cheaper-tools/src/tools/command/mod.rs`
    - exit code が集合内なら非 0 を result data 化、集合外・spawn/
      timeout/abort は Err のまま。grant 解決→pin 検証→`proc::run`
      (:385-467)に exitCodes 判定を組み込む。
  - `crates/cheaper-workflow/src/schema.rs`・`engine.rs`・compiler
    - job schema に `when` を追加(:75-88、binding shape :23-48 と衝突
      しない形)、when-false → `Skipped` の diagnostic 区分、dependents
      の非伝播・`$from`→`null` materialization、workflow error 集約
      (:1051-1060)からの condition-skip 除外、`when` の `$from` 参照の
      dep edge 推論参加。
  - `crates/cheaper-cli/src/grant.rs`・`crates/cheaper-cli/src/policy.rs`
    - resolve/run での grant match 再評価経路(TOCTOU 対策)。
  - `docs/guides/`・skill 配布物
    - routes.yaml 記法・routing≠authorization・trust 境界(`trusted:
      true` の意味と限界)・hook env ≠ shell env・exit-code domain・
      fallback 挙動の記述。
  - `crates/cheaper-cli/tests/integration/`・runtime/workflow の contract
    test — module 追加は各 `main.rs` へ `mod` 宣言する AGENTS.md の規約
    に従う。
* Patterns to follow:
  - trusted-layer-only 境界(Option A、ADR-034EMgaatFhsXjR97kH14c)—
    project 層は route を宣言できても grant を生成できず、execution
    viability は実 argv の grant match のみが決める。
  - `Verdict` の internally-tagged serde・per-target normalize/render
    (`targets.rs` の target 別分岐)— rewrite 応答も同一経路に載せ、
    target 差は render 層に閉じる。
  - `tokenize_shell` の rewrite-safe gate・`command_name` の self-bypass・
    cwd-changing set(既存 Shell 判定と同一実装を再利用、複製しない)。
  - `ArtifactReader` の positioned bounded read(offset/maxBytes)と
    `count_tokens`/bulk-reader が共有する参照 contract — scoped log
    read はこの contract に揃え、別 read 機構を発明しない。
  - `atomic_write`(temp+rename、`fs_util.rs:34`)と runId-unique 命名 —
    `hook_config.rs` の write 規約・artifact store の invoke-local
    bounded 方針(ADR-034zTe2Gt1o8oAkX43ilEm)と同じ形。
  - `usage_log.rs` の best-effort append(記録失敗が exit code・stdout
    を変えない)と `<userConfigDir>` 集約規約 — `kind:"route"` も同じ
    経路に載せる。
  - workflow engine の in-process composition(`cheaper run` の
    injected executors / invocation-owned resources、
    ADR-034AtVcQU6kREPV6w4aHs4)— `route run` は同じ形で workflow を
    起動する。
  - dedup の `VariantInputs`/`FingerprintInput` 明示列挙規約 — hash 入力
    は field 単位で明示し、session・pid・env を混ぜない。
* Patterns to avoid:
  - `kind:exec` orchestrator・workflow を介さない routed executor —
    D1 の確定判断に反し、engine 機構の二重実装になる。
  - routing 層内での raw command fallback 実行(revalidation 失敗時)—
    grant 境界の迂回になる。
  - `permissionDecision:"allow"` の一律併記 — codex では approval
    bypass に直結するため rewrite 応答は `updatedInput` のみ。
  - workspace 外 file を読む generic grant・生 file path による log
    read — scoped runId-addressed 機構のみ。
  - verdict dedup の実行 dedup への拡張 — 共有するのは verdict のみ。
  - env 値の log/usage/worker 入力への混入 — `passthroughEnv` の値は
    子 process env 専用(R32 不変条件)。
  - `when` への LLM/外部 I/O predicate、condition skip の dependents
    への伝播、`DEPENDENCY_FAILED` 経路の変更。
  - route 評価用の invocation-scoped layer の新設 — hook 評価は
    firing 単位で bounded、供給元が存在しない(YAGNI)。
* Dependencies or configuration: 新規外部依存は想定しない。`route`
  subcommand・`routes.yaml` schema・`exitCodes`・`when`・route log
  store は全て既存 crate の拡張で構成する。workflow payload の
  encoding・retention default・scoped read の URI 形は design で確定
  するが、本 ADR の境界(orchestrator 不採用・routing≠authorization・
  `trusted: true`・updatedInput-only・in-process 起動)を超えない。
* Expected tests: spec の Acceptance Criteria(R1-R40 由来の
  schema/gate/resolve/run/exitCodes/when/log/scoped read/usage/
  rewrite/dedup 各 contract)を integration・contract test で固定し、
  `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きで
  実行する。

## Verification

* [ ] `routes.yaml` の schema 違反(未知 key・required field 欠落・
      `kind` 非 `"workflow"`・同一 `match.command` 列の重複)が config
      error として表面化し、argv prefix/longest-match/self-bypass が
      test で固定される(R2/R4-R6/R9)。
* [ ] project route が `trusted: true` 無しに同一 prefix の
      user/builtin route を shadow せず、resolve が抑制を viability /
      診断として報告すること、`trusted: true` 付き project route が
      shadow して provenance が報告されることが test で固定される
      (R7、D7)。
* [ ] `route resolve` が `--command`/`--argv` で規定の output shape
      を返し、command・workflow・外部 process を実行せず、viability 各
      値が再現可能であることが test で固定される(R10-R12)。
* [ ] `route run` が原 command の exit code を透過し、revalidation
      失敗時に command を実行せず cheaper error code で終了し、
      post-processing 失敗時に再実行しないことが test で固定される
      (R14-R16、D11)。
* [ ] `route run` の実行互換 — payload cwd 復元・env が
      `passthroughEnv` allowlist 外を含まないこと・stdin 非接続が
      test で固定される(R18/R32/R33、D8)。
* [ ] `exitCodes` — 集合内の非 0 exit が result data として返る
      こと、集合外・spawn 失敗・timeout・abort が Err のままである
      ことが test で固定される(R19/R20、D5)。
* [ ] workflow `when` — `eq|ne|gt|lt|exists` 各 predicate、
      when-false → `skipped`(DEPENDENCY_FAILED と区別可能)、
      dependents の非伝播・`$from`→`null` materialization、skip 含み
      workflow の `ok:true` valid result が test で固定される
      (R21-R25、D4)。
* [ ] route log store — `maxOutputBytes` 超過 output が
      `<userConfigDir>/route-logs/` に全量残り compact result に
      `truncated` が付くこと、runId-unique + temp+rename 命名、
      size/count/TTL retention、storage failure 時の exit status 保持
      が test で固定される(R26-R29、D6)。
* [ ] `cheaper route logs <id>` が bounded read(offset/maxBytes)を
      提供し、workspace 外 generic file read 経路が存在しないことが
      test で固定される(R30/R31、D6)。
* [ ] `kind:"route"` record が規定 field 群を持ち、best-effort
      append であり、savings estimated 集計へ混入しないことが test
      で固定される(R34/R35、D9)。
* [ ] hook rewrite — claude/devin payload で runnable →
      `updatedInput` のみの応答(`permissionDecision` 無し・exit 0)、
      non-runnable / non-match → Allow、windsurf・codex・cursor・
      copilot で Rewrite verdict を発行しないことが test で固定
      される(R36-R39、D3)。
* [ ] `VariantInputs::Shell` が `routes.yaml` hash を含み、
      routes.yaml 変更で cached verdict が invalidate され、各
      firing が独立に実行されることが test で固定される(R40、D10)。
* [ ] builtin routes が auto-grant 無しで embedded され、grant 未定義
      環境では rewrite が発火しないことが test で固定される(D2/D13)。
* [ ] user-facing doc に routes.yaml 記法・routing≠authorization・
      `trusted: true` の意味と限界・hook env ≠ shell env・exit-code
      domain・fallback 挙動が記述されていることが確認される。
* [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` と `mise run
      check` が pass する。

## Pros and Cons of the Options

### Option A — 専用 `kind:exec` orchestrator

* Good, because routed 実行専用に最適化でき、workflow schema を変更
  しない。
* Bad, because command tool・admission・cancel・capture・usage の
  機構を丸ごと二重実装することになり(D1 却下、#142 で directive
  却下済み)、`when`/`exitCodes` が user workflow に開放されず
  routing 専用の死に筋肉になる。

### Option B — caller-side two-phase orchestration

* Good, because workflow schema を変えずに済む。
* Bad, because orchestration が caller(hook / `route run`)へ分散して
  事実上の orchestrator を workflow の外に生やし、user workflow
  からは同じ条件実行が表現不能になる(D4 の前提として却下)。

### Option C — workflow 拡張(chosen)

* Good, because command tool・engine・dedup・usage の既存機構をその
  まま再利用でき、`when`・`exitCodes`・scoped read が汎用 primitive
  になる。route は「prefix → workflow + grant 参照」の宣言に留まり、
  新しい実行 kind を持たない。
* Bad, because workflow schema/engine・command grant schema に拡張
  が必要で、前提 primitive が先に landed する必要がある(実装順序
  の依存関係)。

### D7 補助選択肢

* `trusted: true`(chosen)— silent override を explicit 宣言へ変換し、
  resolve/usage record が provenance を報告する。self-declared でも
  残存リスクは grant 境界・新規 prefix の非制限・review 可視性で
  限定される。
* user-level opt-in list — consent としては強いが、user config が
  project artifact を参照する第二 config surface + staleness
  semantics が要り、限定的な残存リスクに対して割に合わない(却下)。
* override 経路無し — 正当な project workflow 用途を構造的に排除
  し、spec R7 の「明示 trust 規則」前提と矛盾する(却下)。
* project 内の独立 trust 宣言 file — route file との split-brain
  で explicitness が薄れる(却下)。

### D12 補助選択肢

* in-process 起動(chosen)— `cheaper run` と同じ composition、
  exit-code mirror・signal 伝播・usage 記録の責務が単一 process
  に閉じる。
* nested subprocess — exit code が workflow 成否と衝突し、
  signal/cwd 伝播に hop が増え、usage/log が 2 invocation に分散
  する(却下)。

## More Information

* upstream discovery:
  [pretooluse-auto-routing-to-tasks-and-workflows.md](../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md)
  (DISC-034J7LwMEdPR5HxFkgOivZ)— 実測データ・issue #142 確定判断
  14 項目・alternatives 評価(A-F)・open questions を収録。本 ADR は
  promotion candidate「adr-doc needed」を引き継ぎ、open question のうち
  project route trust mechanics と in-process vs subprocess を確定する
  (scoped log read の形・Claude silent-ignore 検出・retention default
  は design へ defer)。
* sibling spec:
  [pretooluse-route-resolver-and-workflow-delegation.md](../specs/pretooluse-route-resolver-and-workflow-delegation.md)
  (SPEC-034J7SxWvGL3oSbeBpo8a3)— 本決定を R1-R40 の contract へ
  展開した並行 spec。viability precedence(R12)等の spec-level detail
  は spec 側が正本であり、本 ADR は architecture・信頼境界・telemetry
  contract の決定のみを担う。`derives-from` は lineage 上の upstream
  (discovery)へ張り、spec は chain の sibling として `related` で結ぶ
  (前 chain の ADR-034IOqXM5YpqVgFZKatIXN と同じ規約 — `refines` /
  `refined-by` は chain を跨ぐ doc 間では使わない)。
* governing precedent:
  [command-grant-trust-boundary-and-managed-process-reuse.md](./command-grant-trust-boundary-and-managed-process-reuse.md)
  (ADR-034EMgaatFhsXjR97kH14c)— D2/D7 が継承する「grant authoring は
  trusted layer のみ」の信頼境界。
  [compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md](./compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)
  (ADR-034AtVcQU6kREPV6w4aHs4)— D12 が従う in-process workflow
  composition。
  [dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md](./dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md)
  (ADR-034Ie3HFd6tIJms6Z2NIow)— D10 が拡張する verdict dedup。
  [cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md](./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md)
  (ADR-034H2WBNtC4We8dyC5lQZJ)— D3 の rewrite deploy 形態が従う
  hook 登録 lifecycle の CLI ownership。
* issue map(parent #142 と実装分割): #143 Route Resolver +
  `routes.yaml` + `route resolve`、#144 command tool `exitCodes`、
  #145 log store + scoped read、#146 workflow `when`、#147 `route
  run` + routed-exec workflow(依存 #143-#146)、#148 hook Rewrite +
  Claude/Devin + dedup(依存 #143,#147)、#149 usage `kind:"route"`
  + `route logs` + retention + docs(依存 #145,#147)、#150 Codex/
  Cursor/Copilot rewrite(deferred、依存 #148)。実装順序:
  **#143 → #144/#145/#146 → #147 → #148 → #149 → #150**。
* status は `proposed` — review-protocol による promotion は後続
  ゲートで行う。

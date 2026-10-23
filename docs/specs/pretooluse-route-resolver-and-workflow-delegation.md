---
id: "SPEC-034J7SxWvGL3oSbeBpo8a3"
type: "spec"
status: "proposed"
title: "PreToolUse auto-routing: route resolution and delegated workflow execution"
created: "2026-10-20"
updated: "2026-10-23"
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
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "./shell-read-large-file-blocking.md"
    - "./invocation-scoped-command-grants.md"
    - "./policy-gated-command-tool.md"
    - "./hook-duplicate-firing-single-evaluation-dedup.md"
    - "./delegation-usage-measurement-log.md"
    - "./one-shot-workflow-engine-and-run-cli.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "./permission-model-responsibility-redesign.md"
  refines: []
  refined-by:
    - "./codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
  derives-from:
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
  derived-by:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  verifies: []
  verified-by: []
  references:
    - "./usage-and-savings-telemetry-aggregation.md"
    - "./canonical-token-counter.md"
    - "./run-scoped-artifact-store.md"
    - "./workflow-file-interface.md"
    - "./workflow-contract-compiler.md"
    - "../discovery/invocation-scoped-command-grants.md"
    - "../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
    - "../discovery/usage-and-savings-telemetry-for-delegation-reduction.md"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/grant.rs"
    - "crates/cheaper-cli/src/policy.rs"
    - "crates/cheaper-cli/src/count.rs"
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
---

# PreToolUse auto-routing: route resolution and delegated workflow execution

## What Should Be Built

PreToolUse hook が親 agent の shell command 実行を評価し、登録済み
route に match した単一 command を `cheaper route run` への rewrite
として返す自動 routing 機構を定義する。routing pipeline は
`PreToolUse → Route Resolver → 登録済み workflow → 親 agent` で
構成され、routed 実行は専用 orchestrator ではなく**登録済み
workflow の起動**として表現される(command tool による実行 +
output capture + token 計測 + conditional worker による縮約)。

本 spec が formalize する contract:

- **route 宣言(`routes.yaml`)**: argv-prefix match・longest-match
  precedence・層間 trust 境界・rewrite-safe gate・self-bypass。
- **`cheaper route resolve`(dry-run)**: route 解決と viability
  判定を実行を伴わずに行う検査経路。
- **`cheaper route run`**: structured input を受け、実行直前に
  re-resolve + revalidate してから workflow を起動する実行経路。
- **command tool `exitCodes`**: grant-level opt-in で非 0 exit を
  result data として返す拡張(routed workflow が command の非 0
  exit を workflow failure にせず処理するための前提 primitive)。
- **workflow `when`**: `$from`/`$input` 値への deterministic
  predicate による conditional job(skip ≠ failure)。
- **route log store + scoped log read**:
  `<userConfigDir>/route-logs/` への全量 capture と、bounded で
  scope された読み出し(`cheaper route logs <id>`)。
- **env passthrough**: `passthroughEnv` allowlist のみを子 process
  env へ流す境界。
- **usage record `kind:"route"`**: routing の削減実績を既存
  `usage.jsonl` とは別 contract で記録する record。
- **hook rewrite**: `Verdict::Rewrite{command}` の追加と
  claude/devin への `updatedInput` のみの応答
  (upstream approval を温存)。
- **dedup**: `VariantInputs` への `routes.yaml` hash 追加による
  verdict cache の共有(実行の dedup ではない)。

設計判断は issue #142 で確定済みであり(discovery の「確定済み
設計入力」14 項目)、本 spec はそれらを要件として固定する。認可
境界・workflow 主実行経路・trust mechanics 等の decision record
は sibling ADR が担う。

## Why Now

- 実測で manual opt-in の coverage が致命的に低い(discovery
  記録): Devin 238 session で cheaper hook の発火は 12 session
  のみ(adoption <1%)。一方で tool output の frontier context
  流入は 5 週間で 1.43GB(exec 262MB / edit echo 758MB / read
  362MB)、Codex では >10KB の tool output が全 output bytes の
  64% を占める。hook が発火した session では 13.6M tokens の流入を
  実際に block しており、機構の効果は実証済み — 足りないのは
  coverage である。
- 巨大 output が発生する時点で自動的に delegation 経路へ倒すには、
  親 agent が明示呼出する manual opt-in ではなく PreToolUse 層での
  rewrite が必須。
- routing を安全に導入するには、宣言(route)と認可(grant)の分離・
  rewrite の fail-safe・実行互換(cwd/env/timeout/signal)・事後の
  log 参照という一連の不変条件を先に contract として固定する必要が
  ある。本 spec はその受け入れ境界を与える。

## Users and Value

- **親 coding agent(Claude / Devin が先行対象)**: 巨大 tool output
  が frontier context へ流入する前に compact result + log
  reference へ置き換わる。viability 不全時は rewrite せず従来挙動
  (Allow)に落ちるため、routing 層の不具合が command 実行を壊さ
  ない。
- **cheaper user / consumer project**: route を宣言するだけで自動
  delegation の対象を増やせる。認可は既存 `command.grants` が担い、
  project は route を定義できても grant を生成できないため、信頼
  境界は既存 policy 層と同型に保たれる。
- **cheaper maintainer**: `when`・`exitCodes`・scoped log read は
  workflow/tool 層の汎用 primitive として提供され、routing 以外の
  user workflow からも利用できる。

## Scope

### In Scope

- `routes.yaml` の schema(`{name, match:{command:[token…]},
  target:{kind:"workflow", path}, grant, output:{tokenThreshold?},
  passthroughEnv?}`)と層解決(builtin embedded / user
  `<userConfigDir>/routes.yaml` / project
  `<workspace>/.cheaper/routes.yaml`)、argv-prefix match・
  longest-match precedence・同一長衝突の config error 化・層間
  trust の不変条件。
- `cheaper route resolve`(`--command` / `--argv`、dry-run、
  viability enum)と `cheaper route run`(structured input、
  re-resolve + revalidate、exit code contract)の CLI contract。
- command tool `exitCodes` grant-level opt-in(`[0]` default)。
- workflow `when` conditional job semantics(predicate 集合
  `eq|ne|gt|lt|exists`、skip 伝播と output materialization
  規則、skip ≠ workflow failure)。
- route log store(`<userConfigDir>/route-logs/`、全量 tee、
  atomic write、runId-unique 命名、size/count/TTL retention、
  `truncated` flag)と scoped log read(`cheaper route logs <id>`、
  bounded read、ArtifactReader/canonical read と共有される参照
  contract)。
- `passthroughEnv` allowlist による env passthrough と stdin/TTY
  依存 command の routing 除外。
- `kind:"route"` usage record(`<userConfigDir>/usage.jsonl`)。
- `Verdict::Rewrite{command}` verdict 追加と claude/devin への
  `updatedInput` のみの応答、非 runnable viability の Allow
  fallback。
- dedup — `VariantInputs` への `routes.yaml` hash 追加。
- 実行互換 — hook payload cwd の復元、timeout clamp、signal /
  process-tree 伝播、stdin 非接続。
- route 記法・trust 境界・fallback 挙動を記述する user-facing
  doc(guide / skill)の更新。

### Out of Scope

- 専用 `kind:exec` orchestrator、workflow を介さない routed
  executor(#142 で却下済み — workflow が主実行経路)。
- Codex / Cursor / Copilot への rewrite 応答(approval-bypass 検証が
  必要なため issue #150 へ defer)。windsurf は rewrite 非対応のため
  Allow fallback のみ。
- worker から更に route/worker を呼ぶ recursive delegation。
- workspace 外 file を読む汎用 grant(log 読み出しは scoped 機構
  のみ)。
- 複合 shell command(pipe・chain・redirect・metachar 含む)への
  routing — rewrite-safe gate が拒否し続ける。
- project 層による grant 生成・route 経由の暗黙認可(routing は
  実行を認可しない)。
- Claude `updatedInput` silent-ignore(#81340)の version 検出と
  doctor verb。

## Requirements

### Route 定義(`routes.yaml`)と解決

- R1: `routes.yaml` は `{version: 1, routes: Route[]}` を持つ。
  Route の field は `{name, match:{command:[token,…]},
  target:{kind:"workflow", path}, grant, grants:[name,…]?,
  output:{tokenThreshold?}, passthroughEnv:[name,…]?, trusted?}`
  とする。`name` は route を一意に識別する ID、`match.command`
  は argv token prefix、`target` は `kind:"workflow"` のみを
  許容し `path` は登録済み workflow file を指す。`grant` は
  `command.grants` の既存 grant 名への参照であり、route は
  grant を生成しない。`grants`（issue #166）は anchor
  `route.grant` 以外の追加 delegated grant 名の optional 配列で、
  実効 grant set は `{route.grant} ∪ grants` とする（member は
  宣言順に parse され、重複と `route.grant` 自身への参照は
  除去される — 空・未指定は同等）。`grants` は builtin/user の
  trusted 層のみが authoring でき、project 層 entry に `grants`
  key が存在すると値の形状に関わらず route load が
  `InvalidRuntimePolicy` で失敗する（fail-loud — project が
  宣言できるのは従来通り単数 `grant:` anchor 参照のみ）。
  member を呼ぶ routed workflow 内の `command` job は
  `job_id != "exec"` で宣言する — `job_id == "exec"` の
  `command` job は anchor grant + 元 argv の expected-input に
  束縛され、不一致は `exec-input-mismatch`（PolicyViolation）
  とする。
  `output.tokenThreshold` は routed workflow の conditional
  worker を起動する閾値の per-route override、`passthroughEnv`
  は hook env から passthrough を許す env 名の allowlist。
  `trusted` は project 層 route にのみ意味を持つ optional bool
  で、同一 `match.command` を持つ下位層(user/builtin)route を
  shadow する明示宣言とする(R7 — mechanism は
  ADR-034J8eZHNPqARi25HLFOL6 D7)。
- R2: `routes.yaml` の読み込みは fail-loud とする — malformed
  YAML・schema 違反(未知 key・required field 欠落・`kind` 非
  `"workflow"` 等)は「invalid policy is never silently ignored」
  の `load_policy` 規約と同型に config error として表面化し、
  壊れた route を黙って drop しない。
- R3: route 宣言は 3 層で解決する — builtin(binary embedded
  resource、enabled で出荷するが auto-granted な grant を伴わない
  — activation は user の grant 定義で実質 opt-in、
  ADR-034J8eZHNPqARi25HLFOL6 D13)、user
  (`<userConfigDir>/routes.yaml`)、project
  (`<workspace>/.cheaper/routes.yaml`)。route 層に invocation
  layer は設けない(hook 評価は firing 単位で bounded で供給元が
  存在しない — ADR-034J8eZHNPqARi25HLFOL6 D7)。層の解決順は
  builtin → user → project で、同一 `match.command` では上位層が
  下位層を shadow する(R7 の trust 条件を除く)。merge 規則は
  policy layer(`crates/cheaper-core/src/policy/load.rs`)と同型の
  責務分離に従う。
- R4: `match.command` は token 列の argv prefix match とする —
  `argv[i] == match.command[i]` が全 `i < len(match.command)` で
  成立した場合に match。match した route の `args` は
  `argv[len(match.command)..]`(prefix 後ろの残 token 列)。
- R5: match した route が複数ある場合、`match.command` の token 数
  が最大のもの(longest match)を採用する。
- R6: effective route set 内で同一の `match.command` token 列を持つ
  有効 route が 2 つ存在する場合は config error(`invalid-route`)
  とする。silent な last-wins / first-wins / layer 優先黙殺は禁止
  (却下済み alternative F — project が user route を黙って差し替え
  られる経路を作らない)。
- R7: 層間 trust — project 層の route は、明示 trust 規則が無い
  限り、同一 `match.command` に対する user/builtin 層 route を
  shadow / 差し替えてはならない。明示 trust 規則は project
  route 上の `trusted: true` field とする
  (ADR-034J8eZHNPqARi25HLFOL6 D7 で確定) — `trusted: true` を
  宣言した project route のみが同一 `match.command` の下位層
  (user/builtin)route を shadow する。trust 不在で衝突する project route は
  その `match.command` について有効化されず(上位層 route が
  勝ち)、`route resolve` はその旨を viability / 診断として報告
  する(静かに無視しない)。`trusted: true` は explicitness の
  mechanism であり consent の mechanism ではない(project 自身が
  宣言可能 — 実行は引き続き trusted grant の argv match が
  要求される)。override の provenance は `route resolve` と
  `kind:"route"` record が報告する。
- R8: rewrite-safe gate — 対象 command を `tokenize_shell`
  (`crates/cheaper-cli/src/hook/routing_policy.rs:1093`)で評価し、
  以下を全て満たす場合のみ routing 対象とする: (a) ちょうど 1
  segment、(b) `unsafe_segment == false`、(c) 全 token が
  `unsafe_token == false`(pipe・chain・redirect・metachar・command
  substitution 等を含まない)、(d) head command が cwd-changing
  set(`cd`/`chdir`/`pushd`/`popd`/`set-location`/`sl`)に該当し
  ない。tokenize 不能・条件不成立は全て「routing しない」に倒す。
- R9: self-bypass — `command_name(argv[0])`(dir prefix と
  `.exe`/`.bat`/`.cmd` suffix を剥がす
  `routing_policy.rs:1294` の正規化)が `cheaper` に一致する
  command は route match を行わず routing しない(rewritten
  `cheaper route run …` の再 routing・再帰を構造的に防ぐ)。

### `cheaper route resolve`(dry-run)

- R10: 入力は `--command <string>`(shell 文字列 —
  `tokenize_shell` で token 化してから評価)または `--argv`
  (JSON string 配列 — 既 tokenize 済み入力)のいずれか 1 つを
  必須とする。resolve は原 command・route target workflow・外部
  process を一切実行しない(dry-run)。
- R11: resolve の出力 contract は `{matched, route?, target?,
  grant?, executable?, argv, args, viability?}` とする。
  `route`/`grant` は match した route 名・参照 grant 名、
  `executable` は argv[0] の正規化名、`args` は R4 の残 token 列。
  `matched:false` の場合は `route`/`target`/`grant`/`viability`
  を持たない(no match は error ではない)。
- R12: `viability` は enum `{runnable, no-grant, missing-target,
  invalid-route, unsafe-command, stdin-unsupported}` の単一値で、
  match した route に対する実行可否判定を表す — 実 argv に対する
  grant match、target の存在と読み込み可否、route 有効性(R6/R7)、
  rewrite-safe gate(R8)、stdin/TTY 依存(R32)を評価する。複数の
  失敗が同時に成立しうる場合、報告は固定の deterministic な優先順
  `unsafe-command → invalid-route → missing-target → no-grant →
  stdin-unsupported → runnable` で 1 値に畳む。no-grant 評価は
  anchor `grant:` の完全な実行可否（存在・`scope:"trusted" &&
  !fixed`・executable pin・実 argv byte-match）に加え、宣言
  `grants` member それぞれへの 存在 + 同 eligibility + pin 検査
  を含む（issue #166 — member への元 argv 照合は行わず、member
  job の argv は各 grant 自身の pattern が job 実行時に拘束する。
  member 失敗は新規 viability 値を増やさず anchor 失敗と同じ
  `no-grant` へ畳む）。

### `cheaper route run`

- R13: 入力は structured payload `{command, executable, argv, args,
  cwd, env?}` とする。`command` は原 shell 文字列(record/log
  表示用)、`env` は hook 評価時に `passthroughEnv` allowlist で
  capture した値のみを載せる。payload の encoding(単一 argv
  token に載る形)は design で確定する。
- R14: `route run` は受領 payload の `argv` に対して route 解決・
  rewrite-safe gate・grant match・target 存在・`passthroughEnv`
  allowlist を実行直前に再評価する — hook が発行した payload の
  resolve 結果を信用しない(TOCTOU・payload 改竄対策)。
  revalidation 失敗時は原 command を実行せず cheaper error
  (code + exitCode)で終了する — routing 層内で raw command を
  fallback 実行してはならない(grant 境界の迂回になる)。
- R15: `route run` の process exit code は**実行した command** の
  exit code とする(非 0 を含め透過)— 既定 mode では原 command、
  route が `substitute` を宣言する mode では substitute command の
  exit code(親 agent は引き続きそれを原 command の exit として
  解釈する — semantic substitution は trusted 層 opt-in で受容する、
  permission-model-responsibility-redesign spec/design の決定)。
  route 自体の infra failure(resolve 再失敗・payload 不正・target
  読み込み不可等)は cheaper error code で返し、command の
  exit code と混同しない。
- R16: post-processing(output capture・token 計測・conditional
  worker 呼出・usage/log 記録)の失敗は原 command を再実行しない。
  post-processing 失敗は `postprocessFailures` として記録し、
  command の exit code・取得済み output の扱いを変えない。
- R17: routed 実行は `target.path` の登録済み workflow の起動として
  表現する(command tool + capture + token 計測 + `when` 付き
  conditional worker の構成)。専用 `kind:exec` orchestrator や
  workflow を介さない executor は設けない。
- R18: 実行互換 — `cwd` は hook payload の cwd を復元、env は
  `minimal_env` + grant 宣言 `env` + `passthroughEnv` capture 値
  + 原 command の env-prefix 宣言(`FOO=1 cmd` の leading
  `KEY=VALUE` assignment — command 自身の semantics であり
  ambient env capture ではない。`payload.command` から再導出
  されるため payload で transport しない)のみ、timeout/killGrace
  は grant/policy の bound へ clamp、signal/process-tree 伝播
  (親の中断・kill が子 process 木へ届く)を維持、stdin は子
  process へ接続しない。

### command tool `exitCodes`

- R19: `command.grants.<name>` に optional `exitCodes:[<int>,…]` を
  追加する(grant-level opt-in、default `[0]`)。command の終了
  exit code が集合内の場合、非 0 でも Err にせず result data
  `{state, exitCode, stdout, stderr}`(既存 details shape
  `crates/cheaper-tools/src/tools/command/mod.rs:90-110`)として
  返す。exit code が集合外の場合は従来どおり Err。
- R20: spawn 失敗・timeout・abort は `exitCodes` にかかわらず従来
  どおり Err のままとする — `exitCodes` は process が正常終了
  した場合の非 0 code のみを result 化し、infra failure を result
  data に変換しない。

### workflow `when`

- R21: workflow job に optional `when` を追加する — `$input`
  (workflow inputs)と `$from`(他 job の result)値に対する
  deterministic predicate で、operator 集合は `eq|ne|gt|lt|exists`
  に限定する。LLM・外部 I/O を介さず、同じ入力に常に同じ結果を
  返す純粋評価とする。`when` は全依存 job が terminal になった
  後・admission/実行前に評価する。
- R22: `when` が false の job は terminal state `skipped` へ遷移
  し、dependency failure 由来の skip(現行
  `DEPENDENCY_FAILED`、`engine.rs:649-658`)とは outcome 上で
  区別可能な診断を持つ。condition skip は `dependency_failed` を
  dependents へ伝播させず、それ単独で workflow を failure にし
  ない(`engine.rs:1051-1060` の「非 Succeeded job = workflow
  error」規約から condition-skip を除外する)。
- R23: condition-skipped job の dependents は自動 skip されず、
  自身の `when` を評価する(`when` が無ければ従来どおり実行)。
  skipped producer への `$from` 参照は `null` に materialize し、
  `outputs` の `$from` binding が skipped producer を指す場合も
  `null` に materialize する — output schema が `null` を許容
  しない場合は従来どおり `InvalidOutput` の output error(author
  が skip 可能性に応じて schema を宣言する責務)。
- R24: `when` 内の `$from` 参照は `input` binding と同じく compiler
  の依存 edge 推論に参加する(predicate が他 job の result を読む
  場合、その job は実行順序上の先行 node になる)。dependency
  failed/cancelled の job に依存する job は `when` 評価に進まず
  従来の `DEPENDENCY_FAILED` skip 経路を維持する。
- R25: `when` は job-level の optional key として schema に追加し、
  `$input`/`$from`/`$literal` binding shape(`schema.rs:23-48`)と
  衝突しない。condition skip された worker を含む workflow でも、
  他 job 成功 + output materialization 成立なら `ok:true` の有効な
  result を返せる。

### route log store

- R26: routed 実行の stdout/stderr 全量を
  `<userConfigDir>/route-logs/` 配下の runId-unique 名の file へ
  tee する。全量 capture は command tool の `maxOutputBytes` cap
  と独立し、親へ返す compact result は cap 済み + `truncated`
  flag を持つ — parent-facing 出力の切捨てが log の完全性を
  損なわない。secret を含みうるため log を workspace 配下へ置か
  ない。
- R27: log file は実行中に temp 名へ streaming 書き込みし、完了時に
  `atomic_write`(temp+rename、`crates/cheaper-core/src/fs_util.rs:34`)
  で最終名へ rename する — crash 中途の不完全 file は最終名では
  現れない。runId-unique 命名で run 間・retry 間の衝突を防ぐ。
- R28: retention は size 上限・count 上限・TTL の 3 条件で
  best-effort に強制する(default 値は design で確定)。
- R29: log 書き込み・retention の storage failure は原 command の
  exit status・result を失わない — 失敗は記録側の
  postprocessFailure / `logRef` 欠落として縮退し、routing 層の
  I/O 不具合が command 実行結果を隠蔽しない。

### scoped log read

- R30: 事後参照は `cheaper route logs <id>` が担い、offset /
  maxBytes の bounded read(paged 読み出し)を提供する。route log
  の参照 contract は `ArtifactReader` / canonical read
  (`crates/cheaper-tools/src/artifact.rs:82-88` の positioned
  bounded read)と揃え、`count_tokens`・bulk-reader・`route logs`
  が 1 つの参照 contract を共有する形とする。
- R31: workspace 外 file を読む汎用 grant は開けない — log は
  runId-addressed の scoped 機構経由でのみ読み出せ、生の file
  path 指定による workspace 外 read 経路を提供しない(却下済み
  alternative D)。

### env passthrough / stdin 除外

- R32: routed 実行へ引き継ぐ env は `passthroughEnv` allowlist に
  列挙された env 名の hook 評価時 capture 値のみとする。allowlist
  外の hook/親 process env は引き継がない(子 env は R18 の
  `minimal_env` + grant 宣言 `env` + allowlist capture 値で構成)。
  passthrough 値は子 process の実行 env のみに使い、route log・
  usage record・workflow の worker 入力へ載せない。hook env ≠
  shell env(hook process の env は親 shell の interactive env と
  一致しない)である旨を user-facing doc に明記する。
- R33: stdin / TTY 依存の command は routing 対象外とする —
  `route run` は子 process の stdin を親へ接続しないため、stdin
  読み取り・対話を要求する command は resolve が
  `stdin-unsupported` を返す(判定不能なケースでも routing 側が
  stdin を供給しないことは構造的に保証される)。env 互換を
  満たせない command も同様に routing しない。

### usage record `kind:"route"`

- R34: routed 実行の結果は `<userConfigDir>/usage.jsonl` へ
  `kind:"route"` の 1 行 JSONL record として継続記録する — field
  は `{route, commandHash, executed:{grant, argv},
  originalTokens, returnedTokens,
  reductionRatio, worker:{model, usage}, durationMs:{exec, analyze,
  total}, logRef, refetches, postprocessFailures}`。`executed` は
  additive field（issue #166）で、実際に実行された exec job の
  `{grant, argv}`（env-prefix-strip 済み — prefix 値は含まない）
  を記録し、宣言 identity（`route`・`commandHash`）と実行
  attribution を分離する。`executed.argv` は平文の引数値を含む。
  `originalTokens`
  は capture された原 output の token 数、`returnedTokens` は親へ
  返却した result の token 数、`reductionRatio` はその縮約率、
  `durationMs` は command 実行(exec)/worker 分析(analyze)/全体
  (total)の内訳、`logRef` は `route logs` で参照可能な log
  identity、`refetches` は親が scoped read で log 内容を引き直した
  回数、`postprocessFailures` は R16/R29 の失敗記録。
- R35: `kind:"route"` record は既存 `kind:"run"`/`"count"` と同じ
  best-effort append 規約(記録失敗は exit code・stdout を変え
  ない、`usage_log.rs:179`+)に従い、workspace・`.cheaper/` へは
  書き込まない。savings 用 estimated contract(`cheaper usage` の
  `directlyComparable` 母数)とは分離された独立 contract とし、
  `kind:"route"` record が worker run の estimated savings 集計へ
  混ざらない。

### hook rewrite(`Verdict::Rewrite`)

- R36: `Verdict` enum(`routing_policy.rs:116`)に
  `Rewrite{command, fallback}` variant を追加する — wire 表現は
  既存の internally-tagged serde 規約(`Allow` →
  `{"decision":"allow"}`)に倣い
  `{"decision":"rewrite","command":"…","fallback":{…}}`。
  `command` は rewrite-safe に構成された単一 segment の
  `cheaper route run` 呼出し(R13 payload を保持)、`fallback`
  は routing 層を通らなかった場合にその command が得た verdict
  (Allow または read-candidate Deny)を保持し、rewrite 非対応
  target の render が採用する。Rewrite verdict は command が
  route runnable の場合のみ発行する。
- R37: render は target 別に `updatedInput` のみを発行し
  `permissionDecision` を emit しない — claude は
  `hookSpecificOutput` 内の `updatedInput`、devin は同等の
  input-rewrite field を用い、いずれも exit 0・上流 approval
  flow を温存する(却下済み alternative E — `permissionDecision:
  "allow"` 併記は approval bypass のため不可)。
- R38: rewrite 対象 target は claude + devin を先行とする。
  windsurf・codex・cursor・copilot では `updatedInput` を発行せず、
  R36 の `fallback` verdict を各 target の応答規約で render する
  (windsurf は rewrite 非対応、codex/cursor/copilot は
  approval-bypass 検証を要するため issue #150 へ defer — いずれも
  routing 層が read-candidate Deny を Allow に変えることはない)。
- R39: viability が non-runnable(no-grant・missing-target・
  invalid-route・unsafe-command・stdin-unsupported)または route
  non-match の場合、routing 層は Rewrite を発行せず従来の評価
  経路(read-candidate 評価含む)へ継続する — 最終 verdict は
  従来経路が決める(Allow または Deny)。Rewrite が deny・exec
  skip の意味を持つことはなく、routing 層の不備が親の command
  実行を阻害しない。

### dedup

- R40: hook eval dedup の `VariantInputs::Shell`
  (`eval_cache.rs:314`)に effective `routes.yaml` 集合の content
  hash を追加し、route 定義変更で cached verdict が自然に
  invalidate されるようにする。dedup が共有するのは Allow/Deny
  verdict のみ — Rewrite verdict は passthroughEnv capture 値を
  payload 内に含みうるため verdict cache へ publish/serve しない
  (R32 の非永続化意図を守る)。command の実行そのものは dedup
  しない — 各 firing は独立して評価・rewrite され、実行は
  firing 毎に 1 回行われる。

## Acceptance Criteria

- [x] `routes.yaml` の schema(required field 欠落・未知 key・
  `kind` 非 `"workflow"`・同一 `match.command` 列の重複 route)が
  config error として表面化することが contract test で固定される
  (R2/R6)。
- [x] argv prefix match・longest-match precedence・self-bypass
  (`cheaper`・`cheaper.exe`・dir prefix 付き argv[0] が routing
  されない)が test で固定される(R4/R5/R9)。
- [x] rewrite-safe gate — pipe・chain・redirect・metachar・`cd` 系
  head・複数 segment・tokenize 不能の各ケースが全て non-routed
  (`unsafe-command` viability)となることが test で固定される
  (R8/R12)。
- [x] `route resolve` が `--command` / `--argv` 両入力で規定の
  output shape を返し、command・workflow・外部 process を一切
  実行しないこと、および viability 各値(runnable・no-grant・
  missing-target・invalid-route・unsafe-command・stdin-unsupported)
  が再現可能であることが test で固定される(R10-R12)。
- [x] trust 不在で project route が同一 prefix の user/builtin
  route を shadow せず、resolve が当該状況を報告することが test
  で固定される(R7)。
- [x] `route run` が原 command の exit code(0・非 0)をそのまま
  返すこと、revalidation 失敗時に原 command を実行せず cheaper
  error code で終了すること、post-processing 失敗時に原 command
  を再実行しないことが test で固定される(R14-R16)。
- [x] `route run` の実行互換 — payload cwd 復元・env が
  `passthroughEnv` allowlist 外を含まないこと・stdin 非接続が
  test で固定される(R18/R32/R33)。
- [x] `exitCodes` — 集合内の非 0 exit が result data
  `{state, exitCode, stdout, stderr}` として返ること、集合外の非
  0・spawn 失敗・timeout・abort が Err のままであることが test
  で固定される(R19/R20)。
- [x] workflow `when` — `eq|ne|gt|lt|exists` 各 predicate、
  when-false → `skipped`(DEPENDENCY_FAILED と区別可能)、
  dependents の非伝播・`$from` skipped producer → `null`
  materialization、skip 含み workflow の `ok:true` valid result、
  dependency failure 時の従来挙動維持が test で固定される
  (R21-R25)。
- [x] route log store — `maxOutputBytes` 超過 output が
  `<userConfigDir>/route-logs/` に全量残り compact result に
  `truncated` が付くこと、runId-unique + temp+rename 命名、
  size/count/TTL retention、storage failure 時の exit status 保持
  が test で固定される(R26-R29)。
- [x] `cheaper route logs <id>` が bounded read(offset/maxBytes)
  を提供し、workspace 外 generic file read 経路が存在しない
  ことが test で固定される(R30/R31)。
- [x] `kind:"route"` record が規定 field 群を持ち、best-effort
  append(記録失敗が exit code/stdout を変えない)であることと、
  savings estimated 集計へ混入しないことが test で固定される
  (R34/R35)。
- [x] hook rewrite — claude/devin payload で runnable →
  `updatedInput` のみの応答(`permissionDecision` 無し・exit 0)、
  non-runnable / non-match → 従来評価の Allow/Deny を維持、
  windsurf・codex・cursor・
  copilot では Rewrite verdict を発行しないことが test で固定
  される(R36-R39)。
- [x] `VariantInputs::Shell` が `routes.yaml` hash を含み、
  routes.yaml 変更で cached verdict が invalidate されることが
  test で固定される(R40)。
- [x] user-facing doc に routes.yaml 記法・routing≠
  authorization・trust 境界・hook env ≠ shell env・fallback
  挙動が記述されていることが確認される。

## Deferred Design Concerns

- scoped log read の正確な形(`artifact://` URI 拡張か
  invocation-scoped read grant か) | discovery の open
  question | design で確定 | 境界を広く取ると workspace 外 file
  access が開く。
- Claude `updatedInput` silent-ignore bug(#81340)への対応
  (version 検出・doctor verb) | upstream 不具合への検出手段が
  未確定 | 実装時または follow-up issue | silent-ignore されると
  rewrite が透過的に不発となり routing coverage が落ちる。
- log retention の default 値・巨大 log の token 計測方法
  (streaming vs cap) | 実装値の確定が必要 | design で確定 |
  過小 retention で参照不能・過大で disk 圧迫。
- Codex/Cursor/Copilot への rewrite | approval-bypass 検証が必要 |
  issue #150 | 検証前に発行すると上流 approval を素通しする。
- 複合 shell command への routing | rewrite-safe gate は単一
  segment のみ許容する現行境界を維持 | 需要が確認された時点 |
  gate を緩めると parse 誤判定による誤 rewrite リスク。
- `kind:exec` orchestrator / workflow 非依存 executor | workflow
  主実行経路の確定判断(#142)に反する | workflow で表現不能な
  routed 実行要件が出た時点 | orchestrator の二重実装。

## Sources and Relations

- upstream discovery:
  `docs/discovery/pretooluse-auto-routing-to-tasks-and-workflows.md`
  (DISC-034J7LwMEdPR5HxFkgOivZ)— 実測データ・issue #142 確定
  判断 14 項目・alternatives 評価(A-F)・open questions を収録。
  本 spec はその promotion candidate「spec-doc needed」を引き継ぎ、
  route 定義・CLI contract・`when`・`exitCodes`・log store・
  `kind:"route"` record・rewrite 応答を要件化する。
- issue map(parent #142 と実装分割):
  #143 Route Resolver + `routes.yaml` + `route resolve`
  <https://github.com/scarecrow173/cheaper/issues/143>、
  #144 command tool `exitCodes`
  <https://github.com/scarecrow173/cheaper/issues/144>、
  #145 log store + scoped read
  <https://github.com/scarecrow173/cheaper/issues/145>、
  #146 workflow `when`
  <https://github.com/scarecrow173/cheaper/issues/146>、
  #147 `route run` + routed-exec workflow(依存 #143-#146)
  <https://github.com/scarecrow173/cheaper/issues/147>、
  #148 hook Rewrite + Claude/Devin + dedup(依存 #143,#147)
  <https://github.com/scarecrow173/cheaper/issues/148>、
  #149 usage `kind:"route"` + `route logs` + retention + docs
  (依存 #145,#147)
  <https://github.com/scarecrow173/cheaper/issues/149>、
  #150 Codex/Cursor/Copilot rewrite(deferred、依存 #148)
  <https://github.com/scarecrow173/cheaper/issues/150>。
- 関連 doc(`related` 参照): 既存 hook 評価基盤
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  (SPEC-034H2WAU6TQG6N9dmmmKNA — `Rewrite` verdict はこの基盤の
  拡張)、canonical routing policy の由来
  `docs/specs/shell-read-large-file-blocking.md`
  (SPEC-034rBhR3SNIRFZ1B6n26iH — `tokenize_shell` 再利用元)、
  grant 認可層
  `docs/specs/invocation-scoped-command-grants.md`
  (SPEC-034FlOz4hiN9m0nQW7lBN2)、command tool
  `docs/specs/policy-gated-command-tool.md`
  (SPEC-034EMbEGBOWM4CAM6YSWm6 — `exitCodes` 拡張面)、dedup
  `docs/specs/hook-duplicate-firing-single-evaluation-dedup.md`
  (SPEC-034Ie1ZEdUR2aBFVoqkBay — `VariantInputs` 拡張面)、usage
  record
  `docs/specs/delegation-usage-measurement-log.md`
  (SPEC-034r05iHK8bA7gveeDA5HC — `kind:"route"` 追加先)、
  workflow engine
  `docs/specs/one-shot-workflow-engine-and-run-cli.md`
  (SPEC-034AqzLDP7xm2lCmDoyKzg — `when` 拡張面)、hook 登録
  lifecycle
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  (ADR-034H2WBNtC4We8dyC5lQZJ — rewrite deploy 形態の境界)。
- 補助文脈(`references` 参照): usage 集計
  `docs/specs/usage-and-savings-telemetry-aggregation.md`、token
  計測 `docs/specs/canonical-token-counter.md`、artifact store
  `docs/specs/run-scoped-artifact-store.md`、workflow file/compile
  contract `docs/specs/workflow-file-interface.md` /
  `docs/specs/workflow-contract-compiler.md`、前提 discovery
  `docs/discovery/invocation-scoped-command-grants.md` /
  `docs/discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md`
  / `docs/discovery/usage-and-savings-telemetry-for-delegation-reduction.md`。
- code evidence:
  `crates/cheaper-cli/src/hook/routing_policy.rs`
  (`Verdict` :116、`RoutingRequest::Shell` :102、`tokenize_shell`
  :1093、`command_name` :1294、cwd-changing set :1309)、
  `crates/cheaper-cli/src/hook/targets.rs`(per-target render —
  `pretooluse_render` :492、`devin_render` :545、`windsurf_render`
  :182、`cursor_render` :609、`verdict_name` :619)、
  `crates/cheaper-cli/src/hook/eval_cache.rs`(`VariantInputs`
  :301/:314、`FingerprintInput` :364)、
  `crates/cheaper-cli/src/usage_log.rs`(`kind:"run"` :121/:138、
  `kind:"count"` :107、best-effort append :179)、
  `crates/cheaper-cli/src/grant.rs`・`crates/cheaper-cli/src/policy.rs`
  (invocation/user grant 経路)、
  `crates/cheaper-core/src/policy/load.rs`(層 merge と project
  `command` section 拒否 :229-247)、
  `crates/cheaper-core/src/policy/command.rs`(`CommandGrant`
  :37、`command.grants` schema :78-101)、
  `crates/cheaper-tools/src/tools/command/mod.rs`(details schema
  :86-110、grant 解決→pin 検証→proc::run :385-467)、
  `crates/cheaper-tools/src/tools/proc/mod.rs`(`minimal_env`
  :164、`ProcLimits` :120)、
  `crates/cheaper-tools/src/artifact.rs`(`ArtifactReader`
  positioned read :82-88)、
  `crates/cheaper-workflow/src/engine.rs`(`JobStatus::Skipped`
  :174、DEPENDENCY_FAILED :649-658、dep 伝播 :541-547、workflow
  error 集約 :1051-1060)、
  `crates/cheaper-workflow/src/schema.rs`(job schema :75-88、
  binding shape :23-48)、
  `crates/cheaper-core/src/config/dirs.rs`(project dir =
  `<cwd>/.cheaper` :64-71)、
  `crates/cheaper-core/src/fs_util.rs`(`atomic_write` :34)。

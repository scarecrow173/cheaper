---
id: "DESIGN-034JbqoCWm85pKC033fgfd"
type: "design"
status: "approved"
title: "PreToolUse route resolver and delegated workflow execution"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
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
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/logs.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/payload.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/condense.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/condense.input.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/condense.output.schema.json"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/embedded.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/monitor.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/schema.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/types.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/bindings.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/compiler.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/engine.rs"
      - type: "file"
        path: "packages/apm-hook-codex/.apm/hooks/hooks.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "./policy-gated-command-tool.md"
    - "./invocation-scoped-command-grants.md"
    - "./one-shot-workflow-execution-and-cli-composition.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "./documentation-honesty-sweep-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  derived-by:
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  verifies: []
  verified-by: []
  references:
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/src/usage_log.rs"
    - "../../crates/cheaper-cli/src/workflow_run.rs"
    - "../../crates/cheaper-cli/src/embedded.rs"
    - "../../crates/cheaper-cli/src/grant.rs"
    - "../../crates/cheaper-core/src/policy/command.rs"
    - "../../crates/cheaper-core/src/policy/load.rs"
    - "../../crates/cheaper-tools/src/tools/command/mod.rs"
    - "../../crates/cheaper-tools/src/tools/proc/mod.rs"
    - "../../crates/cheaper-tools/src/artifact.rs"
    - "../../crates/cheaper-workflow/src/schema.rs"
    - "../../crates/cheaper-workflow/src/engine.rs"
    - "../../crates/cheaper-core/src/fs_util.rs"
  defers: []
  deferred-by: []
---

# PreToolUse route resolver and delegated workflow execution

## Context

manual opt-in な delegation では coverage が致命的に低い
(DISC-034J7LwMEdPR5HxFkgOivZ: hook 発火 12/238 session、効果自体は
13.6M tokens block で実証済み)。本 design は
SPEC-034J7SxWvGL3oSbeBpo8a3(R1-R40)と
ADR-034J8eZHNPqARi25HLFOL6(D1-D13)を実装構造へ展開し、PreToolUse
hook が shell command を登録済み route へ自動 routing する機構を
定義する。routed 実行は専用 orchestrator を持たず `target.path` の
登録済み workflow の in-process 起動(D1/D12)であり、route 宣言は
認可を含まない(D2 — viability は trusted grant の argv match のみ
が決める)。

既存の単一保守面に乗せる方針(ADR Decision Drivers「reuse over
duplication」)に従い、route 解決は `routing_policy.rs` の
`tokenize_shell`/`command_name`/`CWD_CHANGING_COMMANDS` を再利用し、
実行は `workflow_run.rs` の Shared/ports composition と同一形、log
write は `fs_util::atomic_write` の temp+rename 規約、usage 記録は
`usage_log.rs` の best-effort append 経路を流用する。並立する第二
経路(専用 executor・generic workspace 外 read・実行 dedup)は作ら
ない。

## Scope

### In Scope

- `crates/cheaper-cli/src/route/` 新規 module —
  `routes.rs`(routes.yaml schema・load・3 層解決・trust 評価)、
  `resolve.rs`(`route resolve` dry-run + 共通 resolve 入口)、
  `run.rs`(`route run` — payload decode・revalidate・workflow
  in-process 起動・exit-code mirror)、`logs.rs`(`route logs <id>`
  bounded read)、`log_store.rs`(route-logs store・retention)、
  `payload.rs`(payload schema・base64url codec・allowlist
  subsetting の単一 owner — encoder(hook eval)と decoder
  (route run)の schema skew を防ぐ、B#6)。
- `hook/mod.rs` — hook entry で effective routes + command policy
  を 1 firing 1 回 load し `HookDeps` 経由で eval へ供給
  (`EvaluateOptions` 拡張)。
- `routing_policy.rs` — `Verdict::Rewrite{command, fallback}` 追加と
  `evaluate_shell` への route evaluation arm の組込(runnable →
  Rewrite、それ以外 → 従来の read-candidate 評価へ継続)。
- `targets.rs` — `HookTarget::Codex` variant・pretooluse(claude)・
  devin の `updatedInput` のみの render、codex/windsurf/cursor/
  copilot の Rewrite → `fallback` verdict render、`verdict_name`
  への `"rewrite"` 追加。`packages/apm-hook-codex` の hooks.json
  を `--target codex` へ変更。
- `eval_cache.rs` — `VariantInputs::Shell` への effective
  `routes.yaml` content hash 追加。
- `usage_log.rs` — `kind:"route"` / `kind:"route-refetch"` record
  構築。
- `args.rs` / `cli.rs` / `lib.rs` — `route` subcommand
  (`resolve`/`run`/`logs`)の wiring(`main.rs` は dispatch 外の
  `__grep-eval` 分岐のみで変更不要)。`workflow_run.rs`・
  `invocation.rs`・`registry.rs` — route context(options 注入・
  単一 grant 投影・`RouteLogReader` artifact port)の plumbing。
  `usage_report.rs` — `kind:"route"`/`"route-refetch"` の集計
  join。
- `embedded.rs` + `resources/` — builtin `routes.yaml`・
  routed-exec workflow・`condense` worker の embed・materialize。
  `extract_builtin`/`embedded_fingerprint` の resource 列挙を
  `routes.yaml`・`workflows/` へ対称拡張する(列挙漏れは
  `.ready` materialized dir が新規 file を欠く staleness を
  生む — B#9)。
- `cheaper-core/policy/command.rs` — `command.grants.<name>.exitCodes`
  schema(default `[0]`)。
- `cheaper-tools` — command tool の `exitCodes` result 化・
  `accept_any_exit_code`/`extra_env`/`exec_cwd`/`log_tee` scope
  option・route mode の `outputTokens`/`logRef` details field、
  proc の全量 output tee sink(任意 option、monitor.rs の stream
  読み取り層)。
- `cheaper-workflow` — job `when`(predicate schema・compile の dep
  edge 推論・engine の condition-skip・`$from`→`null`
  materialization・workflow error 集約からの除外)。
- user-facing doc(routes.yaml 記法・routing≠authorization・trust
  境界・hook env ≠ shell env・exit-code domain・fallback 挙動)。
- contract/integration test(AGENTS.md の test 配置規約 —
  `tests/integration/` module + `main.rs` への `mod` 宣言、
  runtime/workflow は `tests/contracts/`・`tests/compile_contracts/`)。

### Out of Scope

- Codex / Cursor / Copilot への Rewrite verdict 発行(issue #150 —
  approval-bypass 検証後)。windsurf の rewrite 対応(非対応 —
  恒久的に Allow fallback)。
- 複合 shell command への routing(rewrite-safe gate の単一
  segment 境界を維持)。
- 汎用 workspace 外 file read grant(runId-addressed scoped read の
  み)。
- `kind:exec` orchestrator・workflow 非依存 executor。
- worker からの recursive delegation。
- Claude `updatedInput` silent-ignore(#81340)の version 検出・
  doctor verb(follow-up)。
- log read の `artifact://` URI scheme 統合 — `RouteLogReader` は
  `ArtifactReader` trait の impl として ports へ装着するが、
  `artifact://run-logs/…` のような URI 解決経路は作らない
  (runId-addressed scoped read に留める、R31)。
- user 層による project route の consent list — `trusted: true` は
  explicitness の mechanism であり consent は別仕組み(ADR D7)。

## Components and Boundaries

| Component | 責務 | 新規/変更 |
| --- | --- | --- |
| `route::routes` | routes.yaml schema 検証・3 層 load・effective route set 構築(層 merge + `trusted` shadow 評価 + 同一 `match.command` 重複の `invalid-route` 化) | 新規 |
| `route::resolve` | argv → route match(prefix/longest/self-bypass)→ viability 評価の純粋評価入口(`ResolveInput` で routes/command_policy を受領)。hook eval・`route resolve`・`route run` の 3 caller が共有する唯一の resolve 実装 | 新規 |
| `route::payload` | payload schema + base64url codec + allowlist subsetting の単一 owner(encoder/decoder の skew 防止) | 新規 |
| `route::run` | payload decode → re-resolve + revalidate → routed workflow in-process 起動 → exit-code mirror + usage/log 記録 | 新規 |
| `route::logs` | `route logs <id>` — runId-addressed bounded read | 新規 |
| `route::log_store` | `<userConfigDir>/route-logs/` への streaming temp write + rename、size/count/TTL retention sweep、runId 採番 | 新規 |
| `hook/mod.rs` | hook entry での effective routes + command policy load(1 firing 1 回、`HookDeps` 経由) | 変更 |
| `routing_policy` | `Verdict::Rewrite` variant・`evaluate_shell` 先頭の route evaluation arm | 変更 |
| `targets` | `HookTarget::Codex` variant・Rewrite の per-target render(claude pretooluse/devin = `updatedInput` のみ、codex/windsurf/cursor/copilot = `fallback` verdict render) | 変更 |
| `apm-hook-codex` hooks.json | codex 登録 command を `--target codex` へ分離 | 変更 |
| `eval_cache` | `VariantInputs::Shell` への routes hash 追加 | 変更 |
| `usage_log` | `kind:"route"`/`"route-refetch"` record builder | 変更 |
| `command` tool | `exitCodes` opt-in result 化・route scope option(`accept_any_exit_code`/`extra_env`/`exec_cwd`/`log_tee`)・route mode `outputTokens`/`logRef` details | 変更 |
| `proc`(`monitor.rs`) | 全量 output tee sink(optional) | 変更 |
| `invocation`/`registry`/`workflow_run` | route context の scope 注入 plumbing・単一 grant 投影・`RouteLogReader` artifact port | 変更 |
| `usage_report` | `kind:"route"`/`"route-refetch"` の独立集計 bucket | 変更 |
| `workflow` | job `when` schema + engine の condition-skip | 変更 |
| `routes.yaml`(builtin)・`routed-exec.workflow.json`・`condense` worker(md + schemas) | route 宣言と routed 実行の参照 workflow + output 縮約 worker | 新規 resource |

境界の原則: route 宣言(`routes.rs`)は「どの argv がどの
workflow へ行くか」の宣言解決のみ、認可は `command.grants`
(grant match は `resolve_command_grant` + `ArgPattern::compile`)
のみ、実行は workflow engine のみ、log/usage 記録は post-processing
のみが担う。routing 層は raw command を実行しない — revalidation
失敗は command 未実行の cheaper error となる(R14)。

## Detailed Design

### routes.yaml schema と 3 層解決(`route::routes`)

`Route` raw shape:

```yaml
version: 1
routes:
  - name: cargo-test
    match: {command: [cargo, test]}
    target: {kind: workflow, path: routed-exec.workflow.json}
    grant: cargo-test
    output: {tokenThreshold: 8192}
    passthroughEnv: [CI]
    trusted: true            # project 層のみ意味を持つ(R7、D7)
```

- load は fail-loud(R2): malformed YAML・未知 key・required
  (`name`/`match`/`target`/`grant`)欠落・`match.command` 空・
  `kind` 非 `"workflow"` は「invalid policy is never silently
  ignored」の `load_policy` 規約と同型に config error
  (`invalid-route`)— 壊れた route を drop しない。
- 層: builtin(`<builtin_dir>/routes.yaml`、embedded materialize)
  → user(`<userConfigDir>/routes.yaml`)→
  project(`<workspace>/.cheaper/routes.yaml`)。各層は独立 file を
  load し、effective route set は builtin → user → project の順に
  `match.command` token 列で key 化して構築する。
- merge 規則: 同一 `match.command` では上位層が下位層を shadow
  する(user は builtin を無条件で shadow)。例外は project 層 —
  project route が下位層 route と同一 `match.command` を持つ場合、
  `trusted: true` 宣言があるときのみ shadow し、無い場合はその
  `match.command` では inactivate されて下位層 route が勝つ(D7)。
  抑制された project route は effective set から外すが resolve の
  診断へ `suppressed` provenance として残す(静かに無視しない)。
- 同一層内の同一 `match.command` 重複・effective set 内に残った
  同一 `match.command` の有効 route 2 件は `invalid-route`
  (R6)とする — silent last-wins/first-wins は禁止。
- `trusted` field は project 層 route にのみ意味を持ち、
  builtin/user 層での宣言は効果を持たない(inert — forward
  compatible に「`trusted` + user 側許可」へ強化可能、ADR D7)。
- `target.path` は宣言層起点で解決する: builtin route →
  `<builtin_dir>/workflows/` 相対、user route → `<userConfigDir>/`
  相対、project route → `<workspace>/.cheaper/` 相対。層起点を
  固定することで project route が workspace 外の任意 file を
  target に指せない。さらに `target.path` は層起点からの
  **relative path のみ**許容し、`..` segment・absolute path・
  non-UTF8 を含む宣言は `invalid-route` — `../outside/x.json`
  のような escape で層境界を潜る project route を構造的に
  拒否する(C10 系の traversal 対策と同じ規則)。

### RouteResolver(`route::resolve`)— 共通 resolve 入口

```rust
pub struct RouteResolution {
    pub matched: bool,
    pub route: Option<String>,       // match した route 名
    pub target: Option<String>,      // 解決済み workflow path
    pub grant: Option<String>,       // 参照 grant 名
    pub executable: Option<String>,  // argv[0] の command_name
    pub argv: Vec<String>,           // env-prefix 除く argv 列
    pub args: Vec<String>,           // argv[len(match.command)..] — R4/R11 表示用
    pub exec_args: Vec<String>,      // argv[1..] — spawn argv / grant 照合の基準
    pub viability: Option<Viability>,
    pub suppressed: Option<String>,  // trust 抑制された project route 名
}

pub enum Viability {
    Runnable,
    NoGrant,
    MissingTarget,
    InvalidRoute,
    UnsafeCommand,
    StdinUnsupported,
}

pub struct ResolveInput<'a> {
    pub argv: &'a [String],
    pub cwd: &'a str,
    pub routes: &'a EffectiveRoutes,          // routes::load 済み 3 層解決結果
    pub command_policy: &'a CommandPolicyView, // effective command.grants の参照面
}
```

**入力供給(hook eval での grants 経路 — A#10 修正)**: viability
の grant-match は effective `command.grants` を要するが、現行の
hook eval 経路(`EvaluateOptions{env, builtin_dir}`)は policy を
一切 load しない。そこで resolve は `routes`/`command_policy` を
**引数として受ける純粋評価**とし、load 責務は caller 側へ置く:

- `route::routes::load(cwd) -> EffectiveRoutes` — builtin embedded +
  `<userConfigDir>/routes.yaml` + `<cwd>/.cheaper/routes.yaml` の
  3 層を読み merge する(file read のみ、bounded)。
- command policy は既存 `load_policy` 系の層解決(builtin + user +
  project `.cheaper`)で `command` section のみを解決する shared
  loader を用いる — `route run` の policy load と同一規約。
- hook eval では firing 1 回につき hook entry(`hook/mod.rs::run`
  → targets dispatch)でこれらを load し `HookDeps`/`EvaluateOptions`
  経由で `evaluate_shell` へ渡す — eval 内で file I/O を分散させ
  ず、dedup verdict cache 命中時は load 自体が skip される
  (routes hash が fingerprint 入力のため cache key 計算段で
  routes file のみ読む — policy file は cache miss 時の eval で
  初めて必要になるが、単純化のため firing ごと load とし、
  policy file read コストは firing あたり 2-3 file の bounded
  read に留まる)。load 自体の失敗(file 欠落/parse error)は
  `invalid-route`/`no-grant` viability へ写像し、hook eval を
  失敗させない(fail-safe — routing 層の不備が親の command 実行
  を阻害しない、R39)。
- `route resolve`/`route run` も同じ loader を使う — 3 caller が
  同一の effective set を見る。

resolve 手順(hook eval・`route resolve`・`route run` が共有):

1. self-bypass(R9)— `command_name(argv[0])` が `cheaper` に一致
   する command は non-match(match 評価自体をしない)。
2. argv prefix match(R4)— `match.command` 全 token が argv 先頭と
   一致する route 集合。複数 match は token 数最大(longest
   match、R5)を採用。同一 token 数の複数 match は `invalid-route`
   (構築時の重複検査と同一 domain)。
   `args`/`exec_args` の区別: `args` は spec R4 の prefix 後ろの残
   token(`argv[len(match.command)..]`、resolve 出力・payload の
   表示用 field)、`exec_args` は `argv[1..]` — spawn される
   `executable + exec_args` と grant `argv` pattern 照合の基準。
   multi-token `match.command`(`[cargo, test]`)では prefix 末尾
   token が `args` から落ちるが `exec_args` には残るため、exec
   job には必ず `exec_args` を渡す(`args` を渡すと
   `cargo test --workspace` が `cargo --workspace` として実行
   される)。grant pattern は spawn argv 基準(例 `["test","*"]`)。
3. viability 評価 — 固定優先順 `unsafe-command → invalid-route →
   missing-target → no-grant → {stdin-unsupported | approval-required
   | runnable}`(R12)で最初に成立した値を 1 値として返す。最終段は
   executable + argv 形状の tri-state 分類(`classify_interaction`
   — `Structural`/`Approval`/`Batch` の最悪階層を typed 側・
   substitute 宣言名/pin 解決名・byte-matched `route.grant` の
   executable 名の union で採る)で、3 値は同じ段から排他的に
   出る(R33/R33bis):
   - `unsafe-command`: `tokenize_shell` の segment 数 ≠ 1・
     `unsafe_segment`・`unsafe_token` 含有・`command_name` が
     `CWD_CHANGING_COMMANDS` 該当(R8)。--command 由来の shell
     文字列を tokenize できない場合もここ。
   - `invalid-route`: route file の config error・同一
     `match.command` 重複(R2/R6)。
   - `missing-target`: 解決済み `target.path` が存在しない・
     load 不能。
   - `no-grant`: route の `grant` 名が effective
     `command.grants` に存在しない、`exec_args` が grant の
     `argv` pattern に match しない(`resolve_command_grant` +
     `ArgPattern::compile`)、または `command::execute` が実行前に
     課す事前 check(executable の pin 解決・denied executable・
     workspace residency・`prepare_cwd` の denylist/stat)に
     落ちる場合 — viability は「`command::execute` が受理するか」
     の事前評価として同じ predicate を共有し、`runnable` と
     判定した command が exec 段で policy/IO 失敗する乖離を
     作らない(routing≠authorization、D2)。実現手段: これらの
     事前 check は `cheaper-cli` から見える `pub` 面に無い
     (`pin_command_grants`・`prepare_cwd`・`minimal_env`・
     `native_executable` は `pub(crate)`、B#3)。cheaper-tools
     `command` module に **`pub fn check_command_execution`**
     (仮名 — grant 解決・pin・denied executable・cwd 事前
     検査を `command::execute` と同一 predicate で行う軽量
     entry、実 spawn はしない)を公開し、`command::execute` 本体
     と `route::resolve` の双方がこの entry を呼ぶ — predicate
     の複製(DRY 違反・恒久的ドリフト)を構造的に防ぐ。
     `exec_args` の shell
     metachar(`argv_shell_re`)該当は `unsafe-command` 側に分類
     する(token 化済み argv であっても grant 側は metachar を
     拒否するため、実行不可の安全性問題として扱う)。
   - `stdin-unsupported`: stdin/TTY 非接続では構造的に実行
     不能な形状 — UI/TTY app(editors・pagers・`top`/`watch`・
     `ftp`/`telnet` 等)、bare REPL/shell(`python`・`node`・
     `bash` 等に positional program が無い)、stdin program
     指示(`-`)、`docker`/`kubectl`/`podman` 等 container CLI の
     stdin/TTY flag(`-it`/`-ti` permutation・`-i`/`-t` 含有
     cluster・`--interactive`/`--tty`/`--stdin` の `=false`
     でない形)。検出は executable family 判定(version suffix
     `python3.11`・`node20` 等を吸収)+ argv scan —
     REPL/interpreter の structural intent(`-NoExit`/
     `-Interactive` 等の pwsh prefix・slash 形を含む)と
     bare session、container flag で最悪階層を採る。判定不能な
     stdin 依存 command は構造的に stdin 非接続で安全側に
     倒れる(R33 — 網羅性ではなく構造保証に依る)。
   - `approval-required`: stdin 非接続でも spawn 可能だが
     interactive intent・inline code・特権/remote 副作用を持つ
     — REPL/shell の inline code(`-c`/`-e`/`-lc` 等の
     grouped/attached 形・`--eval`/`--print`・pwsh
     `-Command`/`-EncodedCommand` の unambiguous prefix と
     `/Command` slash 形・`cmd /c`/`/k`)、interactive intent
     (`-i` 等)、positional program 文字列(`awk`/`sed`/DB CLI
     の query/expr — `-f FILE` 指定なら `Batch`)、
     `ssh`/`sudo`/`su` 系の特権/remote、launcher wrapper
     (`env`・`timeout`・`xargs`・`exec`・`find -exec` 等)の
     inner command 再帰結果。hook は Rewrite を emit するが
     `permissionDecision:"allow"` は一切併記せず host 承認
     prompt を残す(R33bis — `route.rewrite.approvalBypass`
     opt-in があってもこの tier には適用しない)。codex は
     「承認付き置換」を表現できないため fallback verdict へ
     倒す。`route run` 側は `runnable` と同じく実行受理する。
   - `runnable`: 上記いずれも不成立(batch 形状 —
     `python check.py`・`node build.js`・`pwsh -File x.ps1`
     等の script 付き interpreter 起動を含む)。

### hook eval への Rewrite arm(`routing_policy`)

`Verdict` enum(:116)に
`Rewrite{command: String, fallback: Box<Verdict>}` variant を追加。
wire は internally-tagged serde 規約どおり
`{"decision":"rewrite","command":"…","fallback":{…}}`(R36 —
fallback は同じ tagged shape を入れ子にする)。`fallback` は routing
層を通さなかった場合にその command が得た verdict(Allow または
read-candidate Deny)で、non-rewrite target の render がこれを
採用する(C6 — Rewrite→Allow 無条件降格だと `cat huge` 系の
read-candidate Deny が windsurf/codex 系で素通しになるため)。

`evaluate_shell`(:1407)の先頭へ route evaluation arm を組み込む:

```text
evaluate_shell(command, cwd, opts):
  if let Some(segments) = tokenize_shell(command):
    if segments.len() == 1 && segment is safe:
      env_prefix = 先頭の is_env_prefix token 列(KEY=VALUE 分解)
      argv = 残り token の text 列
        (extract_read_candidates と同じ先頭 env-prefix skip 規約 —
          env prefix は argv に含めない)
      res = route::resolve::resolve(argv, cwd, opts)
      if res.matched && res.viability == Runnable:
        fallback = 従来の read-candidate 評価(下の loop と同一)
        return Verdict::Rewrite {
          command: build_route_run_command(res, command, cwd, env),
          fallback: Box::new(fallback),
        }
  // runnable でない・match しない・gate 不成立 → 従来経路
  for candidate in extract_read_candidates(command): …(現行どおり)
```

- eval は target 非依存 — `source_tool` を verdict 分岐に使わない
  (canonical verdict + dedup の target 非依存 fingerprint を維持、
  D3/D10)。rewrite 非対応 target への降格は render 層が
  `fallback` を使って行う。
- Rewrite verdict は dedup verdict cache へ publish/serve しない
  (verdict record に serialize されると payload 内の capture env
  値が `<userConfigDir>/hook-eval/<fp>.json` へ 60s TTL で残り
  R32 の非永続化意図を破るため — Rewrite は評価のたびに
  resolve を実行する。Allow/Deny の dedup は従来どおり)。
- env-prefix(`FOO=1 cmd` の leading `KEY=VALUE` token)は argv
  からは除外し、payload には載せない — `route run` が
  `payload.command` を再 tokenize して再導出する(command 宣言の
  env は transport 不要であり、transport すると改竄 payload で
  allowlist 外 env を密輸できる — C3)。hook ambient env は
  `passthroughEnv` allowlist で capture して payload `env` に
  載せる(spec R13 の field 意味をそのまま維持)。子 env への
  merge 優先順位は `env-prefix(command 宣言) > ambient >
  grant.env` — command 宣言が最も具体的意図。いずれの source も
  子 process env 以外の経路へは載せない(R32)。
- rewrite command 構築: `<cheaper> route run --payload <b64>` の
  単一 segment。executable は `std::env::current_exe()` 由来の
  hook 実行 binary の absolute path — 親 shell の PATH 解決差・
  別 version binary による payload format ドリフトを避ける
  (space 含み path は tokenize 可能な単一 token として emit —
  rewrite 先も単一 segment 安全形)。payload は
  `{command, executable, argv, args, cwd, env}` の JSON を
  base64url(padding 無し)で encode した単一 argv token
  (design で確定 — 生 JSON の quote 回避は cmd/powershell/bash の
  quoting 差で脆弱なため opaque token とする)。`env` には
  `passthroughEnv` allowlist で capture した hook ambient 値
  のみ入る — env-prefix は transport せず `route run` が
  `command` から再導出する(上記 C3)。`args` は R4 の
  post-prefix 表示用で、exec には payload `argv` から
  `argv[1..]` を再導出して使う(C1)。
- Rewrite verdict は hook env の `CliIo.env` から allowlist 値を
  capture する — allowlist 外は payload に載せない。

### render(targets.rs)と target 分離

claude/codex の discriminator は **`--target` 値そのもの**とする —
`apm-hook-claude`/`apm-hook-codex` の hooks.json は現行ともに
`--target pretooluse` を登録しており(各 package の
`.apm/hooks/hooks.json:10`)、`tool_name` payload field は tool 名
(`"bash"`等)で agent 名ではないため、pretooluse 内部では
claude/codex を区別できない(C7 — `tool_name` discriminator は
構造的に成立しない)。そこで target を分離する:

- `HookTarget::Codex` variant を追加(`as_str() == "codex"`)、
  `packages/apm-hook-codex/.apm/hooks/hooks.json` の登録 command
  を `--target pretooluse` → `--target codex` へ変更する。
  codex payload は claude と同一 shape のため normalize は
  `pretooluse_normalize` をそのまま共有し、render のみ codex 専用
  arm とする。`HookTarget::Pretooluse` はこの変更後 claude 専用
  の意味になる。
- `pretooluse_render`(claude): Rewrite →
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"command":"<rewritten>"}}}`
  を stdout emit + exit 0(`permissionDecision` は emit しない、
  D3)。
- `codex_render`: Rewrite → `fallback` verdict を codex の応答
  規約で render(R38 の approval-bypass 検証前のため
  `updatedInput` を発行しない — 構造的に codex へ rewrite が
  届かない)。
- `devin_render`: Rewrite →
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"command":"<rewritten>"}}}`
  + exit 0。E の実装時に公式 Devin hook docs と照合し、当初の
  未検証な top-level `updatedInput` 例を訂正した。Rewrite は
  `decision` / `permissionDecision` を持たず、既存 Deny の
  top-level `decision:"block"`・Allow の silent 応答は不変。
  Claude と同じ wire shape の Rewrite helper を共有する。
- windsurf/cursor/copilot render: Rewrite → `fallback` を
  各 target の応答規約で render(fallback が Deny なら従来の
  deny 応答、Allow なら allow 応答 — read-candidate の context
  guard が route 存在下でも維持される、C6)。
- `verdict_name` に `Rewrite => "rewrite"` を追加(observability
  の verdict 分類値)。

### `route resolve`(route::resolve CLI arm)

- `cheaper route resolve --command <string>` または
  `--argv <json-array>` のいずれか必須(R10)。`--argv` でも
  `--command` と同じく先頭の env-prefix token(`KEY=VALUE` 形)を
  skip して argv を正規化する — 2 経路で resolve 結果が
  乖離しない。dry-run — command・workflow・外部 process を
  一切実行しない。
- 出力 envelope(result): `{matched, route?, target?, grant?,
  executable?, argv, args, viability?, suppressed?}`(R11 +
  suppressed provenance)。`matched:false` では route/target/
  grant/viability を持たない。
- utility command 規約(`policy`/`hook-config` と同じ) —
  `record_usage` 非対象。

### `route run`(route::run)— payload → revalidate → in-process workflow

入力: `cheaper route run --payload <b64>`。decode した
`{command, executable, argv, args, cwd, env}` に対し:

1. **re-resolve + revalidate(R14)** — payload の `argv` で
   `route::resolve` を再評価(rewrite-safe gate・route 有効性・
   `exec_args = argv[1..]` 基準の grant match・target 存在・
   `env` ⊆ `passthroughEnv` allowlist の再確認)。`executable`/
   `args` は診断・表示用で実行には使わず、`env-prefix` は
   `payload.command` の再 tokenize で再導出する(payload の
   env 値は ambient allowlist capture のみ — command 宣言の
   env-prefix を transport しないため、改竄 payload で
   allowlist 外 env は注入できない)。payload 由来の resolve
   結果を信用しない(TOCTOU・改竄対策)。失敗時は原 command を
   実行せず cheaper error(code + exitCode)で終了 — routing 層
   内で raw command を fallback 実行しない(grant 境界の迂回を
   防ぐ)。
2. **scope 構築時の route context 注入** — routed 実行の
   invocation は `CommandToolOptions` へ route context を注入する
   (invocation runtime options → registry ctx →
   `ctx.command()` — registry.rs:311 の同一経路):
   - `extra_env: Map<String, Value>` — `env-prefix`(payload
     `command` 再導出)+ ambient `env`(allowlist 再確認済み)を
     優先順位 `env-prefix > ambient` で合成した子 env 上乗せ
     分。`command::execute` が `minimal_env(&grant.env)` 構築時に
     畳み込み、優先順位は `env-prefix > ambient > grant.env`。
   - `exec_cwd: Option<String>` — payload `cwd`(正規化)を
     子 process cwd として直接使う override(R18)。grant.cwd
     経路を bypass し、存在確認 + denylist 検査のみ行う
     (workspace containment は課さない — 原 command がその
     cwd で動くはずだったという user intent を復元する)。
   - `accept_any_exit_code: bool` — routed scope では任意の
     completed exit を result data 化する(user grant の
     `exitCodes` 設定に依らず exit-code mirror を成立させる
     — C2。spawn 失敗・timeout・abort は従来どおり Err で
     R20 と整合)。
   - `log_tee: Option<ProcTee>` — 全量 output tee(下記 proc §)。
   さらに **解決済み `RuntimePolicy.command.grants` を route の
   grant 1 件だけへ投影する** — `load_policy` で得た resolved
   policy の `command.grants` map を、Shared/invocation 構築
   **前**に `{<route.grant>: <resolved spec>}` へ置き換える。
   routed workflow の command job が `input.name` で参照できる
   grant を route 宣言の grant に限定し、project 由来 workflow
   が他の user grant を勝手に呼ぶ confused deputy を構造的に
   防ぐ(C9。trusted: true の self-declared でも爆発半径は
   宣言 grant の argv pattern に閉じる)。
   投影点は resolved policy であり `InvocationToolRuntimeOptions
   .command` の raw JSON 注入ではない(B#2 — worker scope 生成は
   `options.runtime.command` を `deps.policy.command` から再
   serialize し parity check(invocation.rs:232-238)で
   `inner.command` と比較する。投影を options 注入にすると
   parity が崩れて `kind: worker` job が全滅する。resolved
   policy 側を投影すれば invocation serialize と worker-scope
   serialize が同一の投影済み policy を見るため parity は
   自動的に保たれる)。`load_policy` の `invocation` 引数
   (grants-only・additive・同名 collision 拒否、load.rs:119-185)
   も使わない — additive merge は他 grant を除去できず単一
   投影を表現できない。
   passthrough 値は options → 子 process env のみに流れ、
   log・usage・worker input には載らない(R32 不変条件)。
3. **workflow in-process 起動(D12)** —
   `workflow_run::execute_run` と同じ Shared/ports
   composition(load policy → load workflow `target.path` →
   compile with inputs → `run_workflow`)。context root は
   `route run` 自身の process cwd — 親 agent が rewrite 済み
   command を実行する cwd は原 command の cwd と同一であり、
   `dirs` の `<cwd>/.cheaper` project 層規約により hook eval
   と同一の層解決に自然に一致する。policy layer は追加しない。
   ただし **workflow file の load root は宣言層 dir** とする
   (B#4) — `load_workflow_file` は `resolve_workspace_path(
   options.root, path)` で workspace 境界を強制するため、
   builtin 層の `target.path`(`<builtin_dir>/workflows/…`)は
   workspace root 起点では escape 扱いで load 不能になる。
   `route run` は層 dir を `options.root` として load する
   経路を使い、routed workflow の compile/実行における
   resource inference の `workspace_root` は引き続き payload
   `cwd` 由来の workspace を指す(2 つの root — workflow file
   自体の layer root と、job 内 file 参照の workspace root —
   を混同しない)。`check_denied`/`CliResourceInference` は
   実行側 workspace root 基準の従来規約を維持する。
   inputs は `{grant, executable, argv, args, execArgs, cwd,
   tokenThreshold, runId}` — `env` は inputs に含めない
   (passthrough 値が workflow input 経由で worker job の
   `$input` へ見えると R32 違反になるため、env は scope
   options の `extra_env` 専用経路)。nested `cheaper run`
   subprocess は使わない(exit-code mirror・signal 伝播・
   usage 記録の単一 process 責務)。run timeout は
   `grant.timeout` + post-processing headroom(例 +120s)を
   上限とする — `execution.timeout`(default 300s)が
   grant timeout を下回ると、direct 実行では完走する長い
   command が routing 下で途中殺されるため、command 実行
   domain の timeout は grant 側が authoritative(C14e)。
4. **exit-code mirror(R15/D11)** — routed scope の
   `accept_any_exit_code` により command の completed exit
   (0・非 0 問わず)は必ず exec job の result data になる。
   routed workflow の outputs に `execResult: {$from: exec}`
   を必須で含め、`route run` は
   `outcome.result.execResult.details.exitCode` をそのまま
   process exit code として mirror する — tool job の格納
   result は `CanonicalToolResult{content, details}` の
   serialize 形(`output_schema` の required が
   `["content","details"]`、catalog.rs)であり、command 由来
   field は全て `details` 階層にある(B#1)。exec 自体が失敗
   (spawn 失敗・timeout・abort・workflow engine error)した
   場合は `execResult` が materialize せず、cheaper error
   (code + exitCode)で終了 — command の失敗と routing infra
   の失敗を別 domain に保つ。exec 成功後の post 段階失敗
   (analyze job 失敗・output materialize error・log/usage
   記録失敗)は `execResult` が既にあるため exit code
   mirror と bounded result emit を維持し、失敗項目を
   `postprocessFailures` として記録する(R16/C8 —
   workflow error の有無ではなく `execResult` の有無で
   mirror 可否を決める)。
5. **post-processing** — compact result(analyze 成功時は
   縮約結果、skip/failure 時は bounded output + logRef)を
   **stdout へ生 text として emit** — rewrite 済み command の
   stdout が親 agent の tool result そのものになるため、
   JSON envelope は `--format json` 指定の direct CLI 呼出
   専用とする(C14c)。log/usage 記録後、post-processing 失敗は
   原 command を再実行せず `postprocessFailures` に記録する
   (R16)。

routed-exec builtin workflow(`resources/workflows/routed-exec.workflow.json`)
の構成(analyze job の `condense` worker は新規 builtin worker —
`resources/workers/` の現行 builtin は bulk-reader・code-writer・
doc-writer・tool-operator の 4 件のみで output 縮約 worker は
存在しないため、`condense.md` + input/output schema を新設する、
B#5):

```text
jobs:
  exec:    kind: tool, tool: command
           input: {name: $input.grant, args: $input.execArgs}
  analyze: kind: worker, worker: condense
           when: {left: {$from: exec, path: /details/outputTokens},
                  op: gt, right: {$input: tokenThreshold}}
           input: {logRef, originalTokens, command}
outputs:
  execResult: {$from: exec}                    # exit mirror + bounded fallback
  result:     exec bounded output + analyze 縮約結果 + logRef
```

- `measure` job は設けない — route log は workspace 外で
  `count-tokens` tool の `resolve_workspace_path` 経路が読めず、
  token 計測は exec job 内部で完結させる(C4)。route mode
  (`log_tee` 有り)の command tool は proc 完了後に temp log の
  byte を bounded read(`count_max_bytes` 上限)して canonical
  tokenizer で `outputTokens` を計測し、`logRef`(runId)・
  `outputTokensTruncated` と共に result details の optional
  field として返す — `command_details_schema` に 3 field を
  optional 追加し、`when` が `{$from: exec,
  path:/details/outputTokens}` を参照できるようにする
  (pointer は `CanonicalToolResult.details` 階層を辿る —
  `additionalProperties:false` の compile 検証を通るのは
  `/details/…` のみ、B#1)。
- `when` の `right` は `{$input: tokenThreshold}` — `$literal`
  wrapper は値をそのまま clone するため binding を入れ子に
  できない(C4 の `$literal:$input` 誤りを修正)。
- analyze worker は `logRef`(runId)を input として受け、
  worker 側 scope の `read` tool が artifact source
  (`{kind:"artifact"}`、artifact ref = runId)で log を読む —
  `route run` が `ArtifactPorts.reader` に `RouteLogReader`
  (artifact ref → runId regex 検証 →
  `<userConfigDir>/route-logs/<runId>.log` を写す
  `ArtifactReader` impl)を装着し、invocation → worker scope へ
  継承させる。ref 検証は `route logs` の id 検証と同じ regex
  で、artifact ref 経由の任意 file read を防ぐ。
  `route logs` CLI も同一 reader の positioned bounded read
  contract を共有する(R30/R31)。
- analyze が condition-skip された場合 outputs は exec の
  bounded output + logRef で materialize する(`$from: analyze`
  は `null` — skip ≠ failure、R23/R25)。exec が失敗した場合は
  `execResult` も欠落し workflow error → `route run` の infra
  error 経路へ。

### command tool `exitCodes`(R19/R20、D5)

- `command_policy_properties`(:78-101)の grant schema に
  `exitCodes: {"type":"array","items":{"type":"integer"}}` を追加し、
  `CommandGrant` resolved shape に `exit_codes: Vec<i64>`(default
  `[0]`)を populate。
- `command::execute`(:381-467)の `proc::run` 呼出し結果を
  inspect: Err の場合 `find_cheaper_error` + details で
  process-completed-nonzero を判定し、`exitCode ∈ grant.exit_codes`
  なら Err ではなく result data `{state, exitCode, stdout, stderr}`
  (観測された details をそのまま — `state` の観測値を書き換えない)
  を `CanonicalToolResult` として返す。
- 判定条件の実 shape(monitor.rs:281-327): process が実際に exit した
  非 0 ケースは `{"state":"failed", "exitCode": <non-null>,
  "spawnCode" 不在}` の `TOOL_EXECUTION_FAILURE` "managed process
  failed"。spawn 失敗は `{"state":"failed", "exitCode": null,
  "spawnCode": <code>}` で区別される — `exitCode` non-null かつ
  `spawnCode` 不在のみが exitCodes 判定対象。exit 0 は Err に
  ならず従来どおり Ok。集合外の非 0・spawn 失敗・timeout・abort は
  従来どおり Err(R20)。
- routed scope はこの grant-declared `exitCodes` を使わず、上記
  route context の `accept_any_exit_code` option で「任意の
  completed exit を result data 化」する(C2 — 原 command の
  exit-code mirror は user grant の `exitCodes` 宣言に依存させ
  ない)。`exitCodes` は spec R19/R20 の general opt-in として
  残り、route 以外の command tool 利用が非 0 exit を workflow
  result として扱える。`accept_any_exit_code` は scope option
  であり grant schema には現れない — routing 内部機構と user
  宣言面を分離する。

### proc の全量 output tee(R26)

route log は `maxOutputBytes` cap と独立の全量 capture を要求する。
`proc::run` の stream 読み取り層(`monitor::read_stream`)に
optional sink を追加する — bounded capture とは別に、chunk 到着順に
raw bytes を log file へ append する tee。

- `proc::run` に optional `tee: Option<ProcTee>` を追加
  (`ProcTee = Arc<dyn Fn(&[u8]) + Send + Sync>`、stdout/stderr
  両 stream 共通の byte sink — stream 区別は log に残さず到着順の
  interleaved single file とする。bounded result 側は従来どおり
  stream 分離)。
- tee は option のため既存 caller(process/command tool)は
  `None` で不変。command tool 側は `CommandToolOptions` に
  `log_tee: Option<ProcTee>` を追加し `route run` 由来の
  scope 構築から注入する(`route run` → invocation runtime
  options → registry ctx → `CommandToolOptions` — scope 構築時に
  route context を持つ invocation のみが tee を持つ。
  `extra_env` と同一 plumbing を共有する)。
- tee write 失敗は capture を失敗させない(postprocessFailure へ
  縮退 — log write の I/O 不具合が command 実行結果を隠蔽しない
  R29)。

### route log store(route::log_store)

- 配置: `<userConfigDir>/route-logs/`(secret を含みうるため
  workspace 外、D6)。
- 命名: `<runId>.log`。runId は `<unix-ms>-<pid>-<hex>` 形式の
  run 単位 unique id(`fs_util::temp_path` の pid+nanos 規約と同型
  の衝突耐性)で、厳格 regex `^[0-9]+-[0-9]+-[0-9a-f]+$` に
  一致する。実行中は `<runId>.log.tmp` へ streaming write し、
  完了時に rename(実行中の不完全 file は最終名で現れない —
  `fs_util::atomic_write` と同じ temp+rename 意味論、ストリーミング
  のため write は逐次 append で rename のみ atomic)。log file は
  unix で `0600` permission — command output は secret を含みうる
  (D6)ため credentials.json と同じ保護水準に揃える。
- `route logs <id>`: `<id>` は `<runId>`(拡張子なし)で、上記
  regex に一致しない値・`/`・`\`・`..`・absolute path を含む値は
  `InvalidRequest` で拒否する — `<id>` → `<route-logs>/<id>.log`
  の untrusted 変換は id 検証なしに行わず、runId-addressed
  scoped read が arbitrary `*.log` read へ拡大しない(R31/C10)。
  `--offset`/`--max-bytes` の bounded read(offset は byte 基準、
  `max_read_bytes` cap は ArtifactReader の positioned bounded
  read と同一 contract — `artifact.rs:64-77` の
  `{data, offset, nextOffset, eof}` shape を result として返す)。
  read 実体は `RouteLogReader`(`ArtifactReader` impl)を共有し、
  生 file path による workspace 外 read 経路は提供しない(R31)。
- retention: `route run` 起動時に best-effort sweep —
  `route-logs/` を走査し、`*.tmp`(実行中 log)は対象外とした
  上で、(a) TTL 超過(30 日)、(b) count 上限(200 file、古い順
  削除)、(c) 合計 size 上限(256 MiB、古い順削除)を強制する
  (design default 値 — spec deferred concern をここで確定)。
  `*.tmp` 除外で sweep が進行中 run の log を消して rename 失敗
  に追い込む競合を防ぐ。retention 失敗は記録のみで route run を
  失敗させない(R29)。

### workflow `when`(R21-R25、D4)

- schema(`schema.rs:75-88` `job_schema`): job object に optional
  `when` を追加。predicate shape は単一 comparison object:
  `{left: <binding>, op: "eq|ne|gt|lt|exists", right: <binding>}`
  — `exists` は `right` 禁止、それ以外は `right` 必須。`left`/
  `right` は既存 binding shape(`{$input: name}`・
  `{$from: job, path}`・`{$literal: value}`、:23-48)をそのまま
  使う — binding 語彙の再発明をしない(表現ドリフト回避)。
- compile(`validate_definition`): `job.when` を `references()` へ
  渡して dep edge 推論に参加させる(R24 — `when` 内の `$from`
  参照が先行 node になる)。`right`/`left` 内の `$from` path は
  `validate_pointer_schema` で通常の binding と同じ検証を受ける。
- engine(`engine.rs`): `pump()` で `remaining == 0` かつ
  `dependency_failed == false` の job は `admit` 前に `when` を
  評価する。評価は deterministic 純粋評価(LLM・I/O なし —
  `$input`/`$from` の materialized 値に対する comparison のみ)。
  when-false → `finish_job(Skipped, diagnostic{code:
  "CONDITION_SKIPPED"})` — `DEPENDENCY_FAILED` skip と
  diagnostic code で区別可能(R22)。
- 演算子意味論(C12 — deterministic に固定): `exists` は
  `left` が non-null に materialize することのみ意味する
  (`null`・missing は false)。`eq`/`ne` は JSON 等価
  (number/string/bool/null の同型同値 — object/array は deep
  equality)。`gt`/`lt` は両 operand が number のときのみ数値
  比較し、片方でも number 以外(`null`・string・bool・
  object・array・materialize 不能)なら `false` — 判定不能を
  error にせず、when-false 側へ安全に倒す(skipped producer
  由来の `null` が `gt` を偶然真にしない)。`when` 自体の
  materialize 失敗(unknown input 参照等)は compile 時検査で
  ほぼ排除されるが、runtime で発生した場合も when-false と
  同等の skip ではなく job failure とする — compile を通過
  した参照が runtime で失敗するのは engine の不変条件違反
  であり、静かな skip より明確な失敗が正しい。
- 伝播規則: `finish_job` の children への `dependency_failed` 伝播
  (:541-547)を、condition-skip では立てないよう修正 —
  `Failed`/`Cancelled`/`DEPENDENCY_FAILED` skip のみが
  `dependency_failed` を伝播し、`CONDITION_SKIPPED` は伝播しない
  (dependents は自身の `when` を評価するか `when` が無ければ実行、
  R23)。
- materialization: condition-skipped producer への `$from` 参照は
  binding 式全体が `null` に materialize する(pointer path 付きでも
  — skip された job の result は存在しないため path 解決自体を
  行わない)。実装: `materialize_bindings`(bindings.rs:197)の
  `Binding::From` 分岐(:220-228)で、`results.get(&b.from)` 不在の
  とき condition-skipped 集合に `b.from` が含まれれば
  `Value::Null` を返し、含まれなければ従来どおり
  `"missing job result"` error。集合は
  `MaterializeOptions.skipped_from: &HashSet<String>`(仮名)として
  呼出し側から渡す — `materialize_job_input` は engine が
  `self.results` と並行で持つ condition-skipped job id 集合、
  `materialize_output`/`materialize_outputs` は `states` から
  導出する。`WorkflowJobState` は `{terminal, result}` のみのため
  `condition_skipped: bool` field を追加し、engine が states 構築
  (:1020-1032)時に outcome の diagnostic `CONDITION_SKIPPED` から
  populate する。DEPENDENCY_FAILED skip・
  Failed/Cancelled job の `missing job result` 既存挙動は変えない
  (それらの producer を参照する dependent は自身も skip される
  ため runtime 経路では到達しない)。output schema が `null` を
  許容しない場合は従来どおり `InvalidOutput` — author が skip
  可能性に応じて schema を宣言する責務(R23)。
- workflow error 集約(:1051-1060): `job.phase != Succeeded` を
  全件 error にする現行規約から `CONDITION_SKIPPED` の
  Skipped を除外する — job outcome の diagnostic code が
  `CONDITION_SKIPPED` の job は workflow failure にカウント
  しない(skip 含みでも他 job 成功 + output 成立なら `ok:true`
  の有効 result、R25)。

### env passthrough / stdin 除外・approval tier(R32/R33/R33bis、D8)

- `passthroughEnv`: route 宣言の allowlist env 名のみ hook 評価時に
  ambient env から capture して payload `env` へ載せ、`route run`
  が scope 構築時の `extra_env` 注入で子 process env へ畳む
  (上記 route run §2)。値は route log・usage record・workflow の
  worker input に載せない(構造保証 — capture は payload `env` に
  のみ存在し、scope options の `extra_env` にのみ使われる)。
- stdin: `route run` は子 process の stdin を接続しない
  (`proc::run` は spawn 時 stdin ignore 済み)。stdin 依存 command
  の検出は executable + argv 形状の tri-state 分類による
  best-effort viability で、構造保証は stdin 非接続そのもの(R33)。
- approval tier(R33bis): 構造的には実行可能だが interactive
  intent・inline code・特権/remote 副作用を持つ command は
  `approval-required` と分類し、Rewrite は emit するが承認 bypass
  決定(`permissionDecision:"allow"`)は一切併記しない — host 側
  の承認 prompt が残る。`route.rewrite.approvalBypass` の opt-in
  対象は `codex`・`cursor`・`copilot`・`pretooluse`・`devin` で
  あるが、approval-required tier はいかなる opt-in 下でも bypass
  emit を行わない(codex は「承認付き置換」を表現できないため
  fallback verdict へ倒す)。
- user-facing doc に hook env ≠ shell env を明記。

### usage record(`usage_log.rs`)

- `kind:"route"`: `{schemaVersion:1, ts, root, kind:"route",
  route, commandHash, originalTokens, returnedTokens,
  reductionRatio, worker:{model, usage}, durationMs:{exec, analyze,
  total}, logRef, refetches, postprocessFailures}`(R34)。
  `commandHash` は raw command 文字列の `DefaultHasher` hex
  (`embedded_fingerprint` と同じ決定的 hash 規約 — command 本文を
  log に残さない)。`originalTokens` は capture した原 output の
  canonical token 数(log file を `count_max_bytes` 上限の bounded
  read で計測 — 巨大 log は cap までで計測し、cap 到達時は
  `postprocessFailures` に partial-count として記録。spec の
  deferred concern「巨大 log の token 計測方法」を streaming では
  なく bounded read + partial 記録としてここで確定)。
  `returnedTokens` は親へ返した compact result の token 数、
  `reductionRatio` は `1 - returned/original`。`refetches` は
  record 記録時点までに観測された refetch 回数(write 時点では
  `route logs` は未実行のため `0`)。
- `kind:"route-refetch"`: `route logs <id>` 実行ごとに
  `{schemaVersion:1, ts, root, kind:"route-refetch", logRef,
  bytesRead}` を append — `cheaper usage` 集計が `logRef` で
  route record と join して真の refetch 数を出す(JSONL は
  append-only で元 record は更新できないため、write-time
  `refetches` + refetch record の 2 面構成)。`usage_report.rs`
  の `is_record` kind gate(`"run"|"count"` のみ受理)に
  `"route"`/`"route-refetch"` を追加し、route record を
  `truthy(ok)` error 集計へ誤分類しないよう独立 bucket として
  集計する — `directlyComparable` 母数へは混入しない(D9)。
- `commandHash` の `DefaultHasher` は toolchain 依存で
  rustc upgrade 間で hash がドリフトし得る — 永続 record の
  join 精度は同一 toolchain 内でのみ保証される
  (embedded_fingerprint の既存運用と同じ制約、C14f)。
  厳密な cross-version 安定性が必要になった場合は fixed-hash
  (SHA-256 prefix 等)への移行を別途検討する。
- いずれも `append_usage_log` 経由の best-effort append(失敗が
  exit code・stdout を変えない)、workspace/`.cheaper/` には書か
  ない。estimated savings 集計(`directlyComparable` 母数)へは
  混ぜない(D9)。

### dedup(`eval_cache.rs`)

- `VariantInputs::Shell`(:314)の hash 入力に effective
  `routes.yaml` 集合の content hash を追加 — builtin routes の
  embedded 内容・user/project 各 file の内容を `DefaultHasher` で
  1 値へ畳む(file 不在は `Absent` marker、policy signature 規約
  :343-357 と同型)。route 定義変更で cached verdict が自然に
  invalidate される(R40)。
- Rewrite verdict は verdict cache へ publish/serve しない(上記
  C5 — env 含み payload のディスク残留回避)。Allow/Deny verdict
  の共有は従来どおりで、実行は各 firing が独立に行う(D10)。

### CLI wiring(args.rs/cli.rs/lib.rs)

- `RouteCliArgs { action: Resolve|Run|Logs, command?, argv?,
  payload?, id?, offset?, max_bytes?, format, quiet, cwd }` と
  `ParsedCli::Route` を追加。`route resolve` は `--command`/
  `--argv` XOR、`route run` は `--payload`、`route logs` は
  positional `<id>` + `--offset`/`--max-bytes`。
- `dispatch` に `ParsedCli::Route` arm — `route::execute` へ委譲。
  `route resolve`/`route logs` は utility envelope + exit 0、`route
  run` は `CommandOut.exit_code`(原 command exit mirror)をその
  まま返す(workflow-run arm の exit code 経路と同型)。`route`
  arm は `record_usage` を呼ばない — kind:"route" 記録は
  `route::run` 内部が担い、envelope への generic 記録と二重計上
  しない。
- `command_name` に `"route"` を追加。help text・`route` の argv
  先頭判定(`resolve_log_root` の `--cwd` 対象 command 集合に
  `route` を含める — run 系と同じ扱い)。

## Data and Control Flow

```text
[hook firing]
  PreToolUse payload → target normalize → RoutingRequest::Shell
    → eval_cache::evaluate_dedup
      → routing_policy::evaluate_shell
        → tokenize_shell → safe 単一 segment?
          → route::resolve(argv, cwd)
            → self-bypass → prefix match → longest → viability
          → runnable → Verdict::Rewrite{command, fallback}
          → otherwise → 従来 read-candidate 評価
    → render: claude(pretooluse)/devin = updatedInput のみ、
       codex/cursor/copilot/windsurf = fallback verdict を render
  → 親 agent が rewrite 済み command を実行

[route run]
  `cheaper route run --payload <b64>`
    → decode → re-resolve + revalidate (R14)
    → route context 合成(単一 grant 投影・extra_env・exec_cwd・
       accept_any_exit_code・log_tee)
    → prepare context (policy load) → load target workflow
    → run_workflow (in-process、D12)
      → exec job (command tool、accept_any_exit、proc tee →
         route-logs/<runId>.log.tmp、outputTokens 計測)
      → analyze job (when: exec.details.outputTokens > tokenThreshold
         → worker で縮約、skip 時は bounded output のみ)
    → rename .log.tmp → .log、kind:"route" record append
    → compact result を stdout へ(親が tool result として受領)
    → exit code = execResult.details.exitCode(原 command exit)

[route logs]
  `cheaper route logs <id> --offset --max-bytes`
    → id を runId 規約で検証 → route-logs/<id>.log を
      RouteLogReader の bounded read で読む
    → {data, offset, nextOffset, eof} + kind:"route-refetch" append
```

## Non-Functional Constraints

- routing 層は親の command 実行を壊さない — Rewrite 不成立は全て
  従来経路(Allow or read-candidate deny)へ倒す(fail-safe、
  ADR driver)。
- hook 評価は bounded — routes.yaml load・resolve は file read +
  in-memory match のみで外部 process・LLM・network を呼ばない。
- route log・usage record は `<userConfigDir>` のみ — workspace・
  `.cheaper/` へ書かない(D6/D9)。`passthroughEnv` 値は子 process
  env 専用で observability 経路へ載せない(R32)。
- `route run` は hook process と同じ trust domain — payload は
  opaque encoding で transport されるが revalidate で再評価する
  (payload 改竄は grant 境界を越えられない構造)。
- dedup の verdict cache に routes hash を含めるため、route 変更は
  cache invalidation で自然に反映 — TTL 残留の stale verdict なし
  (Rewrite verdict はそもそも cache しない)。

## Risks and Trade-offs

- `trusted: true` は self-declared — malicious project も同じ flag
  を書けるが、効果は explicit 宣言 + resolve/usage record の
  provenance 報告に限定され、実行は trusted grant が引き続き gate
  する(ADR D7 の残存リスク受容)。doc で意味と限界を明記する。
- proc tee の plumbing(invocation options → command tool)は
  routing 専用の注入路を既存 scope 構築に足す — generic 化せず
  route 用途に限定する(YAGNI)。
- `when` の skip≠failure 化は engine の job 終了意味論に触れる
  (error 集約・dep 伝播の変更) — `CONDITION_SKIPPED` を
  diagnostic code で区別し、DEPENDENCY_FAILED 経路を不変に保つ
  ことで既存 workflow の挙動を壊さない。
- `route logs` は runId 指定のみ — id 列挙経路は提供しない(secret
  を含みうる log への列挙面を作らない、D6 の scoped 境界)。
- Rewrite verdict を claude/devin 以外へ render 降格で吸収する
  設計は、dedup cache が target を跨いで verdict を共有する構造の
  帰結 — target gate を verdict 側へ持ち込むと cache key が
  target 由来で分裂するため、verdict は canonical に保ち降格を
  render 層に閉じ込める。
- base64url payload は opaque — 中身を人間が読めない代わりに
  `route resolve`/`route run` の diagnostic が decode 結果を報告
  する経路で可読性を補う。

### 実装時の upstream contract 確認

Devin の入力書換え schema は E で公式
[Hooks overview](https://docs.devin.ai/cli/extensibility/hooks/overview) と
[Lifecycle hooks](https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks)
により確認した。`hookSpecificOutput.updatedInput` のみを使い、
承認を決定する `decision` は付けない。これは ADR D3 / R37 の
「入力だけを書き換え、上位の承認を保全する」という判断の変更ではなく、
未検証だった具体的な wire 表現の確定である。

## Deferred Design Concerns

- Claude `updatedInput` silent-ignore(#81340)| upstream 不具合で
  rewrite が透過的に不発となりうる | version 検出 + doctor verb
  (follow-up issue) | coverage が落ちるが fail-safe で command
  実行自体は壊れない。
- Codex/Cursor/Copilot への rewrite | approval-bypass 検証が必要 |
  issue #150(#148 後)| 検証前に発行すると上流 approval を
  素通しする。
- INTERACTIVE_COMMANDS list の網羅性 | stdin 依存の検出は curated
  list の best-effort で判定不能な command は残る | 実害報告ごと
  に拡充 | 未検出の stdin 依存 command は stdin 非接続で
  EOF/timeout に倒れる(構造保証で安全側)。
- 複合 shell command への routing | rewrite-safe gate は単一
  segment 境界を維持 | 需要確認時 | gate 緩和は parse 誤判定の誤
  rewrite リスク。
- `current_exe` anchor の配布形態差 | dev/debug build で発火した
  hook が `target/debug/cheaper` へ rewrite する可能性 |
  impl 時に contract test で pin | payload format が binary
  version 間で変わらない限り実害はなく、current_exe 失敗時は
  `cheaper` bare name へ fallback する規約とする。

## Verification Notes

- [ ] spec Acceptance Criteria(R1-R40)と ADR Verification 項目が
      `tests/integration/`・`tests/contracts/`・
      `tests/compile_contracts/` の test matrix へ写像されること —
      plan 作成時に spec の全 AC を task の検証条件へ落とす。
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` と
      `mise run check` が pass する。
- [ ] `route resolve`/`route run`/`route logs` が hook 経由と CLI
      直接呼出の両経路で同一 contract を返すこと。

## Refine 注記（issue #169）

権限モデル再設計（#151 分解 #163–#168）により、本 doc の以下の記述は現行の真実と異なる。本 design は決定時点の記録として本文は保持する。

- `:301-310` — project route の `trusted: true` shadow 規則・`trusted` field は project 層のみ意味を持つ記述 → #168 で `Route.trusted` field は削除・merge は無条件化。trust は宣言値ではなく宣言層 provenance（builtin/user = trusted、project 層は下位層を shadow しない）。project 層 `trusted` key presence は `InvalidRuntimePolicy` 拒否（値不問）。
- `:398-429` — viability 分類で `no-grant` が executable pin・denied executable・workspace residency・`prepare_cwd` denylist の事前 check を共有する記述 → #166 で viability は**条件付き弱化**（byte-matched mode の argv precheck は維持、grant-set member は存在 + eligibility + pin のみ）＋ `grants:[name,…]` 投影へ一般化。#167 で substitute mode の検査対象 retarget。denied executable・workspace residency・metachar の事前 check は #163 で除去（deny は opt-in `security.execDenyPatterns` 宣言時のみ）。`unsafe-command` 分類は caller 報告 flag + cwd-changing executable の 2 条件のみ。
- `:609` — C9 single-grant 投影（resolved policy を route の grant 1 件へ投影）→ #166 の grant-set 投影へ一般化。
- `:1034-1036` — `trusted: true` は self-declared の限界記述 → #168 で field 自体が削除・宣言層 provenance へ再設計。

governing: [ADR-034L6ihibvbjmSnKmSffWH](../adr/documentation-honesty-sweep-and-migration-guide.md)（canonical 文言・訂正方式）・[SPEC-034L6i3XmfCaWRHAiw8TTT](../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md) R5・[migration guide](../guides/migration.md)

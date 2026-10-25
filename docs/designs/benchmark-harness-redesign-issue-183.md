---
id: "DESIGN-034LUqevp1IIwiPQRmWmE4"
type: "design"
status: "approved"
title: "Rust-native benchmark harness — cheaper-bench・scenario manifest v2・result schema v2 (issue #183)"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
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
    - "../specs/benchmark-redesign-scope-issue-183.md"
    - "../adr/rust-native-benchmark-harness.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/benchmark-redesign-scope-issue-183.md"
    - "../adr/rust-native-benchmark-harness.md"
    - "../discovery/benchmark-harness-audit-issue-183.md"
  derived-by:
    - "../plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "../test-specs/cheaper-bench-harness-contract-issue-183.md"
  verifies: []
  verified-by:
    - "../test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/workflow-orchestration-benchmark.md"
    - "../specs/workflow-orchestration-benchmark.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "benchmarks/README.md"
    - "benchmarks/scenarios/scenarios.json"
    - "benchmarks/results/README.md"
    - "docs/guides/workflow-usage.md"
    - "mise.toml"
    - "biome.jsonc"
    - ".pre-commit-config.yaml"
  defers: []
  deferred-by: []
metadata: {}
---

# Rust-native benchmark harness — cheaper-bench・scenario manifest v2・result schema v2 (issue #183)

## Context

`docs/adr/rust-native-benchmark-harness.md`(ADR-tBRMKuj5yPFMYM57an5HpS)
が Option B — dev-only workspace member `crates/cheaper-bench` — を採択
した。本 doc は ADR が design へ委ねた field 級の設計を固定する:
crate layout、scenario manifest v2、result schema v2、集計・claim
境界、`mise run check` 統合、文書同期一覧、移行リスク。

上流 chain(全て本 doc の `derives-from`):
`docs/discovery/benchmark-harness-audit-issue-183.md`(監査)
→ `docs/specs/benchmark-redesign-scope-issue-183.md`(要件)
→ `docs/adr/rust-native-benchmark-harness.md`(architecture 決定)。
#182 chain・旧 #67 benchmark chain への参照は `references` のみ。

設計の原則: harness は **観測側のみ** を実装する(product runtime・
envelope・worker schema への変更無し)。spawn 対象は dev-seam 有効の
product binary で、計測は常に subprocess wall-time — in-process
`run_cli` は使わない(ADR H7)。

## Scope

### In Scope

- `crates/cheaper-bench` の crate 構成・dep edge・CLI surface・
  fixture child(自己再実行)機構。
- `benchmarks/scenarios/scenarios.json` の manifest v2 形式(arm
  語彙・Route・失敗 matrix・`{fixtureChild}` placeholder・
  fakeStream 宣言)。
- result record / aggregate の schemaVersion 2 — claim enum・
  validation invariant・`environment` 後続 field・immutable 保存。
- `benchmarks/test/*.mjs` から `cargo test -p cheaper-bench` への
  回帰 test 移行と `mise.toml` check pipeline の整合。
- 旧 `.mjs` harness の削除と文書同期(§6)。

### Out of Scope

- 実 provider・実 Cursor/Copilot・外部 CLI・Ollama を前提にした
  計測。host approval bypass の保証。
- product runtime・envelope・worker schema・`usage.jsonl` record 形の
  変更(harness は観測側のみ)。
- `fixture.rs` への `Serialize` 付与(将来 refinement — ADR
  「fixture 供給方式の確定」で見送り済み)。
- `cargo test` ambient `node` 結合の fixture binary 化 — issue #188
  (F1)の管轄。actual parent usage の取得経路(現在常に
  `unavailable`)。
- `mise.toml` の `node` pin 除去(#182 Option A の gate 未達)。
- `cheaper usage` 側の route record(`kind:"route"`/`route-refetch`)
  を用いた分析・aggregate 表示 — issue #184 の管轄。本 harness は
  usage.jsonl を観測源として読むのみで、`cheaper usage` 出力自体は
  計測しない。
- `count-tokens` の部分カウント経路(`CHEAPER_COUNT_MAX_BYTES`
  truncation → `partial-count` record)— 測定対象シナリオに含めず、
  usage.jsonl の `kind:"count"` 系も delta 対象外(宣言が必要に
  なった時点で別 scenario として扱う)。

## Components and Boundaries

### 1. Crate layout — `crates/cheaper-bench`

#### 1.1 構成(bin + lib + tests)

```text
crates/cheaper-bench/
  Cargo.toml            — publish = false、[lib] + [[bin]] name="cheaper-bench"
  src/lib.rs            — module root。harness 本体の全責務を lib 側に持つ
  src/main.rs           — 薄い entry: fixture-child dispatch → CLI parse → run
  src/cli.rs            — arg parse・run ループ(scenario×arm×(warmups+reps))
  src/manifest.rs       — manifest v2 load + fail-fast validation
  src/workspace.rs      — temp workspace 生成(files・.cheaper/models.yaml・
                          .cheaper/policy.yaml・<cfg>/policy.yaml merge・
                          routes.yaml・routed-exec copy)
  src/env.rs            — spawn env 組立(PATH passthrough・HOME/USERPROFILE・
                          NODE_ENV=test・CHEAPER_CONFIG_DIR・
                          CHEAPER_TEST_FAKE_STREAM・SystemRoot/TEMP/TMP・
                          CHEAPER_LOG* passthrough・armEnv)
  src/spawn.rs          — product binary / fixture child の spawn・stdout
                          capture・wall-time・exit code 保持。stdout の parse
                          は per-mode(§2.3 の parser 列)— envelope JSON /
                          `route run` raw text+trailer / hook response JSON の
                          3 形を mode 別に解釈し、解釈不能は envelopeOk
                          判定材料として扱い harness error にしない
  src/fixture.rs        — RunFixture JSON 生成(serde_json::json! /
                          template 組立て + RunFixture::parse round-trip)
  src/child.rs          — fixture child mode 実装(emit/fail + 先行 mode)
  src/arms/mod.rs       — arm 実装語彙(§2.3)
  src/measure.rs        — envelope/record 計測(retained bytes・critical path・
                          usage 集計・usage.jsonl record 走査・route 系 field)
  src/record.rs         — result schema v2 型 + validate + armExpect 照合
                          (既知 key 語彙は §2.6・field 意味は §3.3)
  src/aggregate.rs      — median/p95・stableValue・comparisons・estimated
  src/report.rs         — result JSON 組立て + create_new 書込み
  tests/
    manifest.rs         — manifest v2 validation(必須 coverage・armBase/
                          armEnv/armExpect 整合・route/fakeStream 検証)
    fixture.rs          — 生成 fixture JSON の RunFixture::parse round-trip・
                          step 語彙・profile 生成
    schema.rs           — record/aggregate v2 validation・claim invariant
    aggregate.rs        — median/p95/stableValue/comparisons 純粋関数
```

- lib + bin 分離の理由: `tests/` が lib API を直接検証するため
  (bin 内 unit test でも可能だが、module 構造の独立検証と
  fixture-child dispatch の分離を明確にする)。fixture child は
  **bench bin 自身** が spawn される対象であり、child dispatch は
  `main()` 先頭に置く(lib の `child::dispatch` へ委譲)。
- workspace membership は root `Cargo.toml` の `members =
  ["crates/*"]` glob で自動取得 — manifest 変更は不要。
- `publish = false`、crate 独自 feature は定義しない(ADR 確定
  事項)。

#### 1.2 Cargo.toml dep edge

```toml
[dependencies]
cheaper-core    = { path = "../cheaper-core" }
cheaper-cli     = { path = "../cheaper-cli" }
cheaper-runtime = { path = "../cheaper-runtime", features = ["dev-seam"] }
serde.workspace = true
serde_json.workspace = true
tempfile.workspace = true
uuid.workspace = true
```

各 edge の役割と制約:

- `cheaper-runtime` + `dev-seam` — `dev::fixture::RunFixture` 型は
  `#[cfg(feature = "dev-seam")]` 配下(lib.rs:40-41)のため、型を
  名指す crate 自身が edge で feature を有効化する。`cargo run` /
  `cargo test -p cheaper-bench` が `--features` 無しで動くのはこの
  edge のおかげ。**この edge が有効化するのは cheaper-bench の lib
  用 build であり、計測対象の `cheaper` bin ではない**(後者は明示
  build — §1.3)。
- `cheaper-cli` — `route::payload::{encode, EncodeInput}`(`src/
  route/payload.rs`、feature 非 gate の pub API)で `route run
  --payload` token を harness 側で mint するために使用。
  **default feature のみ。`cheaper-cli/dev-seam` をこの edge で宣言
  してはならない** — feature unification で `cargo build --workspace`
  等の同一 invocation に含まれる `cheaper` bin 自体が seam 有効に
  なり、計測対象への seam 付与が暗黙化する(ADR 禁止事項)。
- `cheaper-core` — `cheaper_core::json::stringify::stringify(&Value)`
  / `stringify_pretty`(`crates/cheaper-core/src/json/stringify.rs`)
  による `JSON.stringify` 意味論の byte 計測(v1 parity)、および
  `CheaperError` 型。canonical number formatter(`json/number.rs` の
  `format_number`/`JsNumber`)は stringify が内部で使う正源 —
  harness は stringify 経由で利用する。
- async runtime(tokio)は導入しない — runner は逐次実行で
  `std::process::Command` の blocking spawn で足りる(KISS)。
- `base64` 直接依存は不要(payload encode は cheaper-cli 側)。
- `repositoryRevision` は `git rev-parse HEAD` を spawn して採取
  (v1 `execFileSync("git", ...)` と同じ。dev tool 内の git 呼出は
  ambient 許容)。

#### 1.3 CLI surface と計測対象 binary

```powershell
mise exec -- cargo build --release -p cheaper-cli --features dev-seam
mise exec -- cargo run --release -p cheaper-bench -- `
  --scenario <id> [--scenario <id>…] `
  --warmups 2 --repetitions 15 `
  [--output <path>] [--binary <path>] [--pricing <file>]
```

- `--scenario` は複数指定可・省略時は manifest 全 scenario。
  `--warmups`/`--repetitions`/`--output`/`--binary` は v1 と同意味。
  `--pricing` は §4.3 の estimated layer を有効化する opt-in。
- 計測対象 binary の解決順: `--binary` 明示 →
  `<repo>/target/release/cheaper(.exe)` → `<repo>/target/debug/
  cheaper(.exe)` fallback(v1 `cliBinaryPath` 同等)。repo root は
  `env!("CARGO_MANIFEST_DIR")` の `../..` から導く。
- **binary は dev-seam 有効 build が前提**。build 済み binary から
  feature 有無は外側で検出不能なため、harness は計測開始前に
  **preflight probe** を 1 回行う。probe 構成: minimal temp
  workspace(`.cheaper/models.yaml` に `cheaper-fake/fake-model: low`
  override + fresh cfg dir)+ stdin `{"task":"probe"}`(bulk-reader
  input schema の required `task` を満たす)+ 生成 fixture は
  session 1 件で `finish.text` が **bulk-reader output schema 適合**
  の JSON(`summary`/`findings`/`relevantFiles`/`suggestedReads`/
  `openQuestions` の 5 required key — schema 不適合 finish だと
  INVALID_OUTPUT となり probe が seam 有無を検証できない)。
  この fixture を `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` 下の
  `cheaper run bulk-reader --cwd <probe ws> --quiet` で spawn し、
  stdout が envelope `ok:true` を返すことを確認する。失敗時は
  「binary に dev-seam が無い」旨の診断と build 手順を出して
  fail-fast(seam 無し binary では fixture path が無視され実
  provider 経路へ進み、auth 系 error で失敗する — probe はその
  失敗形を検出する)。空 stdin/parse error では bulk-reader の
  input 検証で落ちて probe が意味を持たないため、上記の最小構成を
  必須とする。
- record には解決した binary の repo 相対 path・profile(release/
  debug — 解決先 dir から判定)・`devSeamProbe:"passed"` を載せる
  (§3)。
- 2 段手順(build → run)は ADR Consequences で受容済みの既知
  コスト。`benchmarks/README.md` で固定する。

#### 1.4 fixture child(自己再実行)

`{node}` workload・route grant 対象・nonzero exit の供給源として、
bench bin が自ら fixture child になる — `current_exe()` + 固定 argv
marker の self-re-exec(`CHEAPER_ROUTE_CHILD_MODE` /
`CHEAPER_PROC_CHILD_MODE` / `workflow_dev_seam_command_fixture_child`
で実証済みの pattern)。

- dispatch 規約: `argv[1] == "--fixture-child"`、`argv[2]` が mode。
  `main()` は何より先にこの判定を行い、child 処理後 `process::
  exit(code)` する — harness 本体・arg parse には進まない。
- **route 経路で mode を届けるのは typed command の argv であり、
  grant の `argv` ではない** — `command.grants.<name>.argv` は
  `ArgPattern` の match pattern(literal / 末尾 `foo*` prefix /
  末尾 bare `*` rest marker / 個数一致 — `exec/mod.rs`
  `authorize_args` は照合のみ)であり、spawn argv への literal 注入
  機構は存在しない。spawn される argv は `route::resolve` の
  `exec_args` = typed argv の `argv[1..]`(byte-matched mode)、
  または `route.substitute.argv`(substitute mode)である
  (resolve.rs:255,319)。route_run.rs が `CHEAPER_ROUTE_CHILD_MODE`
  env で mode を渡していたのはこの制約のため — typed argv は
  `route_run::command_fixture --exact --nocapture` の literal pin
  で固定し、mode は grant `env`(spawn env へ注入される方の宣言
  面)に載せていた。本 harness は **typed command 自体に mode を
  書く**方式を取る: `command = "benchrun --fixture-child <mode>"`
  → `exec_args = ["--fixture-child", "<mode>"]` が literal grant
  argv `["--fixture-child", "<mode>"]` と一致し、そのまま child
  dispatch の `argv[1]`/`argv[2]` になる。typed 名と exec argv を
  分けたい場合の逃げ道は `route.substitute = {grant, argv}`(宣言
  凍結の実行 argv — routes.yaml substitute 規約)だが、required
  scenario は使わない(§2.4)。
- mode 語彙:
  - `emit` — `CHEAPER_BENCH_CHILD_BYTES`(既定 4096)byte を stdout
    へ書き exit 0。`emit.mjs` と `big` mode の置換。内容は固定
    seed PRNG 由来の deterministic な高エントロピ印字可能列とする
    — `'L'.repeat(n)` 型の低エントロピ反復は o200k BPE で高圧縮
    され token 数が byte 数を大きく下回るため、condense
    `when: outputTokens > tokenThreshold` 評価(実 BPE 計数 —
    §2.2/§2.4)の決定性を担保できない。seed 固定で byte→token
    密度に既知の下界を持たせる。
  - `fail` — stdout/stderr marker を書き `CHEAPER_BENCH_CHILD_EXIT`
    (既定 7)で exit。nonzero exit・exit-code mirror 計測用
    (`io`/`e2e` mode 相当)。
  - `sleep`・`envdump` — 先行 mode(将来拡張用)。required 12
    scenario は消費しない(A11 — `sleep` = process timeout 計測、
    `envdump` + `CHEAPER_BENCH_CHILD_OUT` = `passthroughEnv` 到達
    検証の予約語彙として宣言のみ)。`passthroughEnv` manifest
    field も同じく先行宣言であり、12 scenario は全て `[]`。
- spawn 経路は 2 系統: (a) `processPolicy.tasks.*.command` は
  `["{fixtureChild}", "--fixture-child", "<mode>"]` の literal
  argv(fixed grant — task の `command[1..]` がそのまま子 argv。
  route 経路は上記の typed-command 方式で grant argv pattern が
  `exec_args` を照合)、params は grant `env` 宣言(env は照合で
  はなく実注入面 — `CHEAPER_BENCH_CHILD_BYTES`/`_EXIT` 等を乗せる);
  (b) `direct-child-exec` / `direct-worker-sequence` の manual 側で
  harness が `Command::new(current_exe())` を直接 spawn。
- Windows/Unix parity: `current_exe()` が返す abs path をそのまま
  `executable` に使う(拡張子 `.exe` 込みで PATH 解決を経由しない
  ため platform 差が出ない)。引数・env 渡しは `Command` API で、
  shell 経由は一切しない(quoting 差を排除)。route_run.rs が同一
  手法で両 platform を検証済み。

### 2. Scenario manifest v2(`benchmarks/scenarios/*.json`)

#### 2.1 manifest 形式

```jsonc
{
  "schemaVersion": 2,
  "fixtureRevision": "seeded-v2",
  "scenarios": [
    {
      "id": "tool-to-worker-chain",
      "title": "…",
      "files": { "source.txt": "…" },
      "workflowMaxJobs": 3,                  // optional — <cfg>/policy.yaml workflow.maxJobs
      "policy": {                            // optional — <cfg>/policy.yaml への
        "execution": { "timeout": "500ms" }, //   scenario overlay(workflow 経路の
        "output": { "repairAttempts": 1 }    //   lever — fixture policy/abort/expect
      },                                     //   節は run 経路専用で workflow には
                                           //   効かない — §2.4/§2.5)
      "workflow": { "schemaVersion": 1, … },  // optional — workflow definition
      "savedWorkflowFile": "saved-workflow.json", // optional — saved arm の source
      "processPolicy": {                     // optional — project .cheaper/policy.yaml
        "maxConcurrent": 1,                  //   process 節(v1 parity — 層は §2.4)
        "tasks": {
          "emit": {
            "command": ["{fixtureChild}", "--fixture-child", "emit"],
            "inputs": [],
            "outputs": []
          }
        }
      },
      "route": {                             // optional — Route scenario 宣言
        "name": "bench-route",
        "match": { "command": ["benchrun"] },// literal token prefix のみ(`*` 不可)
        "command": "benchrun --fixture-child emit",
                                           // resolve/hook 入力の原 command。
                                           // argv[1..] が exec_args として spawn
                                           // される — mode はここに書く(grant
                                           // argv は match pattern で注入器では
                                           // ない — §1.4)
        "grant": "bench-child",              // policy command.grants の key
        "childMode": "emit",                 // command 末尾 token と一致必須
        "childBytes": 4096,                  // CHEAPER_BENCH_CHILD_BYTES(grant env)
        "tokenThreshold": 8192,              // routes.yaml output.tokenThreshold
        "timeout": "30s",                    // optional — grant timeout(DURATION_
                                             //   PATTERN)。省略時は command section
                                             //   default 2m → workflow deadline は
                                             //   grant timeout + 120s(run.rs:338-348)
        "maxOutputBytes": null,              // grant の output cap(truncation 計測)
        "passthroughEnv": []                 // 先行宣言(§1.4 — 12 scenario は [] 固定)
      },
      "fakeStream": "auto",                  // §2.5
      "arms": ["workflow", "direct-worker-sequence"],
      "armBase": { "alias-arm": "direct-worker-sequence" },
      "armEnv": { "arm": { "CHEAPER_LOG": "debug" } },
      "armExpect": { "arm": { "envelopeOk": false, "exitCode": 7 } },
      "armFakeStream": { "arm": { "fixture": { … } } },
      "manual": { "kind": "direct-worker-sequence", "tasks": ["…"],
                  "child": { "mode": "emit", "bytes": 32768 } },
      "comparisons": [
        { "left": "workflow", "right": "direct-worker-sequence",
          "reduction": { "left": "retainedBytes", "right": "retainedResultBytes" } }
      ]
    }
  ]
}
```

v1 からの変更点:

- `schemaVersion: 2`・`fixtureRevision: "seeded-v2"`(fixture 意味論が
  `{node}` workload → fixture child に変わるため version 付きで明示)。
- `processPolicy` command の placeholder `{node}` → `{fixtureChild}` に
  rename — spawn 先が node から bench bin 自身に変わる名実一致
  (ADR 移行 note)。workspace 生成時に `current_exe()` の abs path へ
  展開する。
- `manual.processScript`(node script path + `process.execPath` spawn)
  → `manual.child {mode, bytes}`(fixture child spawn)に置換。
- `route` block・`armExpect`・`armFakeStream`・`comparisons`・scenario
  級 `policy` overlay(§2.4 の merge 規則)を追加。
- `armBase`/`armEnv`/`arms`/`files`/`workflow`/`workflowMaxJobs`/
  `savedWorkflowFile`/`responseProfiles`/`responseDelayMs`/`manual` の
  既存 key は意味不変で継続(`workflowMaxJobs` は引き続き user 層
  `<cfg>/policy.yaml` の `workflow.maxJobs` へ — §2.4 merge)。
- fixture DSL(`fakeStream`/`armFakeStream`/`manual` 経由で生成する
  RunFixture)の `policy`/`abort`/`expect` 節は **`run`(direct-worker)
  経路専用** — `workflow run` は `restrict_policy`(load 済み
  policy.yaml + `--timeout` narrow)が実効 policy を決め、fixture
  `policy`/`abort`/`expect` は一切適用されない(`expect` は
  replay-harness 語彙で CLI も読まない)。workflow 系 arm の
  timeout/repair 制御は必ず scenario `policy` overlay で供給する
  (hang session を workflow arm に載せたまま既定 `execution.timeout:
  300s` に委ねると 1 obs ~5 分×rep 数の wall-time になる)。

#### 2.2 required scenario 集合(12 件)

`REQUIRED_SCENARIO_IDS` 相当の exact coverage(欠落・unexpected 双方
拒否)を manifest validation が強制する。継続 7 件に加え、Route・
失敗 matrix で 5 件を追加する。

| id | arms | 測定対象 |
| --- | --- | --- |
| `single-simple-tool-call` | workflow, direct-filesystem, direct-worker-tool-call, run-logging-{off,file,debug} | 従来どおり(armBase/armEnv・CHEAPER_LOG matrix 維持) |
| `independent-fanout-worker-fanin` | workflow, direct-worker-sequence | 従来どおり(workflowMaxJobs:3) |
| `tool-to-worker-chain` | workflow, direct-worker-sequence | 従来どおり |
| `process-and-analysis` | workflow, direct-worker-sequence | process task = `{fixtureChild}` emit + Worker |
| `large-intermediate-process-analysis` | workflow, direct-worker-sequence | 同上・32 KiB child 出力 |
| `dynamic-vs-saved-workflow` | dynamic-workflow, saved-workflow, direct-worker-sequence | 従来どおり |
| `definition-vs-compact-interface` | workflow, full-definition, compact-interface | 従来どおり + `parentVisibleReduction` derive |
| `route-resolve-and-rewrite` | route-resolve, route-rewrite | dry-run resolve vs hook verdict(rewrite)の wall-time・verdict 内容。hook arm は各 spawn 前に `<cfg>/hook-eval/` を削除(dedup cache 対策 — §2.3) |
| `route-routed-exec` | route-run, direct-child-exec | routed-exec workflow 実行(condense skip)vs 素の child spawn。route orchestration overhead。`tokenThreshold` は default 8192 のまま、childBytes は emit 内容の既知 token 密度で確実に未満に収める(§1.4)+ `armExpect` で `condenseFired:false` を pin |
| `route-condense-and-refetch` | route-run, route-logs, direct-child-exec | `tokenThreshold: 1`(emit 出力の保証 token 数未満 → condense 確定発火 — `when` は `outputTokens > tokenThreshold` の厳密 `>`)+ `armFakeStream` で **condense output schema 適合 session**(`{"summary":…}` finish — `{summary}` のみ・additionalProperties:false。bulk-reader 形 text を流すと INVALID_OUTPUT → job-failed に落ちる)を analyze job へ供給 + `maxOutputBytes` truncation + `route logs` refetch。`armExpect` で `condenseFired:true`/`truncated:true` を pin |
| `worker-failure-modes` | direct-provider-error, workflow-provider-error, direct-worker-timeout, workflow-worker-timeout, direct-postprocess-failure, workflow-postprocess-failure | armBase → `direct-worker-tool-call`/`workflow`、armFakeStream が error/hang/schema 違反 finish を供給、armExpect が失敗形を pin。workflow 系 arm の timeout/repair は scenario `policy` overlay が担う(fixture `policy`/`abort`/`expect` は run 経路専用 — §2.1): `execution.timeout` を hang 計測用に縮小し、`workflow-postprocess-failure` は `output.repairAttempts` + `repairAttempts+1` 連続 schema 違反 session(§2.5 の消費規則)を宣言する |
| `route-child-nonzero-exit` | route-run, direct-child-exec | childMode `fail`(exit 7)— exit code mirror・log 保持・再実行防止の観測。小出力 + `tokenThreshold` default で condense 非発火を pin(`condenseFired:false`) |

#### 2.3 arm 実装語彙(impl set)

code 側の impl 語彙は固定 enum とし、arm 名は `armBase` 解決後に
impl 名と一致することを要求する(`armBase` の domain は
**宣言 arm ∪ impl 語彙** — 宣言 arm 名自身も concrete impl 名も
写像先として合法。宣言 arm → 別の宣言 arm への alias 連鎖は不可)。

| impl | 計測内容 |
| --- | --- |
| `workflow` / `dynamic-workflow` / `saved-workflow` | workflow.json 生成(saved は copy)→ `workflow interface` → `workflow run` spawn。definition bytes・interface bytes・envelope 計測。**stdout parser: workflow envelope** |
| `direct-filesystem` | harness 内 `std::fs::read` — **in-process floor**(delegate しない baseline。v1 の `node:fs` read の Rust 置換 — §7 baseline 同等性)。spawn 無しのため `exitCode`/`envelopeOk` は null |
| `direct-worker-tool-call` | `run bulk-reader` 1 回 + scripted `tool_calls` step — **real CLI worker tool call floor**(issue #183 の "direct canonical tool/CLI" に相当。mode = `single-direct-worker-with-one-tool-call`)。**parser: worker envelope** |
| `direct-worker-sequence` | file read → hints 化 → 逐次 `run bulk-reader`。`manual.child` 指定時は fixture child を直接 spawn して出力を hint へ。**parser: worker envelope** |
| `full-definition` | harness 内 `std::fs::read` of workflow.json。spawn 無し |
| `compact-interface` | `workflow interface` spawn → result byte。**parser: utility envelope** |
| `route-resolve` | `route resolve --command <route.command> --cwd <ws> --quiet` spawn。**parser: utility envelope** — verdict は `data.matched`/`data.route`/`data.viability`/`data.executable`/`data.argv`/`data.args`/`data.suppressed?` |
| `route-rewrite` | `hook pre-tool-use --target pretooluse` spawn、stdin `{"tool_name":"Bash","tool_input":{"command":<route.command>}}`。**parser: hook response JSON(envelope ではない)** — `hookSpecificOutput.updatedInput.command` の rewritten `"<exe>" route run --payload <tok>` を観測し、`route.rewritten` = 同 key 有無、`route.payloadBytes` = 末尾 token の byte 数を記録。**各 spawn 前に `<cfg>/hook-eval/` を削除**する — `evaluate_dedup` が fingerprint 単位で `<userConfigDir>/hook-eval/<fp>.json` に verdict を 60s TTL cache するため、削除しないと rep≥2 が cache hit して未 cache の `route-resolve` と非対称 bias になる(hook/eval_cache.rs) |
| `route-run` | payload mint(下記導出規則)→ `route run --payload <tok> --cwd <ws> --quiet` spawn。**stdout parser: raw text + trailer**(envelope 非出力 — `cli.rs` `RouteOutput::Run` は `out.text` を直接書き、`--format` も効かない)。観測は trailer regex `\[cheaper route log: (\S+); truncated: (true|false)\]` → `route.runId`/`route.truncated`、usage.jsonl `kind:"route"` record → `worker`(非空 = analyze job meta `{model,usage}` あり = **condenseFired**)・`originalTokens`/`returnedTokens`/`exitCode`/`ok`/`logRef`/`durationMs{exec,analyze,total}`・`postprocessFailures`・`executed.{grant,argv}` を記録 |
| `route-logs` | setup(計測外)で `route run --payload <tok>` を 1 回実行し trailer regex(fallback: `kind:"route"` record `logRef`)で `route.runId` を取得 → `route logs <runId> --cwd <ws> --quiet` spawn を計測。**parser: utility envelope** — `data`/`offset`/`nextOffset`/`eof` を観測し、`route.logRefetchBytes` = `data` の byte 数(or `kind:"route-refetch"` record `bytesRead`)。setup の `kind:"route"` record は同 observation cfgDir の usage.jsonl に残り `directUsageRecordCount` に計上される(§4.4) |
| `direct-child-exec` | fixture child を直接 spawn(route を通さない **child process floor**)。stdout は生 byte — `envelopeOk`/`errorCode` は null、marker・exit code・wall-time を観測 |

route-run arm の payload mint 導出規則(`payload::encode`
`EncodeInput` — hook の `build_rewrite_command` と同規約を再現):

- `command` = manifest `route.command` 原文(env-prefix を含めてよい
  — `route run` が tokenize して再導出する)。
- `argv` = `tokenize_shell(command)` の単一 segment token 列から
  env-prefix(`KEY=VALUE` 先頭列)を除いた部分(`split_env_prefix`
  規約)。`route run` が `rest != payload.argv` を
  `argv-mismatch` で reject するため、この規約に合わない manifest
  `command` は viability 前に落ちる(run.rs:75-81)。
- `args` = `argv[match.command.len()..]`、`executable` =
  `argv[0]` の basename(`command_name` 規約 — dir prefix 除去 +
  `.exe`/`.bat`/`.cmd` suffix 除去)。
- `cwd` = spawn に渡す workspace `--cwd` と同値の canonical path
  (`route run` が `canonicalize_workspace_root` で照合 —
  `cwd-mismatch` reject。run.rs:61-65)。
- `ambient_env` = `{<passthroughEnv 名>: spawn env の値}` のみ
  (route run が `env ⊆ passthrough_env` を `env-not-allowed` で
  reject — run.rs:107-113)。12 scenario は `passthroughEnv:[]`
  のため `ambient_env` は空。

direct 系 3 層の floor 命名整理(§7 baseline 同等性の前提):
`direct-filesystem` = in-process floor、`direct-child-exec` =
fixture-child process floor、`direct-worker-tool-call` = real CLI
worker tool call floor(= issue #183 が求める "direct canonical
tool/CLI" の arm)。

失敗系は arm impl ではなく **fixture + route/child 宣言 + expect**
で表現する — 失敗は計測される観測値であって別実装ではない。

**arm-class 別 success 意味論**(§4.1 `successRate` の母数):
observation の `success` = 宣言 `armExpect` key 全項目一致(§3.3)。
`envelopeOk`/`errorCode` は **envelope を emit する arm でのみ意味を
持ち、raw-text/hook-JSON/harness-内 arm では null** — envelope の
parse 可否は mode 語彙の parser 列で決まり、record の null は
「envelope 形を持たない arm」を示す(measure.rs が mode → parser
写像を担う)。`armExpect` 未宣言の補助 arm のみ fallback として
`exitCode==0`(envelope arm は加えて `envelopeOk===true`)を
success とする。

#### 2.4 Route scenario の materialize

`route` block から harness が生成する物:

- `<cfg>/routes.yaml` — `version: 1` + route 1 件(name/match/
  target `{kind: workflow, path: routed-exec.workflow.json}`/grant/
  output.tokenThreshold/passthroughEnv)。**user 層に置く必須条件**:
  route が参照する grant は `scope==trusted && !fixed` でなければ
  route-eligible でない(`is_route_eligible` — NoGrant に落ちる)
  ため、grant は trusted 層の `command.grants` に宣言する。
  `substitute`/`trusted` key は builtin/user 層のみ受理で project
  層宣言は load error(routes.yaml:8-9)— 本 harness は typed
  command に mode を載せるため `substitute` は emit しない
  (§1.4)。
- `<cfg>/routed-exec.workflow.json` — builtin 定義を
  `env!("CARGO_MANIFEST_DIR")/../cheaper-cli/resources/workflows/
  routed-exec.workflow.json` から copy(exec command job → `when`
  outputTokens>tokenThreshold で condense worker の実体)。
- `<cfg>/policy.yaml` — `command.grants.<grant>` =
  `{executable: <bench bin abs>, argv: ["--fixture-child", childMode],
  env: {CHEAPER_BENCH_CHILD_BYTES: …}, maxOutputBytes?, timeout?}`。
  **`argv` は literal `ArgPattern`** であり、照合対象は typed
  command の `exec_args`(= `argv[1..]`)— 個数一致 + literal
  一致が求められるため、manifest `route.command` の末尾は必ず
  `--fixture-child <childMode>` で終わる(§2.6 で検証)。`timeout`
  は `route.timeout` 由来で省略時は `command` section default
  `2m` — route の workflow deadline は grant timeout + 120s
  (run.rs:338-348)。grant `env` は spawn env へ注入される宣言面
  (照合対象は `argv` のみ)。
- **`<cfg>/policy.yaml` は merge 組立て** — `workflowMaxJobs`
  (`workflow.maxJobs`)・scenario `policy` overlay
  (`execution`/`output`/`retry` 等の section 部分 object)・route
  `command.grants` が同じ file を書きに来るため、workspace.rs は
  `{version:1, …overlay/merge 済み sections…, command:{grants}}` の
  1 document として組み立てる(後勝ち上書きではなく section merge。
  A7)。`processPolicy` は v1 parity で **project 層
  `<ws>/.cheaper/policy.yaml`** へ行く(process.tasks は project
  task として `scope:project` に畳まれる — route grant と
  layer/scope が異なるため同 file に混ぜない。`command.grants` と
  `process.tasks` の同名は `build_exec_input` が
  InvalidRuntimePolicy で fail-loud)。
- condense worker 応答は `fakeStream`/`armFakeStream` fixture の
  session が供給(builtin `condense` worker job が `create_session`
  を 1 回消費)— **condense output schema(`{"summary":…}` のみ)
  適合の finish text を持つ session を condense scenario 側が明示
  bind する**(§2.2)。
- `route run` は `kind:"route"`、`route logs` は `kind:"route-refetch"`
  record を usage.jsonl に書く — 観測は record 件数と envelope 双方
  から行い、token 集計への二重計上はしない(§4.4)。

#### 2.5 fixture JSON 生成

ADR 確定方式: `serde_json::json!`/template 組立て → 書込み前に必ず
`RunFixture::parse` で round-trip 検証 → file write →
`CHEAPER_TEST_FAKE_STREAM` path 設定。`dev::fixture` 型は
Deserialize-only のため generator は canonical schema へ parse で
anchor される(JS 平行実装の drift 解消)。

manifest `fakeStream` 指定:

- `"auto"`(既定)または `{profiles: ["compact"|"large"|…],
  delayMs, toolCallPath}` — v1 generator 相当の profile 駆動 session
  生成(usage_delta scripted 固定値・bulk-reader schema 適合 text を
  Rust 側に移植。worker job N 件には session N 件)。
- `{fixture: {…}}` — RunFixture DSL の部分 object を passthrough
  merge して生成(`sessions[].steps` を含む全節を manifest が直接
  書ける — error/hang/panic/abort 等の DSL 全 step を parallel
  vocabulary なしで使える)。arm 単位の差し替えは `armFakeStream`
  (失敗 matrix arm 用)。
- 生成 JSON は必ず `RunFixture::parse` へ通す — 無効 step・型違反は
  benchmark 実行前に fail-fast。

session 結合規則(per-spawn fixture binding — `FakeSessionFactory`
は `create_session` 1 回ごとに `sessions[]` を宣言順で 1 件消費):

- `direct-worker-*` / `run` 系 arm — `manual.tasks[i]` が i 番目の
  `run bulk-reader` spawn に対応し、その spawn の fixture は
  `manual.responseProfiles[i]`(未指定は scenario `fakeStream` の
  profile 既定値、さらに未指定は `"compact"`)で生成した session
  を持つ(1 spawn = 1 fixture = 1 session 消費が基本形)。
- workflow 系 arm — 1 spawn(`workflow run`)内の `kind:worker`
  job が job 開始順に session を消費する。worker job N 件には
  session N 件。
- repair/postprocess — `output.repairAttempts` = r の worker job は
  schema 違反時に r+1 session を消費する(main + repair r 回)。
  `workflow-postprocess-failure` のように「全試行 schema 違反」を
  計測する arm は、manifest が `armFakeStream` で **r+1 連続の
  schema 違反 finish session を宣言順で並べる**ことで宣言する
  (scenario `policy` overlay の `output.repairAttempts` と session
  数を整合させる — §2.2)。
- route-run arm — condense `analyze` job(発火時のみ)が session を
  1 件消費する。発火しない scenario(route-routed-exec・
  route-child-nonzero-exit)は session を消費しない。

#### 2.6 manifest validation(fail-fast)

`manifest.rs` が enforce する invariant:

- `schemaVersion === 2`・`fixtureRevision` 非空・scenario id 一意・
  `REQUIRED_SCENARIO_IDS` exact coverage(missing/unexpected 双方拒否)。
- `title` 非空・各 scenario が宣言 arm を持つこと。workflow 系 arm を
  含む scenario は非空 `workflow.jobs` を要求(v1 parity)。
- `armBase`/`armEnv`/`armExpect`/`armFakeStream` の key は全て宣言 arm
  内、`armBase` value は宣言 arm ∪ impl 語彙の concrete impl 名
  (alias の連鎖不可 — §2.3)。
- `armBase` 解決後の名前は impl 語彙(§2.3)に含まれること。
- `route` 宣言を持つ scenario は `route-*` impl の arm を 1 つ以上
  含むこと。`route.childMode` は既知 mode に限定。
- `route` block の制約:
  - `match.command` は **literal token 列のみ**(is_prefix の
    exact-prefix 語彙 — `*`/`foo*` は grant `argv` pattern 側の
    語彙であり match.command では使えない。`*` 含有 token は
    reject)。`match.command[0]`(= typed argv[0])が
    `"cheaper"`(case-insensitive)の宣言は reject — self-bypass
    規約で resolve が match 評価自体を返さないため到達不能 route
    になる(resolve.rs:233-237)。
  - `route.command` を `tokenize_shell` した単一 segment の
    env-prefix-strip 後 argv について、`match.command` が `argv`
    の literal prefix(`is_prefix` — 規範形は単 token
    `["benchrun"]`)であり、かつ **`argv[1..] ==
    ["--fixture-child", childMode]` 完全一致**であること
    (exec_args が literal grant argv pattern と child dispatch の
    双方を兼ねる — §1.4/§2.4)。
  - `route.timeout` は `DURATION_PATTERN` 適合のみ受理
    (grant `timeout` へ流す — §2.4)。
- scenario `policy` overlay は既知 top-level section
  (`workflow`/`execution`/`output`/`retry`/`tools`/`filesystem`/
  `model`/`route`/`security`/`command`/`process` 等の policy.yaml
  語彙)のみ受理 — route 宣言との併用時も `<cfg>/policy.yaml`
  merge の衝突は workspace.rs が section merge で解消する(§2.4)。
  未知 section は reject。
- `comparisons` の left/right は宣言 arm。`reduction` の field 名は
  既知 metric/dimension 名に限定(§4.2)。
- `armExpect` value の key 語彙は次に限定(観測後の deterministic
  検証用 — 全項目一致が §2.3 の `success` 定義):
  - top-level: `{envelopeOk?, errorCode?, exitCode?, success?}` —
    `envelopeOk`/`errorCode` は envelope arm にのみ意味を持つ
    (非 envelope arm では常に null — null 期待値も宣言可)。
  - `route`: `{runId?, condenseFired?, truncated?, rewritten?,
    payloadBytes?, logRefetchBytes?, originalTokens?,
    returnedTokens?, childOutputBytes?, returnedBodyBytes?}`
    (§3.3 route block の field 名と同一)。
  - `resolve`: `{matched?, route?, viability?, executable?,
    suppressed?}`(route-resolve arm の verdict pin)。
  - `metrics`: 既知 metric 名(§3.3 `metrics`/`childInvocations`
    等 — stableValue 対象の同名 key)。

### 3. Result schema v2

#### 3.1 result top-level

```jsonc
{
  "schemaVersion": 2,
  "runId": "<uuid>",
  "createdAt": "<iso>",
  "fixtureRevision": "seeded-v2",
  "repositoryRevision": "<git sha>",
  "conditions": {
    "warmupCount": 2,
    "repetitionCount": 15,
    "selectedScenarios": ["…"],
    "ambientPassthrough": ["CHEAPER_LOG", "CHEAPER_LOG_FORMAT"]
  },
  "measuredBinary": {
    "path": "target/release/cheaper.exe",   // repo 相対
    "profile": "release",                    // 解決先 dir(release|debug)
    "devSeamProbe": "passed"                 // §1.3 preflight の結果
  },
  "environment": {                           // §3.2
    "os": "windows", "osRelease": "10.0.26200",
    "arch": "x86_64", "cpuCount": 16,
    "ambientRustc": "rustc 1.99.0 (…)",
    "harnessProfile": "release"
  },
  "pricing": null,                           // --pricing 未指定時は null
  "observations": [ … ],
  "aggregate": { … }
}
```

#### 3.2 `environment` block(v1 `environment.node` の後続)

v1 は `environment{os, architecture, node, cpuCount}`(node field は
検証未宣言の結合残滓)。v2 では次を記録する:

- `os` = `std::env::consts::OS`、`osRelease` = best-effort
  (Windows: `cmd /c ver` の番号抽出 / Unix: `uname -sr`。失敗時
  null — spawn は dev tool 内の ambient 許容)。**format 差異に
  注意**: v1 の `os` は node `platform()` + `release()` の合成
  文字列(`"win32 10.0.x"`)で、v2 は Rust 命名の
  `windows`/`linux`/`macos` + 別 field の `osRelease` に分離する
  — 値 domain が異なるため v1 file との field 比較はしない
  (migration note)。
- `arch` = `std::env::consts::ARCH`(Rust 命名 `x86_64`/`aarch64` —
  node の `x64`/`arm64` と表記が変わる点は migration note で明記)。
- `cpuCount` = `std::thread::available_parallelism`。
- `ambientRustc` = 実行時 ambient `rustc --version` の先頭行 trim
  (mise pin が ambient PATH に載る前提 — `mise exec` 経由)。
  **ambient toolchain の記録であり、計測対象 `cheaper` binary を
  build した toolchain を保証するものではない**(別 dir の既存
  binary を `--binary` で指した場合等に乖離し得る)— 素直な
  `rustToolchain` 命名だと「計測 binary の toolchain」に読める
  ため、名実一致として `ambientRustc` とする。
- `harnessProfile` = `cfg!(debug_assertions)` 由来の `debug|release`
  (bench bin 自身の profile — `cargo run` で debug build された
  harness による spawn-loop overhead を結果へ可視化するため)。
- `node` field は **削除** — schema v2 validation が存在を拒否する
  (再混入の fail-fast guard)。
- 測定対象 binary 側の feature は `measuredBinary.devSeamProbe` で
  表明する(外側からの feature 検出は不能 — probe 通過が証拠)。

#### 3.3 observation record

```jsonc
{
  "schemaVersion": 2,
  "runId": "<uuid>",
  "scenarioId": "…", "scenarioTitle": "…",
  "arm": "…", "repetition": 3, "warmup": false,
  "mode": "workflow|direct-tool-operation|
           single-direct-worker-with-one-tool-call|
           scripted-direct-cli-sequence|
           full-definition-read|public-interface-discovery|
           route-resolve|route-rewrite|route-run|route-logs|
           direct-child-exec",
                                           // v1 mode 名は全て継承 —
                                           // `direct-worker-tool-call` arm
                                           // は v1 どおり
                                           // `single-direct-worker-with-one-tool-call`
                                           // (rename しない — §7 note)
  "parentUsage": { "status": "unavailable", "reason": "…" },
  "fixtureRevision": "seeded-v2",
  "environment": { … },                      // §3.2 と同 block
  "command": "target/release/cheaper.exe workflow run …",
  "wallTimeMs": 123.4,
  "success": true,                           // armExpect 全項目一致(§2.3
                                           //   success 意味論)
  "exitCode": 0,                             // spawn arm のみ。harness 内 arm は null
  "envelopeOk": true,                        // envelope.ok 観測値 — envelope を
                                           //   emit する arm のみ有効、非
                                           //   envelope arm(route-run 生 text/
                                           //   hook JSON/harness 内)は null
  "errorCode": null,                         // envelope.error.code 観測値(同上)
  "directUsageRecordCount": 0,               // 当該 observation cfgDir の
                                           //   usage.jsonl 行数(全 kind・
                                           //   計測外 setup spawn 分を含む —
                                           //   route-logs arm は route+refetch
                                           //   で 2 を期待)
  "directInvocationTurnsProxy": 1,
  "workflowDefinitionBytes": 804,
  "parentVisibleInterfaceBytes": null,
  "interfaceDiscoveryWallTimeMs": null,
  "discoveryMode": "generated-definition",
  "definitionGenerationMs": 0.5,
  "route": null | {
    "payloadBytes": 412,                     // mint/rewrite 観測した payload
                                           //   token の byte
    "rewritten": true,                       // route-rewrite: hook response
                                           //   `hookSpecificOutput.updatedInput.
                                           //   command` の有無(envelope では
                                           //   なく hook JSON 由来)
    "runId": "<route run id>",               // route-run stdout trailer regex
                                           //   `\[cheaper route log: (\S+);`
                                           //   (log 無し時は `unavailable`。
                                           //   fallback: kind:"route" record
                                           //   `logRef`)— route logs/usage
                                           //   record 照合用
    "condenseFired": false,                  // kind:"route" record `worker`
                                           //   非空(analyze job meta
                                           //   {model,usage} あり)
    "truncated": false,                      // trailer `truncated: <bool>`
    "originalTokens": 1200,                  // kind:"route" record —
                                           //   captured log の o200k 実計数
    "returnedTokens": 60,                    // kind:"route" record — 親へ返す
                                           //   text の実計数
    "childOutputBytes": 32768,               // captured exec 出力 byte —
                                           //   `<cfg>/route-logs/<runId>.log`
                                           //   の file size(harness が直接
                                           //   読む — observation 内 cfgDir)
    "returnedBodyBytes": 512,                // route-run stdout の byte 数
                                           //   (body + trailer — parent が
                                           //   受け取る見返り)
    "logRefetchBytes": null                  // route-logs arm: 返却 `data` の
                                           //   byte(or route-refetch record
                                           //   `bytesRead`)
  },
  "metrics": { … }
}
```

- `metrics` は v1 の comparable dims を継承(`jobCount`・
  `workerJobCount`・`toolOnlyJobCount`・`workerTurns`・`toolCalls`・
  `usage`・`retainedBytes`・`retainedResultBytes`・`artifactBytes`・
  `criticalPathMs`・`maxConcurrency`・`directToolOperations`・
  `childInvocations`・`finalResultBytes`)に加え:
  - `usage = {input, output, cachedInput, cachedOutput, total,
    source: "scripted"}` — `source:"scripted"` marker 必須(fake
    provider の宣言値であり実料金・実モデル効率の観測値ではない。
    spec invariant)。
  - `repairs`・`transientRetries` — envelope meta の echo
    (postprocess/repair の観測)。
  - `postprocessFailures` — `INVALID_OUTPUT` 系で終了した
    repair/postprocess round 数(envelope error・meta.repairs から
    導出。観測不能経路は null)。
  - `logRefetches` — `kind:"route-refetch"` record 件数。
  - `materializedPromptBytes`・`workerStreamConcurrency` — seam に
    観測 hook が無いため引き続き null(0 でなく null = 未計測の
    意味論を維持)。
- 非ゼロ exit / `envelopeOk:false` は observation 上の観測値 —
  harness は throw しない(§7 nonzero-exit capture path)。
- `armExpect` を宣言した arm は観測後に key 単位で照合し、
  **不一致項目を持つ observation は `success:false` として記録**
  される(即時 abort ではなく — `successRate` が「期待形への一致
  率」という観測 fact になるため。deterministic fixture の回帰
  anchor として、run 終了時に mismatch obs を抱える scenario を
  harness が失敗報告する — exit≠0)。expect 照合は宣言 key のみを
  対象とし、値の多様体(数値・bool・文字列・null)は等値比較。

#### 3.4 claim 境界の表現と validation invariant

claim enum `{measured, estimated, unavailable}` は
**`aggregate.comparisons[]` entry にのみ存在**する設計とする
(observation は測定 fact であり、claim 分類を重ねる必要がない —
usage は `source:"scripted"` marker が担う)。

```jsonc
// aggregate.comparisons[i]
{
  "scenarioId": "…", "left": "workflow", "right": "direct-worker-sequence",
  "claim": "measured",                       // | estimated | unavailable
  "deltas": { "wallTimeMs": …, "retainedBytes": …, … },
  "reason": null,                            // 常に emit — unavailable 時必須
  "assumptions": null                        // 常に emit — estimated 時必須
}
```

record/aggregate validation が fail-fast で強制する invariant
(spec §claim 境界の強制面):

1. `schemaVersion === 2` 完全一致(v1 record は reject — v1 file は
   改変せず残るが、v2 validator は受理しない)。
2. `parentUsage.status ∈ {observed, unavailable}`、unavailable ⇒
   `reason` 必須、observed ⇒ `source` 必須(v1 parity)。
3. `metrics.usage` を持つ observation は `usage.source ===
   "scripted"` 必須 — 未標識 scripted usage の記録を拒否。
4. `environment.node` 存在は reject。`ambientRustc`・
   `measuredBinary.profile` は必須。
5. comparisons entry: `claim` は enum 限定。
   - `claim:"unavailable"` ⇒ `deltas` の全値が null + `reason` 必須。
   - `claim:"estimated"` ⇒ `assumptions` record 必須 —
     `{pricing:{model(+catalog revision 取得可能なら同梱), source,
     inputUsdPerMTok, outputUsdPerMTok, cachedInputUsdPerMTok?},
     tokenizer:"o200k_base", boundary:"scripted-usage"}`(§4.3)。
   - `claim:"measured"` ⇒ `assumptions`/`reason` は **key 自体は常に
     emit し両方 null** を要求(field presence は固定 — claim ごとに
     shape が変わる可変 key 構成にはしない。null 以外の値を持つ
     measured entry は mislabel として fail-fast)。
   - v1 の `status:"incomplete"`(arm 欠落)は `claim:"unavailable",
     reason:"one or both comparison arms are absent"` に fold する。
6. record は parent usage の数値 field を一切持たない
   (`parentUsage` block 以外) — unavailable な parent usage 上の
   数値主張を schema 形で排除する。

#### 3.5 保存・命名・v1 の扱い

- 保存先 `benchmarks/results/<runId>.json`(UUIDv4)。書込みは
  `OpenOptions::create_new(true)` — 既存 file の上書きを拒否
  (`--output` 明示 path も同じ非上書き契約)。append-only・commit
  対象は不変。
- v1(`schemaVersion:1`)の既存 JSON は一切改変しない。v2 の
  load/aggregate は `schemaVersion` を確認して v1 file を reject
  する(混在 run を作らない)。比較は file 単位で行う。

### 4. Aggregation

#### 4.1 arm 集計

v1 と同じ構成: scenario×arm で group 化 → warmup を除く
observation から `medianWallTimeMs`・`p95WallTimeMs`・
`stableMetrics`(全反復で同一値なら値、変動/欠測なら null —
deterministic seeded fixture の回帰 anchor)+ `observationCount`・
`parentUsage` 集約(全 obs unavailable ⇒ unavailable+reason)。

median/p95 の algorithm は **v1 と同一規約を pin する**
(v1 `benchmarks/lib/aggregate.mjs` — 数値差分の説明可能性のため):

- `median(xs)`: 昇順 sort → 奇数個は `sorted[floor(n/2)]`、偶数個は
  `sorted[n/2-1]` と `sorted[n/2]` の平均。
- `p95(xs)`(`percentile(xs, 0.95)`): 昇順 sort → nearest-rank
  `sorted[max(0, ceil(n*f)-1)]`(補間しない)。
- `stableValue`: select した値が全 obs で finite number かつ全同一
  ならその値、変動・欠測・非数を含むなら null。
- 実装は `aggregate.rs` の純粋関数で、`tests/aggregate.rs` が
  上記 3 規約の boundary case(単一要素・偶数/奇数・全同一・
  欠測混在)を unit test で固定する。

v2 追加の集計 dims:

- `successRate` = `success===true` の obs / 全 obs(`success` の
  定義は §2.3 arm-class 別意味論 — armExpect 全項目一致。failure
  matrix arm は「期待どおり失敗」が success に含まれる設計のため
  successRate 自体は観測 fact)。
- `exitCode`・`errorCode` の stableValue(非 envelope arm の
  errorCode は null — null も stableValue 対象の決定値)。
- `postprocessFailures`・`logRefetches`・route dims
  (payloadBytes・condenseFired・truncated・runId・logRefetchBytes・
  originalTokens・returnedTokens・childOutputBytes・
  returnedBodyBytes)の stableValue(runId は rep 毎に変わる値の
  ため実質 null 化する — presence/形式の pin は armExpect 側)。
- `p95WallTimeMs` は arm 集計として保持 — comparisons の delta は
  median と p95 の双方を出す(v1 は median のみ)。

#### 4.2 comparisons

- 比較対象は manifest `comparisons` 宣言(scenario 単位で
  `{left,right}` pair + optional `reduction {left,right}`)。v1 の
  code-side `COMPARISONS` hardcode から manifest 側へ移す —
  scenario 追加に code 変更を要しない(DRY: scenario 意味論の
  owner は manifest)。
- delta 対象 dim: `wallTimeMs`(median/p95)・`usageInput/Output/
  Total`・`workerTurns`・`toolCalls`・`jobCount`・`workerJobCount`・
  `toolOnlyJobCount`・`retainedBytes`・`materializedPromptBytes`・
  `maxConcurrency`・`criticalPathMs`・`directToolOperations`・
  `workflowDefinitionBytes`・`parentVisibleInterfaceBytes`・
  `definitionReadBytes`・`directInvocationTurnsProxy`・
  `postprocessFailures`・`logRefetches`・`directUsageRecordCount`。
  route arm の byte/token dim は `route.*` の stable 値から
  `childOutputBytes`・`returnedBodyBytes`・`originalTokens`・
  `returnedTokens`・`logRefetchBytes` を参照する(§3.3 — route
  envelope を持たない arm でも record/log file 由来の観測源が
  固定されている)。
- `reduction` 宣言は `reductionBytes = leftStable − rightStable`・
  `reductionRatio = (left−right)/left` を emit(どちらか null ・
  left==0 なら null)。`definition-vs-compact-interface` の
  `parentVisibleReduction{Bytes,Ratio}`(definitionBytes −
  interfaceBytes)と route の「child 生出力 byte − parent-visible
  返却 byte」の削減率を同じ機構で表現する — route 側は
  `reduction: {left: "childOutputBytes", right:
  "returnedBodyBytes"}`(byte 軸)または `{left: "originalTokens",
  right: "returnedTokens"}`(token 軸)と宣言する(v1 の
  per-scenario 特例計算を一般化)。

#### 4.3 estimated-cost layer(opt-in)

- `--pricing <file>` で JSON `{model, inputUsdPerMTok,
  outputUsdPerMTok, cachedInputUsdPerMTok?, source,
  catalogRevision?}` を与えたときのみ estimated 系を emit。
  `source` は価格の出所を記す自由記述で必須(catalog resolve は
  harness 内では行わない — 明示前提のみ)。**file は load 時に
  fail-fast validation する** — malformed JSON・必須 key
  (`model`/`source`/`inputUsdPerMTok`/`outputUsdPerMTok`)欠落・
  非数値・未知 key は即座に error で終了する(中途半端な
  assumptions を record に残さない)。
- estimated entry は comparisons に別 entry として追加される
  (`claim:"estimated"` + `assumptions` echo + `deltas.estimatedCostUsd`
  = usage{input,output,cachedInput}×単価 の delta)。assumptions の
  `pricing` には **`model`(および宣言されていれば
  `catalogRevision`)を必ず echo する** — 「どの価格前提の推計か」
  が entry から辿れるようにする(§3.4 invariant #5)。
- scripted usage 由来の推計であることは assumptions の
  `boundary:"scripted-usage"` が記録する — 実料金比較として表示
  してはいけない(文書規則 — CI 強制不能面は review 対象)。
- `--pricing` 無しでは estimated entry を一切 emit しない。

#### 4.4 会計統一

- workflow の Worker usage は成功 envelope の `jobs[].meta` から
  **一度だけ**集計(workflow run は usage.jsonl に記録されない)。
- direct arm は各 CLI envelope + usage.jsonl 行数を別系統で記録し、
  token 集計への再加算をしない(v1 parity)。
- Route arm では `kind:"route"`/`"route-refetch"` record の件数を
  `directUsageRecordCount`・`logRefetches` として記録し、route
  envelope の usage と二重計上しない。
- 計測外 setup spawn(route-logs arm の route-run 等)が残す record
  も同一 cfgDir の usage.jsonl に残る — 件数は「その observation
  cfgDir に存在する全行」として armExpect で pin する
  (route-logs arm は setup route + 計測 refetch で 2 を期待)。
  setup 実行自体は wall-time 計測の外(§2.3 route-logs row)。
- 失敗系: usage-bearing error(usage を持つ envelope error)も
  record される点に留意 — usage.jsonl 件数と envelope usage の対応
  を observation に記録し、集計規則の例外として文書化する。

### 5. `mise run check` 統合

- `mise.toml` `[tasks.check]` から `node --test benchmarks/test/`
  (現行 :73)を削除し、justification comment(:61-62 —「benchmarks/
  test は node:test …check に配線する」)を書き換える: benchmark
  harness の unit test は `cargo test --workspace` 内の
  `cargo test -p cheaper-bench` が担う旨を記す。
- `node --test scripts/test/`(:72)は repo tooling test として
  **残る** — required node entrypoint は 6→5(#182 design §1.1 の
  非退行表の row 4 が migrated 扱いになる — §6 の addendum)。
- `benchmarks/test/*.mjs` 4 suite(aggregate・measurements・
  result-schema・scenario-manifest・計 312 行)は削除し、対応する
  coverage は `crates/cheaper-bench/tests/` 4 file へ移植する
  (§1.1 — manifest/fixture/schema/aggregate の純粋関数 test)。
- **`cargo test` は spawn 単位の測定実行を一切行わない** —
  cheaper-bench tests は manifest 検証・fixture round-trip・
  schema invariant・aggregate 純粋関数の unit test のみ。benchmark
  実測は手動 `cargo run -p cheaper-bench` のまま(`[[test]]`
  target への測定封入・`--ignored` 運用は ADR が不採用とした形態)。
- `cargo clippy --workspace --all-targets`・`cargo fmt --all` は
  cheaper-bench を自動で含む(workspace member)。check pipeline の
  `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` とは
  独立に、cheaper-bench 側の dev-seam は自 crate の dep edge が
  担う(reach-through 不要 — ADR 確定事項)。
- 実行契約(exit code mirror・log 保持・コマンド再実行防止)の
  回帰 test は既存 `tests/integration/route_run.rs`
  (`CHEAPER_ROUTE_CHILD_MODE` coverage)が継続して担い、
  cheaper-bench 側は fixture 単位 unit test で補完する — spec の
  acceptance 対応どおり spawn 測定自体は test 対象外。

### 6. Doc-sync list

実装と同一 change で同期する文書:

| file | 変更内容 |
| --- | --- |
| `mise.toml` | :73 `node --test benchmarks/test/` 削除・:61-62 comment 書換(§5) |
| `AGENTS.md` | check pipeline 記述(`node --test`(scripts・benchmarks)→ scripts のみ)・node tooling 列挙から `benchmarks/` を除去(harness は Rust crate へ — `benchmarks/` に残るのは言語中立資産のみ)・`crates/` 一覧に cheaper-bench 追加該当があれば記述 |
| `CONTRIBUTING.md` | pipeline 記述・test コマンド一覧(`node --test benchmarks/test/` 除去)・node tooling 記述・構造一覧(`benchmarks/` の説明を言語中立資産 + cheaper-bench 参照へ) |
| `benchmarks/README.md` | Rust harness 手順(`cargo build -p cheaper-cli --features dev-seam` + `cargo run -p cheaper-bench`)・fixture child/seam 説明・schema v2・`{fixtureChild}`・失敗/Route scenario・claim 境界・非上書き保存の説明へ全面書換 |
| `benchmarks/results/README.md` | schemaVersion 2・v1 file 不変・混在不可の方針を追記 |
| `docs/guides/workflow-usage.md` | Rust-era 測定を新規 artifact で引用・旧 TS 値は「TS 時代測定(legacy)」として区別表記・**削除しない**。新値は実装完了後に計測対象 binary で必須 12 scenario 全 arm を回した新 run の artifact からのみ採用(本 doc 時点では値を書かない)。`cheaper usage` 側の route record 分析は issue #184 の管轄で本表の対象外 |
| `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` | 日付付き addendum: §1.1 row 4(`node --test benchmarks/test/`)を migrated 扱い・§5 F2 が issue #183 へ absorb され実施済みである旨を記す |
| `biome.jsonc` | `"benchmarks/**/*.mjs"` dead glob 行を除去(`benchmarks/**/*.json`・`!benchmarks/results` は残存 scenarios/results 対象で維持) |
| `.pre-commit-config.yaml` | 変更不要(確認のみ)— biome mirror files pattern は残存 `benchmarks/**/*.json` に合致し続ける |
| 削除 | `benchmarks/run.mjs`・`benchmarks/lib/*.mjs`・`benchmarks/fixtures/fake-worker-stream.mjs`・`benchmarks/test/*.mjs` |
| 維持 | `benchmarks/scenarios/`(manifest v2 へ更新)・`benchmarks/results/`(v1 不変)・`benchmarks/README.md` |

## Data and Control Flow

1. `cargo run -p cheaper-bench` → `main()` が child dispatch
   (`--fixture-child`)を先に判定し、通常経路では arg parse →
   manifest load/validate(fail-fast)→ binary 解決 →
   **preflight probe**(dev-seam 有効の実証)→ `git rev-parse HEAD`。
2. scenario×arm×(warmups+reps) を逐次実行し、**observation 毎に**
   temp workspace + fresh `CHEAPER_CONFIG_DIR` を生成
   (files・`.cheaper/models.yaml`・policy.yaml merge・route 宣言の
   materialize → env 組立 PATH・HOME/USERPROFILE・`NODE_ENV=test`・
   `CHEAPER_TEST_FAKE_STREAM`・armEnv)→ arm impl 実行(各 spawn
   直前に fixture JSON を生成・`RunFixture::parse` round-trip;
   route-rewrite arm は spawn 前に `<cfg>/hook-eval/` 削除、
   route-logs arm は setup spawn を計測外で先行)→ record 組立て →
   validate。
3. 全 observation を集めて aggregate(arm 集計 + manifest
   comparisons + estimated layer)→ `benchmarks/results/<runId>.json`
   へ `create_new` 書込み。

## Non-Functional Constraints

- spawn は `std::process::Command` blocking で逐次 — wall-time は
  `Instant::now` 前後差(harness 内 arm を除く全計測は product
  binary subprocess で、CLI 起動 cost を含む)。
- temp workspace は `tempfile::tempdir` 等で隔離し、ws・cfgDir とも
  **observation(計測 spawn)毎に新規生成**する — scenario/arm/rep
  間で使い回さない(usage.jsonl・route-logs・hook-eval・seam
  registry 等の全副作用を observation 単位でリセットする粒度)。
  route-logs arm の「setup spawn → 計測 spawn」の 2 段のみ同一
  cfgDir を共有する(setup record が計測 obs の record 件数へ載る
  のは意図 — §4.4)。route-rewrite arm は各 spawn 直前に
  `<cfg>/hook-eval/` を削除する(§2.3)。ambient env passthrough は
  PATH・SystemRoot/TEMP/TMP・CHEAPER_LOG* allowlist のみ — v1
  parity。
- `NODE_ENV=test` は seam activation var として残る
  (`fake_stream.rs:31` の規則 — node 命名の vestige。rename は本
  決定の範囲外、§7 で明記)。
- `cheaper-bench` は product 配布物に含まれない(publish=false・
  workspace member のみ — cargo install 対象外)。

## Risks and Trade-offs

- **baseline 同等性**: `process-*` scenario の v1 wall-time は node
  起動 cost を含んでいた。fixture child(bench bin)化で絶対値は
  ずれる(arm 間 delta は相殺傾向)。`direct-filesystem` も
  `node:fs` → `std::fs::read` へ実装が変わる。**v1 vs v2 の値は直接
  比較せず、置換理由を結果・文書で別記する**(spec 要件) —
  `benchmarks/README.md` に migration note、record の
  `fixtureRevision:"seeded-v2"` で区別可能にする。
- **mode 名は rename しない**: `direct-worker-tool-call` arm が観測
  する mode は v1 どおり `single-direct-worker-with-one-tool-call`
  を継承する(§3.3 mode enum)。arm 名の floor 命名整理(§2.3)は
  manifest/impl 語彙側の話であり、record の mode は v1 との突合
  軸として値を据え置く — rename すると v1 file との arm 対応付け
  が壊れる。
- **`NODE_ENV=test` vestige**: seam activation が `NODE_ENV==="test"`
  を要求するのは TS 由来の命名残滓。Rust 移行後も残るため、
  README・result 記述で「seam activation の条件変数名」として明示
  する(rename は本決定の範囲外 — product 側変更になる)。
- **byte parity**: v1 byte 計測は `JSON.stringify` 由来。Rust
  `serde_json` の number formatting は edge case でずれるため、全
  byte 計測(`interfacePayloadBytes`・`retainedBytes`・definition
  bytes 等)を `cheaper_core::json::stringify::stringify` 経由で
  行い parity を保つ(canonical formatter = `json/number.rs`)。
- **nonzero-exit capture path**: v1 `runCli` は exit≠0 で throw
  するため失敗系計測に使えない。v2 `spawn.rs` は exit code・
  stdout/stderr を常に capture し、envelope parse は best-effort
  (stdout が 1 件の JSON envelope でなければ `envelopeOk` 判定の
  材料として扱い、harness error にはしない)。`armExpect` で期待
  失敗形を deterministic に pin する。
- **Windows/Unix parity**: fixture child は `current_exe()` abs
  path 直 spawn・shell 非経由で platform 差を排除。`osRelease`
  採取は platform 別 best-effort(失敗 null で測定を止めない)。
- **feature unification の罠**: `cheaper-cli/dev-seam` を
  cheaper-bench edge で宣言すると workspace build で product bin
  に seam が混入する — edge は default feature に固定し、計測用
  binary の dev-seam は必ず明示 build で行う(Cargo.toml comment
  + README で固定)。
- **cargo run は cheaper-bench を build するだけ**で product
  binary は build しない — 実行手順は明示的な 2 段
  (`cargo build -p cheaper-cli --features dev-seam` →
  `cargo run -p cheaper-bench`)。preflight probe が seam 欠落を
  fail-fast で検出する。

## Deferred Design Concerns

- `fixture.rs` への `Serialize` 付与による型直接生成 — ~20 型へ
  の impl・untagged `FixtureStep` 正準出力形の設計判断・dev-seam
  carve-out を要し、採用時は `crates/cheaper-runtime/src/dev/
  fixture.rs` が変更対象(ADR で見送り済みの refinement 候補)。
- actual parent usage の取得経路(外部 telemetry が存在した場合の
  `observed` 配線) — 現状は常に `unavailable`。取得経路ができた
  時点で `parentUsage.source` を wire する contract 拡張。
- `process-*` scenario の process task 実行は policy process.tasks
  経由 — fixture child 以外の managed process 語彙を増やす必要が
  出た場合は scenario 側の宣言拡張として扱う。

## Verification Notes

- [ ] `mise exec -- cargo run --release -p cheaper-bench -- --scenario
  <id> --warmups <n> --repetitions <n>` が schemaVersion 2 の
  result JSON を `benchmarks/results/` へ非上書き保存する。
- [ ] `cargo test -p cheaper-bench` が `cargo test --workspace` 内で
  manifest・fixture・schema・aggregate の unit 検証を行う(spawn
  測定なし)。
- [ ] `{node}`・`process.execPath`・`node:fs`・`environment.node`
  が新 harness に存在しない(`git grep` 確認)。
- [ ] preflight probe が dev-seam 無し binary を fail-fast で検出
  する。
- [ ] `mise run check` が `node --test benchmarks/test/` を除いた
  pipeline で全 pass する。
- [ ] parentUsage unavailable 時に親 token/費用削減を主張する表示が
  record・aggregate・文書に存在しない(schema invariant #6)。
- [ ] Route(rewrite→routed-exec→condense)と失敗 matrix が
  executable local fixture で再現される。

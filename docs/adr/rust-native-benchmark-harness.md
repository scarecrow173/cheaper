---
id: "ADR-tBRMKuj5yPFMYM57an5HpS"
type: "adr"
status: "accepted"
title: "Rust-native benchmark harness for Worker/Workflow/Route measurement"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
    - "../discovery/benchmark-harness-audit-issue-183.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/benchmark-redesign-scope-issue-183.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/benchmark-redesign-scope-issue-183.md"
    - "../designs/benchmark-harness-redesign-issue-183.md"
  refines:
    - "./benchmark-harness-placement-and-accounting.md"
  refined-by: []
  derives-from:
    - "../discovery/benchmark-harness-audit-issue-183.md"
    - "../specs/benchmark-redesign-scope-issue-183.md"
  derived-by:
    - "../designs/benchmark-harness-redesign-issue-183.md"
    - "../plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "../test-specs/cheaper-bench-harness-contract-issue-183.md"
  verifies: []
  verified-by: []
  references:
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "../discovery/workflow-orchestration-benchmark.md"
    - "../specs/workflow-orchestration-benchmark.md"
    - "benchmarks/README.md"
    - "mise.toml"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Rust-native benchmark harness for Worker/Workflow/Route measurement

## Context and Problem Statement

issue #183 は benchmark harness の再設計を要求する: 現行 Node
harness(`benchmarks/` 配下 .mjs 計 ≈1.56k 行 — runner+lib+
fixture gen ≈1.25k + `test/` 312)の棚卸し、#182 の toolchain
決定(Option A — Node 維持)を踏えた architecture 採択、
Route・失敗系・claim 境界を含む測定拡張。`.mjs` 継続は要件ではなく、製品に Node 依存を追加しない。

前提となる事実(詳細は
`docs/discovery/benchmark-harness-audit-issue-183.md`):

- harness は動作しているが、測定対象は workflow vs direct の 8 arm
  実装に限られる。issue が要求する Route(rewrite→routed-exec→
  condense)・失敗 matrix(worker failure/timeout・nonzero exit・
  postprocess failure・truncation/log refetch)・claim 境界 field
  は現行に存在せず、**harness の大改修は言語選択と独立して必須**。
- 2 scenario が `{node}` → `process.execPath` 置換で emit.mjs を
  spawn し(node を計測対象 workload として使用)、結果 record の
  `environment.node` field(未検証)が node 結合を記録している。
- `benchmarks/fixtures/fake-worker-stream.mjs` は RunFixture DSL
  の JS 平行実装で、DSL 全 11 step 種のうち 5 種のみ生成する。
  失敗 matrix に必要な error/hang/panic/abort step を使うには
  平行実装の拡張(=drift 面拡大)が必要になる。一方
  `dev::fixture` 型群は **Deserialize-only**(Serialize impl 無し
  — `fixture.rs:71`、`RunFixture::load`/`parse` は parse 専用
  :138-157)で、`FixtureStep` は bare string/keyed object の
  untagged 二形(:418-470)のため、型から生成する正準出力形も
  定義されていない。
- #182 design §5 F2 は benchmark harness の Rust 化を
  「**issue #183 への結合が前提**」として本 issue へ deferred
  し、trigger T2(scenario・計測面の大改修)は発火済み。
- `node --test benchmarks/test/` は `mise run check` の 6 件の
  required node entrypoint の 1 つ。
- **どの選択肢を取っても `mise.toml` の `node` pin は残る**
  (upstream 所有の doc-suite `.js` が agent doc workflow で
  required)。harness 言語の決定は node removal payoff ではなく
  harness 自体の適合性で評価する。

## Decision Drivers

- RunFixture DSL(`cheaper-runtime::dev::fixture`、serde 定義)を
  fixture 生成の canonical schema として再利用できるか — 型は
  Deserialize-only のため、生成 JSON を `RunFixture::parse` で
  round-trip 検証する anchor 構成で JS 平行実装の drift を
  避けられるか。
- `{node}` workload・`environment.node` を runtime 非依存に
  再設計できるか(self-re-exec fixture child は proc_process.rs・
  route_run.rs・workflow_dev_seam.rs で実証済み)。
- Route 測定に必要な面(rewrite verdict・payload mint・grant 付き
  fixture child・route logs)と harness の親和性。
- wall-time 意味論の保存 — 「CLI 起動 overhead と workflow
  overhead の分離」には product binary の subprocess spawn が
  必須で、in-process `run_cli` は使えない。
- baseline 同等性 — Node vs Rust の差を cheaper の改善と誤認
  しないよう、意味の変わる baseline の置換理由を別記できるか。
- `mise run check` への影響 — `node --test benchmarks/test/` の
  行き先(required entrypoint 6→5 だが removal payoff は依然
  ゼロ)。
- KISS/YAGNI — 必要なのは e2e CLI 計測であって、統計機構や
  新 toolchain pattern の導入ではない。

## Considered Options

- **Option A: Node harness を維持して拡張する**。`run.mjs`/`lib/` を
  Route・失敗系・claim 境界へ拡張し、`{node}` workload だけを
  fixture child へ置き換える。
- **Option B: Rust native harness — dev-only workspace member
  `crates/cheaper-bench`**。bin(`cargo run -p cheaper-bench`)+
  unit test を持つ専用 crate を新設し、runner・arm 実装・計測・
  集計・schema 検証を Rust へ移植する。`benchmarks/` には
  言語中立資産(scenarios・results・README)を残す。
- **Option C: hybrid**。Rust 計測 bin + `.mjs` driver、または
  cargo micro-bench + `.mjs` e2e のような 2 系統構成。

形態の sub-option(Option B 内): dedicated crate、xtask、
cheaper-cli の `[[bin]]`、`[[test]]` target、criterion/`cargo bench`。

## Decision Outcome

**Option B — Rust native harness** を採用する。形態は **dev-only
workspace member `crates/cheaper-bench`(bin + unit tests)** とする。

理由は次のとおり。

- **issue の測定拡張面が Rust 側資産と一致する**。Route の
  executable local fixture(明示 grant・fake Worker・fixture
  child)は `tests/integration/route_run.rs` が in-process で実証
  済みの pattern で、同じ機構を spawn 計測へ転用できる。
  `route::payload::encode` は pub で、`route run --payload` の
  token mint に外部 seam が要らない。失敗 matrix の error/hang/
  panic/abort は RunFixture DSL に既に存在し、fixture 生成は
  `serde_json::json!`/template 組立て + `RunFixture::parse`
  round-trip 検証で canonical schema へ fail-fast anchor する
  方式とする — seam module 変更ゼロ(後述「fixture 供給方式の
  確定」)。
- **JS 平行実装の drift を止められる**。`.mjs` generator は DSL
  の subset を独自組立てしており、失敗系 step の追加は平行実装を
  さらに育てる。Rust 化は fixture JSON を必ず canonical schema
  (`RunFixture::parse`)に通す構成にし、無効な step 語彙を生成時
  に fail-fast で検出する — fixture 供給の検証 anchor を単一
  source に戻す(表現ドリフトの解消 — repo 開発原則)。
- **`{node}` workload と `environment.node` を自然に消せる**。
  harness binary 自身を fixture child として再実行すれば
  emit/exit-code/large-output を runtime 非依存で供給でき、
  「JS runtime 前提の fixture/process arm をなくす」という issue
  要件を満たす。結果 record は schemaVersion 2 へ bump し、
  `environment.node` 除去と arm/dimension 追加を version 付きで
  明示する(旧 v1 JSON は改変しない — append-only 方針と整合)。
- **required node entrypoint が 6→5 に減る**。`node --test
  benchmarks/test/` は `cargo test --workspace` の cheaper-bench
  unit test へ吸収される。removal payoff は依然ゼロだが、
  F2 acceptance sketch(「`node --test benchmarks/test/` が cargo
  test 系へ置換される」)と一致する。
- **書き換えコストは正直に計上した上で許容する**。移植面は
  runner+lib+fixture gen の ≈1.25k 行(+test 312 行)だが、これに
  再利用実績の無い新規実装が乗る — self-re-exec child mode 群
  (~100–200 行)・fixture JSON 生成・result schema v2・manifest
  v2・nonzero-exit capture 経路(現行 `runCli` は exit≠0 で
  throw するため失敗系計測に capture 経路の新設が必須)。
  Option A 比の marginal は **≈1.5–3.5k 行 + schema v2 parity
  再検証**であり、実装量差は小さくない。それでも Option B を
  取る根拠はコスト最小化ではなく、fixture 供給の単一 source
  化による drift 解消と、Route・失敗系・fixture child 計測面
  への適合である。

### fixture 供給方式の確定(Deserialize-only への対応)

`cheaper_runtime::dev::fixture` の型群は **Deserialize のみ**
(`fixture.rs:71` — Serialize impl 無し、`RunFixture::load`/`parse`
は parse 専用 :138-157)。`FixtureStep` は bare string/keyed
object の untagged 二形(:418-470)を持ち、型から生成した場合の
正準出力形は定義されていない。したがって fixture JSON の生成は
**`serde_json::json!`/template で組立て、書込み前に必ず
`RunFixture::parse` で round-trip 検証する**方式とする。生成物は
canonical schema へ fail-fast で anchor され、seam module
(`fixture.rs`)への変更はゼロで済む — `tests/fixtures/*.json`
24 件の hand-authored fixture が同一経路で検証されている。

型への `Serialize` 付与による直接生成は将来の refinement 候補
として残るが、~20 型への impl 追加・untagged dual step 形の
正準出力形という設計判断・dev-seam carve-out を要し、採用する
場合は `crates/cheaper-runtime/src/dev/fixture.rs` が affected
paths に追加される。本決定では見送る。

### Node 維持を選ばなかった理由の明記(issue 要求)

#182 Option A は「required entrypoint と upstream doc-suite が
node を要する以上、node pin は残る」という判断であり、
**benchmarks の言語を拘束する決定ではない**。むしろ #182
design は F2 を本 issue へ deferred している。Option B は
Option A と矛盾しない — product path の node-free は不変で、
dev-tooling の node pin も残る。harness が Rust 化しても
`scripts/`・doc-suite・codegen の node 依存は残存するため、
`mise.toml` `node = "26"` は維持する。

### Consequences

* Good, because fixture 供給が `RunFixture::parse` の canonical
  schema 検証に anchor され(生成は `json!` 組立て + parse
  round-trip)、失敗 matrix の全 step 種を drift なしで使える。
* Good, because `{node}` workload・`environment.node` が除去
  され、計測対象 workload としての node 結合が消える。
* Good, because Route・失敗系の新規計測面が Rust 側の実証済み
  test pattern(self-re-exec・dev-seam fixture・payload encode)
  と同じ機構で実装できる。
* Good, because `node --test benchmarks/test/` が cargo test へ
  吸収され、check pipeline の node step が 1 件減る。
* Bad, because ≈1.25k 行(runner+lib+fixture gen)+ 312 行 test
  の移植に加え、新規実装分(child mode・fixture gen・schema/
  manifest v2・nonzero-exit capture)を含め Option A 比 marginal
  ≈1.5–3.5k 行のコストと schema v2 の parity 再検証が発生する。
* Bad, because `cargo run -p cheaper-bench` は dev-seam 付きの
  product binary build を前提とし、benchmark 実行手順が
  `cargo build --features dev-seam` + `cargo run` の 2 段になる
  (README で固定する)。
* Neutral, because `mise.toml` の node pin・doc-suite・他 5
  entrypoint は不変。node removal payoff は Option によらずゼロ。

### Confirmation

- `mise exec -- cargo run --release -p cheaper-bench -- --scenario
  <id> --warmups <n> --repetitions <n> --output <path>` が raw
  observation + median/p95 + aggregate を schemaVersion 2 で
  `benchmarks/results/` へ保存すること(既存 file を上書きしない)。
- `cargo test -p cheaper-bench` が `cargo test --workspace` 内で
  aggregate・schema・manifest の unit 検証を行うこと(測定実行
  自体は test 対象外で手動のまま)。
- 計測 arm が product binary を subprocess spawn すること
  (in-process `run_cli` ではない — CLI 起動 cost を含む
  wall-time 意味論の保存)。
- `benchmarks/results/` の旧 v1 JSON が不変のまま残ること。
- `mise run check` が `node --test benchmarks/test/` を除いた
  pipeline で全 pass すること(mise.toml の対応行を除去し、
  代替が cargo test に含まれること)。

## Implementation Plan

* Affected paths:
  - 新設: `crates/cheaper-bench/`(workspace member・publish=
    false・bin + `tests/`)。依存: `cheaper-cli`(route
    `payload::encode`)・`cheaper-runtime`(`dev::fixture` 型 —
    dev-seam)・serde_json/tempfile 等。
  - 維持: `benchmarks/scenarios/`(manifest は schemaVersion を
    上げて Route・失敗 scenario を追加)・`benchmarks/results/`
    (v1 JSON 不変・append-only)・`benchmarks/README.md`
    (実行手順・seam 説明・schema v2 方針を更新)。
  - 削除: `benchmarks/run.mjs`・`benchmarks/lib/*.mjs`・
    `benchmarks/fixtures/fake-worker-stream.mjs`・
    `benchmarks/test/*.mjs`。
  - 変更: `mise.toml`(`node --test benchmarks/test/` 行 :73 と
    その justification comment :61-62)・`AGENTS.md`(check
    pipeline 記述 :36-37・node tooling 列挙 :83)・
    `CONTRIBUTING.md`(pipeline :27・test コマンド一覧 :51-52・
    node tooling 記述 :56・構造一覧 :107-108)・`biome.jsonc`
    (`"benchmarks/**/*.mjs"` dead glob :23 —
    `benchmarks/**/*.json` と `!benchmarks/results` は残存
    scenarios/results 対象で維持)・
    `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
    (§1.1 row 4 を migrated 扱いとし、§5 F2 が #183 に absorb
    された旨を記す日付付き addendum)・
    `docs/guides/workflow-usage.md`(Rust baseline 再測定へ
    差替え・TS 時代値との区別)・`benchmarks/results/README.md`
    (schema v2 方針)。
  - 確認のみ(変更不要): `.pre-commit-config.yaml` の biome
    mirror files pattern(:35)— 残存 `benchmarks/**/*.json`
    (scenarios)に合致し続ける。
* Patterns to follow: spawn 計測(product binary + isolated
  `CHEAPER_CONFIG_DIR` + `NODE_ENV=test` + `CHEAPER_TEST_FAKE_STREAM`
  fixture JSON)・self-re-exec fixture child(`current_exe()` +
  mode env)・dev-seam feature gate・immutable result write・
  fail-fast schema validation・`parentUsage.status=unavailable`
  の強制・fixture JSON の `json!` 組立て + `RunFixture::parse`
  round-trip(上記「fixture 供給方式の確定」)・byte 計測の JS
  互換 parity(`cheaper-core::json` の canonical number
  formatter — `json/number.rs`。v1 は `JSON.stringify` 由来で、
  serde_json の数値 formatting とは edge case でずれる)。
* Patterns to avoid: in-process `run_cli` による wall-time 計測、
  product crate への benchmark bin 同梱、`[[test]]` target 化
  (`cargo test --workspace` が測定を実行する)、criterion/bench
  基盤の導入、`node --test` 形式の test 追加、旧 v1 JSON の
  改変、親 usage の推計表示。
* Dependencies or configuration:
  - `cheaper-bench` の Cargo.toml は
    `cheaper-runtime = { path = "../cheaper-runtime", features = ["dev-seam"] }`
    を宣言する — `dev::fixture` 型群が
    `#[cfg(feature = "dev-seam")]` 配下(lib.rs:40-41)で、型を
    名指す crate 自身が edge で feature を有効化する必要が
    ある。これにより `cargo run`/`cargo test -p cheaper-bench`
    は `--features` 無しで動く。
  - `cheaper-cli` への依存(`route::payload::encode` — feature
    非 gate の pub fn)は **default feature のまま**とする。
    `cheaper-cli/dev-seam` をこの edge で宣言すると feature
    unification で `cheaper` bin 自体が seam 有効 build になり、
    計測対象 product binary への seam 付与が暗黙化するため
    禁止。計測用 binary の dev-seam 付与は必ず明示 build
    (`cargo build -p cheaper-cli --features dev-seam`)で行う。
  - `cheaper-bench` は独自 feature を定義しない — `mise run
    check` の `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
    は seam 依存 test target 向けであり、cheaper-bench 側の
    dev-seam 有効化は自身の dep edge が担う(reach-through
    不要)。
  - spawn 対象 binary は dev-seam 有効 build(release 優先・
    debug fallback・`--binary` で明示 override)。fixture child
    化は cheaper-bench bin 自身の self-re-exec(`current_exe()`
    + mode env)で賄い、追加 feature は要らない。
* Scenario 意味論の移行 note(design-doc へ引継ぎ):
  - `{node}` placeholder は manifest v2 で rename する(例
    `{bench}`/`{fixtureChild}` — spawn 先が fixture child に
    変わるため名実を合わせる)。
  - `process-*` scenario の v1 wall-time は node 起動 cost を
    含んでいた — fixture child 化で絶対値はずれる(arm 間
    delta は相殺される)。baseline 同等性の別記対象として
    結果・文書で明示する。
  - `processScript` spawn(`process.execPath`)と `node:fs`
    read は fixture child / `fs::read` で置き換える —
    `direct-filesystem` baseline の再定義は discovery の
    open question と一体で design-doc が決める。
  - `NODE_ENV=test` は seam activation var として残る
    (fake_stream.rs:31)— node 命名の vestige として文書で
    明示し、rename は本決定の範囲外。
* Note — 既存 ADR との関係:
  `docs/adr/benchmark-harness-placement-and-accounting.md` が決めた
  「root `benchmarks/` 配置・built CLI spawn・envelope 会計・
  usage.jsonl 分離」は全て維持し、supersede するのは「plain
  **Node** runner」という実装形態のみ(旧 ADR は TS CLI 時代の
  決定であり、harness 言語の再決定は本 issue の範囲)。この
  部分的な改訂関係は front-matter の `refines` で構造化済み。

## Verification

* [ ] 現行 Rust binary(dev-seam build)で Worker/Workflow/Route
  代表ケースが再測定でき、raw observation + median/p95 が
  schemaVersion 2 で保存される。
* [ ] `{node}`・`process.execPath`・`node:fs`・`environment.node`
  が新 harness から除去されている(`git grep` で確認)。
* [ ] 成功/失敗・nonzero exit・postprocess failure・truncation/
  log refetch の scenario が fixture で再現され、exit code
  mirror・log 保持・再実行防止の回帰 test がある(既存
  `tests/integration/route_run.rs` の `CHEAPER_ROUTE_CHILD_MODE`
  coverage が route 実行契約を担保 + cheaper-bench の fixture
  単位 unit test。spawn 測定自体は test 対象外のまま)。
* [ ] `cargo test --workspace` が cheaper-bench unit test を含み
  `mise run check` が全 pass する。
* [ ] parentUsage unavailable 時に親 token/費用削減を主張する
  表示が結果・集計・文書に存在しない。
* [ ] `docs/guides/workflow-usage.md` が Rust baseline を引用し
  TS 時代測定と区別している。

## Pros and Cons of the Options

### Option A: Node harness 維持+拡張 — 不採用

* Good, because 稼働資産・test 済み schema/aggregate の再利用、
  JSON 操作の簡潔さ、実績のある cross-platform spawn 処理。
* Bad, because fixture generator を DSL 残 7 step へ拡張すると
  JS 平行実装が育ち、Route executable fixture・claim 境界を
  全て JS 新規実装することになる。`{node}` workload の除去自体
  は Node harness でも可能(Rust fixture child を spawn すれば
  よい)— issue の「JS runtime 前提の fixture/process arm を
  なくす」は「Rust へ移す場合」の条件付き要件であり、Option A
  はこれに違反しない。実質の不採用理由は parallel-DSL drift・
  harness 自体の node 前提存続・`node --test benchmarks/test/`
  の残留である。

### Option B: Rust native harness(`crates/cheaper-bench`)— 採用

* Good, because fixture 供給が RunFixture canonical schema に
  anchor され drift が解消・`{node}`/`environment.node` 除去・
  Route/失敗面が Rust 実証済み pattern と一致・required node
  entrypoint 6→5・issue の要件面と一致。
* Bad, because port 面 ≈1.25k 行 + test 312 行に新規実装分が
  乗り、Option A 比 marginal ≈1.5–3.5k 行 + schema v2 parity
  検証コスト、2 段 build/実行手順。node pin は残り removal
  payoff はゼロ(本決定の動機ではない)。

### Option C: hybrid — 不採用

* Bad, because 会計・schema・claim 境界が 2 系統に割れて最悪の
  保守面になり、一貫した測定責務を持てない。

## More Information

* 上流 audit: `docs/discovery/benchmark-harness-audit-issue-183.md`
  (file 別責務・7 scenario/20 arm・schema v1・Rust 側資産・
  決定空間)。
* scope spec: `docs/specs/benchmark-redesign-scope-issue-183.md`。
* 依存判断: `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
  (Option A — 本 ADR と矛盾しない)・
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
  §5 F2(本決定が absorb する deferred item)。
* 先行 chain(別 issue): `docs/adr/benchmark-harness-placement-and-accounting.md`
  (配置・会計の原決定 — runner 形態のみ本 ADR が置き換える)。

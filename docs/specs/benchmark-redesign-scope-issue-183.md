---
id: "SPEC-hLFwwZHikKoZ5cMu7GM8oG"
type: "spec"
status: "approved"
title: "Benchmark redesign scope — Worker/Workflow/Route harness (issue #183)"
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
  implements: []
  implemented-by:
    - "../adr/rust-native-benchmark-harness.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/rust-native-benchmark-harness.md"
    - "../designs/benchmark-harness-redesign-issue-183.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/benchmark-harness-audit-issue-183.md"
  derived-by:
    - "../adr/rust-native-benchmark-harness.md"
    - "../designs/benchmark-harness-redesign-issue-183.md"
  verifies: []
  verified-by: []
  references:
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../specs/workflow-orchestration-benchmark.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "benchmarks/README.md"
    - "docs/guides/workflow-usage.md"
    - "mise.toml"
  defers: []
  deferred-by: []
---

# Benchmark redesign scope — Worker/Workflow/Route harness (issue #183)

## What Should Be Built

issue #183 の成果物は次の 4 点。

1. **再設計された benchmark harness** — `docs/discovery/benchmark-harness-audit-issue-183.md`
   の監査を踏まえ、architecture(Node 維持 vs Rust native)を
   `docs/adr/rust-native-benchmark-harness.md` で採択した形態で実装する。
   harness は実行可能なローカル fixture のみで動作し、実 provider・
   実 Cursor/Copilot・外部 CLI を前提にしない。
2. **拡張された計測面** — direct canonical tool/CLI vs Worker vs
   saved Workflow、fan-out/fan-in、large intermediate output、Route
   の rewrite→routed-exec→condense、および成功/失敗の
   representative matrix を scenario として分離して測る。
3. **claim 境界の強制** — measured / estimated / unavailable の
   3 層分類を result schema と集計・文書で一貫させる。
4. **更新された文書と回帰 test** — `docs/guides/workflow-usage.md`
   を Rust 現在形の測定で書き換え(TS 時代値と区別)、
   `benchmarks/README.md`・`mise.toml` check pipeline・
   実行契約の回帰 test を整合させる。

## Why Now

- `docs/guides/workflow-usage.md` の掲載値は TS CLI 時代
  (rev `b3213f8c`・`node packages/cli/dist/main.js`・workflow
  median ≈2,795 ms)の記録であり、Rust binary(部分 run で workflow
  median ≈154 ms)を反映していない。
- #182 は Option A(Node 維持)を採択したが、benchmark harness の
  Rust 化(F2)は「issue #183 への結合が前提」として本 issue へ
  明示的に deferred されている。scenario・計測面の変更は避けられ
  ないため、trigger T2 は発火済み。
- Route(rewrite→routed-exec→condense)は product 機能として存在
  するが計測面に一切載っていない。Route の削減効果と
  orchestration overhead を fixture で検証するのは本 issue が初。

## Users and Value

- maintainer: Worker/Workflow/Route の削減効果と overhead を、
  主張可能な範囲(measured/estimated/unavailable)の区別付きで
  判断できる。
- contributor: scheduler・prompt・workflow engine・route 層の変更
  前後で deterministic な token/byte 面と wall-time 面の回帰を
  比較できる。
- doc 読者(consumer): `workflow-usage.md` の指針が現行 Rust
  binary の測定に基づくと信頼できる。

## Scope

### In Scope

- **harness architecture の採択と実装** — ADR の選択(推奨:
  dev-only workspace member `crates/cheaper-bench`、Node 維持を
  選ぶ場合は理由を明記)。`.mjs` 継続は要件ではない。
- **現行契約の保存** — `warmups`/`repetitions`/`scenario`/`output`
  相当の CLI、arm ごとの isolated temp workspace/config
  (CHEAPER_CONFIG_DIR 分離・ambient env の最小 passthrough)、
  明示 binary 選択(profile fallback + override)、source
  revision・fixture revision・schemaVersion・OS・CPU・arch・
  build profile・feature・測定条件・median/p95 の記録、衝突しない
  immutable な result 保存(上書き禁止)、fail-fast schema 検証と
  その test。
- **fake-stream fixture の再利用** — `CHEAPER_TEST_FAKE_STREAM` +
  `NODE_ENV=test` + `dev-seam` cargo feature の declarative
  RunFixture JSON を fixture 供給源とする。fixture DSL
  (`sessions[].steps` の error/hang/panic/abort 等を含む全 step
  種)を scenario の失敗 matrix に使う。fixture JSON の生成は
  `serde_json::json!`/template 組立て + `RunFixture::parse` での
  round-trip 検証を必須とする(`dev::fixture` 型は
  Deserialize-only — canonical schema への fail-fast anchor を
  parse に置く方式。Serialize 付与による型からの直接生成は
  将来の refinement であり、採用時は `fixture.rs` が変更対象に
  加わる — ADR で固定)。
- **測定拡張(issue §2)** —
  - direct canonical tool/CLI vs Worker vs saved Workflow を分けて
    測る。
  - fan-out/fan-in、large intermediate output の scenario を維持・
    拡張する。
  - Route を `route resolve`(dry-run)/hook verdict
    (rewrite)/`route run`(routed-exec→condense)/`route logs`
    (refetch)で測る。実行可能なローカル fixture — 明示 grant・
    fake Worker・fixture child コマンド実実行 — を使い、実
    Cursor/Copilot や host approval bypass の保証は含めない。
  - 同一入力・同一子コマンド・同一設定を基準に、parent-visible
    returned token/byte・original output・削減率・worker
    token/turn/toolCalls・tool/workflow/route の wall-time/p95・
    成功率・postprocessFailures・log refetch を記録する。
  - representative cases: 小出力(要約不要)・大出力(要約あり)・
    worker failure/timeout・非ゼロ exit・postprocess failure・
    truncation/log refetch。exit code mirror・log 保持・コマンド
    再実行防止を検証対象に含める。
- **会計統一** — `usage.jsonl` 由来の run/route/route-refetch
  record と各 CLI envelope の数え方を統一し、worker usage の
  二重計上を禁止する(workflow envelope 集計と usage.jsonl
  record の別系統規則を維持)。fake provider の scripted tokens
  は実料金・実モデル効率の観測値ではない旨を record/schema・
  文書で明示する。
- **claim 境界(issue §3)** —
  - **measured** = cheaper に流入・返却した byte/token(同一
    tokenizer・同一境界)と実行時間・成功率。
  - **estimated** = 明示的な価格前提を付記した費用比較のみ。
    モデル入出力価格・cache・料金体系の差や baseline 不明時は
    結果を `unavailable` にする。
  - **actual parent usage** = 親 agent usage が同条件で外部から
    取得できる場合のみ `observed`。取得できない場合
    `parentUsage.status=unavailable` とし、`outputBytes`・CLI
    起動回数を親 token 削減・費用削減と断言しない。
  - **record 検証での強制(invariant)** — CI で強制可能な面は
    result record/aggregate の schema validation に限定される。
    schema v2 validation は次を fail-fast で強制しなければ
    ならない: `unavailable` 入力上に数値 delta を emit する
    `comparisons[]` entry の拒否、`estimated` entry への明示的
    assumptions record(価格前提・tokenizer・境界)の要求、
    claim 分類 enum の {measured, estimated, unavailable} への
    限定、fake provider の scripted usage を未標識で記録する
    observation の拒否(`usage.source:"scripted"` 等の marker
    を必須化)。表示・文面側の規則は CI では強制できないため、
    review 対象の文書規則として位置づける。per-dimension の
    field 設計(claim 分類の表現形・assumptions record の
    shape)は design-doc に委ね、強制面は record validation に
    置く。
- **stdout 圧縮のコスト明記** — 要約(condense)には分析コスト・
  遅延が伴うことを測定・文書双方で明示する。
- **文書・pipeline 整合** — `benchmarks/README.md`・
  `docs/guides/workflow-usage.md`(旧 TS 測定と新 Rust 測定の
  区別)・結果 dir README に加え、移行で陳腐化する以下を同時に
  更新する: `mise.toml`(`node --test benchmarks/test/` 行 :73
  とその justification comment :61-62)、`AGENTS.md`(check
  pipeline 記述 :36-37・node tooling 列挙 :83 の
  `benchmarks/`)、`CONTRIBUTING.md`(pipeline :27・test
  コマンド一覧 :51-52・node tooling 記述 :56・構造一覧
  :107-108)、
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
  (§1.1 row 4 を migrated 扱いとし、§5 F2 が issue #183 に
  absorb された旨を記す日付付き addendum)、`biome.jsonc`
  (`"benchmarks/**/*.mjs"` dead glob :23)。
  `.pre-commit-config.yaml` の biome mirror files pattern(:35)
  は残存 `benchmarks/**/*.json`(scenarios)に合致し続けるため
  変更不要(確認のみ)。
- **旧結果の扱い** — `benchmarks/results/` の既存 JSON は改変
  しない。schema 互換なら継承、非互換なら新 schemaVersion と
  移行方針を示す。

### Out of Scope

- 実 Cursor/Copilot を用いた #162 型の実機検証、および host
  approval bypass に関する保証。
- `cheaper usage` の Route 別詳細分析 — issue #184 の管轄。
  既存 telemetry(usage.jsonl schema・record 形)の仕様変更。
- 実 provider・外部 CLI・Ollama 等を CI/check の必須前提に
  すること。
- `mise.toml` の `node` pin 除去 — doc-suite 等が残る限り #182
  Option A の gate 未達で、本 issue の対象外。
- product runtime・envelope・worker schema への変更(harness は
  観測側のみを実装する)。
- Rust への翻訳自体を成果とすること(issue 非目標 — architecture
  は測定要件への適合で選ぶ)。

## Requirements

- 再測定は現行 Rust binary(dev-seam feature 有効 build)で行い、
  run ごとの raw observation と median/p95 を保存する。
- baseline 同等性: Node vs Rust の差を「cheaper の改善」と誤認
  しないよう、arm 間で意味が変わる baseline(例: `node:fs`
  read・`{node}` process workload・`process-*` wall-time に
  含まれていた node 起動 cost)の置換理由を結果または文書で
  別記する。JS runtime 前提の fixture/process arm は Rust 移行時
  に runtime 非依存へ再設計する。
- fixture の意味論・再現性・schema validation を維持するか、変更を
  version 付きで明示する。
- scenario 追加・arm 変更時に manifest 検証(required coverage・
  armEnv/armBase 相当の整合)が fail-fast で機能する。
- 計測は deterministic な seeded fixture を使い、token/byte 面は
  反復間で一致することを回帰 anchor とする。byte 計測は v1 の
  `JSON.stringify` 意味論との parity を維持する(JS 互換
  canonical number formatter — `cheaper-core::json`)。
- Route 測定は local executable fixture(明示 grant・fake Worker・
  子コマンド)で完結し、外部 agent・approval bypass を前提に
  しない。
- `mise run check` / CI が再現可能であり、採択した Node 方針と
  矛盾しない(Rust 採択時は `node --test benchmarks/test/` の
  役割が cargo test 系へ引き継がれる)。
- 実行契約(exit code mirror・log 保持・コマンド再実行防止)の
  回帰 test は、既存 `cheaper-cli` route integration test
  (`tests/integration/route_run.rs` — `CHEAPER_ROUTE_CHILD_MODE`
  経由で同契約を検証済み)と cheaper-bench の fixture 単位
  unit test(schema/aggregate/manifest 検証)で充足する。
  spawn 単位の測定実行自体は `cargo test` 対象外(手動実行)
  のままとする。
- parent usage 未取得時に「トークン費用を x% 削減」の類を結果・
  集計・文書のいずれでも表示しない。

## Acceptance Criteria

issue 本文の Acceptance Criteria への対応:

- [ ] #182 の依存判断を反映した runner/fixture/依存構成(選択
  理由付き)が実装され、Node 維持理由がある場合明記される —
  ADR が選択根拠を固定し、実装がそれに従う。
- [ ] 現行 fixture の意味論・再現性・schema validation を維持する
  か、変更を version 付きで明示する — result schema・scenario
  manifest の version 方針が文書化される。
- [ ] 現行 Rust binary で Worker/Workflow/Route の代表ケースを
  ローカルで再測定でき、run ごとの raw observation と
  median/p95 が保存される。
- [ ] 成功/失敗・nonzero exit・partial/truncated・postprocess
  failure を測定し、実行契約(exit code mirror・log 保持・
  コマンド再実行防止)の回帰 test が存在する — 既存
  `route_run.rs` の route 契約 coverage + cheaper-bench unit
  test で充足し、測定実行は test 対象外(手動)を維持する。
- [ ] parent usage 未取得時に「トークン費用を x% 削減」と誤表示
  しない — `parentUsage.status=unavailable` の fail-fast 検証と、
  集計・文書の表示規則が維持される。
- [ ] 測定結果を `docs/guides/workflow-usage.md` へ反映し、旧 TS
  測定と新 Rust 測定を区別する。
- [ ] `mise run check` / CI と専用 benchmark test が再現可能で、
  Node 移行方針と矛盾しない。
- [ ] (doc chain) 本 spec・discovery・ADR が各 dir の README index
  に登録され、`audit_docs.js` で findings が無い。

## Deferred Design Concerns

- result schema v2 の field 設計(claim 境界の per-dimension 表現・
  environment 置換・Route/失敗 dimension)と scenario manifest v2
  — 実装前の design-doc で固定する。
- actual parent usage の取得経路(外部 telemetry が存在した場合の
  `observed` 配線) — 現状は常に unavailable で、取得経路ができた
  時点で contract を拡張する。
- `cargo test` ambient node 結合の fixture binary 化 — issue #188
  (F1)の管轄で、本 issue と独立。

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/183>
- 上流 discovery: `docs/discovery/benchmark-harness-audit-issue-183.md`
  (現行 harness 監査・Rust 側資産・決定空間)。
- 決定 ADR: `docs/adr/rust-native-benchmark-harness.md`。
- 依存判断(別 chain): `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`・
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` §5 F2。
- 先行 spec(別 chain): `docs/specs/workflow-orchestration-benchmark.md`
  (7 scenario 系・会計分離・parent usage 規則の原規定)。

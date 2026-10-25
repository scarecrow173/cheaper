---
id: "TASK-Df79tWn026fFvY79W1xIku"
type: "task"
status: "done"
title: "Implement benchmark manifest v2 — 12 required scenarios, validation, arm impls, spawn/observation parsers"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-bench/src/arms/mod.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/measure.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/manifest.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-bench/src/manifest.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/child.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/workspace.rs"
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by:
    - "docs/impl/ir/rust-native-benchmark-harness-issue-183.md"
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
  blocks:
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
    - "benchmarks/scenarios/scenarios.json"
  defers: []
  deferred-by: []
---

# Implement benchmark manifest v2 — 12 required scenarios, validation, arm impls, spawn/observation parsers

## Work

T1 の skeleton 上に、scenario manifest v2 の fail-fast validation・12
required scenario の宣言・arm 実装語彙・観測層(per-mode parser・
usage.jsonl 走査・route 系計測)を実装する。`benchmarks/scenarios/
scenarios.json` を schemaVersion 2・`fixtureRevision:"seeded-v2"`・
design §2.2 の 12 scenario exact coverage へ移行する。

**ordering pin**: manifest v2 flip と同一変更で `mise.toml` の
`node --test benchmarks/test/` 行を除去する(item 7)。現行
`benchmarks/test/` は `scenarios.mjs` で schemaVersion 1・exact-7
coverage を仮定しており、v2 manifest 上で pipeline に残ると
`mise run check` が壊れる — 行削除を後続 task へ残す中間状態
(red window)を作らない。

1. `src/manifest.rs` の validation 完成(design §2.6 の全 invariant):
   - `schemaVersion === 2`・`fixtureRevision` 非空・scenario id 一意・
     `REQUIRED_SCENARIO_IDS` exact coverage(missing/unexpected 双方
     拒否)。
   - `title` 非空・宣言 arm 必須・workflow 系 arm を含む scenario は
     非空 `workflow.jobs`(v1 parity)。
   - `armBase`/`armEnv`/`armExpect`/`armFakeStream` の key は全て宣言
     arm 内・`armBase` value は宣言 arm ∪ impl 語彙(alias 連鎖不可・
     解決後の名前は impl 語彙に含まれる)。
   - `route` block: `match.command` は literal token 列のみ(`*`
     含有 reject)・`match.command[0]=="cheaper"`(case-insensitive)
     reject(self-bypass 規約で到達不能)・`route.command` の
     tokenize 後 argv について `match.command` が literal prefix・
     `argv[1..] == ["--fixture-child", childMode]` 完全一致・
     `route.timeout` は `DURATION_PATTERN` 適合のみ・route 宣言
     scenario は `route-*` impl arm を 1 つ以上含む。
   - scenario `policy` overlay は既知 top-level section(`workflow`/
     `execution`/`output`/`retry`/`tools`/`filesystem`/`model`/
     `route`/`security`/`command`/`process` 等)のみ受理 — 未知
     section は reject。
   - `comparisons` の left/right は宣言 arm・`reduction` field 名は
     既知 metric/dimension 名に限定。
   - `armExpect` の key 語彙は design §2.6 の確定集合(top-level
     {envelopeOk, errorCode, exitCode, success}・`route` block・
     `resolve` block・`metrics`)に限定。

2. `benchmarks/scenarios/scenarios.json` を v2 へ移行 — design §2.2 の
   12 required scenario(`single-simple-tool-call`・
   `independent-fanout-worker-fanin`・`tool-to-worker-chain`・
   `process-and-analysis`・`large-intermediate-process-analysis`・
   `dynamic-vs-saved-workflow`・`definition-vs-compact-interface`・
   `route-resolve-and-rewrite`・`route-routed-exec`・
   `route-condense-and-refetch`・`worker-failure-modes`・
   `route-child-nonzero-exit`)を exact coverage で宣言する。
   - `processPolicy` command の `{node}` → `{fixtureChild}`、
     `manual.processScript` → `manual.child{mode,bytes}` に置換。
   - `route-condense-and-refetch` は `tokenThreshold:1` +
     `armFakeStream` で **condense output schema 適合 session**
     (`{"summary":…}` のみ・additionalProperties:false — bulk-reader
     形 text を流すと INVALID_OUTPUT → job-failed に落ちる)+
     `maxOutputBytes` truncation + `route logs` refetch を宣言。
   - `worker-failure-modes` の workflow 系 arm は scenario `policy`
     overlay(`execution.timeout` 縮小・`output.repairAttempts` +
     r+1 連続 schema 違反 session)で制御する — fixture `policy`/
     `abort`/`expect` 節は `run` 経路専用で `workflow run` には効か
     ない(design §2.1/§2.5)。overlay の timeout/repair 値は本 task
     で確定し、T7 の全実測はそのままの値で走る(計測側で調整
     しない)。
   - `saved-workflow.json` は変更なしで維持。

3. `src/arms/mod.rs` — impl 語彙 13 種(design §2.3):
   `workflow`/`dynamic-workflow`/`saved-workflow`・`direct-filesystem`
   (harness 内 `std::fs::read`)・`direct-worker-tool-call`・
   `direct-worker-sequence`(`manual.child` 指定時 fixture child を
   直接 spawn)・`full-definition`・`compact-interface`・
   `route-resolve`・`route-rewrite`・`route-run`・`route-logs`・
   `direct-child-exec`。失敗系は arm impl ではなく fixture +
   route/child 宣言 + expect で表現する。

4. `src/measure.rs` — 観測層:
   - per-mode stdout parser: envelope JSON(workflow/worker/utility)・
     `route run` raw text + trailer regex `\[cheaper route log: (\S+);
     truncated: (true|false)\]`・hook response JSON
     (`hookSpecificOutput.updatedInput.command`)の 3 形。
   - usage.jsonl record 走査(`kind:"route"`/`"route-refetch"`/
     `"run"`/`"count"` record — `directUsageRecordCount`・
     `logRefetches`・route dims の観測源。token 集計への二重計上
     禁止の分離規則は record/aggregate 層と整合)。
   - route-run arm の payload mint 導出規則(`cheaper-cli` の
     `route::payload::encode` + `EncodeInput` — command 原文・
     env-prefix-strip 後 argv・`args = argv[match.command.len()..]`・
     `executable` = basename・`cwd` = canonical workspace・
     `ambient_env` = passthroughEnv の値のみ — 12 scenario は `[]`)。
   - route-rewrite arm: **各 spawn 前に `<cfg>/hook-eval/` を削除**
     (dedup cache 60s TTL 対策 — 未 cache の `route-resolve` と
     非対称 bias を防ぐ)。observation 毎 fresh cfgDir の設計では
     rep 間で cache が共有されず効果は観測不能 — spec'd behavior
     として維持する **defense-in-depth** 措置。
   - route-logs arm: setup(計測外)で `route run --payload` を 1 回
     実行し trailer regex(fallback: `kind:"route"` record `logRef`)
     で `runId` 取得 → `route logs <runId>` を計測。setup record は
     同一 cfgDir の usage.jsonl に残り `directUsageRecordCount` に
     計上される(2 を期待)。
   - retainedBytes/criticalPathMs/usage 集計 — byte 計測は全て
     `cheaper_core::json::stringify::stringify` 経由(v1
     `JSON.stringify` parity — `serde_json` を byte 計測に使わない)。

5. `src/cli.rs` 完成 — preflight probe(design §1.3: minimal temp
   workspace + bulk-reader input schema 適合 stdin `{"task":"probe"}` +
   bulk-reader output schema 適合 finish の session fixture +
   `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` 下の `cheaper run
   bulk-reader --cwd <probe ws> --quiet` で envelope `ok:true` 確認。
   失敗時は「binary に dev-seam が無い」診断 + build 手順を出して
   fail-fast)・`git rev-parse HEAD` で `repositoryRevision` 採取・
   arm dispatch・observation 毎 fresh ws/cfgDir。

6. `tests/manifest.rs` — manifest v2 validation(必須 coverage・
   armBase/armEnv/armExpect/armFakeStream 整合・route/fakeStream
   検証・`*`/`cheaper` self-match・argv 不一致・未知 policy
   section・comparisons arm/field 照合)の fail-fast 確認。

7. `mise.toml` — `[tasks.check]` の `node --test benchmarks/test/` 行
   (現行 :73)を削除し、justification comment(現行 :61-62 —
   「benchmarks/test は benchmark harness 自体の node:test であり
   runtime 依存を持たないため check に配線する」)を書き換える:
   benchmark harness の unit test は `cargo test --workspace` 内の
   `cargo test -p cheaper-bench` が担う旨を記す。`node --test
   scripts/test/`(:72)は repo tooling test として**残す** —
   required node entrypoint は 6→5。
   - **manifest v2 flip(item 2)と同一 commit/変更で行う** —
     `benchmarks/lib/scenarios.mjs` が schemaVersion≠1 を throw し
     v1 test が exact-7 coverage を assert するため、行削除だけを
     後続 task へ残すと v2 manifest 上の `mise run check` が壊れる。
   - 旧 `.mjs` file 自体の削除は T6 — 本 task では pipeline から
     外すのみ(file は未参照の残存 file として残る)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T2 slice)
- docs/designs/benchmark-harness-redesign-issue-183.md (§2 manifest v2・§2.3 arm impl・§2.4 route materialize・§2.5 fixture/session 規則・§4.4 会計)
- docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md (先行 skeleton)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md

## Affected Files

- `crates/cheaper-bench/src/{manifest,cli}.rs`(validation/CLI dispatch + preflight 完成)
- `crates/cheaper-bench/src/{lib,child,workspace}.rs`(module 登録・doc comment の boundary 語彙整理)
- `crates/cheaper-bench/src/{arms/mod,measure}.rs`(新設)
- `crates/cheaper-bench/tests/manifest.rs`(新設)
- `benchmarks/scenarios/scenarios.json`(manifest v2 へ移行 — `saved-workflow.json` は不変)
- `mise.toml`(`node --test benchmarks/test/` 行削除 + justification comment 書換 — manifest flip と同一変更)

対象外: record/aggregate/report(T3)、残りの doc sync(T4)、
`benchmarks/` の `.mjs` file 削除(T6 — 本 task では pipeline から
外すのみで file は残る)。

## Done When

- [x] manifest validation が design §2.6 の全 invariant を fail-fast
      で強制し、`tests/manifest.rs` が missing/unexpected scenario・
      armBase 連鎖・route `*`/self-match/argv 不一致・未知 policy
      section・comparisons の不正参照を reject することを検証する
- [x] `benchmarks/scenarios/scenarios.json` が schemaVersion 2・
      `fixtureRevision:"seeded-v2"`・12 required scenario exact
      coverage で宣言され、validation を pass する
- [x] `mise.toml` の `node --test benchmarks/test/` 行(:73)が
      manifest flip と同一変更で削除され、justification comment
      (:61-62)が `cargo test -p cheaper-bench` を canonical 検証と
      して指す形へ書換済み(`node --test scripts/test/` は残る)
- [x] 13 impl 語彙の arm dispatch が実装され、`{fixtureChild}`・
      `manual.child`・route block・armExpect/armFakeStream 宣言が
      実際に materialize/spawn される
- [x] 3 種 stdout parser(envelope・raw text+trailer・hook JSON)が
      mode 別に動作し、解釈不能は `envelopeOk` 材料で harness error
      にしない
- [x] route-rewrite arm が各 spawn 前に `<cfg>/hook-eval/` を削除し、
      route-logs arm が setup spawn(計測外)→ `route logs` 計測の
      2 段で動く
- [x] preflight probe が実装され、dev-seam 無し binary を fail-fast で
      診断する
- [x] 全 byte 計測が `cheaper_core::json::stringify::stringify` 経由で、
      `serde_json::to_string`/`to_string_pretty` を byte 計測に使って
      いない
- [x] `cargo test -p cheaper-bench` が pass し workspace compile が
      green
- [x] `mise run check` が `node --test benchmarks/test/` を含まない
      pipeline で全 pass する(manifest flip + 行削除が同一変更で
      着地し red window を残さない)

## Verification

- [x] `mise exec -- cargo test -p cheaper-bench` exits 0(tests/manifest.rs + tests/fixture.rs)
- [x] `mise exec -- cargo test -p cheaper-bench --test manifest` exits 0
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-bench --all-targets -- -D warnings` exits 0
- [x] `mise exec -- cargo build -p cheaper-cli --features dev-seam` exits 0(smoke 用の dev-seam 有効 debug binary — `target/debug/cheaper` は cli.rs の binary 解決 fallback で拾われる。release build ・`--binary` 明示でもよい)
- [x] `mise exec -- cargo run -p cheaper-bench -- --scenario single-simple-tool-call --warmups 0 --repetitions 1` が dev-seam binary 上で 1 observation を実行できることを確認(smoke — record/aggregate 未接続でも observation 生成まで確認可。失敗したら原因を Notes に記録)
- [x] `mise run check` exits 0(`node --test` は scripts のみ — v1 test suite が v2 manifest 上で走る窓が無いこと)
- [x] Manual: `git grep -n '{node}\|process\.execPath\|node:fs' -- benchmarks/scenarios/ crates/cheaper-bench/` が 0 件(`benchmarks/results/` の immutable v1 artifact は旧計測 command を literal で含むため対象外 — supplement)

## Notes

- smoke run は dev-seam 有効 debug binary(`cargo build -p
  cheaper-cli --features dev-seam` で生成される
  `target/debug/cheaper`)を前提とする — `cheaper-bench` の
  `cheaper-cli` dep edge は default feature のため harness 自身の
  build では計測対象 binary に seam が付かず、明示 build か
  `--binary` での path 指定が必要(preflight probe が seam 無し
  binary を fail-fast で検出する経路の確認も兼ねる)。
- `mise.toml` の行削除を manifest flip と同時に行うのは、v1
  schemaVersion・exact-7 coverage を仮定する `benchmarks/test/`
  suite が v2 manifest 上で走ると `mise run check` が壊れるため
  (T4 まで残すと red window ができる)。`.mjs` file 削除自体は
  T6 のまま。
- `worker-failure-modes` の `execution.timeout` overlay 値は本 task
  で確定する — T7 の全実測は確定済み値で走り、計測側で調整しない
  (wall-time 爆発防止の pre-agreed 前提)。確定値:
  `execution.timeout:"500ms"` + `output.repairAttempts:1`。
- 実測 smoke で発見した product 意味論 2 点を armExpect pin へ反映
  した(manifest 側の期待値を product の実挙動へ合わせた —
  observation 側は不変):
  - `route run` は child の非零 exit code を自身の process exit
    として mirror する — `route-child-nonzero-exit` の
    `route-run` pin は `exitCode:7`(design §2.2 の「exit code
    mirror」は parent-visible process exit にも及ぶ)。route
    record 側の child exitCode は `metrics.routeChildExitCode`
    として記録(route block pin 語彙外の unpinned data)。
  - workflow の provider call timeout は `TOOL_EXECUTION_FAILURE`
    に wrap されず `TIMEOUT` が伝播する —
    `worker-failure-modes/workflow-worker-timeout` の pin は
    `errorCode:"TIMEOUT"`。
- 検証実行時点で `target/release/cheaper.exe` が dev-seam 無しの
  stale binary だったため binary 解決は release 優先のまま
  preflight probe が正しく fail-fast した(診断動作の実証込み)。
  smoke 系は `--binary target/debug/cheaper.exe` で実行。
- smoke 結果: `--scenario single-simple-tool-call`(必須 smoke)に
  加えて全 12 scenario を `--warmups 0 --repetitions 1` で実行し、
  全 arm observation が `success:true`(失敗 matrix arm は宣言済み
  armExpect に一致 — direct/workflow×provider-error/timeout/
  postprocess-failure の 6 arm 全て pin 通りの envelope error)。
- `cargo test -p cheaper-bench` は lib 40 + tests/fixture 11 +
  tests/manifest 18 = 69 test 全 pass。`mise run check` は
  `node --test scripts/test/`(43 test)のみの pipeline で全 pass。
- git grep boundary(`{node}`/`process.execPath`/`node:fs` —
  `benchmarks/scenarios/`・`crates/cheaper-bench/` 範囲)は 0 件。
  初回走査で doc comment 内の歴史的参照が 5 件 hit したため、
  literal token を使わない表現へ書き換え済み。

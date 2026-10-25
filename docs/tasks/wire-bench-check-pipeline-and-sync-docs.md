---
id: "TASK-oxOS7kvPXiqJ0LEOT1edeB"
type: "task"
status: "todo"
title: "Wire cheaper-bench into check pipeline and sync docs — mise.toml, AGENTS/CONTRIBUTING, benchmarks docs, biome glob, #182 addendum, workflow-usage placeholder"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "CONTRIBUTING.md"
      - type: "file"
        path: "benchmarks/README.md"
      - type: "file"
        path: "benchmarks/results/README.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
      - type: "file"
        path: "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
      - type: "file"
        path: "biome.jsonc"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
  blocks:
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
    - "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
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
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  defers: []
  deferred-by: []
---

# Wire cheaper-bench into check pipeline and sync docs

## Work

T1-T3 で着地した harness に合わせて design §6 doc-sync list を実施する
**doc-sync 専任 task**。`node --test benchmarks/test/` の pipeline 行
削除・justification comment 書換は **T2 の manifest v2 flip と同一
変更で着地済み** — v1 schemaVersion・exact-7 coverage を仮定する
`benchmarks/test/` suite が v2 manifest 上で走る窓(red window)を
残さないためである。旧 `.mjs` file 自体の削除は T6 で行うため、本
task 完了時点では旧 harness file は pipeline から外れた未参照の残存
file となる(ordering: 行削除(T2)→ file 削除(T6)は必ずこの順)。

参考: `cargo clippy --workspace --all-targets`・`cargo fmt --all`・
`cargo test --workspace` は cheaper-bench を自動で含む(workspace
member)。check pipeline の `--features cheaper-runtime/dev-seam,
cheaper-cli/dev-seam` とは独立に、cheaper-bench 側の dev-seam は自
crate の dep edge が担う。

1. `AGENTS.md` — check pipeline 記述(:36-37 の「`node --test`
   (scripts・benchmarks)」→ scripts のみ)・node tooling 列挙(:83 の
   `benchmarks/` を除去 — `benchmarks/` に残るのは言語中立資産のみ、
   harness は Rust crate へ)。

2. `CONTRIBUTING.md` — pipeline 記述(:27)・test コマンド一覧(:51-52
   の `node --test benchmarks/test/` 除去)・node tooling 記述(:56)・
   構造一覧(:85-99 の crate list に `crates/cheaper-bench` 行
   (publish=false の dev-only benchmark harness — `cargo run -p
   cheaper-bench` で手動実行)を追加し、:108 の `benchmarks/` 行を
   言語中立資産 + `crates/cheaper-bench` 参照へ)。併せて `テスト` 節
   に 1 行注記: flag 無し `cargo test --workspace` は cheaper-bench
   の `cheaper-runtime/dev-seam` dep edge 経由で runtime の dev-seam
   依存 target を新たに compile する(`cheaper-cli` 側は flag 必須の
   まま — 意図的な非対称で、T1 の Cargo.toml comment と同趣旨を
   文書側にも残す)。

3. `benchmarks/README.md` — 全面書換: Rust harness 手順
   (`cargo build --release -p cheaper-cli --features dev-seam` →
   `cargo run --release -p cheaper-bench -- --scenario <id>
   --warmups <n> --repetitions <n> [--output] [--binary] [--pricing]`)、
   fixture child(`--fixture-child`・`{fixtureChild}`)・seam 説明
   (`CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` + dev-seam —
   `NODE_ENV` は node 命名の vestige として明記)・schema v2・
   Route/失敗 scenario・claim 境界(measured/estimated/unavailable・
   parentUsage unavailable 時は親 token/費用削減を主張しない)・
   immutable 非上書き保存・**baseline 同等性 migration note**(`process-*`
   scenario の v1 wall-time は node 起動 cost を含んでいた — fixture
   child 化で絶対値はずれる・`direct-filesystem` も `node:fs` →
   `std::fs::read` へ実装変更・v1 vs v2 の値は直接比較せず
   `fixtureRevision:"seeded-v2"` で区別)を含める。

4. `benchmarks/results/README.md` — schemaVersion 2・v1 file 不変・
   混在不可(v2 load が v1 を reject)・append-only 方針を追記。

5. `biome.jsonc` — `"benchmarks/**/*.mjs"` dead glob 行を除去
   (`"benchmarks/**/*.json"`・`!benchmarks/results` は残存
   scenarios/results 対象で維持)。

6. `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` —
   日付付き addendum: §1.1 row 4(`node --test benchmarks/test/`)を
   migrated 扱い・§5 F2(benchmark harness Rust 化)が issue #183 に
   absorb され本 change で実施済みである旨を記す。

7. `docs/guides/workflow-usage.md` — **placeholder 段階**: 旧 TS 測定
   (rev `b3213f8c`・TS CLI 時代)を「TS 時代測定(legacy)」として区別
   表記し(削除しない)、Rust-era 実測値の掲載枠に「必須 12 scenario
   全 arm の新 run artifact から採用・計測 pending」という方針記述を
   置く。**本 task では実測値を書かない**(T7 が置き換える)。

8. `.gitignore` — :5 の comment「Node tooling (scripts/, benchmarks/,
   doc-suite skills)」が stale になるため `benchmarks/` を除去する
   (Node tooling は `scripts/`・doc-suite skills が残り、benchmark
   harness は Rust crate へ移行)。

9. `.pre-commit-config.yaml` — **変更不要**(確認のみ): biome mirror
   files pattern(:35)は残存 `benchmarks/**/*.json`(scenarios)に
   合致し続ける。確認結果を Notes に記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T4 slice)
- docs/designs/benchmark-harness-redesign-issue-183.md (§5 check 統合・§6 doc-sync list)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/implement-bench-record-schema-v2-and-aggregation.md

## Affected Files

- `.gitignore`・`AGENTS.md`・`CONTRIBUTING.md`・`benchmarks/README.md`・
  `benchmarks/results/README.md`・`docs/guides/workflow-usage.md`・
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`・
  `biome.jsonc`
- 確認のみ: `.pre-commit-config.yaml`・`mise.toml`(pipeline 整合は
  T2 で着地済み — 本 task は変更しない)

対象外: `mise.toml` 本体の変更(T2 で確定済み)、`benchmarks/` の
`.mjs` file(T6 で削除)、`benchmarks/scenarios/`(T2 で確定済み)、
`benchmarks/results/*.json`(v1 file 不変)。

## Done When

- [ ] `mise.toml` の pipeline 整合(`node --test benchmarks/test/` 行
      削除・justification comment が cheaper-bench 参照)が T2 で
      着地済みであることを確認(本 task は変更しない —
      `node --test scripts/test/` は残る)
- [ ] `AGENTS.md`・`CONTRIBUTING.md` が `node --test`(scripts のみ)・
      `benchmarks/` = 言語中立資産 + `crates/cheaper-bench` 参照へ整合
      (CONTRIBUTING の crate list への cheaper-bench 行追加・flag
      無し `cargo test --workspace` の dev-seam 非対称注記を含む)
- [ ] `.gitignore` の Node tooling comment から `benchmarks/` が
      除かれている
- [ ] `benchmarks/README.md` が 2 段手順・fixture child/seam・schema
      v2・`{fixtureChild}`・Route/失敗 scenario・claim 境界・
      baseline 同等性 migration note・`NODE_ENV=test` vestige を含む
- [ ] `benchmarks/results/README.md` が schemaVersion 2・v1 file
      不変・混在不可の方針を述べる
- [ ] `biome.jsonc` から `"benchmarks/**/*.mjs"` dead glob が除かれ、
      `"benchmarks/**/*.json"`・`!benchmarks/results` が残る
- [ ] #182 design に日付付き addendum(§1.1 row 4 migrated・§5 F2
      absorb 済み)がある
- [ ] `docs/guides/workflow-usage.md` が旧 TS 値を legacy 区別し、
      Rust-era 実測値の掲載枠を placeholder で持つ(実測値なし)
- [ ] `.pre-commit-config.yaml` の確認結果が Notes に記録済み
- [ ] `mise run check` が `node --test benchmarks/test/` を含まない
      pipeline で全 pass する

## Verification

- [ ] `mise run check` exits 0(node --test は scripts のみの pipeline)
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type plan --json` で本 plan file の findings 0
- [ ] Manual: `grep -n "benchmarks/test" mise.toml` が 0 件(T2 の
      pipeline 整合が維持されている — supplement)
- [ ] Manual: `grep -n "benchmarks" .gitignore` が 0 件(supplement)
- [ ] Manual: `grep -n "benchmarks" AGENTS.md CONTRIBUTING.md` の記述が
      「harness は `crates/cheaper-bench`・`benchmarks/` は言語中立
      資産」と整合(supplement)
- [ ] Manual: `biome check` が dead glob 除去後も pass
      (`"benchmarks/**/*.json"` 対象の scenarios が lint 対象に残る
      こと — supplement)

## Notes

- `.pre-commit-config.yaml` の確認結果(変更不要の結論と根拠)を
  本節へ記録する。
- `mise.toml` の pipeline 行削除は T2 の manifest v2 flip と同一
  変更で着地済み — 本 task が残存 doc-sync を担う。確認として
  `grep -n "benchmarks/test" mise.toml` が 0 件であることを
  Verification で再確認する。

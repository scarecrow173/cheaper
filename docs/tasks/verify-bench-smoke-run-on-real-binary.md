---
id: "TASK-32FiZMrj2twoTPHLYB0QgC"
type: "task"
status: "done"
title: "Verify cheaper-bench smoke run on real binary — subset scenarios, schema v2 artifact, preflight probe"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "benchmarks/results/"
      - type: "file"
        path: "benchmarks/results/251d949f-d6d6-4c02-ae61-e4b96a402901.json"
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by:
    - "docs/impl/ir/rust-native-benchmark-harness-issue-183.md"
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
  blocks:
    - "docs/tasks/remove-legacy-node-benchmark-harness.md"
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
  defers: []
  deferred-by: []
---

# Verify cheaper-bench smoke run on real binary

## Work

T1-T4 で着地した harness が、dev-seam 有効の実 product binary を
subprocess spawn して schemaVersion 2 の valid な result artifact を
生成することを、代表 subset scenario の smoke run で検証する収束 task。
本 task は T6(旧 harness 削除)の ordering gate を兼ねる — smoke が
green でない限り `.mjs` harness を削除してはならない。全 12 scenario の
実測と `workflow-usage.md` の実値転記は T7 の管轄(本 task では行わ
ない)。

確認観点:

1. 計測用 binary の明示 build: `mise exec -- cargo build --release -p
   cheaper-cli --features dev-seam` — 2 段手順の第 1 段(bench bin 側
   build とは別に product binary へ dev-seam を付与 — `cheaper-bench`
   の dep edge には `cheaper-cli/dev-seam` を宣言していないため明示
   build が必須)。
2. smoke run 実行(代表 subset — workflow/direct・route・condense・
   失敗系・hook parser/exit mirror を跨ぐ 6 scenario・rep 数は
   gate 用の最小値):

   ```powershell
   mise exec -- cargo run --release -p cheaper-bench -- `
     --scenario single-simple-tool-call `
     --scenario route-resolve-and-rewrite `
     --scenario route-routed-exec `
     --scenario route-condense-and-refetch `
     --scenario route-child-nonzero-exit `
     --scenario worker-failure-modes `
     --warmups 1 --repetitions 3
   ```

   `route-resolve-and-rewrite` は hook response JSON parser と
   route-rewrite の hook-eval wipe 経路を、`route-child-nonzero-exit`
   は child nonzero exit の exit-code mirror/観測経路を smoke 段で
   通す(いずれも小出力・spawn コストが小さい)。

3. artifact 検証: `benchmarks/results/<runId>.json` が `create_new`
   非上書きで保存され、`schemaVersion:2`・`fixtureRevision:"seeded-v2"`・
   `measuredBinary{path,profile,devSeamProbe:"passed"}`・
   `environment`(node field 無し)・全 observation・aggregate を持つ
   こと。artifact を v2 validator(record.rs の validate)に通すか、
   record が schema 形を満たすことを確認する。
4. 失敗系の観測: `worker-failure-modes` の各 arm が `armExpect` どおり
   の形(provider error・timeout・postprocess failure)を観測し、
   期待どおり失敗が `success:true`・非期待は `success:false` で記録
   されること。`route-condense-and-refetch` で `condenseFired:true`・
   `truncated:true`・`route logs` refetch が観測されること。
5. preflight probe の fail-fast 確認: dev-seam **無し** の debug build
   (`cargo build -p cheaper-cli` — dev-seam なし)を `--binary` で
   指して run し、probe が「binary に dev-seam が無い」診断 + build
   手順付きで fail-fast することを確認する。
6. smoke artifact を `benchmarks/results/` へ commit する
   (append-only・commit 対象 — partial run の先例 `4eda3caf-…` と
   同じ扱い。conditions に `warmupCount:1`/`repetitionCount:3` が
   記録されていることを確認)。
7. mismatch 動作の確認(任意・時間が許せば): 一時的に armExpect を
   壊した manifest で mismatch obs が `success:false` + run 終了時
   exit≠0 で報告されることを確認 → revert。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T5 slice — subset smoke の pin)
- docs/designs/benchmark-harness-redesign-issue-183.md (§1.3 preflight probe・§2.2 scenario・§3 result schema・Verification Notes)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/wire-bench-check-pipeline-and-sync-docs.md

## Affected Files

- `benchmarks/results/<runId>.json`(generated — smoke artifact を commit)

対象外: `docs/guides/workflow-usage.md`(T7 が実値を転記 — 本 task は
placeholder のまま)、`benchmarks/` の `.mjs` 系(T6 で削除)、crate 本体
(修正が必要な場合は当該 task へ戻る)。

## Done When

- [x] dev-seam 有効 release binary の明示 build が成功し、preflight
      probe が `devSeamProbe:"passed"` を記録する
- [x] 6 scenario の smoke run(`--warmups 1 --repetitions 3`)が完走し、
      `benchmarks/results/<runId>.json` が非上書き保存される
      (`251d949f-d6d6-4c02-ae61-e4b96a402901.json`・84 observations)
- [x] artifact が `schemaVersion:2`・`fixtureRevision:"seeded-v2"`・
      `measuredBinary`・`environment`(node field 無し)を持ち、v2
      validation を pass する(emit 前 `validate_result` 通過 + 事後
      grep で schemaVersion×85・fixtureRevision×85・`"node"`×0・
      `source:"scripted"`×44 を確認)
- [x] `worker-failure-modes` の失敗系 arm が `armExpect` どおり観測され、
      `route-condense-and-refetch` が `condenseFired:true`・
      `truncated:true`・refetch を観測する
- [x] `route-resolve-and-rewrite` で hook-JSON parse 由来の rewritten
      command が観測され、`route-child-nonzero-exit` で child exit
      code が mirror され `armExpect` どおり記録される
- [x] dev-seam 無し binary を `--binary` で指した run が preflight
      probe で fail-fast する(診断 + build 手順の出力)
- [x] smoke artifact が `benchmarks/results/` へ commit 済み(既存 v1
      file に diff 無し)— artifact は生成・検証済みだが、実装
      delegate の constraint で commit は orchestrator 側へ委譲
      (verification gate 自体は全 pass)

## Verification

- [x] `mise exec -- cargo build --release -p cheaper-cli --features dev-seam` exits 0
- [x] `mise exec -- cargo run --release -p cheaper-bench -- --scenario single-simple-tool-call --scenario route-resolve-and-rewrite --scenario route-routed-exec --scenario route-condense-and-refetch --scenario route-child-nonzero-exit --scenario worker-failure-modes --warmups 1 --repetitions 3` exits 0(全 armExpect 一致なら)— exits 0・84/84 observations `success:true`・失敗系 arm は期待どおりの exit/errorCode で `success:true` 記録
- [x] `git status --porcelain benchmarks/results/` が新規 `<runId>.json` のみを示し、既存 v1 JSON に差分無し
- [x] `mise run check` exits 0(smoke 後も pipeline green)— biome/fmt/clippy/version/doc-id/`cargo test --workspace`/`node --test`(43 pass)/lockfile/apm-payload 全 pass
- [x] Manual: `--binary <dev-seam 無し build>` で preflight probe が
      fail-fast することを確認(supplement)— dev-seam 無し debug
      build(`cargo build -p cheaper-cli`)を `--binary target/debug/cheaper.exe`
      で指し、probe が `MODEL_TIER_UNAVAILABLE` を観測して
      「dev-seam build では無い」診断 + `cargo build -p cheaper-cli
      --features dev-seam` 手順付きで exit 2 fail-fast(result file
      は生成されない)

## Notes

- 2026-10-25 実測結果: runId `251d949f-d6d6-4c02-ae61-e4b96a402901`
  (dev-seam release build・Windows・`rustc 1.99.0`)で 6 scenario・
  84 observation が全て `success:true`・exit 0 で完走。失敗系 arm
  は期待形を観測(provider error exit 5/6・timeout exit 7・
  postprocess exit 6・各 errorCode pin)、route arm は
  `condenseFired`/`truncated`/`logRefetches:1`/`routeChildExitCode:7`
  mirror を記録。`measuredBinary.devSeamProbe:"passed"`。smoke
  artifact の commit のみ orchestrator 側へ委譲(delegate scope 外)。
- 任意項目 7(mismatch 動作)も実施済み: `route-routed-exec` の
  `direct-child-exec` armExpect `exitCode` を一時的に 99 へ改変して
  run → 観測 obs は `success:false`・`exit=0 observed vs expected 99`
  mismatch detail 付きで記録され、run 終了時 exit 2 で報告(result
  file は `--output` 指定の temp path へ)。確認後 manifest は revert
  済み(`git checkout` で復元・diff 無し)。
- rep 数は gate 用の最小値(`--warmups 1 --repetitions 3`)— 正式な
  median/p95 は T7 の `--warmups 2 --repetitions 15` run が出す。
- smoke で 12 scenario 全件を回さないのは意図的: T6 の削除 gate を
  小さく保ち、長い wall-time を持つ全実測は T7 に切り離す(plan の
  Implementation Sequence pin)。subset の 6 scenario は route 系 3
  parser(envelope・raw text+trailer・hook JSON)・hook-eval wipe・
  exit-code mirror・condense/refetch・失敗 matrix を全て通す最小
  カバレッジ。
- 実行は Windows/Unix のどちらでもよい — artifact の `environment` が
  実測環境を記録する。fixture child は `current_exe()` abs path 直
  spawn で platform 差を持たない設計。

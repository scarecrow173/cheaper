---
id: "TASK-b6CX0aXZBAJ6ElAqC0uXxe"
type: "task"
status: "todo"
title: "Run full 12-scenario benchmark measurement and update workflow-usage.md with Rust-era values"
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
        path: "docs/guides/workflow-usage.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "benchmarks/results/"
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
  blocks: []
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
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
---

# Run full 12-scenario benchmark measurement and update workflow-usage.md

## Work

T5 の smoke で動作証明済みの harness を使い、必須 12 scenario 全 arm を
`--warmups 2 --repetitions 15` で実測し、artifact を
`benchmarks/results/` へ保存・commit する。その artifact から
`docs/guides/workflow-usage.md` の placeholder(T4 で配置)を Rust-era
実測値へ置き換える — spec AC「測定結果を workflow-usage.md へ反映し、
旧 TS 測定と新 Rust 測定を区別する」の着地。**T7 は本 PR で着地
必須** — spec AC が workflow-usage.md の Rust-era 実測更新を要求する
ため、follow-up へ回さない(T6 と並列可だが PR scope から外さない)。

1. 実測 run(dev-seam 有効 product binary・全 scenario 省略なし):

   ```powershell
   mise exec -- cargo build --release -p cheaper-cli --features dev-seam
   mise exec -- cargo run --release -p cheaper-bench -- `
     --warmups 2 --repetitions 15
   ```

   - `--scenario` 省略で manifest 全 12 scenario × 全 declared arm
     (~35 arm × warmups 2 + reps 15 ≈ **600-900 spawn**)が対象。
     期待 wall-time は **~10-40 分 + 初回 release build**(環境負荷
     依存 — worker-failure-modes の hang/timeout fixture を含む)。
   - 長時間 run のため `--scenario` による **batch 分割 run も可** —
     各 run が固有 runId の artifact を生成するため、全 scenario を
     カバーする複数 run を順次実行し、workflow-usage.md は複数
     artifact(runId)を cite してよい。
   - `worker-failure-modes` の `execution.timeout` overlay 値は
     **T2 の manifest で確定済み** — 本 task で計測条件を調整しない
     (hang 計測の wall-time を制する pre-agreed 前提)。
   - run 終了時に mismatch obs を抱える scenario があれば harness が
     exit≠0 で失敗報告する — その場合は fixture/armExpect の不整合
     を修正して再実行する(artifact として記録しない)。
   - artifact(`benchmarks/results/<runId>.json`)を commit する。
     `repositoryRevision` が実行時 HEAD を指すことを確認
     (batch 分割時は各 artifact 分)。

2. `docs/guides/workflow-usage.md` の更新(T4 の placeholder を実値へ
   置換):
   - 旧 TS 時代の掲載値(rev `b3213f8c`・`node packages/cli/dist/
     main.js`・workflow median ≈2,795 ms)は「TS 時代測定(legacy)」
     として区別表記のまま残す(**削除しない**)。
   - 新しい Rust-era 測定を新 run artifact 由来の値のみで記す —
     掲載する値(median/p95 wall-time・retainedBytes 系・route
     削減率・condense overhead 等)の粒度は実測 artifact から選定し、
     `runId`・`repositoryRevision`・`fixtureRevision:"seeded-v2"`・
     計測環境(os/arch/profile)を明記する。batch 分割 run を使った
     場合は cite する runId を複数明記する。
   - **arm 名・prose も v2 語彙へ更新する** — 数値だけでなく表中の
     label も Rust-era へ整合させる。例: 「Node filesystem read」は
     `direct-filesystem`(harness 内 `std::fs::read`)、旧 `process-*`
     scenario 名・`{node}` 言及は manifest v2 の arm/scenario 名へ
     置き換える。
   - **claim 境界の遵守**: `parentUsage.status=unavailable` 前提で、
     親 token 削減・費用削減の数値主張を載せない。estimated 系は
     `--pricing` run を行わない限り掲載しない(掲載する場合は
     assumptions の pricing 前提・`boundary:"scripted-usage"` を
     併記する — 実料金比較として表示しない)。
   - baseline 同等性の明記: `process-*` scenario の値は fixture child
     由来で v1(node workload)と絶対値を直接比較しない —
     `fixtureRevision` の違いと README migration note を参照する。

3. 結果の整合確認: artifact の aggregate が claim 規則(estimated は
   無指定なら emit 無し・comparisons の unavailable entry は
   deltas null + reason)を守り、`workflow-usage.md` の記述が
   artifact の値と一致すること。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T7 slice — smoke/実測分離の pin)
- docs/designs/benchmark-harness-redesign-issue-183.md (§6 workflow-usage.md 行・§3.5 artifact・§4 aggregate)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/verify-bench-smoke-run-on-real-binary.md (T5 smoke が
  harness の動作を証明済みであること。T4 経由で placeholder が着地済み)

## Affected Files

- `benchmarks/results/<runId>.json`(generated — 全 12 scenario 実測
  artifact を commit)
- `docs/guides/workflow-usage.md`(placeholder → Rust-era 実測値)

対象外: harness 本体・`benchmarks/` の `.mjs` 系(T6 と並列可 —
file 面が交差しない)。

## Done When

- [ ] 全 12 scenario × 全 declared arm の実測 run(`--warmups 2
      --repetitions 15`)が exit 0 で完走し、全 armExpect 一致で
      mismatch 報告が無い
- [ ] `benchmarks/results/<runId>.json` が非上書き保存・commit され、
      `schemaVersion:2`・`repositoryRevision` が実測時 HEAD を指す
- [ ] `docs/guides/workflow-usage.md` が新 artifact 由来の Rust-era
      測定を引用し、runId/revision/fixtureRevision/環境を明記、
      TS 時代値を legacy として区別し(削除していない)
- [ ] 掲載表・prose の arm/scenario 名が v2 語彙(`direct-filesystem`
      ・`std::fs::read` 等)へ更新され、「Node filesystem read」等の
      v1 表現が残っていない
- [ ] 文書が `parentUsage.status=unavailable` 前提で親 token/費用
      削減の数値主張を含まない
- [ ] baseline 同等性(v1 との直接比較不可・fixture child 由来の
      置換)が明記されている
- [ ] `benchmarks/results/` の v1 JSON に diff が無い
- [ ] chain 収束処理: 全 task の status → done 反転・
      `docs/tasks/README.md` の status cell 更新・
      `audit_docs.js --type task` で本 chain file の findings 0
      (orchestrator 巻取りの慣行を本 task で pin)

## Verification

- [ ] `mise exec -- cargo run --release -p cheaper-bench -- --warmups 2 --repetitions 15` exits 0
- [ ] `git status --porcelain benchmarks/results/` が新規 `<runId>.json` のみを示し、既存 v1 JSON に差分無し
- [ ] Manual: `docs/guides/workflow-usage.md` の掲載値が artifact の
      `aggregate` と一致し、legacy 区別・claim 境界(parentUsage
      unavailable 時の費用/親 token 削減の非表示)を守る(supplement)
- [ ] `mise run check` exits 0(実測後も pipeline green)

## Notes

- 期待 wall-time: ~35 arm × 17 obs(warmups 2 + reps 15)≈ 600-900
  spawn で **~10-40 分 + 初回 release build**。長時間 run のため
  `--scenario` batch 分割 run も可 — 各 run が固有 runId artifact
  を持ち、workflow-usage.md は複数 runId を cite してよい。
- spec AC が workflow-usage.md の Rust-era 実測更新を要求するため
  **T7 は本 PR で着地する** — follow-up へ回さない(T6 と並列可
  だが PR scope から外さない)。
- 実測の wall-time は実行環境の負荷に依存する — token/byte 面は
  deterministic seeded fixture で反復間一致が期待されるが、wall-time
  の絶対値は記録値としてそのまま扱う(baseline 同等性の別記対象)。
- `--pricing` を使った estimated run は本 task では行わない(opt-in・
  必要になった時点で別 run)。
- `cheaper usage` 側の route record(`kind:"route"`/`route-refetch`)
  詳細分析は issue #184 の管轄 — workflow-usage.md には本 harness の
  route scenario 観測値のみを載せる。

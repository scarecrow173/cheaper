# benchmarks/results/

`cheaper-bench`(`cargo run -p cheaper-bench`)の計測結果 JSON を保持する
append-only の記録置き場。

## 保持方針

- ファイル名は `<run-id>.json`(UUID 付与)で、**上書きしない・削除しない**。
  新しい計測は常に新規ファイルとして追加する。
- commit 対象。計測時点の source revision・fixture revision・
  環境(OS/arch/ambient rustc/CPU)と arm ごとの観測値・median/p95・
  比較差分が記録されるため、過去との比較は commit 済み JSON 同士で
  行う。
- 既存ファイルを上書きしないのは harness 側の保存契約と一致
  (`--output` 明示 path も同じ非上書き — 詳細は
  `benchmarks/README.md`)。

## schemaVersion と v1 file の扱い

- 現行 harness が書き出す結果は **`schemaVersion: 2`**
  (`fixtureRevision: "seeded-v2"`)。
- 既存の v1(`schemaVersion:1`)file は **一切改変しない** —
  Rust harness 移行前の記録としてそのまま残る。
- **混在不可**: v2 の load/aggregate は `schemaVersion` を確認して
  v1 file を reject する。v1 と v2 の値は直接比較せず、
  `fixtureRevision` で区別する(migration note は
  `benchmarks/README.md`「baseline 同等性」節)。
- 参照用の代表結果は `benchmarks/README.md` からリンクする
  (例: TS CLI 時代の `e45d35b7-...json`)。

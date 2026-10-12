# benchmarks/results/

`benchmarks/run.mjs` の計測結果 JSON を保持する append-only の記録置き場。

## 保持方針

- ファイル名は `<run-id>.json`(UUID 付与)で、**上書きしない・削除しない**。
  新しい計測は常に新規ファイルとして追加する。
- commit 対象。計測時点の source revision・fixture revision・seed・
  環境(OS/arch/Node/CPU)と arm ごとの観測値・median/p95・比較差分が
  記録されるため、過去との比較は commit 済み JSON 同士で行う。
- 既存ファイルを上書きしないのは `--output` option の仕様と一致
  (詳細は `benchmarks/README.md`)。
- 参照用の代表結果は `benchmarks/README.md` からリンクする
  (例: TS CLI 時代の `e45d35b7-...json`)。

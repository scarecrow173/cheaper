---
title: "Workflow の使い分け"
---

# Workflow の使い分け

## 測定範囲

この指針は `benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json` の seeded run に基づく（測定 source revision `b3213f8c089c07d521e834df282c0c778b729054`）。各 arm は warm-up 2 回の後に 15 回測定し、fake Worker stream を使った。OS は Windows 10.0.26200、x64、Node v26.10.0、CPU 16 個。壁時計時間はこの環境での CLI 起動を含む測定で、親 agent の実 token usage は接続されていない。

`direct-worker-sequence` は再現可能な script が direct CLI を順番に起動する比較 arm であり、実際の親 agent の turn 数や token usage ではない。親 token の削減量はこの結果から算出しない。

## 結果

表の時間は median / p95、差分は workflow − direct arm。prompt bytes は Worker に実際に materialize された context、retained bytes は harness が保持した結果・中間値を表す。

| Scenario | Workflow median / p95 | Direct median / p95 | Input tokens (workflow / direct) | Prompt bytes (workflow / direct) | Retained bytes (workflow / direct) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 単純な read tool 1 回 vs Node filesystem read | 2,795 / 2,849 ms | 0.1 / 0.2 ms | 0 / 未計測 | 0 / 0 | 45 / 33 |
| 独立 read 2 件 + Worker fan-in | 3,289 / 3,469 ms | 2,909 / 3,259 ms | 2,946 / 2,946 | 12,413 / 12,413 | 36 / 155 |
| read → Worker | 3,072 / 3,151 ms | 2,861 / 3,052 ms | 2,944 / 2,944 | 12,399 / 12,399 | 87 / 144 |
| process → Worker | 5,525 / 5,960 ms | 2,930 / 3,225 ms | 2,935 / 2,935 | 12,360 / 12,360 | 79 / 131 |
| 32 KiB process output → Worker | 5,795 / 5,988 ms | 2,954 / 3,213 ms | 11,123 / 11,123 | 45,113 / 45,113 | 32,823 / 32,876 |

この fixture 群では workflow の Worker input tokens と materialized prompt bytes は対応する direct arm と同じだった。workflow は返却結果・中間値を小さく保ったが、local wall time は read→Worker で約 211 ms、fan-in で約 380 ms、process 系で約 2.6–2.8 秒長かった。単純な read では workflow CLI が 2.8 秒、同じ seeded filesystem read は 0.1 ms だった。小さな deterministic tool work だけなら direct tool invocation が適している。

| Discovery / reuse 比較 | 結果 |
| --- | --- |
| Dynamic workflow definition | 398 bytes、Worker input 2,928 tokens |
| Saved workflow definition | 348 bytes、Worker input 2,928 tokens |
| Dynamic / saved の parent-visible interface | 両方 82 bytes |
| Private definition の read | 351 bytes、median 0.4 ms |
| Compact interface payload | 79 bytes、median 2,744 ms |

この fixture では compact interface payload は private definition より 272 bytes（約 77.5%）小さかった。interface CLI の起動には時間がかかるため、byte 削減を token 節約や wall-time 短縮と読み替えない。workflow を選ぶ根拠は、親が DAG の詳細を読む必要を減らすこと、構造化された fan-in を保持すること、または中間結果を親 context に展開しないことにある。ここで示した byte 差はこの小さな fixture に限る。

## 選択基準

- **1 回の単純な read / grep / process:** direct invocation を選ぶ。測定した tool-only workflow は Node filesystem read と比べて CLI 起動分の wall time が大きかった。
- **少数の tool → Worker 処理:** workflow の Worker token は direct sequence と同じで、この環境では workflow の wall time が長かった。親側の turn/context が別途観測されない限り、workflow をコスト削減として主張できない。
- **独立 tool jobs と fan-in:** fan-out と依存関係を一つの workflow に表現し、必要な summary だけを返す場合に使う。この測定では Worker prompt は短くならず、retained result が 119 bytes 小さかった。read tool が短時間で終わったため同時実行の high-water は 1 であり、この run は並列速度向上を示していない。
- **大きな process output:** workflow は返却範囲を抑えたが、Worker prompt に 45,113 bytes / 11,123 input tokens を materialize した。大きな中間値を扱う場合、binding の対象を狭めるか summary を生成しない限り、workflow を使うだけでは prompt cost は下がらない。
- **繰り返し利用する workflow:** saved workflow の定義はこの例で dynamic definition より 50 bytes 小さく、interface は同じ 82 bytes だった。反復間 wall time のばらつきから saved 実行が速いとは判断しない。定義を再生成せず、公開 interface で呼び出せる運用上の再利用性を基準に選ぶ。
- **親へ DAG 全体を公開する必要がない場合:** `cheaper workflow interface <path>` を使う。測定した interface object は private definition より 272 bytes 小さかった。親 token telemetry は unavailable なので、実際の parent-token savings は未測定である。

## 再測定

`benchmarks/README.md` の手順で再実行できる。scheduler、prompt、model、CLI が変わった後は、同じ seed・反復数で `benchmarks/results/` に新しい UUID artifact を作り、古い結果を上書きせず比較する。canonical `mise run check` と merge 判定は coordinator が実行する。

# Workflow orchestration benchmark

この harness は seeded fixture と fake Worker stream を使い、実 provider を呼ばずに workflow arm と scripted direct arm を比較する。CLI は必ず built entry を subprocess として起動し、`NODE_ENV=test` と `CHEAPER_TEST_FAKE_STREAM` を設定する。

実測済みの 7 scenario / 17 arm / warm-up 2 回 + 測定 15 回の集約 artifact は [結果 JSON](./results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json)。対応する解釈は [Workflow の使い分け](../docs/guides/workflow-usage.md) を参照。

## 実行

```powershell
mise exec -- pnpm build
mise exec -- node benchmarks/run.mjs
```

既定値は arm ごとに warm-up 2 回、測定 15 回。シナリオを限定した smoke は次のように実行する。

```powershell
mise exec -- node benchmarks/run.mjs --scenario single-simple-tool-call --warmups 0 --repetitions 1
```

`--scenario` には `benchmarks/scenarios/scenarios.json` の ID を渡す。`--output <path>` は既存ファイルを上書きせず新規作成する。既定の保存先は UUID 名の `benchmarks/results/<run-id>.json`。各結果には source revision、fixture revision、seed、OS、architecture、Node、CPU 数、arm ごとの観測値、median/p95、比較差分が含まれる。

## 計測と会計

- workflow の Worker usage は成功 envelope の `jobs[].meta` から一度だけ集計する。workflow run は `usage.jsonl` に記録されない。
- direct Worker sequence は各 direct CLI envelope から計測し、workflow aggregate と加算しない。JSONL の件数は記録しているが、aggregate の token source として再加算しない。
- prompt bytes は fake stream が受け取った各 serialized context の UTF-8 byte 数。retained bytes は workflow の返却 result と artifact metadata、direct arm では retained inputs と child result の byte 数。
- compact interface bytes は CLI envelope 全体ではなく `result` object を JSON serialize したサイズ。可変な `meta.durationMs` は除く。
- 親 agent の token usage/turns telemetry は接続されていないため `parentUsage.status=unavailable`。direct CLI invocation 数は proxy としてのみ記録し、親 turn の実測や節約として扱わない。
- 結果の `comparisons` は左 arm − 右 arm。値が測定できないか反復間で変化した dimension は `null` のままにする。

シナリオ追加時は `scenarios.json` と `lib/scenarios.mjs` の必須 coverage を合わせ、schema、bindings、direct arm が同じ作業を表すことを検証する。canonical `mise run check` は coordinator の merge 時に実行する。

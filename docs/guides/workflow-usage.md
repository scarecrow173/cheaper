---
title: "Workflow の使い分け"
---

# Workflow の使い分け

## 測定範囲

以下の数値は **TS 時代測定（legacy）** — TS CLI 時代の旧 Node
harness による seeded run
（`benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json`、
測定 source revision `b3213f8c089c07d521e834df282c0c778b729054`）
に基づく参考記録で、削除せず保持する。各 arm は warm-up 2 回の後に
15 回測定し、fake Worker stream を使った。OS は Windows 10.0.26200、
x64、Node v26.10.0、CPU 16 個。壁時計時間はこの環境での CLI 起動を
含む測定で、親 agent の実 token usage は接続されていない。

現行の Rust-native harness（`crates/cheaper-bench`・manifest/result
schema v2）では計測経路が変わる（fixture child 化・
`direct-filesystem` の in-process 化等）ため、v1 artifact の値と
v2 の値は直接比較しない（`benchmarks/README.md`「baseline 同等性」
参照）。

`direct-worker-sequence` は再現可能な script が direct CLI を順番に起動する比較 arm であり、実際の親 agent の turn 数や token usage ではない。親 token の削減量はこの結果から算出しない。

## 結果

表の時間は median / p95、差分は workflow − direct arm。prompt bytes は Worker に実際に materialize された context、retained bytes は harness が保持した結果・中間値を表す。

| Scenario | Workflow median / p95 | Direct median / p95 | Input tokens (workflow / direct) | Prompt bytes (workflow / direct) | Retained bytes (workflow / direct) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 単純な read tool 1 回 vs filesystem read（v1 spawn） | 2,795 / 2,849 ms | 0.1 / 0.2 ms | 0 / 未計測 | 0 / 0 | 45 / 33 |
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

## Rust-era 測定（manifest v2 — 実測）

現行 harness（`crates/cheaper-bench`・result schema v2）による実測。掲載値は全て run artifact [`benchmarks/results/f4dfecf7-6b76-44d5-b830-f129334288ac.json`](../../benchmarks/results/f4dfecf7-6b76-44d5-b830-f129334288ac.json) の `aggregate` 由来。

- **runId** `f4dfecf7-6b76-44d5-b830-f129334288ac`・**repositoryRevision** `1fbdf1c48a5df5f2f2eb082d7475b3cc97ef55e8`（測定時 HEAD）・**fixtureRevision** `seeded-v2`・schemaVersion 2
- **coverage**: required 12 scenario 全 35 arm・warm-up 2・15 reps・595 observations・`armExpect` mismatch 0・exit 0
- **計測 binary**: `target/release/cheaper.exe`（release profile・preflight dev-seam probe passed）
- **環境**: Windows 10.0.26200.9457・x86_64・CPU 16・rustc 1.99.0・harnessProfile release

**claim 境界**: 全 arm の `parentUsage.status=unavailable` — 親 token 削減・費用削減は未測定で、この結果から主張しない。`--pricing` run は行っていないため estimated cost も掲載しない（scripted usage 由来の推計のみ可能で、実料金比較ではない）。usage は fixture `usage_delta` 宣言の deterministic 値（`source:"scripted"`）。

**baseline 同等性**: これらの値は fixtureRevision `seeded-v2`（fixture child 化・`direct-filesystem` の in-process `std::fs::read`・Rust 命名 environment）由来で、v1（TS 時代・`seeded-v1`）artifact の絶対値とは直接比較しない — `benchmarks/README.md`「baseline 同等性」参照。

### 継続 scenario（workflow vs direct）

| Scenario | Workflow median / p95 | Direct median / p95 | Input tokens (workflow / direct) | Retained bytes (workflow / direct) |
| --- | ---: | ---: | ---: | ---: |
| `single-simple-tool-call` | 167.5 / 192.0 ms | `direct-worker-tool-call` 169.7 / 216.0 ms・`direct-filesystem` 0.1 / 5.7 ms | 0 / 1,000 | 45 / 108 |
| `independent-fanout-worker-fanin` | 198.8 / 213.6 ms | `direct-worker-sequence` 228.3 / 348.5 ms | 1,000 / 1,000 | 36 / 153 |
| `tool-to-worker-chain` | 179.3 / 211.1 ms | `direct-worker-sequence` 178.6 / 219.8 ms | 1,000 / 1,000 | 87 / 144 |
| `process-and-analysis` | 2,105.0 / 2,238.8 ms | `direct-worker-sequence` 167.8 / 216.1 ms | 1,000 / 1,000 | 4,233 / 4,204 |
| `large-intermediate-process-analysis` | 2,127.0 / 2,293.3 ms | `direct-worker-sequence` 167.4 / 203.9 ms | 1,000 / 1,000 | 33,529 / 32,876 |

この fixture 群では workflow の retained result が direct sequence より小さかった（fan-in で 36 vs 153 bytes）。process 系 2 scenario では workflow が約 1.9–2.0 秒長い — fixture child exec が workflow job 内で走るのに対し direct arm は spawn 測定のみ（v1 の node emit spawn とは実装が異なり、絶対値は fixture child 由来）。小さな deterministic tool work だけなら direct invocation が適している — `direct-filesystem`（`std::fs::read`）は 0.1 ms だった。run-logging 系 arm（off/file/debug）は direct-worker-tool-call と同じ usage で wall 差は ±30 ms 内に収まった。

### Discovery / reuse

| 比較 | 結果 |
| --- | --- |
| `dynamic-workflow` definition | 398 bytes・interface 82 bytes・median 215.0 ms |
| `saved-workflow` definition | 348 bytes・interface 82 bytes・median 200.7 ms |
| `full-definition` read | 351 bytes・median 0.7 ms |
| `compact-interface` | interface 79 bytes・median 25.5 ms |

compact interface は full definition より 272 bytes（約 77.5%）小さかった（`claim: measured`）。interface 呼出の 25.5 ms は CLI 起動分 — byte 削減を token 節約や wall-time 短縮と読み替えない。saved definition は dynamic より 50 bytes 小さく、median 差 14.3 ms は反復ばらつき内で saved 実行が速いとは判断しない。

### Route scenario

| Arm | median / p95 | 備考 |
| --- | ---: | --- |
| `route-resolve` | 25.0 / 71.5 ms | resolve のみ |
| `route-rewrite` | 23.4 / 30.8 ms | rewrite payload 生成 |
| `route-run`（routed-exec） | 2,131.2 / 2,420.9 ms | usage rec 1・condense 未発火（origTok 769） |
| `direct-child-exec` | 6.0 / 42.3 ms | fixture child spawn のみ |
| `route-run`（condense-and-refetch） | 2,141.1 / 2,230.5 ms | **condense 発火・truncated**（origTok 12,137） |
| `route-logs` | 18.9 / 53.1 ms | logRefetch 1 件（condense 後の refetch 計測） |
| `route-run`（child-nonzero-exit） | 2,132.0 / 2,481.2 ms | exit 7 を mirror |
| `direct-child-exec`（nonzero） | 6.4 / 42.0 ms | exit 7 |

route orchestration の overhead は `route-run` − `direct-child-exec` ≈ 2.1 秒（route envelope・policy grant・log store 処理を含む end-to-end）。32 KiB 級の child output で condense が発火し truncation + log refetch の経路が計測された。child の nonzero exit（7）は route arm・direct arm ともそのまま mirror された。

### 失敗 matrix（`worker-failure-modes`）

| Arm | median / p95 | exit | errorCode |
| --- | ---: | ---: | --- |
| `direct-provider-error` | 44.1 / 105.9 ms | 5 | PROVIDER_FAILURE |
| `workflow-provider-error` | 51.3 / 54.4 ms | 6 | TOOL_EXECUTION_FAILURE |
| `direct-worker-timeout` | 646.9 / 676.3 ms | 7 | TIMEOUT |
| `workflow-worker-timeout` | 636.2 / 665.4 ms | 7 | TIMEOUT |
| `direct-postprocess-failure` | 48.1 / 88.0 ms | 6 | INVALID_OUTPUT |
| `workflow-postprocess-failure` | 57.1 / 129.7 ms | 6 | TOOL_EXECUTION_FAILURE |

全失敗形が `armExpect` 宣言と一致した（envelope・exitCode・errorCode・postprocessFailures pin）。workflow の timeout は scenario `policy` overlay 値で ~640 ms に収まった。usage は失敗形でも scripted 値のみ — 親 token・費用への言及は `parentUsage` unavailable のため行わない。

### 再測定の区別

この artifact は append-only（`benchmarks/results/` に非上書き保存）で、v1 JSON には diff が無い。掲載値の粒度（median/p95・retainedBytes・condense/refetch・exit mirror）は artifact `aggregate` の measured claim のみ — unavailable や estimated の entry は存在しない。

## 選択基準

- **1 回の単純な read / grep / process:** direct invocation を選ぶ。測定した tool-only workflow は v1 の filesystem read spawn（現在の `direct-filesystem`/`std::fs::read` arm に相当）と比べて CLI 起動分の wall time が大きかった。
- **少数の tool → Worker 処理:** workflow の Worker token は direct sequence と同じ（v2 fixture では input 1,000・output 30 の scripted 値で一致）。v2 測定では wall time が scenario 依存（fan-in/chain は同等以下、process 系は fixture child exec 分 ~2 秒長い）だった。親側の turn/context が別途観測されない限り、workflow をコスト削減として主張できない。
- **独立 tool jobs と fan-in:** fan-out と依存関係を一つの workflow に表現し、必要な summary だけを返す場合に使う。v2 測定では retained result が 117 bytes 小さかった（36 vs 153）。read tool が短時間で終わったため同時実行の high-water は 1 であり、この run は並列速度向上を示していない。
- **大きな process output:** v2 測定では workflow が 33,529 bytes を retained し、Worker usage は direct sequence と同じ scripted 値（input 1,000）だった。大きな中間値を扱う場合、binding の対象を狭めるか summary を生成しない限り、workflow を使うだけでは prompt cost は下がらない。
- **繰り返し利用する workflow:** saved workflow の定義はこの例で dynamic definition より 50 bytes 小さく、interface は同じ 82 bytes だった。反復間 wall time のばらつきから saved 実行が速いとは判断しない。定義を再生成せず、公開 interface で呼び出せる運用上の再利用性を基準に選ぶ。
- **親へ DAG 全体を公開する必要がない場合:** `cheaper workflow interface <path>` を使う。測定した interface object は private definition より 272 bytes 小さかった。親 token telemetry は unavailable なので、実際の parent-token savings は未測定である。

`cheaper-usage` skill の L0 判定規約（通常作業中の Workflow 候補検出）と後続の `cheaper-workflow-discover`（L1、#203）は「候補を見つける」入口であり、上記の選択基準が Workflow 化の可否を決める正本である — L0/L1 が候補と判定しても本節の基準で不適と結論付けられる場合がある。詳細は [parent-agent 連携ガイド](./parent-agent-integration.md#workflow-候補の軽量判定l0) を参照。

## tool job の exec grant 参照

`kind:"tool"` job が使う `tool:"process"`・`tool:"command"` の canonical 名と input shape（`{"task":...}` / `{"name","args"}`）は不変。policy 上は両者とも単一 `exec` grant namespace の grant として解決される（`process` task は `fixed:true` grant、`command` grant は `!fixed` grant — [configuration ガイド](./configuration.md) の unified `exec` grant namespace 節を参照）。workflow の resource inference（producer/consumer edge と resource manifest）は grant の `inputs`/`outputs` 宣言を参照し、未宣言の grant は保守的な workspace root subtree claim に落ちる。`outputs: []` は「副作用を宣言しない」契約で、resource claim を持たず shared access として扱われる（task の I/O 宣言規約は [managed process ガイド](./managed-process.md) を参照）。

## Project workflow catalog（`workflow list`）

project に保存した再利用 workflow を private DAG なしで発見できるよう、
`<workspace>/.cheaper/workflows/catalog.json` に manifest
（schemaVersion 1）を置く規約がある。catalog は**発見用 index** で
あり、workflow 定義の正本・grant/policy の根拠・実行経路ではない。
実行は従来どおり `cheaper workflow run <path>` の明示 path のみ
（catalog の `id` で実行する named lookup は存在しない）。

### manifest schema（schemaVersion: 1）

```json
{
  "schemaVersion": 1,
  "workflows": [
    {
      "id": "review-change",
      "path": "flows/review-change.json",
      "summary": "Review a diff and report findings",
      "tags": ["review", "git"]
    }
  ]
}
```

| field | 制約 |
| --- | --- |
| `id` | 必須。`[A-Za-z0-9][A-Za-z0-9._-]*`・64 文字以内・catalog 内で一意 |
| `path` | 必須。workspace root 相対の workflow JSON path（512 文字以内。絶対 path は malformed） |
| `summary` | 必須。500 文字以内 |
| `tags` | 任意。16 個まで・各 64 文字以内の string array |

manifest 全体は 64 KiB・256 entry まで。上限超過・JSON 不正・
非対応 `schemaVersion`・`workflows` 非 array は manifest 破損として
`workflow list` が error envelope（`INVALID_INPUT`）+ 非 0 exit で
fail-closed する。

### 登録・更新と検証

1. workflow JSON を workspace 内（例: `.cheaper/workflows/` 配下や
   任意の workspace-relative path）に保存する。
2. `catalog.json` の `workflows` array に `{id,path,summary,tags?}`
   entry を追記する（id の重複に注意）。
3. `cheaper workflow list` で登録状態を検証する。

新規 workflow の設計・検証・登録を親 agent へ委譲する場合は
`cheaper-workflow-create` skill（APM package 同梱）がこの手順を
担う。定義 JSON を組み立て、`cheaper workflow interface <path>`
で静的検証し、catalog へ `{id,path,summary,tags}` を追記して
`workflow list` で確認する。定義の保存と実行は別 — skill は
`workflow run` を起動せず、grant/policy/Route も変更しない。

entry ごとの `status` は `ok`（file 存在・通常 file）/
`missing`（file 不在）/`invalid`（workspace 外・deny 対象・
非 file・shape 不備）/`duplicate`（id 重複。該当 entry 全てに
付き、後勝ち上書きはしない）。`diagnostics` の `kind` は
`malformed-entry`/`duplicate-id`/`missing-path`/
`path-out-of-workspace`/`denied-path`/`not-regular-file`。
stale entry は一覧に残り status で報告されるだけで、実行を妨げ
ない・自動修復もされない。catalog は read-only で、未登録の
workflow file を走査したり `.cheaper/` を書き換えたりしない。

### `workflow list` の出力

`{catalog:{path,present}, workflows, diagnostics, truncated}` を
utility envelope で返す。`workflows[]` には manifest の
`id`/`path`/`summary`/`tags` + 検証結果の `status` のみが載り、
workflow JSON の jobs/prompt/binding などの private field は
出力されない。詳細は [usage ガイド](./usage.md#workflow-list)を参照。

## Workflow 候補の発見と提案（L1）

`cheaper-workflow-discover` skill（L1、#203）は、L0 が検出した
候補またはユーザーの明示的な検討要求に対し、上記 catalog と
`workflow interface` を使って既存 Workflow・direct tool・
Worker・Route・新規 Workflow を比較する評価層である。
比較基準（tool-only 処理の overhead・親 token 未観測の claim
境界・fan-in/中間出力の扱い）の正本は本ガイドの
[選択基準](#選択基準)節、evidence label
（measured/estimated/unknown）を含む比較手順・出力契約の
規約の正本は skill 本文であり、guide はそれを適用・参照する —
L1 の提案は作成でも実行でもなく、永続化・登録・実行は別途
承認が要る。親 agent 側の導線詳細は
[parent-agent 連携ガイド](./parent-agent-integration.md#l0--l1-handoff-で読まれた後の-cheaper-workflow-discover)
を参照。

## 再測定

`benchmarks/README.md` の手順（`cargo build -p cheaper-cli --features dev-seam` → `cargo run -p cheaper-bench`）で再実行できる。scheduler、prompt、model、CLI が変わった後は、同じ seed・反復数で `benchmarks/results/` に新しい UUID artifact を作り、古い結果を上書きせず比較する。canonical `mise run check` と merge 判定は coordinator が実行する。

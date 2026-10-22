---
id: "DESIGN-034KqI29dOTRXERRPTp5gV"
type: "design"
status: "approved"
title: "usage.jsonl run error record の診断 field 拡充 (issue #141)"
created: "2026-10-23"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/cli.rs"
    - "crates/cheaper-cli/src/run.rs"
    - "crates/cheaper-cli/src/envelope.rs"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-runtime/src/run/abort.rs"
    - "crates/cheaper-runtime/src/run/repair.rs"
    - "crates/cheaper-rig-adapter/src/error.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/run-error-record-observability.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./delegation-usage-measurement-log.md"
    - "./usage-log-recording-scope-docs.md"
    - "./usage-savings-telemetry.md"
    - "./worker-run-nullable-maxturns-and-stall-timeout.md"
    - "./worker-schema-structure-not-magnitude.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/run-error-record-observability.md"
    - "../adr/usage-error-record-field-contract.md"
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# usage.jsonl run error record の診断 field 拡充 (issue #141)

## Context

SPEC-G1ALq0d5lyFIi0h43tZsZG(spec)・ADR-RrljT4yH9woiGWbIny9ifG(ADR)の
実装設計。DISC-034Kqi8LcVZaxgz7wxHfg4(issue #141 調査)で確定した構造的
observability gap を解消する: `build_usage_record`
(`crates/cheaper-cli/src/usage_log.rs:80-160`) が `kind:"run"` error
record に書くのは `error: {"code": ...}` のみで、`error.message`・
`error.details`・`meta.repairs`・binary version・実効 policy の
いずれも永続 telemetry に残らない。

コード読解で確定した重要な構造的事実 — **record builder は error
型(`CheaperError`/`WorkerRunError`)を見ない**。入力は serialize 済み
envelope `serde_json::Value`(`env`)であり、必要な診断情報の大半は
**既に envelope contract 上に存在する**:

- `env.error.message` — `error_envelope` が `e.message()` を常に書く
  (`envelope.rs:187`)
- `env.error.details` — `e.details().cloned()` の全 key
  (`envelope.rs:190-192`)
- `env.meta.repairs` — `error_meta`(:142)/`WorkerRunMeta.repairs`
  (INVALID_OUTPUT 経路は `with_run_meta` で `PartialRunMeta` が付く —
  `run/mod.rs:815`)

record が持たない入力は 2 つだけであり、これが本 design で配線する
対象: **product version** と **実効 `execution` echo**
(`{timeoutMs, maxTurns, maxToolCalls}` — `meta.execution` は
`ExecutionMetrics` = 観測 metrics であり policy echo ではない、
`execution_metrics.rs:40-72`)。

## Scope

### In Scope

- `crates/cheaper-cli/src/usage_log.rs` — `ExecutionEcho` 型・
  `build_usage_record`/`record_usage` の `echo` 引数・error object の
  allowlist projection・文字数 cap 用の `truncate_chars` helper
- `crates/cheaper-cli/src/cli.rs` — `RunCtx.execution_echo` field と
  `record_usage` 3 呼出し箇所の echo 受け渡し
- `crates/cheaper-cli/src/run.rs` — `restrict_policy` 後の
  `ctx.execution_echo` 設定
- `crates/cheaper-cli/tests/integration/usage_log.rs` — signature 更新
  と新 field の record contract test
- `docs/guides/usage.md`・`AGENTS.md` — run record schema 記述の追随

### Out of Scope

- envelope(stdout)公開 contract — `meta`/`error` の構造・内容は一切
  変更しない(ADR Decision 2。`schemaVersion: 1` 維持)
- runtime(`cheaper-runtime`)・core(`cheaper-core`)・adapter の error
  生成・details 内容 — record 側の選択・切り詰めのみを規定
- `kind:"count"`/`kind:"route"`/`kind:"route-refetch"` record
- `cheaper usage` の集計・filter 機能(新 field の集計利用は別 issue)
- `schemaVersion` bump(additive 追加のため不要 — 後述)
- `<userConfigDir>/invalid-outputs/`・`<userConfigDir>/checkpoints/`
  の sidecar 書き込み機構(現行のまま。record は参照 path のみ扱う)

## Components and Boundaries

- **`usage_log.rs`** — record schema の単一の責務境界(envelope `Value`
  → record `Value` の変換)。error 診断 field の allowlist・切り詰め・
  key 順はここに集約し、呼出し側(`cli.rs`)は echo の供給のみを担う。
  表現変換の集約点(AGENTS.md の表現ドリフト原則)。
- **`cli.rs` の `RunCtx`** — 「dispatch 内で run 経路が埋める
  context(error path の記録用)」の既存境界(`cli.rs:302-308`)。
  `run_root`/`run_user_config_dir`/`worker_name` と同じ carrier に
  `execution_echo` を載せることで、success arm(`cli.rs:464`)と
  汎用 catch(`cli.rs:266`)の両方が新 signature を経由せず同じ値を
  参照できる。
- **`run.rs`** — 実効値の唯一の供給源。`restrict_policy`
  (`run.rs:223-229`)の出力 `effective_policy.execution` は
  `--timeout` narrow・policy 層 merge を通った**解決後**の値であり、
  declared 値ではない(spec Req 2 そのもの)。
- **`envelope.rs`・runtime/core** — 無変更境界。`env.error.details`・
  `env.meta.repairs` は既存 contract を読むだけ。

## Data and Control Flow

### 配線決定(spec Open Item の確定)

「envelope `meta` への additive 追加」案は却下する。envelope は
stdout 公開 contract であり additive であっても regression fixture
(`tests/regression/expected/*.json`)と consumer 依存を動かす。
record 専用の値は record 専用の経路で運ぶ:

1. **`version`**: `env!("CARGO_PKG_VERSION")` を
   `build_usage_record` 内部で `kind:"run"` 両分岐に挿入する。
   compile-time 定数のため配線・失敗モード・builtin_dir 依存を持た
   ない。canonical source(workspace `Cargo.toml`
   `[workspace.package].version`)と `build.rs` 生成 `version.json`
   (`embedded.rs:39-41`)は同一値 — record が記すべきは「どの binary
   が走ったか」なので binary 自身の版で正しい。
2. **`execution` echo**: 下記 `ExecutionEcho` を `usage_log.rs` に
   定義し、`restrict_policy` 直後に `ctx.execution_echo` へ設定する。
   `record_usage`/`build_usage_record` に `echo: Option<&ExecutionEcho>`
   引数を追加し、run の 2 呼出し箇所は
   `ctx.execution_echo.as_ref()`、count-tokens 呼出し箇所は `None`
   を渡す。

```rust
/// `kind:"run"` record の `execution` echo — `restrict_policy` 解決後の
/// `ExecutionPolicy` から 3 field のみ写す固定 allowlist projection。
pub struct ExecutionEcho {
    pub timeout_ms: u64,
    /// `None` は policy 上の turn 上限なし — record では `null` として
    /// 記録し「key 無し(=echo 未供給)」と区別する(spec Req 2)。
    pub max_turns: Option<u64>,
    pub max_tool_calls: u64,
}

impl From<&cheaper_core::policy::schema::ExecutionPolicy> for ExecutionEcho { /* 3 field 写像 */ }
```

`ExecutionPolicy`(`schema.rs:112-124`)は `concurrency` 等他 field を
持つが、record contract は 3 field 固定 — `From` impl に射影を集約
することで policy struct の成長が record shape に漏れない。

### Record schema(additive 分のみ)

`kind:"run"` record の key 挿入位置(insertion order は determinism
のため固定する):

| field | 位置 | 型 | 出所 | 条件 |
| --- | --- | --- | --- | --- |
| `version` | `ok` 直後 | string | `env!("CARGO_PKG_VERSION")` | 常時 |
| `repairs` | `savings` の後 | number | `env.meta.repairs` | non-null のみ(既存 `!is_null` 規約と同じ — `usage_log.rs:146-150`) |
| `execution` | `repairs` の後 | object | `echo` 引数 | `Some` のみ |
| `error.*` | 既存 `error` object 内 | 下表 | `env.error.*` | 下表 |

`execution` の値: `{"timeoutMs": <u64>, "maxTurns": <u64|null>,
"maxToolCalls": <u64>}` — 3 key 常時(`maxTurns` のみ null 可)。
record の `execution` は policy echo であり、envelope
`meta.execution`(`ExecutionMetrics`、record には現行も載らない)とは
別概念 — 同名だが面が違うことを `usage.md` で明示する。

`error` object の拡張(`code` 既存 + 固定順で以下を追加):

| field | 出所 | 条件・cap |
| --- | --- | --- |
| `reason` | `env.error.details.reason` | string かつ abort 語彙 `{"timeout","signal","maxTurns","maxToolCalls"}`(`RunAbortReason::as_str`、abort.rs:36-43)に一致するときのみ |
| `message` | `env.error.message` | string → 先頭 300 文字 |
| `schemaErrors` | `env.error.details.errors` | array のとき先頭 5 件を `{instancePath, message}` で写す(`instancePath` ≤200 文字、`message` ≤120 文字。object entry のみ、欠落 key はその key を省略) |
| `errorMessage` | `env.error.details.errorMessage` | string → 先頭 300 文字 |
| `checkpoint` | `env.error.details.checkpoint` | object のとき `{path, observationsCount}` のみ写す — それ以外の key を将来追加されても漏らさない。両 key 欠落なら `checkpoint` 自体を省略 |

切り詰めは char(Unicode scalar)基準の `truncate_chars` —
`repair.rs:223-228` と同一規約の private helper を `usage_log.rs` に
置く(runtime 側は private のため再利用不可。5 行の local helper で
十分、KISS)。

### `details` 供給元の対応(record が読む envelope field の生成側)

| record field | 生成元 | 内容 |
| --- | --- | --- |
| `reason` | `RunAbortReason::cheaper_error`(abort.rs:56-65) | `{reason: <as_str>}` — TIMEOUT/ABORTED/MAX_TURNS/MAX_TOOL_CALLS |
| `schemaErrors` | `invalid_output_error`(repair.rs:190-218) | `details.errors`(≤20、`{instancePath,message}` の `SchemaErrorEntry`)から record 側で ≤5 に絞る |
| `errorMessage` | `provider_error_to_cheaper`/`report_to_cheaper`(rig-adapter error.rs:85-94, 119-143) | `{errorMessage}` のみ — PROVIDER_FAILURE details contract |
| `checkpoint` | `classified()`(run/mod.rs:226-253)→ `checkpoint_details`(:863-870) | `{path, observationsCount}` — `<userConfigDir>/checkpoints/<id>.json` への参照 |

なお `PromptError::UnknownToolCall` は details 無しの PROVIDER_FAILURE
を生成する(`prompt_error_to_cheaper`、rig-adapter error.rs:172-175)—
この経路では `errorMessage` は載らず、診断は `error.message` 側が担う。

`invalid-outputs` sidecar(`<userConfigDir>/invalid-outputs/
run-<pid>-<nanos>.txt`、repair.rs:163-185)は `details.rawOutputPath`
として envelope に出るが、**record には写さない** — ADR Option A の
固定 allowlist に含まれず、`rawOutput`/`rawOutputPath` は
「中身・payload 参照を durable log に残さない」境界の内側。
`exitCode`・`retryable`・`cause`・`details` の残余 key
(`rawOutput`/`changes`/`missing`/`grant`/`layer` 等)も写さない。

### 既知の構造的 edge

- **`PARTIAL_MUTATION`**: `classified()`(mod.rs:234-247)は mutation
  あり abort を `PARTIAL_MUTATION` で wrap し、`reason` は inner
  `cause` の details に残る — envelope `cause` は `{code,message}`
  に畳まれる(envelope.rs:193-200)ため、record の `reason` は
  **載らない**。`checkpoint` は外側 details に複写済みで記録される
  (budget 枯渇の同定は可能)。`cause.code` を record に写す拡張は
  spec 外(allowlist 固定)のため採用しない。
- **`details.checkpoint` は best-effort**: `save_budget_checkpoint`
  (mod.rs:263-279)は失敗時に `None` を返し `details.checkpoint` 自体を
  省略するだけなので、MAX_TURNS/MAX_TOOL_CALLS abort の record でも
  `checkpoint` key が載らない経路がある。test は `checkpoint` の
  無条件の存在を assert しない。
- **echo 未供給の run record**: usage-bearing error は全て
  `restrict_policy`(`run.rs:223`)以降の `run_worker` 内で発生する
  (それ以前の worker/model/input 解決 error は `run_meta` を持たず
  `meta.usage`/`meta.model` 欠落で記録対象外 — usage_log.rs:133-137)。
  したがって record 対象 run で `echo` が `None` になる経路は構造的に
  存在しないが、防御的に `None` 時は `execution` key を省略するだけ
  (fail-soft、record 書き込みは落とさない)。

## Non-Functional Constraints

- **additive-only 互換**: 追加は全て optional key。`schemaVersion`
  は `1` を維持 — 根拠は reader 側が `is_record` で `kind` のみを
  検査し未知 field を無視すること(`usage_report.rs:219-242`)、および
  既存行は一切変わらないこと。新旧 writer の record は同一
  usage.jsonl に混在可能(現行 `turns`/`savings` と同じ規約)。
- **fail-soft**: 対応値が無い field は key 自体を省略。新 field の
  欠落・型不一致で `build_usage_record` が `None` や panic になら
  ないこと(record 書き込み自体を落とさない — best-effort append
  規約の維持)。
- **privacy/容量境界**: allowlist 固定。model 生出力・workspace 内容
  (`rawOutput`)は写さず、absolute path は `checkpoint.path`
  (resume 参照先として必須)のみに限定(ADR Option B 却下理由と同じ
  境界)。1 行の増分は数十〜数百 byte。
- **`cheaper usage` 非依存**: 既存集計(`--since`/`--worker`/
  `--provider`/`--model`/`--compare-model`)は新 field の存在下でも
  同じ結果を返す — reader が既知 field のみ読む構造で保証される
  (spec Req 7)。
- **determinism**: record の key 順は上表の固定位置に挿入 — 既存の
  insertion-order 規約(`usage_log.rs:4-6` の TS object literal 順
  継承)を崩さない。

## Risks and Trade-offs

- **allowlist drift**(ADR 既知リスク): `error.details` に新 key が
  追加されても record は自動追従せず、drift 検知機構もない。緩和:
  分類に必要な field は spec/ADR で固定済みであり、新しい
  診断 key の追加時に本 design の対応表を更新する運用ルール。
- **`RunCtx` 経由の echo は暗黙の手順依存**: `restrict_policy` 後の
  設定を忘れると `execution` が静かに欠落する。緩和: `echo` は
  明示引数(`Option`)であり signature 変更で全呼出し箇所が compile
  error になるため配線漏れは検出される。設定位置は
  `ctx.run_user_config_dir` と同じ「記録用 context を早期に置く」
  既存パターンに揃える。
- **`version` が `env!` 固定**: `CliHooks.builtin_dir` override で
  別 version.json を指す test 環境でも binary 版を記録する。意図的 —
  record が問うのは「どの binary が走ったか」であり、override
  resource の版ではない。
- **`env!("CARGO_PKG_VERSION")` の評価位置は `cheaper-cli` 固定**:
  `cheaper-runtime` は自身で `version = "0.1.0"` を pin しており
  (`crates/cheaper-runtime/Cargo.toml:3`)、runtime 側で `env!` を
  評価すると workspace version ではなく "0.1.0" が入る。record
  builder が `usage_log.rs`(cheaper-cli)にある限り正しい値になるが、
  record 生成を runtime へ移す場合は version の供給方式を見直すこと。
- **`message`/`errorMessage` の 300 字 cap で末尾が欠ける**:
  PROVIDER_FAILURE の `GetChatMessage HTTP <status>: <body>` は
  status が冒頭に来るため下位分類に必要な情報は冒頭側にある
  (rig_provider.rs:374-386)。cap 超過時も分類可能性は保たれる。

## Deferred Design Concerns

- `cheaper usage` への error 内訳集計(`--error` filter・reason 別
  件数)| spec Open Item で別 issue 候補と明記・record field を先に
  安定させるため | 分類 filter の需要が観測されたとき | record を
  書き出しても集計面が追い付かず jq 依存が残る
- `error.cause.code`(PARTIAL_MUTATION wrap 時の abort code)の record
  追加 | spec の固定 allowlist 外 | mutation 系 worker の abort 分類が
  必要になったとき | PARTIAL_MUTATION record は `checkpoint` のみで
  `reason` を同定できない(additive に追加可能)
- `rawOutputPath` の record 追加 | ADR Option A が payload 参照を
  allowlist から除外 | INVALID_OUTPUT の postmortem 解析需要が
  出たとき | envelope/invalid-outputs dir を直接見なければ raw
  output へ辿れない(additive に追加可能)

## Verification Notes

- [ ] `tests/integration/usage_log.rs` — error envelope fixture から
  `error.reason`/`message`/`schemaErrors`/`errorMessage`/`checkpoint`
  が記録されること、allowlist 外 key(`rawOutput`/`rawOutputPath`/
  `exitCode`/`cause`)が載らないこと
- [ ] 同上 — `version` が `env!("CARGO_PKG_VERSION")` と一致し、
  `execution` echo が `{timeoutMs, maxTurns(null 含む), maxToolCalls}`
  で記録されること(success・error 両分岐)
- [ ] 同上 — 300/120/200 字 cap と 5 件 cap の切り詰め、abort 語彙外
  `details.reason` の非記録、fail-soft(key 省略で書き込み継続)
- [ ] `cheaper usage` が新 field 混在 file を従来通り集計できること
  (spec AC)
- [ ] `run_dev_seam` 系または integration test で `run::execute` →
  `record_usage` の end-to-end を通し、`version`/`execution`/
  `error.reason` が実際の `ctx.execution_echo` 配線経由で pin される
  ことを確認する(fixture レベルの `build_usage_record` test のみに
  留めない — `restrict_policy` 後に echo を置く設定順序は暗黙の依存)
- [ ] `mise run check` が pass すること

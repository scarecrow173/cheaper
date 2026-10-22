---
id: "SPEC-G1ALq0d5lyFIi0h43tZsZG"
type: "spec"
status: "approved"
title: "usage.jsonl run record への version・実効 policy・error 診断 field の追加 (issue #141)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
    - "crates/cheaper-cli/src/usage_log.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/run-error-record-observability.md"
    - "../plans/implement-run-error-record-observability.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/usage-error-record-field-contract.md"
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/invalid-output-raw-output-persistence.md"
    - "../designs/run-error-record-observability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
  derived-by:
    - "../adr/usage-error-record-field-contract.md"
    - "../designs/run-error-record-observability.md"
    - "../plans/implement-run-error-record-observability.md"
    - "../test-specs/usage-jsonl-run-record-diagnostic-fields.md"
  verified-by:
    - "../test-specs/usage-jsonl-run-record-diagnostic-fields.md"
---

# usage.jsonl run record への version・実効 policy・error 診断 field の追加 (issue #141)

## 背景と目的

Issue #141 の調査(DISC-034Kqi8LcVZaxgz7wxHfg4)で確定した構造的制約:
`usage.jsonl` の `kind:"run"` error record は `error: {"code": ...}` のみを
書く(`crates/cheaper-cli/src/usage_log.rs:151-158`)。`error.message`・
`error.details`(abort の `reason`、INVALID_OUTPUT の `errors[].instancePath`、
PROVIDER_FAILURE の `errorMessage`、budget 枯渇の `checkpoint`)・
`meta.repairs`・binary version・実効 policy のいずれも記録されない。

結果として「TIMEOUT の duration が ~120s に密集する出所は旧版の 120s
絶対期限か、user/project policy の固定か」「INVALID_OUTPUT の 9 件は
schema 非適合か `output.maxTokens` 打ち切りか」「PROVIDER_FAILURE の
内訳は transport 失敗・HTTP 非 2xx・EOS 未到達のどれか」という
**分類上の必須情報が record に存在せず**、同種の失敗分析が今後も
telemetry から完結しない。envelope(stdout)には detail が残るが
永続化されず、consumer が envelope を保存しない限り失われる。

目的は、run record だけで (a) どの version が走ったか、(b) どの実効
budget/timeout が生きていたか、(c) error code の直下の原因分類
(abort reason・schema 失格箇所・provider 失敗の記述)を答えられる
最小限の診断 field を JSONL contract へ追加すること。

## スコープ

**対象:**

- `crates/cheaper-cli/src/usage_log.rs` の `build_usage_record` —
  `kind:"run"` record(success + usage-bearing error 両方)の field 拡充
- record へ流す実効値の供給経路(envelope `meta` への additive 追加、
  または `RunCtx`/`CommandOut` 経由の直接受け渡し — 配線方式は
  design の責務)
- `docs/guides/usage.md` の record schema 説明の追随

**除外:**

- record の write 経路・best-effort 規約・`schemaVersion` の変更
- `kind:"count"`/`kind:"route"`/`kind:"route-refetch"` record の field
- `cheaper usage` の集計・filter 機能(新 field の集計利用は別 issue 級)
- error envelope(`error.details`)自体の内容変更 — record 側の選択・
  切り詰めのみを規定する。ただし R4 の `meta.repairs` のように envelope
  に既存の値をそのまま写す field は除く
- consumer 環境ファイル(`~/.config/cheaper/` 以下)への runtime 外書き込み

## 要件

1. **`version` field**: 全 `kind:"run"` record(success・error)に
   product version(workspace `Cargo.toml` の canonical version)を
   `version` として記録する。旧版/新版の識別子。
2. **実効 execution echo**: 全 `kind:"run"` record に、run に適用された
   実効値 `execution: {timeoutMs, maxTurns, maxToolCalls}`(`maxTurns` は
   `null` 許容 — policy `null` と未設定を区別するため)を記録する。
   `--timeout`・policy 層 merge を通った**解決後の**値であり、
   declared 値ではない。
3. **error record の診断 field**: `ok:false` の run record の `error`
   object に、envelope の `error` 由来で次の field を条件付きで追加する。
   全て bounded(切り詰め上限を設ける):
   - `reason` — `error.details.reason` が abort 語彙
     (`timeout`/`signal`/`maxTurns`/`maxToolCalls`)のとき記録
   - `message` — `error.message` を上限 300 文字に切り詰めて記録
   - `schemaErrors` — `error.details.errors` が array のとき
     `{instancePath, message}` を先頭 5 件まで記録
     (INVALID_OUTPUT の失格箇所と失敗 reason の判別用)
   - `errorMessage` — `error.details.errorMessage`(PROVIDER_FAILURE の
     provider 記述)を上限 300 文字に切り詰めて記録
   - `checkpoint` — `error.details.checkpoint` の
     `{path, observationsCount}` をそのまま記録(budget 枯渇の
     resume 参照先)
4. **`repairs` field**: `kind:"run"` record(success・error)に
   `meta.repairs` を記録する(null 相当は省略)。INVALID_OUTPUT が
   repair 尽き由来か、初回 extract 失敗 → attempts=0 由来かを判別する。
5. **`savings` 等の既存 field は不変** — `model`/`usage`/`turns`/
   `toolCalls`/`savings` の記録条件・形は現行通り。
6. **fail-soft**: いずれの新 field も、envelope 側に対応値が無い場合は
   key 自体を省略する(現行 `turns`/`toolCalls`/`savings` の
   `!is_null` 規約と同じ — `usage_log.rs:146-150`)。新 field の欠落で
   record 書き込みを落とさない。
7. **`cheaper usage` との非依存**: 新 field の追加で既存の
   `cheaper usage` 集計(--since/--worker/--provider/--model/
   --compare-model)が変わらないこと(unknown field は無視される)。

## Acceptance Criteria

- [ ] `ok:false` の `kind:"run"` record に `version`・`execution`・
  `repairs` が記録され、`error` object が `code` に加えて条件該当する
  診断 field(`reason`/`message`/`schemaErrors`/`errorMessage`/
  `checkpoint`)を持つ。
- [ ] TIMEOUT の error record で `error.reason === "timeout"` かつ
  `execution.timeoutMs` が記録され、TIMEOUT の出所(絶対 vs stall、
  120s 等の値)が record から判別可能。
- [ ] INVALID_OUTPUT の error record で `schemaErrors[].instancePath` と
  `repairs` が記録され、失格箇所・repair 消費の有無が判別可能。
- [ ] PROVIDER_FAILURE の error record で `errorMessage` が記録され、
  HTTP status・transport 失敗・EOS 未到達等の下位分類が判別可能。
- [ ] MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED の record で
  `error.reason`(`maxTurns`/`maxToolCalls`)と `checkpoint.path`
  が記録される。
- [ ] 旧 schemaVersion=1 の既存行は変わらず、`cheaper usage` の
  既存集計が新 field の存在下でも同じ結果を返す。
- [ ] `mise run check` が pass する。

## Open Items

- `error.details` から record への field 配線方式(envelope `meta`
  の additive 拡張か、record builder への直接注入か)— design で確定。
- `schemaErrors` の件数上限・文字数上限の最終値(暫定 5 件・message
  120 字)は design で再確認。
- `cheaper usage` の `--error` 系 filter・エラー内訳レポートの追加は
  別 issue 候補。
- usage.jsonl の schemaVersion を bump するか(additive 変更のため
  現行方針は bump 不要と見込むが design で最終判断)。

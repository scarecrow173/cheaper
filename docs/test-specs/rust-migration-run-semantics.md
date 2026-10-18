---
id: "TSPEC-034Bgu0BW0cG2z21FznpfR"
type: "test-spec"
status: "proposed"
title: "rust-migration-run-semantics"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# rust-migration-run-semantics

## Purpose

Rust + Rig 移行で agent loop の実行基盤が Pi SDK から Rig に換わっても、
run の意味論(abort 分類・usage 会計・retry 粒度・tool call 計数)が
変わらないことを保証する意図を記録する。これらは envelope の `meta`
field と usage-bearing error record に直結するため、Rig API の意味論への
安易な準拠が許されない領域であり、設計書で固定された意味論ルールを
観測面から固定する。意味論固定ルール自体が改訂された場合にのみ見直す。

## Feature

`run_worker` オーケストレーションにおける abort・usage 集計・retry・
tool call 計数の意味論 parity

## Rules

### abort 分類と失敗包装

- abort trigger は `CancellationToken` + `tokio::time::timeout` + 外部
  signal で一元化し、`abort_reason` は先勝ちで記録する
- reason 分類は timeout→TIMEOUT・signal→ABORTED・maxTurns→
  MAX_TURNS_EXCEEDED・maxToolCalls→MAX_TOOL_CALLS_EXCEEDED とし、
  provider error は adapter 境界で `CheaperError` 系に写像して上位へ
  Rig error を漏らさない
- abort 記録済みで throw する際、mutation journal が非空なら
  PARTIAL_MUTATION で包み `cause` に元 error を保持する
- prompt abort grace settle(既定 5000ms)を維持し、signal を無視する
  provider stream のハングを防ぐ
- panic・tokio `JoinError` は `INTERNAL_ERROR`(exit 70)へ写像し、
  envelope 無し終了を許さない

### usage 集計の唯一の正

- usage/turns/toolCalls の収集源は hook ベースの逐次集計(per-turn
  usage を accumulator に加算)を唯一の正とする。`PromptResponse`/
  `completion_calls` は abort・provider error・`MaxTurnsError`・
  `PromptCancelled` で存在しないため meta 構築に使わず、取得できた場合の
  sanity check に留める
- repair session と transient retry の usage/turns/transientRetries は
  main run と同一 accumulator に合算する
- error/abort/maxTurns/INVALID_OUTPUT の全経路で meta snapshot は hook
  accumulator から構築し、usage-bearing error(`meta.model`+`meta.usage`
  を持つ error が usage.jsonl に記録される契約)を維持する
- `meta.usage.costUsd` は provider 応答値ではなく cheaper pricing 層
  (pricing snapshot × token 数)で算出する

### Rig 適用の固定意味論

- `default_max_turns` には十分大きな値を渡し、maxTurns 判定は cheaper
  側 hook(「`turns >= maxTurns` かつ最終 stop が tool 継続」のときのみ
  abort)のみが行う。Rig の `MaxTurnsError` が表面化した場合は内部
  guard 到達として扱い `MAX_TURNS_EXCEEDED` と混同しない
- `InvalidToolCallAction` は `Retry{feedback}`(corrective feedback で
  turn を巻き戻して loop 継続)のみを選択し、`Fail`・`Repair`・`Skip`・
  `Stop` は契約を変えるため禁止する
- `tool_concurrency` は 1 を明示設定し、provider 層の並行 dispatch を
  導入しない(sibling 並行意味は scheduler 側の責務)
- invalid/unknown tool call は tool runtime に到達しないため
  `toolCalls`・`rejectedToolCalls` のどちらにも計上しない(runtime 実行
  到達時のみ `toolCalls`、boundary/limit 拒否のみ `rejectedToolCalls`)
- transient retry wrapper は completion call 単位(transport/model-call
  層)に限定し、run 全体・tool 実行を包まない。429/5xx/network timeout
  を transient と分類し、retry 実施ごとに `transientRetries` を計上する
- run surface は streaming run のみを expose する(cancel/abort 中継と
  delta hook が streaming 面でのみ発火するため)

## Examples

- Given: `turns >= maxTurns` かつ最終 stop が tool 継続の応答列。Then:
  MAX_TURNS_EXCEEDED で abort し meta に accumulator の usage が残る。
  最終 stop が tool 継続でない場合は maxTurns 到達でも abort しない
- Given: `maxToolCalls` 上限到達。Then: onLimit が abort を起こし
  MAX_TOOL_CALLS_EXCEEDED に分類される
- Given: unknown tool name の tool call。Then: corrective feedback で
  loop が継続し、`toolCalls`/`rejectedToolCalls` はいずれも増えない
- Given: `error{status:429}` を挟む run。Then: completion call のみが
  retry され `transientRetries` が計上されるが、tool 実行や run 全体は
  再実行されない(非冪等 mutation の二重実行が起きない)
- Given: invalid output → repair session 2 attempt。Then: repair 側の
  usage/turns/transientRetries が main run の accumulator に合算され、
  attempts 使い切り時は `INVALID_OUTPUT`(details: errors 先頭 20 件・
  rawOutput 先頭 2000 文字)になる
- Given: repair 中に abort が記録される。Then: repair 継続ではなく abort
  reason 分類が優先される
- Given: provider error で run が失敗。Then: error envelope の meta に
  `model` と `usage` が載り(usage-bearing error)、usage.jsonl 記録対象に
  なる
- Given: run 境界内で panic を注入。Then: `INTERNAL_ERROR`(exit 70)の
  error envelope が返り、exit 101 相当の envelope 無し終了にならない

## Guarantees

- 成功・error・abort・maxTurns・INVALID_OUTPUT の全経路で
  `WorkerRunMeta` が同一 accumulator から構築され、経路による計数の
  欠落・二重計上が起きない
- abort 分類・PARTIAL_MUTATION 包装・prompt grace settle の意味が TS 版
  `run.ts`/`repair.ts`/`prompt-grace.ts` と同一責務で維持される
- Rig API の意味論差(model-call budget 型の max turns・invalid call
  action の選択肢・PromptResponse の存在条件)が cheaper の contract に
  漏れない

## Non-goals

- tool runtime 内部(scheduler・journal・denylist・scheduler の並行意味)
  の parity(Phase 2 側の contract test が担う)
- envelope serialization・exit code 表の byte 互換
  (`rust-migration-envelope-parity` が担う)
- repair の構造詳細(json_schema mode・error feedback 形式)自体の再設計
  — 意味論の維持のみ
- devin provider の wire 互換(実 endpoint 依存、CI 外 smoke)

## Risk

- この保証が落ちると、`meta` の計数(usage・turns・transientRetries)や
  abort 分類が経路によって破綻し、usage-bearing error record が欠落して
  `cheaper usage` の集計が実態とずれる。maxTurns や invalid call の意味論
  差は稀な失敗経路でのみ発火するため、differential golden の case 選択と
  単体検証の両方が無いと検出漏れする

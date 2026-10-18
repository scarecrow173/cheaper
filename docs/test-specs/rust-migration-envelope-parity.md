---
id: "TSPEC-034BgtMs1RxjQZjTTppcEZ"
type: "test-spec"
status: "proposed"
title: "rust-migration-envelope-parity"
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

# rust-migration-envelope-parity

## Purpose

Rust + Rig 移行後も、cheaper CLI の stdout / exit code 境界が TS 版と byte
レベルで互換であることを保証する意図を記録する。envelope は親 coding
agent・hook・launcher が機械的に parse する唯一の stdout contract であり、
serde への置換で生じうる数値表現・optional field・改行・panic 経路の
silent な乖離を検出することが目的。envelope schema 自体を変更する別
spec が発効した場合にのみ見直す。

## Feature

Rust binary における result envelope(serialization・exit code・stdout
境界)の TS 版 parity

## Rules

- run 系コマンド(worker alias・`run`・`count-tokens`・`usage`・`models`・
  `auth`・`workflow`)の stdout は `schemaVersion: 1` の envelope 1 JSON +
  末尾改行 1 つのみ。diagnostics・model line・secret・panic 出力は
  stdout に出ない
- envelope は success / utility / error / workflow の 4 形態を serde で
  厳密 serialize し、TS 版の `JSON.stringify(env) + "\n"` と byte 互換
  とする
- error 形態は `code`/`message`/`exitCode`/`retryable` を必須とし、
  `details`/`cause`/`savings` 等の optional field は「値が無いなら key
  自体を省略」する現行規則を再現する(`skip_serializing_if` 相当)。
  `cause` は CheaperError のときのみ `{code, message}` 形を取る
- error envelope の meta は取得不能 field を `null` で埋める(null 意味論
  の再現)
- exit code は `ERROR_TABLE`(INVALID_REQUEST=2 … INTERNAL_ERROR=70)の
  code→exitCode/retryable 写像をそのまま維持する
- envelope・usage.jsonl・report の数値 field(`costUsd`・`effectiveCost`・
  `outputPerInput`・`estimatedSavingsUsd` 等)は JS 互換 canonical number
  formatter を唯一の実装として通す。整値は `.0` なし、非有限値は
  `null`、`-0`→`0`、`|x| >= 1e21`・`0 < |x| < 1e-6` は JS 形式の指数
  表記とする
- `cheaper __grep-eval` 等の内部 mode は argv 先頭一致の早期分岐として
  envelope・`recordUsage`・汎用 catch の対象外に置き、envelope 契約を
  汚染しない
- panic・tokio `JoinError` は run/command 境界で `INTERNAL_ERROR`
  (exit 70)の error envelope に写像し、panic hook で stdout への panic
  出力を抑止する。envelope 無しの exit 101 等は contract 違反とみなす

## Examples

- Given: fake provider 駆動の正常系 run。When: TS binary と Rust binary
  を同一 input/env/cwd で実行。Then: stdout の envelope byte 列と exit
  code が一致する
- Given: `costUsd` が整値になる成功 run。Then: envelope の
  `meta.usage.costUsd` が `2` と出力され `2.0` ではない
- Given: 境界値の数値 field(`1e21`・`1e-6` 近傍・`-0`・非有限値)。Then:
  JS 形式の指数表記・`0`・`null` で serialize され、serde_json の f64
  既定出力(`1.0`・独自指数形式)が混在しない
- Given: `details`/`cause` を持たない error。Then: それらの key 自体が
  envelope に出現しない(`null` emit ではない)
- Given: worker 解決前の usage error。Then: error envelope の `code`/
  `exitCode` が ERROR_TABLE 写像通りで、meta の未取得 field は `null`
- Given: `cheaper __grep-eval <args>`。Then: runCli dispatch 前に内部
  mode として分岐し、stdout に envelope JSON を出力しない
- Given: run 境界内で panic を注入。Then: stdout に `INTERNAL_ERROR` の
  error envelope が 1 件だけ出て exit code は 70、panic trace は
  stdout に漏れない

## Guarantees

- 「stdout = envelope 1 JSON + 末尾改行」の不変条件が全コマンド・全
  error 経路・panic 経路で維持される
- 数値表現が `cheaper-core` の serializer helper 1 箇所に集約され、
  envelope・usage.jsonl・report の間で表現 drift が起きない
- exit code 表・error 形態の必須/optional field 規則が TS 版と同一の
  authoritative な写像(ERROR_TABLE 写し)で実装される
- 内部 mode が envelope 契約と orthogonal に隔離され、envelope consumer
  が内部経路の出力を誤認しない

## Non-goals

- envelope schema 自体の拡張・新 field 追加(移行後の別 spec 扱い)
- hook 4 target 出力・`usage.jsonl` record 形の byte 互換(別 test-spec
  `rust-migration-usage-log-hooks` が担う)
- differential golden の fixture 網羅・normalize 規則の定義自体
  (`rust-migration-differential-golden` が担う。本 spec は envelope 層の
  不変条件のみ)
- TS 版に存在する envelope 欠陥の修正(parity が目的であり改善はしない)

## Risk

- この保証が落ちると、親 agent が stdout を parse して得る
  result/meta/error が壊れ、委譲契約(envelope → hook → usage 記録)が
  連鎖的に破壊される。数値表現の差異は比較・集計の silent な誤動作
  として残存し、panic 経路の envelope 無し終了は呼び出し側を「応答
  無し・理由不明」で停止させる

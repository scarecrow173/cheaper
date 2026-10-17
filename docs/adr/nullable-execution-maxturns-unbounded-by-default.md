---
id: "ADR-i7la5qAvghCKzWea1fkZky"
type: "adr"
status: "proposed"
title: "execution.maxTurns を nullable にし既定を上限なしとする"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
supersedes: []
superseded-by: []
derives-from:
  - ../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md
  - ../specs/worker-run-nullable-maxturns-and-stall-timeout.md
implements: []
derived-by:
  - ../designs/worker-run-nullable-maxturns-and-stall-timeout.md
related:
  - ../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md
  - ../specs/worker-run-nullable-maxturns-and-stall-timeout.md
  - ../adr/execution-timeout-as-progress-reset-stall-window.md
  - ../adr/budget-exhaustion-checkpoint-and-resume.md
  - ../designs/worker-run-nullable-maxturns-and-stall-timeout.md
---

## Context

`cheaper bulk-read` が low-tier model では `MAX_TURNS_EXCEEDED` で結果を
返さない(Issue #111 様式 A)。`execution.maxTurns` の既定は 8 で、
tool を使う turn がそのまま枯渇すると「回答 turn」に到達する前に
abort される。issue は「回答用 turn の予約」を提案した。

初稿の ADR では「turn 枯渇時に tool admission を閉じて回答専用の
finalization turn を与える」機構(tool gate close)を採る判断としたが、
ユーザー決定により「`maxTurns` を nullable にし既定を上限なしとする」
方へ置き換えた(2026-10-17)。

### Forces

- model は「tool call を出さない turn」でのみ最終 JSON を返せるため、
  turn 予算が「最後の 1 turn は探索に使えない」意味を持つ — だが
  その残予算情報は model に一切渡されていない(discovery F-A)。
- 8 turn は bulk-reader が grep→narrow read を繰り返す現実的な探索量を
  下回り得る。探索量は question と repo 形状に依存し、turn 数での
  一律制限は入力に依らず静的である。
- それでも turn 予算を廃止するわけではない — bounded budget が必要な
  consumer は `maxTurns` を明示設定でき、既定だけを無制限へ動かす
  方が consumer 影響を限定できる。
- runaway 抑止は turn 数だけではない: `maxToolCalls`(tool 反復)、
  stall timeout(進捗停止 — 併走する timeout ADR が既定を引き上げる)、
  Rig 内部 turn guard(`DEFAULT_MAX_TURNS=1024`)が残る。

## Considered Options

### Option 1: `maxTurns` を nullable にし既定を上限なしにする(採用)

- `execution.maxTurns` は `integer(>=1)` または `null` を受理し、
  `null` は「上限なし」を意味する。builtin `policy.yaml` を
  `maxTurns: null` に変更し、`ExecutionPolicy.max_turns` を
  `Option<u64>` にする。
- `None` の場合 run 経路の maxTurns abort 監視は発火しない。
  `Some(n)` の場合の `MAX_TURNS_EXCEEDED` 分類・`details.reason`・
  pending batch drain の規則は従来どおり。
- **長所**: turn 枯渇による無回答を構造的に除去(回答 turn の予約も
  tool gate も不要)。model への残予算通知も新規 runtime 機構も不要。
  bounded 運用は opt-in で残り、既存の semantics は bounded 設定時に
  限り完全互換。
- **短所**: 既定で turn 数の無制限を許すため runaway 抑止を
  stall timeout / `maxToolCalls` / 内部 guard へ移す — delta を
  垂れ流す病的 run に対する時間上限は失われる(stall window では
  検出不可、spec Open Items に残リスクとして記録)。

### Option 2: tool gate close で finalization turn を予約(不採用)

- `turns >= maxTurns` で tool gate を閉じ、denial result で「最終
  JSON を返せ」と促して 1 回答 turn を保証する。初稿で採用した案。
- **長所**: turn 予算自体は維持したまま回答機会だけ保証できる。
- **短所**: tool admission・turn 監視・denial 文言の新規機構が要る;
  denial message が model の行動に依存して失敗し得る;「予算を使い
  切った後に 1 turn だけ特別扱い」するより「turn 上限自体を opt-in
  にする」方が概念が単純。ユーザー決定により不採用。

### Option 3: `maxTurns` 既定値を引き上げるだけ(不採用)

- `maxTurns: 8` → 20 等への据え置き + 上界引き上げだけでは根本解決
  にならない(より大きな repo/質問で同じ枯渇が起きる)。一律の上界が
  問題の本質であり、nullable 化がその上を行く。

## Decision Outcome

`execution.maxTurns` を nullable にし、builtin 既定を `null`
(上限なし)にする。schema・`ExecutionPolicy.max_turns`・run 監視・
`SessionSpec.policy_max_turns`・repair 経路の同 field を一貫して
`Option<u64>` に揃える。整数設定時の `MAX_TURNS_EXCEEDED`
分類は変更しない。

- stall timeout(別 ADR)と合わせて「tool 反復は `maxToolCalls`、
  進捗停止は stall window、turn 数は任意で bounded」と役割分担する。
- Rig 内部 guard(`DEFAULT_MAX_TURNS=1024`)は `maxTurns` 未指定時も
  有効な backstop として残る — 到達時は `MAX_TURNS_EXCEEDED` ではなく
  内部エラーとして失敗する(contract ではない)。
- consumer 互換性: 既存の `maxTurns` 整数設定は挙動不変。既定変更の
  影響は「明示しなかった consumer」のみ。

## Consequences

- Issue #111 様式 A(turn 枯渇)は既定値では構造的に発生しない。
- `maxTurns: null` の場合 run 時間の実効上限は stall window +
  `maxToolCalls` + tool 実行側上限に移る — delta 垂れ流し型の
  病理的 run は stall window で検出できず、別途 `maxDuration` 相当の
  総時間 ceiling が要る場合は Open Items に記録。
- `MAX_TURNS_EXCEEDED` は引き続き存在するが、`maxTurns` を明示設定
  した consumer のみが発火対象になる。
- `SessionSpec.policy_max_turns` と repair 経路の同 field が
  `Option<u64>` になり、factory の `max(internal, policy)` 計算は
  `None` → 内部 guard のみとして扱う。

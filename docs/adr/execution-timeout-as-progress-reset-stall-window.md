---
id: "ADR-gEWi2IvlRq907TdUn24dHp"
type: "adr"
status: "accepted"
title: "Treat execution.timeout as a progress-reset stall window and raise the builtin default"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/111"
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
  related:
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# Treat execution.timeout as a progress-reset stall window and raise the builtin default

## Context and Problem Statement

Issue #111 様式 B。`execution.timeout`(既定 120s)は run 開始からの絶対期限で、
`boundary.rs` が `tokio::time::sleep(timeout_ms)` を一度だけ arm する。
進捗(tool call 完了・delta・turn 進行)では延長されない。Devin CLI 委譲の
completion call は 1 回が数十秒級で非 streaming ゆえ、小規模 task でも
`turns=1`・`toolCalls=2` で 122s に打ち切られる観測と整合する
(discovery F-B)。

## Decision Drivers

* SPEC-onzPU8E4tKg3C6Iv4s9VK0 R2 — 進捗中の run を固定期限で殺さない。
* delegated provider(devin)の completion call は 1 呼出し単位で遅く、
  「イベント間の間隔」ではなく「run 全体時間」で測る現行方式と相性が悪い。
* runaway の総量上限は `maxTurns`/`maxToolCalls` が既に担う —
  絶対時間を唯一の runaway 対策にしているわけではない。
* envelope/error code(`TIMEOUT`・`details.reason="timeout"`)は不変に保つ。

## Considered Options

* **Option A(採用)**: `execution.timeout` を「直近の進捗 event からの
  stall window」に再意味付けし、`RunEvent` 観測毎に期限を延長する。
  併せて builtin 既定を 120s → 300s に引き上げる。
* **Option B**: 絶対期限のまま既定値だけ引き上げる — 却下: 「規定値の
  引き上げ」だけでは進捗中の run が期限超過で殺される構造は残り、
  issue 提案の「進捗がある間は延長」に応えられない。
* **Option C**: `execution.timeout`(絶対)+ `execution.idleTimeout`(進捗
  reset)の 2 knob — 却下: policy schema 拡張と 2 タイマー管理で、
  runaway が既に turn/tool 予算で有界である現状に対して過剰(YAGNI)。
  絶対 ceiling の需要が実測された時点で別 knob として追加する余地を
  Open Items に残す。

## Decision Outcome

Option A を採用する。

1. 「timeout = 進捗の停止検知」は abort の責務(立ち往生を殺す)に忠実で、
   正常に動き続ける run を殺さない。turn 数 × stall window で総時間は
   依然として有界。
2. `RunEvent` は stream 駆動経路で既に観測点が確立しており(on_event)、
   同じ hook 点で deadline を reset できる。repair session の event も
   進捗として扱い、修復中の誤 stall を防ぐ。
3. 既定値 300s は devin 委譲の非 streaming call が実用上収まる窓として、
   ollama e2e の 10m と現行 120s の中間で選択する(config override で
   さらに延長可能)。

## Consequences

* 良い結果: 進捗し続ける run はどれだけ長くても TIMEOUT にならず、
  issue 様式 B(活動中に固定期限で死亡)が解消される。config key は増えない。
* 負の結果: `execution.timeout` を「run の絶対上限」として運用していた
  consumer には意味論変更(互換性影響)。delta を垂れ流し続ける病的 run は
  turn/tool 予算以外の時間上限を持たなくなる(残リスクとして Open Item)。
* **意味の分岐**: `cheaper workflow run` は同じ `execution.timeout_ms` を
  workflow 全体の絶対期限として使い続ける(`engine.rs` の `spawn_deadline`
  は Instant 起点・reset なし — workflow 全体の ceiling として残す)。
  run 経路のみ stall 化するため、同じ policy field が caller で意味を
  分岐させる。configuration guide で「run 経路は stall window、
  workflow 経路は絶対期限」と明記する。
* 実装面: `boundary.rs` の固定 sleep を「event 毎に延長される deadline」
  実装へ置き換え、run 経路(main + repair)の event 観測点から
  progress を通知する。error 分類・envelope・`details.reason` は不変。

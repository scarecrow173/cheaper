---
id: "SPEC-onzPU8E4tKg3C6Iv4s9VK0"
type: "spec"
status: "proposed"
title: "execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume"
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
    - "../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../designs/worker-run-nullable-maxturns-and-stall-timeout.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
  derived-by:
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
---

# execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume

## 背景と目的

Issue #111。consumer(Windows / Devin CLI)から `cheaper bulk-read` を低 tier
model(`devin/swe-2`)で繰り返すと、高確率で結果を返さず終了する。観測された
2 様式:

- **様式 A — MAX_TURNS_EXCEEDED**: worker が `execution.maxTurns`(既定 8)
  の全 turn を read/grep 探索に消費し、回答 JSON を出す turn が残らない。
  機構上「回答は tool call を出さない turn」としてしか存在できず、
  model には残 turn 予算の情報も与えられていない(discovery F-A)。
- **様式 B — TIMEOUT(早期停滞)**: `turns=1`・`toolCalls=2` の小規模 run が
  122s で `TIMEOUT` 打ち切り。`execution.timeout`(既定 120s)は run 開始からの
  **絶対期限**であり、tool call 完了などの進捗で延長されない(discovery
  F-B)。Devin CLI 委譲の completion call は 1 回が数十秒級で、120s は
  実質 2-3 turn 分しか許容しない。
- 併せて `transientRetries: 0` が「retry されない失敗」と誤読されているが、
  transient retry は completion-call 単位の provider transient 失敗
  (429/5xx/transport)にのみ適用され、run 集約 abort(timeout 等)は
  仕様上対象外(discovery F-C)。consumer 向け文書にこの適用範囲が書かれて
  いない。

目的は (a) `execution.maxTurns` を nullable にして既定値を「上限なし」とし、
turn 枯渇による無回答を構造的に除去し、(b) 進捗中の run が固定期限で
殺されないよう timeout 意味論を見直し、(c) `transientRetries` の適用範囲を
文書で明確化すること。

**方針変更の記録**: 初稿では「turn 枯渇時に tool gate を閉じて回答専用の
finalization turn を与える」機構を採る設計だったが、ユーザー決定により
「`maxTurns: null`(上限なし)を許容し既定を上限なしへ変更」へ置き換えた
(2026-10-17)。bounded turn budget が必要な consumer は `maxTurns` に整数を
設定すれば従来どおり `MAX_TURNS_EXCEEDED` で打ち切られる。

## スコープ

**対象:**

- `execution.maxTurns` の nullable 化 — policy schema が `null` を受理し、
  `null` は「turn 数の上限なし」を意味する。builtin 既定を上限なしへ
  変更する。整数が設定された場合の `MAX_TURNS_EXCEEDED` abort 分類は
  従来どおり維持する。
- `execution.timeout` の意味論変更 — 絶対期限から「進捗イベント毎に
  reset される stall window」へ。あわせて builtin 既定値を delegated
  provider の非 streaming completion call に耐える値へ引き上げる。
- budget 枯渇(`MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED`)時の
  checkpoint 保存と `--resume` 再開 — `maxTurns` を明示設定した
  consumer が依然 turn 枯渇する経路への対策(途中結果を保存し、
  再開可能な状態を返却する、ユーザー決定 2026-10-17)。
- `retry.transientAttempts` の適用範囲(provider transient のみ、
  run-level abort は対象外)の文書明確化 — `docs/guides/configuration.md`
  および `cheaper-usage` skill。

**除外:**

- 「途中まで読んだ内容」の `result` field 直接返却(issue 提案 4 の
  partial-JSON 形)— worker output schema の必須 field を満たせず
  contract 破壊になる。R4 の checkpoint/resume で「途中結果の保存と
  再開」はカバーするが、partial result 自体は Open Items に記録。
- per-worker execution 制限 override 機構(既存 spec で除外済みの方針を維持)。
- repair session への finalization(repair は tool-less で該当しない)。
- envelope `schemaVersion:1`・error code 語彙・`meta` shape の変更。

## 要件

### R1: `execution.maxTurns` の nullable 化

1. policy schema(`execution.maxTurns`)は `integer(>=1)` または `null` を
   受理する。`null` は「turn 数の上限なし」を意味し、builtin
   `policy.yaml` の既定を `maxTurns: null` とする。
2. `ExecutionPolicy.max_turns` を `Option<u64>` にする。`None` の場合
   run 経路の maxTurns abort 監視(`turns >= maxTurns &&
   last_turn_tool_calls`)は発火せず、worker は回答 turn に到達するまで
   実行できる。`Some(n)` の場合の abort 分類・`details.reason`・
   pending batch drain の規則は従来どおり。
3. `null` でも runaway は別境界で有界に保つ: `maxToolCalls`(tool 反復)、
   stall timeout(進捗停止、R2)、Rig 内部 turn guard(`maxTurns` 未指定でも
   `DEFAULT_MAX_TURNS=1024` が残る)が引き続き有効。内部 guard 到達は
   `MAX_TURNS_EXCEEDED` ではなく内部エラーとして失敗する(backstop で
   あり contract ではない — guard breach は通常到達不能で、到達した
   場合は runaway 症状を示すため `MAX_TURNS_EXCEEDED` へ写像するより
   `INTERNAL_ERROR` の方が原因調査に適切)。adapter の
   `MaxTurnsError` message は「policy abort が先に発火するはず」という
   現行の invariant 文言を修正する(nullable 既定では到達可能な
   backstop として正確に記述する)。
4. `SessionSpec.policy_max_turns`・repair 経路の同 field も `Option<u64>`
   に揃え、factory の内部 guard 計算(`max(internal, policy)`)は
   `None` → 内部 guard のみとする。

### R2: `execution.timeout` の stall 意味論化

1. `execution.timeout` は「run 開始からの絶対期限」から「**直近の進捗
   event からの stall window**」に意味を変更する。`RunEvent` が観測される
   毎に期限が `timeout` 分延長される(text/reasoning delta・tool call/
   result・completion call・turn retry 等、stream 上の全 event)。
2. 進捗が `timeout` 期間途絶えた時点で従来どおり
   `RunAbortReason::Timeout` が発火し `TIMEOUT` に分類される。
   envelope・`details.reason="timeout"`・exit code は不変。
3. builtin `policy.yaml` の `execution.timeout` 既定値を delegated
   provider の非 streaming completion call が実用上収まる値へ引き上げる
   (120s → 300s)。stall window は「進捗の最大間隔」を縛るものであり
   run の総時間上限ではない(総量は `maxToolCalls`・turn guard・
   tool 実行側の各種上限が担う — delta を垂れ流す病的 call は
   stall window では検出できない残リスクとして Open Items に記録)。
4. repair session の event も progress として扱う(repair 中に stall 判定が
   誤発火しないこと)。

### R3: `transientRetries` 適用範囲の文書化

1. `docs/guides/configuration.md` の `retry.transientAttempts` 説明に
   「provider の transient 失敗(429/5xx/応答なし transport)のみに適用。
   run-level abort(TIMEOUT/MAX_TURNS_EXCEEDED/ABORTED)は retry しない」
   旨を明記する。
2. `cheaper-usage` skill の error/exit 説明に同旨の一文を追加する。

### R4: budget 枯渇時の checkpoint 保存と resume

`maxTurns` を明示設定した consumer は依然として turn 枯渇で
`MAX_TURNS_EXCEEDED` になる。その場合「途中結果を保存し、再開可能な
状態を返却する」(ユーザー決定 2026-10-17)。

1. `RunAbortReason::MaxTurns` または `MaxToolCalls` で abort する run は、
   error を返す前に checkpoint JSON を `<userConfigDir>/checkpoints/`
   へ atomic write する(`unix_mode: 0o600` — 観測に file 内容や
   command argv が入りうるため既存 credential/policy file と同じ権限)。
   `Timeout`/`Signal` は対象外。対象は `cheaper run` 経路のみ —
   `workflow run` 内部の job abort では保存しない(workflow の
   worker error details は envelope に乗らないため)。
2. checkpoint format(`schemaVersion: 1`): worker name・input・
   abortReason・counters(`turns`/`toolCalls`/`rejectedToolCalls`)・
   観測列。観測は `RunEvent::ToolCommitted`(hook patch 後の実実行
   args)と `ToolResult` を `call_id` で join して作る —
   `ModelTurnRetried` で rolled back した call・admission 拒否の
   error result は観測に含めない。args/resultExcerpt は各 entry cap
   付きで、観測列全体にも合計 byte cap(例: 100KB)と truncation
   marker を設ける — resume 時の注入 prompt が低 tier model の
   context を溢れないため。
3. abort error の `details` に `checkpoint: {path, observationsCount}`
   を追加する — **envelope 最外層の error**(mutation あれば
   `PARTIAL_MUTATION` wrapper、無ければ abort error 本体)の
   `details` に付ける。cause 側の details は envelope serialization で
   落ちるため、内側への付与では consumer に到達しない。
   `reason`/`changes` 等の既存 key は不変。details には path と件数のみ
   載せ、観測配列本体は file のみ(巨大 envelope 化を防ぐ)。
4. `cheaper run <worker> --resume <path>`(worker alias 含む)で
   checkpoint を load し、worker/input を復元して新規 run として
   実行する。`checkpoint.worker` は positional `<worker>` と一致必須
   (`--worker-file` 使用時は resolved `def.name` と比較)、不一致・
   不正 checkpoint は `INVALID_INPUT`。input 解決は `--input` 明示
   (`option_tokens` の presence 検知 — `--input -` の stdin 明示も
   明示扱い)> checkpoint の stored input の順。run の user prompt
   末尾に「前回は budget 枯渇で中断。収集済み観測は以下 — 続きから
   回答 JSON を出せ」趣旨の continuation section を注入する。
   `--input` で input を変えても前回観測は注入される(観測は旧 input
   由来 — stale context である旨を docs に明記)。`--resume` は
   `run`/worker alias 以外のコマンドでは reject する
   (strict-parse 規約に従う)。resume run では invocation `--grant` は
   引き継がれない(checkpoint に保持しない — authorization の永続化を
   避ける)。再付与が必要なら caller が再度 `--grant` を渡す。
5. resume run がさらに budget 枯渇した場合、新 checkpoint は前回
   観測 + 新規観測を累積して保存する(多段 resume が成り立つ。
   累積列にも同じ合計 byte cap を適用する)。checkpoint file の
   永続 retention は設けず、`<userConfigDir>/checkpoints/` 配下の
   掃除方針(TTL/count cap)は Open Items に記録する。

## Acceptance Criteria

- [ ] `maxTurns: null` の policy が load・normalize され、turn 数が多い run
      が `MAX_TURNS_EXCEEDED` にならず回答 turn まで完走する dev-seam
      テストが存在する
- [ ] `maxTurns` に整数を設定した run は従来どおり `turns >= maxTurns &&
      last_turn_tool_calls` で `MAX_TURNS_EXCEEDED`・
      `details.reason="maxTurns"` になる(既存 regression を維持)
- [ ] builtin policy の `execution.maxTurns` が `null` で、
      `execution.timeout` が新既定値(300s)である
- [ ] 進捗 event が `timeout` 間隔内に継続する run は `timeout` の絶対経過で
      abort されず、event 停止から `timeout` 経過で `TIMEOUT` になる
      dev-seam テストが存在する
- [ ] `configuration.md` と `cheaper-usage` skill に transient retry の
      適用範囲と `maxTurns: null`・stall 意味論が記載されている
- [ ] `maxTurns` 設定済み run が budget 枯渇した場合、
      `<userConfigDir>/checkpoints/<id>.json` に checkpoint が保存され、
      **envelope 最外層** error の `details.checkpoint.{path,
      observationsCount}` が返る(mutation 経路では `PARTIAL_MUTATION`
      error の details に付く)
- [ ] `cheaper run <worker> --resume <path>` で checkpoint の観測が
      user prompt に注入され、新規 run が完走する dev-seam テストが
      存在する
- [ ] `--resume` の CLI 面(flag parse・checkpoint load/validate・
      worker 不一致 `INVALID_INPUT`・`--input` presence・非 run
      command での reject)を pin する integration test が存在する
- [ ] `run-max-turns`/`run-max-tool-calls` regression fixture が
      `details.checkpoint` を含む形で再 capture され、
      `normalize.rs` に checkpoint path の scrub rule が入る
- [ ] `mise run check` が green

## Open Items

- **部分結果の `result` 直接返却**(issue 提案 4): R4 は checkpoint +
  resume で「途中結果の保存」を実現するが、`result` field への
  partial JSON 埋め込み(schema-validated)は行わない — worker output
  schema の必須 field を満たせないため別 issue 候補のまま。
- stall window で「進捗し続けるが意味的に停滞する run」(delta を垂れ流す
  call)は stall 検知できない — turn 単位では内部 guard(1024)・tool
  反復は `maxToolCalls` が残るが、単一 completion call が delta を出し
  続ける病理的経路は時間上限を持たない。絶対時間 ceiling が要る運用が
  出た場合は `execution.maxDuration` 等の別 knob として検討。
- `workflow run` の `execution.timeout` 適用面は workflow 全体の絶対期限
  (engine の `spawn_deadline`)として現行のまま残す — 同じ policy field が
  run 経路(stall window)と workflow 経路(absolute deadline)で意味を
  分岐させるため、configuration guide で明示する。workflow 側の
  stall 化は需要があれば別件で扱う。
- timeout/signal abort の checkpoint 保存は対象外 — 「続きから再開」の
  意味が薄い。需要が実測されたら拡張を検討。
- checkpoint file の retention: `<userConfigDir>/checkpoints/` 配下に
  1 実行 1 file で蓄積する(TTL/count cap は設けない)。消去方針は
  需要が実測されたら別件。

---
id: "DISC-x1vmG1ECubKZNQ0QMyUyi5"
type: "discovery"
status: "resolved"
title: "bulk-reader low-tier 無回答: turn 枯渇と固定 timeout の構造分析"
created: "2026-10-17"
updated: "2026-10-17"
owners: ["scarecrow173"]
relations:
  source:
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-runtime/src/run/boundary.rs"
    - "crates/cheaper-runtime/src/run/abort.rs"
    - "crates/cheaper-runtime/src/run/repair.rs"
    - "crates/cheaper-runtime/src/run/session.rs"
    - "crates/cheaper-rig-adapter/src/agent.rs"
    - "crates/cheaper-rig-adapter/src/hooks.rs"
    - "crates/cheaper-rig-adapter/src/retry.rs"
    - "crates/cheaper-tools/src/accounting.rs"
    - "crates/cheaper-cli/resources/policy.yaml"
    - "crates/cheaper-cli/resources/workers/bulk-reader.md"
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
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../designs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# bulk-reader low-tier 無回答: turn 枯渇と固定 timeout の構造分析

issue #111。consumer(Windows / Devin CLI)から `cheaper bulk-read` を繰り返すと
低 tier model(`devin/swe-2` tier=low)で結果を返さず終了する、という報告の
コード調査記録。

## Exploration Goal

2 つの失敗様式(MAX_TURNS_EXCEEDED / TIMEOUT)の根本原因を機構レベルで特定し、
issue の提案 4 項目(turn 予約・timeout 延長・transientRetries 明確化・
部分結果)を「実現可能な機構」へ写像する。答え合わせではなく、どの境界で
直すべきかを確定するのが目的。

## Context and Constraints

- 外部 contract: envelope `schemaVersion:1`、exit code 表、error code 語彙
  (TIMEOUT/MAX_TURNS_EXCEEDED/...)、`meta` shape は既存 consumer が依存。
- TS parity の暗黙契約: abort 分類・reason・計上点は `runtime-pi` 由来の
  規則に沿う。振る舞い変更は envelope 値を変えない範囲で。
- builtin `policy.yaml` の既定値: `execution.timeout=120s`・`maxTurns=8`・
  `maxToolCalls=64`・`output.maxTokens=2000`・`output.repairAttempts=1`・
  `retry.transientAttempts=1`。policy は consumer が config で override 可能。

## Observed Mechanism(事実確定)

### F-A: MAX_TURNS_EXCEEDED の発火条件

`crates/cheaper-runtime/src/run/mod.rs` `drive_prompt` の on_event 監視:

- `snap.turns >= maxTurns && snap.last_turn_tool_calls && pending_tool_calls == 0`
  で `RunAbortReason::MaxTurns` を request。stream は drop され run は
  `MAX_TURNS_EXCEEDED`(exit 相当)で終了する。
- turn は `ModelTurnFinished` 単位で計上(AccountingHook)。**最終回答は
  「tool call を出さない turn」としてしか存在できない** — model が
  8 turn をすべて tool 継続で使い切ると、回答を生成する機械経路が
  物理的に存在しない。
- system prompt(build_system_prompt)には「final message は schema 適合の
  JSON 1 object」という出力 contract と batching guidance はあるが、
  **残 turn 予算の情報は model に一切与えられていない**。低 tier model が
  「最終 turn は回答に使う」制御を自律できる前提が組み込まれていない。

### F-B: TIMEOUT の発火機構

`boundary.rs::run_worker_inner` は `tokio::time::sleep(timeout_ms)` を
spawn し、**run 開始からの絶対期限**として `RunAbortReason::Timeout` を
発火させる。進捗(tool call 完了・token delta・turn 進行)で期限は
一切延長されない。120s 既定は「Devin CLI 委譲の completion call 1 回が
数十秒」という現実(devin provider = external CLI delegation)に対して
2-3 turn 分しか許容しない。issue の様式 B(turns=1・toolCalls=2 で
122.2s 打ち切り)は、turn 2 の completion call が応答を返す前に絶対期限が
落ちた形跡と整合する。

### F-C: `transientRetries: 0` は仕様通り

`retry.transientAttempts` は `SessionSpec.transient_retries` として
`RetryingCompletion` handler へ渡り、**completion-call 単位の provider
transient 失敗(HTTP 429/5xx・応答なし transport 失敗)のみ**を retry する
(`cheaper-rig-adapter/src/retry.rs`・`error.rs` の `ErrorClass` 分類)。
run 集約 abort(timeout/signal/maxTurns/maxToolCalls)は provider error
ではないため retry 対象外であり、issue で観測された `transientRetries: 0`
は仕様どおりの値。「timeout は retry されない」は事実として正しいが、
consumer 向け文書(configuration guide / cheaper-usage skill)には
この適用範囲が明記されていない。

### F-D: 部分結果・合成経路の不在

- `repair_output`(repair.rs)は「tool-less の新規 session + corrective
  prompt」という 2nd-session パターンを実装済みで、usage/turns は main run
  に畳み込まれる。**abort 後の類似経路は存在しない** — abort 分類が
  先勝ちで確定し、回収した tool 観測を別 session で総合する機構は無い。
- transcript/transcript の保持も無い: `ToolDetails` は call id →
  details JSON(host metadata)のみで tool 出力本文は保持しない。
  `RunSnapshot` は counter のみ。abort 時点で「何を読んだか」を
  合成 session へ渡す素材が存在しない。
- `RunEvent::ToolResult` は stream 上に流れるため、run 側で output を
  バッファする拡張は原理的に可能(ただし新しい保持面の追加)。

## Issue 提案の実現性写像

| 提案 | 機構としての評価 |
| --- | --- |
| 1. 最終回答用に 1 turn 予約 | **実現可能** — Rig の `agent.prompt` は単発で mid-run の tool 除去や message 注入が無いため「予約」の形は取れないが、turn 枯渇検知時に **tool gate を閉じる**(tool call をすべて denial で返す)ことで model に「回答しかできない turn」を与える形は実装できる。`ToolBudgetRunner.run` の admission に共有 flag を足す程度の面積。denial は `rejectedToolCalls` に計上され model には error result が返る。denial 後も tool call を続ける場合は bounded grace(例: +1 turn)で abort。 |
| 2. TIMEOUT 延長/進捗延長 | **実現可能・ただし意味論選択が要る**。現行は絶対期限。選択肢: (a) 既定値を引き上げるだけ(最も小さい変更)、(b) RunEvent 進捗で期限を延長する stall-timeout 意味論への変更(issue 提案どおりだが semantics 変更)、(c) 絶対期限 + stall 延長の 2 knob 化。devin provider が非 streaming なら単一 completion call が window を超え得るため (b) だけでは不十分な可能性。 |
| 3. transientRetries 条件の明確化 | **文書対応** — 機構上の欠陥ではない。`transientRetries` の適用範囲(provider transient のみ・run abort は対象外)を guide/skill に明記。 |
| 4. 部分結果を返す選択肢 | **大きい**。tool 観測の transcript 保持 + abort 時の合成 session(or envelope への partial 領域追加)が必要。envelope `result` は schema-validated 必須なため「部分的に読んだ内容」を型整合で返すには新しい contract が要る。本 issue では 1-2 の機構で「回答まで到達する率」を上げるのが先。部分結果自体は別 issue 級の面積として切り分けるのが妥当。 |

## Alternatives and Comparison

### MAX_TURNS 対策

|| Option | Pros | Cons |
|| --- | --- | --- |
|| **A-1: turn 枯渇で tool gate を閉じ、bounded finalization turn を許す** | 機構的に保証される「回答用 turn」。model の自律に依存しない。denial message で「もう tool は使えない、今の情報で答えよ」を伝えられる | ToolRuntime/budget に動的 close の延長が要る。denial を受けても答えない model には効かない(その場合 abort) |
|| A-2: system prompt に残 turn 予算の指導を追加するのみ | 変更最小 | 低 tier model が守る保証がない — issue の根本原因である「制御の不在」を解消しない |
|| A-3: bulk-reader の maxTurns を上げる(per-worker override) | 即効 | per-worker override 機構自体が未存在(strict `cheaper` keys 拡張 = core 変更)。枯渇構造は残るので発火率を下げるだけ |
|| **A-4: `execution.maxTurns` を nullable にし既定を上限なしにする** | 「回答 turn 予約」も tool gate も不要で最も単純。bounded 運用は整数を明示する opt-in で残る。model への残予算通知機構が不要 | 既定で turn 上限が無くなるため runaway 抑止は stall timeout・`maxToolCalls`・内部 guard(1024)に移る。delta 垂れ流し型の病理的 run への時間上限は失われる |

### TIMEOUT 対策

|| Option | Pros | Cons |
|| --- | --- | --- |
|| **B-1: `execution.timeout` を「進捗で reset される stall timeout」に再意味付け** | issue 提案どおり。devin 委譲の非 streaming call が長い問題にも対処できる(イベント到着毎に延長) | semantics 変更(ADR 必須)。無限に進捗し続ける runaway の絶対上限が消える → 別途 ceiling が要るかの判断が付随 |
|| B-2: 絶対期限のまま既定値を引き上げ(例 120s→300s) | 最小差分 | 「規定値の恣意的引き上げ」で構造は残る。遅い provider では依然不足し得る |
|| B-3: `execution.timeout`(絶対)+ `execution.idleTimeout`(進捗 reset)の 2 knob | 意味論が明確 | policy schema 拡張と 2 タイマー管理。当面の需要に対して過剰寄り |

## Tentative Conclusions

- **C1**: MAX_TURNS の根本原因は「回答用 turn を保証する機構が存在しない」こと。
  初稿では A-1(tool gate close + bounded finalization)を採ったが、
  ユーザー決定(2026-10-17)により **A-4(`maxTurns` nullable 化・既定
  上限なし)** を採用する。bounded 運用は opt-in で残る。
- **C2**: TIMEOUT の根本原因は「120s 絶対期限が delegated provider の
  completion 粒度に合わない」こと。B-1(stall 意味論への変更)を検討対象とし、
  ceiling の扱いは spec/ADR で確定する。
- **C3**: `transientRetries` は仕様どおり動いているが、適用範囲の文書化が
  不足している。doc 追随で対応。
- **C4**: 部分結果は transcript 保持 + 合成 contract が必要で、当初は
  別 issue 級と評価したが、ユーザー決定(2026-10-17)により「途中結果を
  checkpoint として保存し resume 可能にする」形で本件スコープ化した
  — `result` field への partial JSON 埋め込みではなく、budget 枯渇時に
  `<userConfigDir>/checkpoints/` へ観測列を保存し `--resume` で新規
  run へ注入する contract(ADR-dQUW…)。`result` 直接返却は Open Item
  のまま別件。

## Open Questions

- `maxTurns: null` 時の runaway 抑止分担: tool 反復は `maxToolCalls`、
  進捗停止は stall timeout、turn 数は Rig 内部 guard(1024)が backstop。
  内部 guard 到達時の error 分類は `MAX_TURNS_EXCEEDED` ではないことを
  spec で確定した。
- stall-timeout 意味論の場合、`RunEvent` のどの種別を「進捗」に数えるか
  (TextDelta/ReasoningDelta まで含めるか、tool 系のみか)。
- `workflow run` の `execution.timeout` は workflow 全体の絶対期限として
  維持する — run 経路と意味を分岐させることを configuration guide で
  明記する。

## Promotion Candidates

- [x] spec-doc needed: `maxTurns` nullable 化・timeout 意味論・
  transientRetries 文書化・out-of-scope(部分結果)の確定
- [x] adr-doc needed: (1) 絶対期限 → stall/進捗延長への semantics 変更、
  (2) `maxTurns` nullable 化・既定無制限の方式選択
  (初稿は tool gate close 案 → ユーザー決定で nullable 化へ置換)

## Sources and Relations

- `crates/cheaper-runtime/src/run/{mod,boundary,abort,repair,session}.rs` —
  abort 集約・timeout arming・maxTurns 監視・repair 2nd-session パターン。
- `crates/cheaper-rig-adapter/src/{agent,hooks,retry,run_stream}.rs` —
  transient retry の適用境界・turn 計上・ToolDetails の保持範囲。
- `crates/cheaper-tools/src/accounting.rs` — admission/budget の構造
  (`on_limit`・`rejectedToolCalls`・`limit_hit`)。
- `crates/cheaper-cli/resources/policy.yaml` — builtin 既定値。
- `crates/cheaper-cli/resources/workers/bulk-reader.md` — worker frontmatter。
- issue #111(2026-10-16 起票)の観測値: `turns=1`・`toolCalls=2`・
  `durationMs=122219`・`transientRetries=0`・tier=low。

---
id: "ADR-dQUWgKzTTSsUBIhYYpNAtC"
type: "adr"
status: "proposed"
title: "turn/tool 予算枯渇時に checkpoint を保存し resume 可能にする"
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
  - ../adr/nullable-execution-maxturns-unbounded-by-default.md
  - ../adr/execution-timeout-as-progress-reset-stall-window.md
  - ../designs/worker-run-nullable-maxturns-and-stall-timeout.md
---

## Context

`execution.maxTurns` は nullable 化で既定無制限になるが、consumer や親
エージェントが明示的に上限を設定する経路は残る(ADR-i7la…)。その場合
run は依然として `MAX_TURNS_EXCEEDED`(`maxToolCalls` でも
`MAX_TOOL_CALLS_EXCEEDED`)で無回答終了し、収集済みの tool 観測は
全て失われる。

issue #111 の提案 4「部分結果を返す」は transcript 保持 + output
contract 拡張が要るため当初は別件としたが、ユーザー決定(2026-10-17)
により「途中結果を保存し、再開可能な状態を返却する」形で本件
スコープに取り込む。部分 result を envelope `result` として返す
(schema-validated で「途中まで」の JSON は型不整合)のではなく、
**checkpoint として保存し、後続 run が続きから再開できる**形にする。

### Forces

- worker ごとの output schema 統一は困難 — 「部分的な bulk-reader
  result」は `findings`/`summary` の必須 field を満たせないことが多く、
  per-worker partial contract は worker 数 × schema の面積になる。
  checkpoint 形なら worker 非依存で一括適用できる。
- tool 観測は `RunEvent::ToolCall`/`ToolResult` で stream に既に流れて
  いる — run 経路で observation を capture する追加の保持面だけで
  実現できる。
- resume は「前回の観測を注入した新規 run」で足りる — Rig session の
  mid-flight 再開(serializable session state)は session internals の
  大型変更が要り、必要ではない。worker の tool は read/grep/ls 等の
  idempotent read が主体で、新規 run で同じ探索を再実行しても安全。
- checkpoint を親エージェント側で消費するには返却面が必要 —
  error `details` に checkpoint path を載せ、CLI flag で再開できる
  機械的な contract にする。

## Considered Options

### Option 1: budget 枯渇時に checkpoint file を保存し `--resume` で再開(採用)

- `MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED` で abort する run は、
  error を投げる前に checkpoint JSON を
  `<userConfigDir>/checkpoints/<id>.json` へ保存する(atomic write)。
- envelope error `details` に `checkpoint: {path, observations}` を
  追加する(新 field — 既存 `reason`/`changes` 等との併存)。
- `cheaper run <worker> --resume <path>`(worker alias も同様)で
  checkpoint を読み込み、worker/input を復元して新規 run を開始する。
  新規 run の user prompt 末尾に「前回は budget 枯渇で中断した。
  収集済み観測は以下 — 続きから回答 JSON を出せ」趣旨の
  continuation section を付加する。
- **長所**: worker/schema 非依存、envelope 後方互換(`details` への
  追加のみ)、観測の重複実行なし、親 agent が機械的に resume を
  扱える。
- **短所**: checkpoint の capture/persist/resume の新規面積。
  prompt 注入で「前回観測」を渡す形のため model 品質に依存する。

### Option 2: `result` に partial output を直接返す(不採用)

- schema-validated `result` に「途中までの JSON」を載せる案。
- **短所**: worker output schema の必須 field(summary/findings 等)を
  満たせず、schema 緩和 or `result` の optional 化が必要 —
  envelope contract の破壊。面積が大きく issue #111 の範囲を超える。

### Option 3: checkpoint を transcript として再開時に replay する(不採用)

- checkpoint に tool call/result の全履歴を保存し、resume run で
  「同じ tool call を replay して model context を再構築」する案。
- **短所**: tool call が非決定的(同じ grep が同じ結果を返すとは
  限らない)かつ Rig session の内部状態と分離されるため context の
  再現性が取れない。prompt 注入の方が deterministic で単純。

## Decision Outcome

Option 1 を採用する。

1. run 経路の abort handler(`classified()`)で `MaxTurns`・
   `MaxToolCalls` の場合に限り checkpoint を保存する。
   `Timeout`/`Signal` は対象外(進捗停止・外部 cancel は「続きから
   再開」の意味が薄く、resume 誘導の正当性が弱い — 拡張余地は
   Open Items に記録)。workflow 内部の job abort も対象外
   (workflow error の details は envelope に乗らない構造のため)。
2. checkpoint format `schemaVersion:1` — worker name・input・
   abortReason・counters(turns/toolCalls/rejectedToolCalls)・
   観測列(`{tool, args, resultExcerpt}`、entry cap + 合計 byte cap +
   truncation marker 付き)。観測は `ToolCommitted`(hook patch 後の
   実実行 args)と `ToolResult` を `call_id` で join し、
   `ModelTurnRetried` で roll back された call・admission 拒否の
   error result は含めない。
3. `details.checkpoint` は **envelope 最外層の error** に付ける
   (`PARTIAL_MUTATION` wrap 時は wrapper の details。cause 側の
   details は envelope serialization で落ちるため)。値は
   `{path, observationsCount}` — 観測配列本体は file のみ。
4. `cheaper run <worker> --resume <path>` で checkpoint から worker・
   input・観測を復元し、user prompt に continuation section を注入して
   新規 run として実行する。`--input` 明示(presence 検知、値 `-` も
   含む)は checkpoint input を上書きする。`--resume` は run/alias
   以外のコマンドで reject する(strict-parse 規約)。
5. checkpoint の連鎖 — resume run がさらに budget 枯渇で abort した
   場合、新しい checkpoint は前回観測 + 新規観測を累積して保存する
   (合計 byte cap も維持 — resume → abort → resume の多段継続)。
6. checkpoint file は `unix_mode: 0o600` で atomic write する
   (観測に file 内容/command argv が入りうるため credential と同じ
   権限)。`<id>` は `fs_util::temp_path` 系の pid+nanos で生成し、
   新規依存を増やさない。
7. invocation `--grant` は checkpoint に保持しない — authorization の
   永続化を避ける。resume 時に grant が必要なら caller が再度
   `--grant` を渡す。

## Consequences

- bounded 運用の consumer は `MAX_TURNS_EXCEEDED` で「全部失う」ので
  はなく「途中経過を保存して続きから再開できる」— issue #111 の
  「結果を返さない」苦情を bounded 経路でも緩和する。
- envelope `details.checkpoint` は additive(既存 key 変更なし)で
  後方互換 — ただし付与点は「envelope 最外層の error」で、
  `PARTIAL_MUTATION` wrap 経路では wrapper の details に付く
  (cause 側 details は envelope に乗らない)。
- `<userConfigDir>/checkpoints/` は新規の write surface —
  usage.jsonl と同じ user config scope に置き、workspace を汚さない。
  file は `unix_mode: 0o600`、retention は設けない(Open Items)。
- timeout/signal 経路の resume は対象外(観測が少ない + 意味が薄い)。
  需要が実測されたら拡張を検討(Open Items)。
- repair 経路(tool-less session)は対象外 — main session の観測のみ
  capture 対象。
- regression fixture(`run-max-turns`/`run-max-tool-calls`)は
  `details.checkpoint` 追加で更新が要る — `normalize.rs` に run-id
  の path scrub rule を追加する(非決定性の解消)。

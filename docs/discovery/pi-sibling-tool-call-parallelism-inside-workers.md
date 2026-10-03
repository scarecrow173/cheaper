---
id: "DISC-034zzZxx7gzas0a5Anlkwi"
type: "discovery"
status: "resolved"
title: "Pi sibling tool-call parallelism inside Workers"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/55"
    - "../ideas/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
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
    - "../ideas/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
  derived-by:
    - "../specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
    - "../adr/pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/cheaper/issues/57"
    - "https://github.com/scarecrow173/cheaper/issues/62"
  defers: []
  deferred-by: []
---

# Pi sibling tool-call parallelism inside Workers

## Exploration Goal

Pi Worker が 1 assistant turn 内で複数の独立した tool call（sibling tool call）を
発行したとき、pinned Pi version が実際にそれらを並列実行するかを runtime test で
決定的に確認し、並列性を Worker の supported contract として活用する方法を探る。

trigger は GitHub issue #55。serial な ReAct 型の tool → model → tool → model
シーケンスは Worker turn 数と、ターンごとに増大するコンテキストの反復送信を増やし、
delegation のコスト削減効果を薄める。sibling tool-call 並列性は Worker 内
（intra-Worker）の並列性であり、#57 ToolScheduler（runtime 操作の並行度制限）や
#62 Workflow Engine（job 横断の並列化）とは別軸の話題として扱う。

コスト上の効果は間接的である点に注意: 並列化は token 単価を変えない。効果源は
Worker turn 数の削減と、増大コンテキストの反復送信の削減である。

## Key Issues and Assumptions

- **pinned version**: `@earendil-works/pi-coding-agent@0.85.1`
  （`packages/runtime-pi/package.json`）。agent loop は
  `@earendil-works/pi-agent-core@0.85.1` の `dist/agent-loop.js` にある。
- **Non-goals（issue #55 より）**: 独自 agent loop を作らない、operation 完了で
  model を起こす completion-driven wake（Unreal 式）を導入しない、daemon API を
  増やさない、並列 executor を重複実装しない、Workflow DAG scheduler は含まない。
- **cost-first 制約**: heartbeat や operation 完了通知が新たな model turn を
  生成してはならない。sibling tool result の返却は Pi 本来の tool-call barrier
  semantics に任せる。
- **contract 不変**: CLI / stdout envelope contract の変更は scope 外。
- **指示の所在**: Worker 指示は `packages/cli/workers/` の markdown
  （`worker.instructions`）と、`buildSystemPrompt`
  （`packages/runtime-pi/src/run.ts:111-125`）が末尾に付ける runtime 生成の
  `## Output contract` セクションの 2 層からなる。

## Alternatives and Comparison

### 論点 A: same-turn 並列呼び出し指示をどこに書くか

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| runtime 生成の `## Output contract` セクション（`buildSystemPrompt`）に追加 | 全 Worker に一律適用される。runtime と version が揃い、ドリフトしない。「runtime が timeout / process lifecycle を持つので polling 禁止」のような runtime 固有の事実と同居できる | Worker 個別の調整（例: write-heavy worker 向けの注意）には向かない | yes |
| 各 worker markdown の `instructions` に記述 | Worker ごとのチューニングが可能 | bulk-reader / code-writer / doc-writer で文言が重複し、表現ドリフトしやすい。project の「同一概念の表現は一箇所」原則に反する | maybe |
| ハイブリッド（runtime 生成で共通原則 + markdown で worker 固有の補足） | 原則の一元化と個別調整を両立 | 層の責務境界を明文化しないと再度ドリフトする。まず runtime 生成だけで要件を満たせるなら YAGNI | maybe |

### 論点 B: mutation 系 tool に `executionMode: "sequential"` を付けるか

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 指示ガイダンス + Pi 既存の `withFileMutationQueue` 同一ファイル直列化に依存 | ファイル横断の並列性を維持できる（異なるファイルへの write/edit は並列のまま）。実装変更がほぼ不要 | 指示に依存するため model が unsafe な mutation 重複を組む余地は残る（ただし同一ファイルは Pi が直列化する） | yes |
| cheaper の write/edit に `executionMode: "sequential"` を設定 | 強制力が強い | batch 内に mutation call が 1 つでもあると Pi の semantic 上 batch 全体が直列化される（`executeToolCallsParallel` に入らない）ため、read 群との混在 batch ですら並列化が潰れる。効果源を自ら狭める | no |

### 論点 C: contract の証明方法

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `WorkerSessionOptions.streamFn` seam を使う scripted fake-stream test（N 個の sibling tool call を持つ 1 assistant message を emit し、実行の overlap と call-order の result を assert） | 実 provider 不要・決定的・CI で回帰できる。既存の scripted fake-stream fixture（task `add-scripted-fake-stream-test-fixture`）を再利用できる | fake-stream の忠実度に依存するため、Pi upgrade 時の real 動作ずれは別途検知が要る | yes |
| 実 provider（Ollama）での smoke | 実挙動に近い | モデルが sibling call を発行するかを制御できず非決定的。CI では自動 skip になる | no |

## Tentative Conclusions and Hypotheses

### pinned Pi の挙動（node_modules 一次ソースを読んで確認済み）

一次ソース確認であり runtime 実測ではない。また `pi-agent-core` は
`pi-coding-agent` 経由の **transitive 依存**であり direct pin は
`pi-coding-agent@0.85.1` / `pi-ai@0.85.1` のみ
（`packages/runtime-pi/package.json`）。今日の version 固定は
`pnpm-lock.yaml` の解決結果に依存するため、Pi upgrade 時は以下の
semantics を再確認すること。

1. **`toolExecution` の既定は `"parallel"`**: `Agent` の default が
   `toolExecution: "parallel"`（`pi-agent-core` `dist/agent.js:134`）。cheaper の
   `createWorkerSession`（`packages/runtime-pi/src/session.ts`）は
   `createAgentSession` 経由で `new Agent`（pi-coding-agent
   `dist/core/sdk.js:177`）しており `toolExecution` を渡さないため、
   parallel default が既に適用されている。
2. **batch 単位の直列化判定**: `executeToolCalls`
   （`agent-loop.js:285-292`）は `config.toolExecution === "sequential"` または
   batch 内のいずれかの tool が `executionMode === "sequential"` を持つと
   batch 全体を直列実行し、それ以外は `executeToolCallsParallel` へ行く。
3. **parallel path の semantics**（`agent-loop.js:330-383`）:
   per-call preparation（`tool_execution_start` イベント、`prepareToolCall`、
   `beforeToolCall` hook）は call 順に serial に実行され、その後各実行が
   deferred thunk として `Promise.all` で同時に走る。`tool_result` message は
   全 settle 後に**元の call 順**で emit される（result 順序は安定）。
4. **`stopReason === "length"` の guard**: 切り詰められた assistant message
   由来の tool call は `failToolCallsFromTruncatedMessage` により sibling 全員が
   実行されず error result になる（truncated arguments の実行防止）。
5. **batch 途中の abort**: 未開始の deferred call は "Operation aborted"
   の error result を返し、開始済み call は abort signal を受け取る。
6. **terminate 判定**: `shouldTerminateToolBatch` は batch 内の全 result が
   `terminate === true` のときだけ batch を終了させる。
7. **同一ファイル mutation の直列化**: Pi built-in の write/edit tool は
   `withFileMutationQueue(absolutePath, ...)` で包まれている
   （`pi-coding-agent` `dist/core/tools/write.js:34`、`edit.js:96`）。
   同一ファイルへの mutation は直列化され、別ファイル同士は並列のまま。
   cheaper の write/edit は `createWriteToolDefinition` /
   `createEditToolDefinition` に boundary operations を注入する形
   （`packages/runtime-pi/src/tools/registry.ts:67-83`）なのでこの直列化を
   そのまま継承する。直列化の**適用順は決定的で call 順と一致する**:
   deferred thunk は call 順に同期的に起動され
   （`agent-loop.js:372`）、`withFileMutationQueue` の
   `registrationQueue` がその順序のまま per-key chain を組む
   （`file-mutation-queue.js`）。ただし queue key は
   `resolve()` + 存在時のみ `realpath` で正規化されるため、
   **存在しないファイルへの別 spelling**（symlinked dir 経由や
   Windows の case variant）は別 key になり同一対象を直列化しない
   余地がある（narrow edge）。read / grep / find / count_tokens は
   自前 tool（`registry.ts:64-65,84-89`）で queue の対象外であり、
   同一 batch 内の **read↔mutation の ordering は保証されない**。
8. **`executionMode` を持つ tool は現状ゼロ**: Pi built-in にも cheaper の
   sandboxed tool にも `executionMode` 設定がなく、今日は全 tool が
   parallel-eligible。`wrapToolBudget`
   （`packages/runtime-pi/src/tools/budget.ts`）の check-then-increment
   counter は同期処理なので JS の並行下で安全。`onLimit` は session abort を
   起こし、pending の sibling 実行を cancel する。
9. **Worker 指示の構成**: `packages/cli/workers/`（bulk-reader.md /
   code-writer.md / doc-writer.md）が `worker.instructions` を供給し、
   `buildSystemPrompt`（`packages/runtime-pi/src/run.ts:111-125`）が末尾に
   runtime 生成の `## Output contract`（JSON output schema・path 規則・
   available tools）を付ける。same-turn 並列ガイダンスの候補地はこの
   contract セクション。
10. **`output.maxTokens` cap と batching の相互作用**:
    `createWorkerSession` は `maxTokens = min(policy.output.maxTokens,
    model.maxTokens)` を**全 assistant turn**の stream call に注入する
    （`session.ts:87-90`）。現行 `policy.yaml` の `output.maxTokens:
    2000` は最終 JSON 1 個分のサイズであり、N 個の sibling call（特に
    write/edit の大きな引数）を含む turn は cap を超え得る。
    `stopReason: "length"` になると上記 4 の通り **sibling 全員が未実行
    error** になり、model が同じ batch を retry すると turn が
    `stopReason === "toolUse"` にならないため `maxTurns` には乗らず
    `timeout`（既定 120s）まで spin し得る。same-turn batching を促す
    指示と相まって生じる自己誘発の failure mode であり、指示側で
    「batch と各 call の引数を turn の出力上限に収まる粒度に保つ」
    ことを求める必要がある。

### 仮説

- (hypothesis) sibling tool-call 並列性の「新規実装」は不要で、要件は
  「pinned Pi の既存 parallel 実行を決定的 test で証明する」+「runtime 生成
  指示で model に same-turn 発行を促す」+「regression test と execution engine
  文書化」に分解できる。
- (hypothesis) runtime 生成の `## Output contract` セクションに共通原則
  （独立した tool call は同一 turn で発行する / read・search・count を
  serial にしない / timeout・process lifecycle は runtime が持つため
  polling しない / unsafe な mutation を重ねない / 最終 JSON output
  contract は不変）を置くのが、ドリフト耐性と一律適用の点で最も単純。
- (hypothesis) mutation 系に `executionMode: "sequential"` は付けない方が
  よい。付けると mutation を含む batch が丸ごと直列化され、同一ファイル
  直列化は既に `withFileMutationQueue` が担保しているため、付加する安全性に
  対して並列性の損失が大きい。
- (hypothesis) heartbeat / 完了通知で model turn を作る機構は一切不要。
  sibling result は Pi の tool-call barrier が通常どおり次 turn 入力として
  返す。

## Open Questions

- **決定的 test の設計**: `streamFn` fixture で「N 個 sibling call を持つ 1
  assistant message」を emit したときの overlap assert は、test 制御の
  `ToolDefinition`（`WorkerSessionOptions.customTools` 経由で注入可能、
  `session.test.ts` に先例あり）の `execute` を「開始 marker を記録してから
  全 N 個の開始が揃うまで待つ bounded gate」にする手法が timing 非依存で
  決定的（vitest timeout で clean fail、unbounded barrier の deadlock は
  回避される）。補助的に、sequential path は call ごとに
  `tool_execution_end`→次 `tool_execution_start` の順、parallel path は全
  `tool_execution_start` が先に揃うため、イベント順序でも経路を区別できる。
  fixture の `fauxAssistantMessage` は複数 `fauxToolCall` block と
  `stopReason` を表現でき、`createFakePi().streamFn` は実 agent loop を
  駆動する（既存 test が sibling call を既に通している）。
- **write-heavy worker の追加ガイダンス要否**: code-writer が同一ファイルへの
  複数 mutation を同一 turn に組んだ場合の semantics（queue 直列化はされるが
  model が意図した適用順と一致するか）をどこまで指示で保証するか。
- **#57 ToolScheduler との整合**: sibling call の batch 並列度と scheduler の
  concurrency limit が二重に効かないか、budget/`onLimit` abort と sibling
  pending cancel の相互作用が仕様として矛盾しないか。#57 側の設計入力になる。
- **指示文の効果検証**: runtime 生成指示を入れたとき実モデルが same-turn
  発行をどの程度守るか（Ollama smoke 等の非決定的観測で裏取りするか、
  AC を「指示が存在する」ことに留めるか）。
- **`stopReason === "length"` 時の UX**: sibling 全員が error result で返る
  ケースを Worker の最終 output contract がどう扱うか（failure として扱う
  既存経路で足りるか）。`output.maxTokens` cap との相互作用は上記 10 で
  整理済み — 残るのは turn 数を浪費する retry loop の扱い。

## Promotion Candidates

- [x] spec-doc needed: runtime 生成 Worker 指示への same-turn 並列ガイダンス
      追加と、その regression test・execution-engine 文書化の要件（issue #55
      の AC: 決定的 test、指示内容、#57 との互換、CLI/envelope contract 不変、
      「節約は turn/コンテキスト削減由来で token 単価由来ではない」旨の文書化）
      → `SPEC-034zA513QKarQrr5SQtgp7` として作成済み
- [x] adr-doc needed: 指示の所在（`## Output contract` vs per-worker
      markdown）、`executionMode: "sequential"` を mutation tool に付けない
      判断、証明方法（`streamFn` scripted fake-stream）の記録
      → `ADR-034zA5nZuUdGLDMTGh2V6z` として作成済み

## Sources and Relations

- issue #55（探索の trigger・scope・non-goals・AC の一次情報）:
  https://github.com/scarecrow173/cheaper/issues/55
- 派生元 idea: `docs/ideas/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md`
  （IDEA-034zzQkQLoqLftac2tPws5）
- 関連 issue（別軸の並列性）: #57 ToolScheduler（runtime 操作の並行度制限）、
  #62 Workflow Engine（job 横断並列化）
- 一次ソース（pinned `node_modules`）:
  - `@earendil-works/pi-agent-core@0.85.1` `dist/agent.js` / `dist/agent-loop.js`
  - `@earendil-works/pi-coding-agent@0.85.1` `dist/core/sdk.js` /
    `dist/core/tools/write.js` / `dist/core/tools/edit.js`
  - `packages/runtime-pi/src/session.ts` / `src/run.ts` /
    `src/tools/registry.ts` / `src/tools/budget.ts`

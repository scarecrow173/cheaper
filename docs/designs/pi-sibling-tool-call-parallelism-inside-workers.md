---
id: "DESIGN-034zAKcWuB8SuFkOePsubg"
type: "design"
status: "approved"
title: "Pi sibling tool-call parallelism inside Workers"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/55"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "pi-execution-engine.md"
    - "workspace-filesystem-boundary-sandboxed-tools.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/verify-and-leverage-pi-sibling-tool-call-parallelism-inside-workers.md"
    - "../adr/pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/pi-sibling-tool-call-parallelism-inside-workers.md"
    - "https://github.com/scarecrow173/cheaper/issues/57"
    - "https://github.com/scarecrow173/cheaper/issues/62"
  defers: []
  deferred-by: []
---

# Pi sibling tool-call parallelism inside Workers

## Context

SPEC-034zA513QKarQrr5SQtgp7 / ADR-034zA5nZuUdGLDMTGh2V6z の実装設計
（trigger: issue #55）。

cheaper の Pi Worker は serial な ReAct 型シーケンスで tool call を消化して
おり、独立した read / search / count でさえ 1 call ごとに model turn を
消費する。turn ごとに増大する input context の反復送信が delegation の
コスト削減効果を薄めている。

pinned `@earendil-works/pi-agent-core@0.85.1` の一次ソース確認
（DISC-034zzZxx7gzas0a5Anlkwi）により、sibling tool call は既に並列実行
されることが分かっている:

- `Agent` の default は `toolExecution: "parallel"`
  （`dist/agent.js:134`）。cheaper は `toolExecution` も per-tool
  `executionMode` も設定していないため、全 sandboxed tool は今日既に
  parallel-eligible
- parallel path は各 call の `execute` を `Promise.all` で同時実行し、
  `tool_result` message は元の call 順で emit される
  （`agent-loop.js:330-383`）
- `stopReason: "length"` の assistant message 由来の sibling call は
  `failToolCallsFromTruncatedMessage` により全員が未実行のまま error
  result になる
- Pi built-in の write/edit は `withFileMutationQueue` で同一ファイル
  mutation を call 順に直列化しており、cheaper の write/edit は
  `createWriteToolDefinition` / `createEditToolDefinition` に boundary
  operations を注入する形（`packages/runtime-pi/src/tools/registry.ts:67-83`）
  でこれを継承する

つまり本設計の対象は「新規並列実行機構」ではなく、ADR-034zA5nZuUdGLDMTGh2V6z
Option A の具体化である:

1. `buildSystemPrompt`（`packages/runtime-pi/src/run.ts:111-125`）が全
   worker の system prompt 末尾に付ける `## Output contract` セクションへ
   の same-turn 並列ガイダンス追加（exact な文言の決定）
2. `streamFn` / `customTools` seam を使った sibling 並列 semantics の
   決定的 regression test
3. execution-engine 文書（`docs/designs/pi-execution-engine.md`）への
   並列性と節約源の記述追加
4. envelope / exit code contract の維持と残余 risk の明記

sibling tool-call 並列性は Worker 内（intra-Worker）の並列性であり、#57
ToolScheduler（runtime 操作の並行度制限）や #62 Workflow Engine（job 横断
の並列化）とは別軸として扱う。

## Scope

### In Scope

- `buildSystemPrompt` の `## Output contract` セクションへの same-turn
  並列ガイダンス追加（下記「Data and Control Flow」に exact な英文 bullet
  を定義する）
- pinned Pi の実 agent loop 経路に対する決定的 regression test の追加
  （新規 test file、fixture は既存 `test/fixtures/fake-stream.ts` 経由の
  `createFakePi` / `fauxAssistantMessage` / `fauxToolCall` を再利用）:
  - N 個の sibling tool call が並列（overlap）実行されること
  - `tool_result` message が元の call 順で emit されること
  - `stopReason: "length"` で sibling 全員が未実行 error result になること
  - 同一ファイルへの複数 sibling write/edit が call 順に直列化されること
    （`withFileMutationQueue` 継承の pin）
- `buildSystemPrompt` の unit test 用の export 化とガイダンス含有 assert
- `packages/runtime-pi` に `toolExecution` / `executionMode` の強制設定が
  ないことを機械的に検査する code-inspection test
- `docs/designs/pi-execution-engine.md` への sibling tool-call 並列性
  セクション追加（節約源は Worker turn 数と反復 input context の削減で
  あり、token 単価の変化ではない旨を明記）

### Out of Scope

- 独自 agent loop・独自並列 executor の実装（Pi の agent loop をそのまま
  使う）
- operation 完了や heartbeat / progress で model turn を生成する
  completion-driven wake
- daemon / persistent job API の追加
- Workflow DAG scheduler 等の job 横断オーケストレーション（#62 が担う）
- CLI / stdout envelope contract の変更
- mutation 系 tool への `executionMode: "sequential"` 付与（batch 全体を
  直列化して効果源を潰すため不採用。同一ファイル直列化は既存の
  `withFileMutationQueue` が担保する）
- per-worker markdown（`packages/cli/workers/*.md`）への同一ガイダンスの
  複写（表現ドリフト防止。「同一概念の表現は一箇所」原則）
- #57 ToolScheduler の実装（本設計はその導入余地を閉じないことの確認に
  留める）
- 実 model がガイダンスにどの程度従うかの実測（非決定的であり AC の対象に
  しない。Deferred Design Concerns 参照）

## Components and Boundaries

| Component | 責務 | 変更 |
| --- | --- | --- |
| `packages/runtime-pi/src/run.ts` `buildSystemPrompt` | 全 worker 共通の `## Output contract` 生成。same-turn 並列ガイダンス bullet を追加し、unit test のため `export` 化する | 修正 |
| `packages/runtime-pi/src/session.ts` | `WorkerSessionOptions.streamFn` / `customTools` seam を提供（変更不要） | なし |
| `packages/runtime-pi/src/tools/registry.ts` | sandboxed tool 生成。write/edit は `withFileMutationQueue` 継承（変更不要、`executionMode` も付与しない） | なし |
| `packages/runtime-pi/test/sibling-parallelism.test.ts` | sibling 並列 semantics の決定的 test 群（新規） | 追加 |
| `packages/runtime-pi/test/run-prompt.test.ts`（または `run.test.ts` 内の describe 追加） | `buildSystemPrompt` 出力のガイダンス含有 assert | 追加 |
| `packages/runtime-pi/test/no-execution-mode.test.ts` | src 配下に `toolExecution` / `executionMode` が無いことの code-inspection test | 追加 |
| `docs/designs/pi-execution-engine.md` | execution-engine 文書。status は approved のため、追記は新セクションとして行い issue #55 / 本 DESIGN による更新である旨を明記する（repo 内に approved-doc 追記の明文化された規約はないが、ADR の日付付き addendum 前例に倣った自律的な attribution として行う） | 修正 |

test 用 seam はすべて既存のものを使う:

- `createFakePi()`（`src/testsupport.ts`、fixture
  `test/fixtures/fake-stream.ts` が re-export）: `fake.streamFn` を
  `WorkerSessionOptions.streamFn` へ渡し、`fake.setResponses` /
  `createFakePi({ responses })` で scripted assistant message を流す。
  `fauxAssistantMessage` は `FauxContentBlock[]` を取れるため、1 message
  に複数 `fauxToolCall` block を持たせて sibling batch を emit できる
  （`pi-ai` `dist/providers/faux.d.ts:23`）
- `WorkerSessionOptions.customTools`: 決定的計装 `ToolDefinition` を注入
  できる（`session.ts:34`）。並列性 assert は `createSandboxedTools` では
  なく test 独自の instrumented tool を使う
- `createSandboxedTools`（`registry.ts`）: 実 write/edit を通す
  `withFileMutationQueue` 継承 pin test で使用
- `session.subscribe` / `session.messages`: `tool_execution_start` /
  `tool_execution_end` event（`pi-agent-core` `types.d.ts:399,410`）と
  `tool_result` message の順序観測に使用

## Data and Control Flow

### runtime 指示拡張（`buildSystemPrompt`）

現行（`run.ts:111-125`）は `- ` bullet の機械生成テキストを worker
instructions 末尾に連結する。ガイダンスは `Available tools:` bullet の
**後**、`"Output schema:"` の**前**に挿入する（schema JSON は常に
contract 末尾に隣接させる既存構造を維持）。追加する exact な英文
bullet は以下の 5 本とする:

```ts
const contract = [
  "",
  "## Output contract",
  "",
  "- The final message must be a single JSON object that validates against the output schema below.",
  "- Do not wrap the JSON in markdown code fences or any surrounding prose.",
  "- All file paths are workspace-relative POSIX paths.",
  `- Available tools: ${toolNames.length > 0 ? toolNames.join(", ") : "none"}`,
  ...(toolNames.length > 0
    ? [
        "- Independent tool calls must be issued together in the same assistant turn; do not serialize independent reads, searches, or token counting across multiple turns.",
        "- Do not poll or re-ask whether a call finished: the runtime owns timeouts and process lifecycle, and every tool result is returned automatically as input for the next turn.",
        "- Do not batch unsafe mutations: mutation calls that depend on each other or apply multiple mutations to the same target, and reads that expect the result of a sibling write or edit, are not independent — issue them in separate turns.",
        "- Keep each same-turn batch and its per-call arguments small enough to fit within the output token limit applied to each response; an oversized response is truncated and every tool call in it fails unexecuted.",
        "- This batching guidance does not change the final-message contract: the last message is still a single JSON object that validates against the output schema below.",
      ]
    : []),
  "",
  "Output schema:",
  JSON.stringify(worker.outputSchema.schema, null, 2),
];
```

ガイダンス bullet は `toolNames.length > 0` の場合のみ emit する。
現行の validated path（`normalizeToolNames` / `checkCapabilities` /
`createSandboxedTools`）では tool 0 件に到達しないが、export 後の
`buildSystemPrompt` を直接呼ぶ用途（unit test 等）では `[]` が表現可能で
あり、「Available tools: none」と batching 指示が同居する不整合を
構造的に防ぐ。

各 bullet と spec/ADR の要求の対応:

| bullet | 要求 |
| --- | --- |
| 1 本目（same turn 発行 / serial 化禁止） | 独立した tool call は同一 assistant turn で発行する、独立した read / search / counting を serial にしない |
| 2 本目（polling 禁止） | model-driven polling を禁止し、timeout / process lifecycle が runtime 管理である旨 |
| 3 本目（unsafe mutation） | unsafe な mutation の意図的重複禁止。互いに依存する mutation・同一対象への複数 mutation 適用（`write(f,"a")` + `write(f,"b")` のような last-writer-wins ケース）、および副作用に依存する read↔mutation pair（`read(f)` が sibling の `write(f)` / `edit(f)` の結果を期待する pair）を同一 turn に組まない旨を含む |
| 4 本目（output token cap） | same-turn batch と各 call の引数を per-response の effective cap（`min(policy.output.maxTokens, model.maxTokens)`、`session.ts:87-90` で stream call ごとに注入）に収める粒度に保つ旨。cap 超過時の `stopReason: "length"` → sibling 全員未実行 error も併記。数値は model には公開されないため bullet では「output token limit applied to each response」とのみ表現する |
| 5 本目（contract 不変） | 最終 JSON output contract は不変である旨 |

read↔mutation pair を明示する根拠: read 系 tool（read / grep / find /
count_tokens）は `withFileMutationQueue` の対象外であり、同一 batch 内の
mutation との ordering 保証がない（discovery 確認済み挙動 7）。

### 決定的 test の制御 flow

新規 `packages/runtime-pi/test/sibling-parallelism.test.ts`。
`session.test.ts` と同じ構成（`mkdtemp` + `realpath` workspace、
`ModelRuntime.create({ refreshOnCreate: false })` +
`runtime.registerNativeProvider(fake.provider)`、`createWorkerSession` +
`session.prompt()`）を使う。

1. **並列実行の証明（bounded-gate）**:
   - instrumented `ToolDefinition` を N=3 個作り、`customTools` /
     `toolNames` で `createWorkerSession` に注入する。各 `execute` は
     「開始 marker を記録 → 共有 gate で N 個全員の開始が揃うまで待つ →
     成功 result を返す」実装とする
   - instrumented tool の `parameters` schema は `Type.Object({})`（または
     全 field optional）とし、`fauxToolCall("probe_i", {})` が
     `validateToolArguments` を通過して `kind: "prepared"` に到達する
     ことを保証する。schema 不適合は `kind: "immediate"`（`execute` 未
     呼出）になり、「直列実行検出」と誤診されるため必須条件である
   - fake stream は `[fauxToolCall("probe_1",{}), fauxToolCall("probe_2",{}),
     fauxToolCall("probe_3",{})]` を持つ 1 assistant message
     （`stopReason: "toolUse"`）→ 終了用の text message の 2 step を emit
   - Pi が直列実行なら gate は揃わず各 `execute` が deadline で reject →
     error tool result となり assertion で clean fail する（deadlock で
     はない）。parallel path なら gate が揃い全員完了
   - gate は unbounded にしない: `execute` 内の待機に短い deadline
     （約 1s、`Promise.race` で reject）を持たせる。直列 regression 時は
     N×deadline が掛かるため、`it(..., { timeout })` で test timeout を
     N×deadline 超（例: 10s）に明示設定し、assertion 由来の失敗が
     timeout に潰されないようにする
   - gate の要求は「全 N 個が同時に in-flight」とする。これは現行 Pi
     の unbounded `Promise.all`（`agent-loop.js:372`）を pin する意図的
     に厳しい基準であり、将来 Pi が batch 並行度 cap を導入した場合は
     「少なくとも 2 個の overlap」への緩和を要する。test コメントに
     この意図を明記する
   - 補助 assert: `session.subscribe` で記録した event 列で、全
     `tool_execution_start` が最初の `tool_execution_end` より前に
     揃うこと。この不変条件は **全 sibling call が `kind: "prepared"` に
     到達した場合に限り** 成立する — `prepareToolCall` が
     `kind: "immediate"`（tool 未発見・引数 validation 失敗・
     `beforeToolCall` block・abort）を返すと、その `tool_execution_end`
     は prep loop 内で inline emit され（`agent-loop.js:340-351`）、
     parallel path でも start/end が交互に見える。assert 前に全 call が
     `execute` に到達したこと（marker 数 === N）を確認し、immediate 発生
     時は fixture 不備として区別できる形にする
2. **result 順序**: sleep ではなく決定的な reverse-completion gate で
   完了順を逆転させる。probe_i（i < N）の `execute` は probe_{i+1} の
   完了 marker を await し、probe_N は即座に成功を返す — これにより
   並列実行下で完了順が確定的に probe_N … probe_1 になる（timer 不使用、
   NFC の timing 非依存要件と整合）。その上で `session.messages` 内の
   `tool_result` message 並びが元の call 順（`toolCallId` / `toolName`
   対応）であることを assert する。`tool_execution_end` **event** は
   完了順に emit されるため対象は `session.messages` のみとする
3. **`stopReason: "length"`**:
   - sibling batch を持つ assistant message を
     `{ stopReason: "length" }` で emit し、続けて終了用 text message
   - instrumented tool の `execute` が一度も呼ばれないこと、および emit
     された tool result（session.messages / `tool_execution_end` の
     `isError`）が全員 error であることを assert する
4. **同一ファイル mutation の call 順直列化**（`withFileMutationQueue`
   継承 pin）:
   - `createSandboxedTools({ names: ["write","edit"], ... })` の実 tool を
     使い、sibling batch `[write("f.txt","hello"),
     edit("f.txt",{edits:[{oldText:"hello",newText:"world"}]})]` を emit
   - edit の `oldText` は write の結果にしか存在しないため、queue が
     正しく機能していれば両 call 成功 + 最終内容 `"world"` になる
     （apply 順自体は call 順で決定的: deferred thunk が call 順に起動
     され `registrationQueue` が per-key chain をその順で組む）
   - **検出の非対称性を明記する**: queue が regression した場合、write
     の `writeFile` が edit の `access` より先に完了すれば edit は成功
     してしまい false-PASS し得る（逆は edit の ENOENT error で検出）。
     したがってこの test は PASS 経路は決定的だが regression 検出は
     timing 依存である。強化する場合は injected ops
     （`createWriteOperations` / `createEditOperations` seam 経由）に
     op 呼出順記録を仕込み、`write.writeFile` 完了が `edit.access` 開始
     より先行することを assert する — これは test コメントに選択肢
     として明記する
5. **ガイダンス含有**: `buildSystemPrompt` を export 化し、unit test で
   出力文字列に same-turn 発行・serial 化禁止・polling 禁止・unsafe
   mutation・output token cap の各文言（英 bullet の key phrase）が
   含まれることを assert する。`session.systemPrompt` 経由ではなく
   function 単位で検査し、文言の所在を `run.ts` に pin する
6. **code inspection**: `packages/runtime-pi/src/**/*.ts` を走査し
   `toolExecution` / `executionMode` の識別子が出現しないことを assert
   する vitest test（test file 自身と本設計が参照する test 配下は除外）。
   #57 ToolScheduler の concurrency limit 導入余地を機械的に pin する
   - matching semantics を明示する: comment（`//`・`/* */`）と string
     literal を strip した上での identifier 一致とし、「意図的に設定し
     ない旨の説明 comment」が self-sabotage しないようにする（実装は
     簡易 strip または property-assignment pattern
     `toolExecution\s*:|executionMode\s*:` で十分）
   - この pin は「identifier の完全な不存在」を要求する点で #57 余地の
     確保より強く、将来 legitimate な sequential tool を追加する場合は
     test の緩和を要する — 意図的な厳格さとして明記する
   - `src/testsupport.ts` も走査対象である（`src/` 配下のため）。test
     support に `executionMode` が必要になった場合は同様に trip するが、
     それは意図した pin 効果とする

### 文書化

- `docs/designs/pi-execution-engine.md` に「Sibling tool-call
  parallelism」セクションを追記する。記述内容:
  - pinned `pi-agent-core@0.85.1` の `toolExecution: "parallel"` 既定を
    cheaper はそのまま利用する（`toolExecution` / `executionMode` は
    一切設定しない）
  - parallel path の semantics（`Promise.all` overlap 実行、
    `tool_result` は call 順 emit、`stopReason: "length"` で sibling
    全員未実行 error、同一ファイル write/edit は
    `withFileMutationQueue` で call 順直列化）
  - 節約源は **Worker turn 数と増大 input context の反復送信の削減**で
    あり、並列化は token 単価を変えない
  - `output.maxTokens` cap との相互作用（cap 超過 turn は全 sibling 未
    実行になるため batch 粒度を cap に収める）
  - approved doc への追記である旨と参照元（本 DESIGN /
    SPEC-034zA513QKarQrr5SQtgp7 / issue #55）を明記する（repo 内に
    approved-doc の追記規約は明文化されていないが、ADR の日付付き
    addendum 前例に倣った自律的な attribution とする）
- user guide（`docs/guides/workers.md`）の更新は本設計の必須要件ではない
  （spec は execution-engine 文書のみを要求）。実装時に必要と判断されれば
  別途検討する

## Non-Functional Constraints

- 決定的 test は実 provider・実 model に依存せず、`streamFn` seam 経由の
  scripted fake stream のみで CI で安定的に再現できること
- 並列性 assert は timing 非依存であること（sleep 比較ではなく gate /
  event 順序 / edit 成功可否で判定する）
- 指示変更後も最終 JSON output contract、stdout envelope、exit code が
  現行と一致すること（`buildSystemPrompt` の変更は system prompt 文言のみ。
  既存の envelope / contract test は無変更で pass すること）
- `packages/runtime-pi` は `toolExecution` / per-tool `executionMode` を
  新たに強制しないこと（Pi parallel 既定を維持し、#57 の scheduler
  concurrency limit 導入余地を残す）
- 新規依存の追加なし（vitest・既存 seam のみ）

## Risks and Trade-offs

- **model がガイダンスに従わない余地**: 指示は model の振る舞いを強制
  しない。serial 発行を続けても機能上の誤りではなく turn 削減効果が出ない
  だけ。実効性は `meta.turns` / `usage.jsonl` の run record と
  `cheaper usage` 集計で後から観測する（Deferred）
- **fake-stream 忠実度**: 決定的 test は faux provider の再現度に依存する。
  `pi-agent-core` は transitive 依存で pin は lockfile 解決結果に依存する
  ため、Pi version bump 時は本 test の再検証で drift を検知する
- **effective output token cap と batching の相互作用**: same-turn
  batching を促す指示は、per-response の effective cap
  （`min(policy.output.maxTokens, model.maxTokens)`、現行
  `policy.yaml` の `output.maxTokens: 2000` が主な制約項）を超える
  batch を生じさせ得る。cap 超過 turn は `stopReason: "length"` となり
  sibling 全員が未実行 error になり、retry は `stopReason === "toolUse"`
  の turn を生まないため `maxTurns` には乗らず `timeout`（既定 120s）
  まで spin し得る。指示 bullet 4 本目で緩和するが、cap を大きく超える
  workload は残余 risk として残る
- **`withFileMutationQueue` の key edge**: queue key は `resolve()` +
  存在時のみ `realpath` で正規化されるため、**存在しないファイルへの別
  spelling**（symlinked dir 経由・Windows の case variant）は別 key に
  なり同一対象を直列化しない narrow edge が残る
- **read↔mutation ordering**: read 系 tool は queue 対象外のため同一
  batch 内の mutation との ordering 保証がない。指示 bullet 3 本目で
  同一 turn への編入を禁じて緩和するが、強制力はない
- **write-heavy worker の mutation 順**: code-writer が同一ファイルへの
  複数 mutation を同一 turn に組んだ場合、queue 直列化はされるが model
  の意図した適用順とずれる余地がある（Deferred）

## Deferred Design Concerns

- #57 ToolScheduler と sibling batch 並列度の統合方針 | #57 が未実装で
  本設計だけでは確定できない | #57 の設計・実装着手時 | sibling batch
  並列度と scheduler limit の二重制御で過剰な直列化が起きる余地
- 実 model の same-turn 発行遵守度の実測（Ollama smoke 等） | model 挙動が
  非決定的で AC の対象にできない | 指示効果の定量化要求が上がった時点 |
  指示の実効性が未検証のまま運用される余地。測定経路は既存の
  `meta.turns` / `usage.jsonl` run record と `cheaper usage` 集計
  （turn 数の前後比較）
- write-heavy worker（code-writer）向けの worker 個別 mutation 補足 |
  runtime 生成の共通原則でまず足りる想定（YAGNI） | worker 個別調整の
  要求が上がった時点 | 同一ファイルへの複数 mutation で model の意図した
  適用順とずれる余地

## Verification Notes

- [ ] `sibling-parallelism.test.ts`: N 個の sibling tool call を持つ 1
      assistant message が bounded-gate で overlap 実行されることを実
      provider なしで assert する（`parameters: Type.Object({})` の
      instrumented tool、約 1s の per-execute deadline、N×deadline 超の
      明示 `it` timeout。直列実行時は全 `execute` が到達せず error
      result → assertion で clean fail）
- [ ] 同 test: `tool_result` message が元の call 順で emit されることを
      assert する（sleep ではなく reverse-completion gate で完了順を
      決定的に逆転させて検証）
- [ ] 同 test: `stopReason: "length"` で sibling 全員が未実行（execute
      未呼出）かつ error result になることを assert する
- [ ] 同 test: 同一ファイルへの sibling write → edit が call 順に直列化
      されること（edit 成功 + 最終内容）を assert する。PASS 経路は
      決定的、regression 検出側は timing 依存である旨を test コメントに
      明記する
- [ ] `buildSystemPrompt` unit test: same-turn 発行・serial 化禁止・
      polling 禁止・unsafe mutation（副作用依存 read↔mutation と同一
      対象への複数 mutation を含む）・output token cap の各文言を
      assert する。`toolNames.length === 0` の場合にガイダンス bullet が
      emit されないことも assert する
- [ ] code-inspection test: `packages/runtime-pi/src` に `toolExecution` /
      `executionMode` が存在しないことを assert する（comment /
      string literal strip 済みの identifier 一致）
- [ ] 既存の envelope / contract test が無変更で pass する
      （`mise run check`、少なくとも runtime-pi / cli の test suite）
- [ ] `docs/designs/pi-execution-engine.md` に並列性セクションが追記
      され、節約源が turn 数・反復 context 削減であり token 単価の変化
      ではない旨が明記される

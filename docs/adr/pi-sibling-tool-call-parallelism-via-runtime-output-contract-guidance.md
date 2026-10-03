---
id: "ADR-034zA5nZuUdGLDMTGh2V6z"
type: "adr"
status: "proposed"
title: "Pi sibling tool-call parallelism via runtime output-contract guidance"
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
    - "../discovery/pi-sibling-tool-call-parallelism-inside-workers.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/cheaper/issues/57"
    - "https://github.com/scarecrow173/cheaper/issues/62"
    - "./agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "./worker-definition-runtime-policy-runtime-invariants-capability.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Pi sibling tool-call parallelism via runtime output-contract guidance

## Context and Problem Statement

issue #55。cheaper の Pi Worker は serial な ReAct 型シーケンス
（tool → model → tool → model）で tool call を消化しており、独立した
read / search / count でさえ 1 call ごとに model turn を消費する。
turn ごとに増大する context の反復送信が発生し、delegation の
コスト削減効果を薄めている。

調査（DISC-034zzZxx7gzas0a5Anlkwi）により、pinned
`@earendil-works/pi-agent-core@0.85.1` は sibling tool call を既定で
並列実行することが確認済みである: `Agent` の default は
`toolExecution: "parallel"`（`dist/agent.js:134`）。batch 全体が
直列化されるのは `config.toolExecution === "sequential"` または
batch 内のいずれかの tool が `executionMode: "sequential"` を持つ
場合のみ（`dist/agent-loop.js:285-292`）。並列 path では
`Promise.all` で同時実行され、`tool_result` は元の call 順で emit
される（`agent-loop.js:330-383`）。cheaper は `toolExecution` も
per-tool `executionMode` も設定しておらず、全 sandboxed tool は
今日既に parallel-eligible。

つまり決定すべきは「並列実行機構の新規実装」ではなく、(a) 既存の
並列性を model が実際に使うよう促す指示の置き場所、(b) mutation
安全性の担保方法、(c) contract の証明方法、である。

## Decision Drivers

* issue #55 の scope / non-goals: 独自 agent loop、operation 完了で
  model を起こす completion-driven wake、daemon API 追加、並列
  executor の重複実装は行わない。CLI / stdout envelope contract は
  不変。
* cost-first: 並列化の効果源は Worker turn 数と増大 context の
  反復送信の削減であり、token 単価の変化ではない。
* 表現ドリフト防止: 同一概念の表現は一箇所（AGENTS.md）。3 つの
  worker markdown に同じ文言を分散しない。
* mutation 安全性: 同一ファイルへの mutation は Pi built-in の
  `withFileMutationQueue` が既に直列化する（`pi-coding-agent`
  `dist/core/tools/write.js:34`、`edit.js:96`）。cheaper の
  write/edit は `createWriteToolDefinition` /
  `createEditToolDefinition` に boundary operations を注入する形
  （`packages/runtime-pi/src/tools/registry.ts:67-83`）でこの
  直列化を継承する。
* `wrapToolBudget`（`packages/runtime-pi/src/tools/budget.ts`）の
  check-then-increment は同期処理で並列下でも安全。`onLimit` は
  session abort を起こし pending の sibling 実行を cancel する。
* `stopReason: "length"` 由来の tool call は sibling 全員が未実行で
  error result になる（truncated arguments の実行防止）。
* 指示の 2 層構成: worker 指示は `packages/cli/workers/*.md` の
  `worker.instructions` と、`buildSystemPrompt`
  （`packages/runtime-pi/src/run.ts:111-125`）が末尾に付ける runtime
  生成 `## Output contract` セクションからなる。

## Considered Options

* Option A: runtime 生成 `## Output contract` セクション
  （`buildSystemPrompt`）に same-turn 並列ガイダンスを追加し、
  Pi の parallel 既定と `withFileMutationQueue` に mutation 安全性を
  委ね、`streamFn` seam の決定的 test で semantics を pin する。
* Option B: ガイダンスを各 worker markdown
  （`packages/cli/workers/bulk-reader.md` / `code-writer.md` /
  `doc-writer.md`）の instructions に記述する。
* Option C: write/edit tool に `executionMode: "sequential"` を
  付与し、mutation を含む batch を強制直列化する。
* Option D: 指示変更は行わず regression test のみ追加する
  （model の既定挙動に依存する）。

## Decision Outcome

Chosen option: "Option A"。

- **指示の所在**: `buildSystemPrompt`
  （`packages/runtime-pi/src/run.ts:111-125`）が全 worker の
  system prompt 末尾に付ける `## Output contract` セクションを
  拡張し、以下の共通原則を記述する:
  - 独立した tool call は同一 assistant turn でまとめて発行する。
  - 独立した read / grep / find / count_tokens を順番待ちに直列化
    しない。
  - timeout・process lifecycle は runtime が所有するため、model
    側で polling loop を組まない。
  - 互いに依存する mutation や同一対象への重複適用のような
    unsafe な mutation を意図的に重ねて発行しない。`read(f)` が
    sibling の `write(f)` / `edit(f)` の結果を期待するような
    副作用依存の read↔mutation pair も独立ではないため同一 turn
    に組まない（read 系 tool は queue 対象外で ordering 保証が
    ない）。
  - same-turn batch と各 call の引数は turn ごとの
    `output.maxTokens` cap（現行 `policy.yaml` `2000`）に収まる
    粒度に保つ。cap 超過は `stopReason: "length"` となり sibling
    全員が未実行 error result になる。
  - 最終 JSON output contract は不変。
- **mutation 安全性**: `executionMode: "sequential"` は付けない。
  付けると mutation call を含む batch が丸ごと直列化され
  （`agent-loop.js:285-292`）、read 群との混在 batch の並列性まで
  潰れる。同一ファイル直列化は既に `withFileMutationQueue` が
  担保している。
- **証明方法**: `WorkerSessionOptions.streamFn`
  （`packages/runtime-pi/src/session.ts:44`）の scripted fake-stream
  seam（fixture: `packages/runtime-pi/test/fixtures/fake-stream.ts`）
  で「N 個の sibling tool call を持つ 1 assistant message」を emit
  し、実行の overlap と call 順の result emit を assert する
  決定的 test を追加する。実 provider 不要で CI 回帰できる。

### Consequences

* Good, because 実装差分は `buildSystemPrompt` の文案追加と test の
  みで、runtime 機構の新規実装が不要（Pi pinned 版の確認済み
  semantics をそのまま活用する）。
* Good, because 全 worker（bulk-reader / code-writer / doc-writer
  および将来の worker）に同一原則が一律適用され、per-worker
  markdown への複写による表現ドリフトを防げる。
* Good, because mutation tool を sequential 化しないため、異なる
  ファイルへの write/edit や read+write 混在 batch の並列性が
  維持される（ただし副作用依存の read↔mutation pair は指示で
  同一 turn から除外する。read 系 tool は
  `withFileMutationQueue` 対象外で ordering 保証がないため）。
* Good, because fake-stream test は決定的で、issue #55 の「実測で
  証明する」要件を CI で回帰可能にする。
* Bad, because 指示は model の振る舞いを強制しない。model が serial
  発行を続けても機能上の誤りにはならず、turn 削減効果が出ない
  だけである。指示の実効性は非決定的観測（real-provider smoke
  等）に委ねる。
* Bad, because 決定的 test は fake-stream の忠実度に依存する。
  Pi version bump で streamFn や batch semantics が変わった場合の
  ずれは、upgrade 時に本 test の再検証で別途検知する必要がある。
* Bad, because same-turn batching の推奨は `output.maxTokens` cap
  との相互作用を持つ。cap（現行 `2000`）は全 assistant turn に
  適用され（`session.ts:87-90`）、N 個の sibling call を含む turn
  が cap を超えると `stopReason: "length"` で sibling 全員が未実行
  error になる。retry が `"toolUse"` turn を生まないため
  `maxTurns` ではなく `timeout`（既定 120s）まで spin し得る。
  指示で batch・引数の粒度を cap に収めるよう求めることで緩和
  するが、cap を大きく超える workload は残るリスクとして
  discovery の open question に残す。
* Bad, because `withFileMutationQueue` の直列化は mutation 同士の
  みで、queue key は存在しないファイルへの別 spelling（symlinked
  dir 経由・case variant）を別 key として扱う narrow edge が残る
  （`file-mutation-queue.js`）。適用順自体は call 順で決定的
  （`agent-loop.js:372` + `registrationQueue`）。

### Confirmation

* fake-stream test で「N 個の sibling call が並列に overlap 実行
  され、result が元の call 順で emit される」ことを timing 非依存
  の方法で assert する。具体的には test 制御 `ToolDefinition`
  （`customTools` 注入）の `execute` が開始 marker を記録し全 N
  個の開始が揃うまで待つ bounded gate（vitest timeout で clean
  fail）を使う。イベント順序でも区別できる: sequential path は
  call ごとに `tool_execution_end` → 次 `tool_execution_start`、
  parallel path は全 `tool_execution_start` が先に揃う。
* fake-stream test で「同一ファイルへの複数 sibling write/edit
  が call 順に直列化される」ことを assert する
  （`withFileMutationQueue` 継承の固定）。
* `buildSystemPrompt` が生成する system prompt に same-turn 並列
  ガイダンスが含まれることを unit test で assert する。
* write/edit を含む全 tool に `executionMode` が設定されていない
  こと（`toolExecution` も未設定であること）を code 検査で確認
  する。

## Implementation Plan

* Affected paths:
  - `packages/runtime-pi/src/run.ts`（`buildSystemPrompt` の
    `## Output contract` セクション拡張）
  - `packages/runtime-pi/test/`（sibling parallelism の決定的 test
    を新規追加。fixture は `test/fixtures/fake-stream.ts` の
    `createFakePi` / `fauxAssistantMessage` / `fauxToolCall` を
    再利用）
  - worker runtime の execution engine 文書化（parallel 既定・
    同一ファイル直列化・`stopReason:"length"` の扱い）
* Patterns to follow: `buildSystemPrompt` の既存 contract 文案
  スタイル（`- ` bullet の機械生成テキスト）、`test/fixtures/
  fake-stream.ts` の scripted stream 手法、`wrapToolBudget` の
  tool wrap 方式。
* Patterns to avoid:
  - `toolExecution` / per-tool `executionMode` の設定（Pi の
    parallel 既定を維持する）
  - per-worker markdown への同一ガイダンスの複写
  - 独自 agent loop・独自並列 executor・polling / heartbeat 機構の
    追加（issue #55 non-goals）
  - CLI / stdout envelope contract の変更
* Dependencies or configuration: 新規依存なし。
  `@earendil-works/pi-agent-core@0.85.1` /
  `@earendil-works/pi-coding-agent@0.85.1` の pinned 挙動に依存する
  ため、Pi version bump 時は本 ADR の semantics を再検証する。

## Verification

* [ ] fake-stream test: 1 assistant message 内の複数 sibling tool
      call が並列（overlap）実行されることを assert
* [ ] fake-stream test: tool result message が元の call 順で emit
      されることを assert
* [ ] fake-stream test: 同一ファイルへの複数 sibling write/edit が
      call 順に直列化されることを assert
      （`withFileMutationQueue` 継承の固定）
* [ ] unit test: `buildSystemPrompt` の output に same-turn 並列
      ガイダンス（独立 call の同一 turn 発行・serial 化しない・
      polling 禁止・unsafe mutation を重ねない）が含まれる
* [ ] code 検査: `packages/runtime-pi` に `toolExecution` /
      `executionMode` の設定がないこと
* [ ] `stopReason: "length"` で sibling 全員が未実行 error result
      になることを assert する test があること（spec AC と同等）
* [ ] `mise run check` が pass

## Pros and Cons of the Options

### Option A（採用）: runtime 生成 Output contract ガイダンス + Pi parallel 既定 + streamFn test

* Good, because 全 worker に一律適用され、runtime と文言の version
  が揃って表現ドリフトしない。「timeout / process lifecycle は
  runtime が持つので polling 禁止」のような runtime 固有の事実と
  同居できる。
* Good, because 実装変更が最小で、確認済みの Pi semantics を
  そのまま contract として pin できる。
* Bad, because worker 個別の調整（write-heavy worker 向けの注意等）
  には向かない。必要になった場合は層の責務境界を明文化したうえで
  worker 固有補足を別途検討する。

### Option B（不採用）: per-worker markdown instructions

* Good, because worker ごとのチューニングが可能。
* Bad, because bulk-reader / code-writer / doc-writer で文言が
  重複し表現ドリフトしやすく、「同一概念の表現は一箇所」原則に
  反する。runtime 生成だけで要件を満たせるため YAGNI。

### Option C（不採用）: write/edit に `executionMode: "sequential"` 付与

* Good, because 強制力が強く model の判断に依存しない。
* Bad, because batch 内に mutation call が 1 つでもあると Pi の
  semantic 上 batch 全体が直列化され（`agent-loop.js:285-292`）、
  read+write 混在 batch の並列性まで潰れる。同一ファイル安全性は
  既に `withFileMutationQueue` が担保するため、付加する安全性に
  対して並列性の損失が大きく、効果源を自ら狭める。

### Option D（不採用）: 指示変更なし（test のみ）

* Good, because 差分が最小。
* Bad, because model が sibling call を同一 turn に発行するかは
  model 既定任せになり、issue #55 が求める「並列性の活用」が
  model 次第になる。指示追加のコストは低く、見送る理由がない。

## More Information

- issue #55（scope・non-goals・AC の一次情報）:
  https://github.com/scarecrow173/cheaper/issues/55
- DISC-034zzZxx7gzas0a5Anlkwi（pinned Pi の一次ソース確認済み
  semantics・論点比較の派生元）:
  `docs/discovery/pi-sibling-tool-call-parallelism-inside-workers.md`
- `pi-agent-core` は `pi-coding-agent` 経由の transitive 依存
  （direct pin は `pi-coding-agent` / `pi-ai` のみ）。version
  固定は `pnpm-lock.yaml` の解決結果に依存するため、Pi upgrade
  時は本 ADR が引用する semantics を再検証する
- 関連 issue（別軸の並列性。本 ADR は #57 ToolScheduler の設計
  入力にもなる）: https://github.com/scarecrow173/cheaper/issues/57 、
  https://github.com/scarecrow173/cheaper/issues/62
- 一次ソース（pinned `node_modules`）:
  `@earendil-works/pi-agent-core@0.85.1` `dist/agent.js:134` /
  `dist/agent-loop.js:285-292,330-383`、
  `@earendil-works/pi-coding-agent@0.85.1` `dist/core/tools/write.js:34`
  / `edit.js:96`、
  `packages/runtime-pi/src/run.ts:111-125` /
  `src/tools/registry.ts:67-83` / `src/tools/budget.ts` /
  `src/session.ts:44`


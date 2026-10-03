---
id: "SPEC-034zA513QKarQrr5SQtgp7"
type: "spec"
status: "approved"
title: "Verify and leverage Pi sibling tool-call parallelism inside Workers"
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
  defers: []
  deferred-by: []
---

# Verify and leverage Pi sibling tool-call parallelism inside Workers

## What Should Be Built

pinned Pi version（`@earendil-works/pi-coding-agent@0.85.1`。agent loop は
`@earendil-works/pi-agent-core@0.85.1` の `dist/agent-loop.js`）が、1
assistant turn 内の複数の独立した tool call（sibling tool call）を実際に
並列実行することを、実 provider・実 model 非依存の決定的 runtime test で
証明し、その挙動を cheaper の supported contract として固定する。

あわせて、runtime 生成の Worker 指示（`buildSystemPrompt`
（`packages/runtime-pi/src/run.ts`）が付ける `## Output contract`
セクション）を拡張し、Worker が独立した tool call を同一 assistant turn
で発行するよう model を誘導する。併せて regression test と
execution-engine 文書を整備する。

sibling tool-call 並列性は Worker 内（intra-Worker）の並列性であり、#57
ToolScheduler（runtime 操作の並行度制限）や #62 Workflow Engine（job 横断
の並列化）とは別軸の話題として扱う。

## Why Now

serial な ReAct 型の tool → model → tool → model シーケンスは Worker turn
数と、ターンごとに増大する input context の反復送信を増やし、delegation
のコスト削減効果を薄める。

discovery（`DISC-034zzZxx7gzas0a5Anlkwi`）で pinned Pi の挙動が一次
ソース（`node_modules`）から確認済みである:

- `Agent` の default は `toolExecution: "parallel"` で、cheaper は
  `toolExecution` も per-tool `executionMode` も設定していないため、
  既に parallel 経路が有効
- parallel 経路は各 call の `execute` を `Promise.all` で同時実行し、
  `tool_result` message は元の call 順で emit される
- batch 内に `executionMode: "sequential"` の tool が 1 つでもあると
  batch 全体が直列化される
- `stopReason: "length"` の assistant message 由来の sibling call は
  全員が未実行のまま error result になる
- Pi built-in の write/edit は `withFileMutationQueue` で同一ファイル
  mutation を直列化しており、cheaper の write/edit もこれを継承する

つまり新規の並列実行機構は不要で、不足しているのは「既存挙動の決定的な
証明」「model への same-turn 発行の指示」「regression test・文書化」で
ある。

## Users and Value

- **cheaper CLI の利用者 / parent coding agent**: Worker が独立した
  read・search・counting を same-turn で発行することで Worker turn 数と
  反復 input context が減り、delegation のコスト削減効果が維持される。
  節約源は turn 数とコンテキスト反復の削減であり、並列化が token 単価を
  変えるわけではない。
- **cheaper メンテナ**: pinned Pi の sibling tool-call 挙動が決定的 test
  で contract 化され、Pi upgrade 時の挙動ドリフトを regression として
  検出できる。

## Scope

### In Scope

- pinned Pi version の実際の agent loop に対し、sibling tool call の
  並列実行・call 順の result emit・truncated message 時の sibling 全員
  fail を示す決定的 runtime test（実 provider 不要、CI で再現可能）
- runtime 生成 Worker 指示への same-turn 並列ガイダンス追加:
  独立した tool call は同一 assistant turn で発行する、独立した
  read / search / counting を serial にしない、timeout / process
  lifecycle は runtime が管理するため model-driven polling をしない、
  unsafe な mutation を意図的に重複させない、最終 JSON output contract
  は不変
- 上記挙動と指示内容を固定する regression test
- sibling tool-call 並列性と、そのコスト効果の由来（Worker turn 数・
  反復 input context の削減であり token 単価の変化ではない）を説明する
  execution-engine 文書
- #57 ToolScheduler の将来導入を阻害しないことの確認（#57 は未実装だが
  本変更がその設計余地を閉じない）

### Out of Scope

- 独自 agent loop の実装（Pi の agent loop をそのまま使う）
- operation 完了や heartbeat / progress で model turn を生成する
  completion-driven wake（sibling result は Pi 本来の tool-call barrier
  semantics で次 turn 入力として返る）
- daemon / persistent job API の追加
- Pi の sibling-call 並列実行を置き換える独自並列 executor の実装
- Workflow DAG scheduler 等の job 横断オーケストレーション（#62 が担う）
- CLI / stdout envelope contract の変更
- mutation 系 tool への `executionMode: "sequential"` 付与（batch 全体を
  直列化して効果源を潰すため採用しない。同一ファイル直列化は既存の
  `withFileMutationQueue` が担保する）

## Requirements

- 決定的 test は実 provider・実 model に依存せず、CI で安定的に再現
  できること（scripted stream 等の決定的 seam を利用）
- 決定的 test は pinned version の実 agent loop 経路で sibling tool call
  が並列実行されること、`tool_result` が元の call 順で emit されること、
  `stopReason: "length"` 時に sibling 全員が未実行で error result に
  なることを assert する
- runtime 生成 Worker 指示が、独立した tool call の同一 assistant turn
  発行を明示的に要求し、独立した read / search / counting の serial 化を
  避ける旨を含む
- 指示が model-driven polling（完了確認・heartbeat 的な再問い合わせ）を
  禁止し、timeout / process lifecycle が runtime 側の責務である旨を
  明記する
- 指示が unsafe な mutation の意図的重複を禁じる警告を含む（同一ファイル
  mutation の直列化は Pi が担保するが、model は依存関係のある mutation
  を同一 turn に組まない）。「依存関係」には副作用への依存も含め、
  `read(f)` が sibling の `write(f)` / `edit(f)` の結果を期待するような
  read↔mutation pair は独立ではないため同一 turn に組まない旨を明記する
  （read 系 tool は `withFileMutationQueue` の対象外で ordering 保証が
  ないため）
- 指示が same-turn batch と各 call の引数を、turn ごとに適用される
  `output.maxTokens` cap（`session.ts:87-90`、現行 `policy.yaml`
  `output.maxTokens: 2000`）に収まる粒度に保つよう求めること。cap を
  超えた turn は `stopReason: "length"` となり sibling 全員が未実行
  error result になる（詳細は discovery 確認済み挙動 10）
- 指示変更後も最終 JSON output contract、stdout envelope、exit code が
  現行と一致する
- cheaper が `toolExecution` / per-tool `executionMode` を新たに強制
  しないなど、#57 の scheduler concurrency limit 導入余地を維持する
- execution-engine 文書が節約源を「Worker turn 数と反復 input context の
  削減」と明記し、並列化が token 単価を変えないことを説明する

## Acceptance Criteria

- [ ] pinned Pi version の実 agent loop に対し、同一 assistant turn の
      複数 sibling tool call が並列実行されることを示す決定的 runtime
      test が存在し、実 provider なしに CI で pass する
- [ ] 同 test が `tool_result` message が元の call 順で emit される
      ことを assert する
- [ ] `stopReason: "length"` の assistant message 由来の sibling call
      が全員未実行で error result になることを assert する test がある
- [ ] runtime 生成 Worker 指示に「独立した tool call は同一 assistant
      turn で発行する」旨の文言が含まれ、test で固定される
- [ ] 指示に「独立した read / search / counting を serial にしない」
      旨の文言が含まれる
- [ ] 指示が model-driven polling を禁止し、timeout / process
      lifecycle が runtime 管理である旨を含む
- [ ] 指示が unsafe な mutation の意図的重複を避ける警告を含み、
      副作用に依存する read↔mutation pair を同一 turn に組まない旨も
      含む
- [ ] 指示が same-turn batch と各 call の引数を turn の出力上限に
      収まる粒度に保つ旨を含む
- [ ] 同一ファイルへの複数 sibling write/edit が call 順に直列化
      されることを assert する test がある
      （`withFileMutationQueue` 継承の固定）
- [ ] 最終 JSON output contract / stdout envelope / exit code が現行と
      一致する（既存 contract test が変更なしで pass する）
- [ ] execution-engine 文書が sibling tool-call 並列性を説明し、節約源が
      Worker turn 数と反復 input context の削減であり token 単価の変化
      ではないと明記する
- [ ] `packages/runtime-pi` に `toolExecution` / per-tool
      `executionMode` の強制設定がないことが code 検査で確認される
      （#57 ToolScheduler の concurrency limit 導入余地を維持する
      機械的に検査可能な条件）

## Deferred Design Concerns

- #57 ToolScheduler と sibling batch 並列度の統合方針 | #57 が未実装で
  本 spec だけでは確定できない | #57 の設計・実装着手時 | sibling batch
  並列度と scheduler limit の二重制御で過剰な直列化が起きる余地
- 実 model が same-turn 発行指示にどの程度従うかの実測（Ollama smoke 等） |
  model 挙動が非決定的で AC の対象にできない | 指示効果の定量化要求が
  上がった時点 | 指示の実効性が未検証のまま運用される余地。測定経路は
  既存の `meta.turns` / `usage.jsonl` run record と `cheaper usage`
  集計（turn 数の前後比較）
- write-heavy worker（code-writer）向けの worker 個別の mutation 補足 |
  runtime 生成の共通原則でまず足りる想定（YAGNI） | worker 個別調整の
  要求が上がった時点 | 同一ファイルへの複数 mutation で model の意図した
  適用順とずれる余地

## Sources and Relations

- issue #55（trigger・scope・non-goals・AC の一次情報）:
  https://github.com/scarecrow173/cheaper/issues/55
- 派生元 discovery `pi-sibling-tool-call-parallelism-inside-workers`
  （`DISC-034zzZxx7gzas0a5Anlkwi`）: pinned Pi の一次ソース確認済み挙動、指示の
  所在（runtime 生成 `## Output contract` セクション）、`executionMode`
  を mutation tool に付けない判断、証明方法（決定的 seam）の比較検討
- 関連 issue（別軸の並列性）: #57 ToolScheduler（runtime 操作の並行度
  制限）、#62 Workflow Engine（job 横断並列化）
- 一次ソース（pinned `node_modules`）:
  `@earendil-works/pi-agent-core@0.85.1` `dist/agent.js` /
  `dist/agent-loop.js`、`@earendil-works/pi-coding-agent@0.85.1`
  `dist/core/sdk.js` / `dist/core/tools/write.js` /
  `dist/core/tools/edit.js`

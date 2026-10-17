---
id: "DESIGN-LAY8HNBqNyjH7L5DFvTPc2"
type: "design"
status: "approved"
title: "execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume の実装設計"
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
  implements:
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
  implemented-by: []
  depends-on:
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../discovery/bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
    - "../plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../specs/worker-run-nullable-maxturns-and-stall-timeout.md"
  derived-by:
    - "../plans/worker-run-nullable-maxturns-stall-timeout-and-resume.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume の実装設計

## 概要

SPEC-onzPU8E4tKg3C6Iv4s9VK0 の実装設計。3 系統の機構変更と文書追随で
構成する:

1. **`execution.maxTurns` の nullable 化** — schema が `null` を受理し
   「上限なし」を意味する。builtin 既定を `maxTurns: null` に変更
   (ADR-i7la…)。整数設定時の `MAX_TURNS_EXCEEDED` 監視は不変。
2. **stall timeout** — `execution.timeout` を進捗 reset 型の stall
   window へ再意味付けし、built-in 既定を 120s→300s にする(ADR-gEWi…)。
3. **budget 枯渇 checkpoint/resume** — `MAX_TURNS_EXCEEDED`/
   `MAX_TOOL_CALLS_EXCEEDED` で abort する run が途中結果を
   `<userConfigDir>/checkpoints/` へ保存し、`--resume` で続きから
   再開できる(ADR-dQUW…)。
4. **文書** — `transientRetries` の適用範囲・`maxTurns: null`・
   stall 意味論・resume 面を configuration guide と `cheaper-usage`
   skill に明記する。

初稿では「tool gate close による finalization turn」だったが、ユーザー
決定により nullable maxTurns へ置き換えた(2026-10-17)。gate 機構は
不要になり、run 監視は「`maxTurns` が設定されている場合のみ発火」に
単純化される。

## 1. `execution.maxTurns` の nullable 化

### 1a. policy schema・normalize(`cheaper-core`)

`crates/cheaper-core/src/policy/schema.rs`:

- `execution_properties()` の `maxTurns` を
  `{"type": ["integer","null"], "minimum": 1}` に変更(jsonschema
  draft-2020-12 は union type を受理する)。
- `ExecutionPolicy.max_turns` を `Option<u64>` に変更。
  `#[serde(rename = "maxTurns")]` は維持し、`null` は `null` のまま
  serialize する(`skip_serializing_if` は付けない — 正規化出力で
  「未設定」と「null=無制限」を同一視せず explicitness を保つ)。
- `normalize_policy` の `max_turns` 取得を `get_u64` から
  nullable 対応へ変更 — `Null`/`Missing` → `None`、integer → `Some`。
  非 integer 非 null の residual は現行 `get_u64` の `unwrap_or(0)`
  と同じく `Some(0)` 相当へ畳む(schema gate で到達不能だが、
  fail-closed 寄りの従来挙動を変えない)。

### 1b. 既定値(`cheaper-cli`)

`crates/cheaper-cli/resources/policy.yaml`: `execution.maxTurns`
`8` → `null`。

### 1c. run 経路の監視(`cheaper-runtime`)

`crates/cheaper-runtime/src/run/mod.rs`:

- `let max_turns = policy.execution.max_turns;` → `Option<u64>`。
- on_event 内の abort 条件を `Some(limit)` を持つ場合のみ発火する
  形にする:

```rust
if let Some(limit) = max_turns {
    if snap.turns >= limit && snap.last_turn_tool_calls
        && pending_tool_calls == 0
    {
        check_abort.request_abort(RunAbortReason::MaxTurns);
    }
}
```

- post-stream の F1 fallback も同じ `Option` 条件で囲む
  (`maxTurns` 未設定時は一切発火しない)。
- `SessionSpec.policy_max_turns` を `Option<u64>` に変更。
  `RigSessionFactory` の `effective_internal_max_turns` は
  `policy_max_turns.map(usize::try_from ... unwrap_or(usize::MAX))
  ` 形式の saturating 変換を維持したまま `None → internal`
  とする(`p as usize` は 32-bit で truncate するため使わない)
  — `None` でも `DEFAULT_MAX_TURNS=1024` の内部 guard が残る。
- repair 経路の `policy_max_turns` も `Option<u64>` に揃える
  (tool-less session なので maxTurns abort は実質関係しないが、
  内部 guard 計算が共通のため型は一致させる)。
- `cheaper-rig-adapter/src/error.rs` の `MaxTurnsError` message を
  修正 — 現行文言「cheaper policy abort should have fired first」は
  `maxTurns: null` 既定では到達可能な正当 backstop に対して invariant
  違反を装うため不正確になる。「internal turn guard reached (N);
  set `execution.maxTurns` to bound turns earlier」程度へ改める
  (error code は `INTERNAL_ERROR` のまま — 分類変更は contract 影響が
  あるため spec R1.3 のとおり維持)。

### 1d. 影響範囲(型伝播)

- `crates/cheaper-runtime/src/dev/fake_provider.rs`/`fixture.rs`:
  `RecordedSpec.policy_max_turns` を `Option<u64>` へ。
  `FixturePolicy.max_turns`(DSL key `max_turns`)は presence-only
  `Option<u64>` のまま維持 — `Some(v)` だけを policy へ適用する
  現行 apply を変えない(明示 `null` は DSL で表現しない)。
- **dev-seam baseline 決定**: `fixture.rs` の `default_policy()` と
  `run_dev_seam.rs` の `policy()` helper の `max_turns: 10` は
  builtin の新既定に揃えて `None` にする(nullable を既定経路で
  検証するため)。bounded を必要とする既存テストは `max_turns` を
  明示設定する。
- `crates/cheaper-core/src/policy/capability.rs`・`restrict.rs` の
  テスト policy builder: `max_turns: 1` → `Some(1)`。
- `run_dev_seam.rs` の `p.execution.max_turns = N` 系・
  `ollama_e2e.rs` の `max_turns: 16`・`schema.rs` 内の assert 等、
  `Some(N)` への型追従を全件反映(compiler が網羅を強制)。
- golden fixture `crates/cheaper-core/tests/golden/fixtures/policy/
  load.json`: builtin policy embed の `maxTurns: 8` → `null`、
  normalized output の `"maxTurns": 8` → `null`。
- **builtin drift pin**: golden embed は `resources/policy.yaml` の
  snapshot コピーで自動同期しない — `common::builtin_dir()` 経由で
  実 `resources/policy.yaml` を `load_policy` へ渡し
  `max_turns.is_none() && timeout_ms == 300_000` を assert する
  cheaper-cli test を追加し、resource と snapshot の drift を塞ぐ。
- regression/corpus fixture の `policy.maxTurns` を明示しているものは
  整数のまま意味不変。

## 2. stall timeout

### 2a. progress 通知面

`crates/cheaper-runtime/src/run/abort.rs` `RunAbort` に進捗時刻の
共有面を追加:

- 内部に `progress: Arc<AtomicU64>`(arm 時点からの経過 ms、
  `Instant` 基準)を持たせる。
- `RunAbort::progress()` で cloneable な `Progress` handle
  (`note()` メソッドのみ公開)を返す — timeout timer と event
  watcher が共有する最小面。
- 時刻源は `Instant`(dev-seam fixture は実時間 `delay_ms` を使うため
  virtual clock は不要 — `operation_clock` は tool operation 用で
  run timer と別)。

### 2b. deadline の延長ループ

`crates/cheaper-runtime/src/run/boundary.rs` の固定
`tokio::time::sleep(timeout_ms)` を置き換える:

```rust
// arm 直後に last_progress = now を記録してから loop:
//   deadline = last_progress + timeout_ms
//   now >= deadline → request_abort(Timeout) で終了
//   else sleep(deadline - now) で再覚醒(進捗があれば次週期で延長)
```

- 「進捗」= `RunEvent` 全件(text/reasoning delta・ToolCall/Committed/
  Result・CompletionCall・ModelTurnRetried・Finished)。
- 通知点: `drive_prompt` の `on_event` 先頭で `check_abort.progress()
  .note()` を呼ぶ(全 event を通過する既存の一点)。
- repair 経路: `repair.rs` の `sink`(`|_event| {}`)を
  `progress.note()` する closure に変える。`RepairOptions` に
  `progress` handle を追加する(tool-less session でも event は流れる)。
- setup 区間(invocation/model resolve/scope ready)は event を出さない
  ため window は run 開始時点から数える — 現行と同じ「開始から timeout」
  の初期期限を意味し、最小変更に留める。

### 2c. workflow 側の扱い(意味の分岐を明記)

`cheaper workflow run` は同じ `effective_policy.execution.timeout_ms` を
workflow 全体の**絶対期限**として使う(`engine.rs` の `spawn_deadline`
は run 単位でなく workflow 単位の Instant 起点・進捗 reset なし)。
本設計は run 経路のみを stall 化し、workflow の絶対期限は変更しない —
runaway を workflow 内でも止める絶対 ceiling が workflow 全体に残る
構造を保つ。`configuration.md` で「run 経路は stall window、
workflow 経路は絶対期限」と意味の分岐を明記する。

### 2d. 既定値と fixture

- `crates/cheaper-cli/resources/policy.yaml`: `execution.timeout`
  `120s` → `300s`。
- golden fixture: `crates/cheaper-core/tests/golden/fixtures/policy/
  load.json` に embedded された `builtin/policy.yaml` 文字列の
  `timeout: 120s` を同値へ追随(複数 entry)。
- regression fixture `crates/cheaper-cli/tests/fixtures/05-timeout-delay.json`
  系: timeout は fixture 側で小さく override 済みのため意味論変更後も
  stall が発生する構造(進捗なし delay)なら不変で通る。要確認: fixture が
  `emit` で進捗を出し続ける形なら reset が効き timeout しない — その
  場合は stall 経路を別 fixture で pin する。
- `docs/guides/configuration.md` の `timeout` 説明を stall 意味論へ更新。

## 3. budget 枯渇 checkpoint / resume

### 3a. 観測 capture

`crates/cheaper-runtime/src/run/mod.rs` `drive_prompt` の `on_event` で
`RunEvent::ToolCommitted` と `RunEvent::ToolResult` を観測し、
`RunCtx.observations` へ追記する(`drive_prompt`/`classified` は
`&self` のため、`RefCell<Vec<Observation>>` 等の interior
mutability、または `&mut self` 化が必要 — 実装で確定):

- `ToolCall`(model 出力 args)ではなく **`ToolCommitted`**(hook patch
  後の実実行 args)を採用 — invalid-call retry で roll back される
  call の args が混入しない。
- `ToolCommitted`/`ToolResult` は `call_id` で join する。
  `ModelTurnRetried`/`CompletionCall` で pending を evict し、
  result 未到達の call は観測に含めない(既存の
  `pending_tool_calls` reset 規則と同じ)。
- admission 拒否の error result(budget 超過等)は観測に含めない
  (探索の実体ではないため)。
- `result_excerpt` は `ToolResult.output`(`Vec<ToolResultContent>`
  serialized)から text item を join して excerpt 化し cap を設ける。
  `args` も serialize 後に cap — entry 毎 2000 chars 目安、さらに
  **観測列の合計 byte cap(100KB 目安)+ truncation marker** を設け、
  resume 注入 prompt が low-tier context を溢れないようにする。
- repair session は tool-less のため capture 対象外(main session のみ)。

### 3b. checkpoint 保存

`classified()`(mod.rs:202-223)内、`reason` が
`MaxTurns`/`MaxToolCalls` の場合に限り、error を返す前に
checkpoint JSON を保存する:

- 保存先: `<userConfigDir>/checkpoints/<id>.json`。dir 解決は
  `RunWorkerDeps.checkpoint_dir: Option<PathBuf>` を CLI が渡す
  形にし、runtime 層は dir を知らない(workflow 経路は `None` で
  checkpoint を生成しない — run 経路のみの対象)。
- `<id>` は `fs_util::temp_path` 系の pid+nanos pattern で生成
  (新規 `uuid` 依存を増やさない)。
- `create_dir_all(checkpoints)` は保存側で行う(`atomic_write` は
  parent dir を作らない — store.rs 規約に倣う)。
- `unix_mode: Some(0o600)` — 観測に file 内容/command argv が
  入りうるため credential/policy file と同じ権限を pin する。
- format `schemaVersion: 1`:
  ```json
  {
    "schemaVersion": 1,
    "worker": "bulk-reader",
    "input": {...},
    "abortReason": "maxTurns",
    "turns": 8, "toolCalls": 12, "rejectedToolCalls": 0,
    "observations": [{"tool": "grep", "args": {...}, "resultExcerpt": "..."}]
  }
  ```
- **error `details` の付与点**: `PARTIAL_MUTATION` wrap が起きる場合は
  最外層(`PARTIAL_MUTATION` error)の `details` に
  `checkpoint: {path, observationsCount}` を付ける。無 mutate の
  場合は abort error 本体の `details` に付ける。cause 側 details は
  envelope serialization で落ちるため、内側への付与では consumer に
  到達しない — 「envelope 最外層の error」に付けることを spec に
  明記する。`reason`/`changes` 等の既存 key 不変、additive のみ。
  `details` には path + 件数のみ載せ、観測配列本体は file のみ
  (envelope/stdout の巨大化を防ぐ)。
- write 失敗は best-effort: checkpoint 保存失敗で abort error が
  変わらないよう、save は `Result<(), _>` を internal に吸収し
  `details.checkpoint` のみ省略する(stderr warn も出さない —
  caller には error details が contract 面)。
- regression fixture `run-max-turns`/`run-max-tool-calls` は
  `details.checkpoint.path` の run-id 部分が非決定的になるため、
  `tests/regression/normalize.rs` に checkpoint path scrub rule を
  追加し、2 fixture を `CHEAPER_REGRESSION_CAPTURE=1` で再 capture
  する。

### 3c. resume 経路

CLI layer(`cheaper-cli`):

- `cheaper run <worker>`・worker alias(`cheaper bulk-read` 等)に
  `--resume <path>` を追加(`RunCliArgs` は run/alias 共有のため
  1 箇所で済む)。
- `--resume` は run/alias 以外のコマンドでは reject する —
  `option_tokens` の command 別 allowlist(`workflow`/`policy`/
  `hook-config` は既存 reject、残りの `usage`/`count-tokens`/`models`/
  `auth`/`count`/`workflow-run`/`version`/`hook`/`hook-config` も
  同規約で弾く)に `resume` を含めない。
- `--resume` 指定時: checkpoint JSON を load し `schemaVersion == 1`
  と基本型を validate(不正なら `INVALID_INPUT`)。
  `checkpoint.worker` は positional `<worker>` と一致必須、
  `--worker-file` 使用時は resolved `def.name` と比較する。
- input 解決: `option_tokens` の `input` presence で「`--input` が
  明示されたか」を検知する(`workflow` arm の既存 pattern)。
  明示(値 `-` を含む)> checkpoint の stored input の順。
  明示なし時は checkpoint input を使い `read_task_input` の stdin
  read を skip する。
- `RunWorkerDeps.resume: Option<ResumeCheckpoint>` を追加し、CLI は
  checkpoint file の load/validate のみ担う。

runtime layer:

- `drive_prompt` で `deps.resume` があれば user prompt の末尾へ
  continuation section を付加:
  ```
  The previous run was interrupted by <abortReason> after <turns> turns
  and <toolCalls> tool calls. Observations collected so far (JSON):
  <observations array>
  Continue the task from this state and produce the final JSON answer.
  ```
- resume run の session/policy/budget は全て新規(観測注入のみ引き継ぎ)
  — Rig session の中身は serialize しない。invocation `--grant` は
  checkpoint に保持せず引き継がない(authorization の永続化回避 —
  必要なら caller が再度 `--grant` を渡す旨を docs に明記)。
- `--input` で input を変えても前回観測は注入される — 旧 input 由来の
  stale context になり得ることを configuration guide に明記する。

### 3d. 連鎖 resume

resume run が再び `MaxTurns`/`MaxToolCalls` で abort した場合、新規
checkpoint は**前回観測 + 新規観測を累積**して保存する
(`RunCtx.observations` の初期値を `deps.resume.observations` にする)。
累積列にも同じ合計 byte cap を適用し、超過分は古い方から truncate
する(truncation marker を残す)。
これにより `run → abort → resume → abort → resume` の多段継続が成り立つ。

## 4. 文書追随(R3 + R4)

- `docs/guides/configuration.md`:
  - `execution.maxTurns` の説明を `null` 受理・既定は無制限・整数は
    opt-in の bounded turn budget として `MAX_TURNS_EXCEEDED` が発火、
    に更新。
  - `execution.timeout` の説明を stall window 意味論へ更新し、既定値を
    300s に追随。run 経路と workflow 経路(絶対期限)の意味分岐を明記。
  - `retry.transientAttempts` に「completion-call 単位の provider
    transient 失敗(429/5xx/応答なし transport)にのみ適用。run-level
    abort(TIMEOUT/MAX_TURNS_EXCEEDED/ABORTED/MAX_TOOL_CALLS_EXCEEDED)は
    retry 対象外」と明記。
  - stall window は「進捗の最大無言間隔」であり、最長の単一 tool/
    command 実行(`process.timeout`・grant `timeout`)や非 streaming
    completion call より長く設定する必要がある旨を明記。
  - `--resume` と checkpoint(`details.checkpoint`・
    `<userConfigDir>/checkpoints/`・schemaVersion)の説明を追加。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  - `transientRetries` は run-level abort を含まない(provider
    transient のみ)」の一文を追加。
  - `execution.maxTurns` の `null`(既定 = 無制限)と
    `execution.timeout` の stall 意味論を `--timeout`/policy 節に記載。
  - `details.checkpoint` と `cheaper run --resume` の説明を追加。

## テスト方針

dev-seam(`fake_provider`/fixture DSL)で deterministic に検証する:

| # | 内容 | fixture 構成 |
|---|------|-------------|
| T1 | `maxTurns: null` で multi-turn が完走 | `policy.maxTurns` 未設定(=既定 null)。turn(tc)×4 → tool_calls → turn(tc=false, answer) → `ok:true`・`MAX_TURNS_EXCEEDED` 非発火。turns が旧既定(8)を超える 10+ turn 構成で nullable を pin する |
| T2 | `maxTurns` 整数設定で従来の abort | `maxTurns=2`。turn(tc)×2 → tool_calls → turn(tc) で `MAX_TURNS_EXCEEDED`・`details.reason="maxTurns"`(既存 regression と同一構造) |
| T3 | stall reset | `timeout=80ms` 程度の小値。emit/delay を `timeout` 未満間隔で繰り返し総経過 > timeout でも `ok:true`(absolute 経過では死なない) |
| T4 | stall timeout 発火 | emit 後 `delay_ms > timeout` で沈黙 → `TIMEOUT`・`reason="timeout"` |
| T5 | 型・schema 回帰 | `maxTurns` の Option 化で既存 `max_turns` fixture/test が整数・`null` 両経路で成立すること(golden `load.json`・`capability.rs`・`restrict.rs` 等の max_turns 使用箇所を Option で整合) |
| T6 | budget 枯渇 → checkpoint 保存 | `maxTurns=2` で `MAX_TURNS_EXCEEDED` → `<userConfigDir>/checkpoints/<id>.json` が schemaVersion 1・worker・input・observations 含む・**envelope 最外層** error `details.checkpoint.{path,observationsCount}` が返る(mutation 経路は `PARTIAL_MUTATION` の details に付く) |
| T7 | resume run | `--resume` で前回観測が user prompt に注入され、新規 run が完走(`ok:true`)。`--input` 明示(値 `-` 含む)は checkpoint input を上書き |
| T8 | 連鎖 resume | resume run が再度 `MaxTurns` で abort → 新 checkpoint が前回+新規観測を累積 |
| T9 | CLI 面(`--resume` parse/load/validate) | `cheaper run <worker> --resume <path>` の flag parse・checkpoint load・worker 不一致 `INVALID_INPUT`・`--input` presence 検知・`--resume` 非 run command reject を integration test で pin(dev-seam は `RunWorkerDeps` 直組みのため CLI flag 面を検証できない — `config.files` 経由の seeded checkpoint もカバー) |
| T10 | regression fixture 更新 | `run-max-turns`/`run-max-tool-calls` の expected JSON へ `details.checkpoint` を追加、`normalize.rs` に run-id scrub rule を追加 |

`turn` step は RunEvent を emit しないため、stall reset 検証(T3)は
`emit`/`tool_calls` step が必須 — `turn` だけの繰り返しでは progress
reset にならない(dev-seam DSL の `emit` step を使う)。同じく
`usage_delta` と transient retry に吸収される `error` step も
RunEvent を emit しない(retry 経路では実 adapter が
`CompletionCall`/`ModelTurnRetried` を emit して reset されるが、
fake DSL では表現できない点に注意)。T3/T4 は実時間 margin を十分
取る(window 500ms / delay 50ms 程度 — loaded CI の timer jitter 回避)。

## 非対象

- `result` field への partial JSON 埋め込み(worker output schema の
  必須 field を満たせないため別件 — R4 は checkpoint/resume で
  「途中結果の保存」を担う)。
- `execution.maxDuration` 等の絶対 ceiling knob(需要が実測されたら別件)。
- per-worker execution override。
- fake provider / fixture DSL への新 step 追加は不要(既存 `emit`/
  `tool_calls`/`delay_ms`/`turn`/`hang_until_abort` で表現可能)。
- tool gate close / finalization turn 機構(ユーザー決定で不採用)。
- `Timeout`/`Signal` abort の checkpoint 化(観測が少なく意味が薄い —
  Open Items)。
- workflow 内部 job abort の checkpoint(`details` が envelope に
  乗らない構造のため)。
- `<userConfigDir>/checkpoints/` の retention/TTL(需要が実測されたら
  別件 — Open Items)。

## 検証

- `cargo test -p cheaper-core --features cheaper-runtime/dev-seam`
  (policy schema/normalize の `maxTurns` null 受理)
- `cargo test -p cheaper-runtime --features ... --test run_dev_seam`
  (T1–T5)
- `cargo test -p cheaper-cli --features ...`(golden `load.json`・
  regression fixture・T6–T10)
- `mise run check` 全工程

---
id: "DESIGN-034r05iHy2gdnoCKJhKNGD"
type: "design"
status: "approved"
title: "Repair abort and failure normalization hardening"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
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
    - "../designs/pi-execution-engine.md"
    - "../designs/cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
    - "../adr/normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Repair abort and failure normalization hardening

## Context

実 provider smoke（EXP-0002）で repair session が通常経路であることが判明し、レビュー検証（DISC-034r05iHy2gdnp6g5GTWKG）で repair 経路に main session と非対称な不変条件があることが確定した。SPEC-034r05iHK8bA7g9KcsCqKj / ADR-034r05iHrZUE6W2tpzbgKv に基づき、repair に main と同じ abort grace・provider failure 分類を適用し、telemetry・timeout 契約・test 隔離を整合させる。

## Scope

### In Scope

- `session.prompt()` の abort grace（signal abort 後 `graceMs` で待機打ち切り）を main / repair 共通の helper に抽出
- repair の provider failure を `PROVIDER_FAILURE` に正規化（prompt rejection・`stopReason === "error"`）
- repair attempt 消費を `meta.repairs` に失敗・中断時も計上（`onAttempt` コールバック）
- timeout タイマー開始を `runWorker` 先頭へ移し、DESIGN-034r05iHrZUE6WvaQsRRsM を「agent execution timeout」に改訂
- abort grace timer の `clearTimeout` による event loop 保持の解消
- `resolveConfigDirs` が `env.USERPROFILE`/`env.HOME` から homedir を解決するよう修正し、unit test を tmp 隔離
- README 更新と provider 運用ドキュメント（`docs/guides/providers.md`）の追加

### Out of Scope

- CLI 入口 deadline への再設計（ADR-034r05iHrZUE6W2tpzbgKv で agent execution timeout を採用）
- `response_format` の main session 適用、新 error code・worker・policy の追加
- `meta.repairs` 以外の telemetry 変更

## Components and Boundaries

- **`promptWithAbortGrace`（新規、runtime-pi）**: `session.prompt()` と grace timer の race を一箇所に定義する。引数は session・prompt 文字列・集約済み `AbortSignal`・`graceMs`。signal abort 時に `session.abort()` を呼び `graceMs` 後に grace 解決。`prompt()` settle 時は grace timer を `clearTimeout` する。戻り値は `{ kind: "settled" } | { kind: "rejected", err } | { kind: "grace" }`
- **`run.ts`**: `requestAbort` は `abortReason` 記録と `runAbort.abort()` のみに縮退（`session.abort()`・`setTimeout(resolveGrace)` は helper へ移譲）。main prompt は `promptWithAbortGrace` を使う。timeout タイマーは `runWorker` 先頭（`resolvePiModel` 前）で開始し、`session` 未生成でも `runAbort` 経由で機能する。`repairOutput` に `onAttempt` を渡して `repairs` を attempt 消費時点で更新
- **`repair.ts`**: `promptWithAbortGrace` を各 attempt に適用。`unlink`（signal→`session.abort()`）は helper に吸収。prompt rejection → `PROVIDER_FAILURE`、`lastAssistantStop().stopReason === "error"` → `PROVIDER_FAILURE`、`"aborted"` → loop 抜け（呼び出し側の abort 分類に委譲）
- **`lastAssistantStop`**: `run.ts` ローカル関数を共有モジュールへ移し repair からも利用（表現ドリフト回避）
- **`dirs.ts`**: `homedir` 解決を `opts.homedir ?? env.USERPROFILE ?? env.HOME ?? os.homedir()` に変更。`CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME` の優先順位は不変。実環境では `USERPROFILE`/`HOME` が `os.homedir()` と一致するため挙動不変で、env 注入による隔離が有効になる
- **`main.test.ts`**: `makeIo` の既定 `env` を `{ HOME, USERPROFILE, CHEAPER_CONFIG_DIR }` = tmp dir、`cwd` を tmp workspace に変更（e2e helpers と同じ方針）
- **docs**: DESIGN-034r05iHrZUE6WvaQsRRsM の timeout 起点記述を改訂。README の開発状況記述を更新し、`docs/guides/providers.md` に実 provider 設定手順を集約

## Data and Control Flow

1. `runWorker` 開始 → timeout タイマー開始（model 解決・session 構築を含む run 区間を計測）
2. main prompt: `promptWithAbortGrace` が runAbort.signal を監視。abort → `session.abort()` + grace timer → `grace` 解決で待機打ち切り
3. 出力失敗 → `repairOutput`: 各 attempt で `onAttempt(n)` 呼出 → 新規 session → `promptWithAbortGrace`（runAbort.signal を共有、timeout/外部 abort が repair にも伝わる）
4. repair の prompt rejection/`stopReason:"error"` → `PROVIDER_FAILURE`。attempts 使い切り → `INVALID_OUTPUT`（`meta.repairs` は消費回数）
5. abort grace timer は prompt settle で `clearTimeout`（event loop を保持しない）

## Non-Functional Constraints

- abort 不変条件の定義は `promptWithAbortGrace` 一箇所（DRY）。main/repair 以外の将来 session 種別も同 helper を使う
- `PROVIDER_FAILURE` は retryable、`INVALID_OUTPUT`/`INTERNAL_ERROR` は non-retryable。分類の非対称を残さない
- unit test は開発者の user config・repo `.cheaper` に依存しない（hermetic）
- 新規依存なし、公開 API 変更は `RepairOptions.onAttempt`・`resolveConfigDirs` の homedir 解決順のみ

## Risks and Trade-offs

- リスク: `promptWithAbortGrace` への集約で main 側の abort タイミングが変わる → 緩和: `session.abort()` の呼出点を requestAbort から helper の signal listener へ移すのみで、signal 伝播は同期的に同等。既存 abort 系テストで回帰確認
- リスク: `env.USERPROFILE`/`env.HOME` 優先で実環境の homedir 解釈が変わる → 緩和: 実環境では `os.homedir()` と一致するため不変。git-bash 等 `HOME` だけ立つ環境では `USERPROFILE` 優先で従来どおり
- トレードオフ: timeout 起点を run 先頭にすると `createWorkerSession` の遅延も policy timeout に含まれる（意図どおり。CLI 前段は対象外）

## Deferred Design Concerns

- CLI 入口 deadline（stdin 読込・runtime 初期化を含む end-to-end deadline）は将来拡張。親 Agent が全体 deadline を持つ場合に再検討

## Verification Notes

- [ ] repair 中 abort で grace 打ち切りによりハングしない（fake stream が signal を無視するケースのテスト）
- [ ] repair の prompt rejection / `stopReason:"error"` が `PROVIDER_FAILURE` になる
- [ ] repair 失敗時 `meta.repairs` が消費回数になる
- [ ] user config を置いた環境でも `pnpm -r test` が green
- [ ] abort 経路で grace timer が残らず process が遅延なく exit する

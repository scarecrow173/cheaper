---
id: "IMPL-0009"
type: "impl"
status: "completed"
title: "Repair abort grace and provider failure normalization"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/runtime-pi/src/prompt-grace.ts"
      - type: "file"
        path: "docs/guides/providers.md"
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/run.ts"
      - type: "file"
        path: "packages/runtime-pi/src/repair.ts"
      - type: "file"
        path: "packages/runtime-pi/src/output.ts"
      - type: "file"
        path: "packages/core/src/config/dirs.ts"
      - type: "file"
        path: "packages/cli/test/main.test.ts"
      - type: "file"
        path: "packages/core/test/config/dirs.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/run.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/repair.test.ts"
      - type: "file"
        path: "docs/designs/0004-pi-execution-engine.md"
      - type: "file"
        path: "README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0057-extract-promptwithabortgrace-helper-and-apply-to-main-run.md"
    - "docs/tasks/0058-apply-abort-grace-and-provider-failure-normalization-to-repair.md"
    - "docs/tasks/0059-count-consumed-repair-attempts-in-meta-repairs-on-failure.md"
    - "docs/tasks/0060-anchor-policy-timeout-at-runworker-start-and-amend-design-0004.md"
    - "docs/tasks/0061-isolate-unit-tests-from-user-and-project-config-via-env.md"
    - "docs/tasks/0062-update-readme-and-add-provider-setup-guide.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
  experiments:
    adopted: []
    rejected: []
---

# Repair abort grace and provider failure normalization

## Summary

実 provider smoke（EXP-0002）後のコードレビュー指摘 6 件 + 文書指摘 1 件を修正した。`promptWithAbortGrace` を新設して main / repair session の abort grace を共通化し、repair の provider failure を `PROVIDER_FAILURE` に正規化、`meta.repairs` を失敗時も計上、timeout を `runWorker` 開始時点に anchor、grace timer を解放、unit test を tmp 隔離、README と provider ガイドを整備した。

## Context

TASK-0057〜0062（PLAN-0005 / SPEC-0005 / ADR-0010 / DESIGN-0010）に従う。DISC-0003 で指摘を実コード検証済み。

## Implementation

### Changes

- `prompt-grace.ts`（新規）: `session.prompt()` と grace timer の `Promise.race` を一箇所に定義。signal abort 時に `session.abort()` を呼び `graceMs` で `"grace"` 解決。settle 時に `clearTimeout` して event loop を保持しない
- `run.ts`: main prompt を `promptWithAbortGrace` 経由に変更。`requestAbort` は `abortReason` 記録 + `runAbort.abort()` のみに縮退（`session.abort()`・grace timer は helper へ移譲）。timeout タイマーと外部 signal 登録を `runWorker` 先頭へ移動し、model 解決・session 構築を含む run 区間を計測。`repairOutput` に `abortGraceMs`・`onAttempt` を配線し `repairs` を attempt 消費時点で更新
- `repair.ts`: `promptWithAbortGrace` を各 attempt に適用（`unlink` 吸収）。prompt rejection → `PROVIDER_FAILURE`（CheaperError はそのまま再 throw）、`stopReason === "error"` → `PROVIDER_FAILURE`、signal abort 時は extraction を skip して break（分類は呼び出し側）
- `output.ts`: `lastAssistantStop` を `run.ts` から移動して export（main/repair 共用）
- `dirs.ts`: homedir 解決を `opts.homedir ?? env.USERPROFILE ?? env.HOME ?? os.homedir()` に変更（実環境では `os.homedir()` と一致するため挙動不変、env 注入による隔離を有効化）
- `main.test.ts`: `makeIo` が `HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR`/`cwd` を mkdtemp した tmp dir に向ける（e2e helpers と同じ方針）。TTY テストも `makeIo` 経由に統一
- `dirs.test.ts`: env 由来 homedir 解決のテスト追加
- `docs/designs/0004`: timeout 起点を「CLI 入口」→「`runWorker` 開始時点（agent execution timeout、CLI 前段は対象外）」に改訂、abort grace が `promptWithAbortGrace` で main/repair 共用であること・repair の `PROVIDER_FAILURE` 正規化・`meta.repairs` 計上を明記
- `README.md`: 「開発初期」記述を実装済み・実 provider 検証済みに更新、使い方と provider ガイドへのリンク追加
- `docs/guides/providers.md`（新規）: `models.json`/`models.yaml`/`policy.yaml` の配置、`apiKey` リテラル必須、`reasoning_effort: "none"`、`response_format` は repair のみ、小モデル運用の実測知識を集約

### Decisions During Implementation

- **grace timer の扱い**: `setTimeout` の handle を保持し `finally` で `clearTimeout`。`unref()` より deterministic でテストしやすい
- **`stopReason === "aborted"` の repair 側扱い**: `opts.signal?.aborted` なら extraction を skip して break。abort 分類は呼び出し側（run の `abortReason`）の責務で二重分類しない
- **homedir 解決順**: `env.USERPROFILE ?? env.HOME`。Windows では `USERPROFILE` が `os.homedir()` と一致するため優先し、git-bash 等で `HOME` が立つ環境でも従来挙動を維持
- **既に abort 済みの signal**: listener 登録後に `if (signal?.aborted) onAbort()` で取りこぼしを防ぐ（登録前に abort された signal には listener が発火しないため）

## Related Experiments

### Adopted

- `promptWithAbortGrace` による main/repair の abort 不変条件共通化
- repair session の `samplingParams` への `json_schema` 注入（前セッション `38e1e19` で導入済み。tool 無し session のため grammar 制約が安全に機能）

### Rejected

- CLI 入口 deadline への再設計（ADR-0010 で agent execution timeout を採用し、CLI 前段は呼び出し側の責務とした）
- `response_format` の main session 適用（tool call を抑制するため）

## Validation

- `mise exec -- pnpm -F @cheaper/runtime-pi build` / `test`: 136 tests green（新規 5 件: repair prompt rejection → PROVIDER_FAILURE、repair `stopReason:"error"` → PROVIDER_FAILURE、repair 中 abort の grace 打ち切り、repair provider error → PROVIDER_FAILURE（run）、repair 中 hang の grace 打ち切り → TIMEOUT（run）、repair 失敗時 `meta.repairs` 計上）
- `mise exec -- pnpm -F @cheaper/core test`: 233 passed / 2 skipped（dirs の env homedir 解決テスト追加）
- `mise exec -- pnpm -F cheaper test`: 68 tests green
- hermeticity 実証: 実 `~/.config/cheaper/policy.yaml`（timeout 10m）を置いた状態で `pnpm -F cheaper test` が 68 tests green（以前は 2 件失敗していたシナリオ）

## Risks

- `promptWithAbortGrace` への移行で main の abort タイミングが内部構造的に変わった（`session.abort()` の呼出点が `requestAbort` → helper の signal listener）。signal 伝播は同期的に同等だが、既存 abort 系テストの green を以て回帰確認済み
- repair session が `stopReason === "aborted"` を signal 非連動で返す経路（provider 内部 abort）は `INVALID_OUTPUT` 側に倒れる。実用上は runAbort 経由の abort のみを想定

## Follow-ups

- CLI 入口 deadline（stdin 読込・runtime 初期化を含む end-to-end deadline）は将来拡張（ADR-0010 で deferred）

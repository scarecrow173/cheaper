---
id: "IMPL-0004"
type: "impl"
status: "completed"
title: "Code review fixes (post-merge)"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/0004-code-review-fixes.md"
    modified:
      - type: "file"
        path: "packages/core/src/workspace/paths.ts"
      - type: "file"
        path: "packages/core/src/policy/load.ts"
      - type: "file"
        path: "packages/core/src/model/config.ts"
      - type: "file"
        path: "packages/core/test/workspace/paths.test.ts"
      - type: "file"
        path: "packages/core/test/policy/load.test.ts"
      - type: "file"
        path: "packages/core/test/model/config.test.ts"
      - type: "file"
        path: "packages/runtime-pi/src/tools/walker.ts"
      - type: "file"
        path: "packages/runtime-pi/src/tools/grep.ts"
      - type: "file"
        path: "packages/runtime-pi/src/run.ts"
      - type: "file"
        path: "packages/runtime-pi/src/repair.ts"
      - type: "file"
        path: "packages/runtime-pi/src/index.ts"
      - type: "file"
        path: "packages/runtime-pi/test/run.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/tools/grep.test.ts"
      - type: "file"
        path: "packages/cli/src/envelope.ts"
      - type: "file"
        path: "packages/cli/test/envelope.test.ts"
      - type: "file"
        path: "docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md"
      - type: "file"
        path: "docs/designs/0004-pi-execution-engine.md"
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
    - "../../designs/0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "../../designs/0004-pi-execution-engine.md"
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

# Code review fixes (post-merge)

## Summary

マージ後コードレビュー指摘 5 件（High 2・Important 3）を red → 実装 → green で修正した。

- **[High] case-sensitive FS での workspace boundary 迂回**: core に `isInsideWorkspaceRoot(root, candidate, { caseInsensitive })` を集約 export し、`resolveWorkspacePath` は `process.platform === "win32"` のときのみ caseInsensitive で比較。`walker.ts` の重複実装と cycle dedup キーの `toLowerCase()` を除去。
- **[High] repair が global timeout 対象外 + timeout 後 success 経路**: `runWorker` に内部 `runAbort` AbortController を導入し、timeout/外部 signal/maxTurns/maxToolCalls の全 trigger を `requestAbort` → `runAbort.abort()` + `session.abort()` に集約。repair へ `runAbort.signal` を渡し、repair 解決後に `abortReason` を再確認して success を返さない。
- **[Important] optional config の silent skip**: `readPolicyFile`/`readModelsConfig` は `ENOENT`/`ENOTDIR` のみ skip。それ以外（EISDIR・EACCES・壊れた symlink 等）は `INVALID_RUNTIME_POLICY`/`INVALID_MODEL_CONFIG`（details: `{path, reason: "read-failed", errno}`）。
- **[Important] grep regex の event loop ブロック（ReDoS）**: regex 評価のみ `node:worker_threads` の eval 文字列 worker に移し、1 ファイル 5s watchdog（`worker.terminate()` → tool error `"grep pattern evaluation timed out"`）と累計 60s 上限（terminate + `[truncated: evaluation time limit reached]` マーカー）を設けた。
- **[Important] repair usage 未集計 + error envelope meta 全 null**: `repairOutput` に `collector`/`transientRetries` を追加し repair session のイベントを合算。`WorkerRunError`（`runMeta` snapshot 付き CheaperError）を追加し、cli `errorMeta` が各 field を `runMeta` から埋めるようにした。

## Context

- Designs: DESIGN-0002（workspace boundary / sandboxed tools）、DESIGN-0004（実行エンジン・abort/repair/usage）、DESIGN-0005（envelope）
- レビュー指摘は post-merge のため task/plan doc 無し。`relations.implements` は空。

## Implementation

### Changes

- `packages/core/src/workspace/paths.ts` — private `isInsideRoot` を `export function isInsideWorkspaceRoot(root, candidate, { caseInsensitive })` に置き換え。`resolveWorkspacePath` 内で `caseInsensitive = process.platform === "win32"` を 1 回だけ評価し、abs 判定・symlink target 判定の両方に渡す。
- `packages/runtime-pi/src/tools/walker.ts` — 重複 `isInsideRoot` を削除し core の `isInsideWorkspaceRoot` を import（`CASE_INSENSITIVE_FS = process.platform === "win32"` で呼ぶ）。`canonical` cycle キーの `toLowerCase()` を除去（生 realpath を dedup キーに）。
- `packages/core/src/policy/load.ts` / `model/config.ts` — read 失敗を errno で分岐。`ENOENT`/`ENOTDIR` のみ「未配置」として skip（required builtin は従来どおり not-found エラー）、その他は `read-failed` reason + errno 付きで throw。
- `packages/runtime-pi/src/tools/grep.ts` — `GREP_EVAL_WORKER_SOURCE`（自己完結 eval 文字列、`require("node:worker_threads").parentPort` のみ）と `GrepEvalWorker` ラッパーを追加。`execute()` は worker を 1 個 spawn しファイルごとに `{id, display, text}` を postMessage、match 行 index 配列を受け取る。`GrepToolOptions.evalPerFileMs`（既定 5000）超過で terminate + `Error("grep pattern evaluation timed out")`（普通の Error = tool error、`rejectedToolCalls` 非計上）、`evalTotalMs`（既定 60000）累計超過で truncated マーカー。
- `packages/runtime-pi/src/run.ts` — `WorkerRunError`（`runMeta?: Partial<WorkerRunMeta> & { durationMs?: number }`）を追加・export。`runAbort` AbortController を導入し `requestAbort` で `runAbort.abort()` + `session.abort()`。repair に `signal: runAbort.signal`・`collector`・`transientRetries: policy.retry.transientAttempts` を渡す。repair 成功後にも `abortReason` を再確認。`throwClassified`/`withRunMeta` が `buildMeta()` snapshot を `runMeta` に載せる（PARTIAL_MUTATION は `details.changes` と `runMeta.changes` の両方）。
- `packages/runtime-pi/src/repair.ts` — `RepairOptions.collector`/`transientRetries` を追加。各 repair session を `session.subscribe(collector.handleSessionEvent)` に接続し finally で unsubscribe + dispose。`transientRetries` の `0` 固定をやめ opts 経由に。
- `packages/runtime-pi/src/index.ts` — `WorkerRunError` を export。
- `packages/cli/src/envelope.ts` — `errorMeta` が `err.runMeta`（WorkerRunError 等の meta 付き CheaperError）から各 field を埋め、`runMeta.changes` 優先・fallback `details.changes`。`durationMs` は常に CLI 計測値。
- `docs/designs/0002-*.md` — Components 節 grep.ts 行に eval worker + watchdog を 1 行追記、`updated` を 2026-09-16 に。
- `docs/designs/0004-*.md` — 制限の強制節に runAbort 集約の 1 行を追記、`updated` を 2026-09-16 に。

### Decisions During Implementation

- **worker は eval 文字列方式**: `new Worker(code, { eval: true })` でファイル配置不要（`import.meta.url` 非依存、vitest/dist 両対応）。`unref()` せず終了時に必ず `terminate()`（finally）。
- **walk/read/boundary は main thread のまま**: worker には `{display, text}` だけを送り regex 評価のみ逃がす。I/O は interruptible であり ReDoS 対策は評価に限定。
- **累計上限は「評価時間の合算」で判定**: ファイルごとの eval 経過時間を加算し 60s 超で打ち切り。wall-clock ではなく eval 時間そのものを計ることで I/O 待ちを除外。
- **`runMeta` は duck-typing で検出**: envelope 側は `err.runMeta` の object 判定で埋める（WorkerRunError 以外の meta 付き CheaperError も救済、module 複製にも強い）。
- **POSIX-only の case-sibling 統合テスト**: `it.skipIf(process.platform === "win32")` で追加し、case-insensitive FS（macOS 等）では realpath 同一判定で `ctx.skip()`。本開発機は win32 のため skipped。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

全コマンド `mise exec -- pnpm ...`（出力は `tmp/test-output/`）。

- Fix 1 red: `pnpm vitest run test/workspace/paths.test.ts`（core）→ exit 1、`isInsideWorkspaceRoot` 4 tests failed（`fix1-red.txt`）
- Fix 1 green: 同上 → exit 0、20 passed / 2 skipped（`fix1-green-core.txt`）。runtime-pi `walker.test.ts` + `grep.test.ts` → exit 0、20 passed（`fix1-green-runtime.txt`）
- Fix 3 red: `pnpm vitest run test/policy/load.test.ts test/model/config.test.ts`（core）→ exit 1、2 failed（`fix3-red.txt`）
- Fix 3 green: 同上 + paths → exit 0、41 passed / 2 skipped（`fix3-green.txt`）
- Fix 4 red: `pnpm vitest run test/tools/grep.test.ts` → catastrophic pattern で event loop がブロックされハング（bug の再現。watchdog 未実装のため終了せず kill）
- Fix 4 green: 同上 → exit 0、13 passed（`fix4-green.txt`）。watchdog 注入 opts で 50ms timeout → tool error、累計 0 → truncated マーカーを確認
- Fix 2+5 red: `pnpm vitest run test/run.test.ts` → exit 1、3 failed（repair usage 未集計・repair retry 無効・repair 中 timeout 未分類）（`fix25-red.txt`）
- Fix 2+5 green: `pnpm vitest run test/run.test.ts test/repair.test.ts` → exit 0、20 passed（`fix25-green.txt`）。cli `envelope.test.ts` → exit 0、8 passed（`fix25-cli.txt`）
- `pnpm -r test` → exit 0、430 tests（core 232 + skipped 2 / runtime-pi 130 / cli 68）（`full-test.txt`）
- `pnpm biome check .` → exit 0、105 files（`biome-all.txt`）
- `pnpm tsc -b` → exit 0（`tsc-all.txt`）

新規テスト内訳（+16）: paths.test +6（うち POSIX-only 2 は win32 で skip）、load.test +2、config.test +1、grep.test +2、run.test +3、envelope.test +2。

## Risks

- case-sibling の POSIX 統合テストは本機（win32）では skip のため、case-sensitive FS での実動は未検証。純粋関数テスト（`caseInsensitive: false`）でロジックはカバー済み。
- grep worker は `eval: true` の自己完結ソースのため、worker 内コードはテスト経由でのみ検証される（型チェック外）。
- `evalTotalMs` はファイル単位の粒度で打ち切るため、上限ちょうどでの中断ではない（60s を大きく超えることはないが、厳密なリアルタイム deadline ではない）。

## Follow-ups

- POSIX 環境での case-sibling 統合テスト実動確認（CI が win32 のみなら手動）。
- error envelope `meta` の runMeta 拡充を DESIGN-0005 に反映するかは任意（現状は実装先行、design 本文の error meta 記述は「取得不能のみ null」の範囲内）。
- レビュー指摘 #6（`cheaper.input.schema` を必須化するか）はユーザー判断で対応なしと決定（2026-09-16）。任意項目のまま維持する DESIGN-0001 の契約を再確認した。

---
id: "IMPL-0003"
type: "impl"
status: "completed"
title: "Implement cheaper CLI and built-in workers (PLAN-0003)"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "packages/cli/"
      - type: "file"
        path: "packages/runtime-pi/src/testsupport.ts"
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/index.ts"
      - type: "file"
        path: "packages/runtime-pi/src/model-runtime.ts"
      - type: "file"
        path: "packages/runtime-pi/src/run.ts"
      - type: "file"
        path: "packages/runtime-pi/package.json"
      - type: "file"
        path: "packages/runtime-pi/test/fixtures/fake-stream.ts"
      - type: "file"
        path: "tsconfig.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md"
    - "docs/tasks/0040-implement-cli-argument-parsing.md"
    - "docs/tasks/0041-implement-input-reading-from-stdin-or-file.md"
    - "docs/tasks/0042-implement-result-and-error-envelope-output.md"
    - "docs/tasks/0043-implement-human-readable-text-format-output.md"
    - "docs/tasks/0044-add-built-in-worker-json-schemas.md"
    - "docs/tasks/0045-add-built-in-worker-definitions.md"
    - "docs/tasks/0046-implement-cli-main-orchestration.md"
    - "docs/tasks/0047-add-e2e-tests-for-non-execution-cli-paths.md"
    - "docs/tasks/0048-add-fake-stream-hook-and-e2e-execution-tests.md"
    - "docs/tasks/0049-run-full-build-test-and-lint-verification.md"
    - "docs/tasks/0050-write-implementation-record-and-experiment-log.md"
    - "docs/tasks/0051-run-document-status-audit.md"
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
    adopted:
      - "docs/impl/exp/0001-pi-sdk-probe-and-test-stream-seam.jsonl"
    rejected: []
---

# Implement cheaper CLI and built-in workers (PLAN-0003)

## Summary

PLAN-0003（TASK-0039〜0051）を実装した。`packages/cli`（name `cheaper`、bin `./bin/cheaper.js`）を scaffold し、DESIGN-0005 の CLI contract（args/input/envelope/text-format/main orchestration）、DESIGN-0006/0007 の built-in Worker（bulk-reader / cheap-writer）と JSON Schema 4 本、bin 経由の e2e（非実行系 7 件 + fake stream 実行系 4 件）を実装した。stdout は envelope 1 JSON + 末尾改行 1 つのみ、exit code は core ERROR_TABLE どおり。`pnpm -r build && pnpm -r test && pnpm biome check .` 全 pass（416 tests）。

## Context

- Plan: `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- Specs: SPEC-0001（CLI contract・error model）、SPEC-0002（bulk-reader）、SPEC-0003（cheap-writer）
- Designs: DESIGN-0005（envelope・exit code）、0006（bulk-reader contract）、0007（cheap-writer contract）、0008（repo structure）
- 依存: `@cheaper/core`（worker resolve/policy/model config）と `@cheaper/runtime-pi`（`runWorker`）は PLAN-0001/0002 で完成済み。

## Implementation

### Changes

- `packages/cli/package.json` / `tsconfig.json` / `bin/cheaper.js` — name `cheaper`、bin は `#!/usr/bin/env node` + `import "../dist/main.js"`。deps は `@cheaper/core` / `@cheaper/runtime-pi`（workspace）+ `yaml`。
- `src/args.ts` — `util.parseArgs`（strict）。`run <worker>` / `bulk-read` / `cheap-write` エイリアス、`--input`（既定 `-`）/`--worker-file`（run 専用・name と併用不可）/`--cwd`/`--timeout`/`--format json|text`/`--quiet`/`--help`/`--version`。path 風 name は args 層で `INVALID_WORKER_NAME`、その他の引数不正は `INVALID_REQUEST`。
- `src/input.ts` — `--input` file または stdin 読取。TTY なら待たず `INVALID_REQUEST`。JSON object 必須。
- `src/envelope.ts` — success/error envelope（`schemaVersion: 1`）。error は `code`/`exitCode`/`retryable`/`details`/`cause{code,message}`（CheaperError cause のみ）。error meta は取得できた範囲で、未取得 field は null（`PARTIAL_MUTATION` では `details.changes` を `meta.changes` に移す）。
- `src/text-format.ts` — `--format text`。1 行目 `ok`/`error <code>`、worker/model/usage/changes 表、result/error を整形 JSON。
- `src/cli.ts` — オーケストレーション。`runCli(argv, io, hooks)` は io/hooks を注入可能にして in-process で単体テスト可能にした。実行順: args → `canonicalizeWorkspaceRoot` → `resolveConfigDirs` → worker resolve（name/`--worker-file`/alias 固定名）→ input → `validateWorkerInput` → `loadPolicy` → `restrictPolicy(--timeout)` → `loadModelConfig` → ModelRuntime 生成 → `runWorker` → envelope。timeout タイマーは runtime-pi 側（`policy.execution.timeoutMs`）に委譲し、CLI は SIGINT/SIGTERM を AbortController で `deps.signal` に中継するのみ。
- `src/main.ts` — thin entry（`runCli` 呼び出し → `process.exitCode`）。`src/version.ts` — package.json version 読取。
- `workers/bulk-reader.md` / `cheap-writer.md` + `workers/schemas/*.schema.json` 4 本 — DESIGN-0006/0007 の表を draft 2020-12 に verbatim で落とした。path 共通 pattern は `$defs.path`。
- `packages/runtime-pi`: `createModelRuntime` factory を `model-runtime.ts`/`index.ts` に追加（cli が `@earendil-works/*` に直接依存しない構成を維持）。`src/testsupport.ts` に fake stream seam を移し `"./testsupport"` subpath export を追加（test fixture は re-export に変更）。`run.ts` の template literal fix（biome）。
- `test/e2e/` — `helpers.ts`（spawn helper・隔離 env・envelope 純度 assert）、`cli.test.ts`（非実行系 7 件）、`exec.test.ts`（実行系 4 件）、`fixtures/`（function 形 2 本 + array 形 1 本）。

### Decisions During Implementation

- **`createModelRuntime` を runtime-pi 側に置いた**。cli が `@earendil-works/pi-coding-agent` を直接 import しない方針（依存方向の維持）に従い、薄い factory を runtime-pi に追加した。返り値型は `Awaited<ReturnType<typeof createModelRuntime>>` で cli 側が pi 型を名指ししない。
- **`CHEAPER_TEST_FAKE_STREAM` の contract**: default export が関数なら `(ctx: { modelRuntime }) => StreamFnLike` として呼び、fixture 自身が `registerNativeProvider` + `refresh({allowNetwork:false})` する。配列なら cli 側が testsupport の `createFakePi` で wrap して provider 登録する。`NODE_ENV === "test"` 以外では env var を完全に無視する（e2e で assert）。
- **fake model の tier 解決**: e2e では `<ws>/.cheaper/models.yaml` の `overrides: {"cheaper-fake/fake-model": low}` で tier を pin（classifyModel の cost heuristic による medium 落ちを回避）。
- **error envelope の meta**: field 単位で null を許容する形にした（「取得できた範囲・未取得は null」を meta object 内の null field として表現）。`durationMs` は常に計測。
- **スキルスクリプトが CJS bundle で `"type": "module"` 下で直接実行できない**ため、`tmp/skill-scripts/` に `.cjs` コピー + assets 配置で実行した（repo 内の skill 本体は無変更、tmp は gitignore 済み）。

## Related Experiments

### Adopted

- `docs/impl/exp/0001-pi-sdk-probe-and-test-stream-seam.jsonl` — Pi SDK probe（`tmp/pi-probe/`）の知見、faux provider ベースの fake stream seam、`streamFunction` wrap、`\bmini\b` パターン修正、`CHEAPER_TEST_FAKE_STREAM` フック設計を採用した。

### Rejected

なし。

## Validation

- `mise exec -- pnpm -r build` → exit 0（core / runtime-pi / cli 全て `tsc -b` pass）
- `mise exec -- pnpm -r test` → exit 0、416 tests green（core 225 / runtime-pi 125 / cli 66）
- `mise exec -- pnpm biome check .` → exit 0、105 files、no errors
- `mise exec -- pnpm tsc -b` → exit 0
- `node packages/cli/bin/cheaper.js --version` → `0.0.0`、exit 0
- e2e（bin spawn）: `--help`/`--version` exit 0、stdin 無し `INVALID_REQUEST` exit 2、path 風 name `INVALID_WORKER_NAME` exit 3、`--timeout 9999s` `POLICY_VIOLATION` exit 4、未認証 `MODEL_TIER_UNAVAILABLE` exit 5、stdout が envelope 1 行のみ
- e2e 実行系: bulk-read 成功 envelope（`meta.changes.files` 空・`meta.model = cheaper-fake/fake-model tier low`）、cheap-write `meta.changes` 実測非空（`out.txt` created・実ファイル生成確認）、array fixture 受理、`NODE_ENV=production` で env var 無視（`MODEL_TIER_UNAVAILABLE`）
- 実 provider 非依存: e2e は全て fake stream / 非実行系

## Risks

- `bulk-read`/`cheap-write` エイリアスは built-in 名固定のため将来の rename で壊れやすい（DESIGN-0005 記載どおり V1 許容）。
- `PI_CODING_AGENT_DIR`/`PI_OFFLINE`/`CHEAPER_CONFIG_DIR`/`HOME` による e2e 隔離は pi-coding-agent の config 解約順に依存する。SDK 側の env var 仕様変更に弱い。
- 未捕捉例外 → `INTERNAL_ERROR` 70 経路は単体テスト（runWorker seam からの非 Error throw）でカバー。実プロセスでの unhandled rejection は未カバー。

## Follow-ups

- 実 provider での smoke（認証環境での `cheaper bulk-read` 実実行）は手動手順として残る（PLAN-0002 Review Handoff 記載どおり）。
- APM integration（Hook/Skill）は別 spec に deferred。

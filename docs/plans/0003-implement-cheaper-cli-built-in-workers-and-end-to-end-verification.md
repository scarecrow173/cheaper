---
id: "PLAN-0003"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Implement cheaper CLI, built-in workers and end-to-end verification"
created: "2026-09-15"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "docs/specs/0002-bulk-reader-built-in-worker.md"
    - "docs/specs/0003-cheap-writer-built-in-worker.md"
  implemented-by:
    - "../tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md"
    - "../tasks/0040-implement-cli-argument-parsing.md"
    - "../tasks/0041-implement-input-reading-from-stdin-or-file.md"
    - "../tasks/0042-implement-result-and-error-envelope-output.md"
    - "../tasks/0043-implement-human-readable-text-format-output.md"
    - "../tasks/0044-add-built-in-worker-json-schemas.md"
    - "../tasks/0045-add-built-in-worker-definitions.md"
    - "../tasks/0046-implement-cli-main-orchestration.md"
    - "../tasks/0047-add-e2e-tests-for-non-execution-cli-paths.md"
    - "../tasks/0048-add-fake-stream-hook-and-e2e-execution-tests.md"
    - "../tasks/0049-run-full-build-test-and-lint-verification.md"
    - "../tasks/0050-write-implementation-record-and-experiment-log.md"
    - "../tasks/0051-run-document-status-audit.md"
  depends-on:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "docs/specs/0002-bulk-reader-built-in-worker.md"
    - "docs/specs/0003-cheap-writer-built-in-worker.md"
    - "docs/designs/0005-cli-contract-envelope-exit-codes.md"
    - "docs/designs/0006-bulk-reader-contract.md"
    - "docs/designs/0007-cheap-writer-contract.md"
    - "docs/designs/0008-repository-structure-toolchain.md"
    - "docs/adr/0004-one-shot-json-first-cli.md"
    - "docs/adr/0006-wrapper-measured-deterministic-metadata.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement cheaper CLI, built-in workers and end-to-end verification Implementation Plan

## Goal

`cheaper` CLI と built-in Worker（bulk-reader / cheap-writer）を DESIGN-0005/0006/0007 のとおり実装し、bin 経由の e2e で envelope・exit code・stdout 純度を検証し、`docs/impl` に実装記録を残す。

## Scope

1 つの実装ストリーム（`packages/cli` + built-in workers + e2e + 実装記録）。境界は「`bin/cheaper.js` から envelope 出力まで」。Non-goal: APM integration（Hook/Skill）は別 spec に deferred（SPEC-0001 の記載どおり）。

共通事項:

- 全コマンドは `mise exec -- pnpm ...` 経由。
- 実装は引継ぎ §41 の順序（core → runtime-pi → cli）で、Pi integration から書き始めない（PLAN-0002 完了後に着手）。

## Upstream Documents

- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`
- `docs/specs/0002-bulk-reader-built-in-worker.md`
- `docs/specs/0003-cheap-writer-built-in-worker.md`
- `docs/adr/0004-one-shot-json-first-cli.md`
- `docs/adr/0006-wrapper-measured-deterministic-metadata.md`

## Design Inputs

- `docs/designs/0005-cli-contract-envelope-exit-codes.md`
- `docs/designs/0006-bulk-reader-contract.md`
- `docs/designs/0007-cheap-writer-contract.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## File Map

- `packages/cli/package.json` - name `cheaper`、`bin: { cheaper: "./bin/cheaper.js" }`、deps: `@cheaper/core`, `@cheaper/runtime-pi`（workspace）, `yaml`
- `packages/cli/tsconfig.json` - base 継承
- `packages/cli/bin/cheaper.js` - `#!/usr/bin/env node` → `../dist/main.js` import
- `src/args.ts` - `util.parseArgs`。`run`/`bulk-read`/`cheap-write`、options 一覧は DESIGN-0005
- `src/input.ts` - stdin/`--input` 読取、TTY で待たない、JSON object 必須
- `src/envelope.ts` - success/error envelope 組み立てと stdout 出力
- `src/text-format.ts` - `--format text`
- `src/main.ts` - オーケストレーション、timeout タイマー、SIGINT/SIGTERM、未捕捉例外 → `INTERNAL_ERROR` 70
- `workers/bulk-reader.md`, `workers/cheap-writer.md` - built-in Worker 定義
- `workers/schemas/bulk-reader.input.schema.json`, `bulk-reader.output.schema.json`, `cheap-writer.input.schema.json`, `cheap-writer.output.schema.json` - DESIGN-0006/0007 の表を JSON Schema 2020-12 に落とす。path pattern は `$defs`
- `test/args.test.ts`, `test/envelope.test.ts`, `test/text-format.test.ts`
- `test/workers.test.ts` - built-in md が core validate を通る・tools が設計どおり・schema が `validateSchema` を通る
- `test/e2e/cli.test.ts` - `node bin/cheaper.js` を `execFile` で起動。`--help`/`--version` exit 0; stdin 無し → `INVALID_REQUEST` exit 2 で stdout が envelope 1 行のみ; 不正 worker 名 → exit 3; `--timeout` 拡張 → `POLICY_VIOLATION` exit 4; 未認証環境 → `MODEL_TIER_UNAVAILABLE` exit 5。実行系は runtime-pi の `stream` seam を環境変数 `CHEAPER_TEST_FAKE_STREAM=<fixture path>` で注入する test-only フック経由（`NODE_ENV=test` 以外では無効化）で bulk-read の成功 envelope と cheap-write の `meta.changes` 実測を e2e 化
- `docs/impl/` - 実装記録スキルに従い implementation record（ir）1 本と experiment log（exp）を本 plan の最終ステップで作成（PLAN-0001/0002 を含めた V1 全体の記録）

## Implementation Sequence

- [ ] 1. `packages/cli` scaffold + bin 起動 `--version`
- [ ] 2. `args.ts`
- [ ] 3. `input.ts`
- [ ] 4. `envelope.ts`
- [ ] 5. `text-format.ts`
- [ ] 6. built-in schemas（4 ファイル）
- [ ] 7. built-in worker md（2 ファイル）
- [ ] 8. `main.ts`（core resolve → policy → model → runtime-pi run → envelope）
- [ ] 9. e2e 非実行系（help/version/INVALID_REQUEST/INVALID_WORKER_NAME/POLICY_VIOLATION/MODEL_TIER_UNAVAILABLE）
- [ ] 10. `CHEAPER_TEST_FAKE_STREAM` フック + e2e 実行系（bulk-read 成功 envelope・cheap-write `meta.changes`）
- [ ] 11. `pnpm -r build && pnpm -r test && pnpm biome check .` 全 pass
- [ ] 12. `docs/impl` 実装記録（ir + exp）
- [ ] 13. `audit_docs.js`（doc-status audit）

## Dependency Graph

| ステップ | 依存先 | 並列可能 |
| --- | --- | --- |
| 1 scaffold + bin | PLAN-0002 | - |
| 2 args | 1 | 3,4,5,6 と並列 |
| 3 input | 1 | 2,4,5,6 と並列 |
| 4 envelope | 1 | 2,3,5,6 と並列 |
| 5 text-format | 1 | 2,3,4,6 と並列 |
| 6 schemas | 1 | 2,3,4,5 と並列 |
| 7 worker md | 6 | - |
| 8 main | 2,3,4,5,7 | - |
| 9 e2e 非実行系 | 8 | - |
| 10 e2e 実行系 | 9 | - |
| 11 全検証 | 10 | - |
| 12 impl 記録 | 11 | - |
| 13 doc audit | 12 | - |

## Task Granularity

各「実装単位」は必ず次の 5 ステップに分解する: (1) 失敗するテストを書く (2) 対象テストを実行して red を確認 (3) 最小実装 (4) 再実行で green (5) `pnpm biome check` と `tsc -b` が pass。task-doc 化するときはこの単位を 1 task にする。

## Dependencies and Risks

### Risk Register

| リスク | 発生確率/影響 | 対策 |
| --- | --- | --- |
| 未認証環境での e2e が `MODEL_TIER_UNAVAILABLE` で止まる | 高/中 | `CHEAPER_TEST_FAKE_STREAM` フックで実行系を検証 |
| test-only フックが本番に漏れる | 低/高 | `NODE_ENV` ガード + フック無効時の e2e で環境変数を与えても無視されることを assert |
| breaking-change: envelope `schemaVersion` | 低/中 | `schemaVersion: 1` を固定（新規） |
| data-loss | なし | e2e は `fs.mkdtemp` 内のみ |
| 外部依存 availability: Node 26 `util.parseArgs` | 低/低 | 安定 API |

## Rollback Strategy

ファイルシステム外・DB・公開 API・マイグレーション無し。irreversible step は無い。rollback = 該当コミットの `git revert`。検証は `pnpm -r build && pnpm -r test` が green であること。e2e で実 filesystem を触るテストは vitest の tmp dir（`fs.mkdtemp`）内に限定し、repo を汚さない。

## Verification

### Verification Matrix

| 対象 | 検証 | pass 条件 |
| --- | --- | --- |
| SPEC-0001 CLI: `run`/`bulk-read`/`cheap-write`・`--worker-file` 併用不可・path 風 name → `INVALID_WORKER_NAME` | `pnpm --filter cheaper test -- args` | test green |
| SPEC-0001 stdin: TTY 未指定で待たず `INVALID_REQUEST` exit 2・JSON object 必須 | `pnpm --filter cheaper test -- input` + e2e | test green |
| SPEC-0001 stdout: envelope 1 行のみ・末尾改行 1 つ・内部イベント混入なし | e2e で stdout を `JSON.parse` して 1 オブジェクト | e2e green |
| SPEC-0001 stderr: 診断のみ・`--quiet` で警告抑止（error は出る）・TTY 判定なし | `pnpm --filter cheaper test -- envelope` + e2e | test green |
| SPEC-0001 exit code: 対応表どおり・未捕捉例外 → `INTERNAL_ERROR` 70 | e2e | e2e green |
| SPEC-0001 envelope: success/error の schemaVersion/error.code/exitCode/retryable/meta | `pnpm --filter cheaper test -- envelope` | test green |
| SPEC-0001 `--format text`: 1 行目 `ok`/`error <code>` + 表 + 整形 JSON | `pnpm --filter cheaper test -- text-format` | test green |
| SPEC-0002: bulk-reader read-only（`meta.changes.files` 空）・tools read/grep/find/ls・入出力 schema 適合 | `pnpm --filter cheaper test -- workers` + e2e bulk-read | test green |
| SPEC-0003: cheap-writer changed files は `meta.changes` 実測・abort 時 `PARTIAL_MUTATION`・rollback なし | `pnpm --filter cheaper test -- workers` + e2e cheap-write | test green |
| test-only フックの本番無効化 | e2e で `NODE_ENV` 非 test + env var 指定時に無視されることを assert | e2e green |
| 全体 | `pnpm -r build && pnpm -r test && pnpm biome check .` | 全 green |

## Task Breakdown

Implementation Sequence の各番号が 1 task（内部は Task Granularity の 5 ステップ）。並列可能ブロックは Dependency Graph の「並列可能」列どおり。

## Open Questions / Gaps

- APM integration（Hook/Skill）は別 spec に deferred（SPEC-0001 の記載どおり、本 plan の Non-goal）

## Self-Review

- [x] Every spec requirement maps to a task.（SPEC-0001 CLI/stdout/exit code・SPEC-0002/0003 受け入れ条件を Verification Matrix に対応付け）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated / subagent-capable 実装を推奨
- Independent work streams: Dependency Graph の並列可能行（{2,3,4,5,6}）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮定: `CHEAPER_TEST_FAKE_STREAM` は test-only フックで `NODE_ENV=test` 以外では無効（本番に絶対に出さないことを Verification Matrix で担保）
- レビュー重点: e2e の stdout 純度 assert（envelope 1 行 + 末尾改行 1 つ）、built-in schema が DESIGN-0006/0007 の制約値と一致するか
- `docs/impl` の記録は実装記録スキルの手順に従う

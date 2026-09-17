---
id: "TASK-0092"
type: "task"
status: "done"
title: "Bulk-reader guidance, deny/registry parity, and final verification"
created: "2026-09-17"
updated: "2026-09-17"
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
  implements:
    - "docs/plans/0014-bulk-reader-huge-file-safe-io.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0014-bulk-reader-huge-file-safe-io.md"
    - "docs/tasks/0090-bounded-read-tool-and-registry-swap.md"
    - "docs/tasks/0091-streaming-grep-batch-protocol-and-bounded-footer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0019-bulk-reader-huge-file-safe-io.md"
    - "docs/specs/0014-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Bulk-reader guidance, deny/registry parity, and final verification

## Work

PLAN-0014 の仕上げ task。bulk-reader worker への guidance 追記、
deny 存在秘匿と read-only の parity テスト追加、そして plan の
verification matrix 全項目の実行と記録を行う。

### 1. `packages/cli/workers/bulk-reader.md`（guidance 追記）

`## Procedure` に以下の趣旨を追記する:

- `read` は `offset`/`limit` で範囲を絞り、巨大 file や長大行は
  trailer の `Use offset=N` / `Use offset=E byteInLine=C` に従って
  継続読み取りする。`byteInLine` は長大行の途中再開用であること。
- `grep`/`find` の `[skipped: binary/unreadable/oversized-line —
  path(:line)]` footer は「未検索・未評価があり」を意味し、
  `(no matches)`（検索済みだが一致なし）と必ず区別して解釈する。
  `[truncated: ...]` 系は部分結果である旨を併記する。
- 明示 error（binary/image/`read does not support image files` 等）を
  受けた場合の扱いも簡潔に触れる。

tool 名・schema 宣言（frontmatter の `tools:`/`input`/`output`）は
不変。`.apm/hooks/cli/workers/` 等の配布物は bundle/apm install の
生成物であり手編集しない。

### 2. `test/tools/scope-deny.test.ts`（deny 秘匿の回帰強化）

PLAN-0014 Risk #5 の対応。deny 対象が新しい報告経路に漏れない
ことを assert する:

- `grep`/`find` の結果に加えて `[skipped: ...]` footer 行と
  `and N more` の集計件数にも deny path が一切出現しないこと
  （deny 件数を数えて漏らす経路がないこと）。
- read-only の維持: read/grep/find/count_tokens の各 tool 呼出し
  前後で `journal.finalize()` の差分がないこと（write 系 syscall
  を呼ばない新規コードの回帰確認。PLAN-0014 Risk #10）。
- 直接指定 path の boundary/policy 違反は tool error（unreadable
  として飲み込まない）であることを確認。

### 3. `test/tools/registry.test.ts`（parity 追加）

TASK-0090 が更新した `read tool reads via boundary ops` 系の
assert に対し、新 read tool 契約での parity 確認を追加する
（boundary/deny 経路が `createReadTool` でも既存挙動を維持する
こと）。同一 file は TASK-0090 が先行で更新済みのため、本 task は
追加のみ（sequential 制約。PLAN-0014 Dependency Graph）。

### 4. 最終検証

PLAN-0014「Verification commands（全 task 共通の最終ゲート）」の
全コマンドを実行し、結果を本 task doc（または impl record）に記録
する:

```bash
mise exec -- pnpm -r build
mise exec -- pnpm --filter @cheaper/runtime-pi test
mise exec -- pnpm --filter cheaper test
mise exec -- pnpm lint
mise exec -- pnpm --filter cheaper exec tsc --noEmit -p .
mise exec -- node .claude/skills/doc-status/scripts/audit_docs.js --type all
mise exec -- node scripts/bundle-cli.mjs
```

- `node scripts/bundle-cli.mjs` は committed 生成物
  （`.apm/hooks/cli/bin/cheaper.mjs` 等）の freshness 確認を兼ねる。
  runtime-pi tool 層の変更で bundle 内容が変わる場合は再生成して
  一致させる。
- 既存 suite の parity: `count_tokens` の 16MB cap error と cap 未満
  canonical count、親 hook の exit 2 + 委譲 guidance の既存検証
  （`packages/cli` e2e 等）が無変更で pass することを suite 全体で
  確認する。
- 実 provider / CLI / hook の検証は TASK-0086/0087 と同様に一時
  ディレクトリへ `apm install` した隔離環境でのみ行う。本
  リポジトリ内の bundled cheaper CLI・`.apm/hooks`・cheaper-*
  skill による自己委譲・検証は実行しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0014-bulk-reader-huge-file-safe-io.md`
- `docs/designs/0019-bulk-reader-huge-file-safe-io.md`
- `docs/specs/0014-bulk-reader-huge-file-safe-io.md`

## Dependencies

- `docs/tasks/0090-bounded-read-tool-and-registry-swap.md`
  （新 read tool 契約が registry.test.ts parity 追加の前提）
- `docs/tasks/0091-streaming-grep-batch-protocol-and-bounded-footer.md`
  （footer・skip 経路が scope-deny assert の前提）

## Affected Files

- `packages/cli/workers/bulk-reader.md`
- `packages/runtime-pi/test/tools/scope-deny.test.ts`
- `packages/runtime-pi/test/tools/registry.test.ts`
- `.apm/hooks/cli/bin/cheaper.mjs`（bundle 再生成が必要な場合のみ）

変更しない file（明示）: `packages/runtime-pi/src/tools/` 配下の
実装 file（0089–0091 で確定済み）、`.apm/hooks/` の hook 本体・
worker 配布物の手編集、`count-tokens.ts`、`packages/cli/test/e2e/`
配下。

## Done When

- [x] `bulk-reader.md` に `offset`/`byteInLine` 継続と footer /
      `(no matches)` の区別が記述され、frontmatter 宣言は無変更
- [x] `scope-deny.test.ts` が結果・`skipped:` 行・`and N more`
      件数への deny 非出現と journal 差分なしを assert する
- [x] `registry.test.ts` に新 read tool 契約の parity assert が
      追加されている
- [x] 上記 verification matrix の全コマンドが pass し結果が記録
      されている（apm install e2e の失敗は apm-cli を lock 生成版
      0.28.0 に pin して解消 — 下記 Verification results）

## Verification

- [x] `mise exec -- pnpm -r build` が exit 0
- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test` が exit 0
      （21 files / 255 tests pass — review fix 後の最終値）
- [x] `mise exec -- pnpm --filter cheaper test` が exit 0
      （13 files / 126 tests pass — apm-cli 0.28.0 pin 後）
- [x] `mise exec -- pnpm lint` が exit 0
- [x] `mise exec -- pnpm --filter cheaper exec tsc --noEmit -p .` が exit 0
- [x] `node .claude/skills/doc-status/scripts/audit_docs.js --type all` が exit 0
      （177 files / no findings）
- [x] `mise exec -- node scripts/bundle-cli.mjs` が exit 0 で
      生成物を再生成・commit 済み

## Verification results

2026-09-17/18 実施（Windows / node v26.8.2 / pnpm 11.0.0）。
以下は code review fix（traversal order・UTF-8 cap 境界・
batch hard cap・split parity）適用後の最終 head での実測値。

| Command | Result |
|---------|--------|
| `mise exec -- pnpm -r build` | exit 0（core / runtime-pi / cli tsc -b 全 Done） |
| `mise exec -- pnpm --filter @cheaper/runtime-pi test` | exit 0（Test Files 21 passed / Tests 255 passed） |
| `mise exec -- pnpm --filter cheaper test` | exit 0（Test Files 13 passed / Tests 126 passed） |
| `mise exec -- pnpm lint` | exit 0（biome check、131 files / no errors） |
| `mise exec -- pnpm --filter cheaper exec tsc --noEmit -p .` | exit 0 |
| `mise exec -- pnpm --filter @cheaper/runtime-pi exec tsc --noEmit -p .` | exit 0 |
| `node .claude/skills/doc-status/scripts/audit_docs.js --type all` | exit 0（177 files / no findings） |
| `node scripts/bundle-cli.mjs` | exit 0（`.apm/hooks/cli/bin/cheaper.mjs` 再生成・commit 済み） |

`pnpm --filter cheaper test` は初回実施時に apm install e2e が
失敗していたが、原因は repo コードではなく apm-cli の version
drift（mise `latest` 解決で 0.30.0 が入り、lock 生成版 0.28.0 と
不整合。0.30.0 は Windows の path dep 解決が WinError 3 で壊れて
いた + mise shim が repo 外 cwd で binary を解決不能）。
`mise.toml` で `pipx:apm-cli` を 0.28.0 に pin し `mise.lock` を
追加した後は 126/126 pass（apm install e2e 8 件・
delegation-loop e2e を含む）。同じ run で観測された timeout /
EBUSY の flake は単独再実行で全 pass。

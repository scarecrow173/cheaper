---
id: "PLAN-0014"
type: "plan"
status: "approved"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Bulk-reader huge-file safe I/O"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
  implemented-by:
    - "../tasks/0089-shared-limits-line-framer-and-path-only-walker-find.md"
    - "../tasks/0090-bounded-read-tool-and-registry-swap.md"
    - "../tasks/0091-streaming-grep-batch-protocol-and-bounded-footer.md"
    - "../tasks/0092-bulk-reader-guidance-parity-and-final-verification.md"
  depends-on:
    - "../plans/0012-huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../plans/0013-cross-agent-distribution-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
    - "../adr/0017-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0019-bulk-reader-huge-file-safe-io.md"
  derived-by: []
  verifies:
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-0014: Bulk-reader huge-file safe I/O

## Goal

bulk-reader の sandboxed `read` / `grep` / `find` を DESIGN-0019 の
契約どおり有界化する。`read` は独自 bounded read tool
（`offset`/`limit`/`byteInLine`、返却 ≤ `READ_MAX_BYTES`）、`grep` は
固定 chunk 読み取り + 上限付き行フレーマー + batch protocol の
streaming 評価、`find` は path-only 列挙へ移し、skip/不完全性を有界
footer で報告する。作業メモリは入力ファイル総 byte 数 N に比例しない
（O(N) 禁止）。

## Scope

単一 implementation stream。`packages/runtime-pi/src/tools/` の
sandboxed tool 層とその test、`packages/cli/workers/bulk-reader.md` の
guidance に限定され、4 task（TASK-0089〜0092）で完結する規模のため
plan 分割はしない。

### In Scope

- 新規 `tools/limits.ts`（共有上限定数）、`tools/line-framer.ts`
  （上限付き行フレーマー）、`tools/read.ts`（bounded read
  `defineTool`）
- `tools/walker.ts` の path 列挙 / text eligibility 分離
  （`onIssue`・`sniffTextFile`）
- `tools/grep.ts` の streaming 化（batch protocol・有界 footer・
  scan deadline・cancellation）
- `tools/find.ts` の path-only 化と列挙失敗 footer
- `registry.ts` の read mapping 置き換え、`operations.ts` の
  `createReadOperations` 削除、tool description 更新
- `packages/runtime-pi/test/tools/` の境界・有界性・parity テスト
- `packages/cli/workers/bulk-reader.md` の継続読み取り / footer 解釈
  guidance

### Out of Scope

- `count_tokens` の既定 16MB cap、`EditOperations` の全文
  `readFile`、親側 `pre-read-code` hook の block 契約
- bulk-reader の output schema・excerpt 制約（DESIGN-0006 のまま）
- 画像読み取りの代替経路（明示 error のみ。再導入には同じ有界性を
  証明する別契約が必要）
- 依存追加、新しい環境変数、外部プロセス（`rg`/`fd`/shell）
- task doc 自体の作成（本 plan では TASK-0089〜0092 を計画するのみ。
  task doc 作成は plan 承認後の follow-up step）

## Upstream Documents

- `docs/specs/0014-bulk-reader-huge-file-safe-io.md`（proposed —
  acceptance criteria は本 plan の verification matrix に対応付け済み。
  status が proposed のままである点は Review Handoff に記録）
- `docs/adr/0017-bulk-reader-huge-file-safe-io.md`（proposed — 方式
  選定と制約の根拠。数値確定は DESIGN-0019 に委譲済み）
- `docs/discovery/0010-bulk-reader-huge-file-safe-io.md`（調査起点）

## Design Inputs

- `docs/designs/0019-bulk-reader-huge-file-safe-io.md`（approved —
  本 plan の直接の実装対象。上限定数・schema・batch protocol・
  メモリ予算・test plan はすべてここに確定済み）
- DESIGN-0002 / ADR-0007: workspace boundary・symlink policy の既存契約
- DESIGN-0006: bulk-reader の read-only・excerpt 契約（維持対象）
- SPEC-0010: privacy deny の存在秘匿（footer/件数への非漏洩）

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `packages/runtime-pi/src/tools/limits.ts` | 共有上限定数（C/R/L/B/K/O/S・eval/deadline・footer 上限・worker heap cap） | TASK-0089。新規。schema `maximum` で固定、options 注入で縮退のみ可 |
| `packages/runtime-pi/src/tools/line-framer.ts` | `createLineFramer` — raw Buffer 上の `0x0A` 走査、L 超過行の破棄 + `onOversized`、`flush()` 未終端行 emit | TASK-0089。新規。UTF-8 安全（byte 基準） |
| `packages/runtime-pi/src/tools/walker.ts` | `walkFiles` を path 列挙専念化 + `onIssue` 通知 + `sniffTextFile` export | TASK-0089。`MAX_FILE_BYTES`/`isEligibleFile`/`isBinaryFile` を yield 判定から除去。deny filter・skip dir・cycle 防止は維持 |
| `packages/runtime-pi/src/tools/find.ts` | path-only 化 + `onIssue` footer + description 更新 | TASK-0089。内容 `open`/`read`/`sniff` を一切行わない |
| `packages/runtime-pi/src/tools/read.ts` | bounded read tool（`offset`/`limit`/`byteInLine`、`ReadToolDetails`、trailer、binary/image 明示 error） | TASK-0090。新規。境界検証は count-tokens と同一経路（`resolveWorkspacePath` + `rethrowAsDenied` + `denylist.isDenied`） |
| `packages/runtime-pi/src/tools/registry.ts` | `case "read"` を `createReadTool(opsOpts)` に置き換え、Pi read import 除去 | TASK-0090。他 tool mapping は不変 |
| `packages/runtime-pi/src/tools/operations.ts` | `createReadOperations` 削除 | TASK-0090。write/edit/ls operations は不変 |
| `packages/runtime-pi/src/tools/grep.ts` | streaming 書き換え（chunk read + framer + batch protocol + ring/pending context + footer + deadline/abort cleanup） | TASK-0091。worker `file` メッセージ → `batch`、`resourceLimits.maxOldGenerationSizeMb = 64` |
| `packages/cli/workers/bulk-reader.md` | `offset`/`byteInLine` 継続・footer/`(no matches)` 区別の guidance 追記 | TASK-0092。tool 名・schema 宣言は不変。`.apm/hooks/cli/workers/` 等の配布物は手編集しない |
| `packages/runtime-pi/test/tools/read.test.ts` | bounded read の有界性・境界・継続・明示 error | TASK-0090。新規 |
| `packages/runtime-pi/test/tools/line-framer.test.ts` | framer 単体（chunk 跨ぎ行復元・oversized 破棄・行番号消費・flush） | TASK-0089。新規。design File Map 未記載の追加（純粋 helper の単体検証用。Gap 参照） |
| `packages/runtime-pi/test/tools/walker.test.ts` | eligibility 移動に伴う更新 + `onIssue`/`sniffTextFile` 検証 | TASK-0089。`skips files larger than 1MB`/`skips binary files` は責務移動先（grep 側）へ書き換え |
| `packages/runtime-pi/test/tools/find.test.ts` | path-only 列挙（巨大/binary file を返す）+ footer + 内容 I/O spy | TASK-0089 |
| `packages/runtime-pi/test/tools/operations.test.ts` | `createReadOperations` 削除に伴う更新 | TASK-0090 |
| `packages/runtime-pi/test/tools/registry.test.ts` | `read tool reads via boundary ops` 等を新 tool 契約に更新 | TASK-0090 が先行で更新、TASK-0092 が parity 確認を追加（同一 file のため sequential） |
| `packages/runtime-pi/test/tools/grep.test.ts` | streaming/batch・footer・deadline/cancellation・境界跨ぎ一致・メモリ instrumentation | TASK-0091 |
| `packages/runtime-pi/test/tools/scope-deny.test.ts` | deny 存在秘匿の回帰（結果・footer・`and N more` 件数への非出現） | TASK-0092 |

変更しない file（明示）: `packages/core/src/token/count.ts`、
`packages/runtime-pi/src/tools/count-tokens.ts`、`operations.ts` の
edit 系、`packages/cli/test/e2e/pre-read-hook.test.ts`、
`.apm/hooks/**`、`packages/cli/workers/` の schema 宣言。

## Implementation Sequence

- [ ] TASK-0089: `limits.ts`・`line-framer.ts` を新規作成し、
      `walker.ts` を path-only + `onIssue` + `sniffTextFile` に分割、
      `find.ts` を path-only + footer に更新。walker/find/framer の
      test を追加・更新し、eligibility 検証を責務移動先へ合わせる
- [ ] TASK-0090: `read.ts`（bounded read）を新規作成し、
      `registry.ts` の read mapping を置き換え、`operations.ts` から
      `createReadOperations` を削除。`read.test.ts` を新規作成し、
      `operations.test.ts`・`registry.test.ts` を更新
- [ ] TASK-0091: `grep.ts` を streaming + batch protocol + 有界
      footer + scan deadline/abort cleanup へ書き換え、
      `grep.test.ts` を更新（batch 境界・context merge・
      in-flight ≤1・eval 累積予算・oversized-line footer）
- [ ] TASK-0092: `bulk-reader.md` guidance 追記、scope-deny /
      registry parity テスト、design Verification Notes の全項目と
      monorepo 全体検証を実施

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| TASK-0089（shared limits/framer/walker/find） | — | TASK-0090, TASK-0091 |
| TASK-0090（bounded read + registry swap） | TASK-0089（`limits.ts` の定数と `sniffTextFile` 相当の bounded 判定を利用） | TASK-0092 |
| TASK-0091（streaming grep） | TASK-0089（`limits.ts`・`line-framer.ts`・`sniffTextFile`・`onIssue`） | TASK-0092 |
| TASK-0092（guidance + parity + 最終検証） | TASK-0090, TASK-0091 | — |

- critical path: TASK-0089 → {TASK-0090, TASK-0091} → TASK-0092。
- TASK-0090 と TASK-0091 は触る file が disjoint（`read.ts`/
  `registry.ts`/`operations.ts` vs `grep.ts`/`grep.test.ts`）のため
  論理上は並列化可能。ただし同一 package（`packages/runtime-pi`）内の
  変更であり、単一 worktree での sequential 実行を推奨する。
  `registry.test.ts` は TASK-0090 と TASK-0092 の双方が触れるため
  この 2 task 間も必ず sequential とする。
- `packages/runtime-pi/src/tools/` の file を共有する task（本 plan の
  全 task）は同一 worktree で sequential に実施する。並列 worktree
  への fan-out は行わない。

## Task Breakdown

- TASK-0089: Shared limits, line framer, and path-only walker/find
  （`docs/tasks/0089-shared-limits-line-framer-and-path-only-walker-find.md`）
- TASK-0090: Bounded read tool and registry swap
  （`docs/tasks/0090-bounded-read-tool-and-registry-swap.md`）
- TASK-0091: Streaming grep with batch protocol and bounded footer
  （`docs/tasks/0091-streaming-grep-batch-protocol-and-bounded-footer.md`）
- TASK-0092: Bulk-reader guidance, deny/registry parity, and final
  verification
  （`docs/tasks/0092-bulk-reader-guidance-parity-and-final-verification.md`）

task doc 自体は本 plan の範囲外（plan 承認後の follow-up で作成）。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | oversized-line 意味論: L（256KiB）超過行を未評価で破棄すると、その行にのみ存在する一致を見落とす | med | med | 仕様上の受け入れ済み制約（ADR-0017）。`onOversized` で footer `oversized-line` に `path:行番号` を必ず報告し `(no matches)` と区別。行番号を消費して後続行の番号ずれを防ぐテストを `line-framer.test.ts`/`grep.test.ts` に置く |
| 2 | UTF-8/chunk 境界の破損: chunk 境界の multibyte 文字や `byteInLine` が文字中を指す場合に文字化け・欠落・重複が出る | med | high | 行分割は raw Buffer の `0x0A` 走査（UTF-8 安全）、decode は `StringDecoder("utf8")` を chunk 跨ぎで継続。`byteInLine` は前方スナップ（最大 3 byte）のみ。境界跨ぎ fixture・不正 byte → U+FFFD・CRLF parity を `read.test.ts`/`grep.test.ts`/`line-framer.test.ts` で検証 |
| 3 | 累積 eval 予算の誤計上: batch ごとに per-file/total タイマーをリセットすると ReDoS 抑止が効かなくなる | med | high | per-file 予算は同一 file の全 batch eval 時間を累積、total は走査全体で累積（batch でリセットしない）。`evalPerFileMs`/`evalTotalMs` 縮退注入テストで累積挙動と worker terminate → 再生成を検証 |
| 4 | worker protocol 移行（`file` → `batch`）が既存 grep.test.ts を破壊する | high | med | TASK-0091 で既存 12 case を新 protocol の期待値へ全面的に更新し、赤→緑で進める。batch 内 0-indexed matched・in-flight ≤1 を postMessage spy で構造検証 |
| 5 | deny 存在秘匿の回帰: `onIssue`/footer/集計経路で deny 対象の path・件数が漏れる | med | high | deny 対象は walker filter 段で消し `onIssue` に載せない。`scope-deny.test.ts` に結果・`skipped:` 行・`and N more` 件数への非出現 assert を追加（TASK-0092） |
| 6 | メモリ instrumentation の CI flaky: `heapUsed` delta が GC タイミングや環境で振れる | med | med | 主証拠は構造的上限（spy 記録の chunk/batch/出力が C/L/B/O/S 以下、in-flight ≤1）とし、`heapUsed` delta（目安 4×(C+L+B+O+S)）は補助指標。RSS 一回観測を証拠にしない（ADR-0017 Confirmation） |
| 7 | read tool の Pi 互換 API 喪失（綴り変種解決・画像 attachment）が既存 worker 動作を壊す | med | med | 破壊的変更として受け入れ済み（ADR-0017）。image/非 regular file は明示 error、tool description と `bulk-reader.md` guidance で吸収。`registry.test.ts`/`scope-deny.test.ts` の read 経路を新契約で維持 |
| 8 | scan deadline / abort のタイミング依存テストが flaky になる | med | low | `scanDeadlineMs` を options 注入で縮退させ決定的に発火させる。wall-clock 実待機に依存する assert は避ける |
| 9 | 外部依存 availability: なし | low | low | 依存追加なし（`fs/promises`・`worker_threads`・`string_decoder`・既存 `defineTool`/typebox のみ）。mise/pnpm toolchain は既存のまま |
| 10 | データ損失: 新規コードは `open(...,"r")` と stat/readdir のみで read-only | low | high | read-only 維持を `scope-deny.test.ts` の journal 差分なし検証と write-scope 既存テストで回帰確認 |

### External Dependencies

- なし（依存追加・新 env・外部プロセスを導入しない。DESIGN-0019
  Non-Functional Constraints どおり Node.js 標準 API と既存
  `defineTool`/`typebox` のみ）

## Rollback Strategy

全変更は feature branch（`feature/bulk-reader-huge-file-safe-io`）上の
task 単位 commit で進め、不可逆な step（migration・外部公開 API の
破壊・配布物の手編集）は存在しない。sandboxed tool の内部契約変更のみ。

- **Trigger**: 既存 suite の regression が残る、boundary/deny 秘匿が
  破れる、または instrumentation が有界性違反を示した場合。
- **Procedure**: 対象 task の commit を `git revert`。依存順のため
  revert は逆順（TASK-0092 → 0091 → 0090 → 0089）に遡る。TASK-0089
  だけを戻すと 0090/0091 が `limits.ts`/`sniffTextFile`/
  `line-framer.ts` を解決できず compile 不可になるため、単独 revert
  はしない。
- **registry read mapping の revert point**: read 経路だけ旧契約へ
  戻す最小手順は、`registry.ts` の `case "read"` を
  `defineTool(createReadToolDefinition(opts.root, { operations:
  createReadOperations(opsOpts) }))` に戻し、`operations.ts` の
  `createReadOperations` と Pi import を復元すること（TASK-0090 の
  commit を revert するのと等価）。
- **Verification**: revert 後に `mise exec -- pnpm -r build` /
  `mise exec -- pnpm --filter @cheaper/runtime-pi test` が revert
  前状態と同一に pass すること。

## Verification

### Verification commands（全 task 共通の最終ゲート）

```bash
mise exec -- pnpm -r build
mise exec -- pnpm --filter @cheaper/runtime-pi test
mise exec -- pnpm --filter cheaper test
mise exec -- pnpm lint
mise exec -- pnpm --filter cheaper exec tsc --noEmit -p .
mise exec -- node .claude/skills/doc-status/scripts/audit_docs.js --type all
mise exec -- node scripts/bundle-cli.mjs
```

`node scripts/bundle-cli.mjs` は committed 生成物
（`.apm/hooks/cli/bin/cheaper.mjs` 等）の freshness 確認を兼ねる。
runtime-pi の tool 層変更で bundle 内容が変わる場合は再生成して
一致させる。

実 provider / CLI / hook の検証は TASK-0086/0087 と同様に、一時
ディレクトリへ `apm install` した隔離環境でのみ行う。本リポジトリ内の
bundled cheaper CLI・`.apm/hooks`・cheaper-* スキルによる自己委譲・
検証は実行しない。

### SPEC-0014 acceptance criteria → task/test mapping

| SPEC-0014 acceptance criterion | 対応 task / test |
|--------------------------------|------------------|
| 16MB 超・100MB 以上 fixture の範囲 read が cap 以下または規定 error で、全文 `readFile` 不呼出・低層 read 最大要求量を spy/fake で示す | TASK-0090 — `read.test.ts`: `readFile` spy 不呼出、`fh.read` の `length ≤ IO_CHUNK_BYTES` 記録、`maxBytes`/`chunkBytes` 縮退注入による決定的検証 |
| 先頭/中間/末尾・空 file・改行なし長大行・UTF-8 境界を読み、cap 到達と EOF を区別、継続取得可・非対応範囲は明示 error | TASK-0090 — `read.test.ts`: `details.eof`/`truncatedBy`/`continuation`、trailer 分岐、`byteInLine` 連続呼出の欠落・重複なし、`byteInLine` 単独/EOF 超過/binary/image の明示 error |
| 1MB 超・100MB 以上の許可 text fixture 後半の既知一致を grep が返し、処理境界跨ぎ一致を保持、非対応条件は明示拒否 | TASK-0091 — `grep.test.ts`: 巨大 fixture 後半の既知一致、C/B 縮退による chunk/batch 境界跨ぎ行、`pattern` > `GREP_PATTERN_MAX_CHARS` の invalid pattern error |
| find が 1MB 超・100MB・binary file を内容全体の読み取りなしで列挙 | TASK-0089 — `find.test.ts`: `open`/`readFile` spy が対象 file の内容 I/O を一度も呼ばないこと |
| binary 除外・unreadable・timeout・結果上限を注入し未処理理由が `(no matches)` と区別可能、skip 報告上限超過も識別可能 | TASK-0089（`find.test.ts`: `onIssue` footer）+ TASK-0091（`grep.test.ts`: `[skipped: binary/unreadable/oversized-line]`、`[truncated: evaluation time limit reached]`、`[truncated: scan deadline reached]`、`[truncated: more matches]`、`and N more`、`[skipped-report: truncated]`） |
| 16MB→100MB 入力増大・長大行・多数一致でメモリ予算内（instrumentation 明記） | TASK-0090/0091 — spy 記録の最大 chunk/batch/出力 ≤ C/L/B/O/S、in-flight batch ≤1、`heapUsed` delta（目安 4×(C+L+B+O+S)）を補助指標として記録 |
| workspace 外・symlink escape・built-in/custom deny の直接 read 拒否、grep/find 結果と footer に deny 非出現、read-only/write-scope 回帰 | TASK-0090（`read.test.ts`/`registry.test.ts`: boundary/deny 経路）+ TASK-0092（`scope-deny.test.ts`: 結果・footer・件数への非出現、journal 差分なし、write-scope 維持） |
| `count_tokens` の 16MB 超 tool error と cap 未満 canonical count、親 hook の exit 2 + 委譲 guidance の既存検証維持 | TASK-0092 — 既存 `count-tokens.test.ts`・`packages/cli` e2e（`pre-read-hook.test.ts` 等）の無変更 pass を suite 全体で確認 |
| 既存 126 テスト維持 + 追加テスト、実 provider/hook 検証は隔離 `apm install` 環境のみ | TASK-0092 — `mise exec -- pnpm -r build` / `pnpm --filter @cheaper/runtime-pi test` / `pnpm --filter cheaper test` 全 green。隔離環境検証は TASK-0086/0087 fixture pattern に従い本 repo の CLI/hook/skill は使わない |

## Open Questions / Gaps

- `line-framer.test.ts` は DESIGN-0019 File Map に未記載の追加
  test file。framer は純粋 helper のため単体 test を置くのが自然だが、
  grep 経由の間接検証に寄せる判断もあり得る — TASK-0089 実装時に
  確定する（blocker ではない）
- SPEC-0014・ADR-0017 の status は現時点で `proposed`（design は
  approved）。plan 自体は `draft` で提出し、coordinator review で
  upstream status 更新と併せて承認判断を受ける
- 既存 `registry.test.ts` の `read tool reads via boundary ops` が新
  read tool でもそのまま pass するかは実装時に確認する（応答が
  verbatim text 1 block なら互換の見込み）

## Self-Review

- [x] SPEC-0014 の全 acceptance criterion が task/test に map されている
      （上記 matrix 参照）
- [x] placeholder（TBD/TODO/implement later）なし
- [x] file path・relation key・status・command・用語が plan / upstream
      doc 間で一致（file map は実在する `packages/runtime-pi/src/tools/`
      と `test/tools/` の file 名で検証済み）

## Implementation Handoff

- Recommended mode: inline execution（critical path が TASK-0089 →
  {0090, 0091} → 0092 で、並列化可能な 0090/0091 も同一 package の
  file を共有するため単一 worktree の sequential 実行を推奨）
- Independent work streams: 実質なし。TASK-0090/0091 は file 上
  disjoint だが別 worktree 化の利が小さい
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: DESIGN-0019 は approved で design gate を満たす。SPEC-0014 /
  ADR-0017 は `proposed` のまま — acceptance 前に status 更新が必要
  （plan の `approved` 昇格条件とする）
- 番号: TASK-0088 まで使用済みのため TASK-0089 から採番。plan は
  PLAN-0014
- reviewer の注目点: (1) 上限定数・protocol・footer 仕様が
  DESIGN-0019 の確定値と一致しているか、(2) `registry.test.ts` を
  TASK-0090/0092 が共有する sequential 制約が守られているか、
  (3) deny 存在秘匿と read-only の回帰経路が verification matrix に
  入っているか、(4) task doc 未作成を前提に `implemented-by` が空で
  あること
- 主要 upstream: DESIGN-0019（実装設計・確定値）、SPEC-0014
  （acceptance criteria）、ADR-0017（方式選定と制約）、DESIGN-0002 /
  DESIGN-0006（維持する境界・worker 契約）

---
id: "SPEC-034rBhR3SNIRFZ1B6n26iH"
type: "spec"
status: "approved"
title: "Shell-read large-file blocking via canonical routing policy"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./cross-agent-native-routing-adapters-over-apm-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/default-large-read-threshold.md"
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
    - "../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../designs/cross-agent-native-routing-adapter-architecture.md"
  refines: []
  refined-by:
    - "../adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
  derives-from:
    - "../discovery/shell-read-large-file-detection-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/shell-read-blocking-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Shell-read large-file blocking via canonical routing policy

## What Should Be Built

shared routing policy（`routing-policy.js`）の `evaluateShell` seam に、
shell command 文字列から read 対象 file を抽出する保守的な read-candidate
抽出器を接続し、抽出した各 file を既存の canonical large-read 判定
（`evaluateFileRead`: workspace 境界 → stat → byte-size fast path →
`cheaper count-tokens` → threshold / `FILE_TOO_LARGE` verdict）へ流す。

対象 command は `cat` / `head` / `tail` / `less` / `more`。
#43 で全 target adapter が canonical `{kind:"shell", command, cwd}`
request を deliver 済みのため、adapter 層と `hooks.json` は変更しない。

## Why Now

`pre_read_code` 系 hook は file-read tool の直接 read しか捕捉できず、親
Coding Agent は `cat huge.ts` のような shell command で large-read
routing を迂回できる。issue #42 はこの経路の封鎖を要求する。#43 の
cross-agent adapter 構成が canonical shell request の seam を既に提供
しており、policy 側を埋めるだけで全 target に同一 policy が適用される。

## Users and Value

- 全 target（Windsurf / Claude Code / Codex / Cursor / Copilot）の利用者:
  shell 経由の巨大 file 直接 read も `bulk-read` 委譲へ誘導され、親 agent
  の context を圧迫しない。
- 開発者: large-read 判定は引き続き shared policy 1 箇所。shell 側の
  追加責務は「command 文字列 → file path 候補」の抽出のみ。

## Scope

### In Scope

- `evaluateShell` 内での shell command の保守的 tokenize と
  `cat`/`head`/`tail`/`less`/`more` の file operand 抽出。
- `|`・`&&`・`||`・`;` で区切られた複合 command の各 segment の独立評価。
- 抽出した各 file path に対する既存 large-read 判定の適用（複数 file の
  うち 1 つでも threshold 超過 / `FILE_TOO_LARGE` なら command 全体を
  deny）。
- deny diagnostics に block された command・対象 file・token 数または
  `FILE_TOO_LARGE`・configured threshold・`bulk-read` 誘導を含める。
- representative command の unit 的検証（hook script への stdin 注入
  contract test を既存 suite に拡張）。
- `docs/guides/parent-agent-integration.md` の更新（shell 経由検出、
  target 別対応状況、fail-open/fail-closed 挙動、既知制限）。

### Out of Scope

- arbitrary shell command の完全な AST 解析、`sed`/`awk`/`python` 等の
  任意コード経由 read の封鎖、shell 実行内容の sandbox 化（issue の
  非目標どおり）。
- adapter / `hooks.json` / bundle pipeline の変更（#43 の seam をそのまま利用）。
- shell expansion・command substitution・glob の評価（parser は実行しない）。
- 新規依存の追加。

## Requirements

1. **対象 command**: segment 先頭 token が `cat` / `head` / `tail` /
   `less` / `more` の場合に file operand を抽出する。それ以外の
   command は allow。
2. **operand 抽出**: `-` 始まりの token は option として扱い file path と
   誤認しない。`head -n 200`、`tail -100`、`tail -n +5` 等の行数指定を
   吸収する。operand が 0 個（`cat` 単独 = stdin read）の segment は
   対象外。複数 operand は全て判定する。`"`/`'` で quote された path
   （空白含有を含む）を1 operand として復元する。
3. **fail-open の保守性**: `$`・`` ` ``・`>`・`<`・glob 文字・未閉鎖
   quote 等、安全に解釈できない構文を含む segment は抽出を諦めて
   allow。command parse 全体の失敗・malformed payload・CLI 起動失敗・
   `count-tokens` output の parse 失敗も allow（既存 contract）。
4. **判定の共有**: 抽出した path は `evaluateFileRead` と同一の判定経路
   （workspace 境界 → stat → byte-size fast path → `count-tokens` →
   threshold 8192 / `CHEAPER_READ_TOKEN_THRESHOLD` override →
   `FILE_TOO_LARGE` fail-closed）を通す。shell 側で判定ロジックを
   重複実装しない。
5. **verdict**: いずれかの file が deny なら command 全体を deny。
   deny reason は block された command と対象 file を含む簡潔な
   誘導文とする。
6. **adapter 分離**: target 固有の payload shape・response contract は
   adapter の責務のまま。policy は canonical request のみを解釈する。
   shell hook を emit できない target は既知制限として docs に明記する
   （現状 Codex は `PreToolUse` matcher `Bash` で shell のみ到達、
   file-read は upstream gap で unsupported のまま）。

## Acceptance Criteria

- [ ] `cat` / `head` / `tail` / `less` / `more` で巨大 file を開く
      command が全 target の shell adapter 経由で deny される。
- [ ] 巨大でない file、対象外 command、option のみ、file 不存在、
      directory、workspace 外 path、malformed payload、CLI 起動失敗、
      `count-tokens` output parse 失敗では allow（fail-open）。
- [ ] `FILE_TOO_LARGE` envelope は fail-closed で deny される。
- [ ] `CHEAPER_READ_TOKEN_THRESHOLD` override が shell-read にも効く。
- [ ] 複数 file 指定で 1 つだけ巨大でも deny される。
- [ ] `cat a.ts | grep foo`、`cd src && cat huge.ts`、quoted path
      （`cat "path with spaces/file.ts"`）等の代表 command が test で固定される。
- [ ] shell parser と token 判定が分離され、判定は `evaluateFileRead` を
      共有する（adapter / shell 側に threshold 判定の複製がない）。
- [ ] 通常 read と shell-read が同一 threshold・verdict contract を共有する
      ことを regression test が保証する。
- [ ] `docs/guides/parent-agent-integration.md` に shell 経由検出の挙動・
      target 別対応・既知制限が反映される。
- [ ] `mise run check` が通る。

## Deferred Design Concerns

- `sed`/`awk`/`perl`/`python`/`xargs`/`find -exec` 等の任意経路の封鎖は
  将来の拡張候補。parser は対象 command set を table 駆動にして拡張余地を
  残すが、本 issue では 5 command のみ実装する。
- `cd` による segment 間の cwd 変化は追跡しない（`cd src && cat huge.ts`
  は hook 実行 cwd 基準で `huge.ts` を解決する。実装上の既知制限として
  docs に記録する）。

## Sources and Relations

- Issue #42: 本仕様の起点と完了条件。
- DISC-034rBfZ3dqNmEQGXCIwiLn: parser 境界と target 到達状況の探索結果。
- SPEC-034rzTJydXfLvQhTIqiDqW / ADR-034rzTJYDRmre6D5OS8HDW /
  DESIGN-034rA79Uog7JOa7evw6rOL: canonical request + per-target adapter
  構成。本仕様は `kind:"shell"` request の policy 側を実装する。
- ADR-034r05iHrZUE6W4XOqUBih（default threshold）/
  ADR-034r05iHrZUE6W82VsDbPq（FILE_TOO_LARGE fail-closed）: 共有する
  判定 contract。

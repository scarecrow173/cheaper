---
id: "ADR-034rBhRB7QyWkAxRiTDXXq"
type: "adr"
status: "accepted"
title: "Conservative shell read-candidate extraction inside the shared routing policy"
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
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "./default-large-read-threshold.md"
    - "./treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/cross-agent-native-routing-adapter-architecture.md"
    - "../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../specs/shell-read-large-file-blocking.md"
  refines:
    - "../specs/shell-read-large-file-blocking.md"
  refined-by: []
  derives-from:
    - "../discovery/shell-read-large-file-detection-boundary.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Conservative shell read-candidate extraction inside the shared routing policy

## Context and Problem Statement

親 Coding Agent は shell tool 経由の `cat` / `head` / `tail` / `less` /
`more` で file-read hook を迂回し、巨大 file を context に直接取り込める
（issue #42）。#43 で全 target adapter が canonical
`{kind:"shell", command, cwd}` request を shared routing policy の
`evaluateShell` seam（現状 allow-only）へ deliver する構造が完成した。

残る決定は、shell command 文字列から read 対象 file をどこに・どの精度で
抽出するかである。hook は親 agent の全 shell 実行の同期経路に乗るため、
誤 block は通常 ops を壊し、shell 構文の実行・展開は副作用とセキュリティ
リスクを持つ。issue は「完全な shell parser は不要、誤 block を最小化する
保守的 parser」と明示する。

## Decision Drivers

- large-read 判定（workspace 境界・threshold・`count-tokens`・
  `FILE_TOO_LARGE` fail-closed）は `evaluateFileRead` に単一実装済み。
  重複させると threshold override や fail semantics が drift する。
- adapter 層は native payload → canonical request の normalize に限定する
  #43 の責務分界を維持する。command 文字列の解釈は target 非依存であり
  adapter に置く理由がない。
- parser は shell を実行しない・展開しない。`$(...)`・backtick・
  redirect・glob・変数を含む command を安全に解釈できない場合は抽出を
  諦めて fail-open する。
- 新規依存は追加しない（issue 制約 + hook は配布 bundle の一部で
  dependency を持てない）。

## Considered Options

- shared policy 内の保守的 tokenizer + `evaluateFileRead` 再利用
- 本格 shell parser（AST library または自作）の導入
- per-target adapter 側での command parse
- 正規表現による file 名抽出のみ

## Decision Outcome

Chosen option: "shared policy 内の保守的 tokenizer + `evaluateFileRead`
再利用"。判定の source of truth を 1 箇所に保ち、adapter・hooks.json を
一切変更せず全 target に同一 policy を適用できる唯一の option だから。

### Consequences

- Good: file-read / shell-read が同一 threshold・同一 verdict・同一
  fail-open/fail-closed contract を構造的に共有する。adapter は #43 の
  まま無変更。
- Good: tokenizer は空白・quote・segment 区切り・既知 option skip だけを
  扱う小さな pure function に留まり、table 駆動で対象 command を拡張可能。
- Bad: shell 構文の一部（`cd` 後の相対 path、heredoc、process
  substitution）は追跡しないため検出漏れがあり得る。fail-open を選ぶ
  設計上の許容済み制限として docs に記録する。
- Bad: 独自 tokenizer は shell 互換性の保守負担を持つ。代表 command の
  contract test で挙動を固定して緩和する。

### Confirmation

`cross-agent-routing-adapters.test.ts` の stdin 注入 contract test で、
代表 command（単一/複数 file、option、pipe、`&&`、quoted path、
ambiguous syntax）に対する allow/deny を固定する。通常 read と
shell-read が同一 verdict を返す regression test を含める。

## Implementation Plan

- Affected paths: `packages/apm/.apm/hooks/routing-policy.js`
  （`evaluateShell` + tokenizer + operand 抽出）、
  `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`（contract
  test 拡張）、`docs/guides/parent-agent-integration.md`（挙動・既知制限）。
- Patterns to follow: `evaluateFileRead` の verdict 形状
  （`{decision:"allow"}` / `{decision:"deny", reason}`）、deny reason の
  非 executable guidance 方針（path を shell command に埋め込まない）、
  fail-open via allow。
- Patterns to avoid: adapter 側での command 解釈、shell 実行・
  `child_process` を使った parse、`eval`、新規依存、token 判定ロジックの
  shell 側への複製。
- Dependencies or configuration: なし（Node stdlib のみ）。
  `CHEAPER_READ_TOKEN_THRESHOLD` は既存のまま共有。

## Verification

- [ ] `mise exec -- pnpm test` で hook contract test を含む全 test が pass。
- [ ] `routing-policy.js` 以外に threshold・`count-tokens` 呼出し・
      `FILE_TOO_LARGE` 解釈の複製がないことの review。
- [ ] deny diagnostics が command・file・token/threshold または
      `FILE_TOO_LARGE`・`bulk-read` 誘導を含むことの test 確認。

## Pros and Cons of the Options

### Shared policy 内の保守的 tokenizer

- Good: 判定の単一化、全 target へ一括適用、fail-open が構造に沿う。
- Bad: shell 構文カバレッジは意図的に部分的。

### 本格 shell parser

- Good: 複合構文・quoting の網羅。
- Bad: issue の非目標（完全 AST 解析）に抵触、依存追加または大きな自作
  コスト、hook 実行時間の増大。

### per-target adapter で parse

- Good: target 差異の局所化。
- Bad: 判定ロジックが adapter 数だけ重複し drift する。#43 の責務分界を
  破壊する。

### 正規表現のみ

- Good: 実装最小。
- Bad: quote・option・複合 command に脆弱で誤 block / 検出漏れが多い。

## More Information

- Issue #42: 要件の正（対象 command・fail-open/fail-closed・非目標）。
- DISC-034rBfZ3dqNmEQGXCIwiLn: parser 境界の探索と target 到達状況。
- ADR-034rzTJYDRmre6D5OS8HDW: canonical request + per-target adapter の
  前提構造。本 ADR は `kind:"shell"` の policy 側を確定する。

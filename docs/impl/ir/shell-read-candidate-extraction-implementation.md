---
id: "IMPL-034rBOXmCoWTHAK8NRcewR"
type: "impl"
status: "completed"
title: "Shell read candidate extraction implementation"
created: "2026-09-19"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/apm/.apm/hooks/routing-policy.js"
      - type: "file"
        path: "packages/cli/test/e2e/cross-agent-routing-adapters.test.ts"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/shell-read-candidate-extractor-in-routing-policy.md"
    - "docs/tasks/shell-read-contract-tests.md"
    - "docs/tasks/shell-read-docs-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/shell-read-large-file-blocking.md"
    - "docs/designs/shell-read-candidate-extraction.md"
    - "docs/adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
    - "docs/test-specs/shell-read-blocking-contract-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/shell-read-large-file-blocking.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Shell read candidate extraction implementation

## Summary

issue #42。shell command 経由の巨大 file read（`cat` / `head` /
`tail` / `less` / `more`）を shared routing policy 内の保守的
parser で検出し、通常の file-read と同一の `count-tokens` 判定へ
流して block する。adapter 層・`hooks.json` は無変更で、
`evaluateShell` の allow-only seam に read candidate 抽出を
接続しただけの構成。

## Context

TASK-034rBt3hfF2knHMAeo5KhB /
TASK-034rBt3LLoIUcFHjk4gRlI /
TASK-034rBt4e20Wrlc3FH3NQWd（PLAN-034rBrrA2j5usWH2G77b8G /
SPEC-034rBhR3SNIRFZ1B6n26iH /
ADR-034rBhRB7QyWkAxRiTDXXq /
DESIGN-034rBoPPFNHXR5eSfv0hsO /
TSPEC-034rBrs4K5W7pe0ixjegZk）に従う。Issue #42。

## Implementation

### Changes

- `packages/apm/.apm/hooks/routing-policy.js`
  - `tokenizeShell(command)`: whitespace・quote・escape・
    segment 区切り（`|`・`||`・`&&`・`;`・改行）の tokenizer。
    `$`・backtick・glob・`~`・`()`・`{}`・`!` を含む token と
    `<`・`>` を含む segment、未閉塞 quote を unsafe として
    マークし、解釈不能部分は skip して fail-open にする。
  - `READ_COMMANDS`: `cat` / `head` / `tail` / `less` / `more`
    の option 定義（`-n 200`・`-100`・`+F` 等の option value
    消費を含む）。
  - `extractReadCandidates(command)`: env 代入 prefix を skip し、
    command 名（`.exe` / `.cmd` / `.bat` 等の Windows suffix 含む）
    を照合して、option を除いた operand を一意化して返す。
  - `evaluateShell(request, opts)`: 各 candidate を
    `evaluateFileRead` に委譲し、最初の deny で command 全体を
    deny。deny reason の先頭に正規化した command を付記する。
- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`
  - canonical shell request 経路の contract test 16 case と、
    4 target adapter（Windsurf exit 2 / pretooluse deny JSON /
    Cursor permission / Copilot permissionDecision）の native
    deny 表現を固定。
- `docs/guides/parent-agent-integration.md`
  - 「shell 経由の file read 検出」節を追加し、対象 command・
    target 別対応・fail-open/fail-closed・既知制限（cwd 追跡
    なし・ambiguous syntax fail-open・任意コマンド non-goal・
    Codex file-read unsupported）を記述。

### Decisions During Implementation

- deny 判定は `evaluateFileRead` への委譲のみとし、shell 側に
  threshold・workspace・`FILE_TOO_LARGE` ロジックを複製しない
  （DRY / SPEC-034rBhR3SNIRFZ1B6n26iH の同一判定要件）。
- `cd` 追跡は行わず hook 起動時 cwd を基準に評価する。誤検出
  ではなく既知制限として contract test と guide に固定した。
- 手動 smoke で `x`×9000 fixture が threshold 未達だった件は、
  token 数で閾値判定される仕様通りの挙動。test fixture は
  `lorem ipsum` 系の語列で token 数を確保する方式にした。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- pnpm --filter cheaper exec vitest run
  test/e2e/cross-agent-routing-adapters.test.ts`:
  31 tests 全件 pass（2026-09-20）。
- `mise run check`: 後述の task 3 verification で実行。

## Risks

- `cd` による cwd 遷移・shell expansion を伴う operand は
  fail-open で素通しになる（設計上の既知制限、guide に記載済み）。

## Follow-ups

なし。

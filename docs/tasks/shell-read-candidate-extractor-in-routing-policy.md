---
id: "TASK-034rBt3hfF2knHMAeo5KhB"
type: "task"
status: "done"
title: "Shell read-candidate extractor in routing-policy"
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
  implements:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
  implemented-by:
    - "docs/impl/ir/shell-read-candidate-extraction-implementation.md"
  depends-on:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
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
  references:
    - "docs/test-specs/shell-read-blocking-contract-tests.md"
  defers: []
  deferred-by: []
---

# Shell read-candidate extractor in routing-policy

## Work

`packages/apm/.apm/hooks/routing-policy.js` の `evaluateShell` を実装する。
`tokenizeShell`（空白・`'`/`"` quote・`\` escape・`|`/`||`/`&&`/`;`/改行の
segment 分割）と `READ_COMMANDS` table（`cat`/`head`/`tail`/`less`/`more`
の operand 抽出規則）を追加し、`extractReadCandidates` で file path 候補を
集めて各々を `evaluateFileRead` へ委譲する。deny 時は reason 先頭に
block された command を添える。解釈不能構文・対象外 command・operand
なしは allow（fail-open）。shell 実行・展開・依存追加はしない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/shell-read-candidate-extraction-implementation.md`
- `docs/designs/shell-read-candidate-extraction.md`
- `docs/specs/shell-read-large-file-blocking.md`
- `docs/adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md`

## Dependencies

- なし（chain の先頭 task）

## Affected Files

- `packages/apm/.apm/hooks/routing-policy.js`

## Done When

- [x] `evaluateShell` が対象 command の operand を抽出して
      `evaluateFileRead` へ判定を委譲する
- [x] deny reason が command・file・token/threshold または
      `FILE_TOO_LARGE`・`bulk-read` 誘導を含む
- [x] 解釈不能構文・対象外 command・operand なしで allow を返す

## Verification

- [x] `node -e` 等で `evaluate({kind:"shell", command:"cat <big>"})` が
      deny を返すことを確認（contract test は後続 task）

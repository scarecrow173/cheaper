---
id: "TASK-034z1PTkni5iNJPysyfxRN"
type: "task"
status: "done"
title: "Verify SEA PoC smoke without Node.js and record size"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
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
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
    - "docs/tasks/implement-build-executable-mjs-and-bump-node-to-26-9.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/standalone-executable-contract-parity.md"
    - "docs/test-specs/embedded-builtin-resource-resolution-across-distribution-forms.md"
  references: []
  defers: []
  deferred-by: []
---

# Verify SEA PoC smoke without Node.js and record size

## Work

生成した `dist/sea/cheaper-windows-x64.exe` を、PATH から Node.js を
除去した環境で smoke する（PowerShell で `PATH` を絞った子 process
起動、または Node が存在しない一時 PATH）。以下を確認・記録する:

- `--version` / `count-tokens <file>` / `models` / `auth status` が動作
- fake/test runtime 経由で built-in worker resolution と JSON schema
  load が executable 内 VFS resource から成立する（tiktoken WASM の
  VFS load 可否をここで判定する）
- worker run の stdout が 1 result envelope、diagnostics が stderr、
  exit code が現行 CLI と一致
- executable size（MiB）を計測し plan または issue #71 に記録

失敗時の扱い: `import.meta.dirname` が VFS root を指さない場合は
`node:sea` の `getAsset` resolver への切替を design amend として
検討し、tiktoken/WASM が VFS で動かない場合は real FS 退避設計へ
エスカレーションする（plan の Open Questions 参照）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md
- docs/test-specs/standalone-executable-contract-parity.md
- docs/test-specs/embedded-builtin-resource-resolution-across-distribution-forms.md

## Dependencies

- implement-build-executable-mjs-and-bump-node-to-26-9.md

## Affected Files

- `dist/sea/cheaper-windows-x64.exe`（検証対象の生成物）

## Done When

- [ ] Node 不在 PATH で 4 command（`--version`・`count-tokens`・
      `models`・`auth status`）が exit 0 または既存 CLI と同一の
      exit code で完了する
- [ ] worker/schema が外部ファイルなしに解決される
- [ ] executable size が記録される

## Verification

- [ ] PATH を絞った PowerShell 子 process で
      `dist/sea/cheaper-windows-x64.exe --version` exits 0
- [ ] 同条件で `count-tokens` が token envelope を stdout に出力する
- [ ] `git status` で `dist/sea/` が untracked/ignore 済みであること
      を確認（生成物はコミットしない）

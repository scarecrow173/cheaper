---
id: "IMPL-034z1SUIDKBAfvsAeKimVK"
type: "impl"
status: "completed"
title: "Verify SEA PoC smoke without Node.js"
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
    - "docs/tasks/verify-sea-poc-smoke-without-node-js-and-record-size.md"
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
    adopted: []
    rejected: []
---

# Verify SEA PoC smoke without Node.js

## Summary

`dist/sea/cheaper-windows-x64.exe`（112.3 MiB）を PATH から Node.js を
除去した環境（`PATH=/c/Windows/System32:/c/Windows:...`）で実行し、
issue #71 Phase 1 の smoke を完了した。全検証 pass。

## Context

docs/tasks/verify-sea-poc-smoke-without-node-js-and-record-size.md
（PLAN-034z1M6zCAdKb12eiCWZRm task 5）。TSPEC-034z1OZC4sSkRlvZfFfIXt /
TSPEC-034z1P09dg86lO9uB31cUi の検証。

## Implementation

### Changes

コード変更なし（検証のみ）。生成物 `dist/sea/cheaper-windows-x64.exe`
は gitignore 済み（`dist` パターン）でコミット対象外。

### Decisions During Implementation

- smoke は Git Bash の `env` で PATH を Windows 最小構成に絞って実行
- worker 解決の証明として `run bulk-reader` を不正 input → schema エラー、
  正 input → `MODEL_TIER_UNAVAILABLE`（model 解決段まで到達）、
  `--model cheaper-fake/fake-model` + `CHEAPER_TEST_FAKE_STREAM` で
  provider stream 起動（`turns: 1`）まで確認

## Related Experiments

### Adopted

なし（PoC 判定用の検証。最小 probe は前 task の IR に記録）

### Rejected

なし

## Validation

Node 不在 PATH（`env PATH=...` で Windows 系 dir のみ）:

- `cheaper-windows-x64.exe --version` → `1.0.0` exit 0
- `cheaper-windows-x64.exe count-tokens <file>` → envelope
  `{tokens:4, tokenizer: js-tiktoken 1.0.21}` exit 0。**tiktoken
  （pure JS/WASM なし）は VFS で問題なく動作**
- `cheaper-windows-x64.exe models` → envelope exit 0
- `cheaper-windows-x64.exe auth status` → providers 一覧 envelope exit 0
- `run bulk-reader`（不正 input）→ `INVALID_INPUT` envelope + exit 2 +
  stderr diagnostics — schema load が VFS から成立
- `run bulk-reader`（正 input）→ `MODEL_TIER_UNAVAILABLE` exit 5 —
  worker definition・input/output schema を VFS から解決して model
  解決まで到達
- `--model cheaper-fake/fake-model` + fake stream → `turns: 1` で
  provider 実行に到達（`CHEAPER_TEST_FAKE_STREAM` の外部 dynamic
  `import()` も SEA 下で動作）
- executable size: **112.3 MiB**

## Risks

- fake stream の応答 shape は stub 側の問題で完全な run 成功までは
  未確認（provider stream 呼び出し到達は確認済み）。実 provider での
  end-to-end run は Phase 2 の smoke test で確認する
- `import.meta.dirname` が Windows では `\\.\nul\vfs\0` を返す。
  POSIX では別 mount point になるはずで、cross-platform で同じ挙動かは
  Phase 2 で確認する

## Follow-ups

- Phase 2: cross-platform matrix・GitHub Release 接続・CI smoke
- Phase 3: APM payload + launcher（DESIGN-034z1EgnQwax4VamK2xmdT）
- Phase 4: `cheaper hook` subcommand 化
  （DESIGN-034z1EhuztlLSb2x2XXxgD）

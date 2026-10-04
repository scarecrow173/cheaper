---
id: "IMPL-034A6mdUuMzX4DtyfiRMjx"
type: "impl"
status: "in-progress"
title: "Verify consumer acquisition and close migration"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/tasks/verify-consumer-acquisition-and-close-migration.md"
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

# Verify consumer acquisition and close migration

## Summary

不同梱 APM source の install/acquisition と real CLI semantic hook を Node-free launcher から検証した。

## Context

PLAN-034zXjR2oE7USB76eUPCAq の Unit C。IR を test fixture の変更前に開いた。

## Implementation

### Changes

native-executable-fixture は unique repository dist directory へ fresh local SEA を build/cleanup する。
deployed-executable は verified user cache を seed し、両 OS とも launcher 経由の version/models/deny を検証する。
launcher-acquisition は committed executable 依存を除去し、4 non-Copilot target の source install →
local HTTP acquisition → Node-free real hook deny と cold/warm offline tests を追加した。
APM target tests は source binary 不在を assert し、Copilot の JSON/schema gap と各 target config 検証を保持した。
移行/復旧 guide と旧 ADR/SPEC/TSPEC の reciprocal supersede を追加した。

### Decisions During Implementation

Copilot source は binary-free acquisition 非対応として扱い、archive/local sibling を保持する。
Cursor/Copilot host の allow fallback と launcher の非 0 を区別した。
Windows PowerShell 5.1 の Get-FileHash failure は Vitest uppercase PSMODULEPATH と fixture camelCase の
case-insensitive collision が原因と診断し、test env の表現を一つに揃えた。launcher 本体は変更していない。
POSIX は /usr/bin 自体に node がある環境も含め、必要な utility だけを symlink した PATH を使う。

## Related Experiments

### Adopted

なし。diagnostic は evidence log に記録した。

### Rejected

なし。

## Validation

RED: 旧 deployed-executable は untracking 後に exit 1、deployed-red.log。
GREEN: deployed 単独は 1 pass / 1 skip、target 単独は 6 pass、acquisition 単独は 55 pass / 5 skip（304.10 s）。
lint/typecheck/build 各 exit 0。初回 lint の変更 file の format error を Biome で修正した。
focused combined suite は 5 files / 69 pass / 6 skip、exit 0（308.73 s）。focused-green.log。
scripts combined は 43 pass / 1 skip、exit 0。source checker CLI と git diff --check も成功。
doc-status scoped（本 chain + shared ADR）は blocking 0。shared ADR の既存 reciprocal warnings 2 件は非 blocking。
global は既存 blocking 41 件のままで scope 外。managed index は doc-maintenance plan → apply。

## Risks

Windows の local smoke を foreign OS native 実証とは呼ばない。Copilot JSON gap と実 publish は scope 外。
tracked cheaper.mjs は再生成せず coordinator が merge 時に refresh する。

## Follow-ups

coordinator implementation/test-spec review と scoped exit-audit。
canonical check deferred to coordinator merge run。

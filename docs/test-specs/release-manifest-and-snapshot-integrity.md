---
id: "TSPEC-034zTWh6x6esCzZmjDdiT0"
type: "test-spec"
status: "superseded"
title: "Release manifest and snapshot integrity"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: ["TSPEC-034DmUtUjWCThCnJulok0c"]
  related: []
  refines: []
  refined-by: []
  derives-from: ["../designs/one-dispatch-release-workflow.md"]
  derived-by: []
  verifies: ["../specs/one-dispatch-release-workflow.md","../adr/one-dispatch-release-workflow.md"]
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Release manifest / snapshot integrity

## Purpose

acquisition launcher が別version・破損・不完全assetを起動しない契約を公開側から守る。manifest形式または配布方式を別契約へ変更した時に見直す。

## Feature

同一runのraw3 executableからstrict manifestとplain staged package snapshotを生成する。

## Rules

- Rule: UTF-8 ASCII subset、LF only / CR拒否、65536byte以下、空行なし。
- Rule: 64hex + ASCII space2個 + basename、全basename duplicate禁止、raw3各exactly1。
- Rule: tag / canonical version / APM / version.jsonは一致し、asset名は両launcherと一致。
- Rule: rawbytes/manifest同梱、POSIX0755、checkout source非変更、dry-run非変更。
- Rule: non-file/symlink/LFS pointer/missing/hash mismatch/既存archiveはfailclosed。

## Examples

- Example: 手計算のabc SHA256をraw3 entryに使うとASCII LFの3行を生成する。
- Example: CRLF / BOM / tab delimiter / slash basename / 空行 / 65537byte manifestは拒否。
- Example: 1 target欠落、対象/対象外basename重複は拒否。
- Example: source旧payloadのmarkerを保持したままarchiveにはfreshrawを配置し、展開後hashが同じ。
- Example: dry-runと不一致tagはsource/outputへ変更しない。

## Guarantees

R1/R2/R3/R6 とAC1/AC2/AC4/AC7のartifact境界をslice変更後も維持する。named testはplan Step→Spec Mappingで識別する。

## Non-goals

署名保証、実native3 runner、実Release公開、provider calls、LFS/通常CI移行。fixturerawbytesは実3target executable成功のevidenceではない。

## Risk

形式driftはfirst-run failclosedを起こし、hash/coverage省略は誤配送・不完全bundleを見逃す。source変更は旧LFS経路を壊す。

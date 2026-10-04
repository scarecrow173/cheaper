---
id: "IMPL-034A6i3cxbZXoHqDxhuyjB"
type: "impl"
status: "completed"
title: "Migrate native workflows and release guidance"
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
    - "docs/tasks/migrate-native-workflows-and-release-guidance.md"
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

# Migrate native workflows and release guidance

## Summary

committed executable を前提にした CI を source/native build の責務へ移行した。

## Context

PLAN-034zXjR2oE7USB76eUPCAq の Unit B。承認済み spec/design に従い IR を実装前に開いた。

## Implementation

### Changes

build-executables の strict_freshness、matrix payload、cmp、LFS checkout を除去した。
expected_version と native --version/models smoke、3 runner artifact upload は保持した。
release-acquisition の caller を更新し、旧 release は contents: read の refusal job とした。
CI/AGENTS と release guide、apm-bundle のコメントのみを整合させた。

### Decisions During Implementation

公開は既存 release-acquisition の 1 dispatch を再利用する。実 publish は行わない。
source checker の変更で既存 mise check contract を保てるため mise.toml は変更しなかった。

## Related Experiments

### Adopted

なし（承認済み設計の機械的移行）。

### Rejected

なし。

## Validation

RED: release-workflow.test.mjs exit 1（freshness が残り legacy publication が許可される）。workflow-red.log。
GREEN: release-workflow.test.mjs 単独と release-artifacts/workflow の combined suite とも exit 0。
combined は 27 pass / 1 skip（POSIX tar mode）、release-green.log。

## Risks

初回実 release dispatch/publication、foreign OS native execution はこの worker では未検証。
全 5 asset/native smoke 成功前に consumer を切り替えない。旧 manifest/deploy 復元と新 tag fix-forward を記録した。

## Follow-ups

APPROVED: implementation-review / test-spec-approval を受領。canonical check deferred to coordinator merge run。

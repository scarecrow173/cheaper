---
id: "IMPL-034zUpcuJnZnznZ79UL3Q2"
type: "impl"
status: "completed"
title: "Wire one-dispatch release publication"
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
    - "../../tasks/wire-one-dispatch-release-publication.md"
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

# Wire one-dispatch release publication

## Summary

code変更前にIRを開き、承認済みnative3 DAG / fixed SHA / draft upload設計を実装する。

## Context

PLAN-034zTJptHTdovrvYW7wQ1k slice2。APPROVED: 73b-plan / 73b-spec-adr-design受領済み。

## Implementation

### Changes

release-acquisition.ymlのresolve→native build→Ubuntu archive→native3 APM smoke→publish DAGを追加。
build-executablesにはoptional expected_versionと不在skipのearly gateだけを追加した。
smoke-release.mjsは全3hash/internal-external manifest/metadataをdeploy前後で検証し、isolated consumerへAPM installしたlauncherをNode-free PATHで実行する。

### Decisions During Implementation

tagをenvで渡し、固定SHAを全jobで再利用。publish前にFETCH_HEAD^{commit}を再照合する。
成功したpaginated Release一覧からexact既存tagを拒否し、全5assetをdraftへuploadしてからpublic化する。
download sea-*はrawのみなのでpublish時にrelease-bundle側SHA256SUMSを配置し、--verifyでrawを再照合する。
libraryのsafe SemVerは#73-A launcherのgrammarへ合わせた。新caller strict:false / 旧trueを維持する。

## Related Experiments

### Adopted

なし（承認済み設計を実装）。

### Rejected

なし。

## Validation

REDは新workflow/smoke不在の7fail。実装後workflow suite 11pass / 0fail。
artifactと合計26pass / 0fail / Linux producer-only 1skip（最新両suiteは最終sliceで再実行）。
既存installed yaml@2.9.1を使った追加syntax検査は新workflow5jobs / reusable1jobでPASS（script/testsの依存追加なし）。
Windows host smoke: committed LFS payloadからstageし、APM windsurf install、deploy後3hashとNode-free --version/modelsがexit0。target win32-x64、version1.0.0、verified3、nodeFree:true。
fixture path: tmp/host-release-WZ0n1s。実CIfreshbuildではなく既存payloadの補助evidenceである。
focused reviewでjob依存、token権限、旧job/strict経路不変、draft順序、tagref照合を確認した。
Biomeはscripts/**対象外。最終sliceのcanonical pnpm lint evidenceへ委ねる。

## Risks

Linux native x64/arm64、Ubuntu archive0755、実GitHub API/upload/publicationはCI-only。実dispatchなし。
draft失敗は手動確認・回復が必要。tag変更を再fetchで検出するが外部tag操作をロックしない。

## Follow-ups

次sliceでrelease/distribution guideと最終scoped auditを完了する。
canonical check deferred to coordinator merge run。

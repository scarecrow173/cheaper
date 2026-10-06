---
id: "TSPEC-034zTWhBVg6DzifF8qlhAc"
type: "test-spec"
status: "proposed"
title: "Release publication and native smoke"
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
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../designs/one-dispatch-release-workflow.md"]
  derived-by: []
  verifies: ["../specs/one-dispatch-release-workflow.md","../designs/one-dispatch-release-workflow.md"]
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Release publication / native smoke

## Purpose

release担当者が同一sourceの全assetを公開前に検証し、部分公開・上書き・過剰権限を避ける意図を保持する。publisherやrunner構成を変更した時に再検討する。

## Feature

固定SHAのnative3 build/bundle smokeを経てdraftへ全assetを揃え、最後に公開する。

## Rules

- Rule: workflow_dispatchのみ、tag毎concurrency / cancel:false、default contents:read、publishのみwrite。
- Rule: tagはenvで渡しcanonicalgate / sourceSHA固定 / publish前FETCH_HEAD^{commit}再照合。
- Rule: optional expected_version空ならskip、新strictfalse/旧stricttrueを保持。
- Rule: native3 smokeを全て待ち、Windows以外ではWindowsbinaryはhashのみ。APMinstall後Node-free PATHでversion/modelsを実行。
- Rule: GH_TOKENはGITHUB_TOKENのみ、existingdraft/public拒否、APIerrorはfailclosed。
- Rule: draftにraw3+SHA256SUMS+tar.gzを一括upload後public化。新stdlibsuiteをpre-gateで実行。

## Examples

- Example: tag drift、APIerror、existingRelease、version非一致、smokenonzeroはpublishを止める。
- Example: Windows/Linux x64/Linux arm64のnative jobが全てsuccess後だけpublish jobへ進む。
- Example: draft uploadの途中で失敗した場合、operator確認/手動回復後に再dispatchする。
- Example: 新経路のmanifest付きReleaseはacquisition互換、旧rawのみReleaseではfallbackがfailclosed。

## Guarantees

R1/R3/R4/R5 とAC3/AC5/AC6のCI declaration・smoke失敗境界を保持。guideはplanのnamed review/test anchorsとoperator reviewで確認し、構造検証だけで実公開成功を主張しない。

## Non-goals

GitHub APIへの実write、実native3のPASS、署名、tagの外部変更の禁止、旧LFS廃止。API/client自体を再実装しない。

## Risk

dependency/publish順序やpermissionのdriftでpartial public releaseが生じる。API failureをabsenceとして扱うと誤った再作成を試みる。

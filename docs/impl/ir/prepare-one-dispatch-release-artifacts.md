---
id: "IMPL-034zU5o0mgVqx5ZwBdxc0L"
type: "impl"
status: "completed"
title: "Prepare one-dispatch release artifacts"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: ["../../tasks/prepare-one-dispatch-release-artifacts.md"]
  references: ["../../plans/implement-one-dispatch-release-workflow.md"]
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Release artifact preparation record

## Summary

task graph の先頭sliceとしてstdlib manifest / snapshot境界を実装する。code変更前にIRを開いた。
implementation-flowのdispatched-worker SUBAGENT-STOPにより明示planを直接実行する。
superpowers:test-driven-developmentを適用し、ユーザーのfocused検証/ canonical deferを優先する。

## Changes Made

strict ASCII/LF manifest、required raw3 mapping、streaming hash、version/tag gate、source非変更のstaged snapshot/tar.gz、dry-runを実装した。codeとcontractのfocused reviewでbasename/duplicate/hash/owned cleanupを確認した。

## Validation and Evidence

plan/spec/ADR/designはcoordinator APPROVED受領済み。taskGraph3nodes/2edges/issues0。
commit 0968758 は承認済み文書契約。implementationのtaskBudgetCount=3、maxHops=12。
baseline HEAD=096875864600293679d9ecaacac2b96005f486e4、IR/index開始後のdirtyを記録。
taskcheckpointでは全owneddirtyをcommitしてから完了させる。

RED: library/CLI不在で15 fail。実装後のfocused suiteは15 pass / 0 fail / 1 Linux-only skip。
Windows tarでCRLF listingとPOSIX chmod非保存を観測したため、listingのtrimEndを適用し、0755 testはUbuntu producerでのみ実行する。新workflowのUbuntu pre-gateがこのtestを実行する。
Biomeはroot設定でscripts/**を対象外とするため、scoped invocationはNo files processed。依存/configは変更せず、canonical pnpm lintを最終sliceで実行する。
impl/ir indexは承認済み例外のupdateIndexForMarkdownDirによる再生成を使う。

## Risks or Follow-ups

実native3とRelease publishはCI-only。fixture rawbytesを実3target成功とは主張しない。
canonical check deferred to coordinator merge run。


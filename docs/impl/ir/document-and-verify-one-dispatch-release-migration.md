---
id: "IMPL-034zUCrFChJvE5zTGpzwnI"
type: "impl"
status: "completed"
title: "Document and verify one-dispatch release migration"
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
    - "../../tasks/document-and-verify-one-dispatch-release-migration.md"
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

# Document and verify one-dispatch release migration

## Summary

guide変更前にIRを開いた。1dispatch公開と旧LFS準備の併存を説明し、承認済みchainの最終検証を実施する。

## Context

PLAN-034zTJptHTdovrvYW7wQ1k slice3。coordinatorのcanonical defer / scoped exit-audit運用を適用する。

## Implementation

### Changes

release.mdを「現行version/LFS準備」→「公開1dispatch」のモデルへ更新した。
新Releaseの5asset、固定SHA/native3/checksum/APM/draft DAG、最小権限、手動draft recovery、旧raw-only Releaseの取得failclosedを記載した。
distribution.mdにplain snapshot展開→local path APM installを追加した。

### Decisions During Implementation

one-dispatchの範囲は公開pipeline。現行Git-based APM payload/canonical checkのLFS round-tripは#73-Cまで保持する。
guideはnamed manual reviewで確認し、proseのsnapshot testは追加しなかった。
root Biomeのpackages/** include範囲を維持し、scripts/**はstdlib focused testsとsyntax/workflow reviewで検証する。
impl/ir indexは承認済み例外のupdateIndexForMarkdownDirを一時cjs moduleから呼んで再生成する。一時moduleはcommitしない。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

2026-10-04の実command evidence:

- `mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs`: exit0、26pass / 0fail / 1Linux-only skip。
- `mise exec -- pnpm lint`: exit0（既存configを維持、scripts/**は対象外）。
- `mise exec -- pnpm typecheck`: exit0。
- `mise exec -- pnpm build`: exit0、core/tool-runtime/runtime-pi/workflow/cliの5package build。
- `mise exec -- node scripts/check-doc-ids.mjs`: exit0、523Markdown files。
- YAML syntax: installed yaml@2.9.1でnew5jobs/reusable1jobをparse、error0（依存変更なし）。
- Windows committed payloadの補助APM/native smoke: IMPL-034zUpcuJnZnznZ79UL3Q2を参照。freshCI buildではない。
- `release guide distinguishes acquisition and legacy paths`: manual review PASS。新1dispatch / 旧stricttrue / 現行LFS準備 / sums無しfailclosed / draft recoveryを確認。
- `distribution guide installs a plain local snapshot`: manual review PASS。root cheaperを展開しconsumerの../cheaperをAPM local dependencyとする。
- 最終maintenance plan→apply後のall audit: chain+共有ADR scoped blocking0、共有ADRの既存reciprocal warning2のみ。global blocking48はownership外。global PASSは主張しない。
- impl-record audit: 77IR / findings0。doc-idcheck: 523Markdown / exit0。

## Risks

Linux native2targetとUbuntu archive0755、GitHub API/draft/upload/publicationはCI-only。実releaseは発火していない。
現行APM/git payload/LFSは#73-Cまで残る。CopilotのJSON deploy制約は#73-A guide契約を維持する。

## Follow-ups

follow-up classification: terminal。追加のlocal bug/feature/doc taskはない。
#73-Cの既存scopeとCI-only実行項目はownerを変えずcoordinator/CIへ引き継ぐ。
canonical check deferred to coordinator merge run。

---
id: "DISC-034zIOnoIjhMd2vKGOtgXd"
type: "discovery"
status: "resolved"
title: "Core resource manifest identity investigation"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/ideas/core-resource-manifest-normalization.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../ideas/core-resource-manifest-normalization.md"
  derived-by:
    - "../specs/core-resource-manifest-normalization.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---

# Core resource manifest identity investigation

## Exploration Goal

workspace boundary を再実装せず D4 の identity と conflict を実現する。

## Entry Decision

A-4: dispatch と issue Triage が受け入れ条件を指定済み。dispatch 固有の skill 指示を優先し、idea-doc / discovery-doc / spec-doc を順に実行する。

## Constraints and Evidence

- issue #65 の Triage 2026-10-04 と accepted ADR D4 を確認した。型の path は絶対 real path で、workspace-relative は入力のみ。
- paths.ts の resolveWorkspacePath は最深存在祖先と不存在 tail を解決し、component ごとに symlink と escape を検査する。
- scaffold 不足は coordinator の指示でローカル main に rebase し、setup を再実行した。

## Options and Trade-offs

1. paths.ts を再利用する: policy が一元化される。採用。
2. resource 層で独自 realpath / glob enumeration を実装する: boundary の重複と future output の欠落を招く。不採用。

## Conclusion

ResourceClaim は literal scope を存在から推測しない。reads/writes glob は最初の glob component より前の subtree に近似する。produces は literal または明示 subtree の terminal ** のみとし partial glob を拒否する。

## Open Questions

なし。hard-link alias / 外部 process の symlink 差替えは既存 boundary の限界を継承する。

## Promotion Candidates

本 chain の spec に昇格する。追加 ADR は D4 の変更がないため不要。

## Execution Checkpoints

- mode: run-until-yield、初回 probe は focus-required。idea を focus に probe-to-briefing を選択した。
- spec gate は P1（resources {} vs []）を修正し、coordinator の APPROVED: spec を受領した。Windows は比較 key のみ case-fold とする。
- accepted 共通 ADR を derives-from に接続すると既存複数 plan に lineage が広がり、pre-plan router は focus-required。共有 skill の変更はせず coordinator に escalation した。
- baseline: mise run setup（main rebase 後も再実行）は exit 0、core tests は 23 files / 291 passed / 2 skipped、exit 0。
- repository-wide doc audit の既存 upstream / EXP legacy エラーは scope 外。自 chain の index は完了時の doc-maintenance で再生成する。

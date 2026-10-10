---
id: "ADR-034zThoekpLqCcHhYO4MPe"
type: "adr"
status: "superseded"
title: "One-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
  implements: []
  implemented-by: []
  depends-on: ["./version-pinned-launcher-acquisition.md"]
  blocks: []
  supersedes: []
  superseded-by:
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  related:
    - "../specs/one-dispatch-release-workflow.md"
    - "./distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  refines: []
  refined-by: []
  derives-from: ["../discovery/one-dispatch-release-workflow.md"]
  derived-by:
    - "../designs/one-dispatch-release-workflow.md"
    - "../plans/implement-one-dispatch-release-workflow.md"
  verifies: []
  verified-by: ["../test-specs/release-manifest-and-snapshot-integrity.md"]
  references:
    - "./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../designs/version-pinned-launcher-acquisition.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 同一 dispatch の検証済み SEA asset と staged APM snapshot を公開する

2026-10-04: APPROVED: 73b-spec-adr-design（msg_e6292456fa5b）により accepted。

## Context and Problem Statement

#73-A launcher は v<version> の strict SHA256SUMS と raw asset を要求する。
現行 release は LFS payload との byte 一致を検査し、build dispatch → download →
LFS commit → release dispatch の往復を必要とする。#73-B はその契約を保持したまま
同一 run 内の公開経路を追加する。2026-10-04 coordinator の
APPROVED: 73b-adr-intent（msg_4284b6070bc5）を受けて本 ADR を提案する。

## Decision Drivers

- version / source commit / raw bytes の一貫性、公開前の3 native target smoke。
- #73-A の manifest / asset / Node-free launcher 契約を維持。
- 旧 mandatory build / release / LFS 経路の保持、stdlib のみ、公開権限の限定。

## Considered Options

- 新 additive workflow + reusable native build + staged package snapshot（採用候補）。
- 既存 release の strict_freshness を外して置き換える（旧契約を破壊する）。
- LFS 往復だけを継続する（1 dispatch と manifest 公開を満たさない）。

## Decision Outcome

Chosen option: 新 additive workflow。release-acquisition.yml の tag input を canonical
version と照合し commit SHA に固定する。既存 build-executables を
strict_freshness:false で呼び3 targetを native runner で build / smoke。
旧 caller の true / byte 比較 / job 名は維持する。

同一 run の raw3 executable から strict SHA256SUMS を生成し、staging だけに
packages/apm の primitives / committed JS / version.json をコピーする。
旧 target payload をコピーせず同じ raw bytes を cli/bin/<target> に配置、
cli/SHA256SUMS を同梱して cheaper-apm-v<version>.tar.gz を作る。
archive は plain staged package snapshot（展開後 apm local path install）。
apm pack は arbitrary file を落とすため使用しない。

SHA256SUMS は UTF-8 ASCII subset / LF only（CR禁止）/ 64 hex + ASCII space
2個 + basename、空行なし、64KiB以下、3 target各exactly1、全basename duplicate禁止。
権威は acquisition design の Release Contract と launcher-acquisition.test.ts。

bundle を3 native runnerで展開し APM local install → Node を PATH から外した
launcher --version / models を検証する。publish jobのみ contents:write と
GITHUB_TOKEN を使用する。公開前に tag が固定SHAを指すことを再照合し、
existing Releaseを拒否、draftに全assetをuploadしてから公開する。latest推測、
上書き、local publishは行わない。

### Consequences

- 新経路の公開物は同一runのbytesを持ち、checkout payloadを変更しない。
- 移行中は新旧2 workflowが併存し、git-based consumerのLFS運用は#73-Cまで残る。
- CI native3 / draft publicationの実証は本workerでは未実行。stdlib test、
  dry-run、workflow構造検証を代替evidenceとし、CI PASSとは区別する。
- upload/公開失敗でdraftが残る場合はoperatorが確認・手動回復する。
  自動削除/再利用/上書きはしない。

## Compatibility and Related Decisions

旧 SEA ADR-034z1xMK43jRs3D79Urhpc は SEA 選択と旧配布経路を引き続き所有する。
related で接続し全体 supersede しない。acquisition ADRの固定契約を使う。
共有 ADR-034zGZpjoQlppwkkOs5gur D1（package依存）/ D6（CLI surface）を維持。
workspace lease / runtime / package には変更を加えない。

## Implementation Plan

- scripts/lib/release-artifacts.mjs: asset mapping / manifest / streaming hash /
  tag-version / staged snapshotの境界を所有。
- scripts/generate-release-checksums.mjs、assemble-release-bundle.mjs、
  smoke-release.mjs: stdlib の公開前検証・組立 CLI。publisher codeは置かない。
- scripts/test/release-artifacts.test.mjs: manifest / version / snapshot / smoke
  failure と新workflow構造を検証する。
- .github/workflows/release-acquisition.yml を追加。
  build-executables.yml には固定version伝達 / early gate / shared smokeだけ最小追加。
- docs/guides/release.md を新旧併存モデルに更新。LFS削除 / check / CI移行は#73-C。

## Verification

- [ ] stdlib focused suiteがmanifestの全invariantとraw/bundle一致・source非変更を検証。
- [ ] 構造検証がfixed SHA / version gate / native smoke / draft publish /
  contents権限 / existing Release拒否を確認。
- [ ] pnpm lint / pnpm typecheck / pnpm build がpass。
- [ ] chain + shared ADRのscoped exit-auditがpass。
- [ ] reportにcanonical check deferred to coordinator merge runとCI-only境界を記載。

## Revisit Conditions

#73-C移行、target追加、署名導入、release復旧方針またはAPM archive format変更時に再検討する。

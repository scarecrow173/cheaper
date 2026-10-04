---
id: "SPEC-034zWqZR8Dmu9TXTR2qcsg"
type: "spec"
status: "implemented"
title: "Payload untracking migration"
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
  implements: []
  implemented-by: ["PLAN-034zXjR2oE7USB76eUPCAq"]
  depends-on: []
  blocks: []
  supersedes: ["SPEC-034z14nGIf6fXRtTskkrk8"]
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["DISC-034zWqZnTAsRi8D1paYgln"]
  derived-by: []
  verifies: []
  verified-by: ["TSPEC-034zXB13nuorLP81dn8ro8"]
  references: ["ADR-034zWr0viJFoTiUALvhE6N","SPEC-034z14nGIf6fXRtTskkrk8","ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
---

# Payload untracking migration

## Summary
Git-based APM package を executable 不同梱に移行し、固定 version/platform の launcher acquisition を正式取得経路にする。

## Motivation
3 target 約 330 MB の binary を checkout・LFS・version bump から切り離す。consumer の Node 非依存性は維持する。

## Users and Value
親 Coding Agent の consumer は Node を入れず hook を実行でき、開発者は独立した fresh worktree を setup できる。

## Scope
Git/LFS の current payload 追跡、source payload check、既存 workflow freshness、release/distribution guide、移行 traceability。
旧 SPEC-034z14nGIf6fXRtTskkrk8 の SEA、supported target、CLI parity、stream/exit、version 整合性は継承する。APM binary の Git 同梱と byte freshness 要件を本仕様で置換する。
共有 ADR の package topology/tool/resource/workflow 契約は変更しない。この distribution slice に該当する D 番号はなく、runtime 契約に新たな policy を追加しない。

### Out of Scope
LFS 履歴 rewrite、launcher acquisition の再実装、依存変更、実 release dispatch/publish、generated cheaper.mjs の refresh。

## Required Behavior
- win32-x64/linux-x64/linux-arm64 の payload executable を index から除去し ignore する。LFS tracking pattern を除去する。既存 LFS 履歴は保持する。
- source payload check は launcher 2 種、hooks.json、CLI bundle と小さい runtime asset の存在・regular file・LFS pointer 拒否、version 宣言整合を検証する。platform executable の存在・サイズ・byte 比較・実行は必須にしない。
- fresh mise run setup は locked toolchain/dependency/APM skill/build を復元する。dev は workspace CLI を使う。mise run check は binary 不在で構造検証でき、bundle freshness は小さい tracked asset のみに限定する。
- release pipeline の fresh native build、canonical version 照合、Node-free bundle smoke は維持する。旧 release entrypoint は新 release-acquisition を案内して fail closed する。
- Windsurf/Claude/Codex/Cursor の Git-based consumer の apm install 後、sibling 不在の launcher が固定 release を SHA256 検証・lock・atomic install で取得する。shell の PATH から Node を除いた状態でも hook の stdin/stdout/stderr/exit を保つ。
- cold cache offline は launcher の cheaper launcher: download ... failed 診断と非 0 exit。verified cache offline は成功。旧 checksum 不在 Release の初回取得は fail closed。既存 install の旧 sibling binary は優先されるため再 install と旧 payload 除去または新 archive install を案内する。
- hooks.json のすべての PowerShell entry は cheaper.ps1 launcher を経由する。現行配線を維持する。
- release guide は version bump → small asset refresh/check/commit → immutable tag → release-acquisition 1 dispatch とする。binary download/commit の準備 dispatch は不要。

## Acceptance Criteria
1. Git index に 3 payload executable がなく、.gitattributes は LFS に選択せず、local build は ignore される。
2. non-Copilot target で binary のない package を apm install して launcher hook が Node-free で実行できる。network は local fixture に固定する。
3. source check は binary 不在を受け入れ、必須 asset の欠落・pointer・version drift を拒否する。
4. CI は committed payload の byte comparison を行わず、旧 release は publish せず失敗する。native version smoke は残る。
5. PowerShell hook routing、cold/warm offline、移行手順、1 dispatch guide が test/evidence と対応する。

## Sources and Relations
issue #73-C brief、DISC-034zWqZnTAsRi8D1paYgln、ADR-034zWr0viJFoTiUALvhE6N。canonical check は coordinator の merge run に defer する。

## Review 修正: 対応 consumer と外部 deploy 制約
binary 不同梱の Git-based acquisition 契約は Windsurf / Claude / Codex / Cursor に限定する。
APM 0.28.0 の Copilot deploy は *.json を除外し version.json が欠落するため、Git-based binary-free Copilot install を対応経路として案内しない。
Copilot は release archive（sibling executable 同梱）または明示的 local build の sibling を保持する。JSON deploy gap 解消は、Copilot の binary-free acquisition を有効化する前提となる別変更であり、本 slice では修正しない。
Copilot の exit 0 / allow fallback により起動 failure が deny を消すことを guide に明記し、移行時に旧 sibling を除去する指示は Copilot へ適用しない。
in-scope の Cursor も hook entry が launcher 失敗を `permission: allow` + exit 0 に変換するため、cold cache offline の非 0 exit 契約は launcher 層のものであり、Cursor host 境界では allow fallback となることを Copilot と同様に guide に明記する。
共有 hooks.json の launcher 配線の維持を検証することと、Copilot source install の runtime 対応を区別する。

## Review 修正: 移行の有効化と復旧
旧 release.yml の拒否は本変更から有効化する。初回 release-acquisition の実 publish 成功は未確認であり、source consumer の切替条件に全 5 asset の公開と native smoke 成功を置く。
失敗時は maintainer が新 workflow を修正して新 immutable version/tag で再実行する。旧 checksum 不在 publication へ fallback しない。
consumer は切替前に稼働中の manifest/ref と deploy directory を退避し、新 Release が未公開・検証失敗なら旧 manifest と sibling を含む deploy を復元する。Git payload を再 commit する rollback は行わない。

## Review 修正: status/index 移行対象
- ADR-034z1xMK43jRs3D79Urhpc と SPEC-034z14nGIf6fXRtTskkrk8 を本 ADR/SPEC へ superseded と reciprocal link で更新する。
- TSPEC-034z3yKWLolrImpMY8UMhZ (docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md) の同梱前提を新 migration test-spec へ supersede する。
- 各 generated README index を doc-maintenance plan/apply で同期する。AGENTS.md と ci.yml の現行 LFS 取得前提も更新する。

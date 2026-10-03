---
id: "ADR-034zIOoBXZtluQdBMlJQge"
type: "adr"
status: "accepted"
title: "Version pinned launcher acquisition"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/73"
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
  related:
    - "../specs/version-pinned-launcher-acquisition.md"
    - "./distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  refines:
    - "./distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  refined-by: []
  derives-from:
    - "../discovery/version-pinned-launcher-acquisition.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Version 固定 Release executable を verified user cache に取得する

## Context and Problem Statement

git-based APM の consumer hook を Node 非依存で維持しつつ、将来 binary を git/LFS から外せる取得経路が必要。APM remote URL install は使用できない。#73-A は LFS payload を残して取得 fallback を先行実装する。

## Decision Drivers

version 固定、公開整合性検査、offline/proxy 対応、stdin/argv/exit 保全、同時取得安全、Node 非依存、変更範囲の維持。

## Considered Options

1. version 固定 Release asset を native downloader で取得して user cache に配置（採用）。
2. LFS sibling のみ維持（取得 fallback を提供できない）。
3. Release bundle を consumer が手動 download/install（remote URL 制約による 2 step）。
4. Node downloader（Node 非依存要件に反する）。

## Decision Outcome

Chosen option: 1。sibling binary があれば互換経路で起動し、なければ cli/version.json で固定された v<version> Release の 3 target asset と SHA256SUMS を取得する。cache hit/取得時とも hash と --version を照合する。検証処理の stdin は null に接続し、通常起動の stdin は継承する。

cache root は CHEAPER_CACHE_DIR、なければ POSIX XDG_CACHE_HOME/cheaper（fallback HOME/.cache/cheaper）、Windows LOCALAPPDATA/cheaper/cache。key は <version>/<platform>。staging directory に binary と manifest を揃え、同一 filesystem の directory rename で key 全体を atomic publish する。同一 key の sibling lock directory 取得後に cache を再確認する。期限付き待機、通常 signal/例外 cleanup、stale lock は自動奪取なし。

manifest を解決できない取得・cache entry は検証不能として fail closed とする。現在の raw asset のみの Release は #73-B の manifest 公開まで fallback 取得に使用できず、sibling 互換経路は維持される。

production origin は https://github.com/scarecrow173/cheaper/releases/download。CHEAPER_RELEASE_BASE_URL は明示的な trusted mirror/test origin override（HTTPS または loopback HTTP）。SHA256SUMS は <64 hex><two spaces><asset basename> の LF 行で、asset 名は cheaper-linux-x64 / cheaper-linux-arm64 / cheaper-windows-x64.exe。asset 自体は raw executable。tag は v<cli/version.json version>。公開 checksum は誤配送/破損検出であり、origin の信頼や user cache owner の権限を超える署名保証は主張しない。

### Consequences

初回起動に curl（POSIX）または Invoke-WebRequest（Windows）と hash utility が必要。cache の manifest/binary を preseed すれば offline 起動可能。強制 kill で残った lock/staging は、該当 process がいないことを確認して手動削除する。supported targets の追加は Release build と同時に別変更で行う。

### Historical ADR / spec reconciliation

ADR-034z1xMK43jRs3D79Urhpc は既存実装済み SEA の履歴として proposed → accepted、SPEC-034z14nGIf6fXRtTskkrk8 は proposed → approved とし、旧本文を保持して #73-A の refine を追記する。本 ADR は SEA の Node 非依存選択を refines。ADR-034r05iHrZUE6W5HVbwRSz は cli/bin/cheaper.mjs が同梱・参照される間は有効であり本 ADR では related で接続する。同 ADR の supersede は payload/bundle 削除を行う #73-C の ADR が宣言する。LFS 同梱削除は #73-C が実施するまで維持する。変更は coordinator の gate 承認後に行う。

### Shared Contracts

ADR-034zGZpjoQlppwkkOs5gur D1/D6 を維持。D4 の workspace resource locking と acquisition locking は責務が異なり、runtime package は変更しない。

## Implementation Plan

- packages/apm/.apm/hooks/cheaper と cheaper.ps1 に対称な acquisition 契約を実装。
- hooks.json の powershell entry を cheaper.ps1 に統一（preToolUse の exit 0 と Cursor の allow fallback を含む既存 event ごとの semantics は維持）。
- packages/cli/test/e2e/ の新規 launcher tests に fetch/hash/concurrency/stdio/失敗を追加。
- docs/guides/distribution.md と cheaper-usage skill の取得経路説明を追加。
- workflow/scripts/cli 生成物/LFS payload/release guide は変更しない。

## Verification

sh/pwsh executable-path tests、既存 deployed-executable e2e、mise run check、document exit audit。実 public checksum asset の生成は #73-B の follow-up。

## Revisit Conditions

target 追加、署名導入、mirror/cache contract の変更、native downloader support の変更時に本 ADR を見直す。

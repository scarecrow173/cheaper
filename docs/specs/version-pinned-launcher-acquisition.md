---
id: "SPEC-034zIO2lm7gJPz6Gy4JIQP"
type: "spec"
status: "approved"
title: "Version-pinned launcher acquisition"
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
    - "../adr/version-pinned-launcher-acquisition.md"
  refines:
    - "./standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  refined-by: []
  derives-from:
    - "../discovery/version-pinned-launcher-acquisition.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---

# Version 固定 launcher acquisition 契約

## What Should Be Built

APM launcher が sibling binary を優先し、不在時に package 固定 version の supported host Release binary を検証済み user cache から起動する。POSIX sh と PowerShell から argv/stdin/exit code を保持し、hook の PowerShell entry も launcher を通す。

## Why Now

LFS を将来削除できる取得経路を先に検証する必要がある。現在は sibling 不在で hook が起動できず、PowerShell entry は launcher を bypass している。

## Users and Value

APM git install consumer と proxy/offline 利用者が、Node を導入せず再現可能な version の hook を実行できる。

## Scope

### In Scope

launcher 取得/version/cache/整合性/同時初回起動、PowerShell hook 経路、launcher e2e、新規 distribution guide、#73-A 文書チェーン。

### Out of Scope

#73-B Release workflow/manifest 生成、#73-C payload/LFS/check/CI/release guide 移行、Node SEA 実装変更、tool runtime/workflow/core の変更、依存追加。

## Requirements

- R1: supported targets は linux-x64/linux-arm64/win32-x64。未対応 OS/arch の組合せを明示拒否する。
- R2: sibling binary があれば従来の直接実行を維持する。LFS pointer は取得成功と扱わず明示失敗する。
- R3: sibling 不在時だけ cli/version.json の固定 version を使う。latest、別 version、破損 executable を実行しない。
- R4: cache は hook directory の外で version/platform ごとに分離。公開 SHA-256 と executable --version を照合し、cache hit でも検査する。
  SHA256SUMS を取得・提示できない場合、検証をスキップせず actionable stderr と非 0 exit で取得失敗とする。cache hit で manifest が欠落している場合も同様に失敗し、binary のみの cache entry は実行しない。
- R5: 同時 first-run では同一 key の取得は一度だけ。未検証状態を公開せず、通常取得失敗/中断の staging は削除する。待機は期限付きとする。
- R6: acquisition error は actionable stderr と非 0 exit。起動前の取得処理は stdin を消費せず、argv の境界と CLI exit code を保持する。
- R7: proxy/offline preseed、既存 install と local build の移行、lock recovery、supported target 制限を新規 guide に記載する。
- R8: 共有 ADR D1 の package 境界と D6 の CLI surface を維持し、launcher acquisition lock は D4 の resource lease を流用しない。

## Acceptance Criteria

- [ ] AC1: sh/pwsh の sibling/cache hit/cache miss（stub Release）を executable 経路で検証。
- [ ] AC2: 破損/不一致/中断 download、wrong version、unsupported platform、cache 書込不可は stderr + 非 0、通常失敗後に partial を残さない。
- [ ] AC3: 同時 2 process の first-run は binary/manifest の取得各 1 回で両 process 成功。
- [ ] AC4: hook event + stdin JSON を使い stdin passthrough と exit code を検証し、PowerShell hooks.json に exe 直呼びがない。
- [ ] AC5: 現行 sibling LFS payload の既存 deployed e2e と mise run check が pass。
- [ ] AC6: guide と終端 document chain の audit が pass。

## Deferred Design Concerns

公開 workflow と payload 削除は #73-B/C に defer。強制 kill 後の lock 自動回収は誤奪取を避け採用せず、手動復旧と次回明示失敗を記載する。

## Sources and Relations

issue #73 Triage と discovery に基づく。SPEC-034z14nGIf6fXRtTskkrk8 の Node 非依存 standalone 要件を refine する。

---
id: "TSPEC-034zQTnsUJl3pXIuYsvZIQ"
type: "test-spec"
status: "superseded"
title: "Version-pinned launcher integrity and stream preservation"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: ["TSPEC-034DmUtUjWCThCnJulok0c"]
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/version-pinned-launcher-acquisition.md"
  derived-by: []
  verifies:
    - "../specs/version-pinned-launcher-acquisition.md"
    - "../designs/version-pinned-launcher-acquisition.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Launcher integrity and stream preservation

## Purpose
固定versionの検証とhook入出力の境界を守る。acquisition契約が廃止された場合のみretireする。

## Feature
Node無しでsiblingまたはverified user cacheからCLIを起動する。

## Rules
- Rule: 全hostでsiblingを優先しLFS pointerは拒否する。
- Rule: acquisitionは3target、固定tag、公開manifest/hash/version照合、binary+manifest pair。
- Rule: cache hitも再検証し、検証不能/不一致はexecしない。
- Rule: lock内で再確認し、staging全体をatomic公開。通常失敗はpartialを削除する。
- Rule: probeはstdinを消費せずargv境界/CLI exit/stdout/stderrを保持する。

## Examples
- Example: siblingはversion.jsonが無くても起動し、未対応hostも現行互換。
- Example: cache missはv1.0.0のmanifest+asset各1取得、hitはofflineで成功。
- Example: 同時2processは1取得で両成功、staging/lockが残らない。
- Example: binary-only/manifest-only/cache hash変更/manifest重複や欠落/wrong version/不正JSON/書込不可は非0+stderr。
- Example: broken/truncated downloadとplain HTTP redirectは拒否しpartial無し。
- Example: Windows ARM64 native archとPOSIX unsupported acquisitionは拒否。
- Example: hook JSONをstdin、空白/空文字/記号をargvに渡しexit2を保持。
- Example: PowerShell hook entryはexe直呼びが無く、Copilot exit0/Cursor allow fallbackを維持。
- Example: Copilotのversion.json不在は取得せず明示失敗。

## Guarantees
公開されたcache keyは検証済みpairでありlatest/別version/不正binaryを実行しない。普通の失敗後に自身のpartialを残さず、deployed hook directoryにcacheを書かない。

## Non-goals
公開Release workflowの生成、LFS削除、Copilot sidecar補完、SIGKILL lock自動回収、publisher署名の保証、user自身によるcache書換え耐性。

## Risk
検証を落とすと誤ったexecutableをhookとして実行し、stdin/exit破壊は親agentのdeny/allow判断を変える。

---
id: "TSPEC-034zXB13nuorLP81dn8ro8"
type: "test-spec"
status: "proposed"
title: "Payload-free source acquisition and native verification"
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
  supersedes: ["TSPEC-034z3yKWLolrImpMY8UMhZ"]
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/payload-untracking-migration.md"
    - "docs/designs/payload-untracking-migration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Payload-free source acquisition and native verification

## Purpose
source package に binary がなくても version-pinned launcher の取得・stream/exit と real CLI hook の契約を保つ。配布モデルが廃止された場合にのみ置換する。

## Feature
Git-based non-Copilot consumer の Node-free acquisition と native executable の検証。

## Rules
- source の必須 launcher/CLI asset/metadata は欠落・pointer・version drift を拒否する。platform binary の存在と byte 比較は要求しない。
- Rule: acquisition は固定 tag の checksum 検証後に pair を公開し、hook stdin/stdout/stderr/exit を維持する。
- Rule: cold offline は launcher の非 0 と download 診断。verified cache は offline で起動する。Cursor host の allow fallback と Copilot source JSON gap は明示的に区別する。
- Rule: native version/models/semantic deny の検証は local SEA fixture と release native smoke で継続する。
- Rule: Git payload/LFS と workflow byte freshness はない。旧 release は拒否し、公開用の取得経路は 1 dispatch の新 workflow。

## Examples
1. binary を含まない source fixture の check は成功し、cheaper.ps1/worker schema/version.json の欠落や pointer は失敗する。
2. apm install 後に sibling がなく、Node を除いた PATH で local HTTP fixture の v1.0.0/SHA256SUMS と host asset を取得して元の hook input と exit を返す。
3. localhost fixture が停止した cold cache は stderr に cheaper launcher: download ... failed、tagged Release/SHA256SUMS と offline preseed の案内を出して失敗する。verified pair は停止した server なしで成功する。
4. fresh local SEA を user cache に seed した consumer は --version、models、実 CLI の bulk-read deny (exit 2 + bulk-read 診断) を返す。
5. Copilot source deploy は version.json 不在として記録し、binary-free runtime の成功とは呼ばない。Copilot archive/local sibling は保持する。Cursor の launcher failure は declared hook で permission: allow に変換される。

## Guarantees
実ネットワークと committed executable に依存せず、取得機構と real CLI semantic behavior を別の fixture で検証できる。existing launcher integrity/fallback tests を弱めない。

## Non-goals
実 GitHub publish、foreign OS の local 実行、provider generation、外部 APM Copilot JSON gap の修正。canonical check deferred to coordinator merge run。

## Risk
この検証を落とすと、不同梱 checkout の failure、silent hook deny loss、旧 checksum 不在 publish、version drift が再導入される。

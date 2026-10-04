---
id: "DISC-034zIO1UQe9CI5VCzqot1c"
type: "discovery"
status: "resolved"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../ideas/version-pinned-launcher-acquisition.md"
  derived-by:
    - "../specs/version-pinned-launcher-acquisition.md"
    - "../adr/version-pinned-launcher-acquisition.md"
    - "../designs/version-pinned-launcher-acquisition.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Version 固定 launcher acquisition の探索

## Exploration Goal

issue #73 本文と Triage 2026-10-04 に従い、LFS 非同梱時にも Node 非依存 hook を動かす取得責務を定める。Entry Decision: A-4（dispatch の scope/AC が明確）。dispatch-specific override: supervised worker が指定された文書スキルを inline 実行し、meta skill の再 dispatch は行わない。

## Key Issues and Assumptions

- 現行 sh は sibling 不在で失敗し、PowerShell hook は launcher を bypass する。
- Release asset は cheaper-linux-x64 / cheaper-linux-arm64 / cheaper-windows-x64.exe。現在の supported targets はこの 3 つ。Darwin は配布されない。
- build は Node --build-sea であり postject は未採用。root apm.yml は development-only で cheaper hook を deploy しない。
- freshness の byte 比較は workflow の cmp。check-apm-payload は target 存在・LFS pointer・host version の検査。
- APM source は git ref/local path/local packed bundle。remote release tarball の直接 install は不可。
- setup はローカル main 36d1c18 へ coordinator 指示で rebase 後に成功。共有 ADR D1/D6 の package/CLI surface を変更しない。acquisition lock は D4 の workspace resource lease と別責務。

## Constraints

#73-A では workflow/scripts/LFS payload/生成物/release guide を変更しない。依存追加なし。取得先は deployed hook directory の外。latest の参照なし。

## Alternatives Considered

| 案 | 利点 | 不利 |
| --- | --- | --- |
| version 固定 Release download | git-based APM を維持し Node 不要 | 初回 network、checksum/cache/lock が必要 |
| LFS payload 維持のみ | offline 互換 | runner 往復と LFS コストが残る |
| Release bundle を手動 download/install | 単純な payload | APM remote URL 制約で 2 step |
| Node downloader | JSON/lock 実装が容易 | consumer Node 非依存を失う |

## Tentative Conclusions

sibling 優先を維持し、不在時に verified user cache を取得する。cache hit も hash/version 検査を行い、offline preseed は executable と公開 SHA256SUMS を一緒に置く。version/platform ごとの lock と staging directory rename で公開を一度に行う。中断時に通常 signal の cleanup を行い、強制 kill 後の orphan は自動奪取せず期限付き失敗と手動復旧手順にする。

## Open Questions

実装前 blocker はなし。Release checksum の生成・公開は #73-B、LFS payload の削除と freshness/check/CI 移行は #73-C。本 slice は stub release と既存 payload で検証する。

## Promotion Candidates

同名 spec/ADR/design へ昇格する。旧 proposed SEA ADR と spec は実装済み履歴として accepted/approved に整合し、SEA 決定は refine する（coordinator 承認後）。committed esbuild ADR は cli/bin/cheaper.mjs が同梱・参照される間は related で接続し、supersede は payload/bundle 削除を実施する #73-C の ADR に繰り延べる。

## Research Evidence

issue #73 Triage、packages/apm/.apm/hooks/cheaper と cheaper.ps1/hooks.json、既存 deployed-executable e2e、build-executables workflow と build-executable script を read-only で確認した。

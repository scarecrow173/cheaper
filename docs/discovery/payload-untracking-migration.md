---
id: "DISC-034zWqZnTAsRi8D1paYgln"
type: "discovery"
status: "resolved"
title: "Payload untracking migration"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/ideas/payload-untracking-migration.md"
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
  derives-from: ["IDEA-034zWkslEDhnpFKJZpyzU4"]
  derived-by: ["SPEC-034zWqZR8Dmu9TXTR2qcsg","ADR-034zWr0viJFoTiUALvhE6N"]
  verifies: []
  verified-by: []
  references: ["SPEC-034z14nGIf6fXRtTskkrk8","ADR-034z1xMK43jRs3D79Urhpc"]
  defers: []
  deferred-by: []
---

# Payload untracking migration

## Exploration Goal
Git から 3 target の executable を除去しても APM consumer と開発 checkout が成立する境界を決める。

## Constraints
issue #73-A/B の launcher と release pipeline を再利用する。履歴 rewrite、依存変更、実 publish、payload の再 commit は行わない。
Entry Decision: A-4。dispatch の scope と acceptance が明示済みのため直接文書化する。子 worker は dispatch 共通ルールで禁止されているため inline 実行する。

## Evidence
- scripts/check-apm-payload.mjs は全 target の real binary と host version を要求する。source checkout の payload absence と両立しない。
- build-executables.yml は committed payload と cmp -s で比較する。release-acquisition.yml は strict_freshness: false で呼ぶ。
- hooks.json の PowerShell entry は既にすべて cheaper.ps1 を経由する。既存配線を維持し test で固定する。
- 旧 ADR は accepted、旧 SPEC は approved。brief の proposed という snapshot は現行 checkout と異なる。
- launcher は sibling 不存在時に固定 version/platform の user cache と release acquisition を使う。

## Alternatives and Trade-offs
| 選択肢 | 利点 | 問題 |
| --- | --- | --- |
| LFS 同梱の維持 | 初回 offline 利用 | checkout と release の binary commit 負担を解消しない |
| check ごとに local SEA を build | host version を実行検証 | 全 target の存在は検証できず canonical check に重い build を追加する |
| source は構造検証、release は native smoke | 役割と evidence の境界が明確 | cold-cache consumer は固定 Release への接続が必要 |

## Conclusions
source checkout は launcher・小さい CLI asset・version 宣言を検証する。executable version は release build と native smoke の責務として維持する。
dev setup/build は Node の workspace CLI を使い、hook の executable が必要なときは launcher acquisition または明示的 local build を使う。
旧 release.yml は非互換公開を続けず、明示的診断を出して失敗する retired entrypoint にする。

## Open Questions
coordinator の仕様・設計承認が必要。CLI acquisition test の最小変更を行う場合は許可を得る。

## Promotion Candidates
SPEC-034zWqZR8Dmu9TXTR2qcsg と ADR-034zWr0viJFoTiUALvhE6N に昇格済み。

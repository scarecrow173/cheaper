---
id: "ADR-034zWr0viJFoTiUALvhE6N"
type: "adr"
status: "superseded"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: ["ADR-034z1xMK43jRs3D79Urhpc"]
  superseded-by:
    - "docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  related: []
  refines: []
  refined-by: []
  derives-from: ["DISC-034zWqZnTAsRi8D1paYgln"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["SPEC-034zWqZR8Dmu9TXTR2qcsg","ADR-034z1xMK43jRs3D79Urhpc","ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Payload untracking migration

## Context and Problem Statement
Git/LFS の SEA payload 同梱と byte freshness は acquisition と 1 dispatch release の導入後も残り、binary 不在の checkout を拒否している。

## Decision Drivers
- Node 非依存 consumer hook と fail-closed integrity を保つ。
- source checkout と release artifact の責務を分ける。
- 過去 LFS 履歴と既存 consumer を壊す履歴操作を行わない。

## Considered Options
- LFS 同梱を継続する。
- canonical check に local SEA build を追加する。
- Git source は構造検証、executable は固定 release/acquisition に委ねる。

## Decision Outcome
第 3 案を採用する。ADR-034z1xMK43jRs3D79Urhpc を supersede し、Node 26 SEA・3 supported target・CLI parity・stream/exit preservation の判断を継承する。Git 同梱・freshness のみを置換する。
source check は必須 asset と version 宣言を検証する。executable の --version は build-executables と release-acquisition native smoke の検証として保持する。
旧 release workflow は retired fail-closed entrypoint にし、checksum を持たない新規 Release を生成させない。

### Consequences
checkout の binary 取得と commit が不要になる。cold-cache の初回 consumer hook は release network を必要とし、offline では明示的失敗になる。
verified cache と release archive/local build sibling は offline 利用できる。既存 install の sibling は優先されるため guide に更新・除去手順を明記する。

## Pros and Cons of the Options
LFS 維持は offline 初回利用を保つが issue の問題を解消しない。local build の強制は重く cross-target check を解決しない。構造検証と release smoke の分離は既存 A/B の成果を再利用する最小の責務変更となる。

## Confirmation
SPEC-034zWqZR8Dmu9TXTR2qcsg の acceptance、focused launcher/APM suites、source checker negative fixtures、workflow pins、scoped exit-audit で確認する。

## More Information
承認前の status は proposed。old ADR は現行 accepted であり、承認後に superseded と reciprocal link を設定する。

## Review 修正: consumer scope と release 復旧
Git-based binary-free acquisition は Windsurf / Claude / Codex / Cursor が対象。Copilot の JSON deploy gap は外部 APM 制約の別変更として残し、Copilot は release archive/local build sibling を保持する。
旧 release.yml はこの移行と同時に拒否する。初回新経路 publish が失敗した場合は新 workflow の修正と新 immutable tag で復旧し、旧 publication を再開しない。
consumer の切替は全 5 asset の公開・native smoke 成功後とし、失敗時は事前退避した旧 manifest/deploy（sibling を含む）を復元する。binary の Git 再追加はしない。

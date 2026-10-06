---
id: "DESIGN-034zWr0ZeceJCJaL3ZRK3H"
type: "design"
status: "approved"
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
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["SPEC-034zWqZR8Dmu9TXTR2qcsg","ADR-034zWr0viJFoTiUALvhE6N"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
---

# Payload untracking migration

## Purpose
SPEC-034zWqZR8Dmu9TXTR2qcsg の source/acquisition/release 境界を既存実装で成立させる。

## Architecture
Git APM snapshot → apm install → cheaper / cheaper.ps1 → sibling（local/release archive）または version/platform cache → SHA256/lock/atomic acquisition → executable hook。
source checkout の setup/build は Node workspace CLI。release-acquisition は固定 SHA native build → archive/checksums → native Node-free smoke → draft 全 asset upload →公開。

## Components and Responsibilities
- .gitattributes の payload filter を除去し、.gitignore の `cli/bin/*/` で 3 payload と foreign target の local output の accidental add を防ぐ。git rm --cached で履歴を保持する。
- scripts/check-apm-payload.mjs の checkApmPayload(root) と CLI を同一実装にする。必須 launcher/hooks/small CLI assets の regular-file/pointer/JSON metadata 検証と version 一致を担う。foreign binary の存在は要求しない。
- build-executables.yml の strict_freshness input と matrix payload、cmp step、LFS checkout を除去する。release-acquisition caller の不要 input を除去する。旧 release.yml は contents: read の拒否 job とする。
- mise check は既存 task を維持する。untracking 後の diff は small tracked CLI assets のみを対象とする。
- hooks.json は現行 launcher entry を維持し tests で実際の PowerShell command を検証する。

## Data Flow and Interfaces
構造 checker は root 引数を内部 test seam に持ち、entrypoint は import 時に exit しない。必須 asset 欠落/pointer/metadata drift を診断して CLI は exit 1。既存 tool-runtime と workflow の公開契約は変更しない。
consumer integration fixture は source package snapshot から payload を除き apm install する。download は local fixture server、PATH から Node 除外、PowerShell launcher hook を stdin 付き実行する。

## Error Handling and Offline Behavior
acquisition 診断と fail-closed cache validation は #73-A を再利用する。旧 release の checksum 不在、cold offline、download/hash failure は非 0。warm verified cache は network 不要。

## Migration
新 immutable tag を取得して apm install し、古い deploy の sibling binary が残る場合は package の cli/bin/<target>/cheaper[.exe] のみを除去して launcher acquisition を使う。offline consumer は新 release archive を local install または verified cache を事前取得する。dev の旧 LFS payload は不要、local binary は ignore される。

## Verification
source checker fixtures、workflow tests、既存 launcher acquisition/APM install focused suites と consumer Node-free fixture。lint/typecheck/build を実行し、generated cheaper.mjs は変更しない。canonical check deferred to coordinator merge run。

## Risks and Non-goals
checksum 未公開の version では Git-based cold acquisition は失敗する。新仕様の公開 release を指定する。実 publish、history rewrite、依存変更、launcher再実装は行わない。

## Review 修正: 既存 tests の disposition
| File | 移行後の責務 |
| --- | --- |
| packages/cli/test/e2e/apm-install-targets.test.ts | committed win32 binary の存在 assertion を source snapshot の不同梱 assertion に置換。全 target の small assets/config と既存 Copilot JSON gap assertion を維持する |
| packages/cli/test/e2e/deployed-executable.test.ts | ローカルに fresh SEA fixture を生成し verified user cache に seed。binary 不同梱 package の apm install 後、両 OS とも launcher を使い Node-free --version/models/実 CLI hook deny を検証。GitHub download を禁止する |
| packages/cli/test/e2e/launcher-acquisition.test.ts | 421-422 行の committed real CLI binary 必須を fresh SEA fixture に置換。既存 cache/acquisition/stream/exit tests を保持し、APM install → sibling 不在 → local HTTP fixture acquisition → hook と cold-cache offline 診断を追加する |
| scripts/test/release-workflow.test.mjs | strict_freshness caller assertions を廃止済み input/byte comparison の不存在と旧 release 拒否に置換。native version と 1 dispatch publish ordering を維持する |
| packages/cli/test/e2e/native-executable-fixture.ts (新規) | scripts/build-executable.mjs --out-dir を unique dist directory に呼び、local host SEA fixture を生成・掃除する。committed payload/Release network に依存しない |

## Review 修正: consumer の境界と変更対象
Git-based acquisition の対応は Windsurf/Claude/Codex/Cursor。Copilot は sibling を持つ release archive/local build に限定し、JSON deploy gap と fallback による silent deny loss を guide で明示する。旧 sibling の除去手順は Copilot に適用しない。
.gitignore は cli/bin/*/ を ignore し、foreign target の local build も accidental tracking から除外する。git index の binary 不在を test で固定する。
AGENTS.md と .github/workflows/ci.yml の LFS 説明・lfs:true を source model に更新する。scripts/lib/apm-bundle.mjs の LFS と記した説明のみ更新する（実装変更不要・hot-file 許可を ask）。
mise.toml は変更しない。既存 diff 対象は untracking 後に small tracked CLI assets のみとなり、構造 checker の置換で payload 不在を受理できる。

## Review 修正:旧 artifact と復旧
旧 ADR-034z1xMK43jRs3D79Urhpc、SPEC-034z14nGIf6fXRtTskkrk8、TSPEC-034z3yKWLolrImpMY8UMhZ と respective index を supersede/同期する。
旧 release.yml 拒否の有効化は本変更から。新 publish 未検証であること、初回新 Release 成功前に Git-based consumer を切り替えないこと、失敗時の旧 manifest/deploy 復元と maintainer fix-forward を guide に明記する。

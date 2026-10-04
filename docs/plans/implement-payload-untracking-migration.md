---
id: "PLAN-034zXjR2oE7USB76eUPCAq"
type: "plan"
status: "approved"
title: "Implement payload untracking migration"
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
  implements:
    - "docs/specs/payload-untracking-migration.md"
  implemented-by: ["TASK-034zXB1vK84hUCtRlEyiqG","TASK-034zXB21CtI0XLnu6QAHPe","TASK-034zXB2z3uWw5lq9IyKOqe"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/payload-untracking-migration.md"
    - "docs/designs/payload-untracking-migration.md"
  derived-by: []
  verifies: []
  verified-by: ["TSPEC-034zXB13nuorLP81dn8ro8"]
  references: []
  defers: []
  deferred-by: []
---

# Implement payload untracking migration Implementation Plan

## Goal
SPEC-034zWqZR8Dmu9TXTR2qcsg の不同梱 source package、固定 acquisition、native release 検証を成立させる。

## Scope
単一の distribution migration として 3 task unit に分ける。payload 履歴 rewrite、実 dispatch/publish、依存変更、generated cheaper.mjs refresh、Copilot JSON deploy gap の実装修正は対象外。

## Upstream Documents
- SPEC-034zWqZR8Dmu9TXTR2qcsg (approved)、ADR-034zWr0viJFoTiUALvhE6N (accepted)。
- coordinator から APPROVED: spec-approval / adr-acceptance / design-approval を受領。Cursor の allow fallback 注意と cli/bin/*/ への統一は coordinator の直接変更を保持する。

## Design Inputs
DESIGN-034zWr0ZeceJCJaL3ZRK3H (approved)。Git-based acquisition は Windsurf/Claude/Codex/Cursor。Copilot は release archive/local build sibling を保持する。cold offline の非 0 は launcher 層の契約であり Cursor host entry は allow fallback する。

## File Map
| File | 責務 |
| --- | --- |
| .gitattributes / .gitignore / git index | LFS pattern と 3 tracked binary の除去、cli/bin/*/ の local output ignore |
| scripts/check-apm-payload.mjs / scripts/test/apm-payload.test.mjs | 必須 launcher/small asset の構造・pointer 拒否と version 整合を checker と実 CLI fixture で検証 |
| .github/workflows/build-executables.yml / release-acquisition.yml / release.yml / ci.yml | freshness 廃止、native version smoke 維持、旧 publish の拒否、LFS checkout 不要化 |
| scripts/test/release-workflow.test.mjs | 既存 strict_freshness assertion を置換し新 CI contract を pin |
| packages/cli/test/e2e/native-executable-fixture.ts (新規) | unique repository dist directory への local SEA fixture build/cleanup |
| packages/cli/test/e2e/deployed-executable.test.ts | committed binary を local native fixture の verified cache に置換し Node-free version/models/実 hook deny を維持 |
| packages/cli/test/e2e/launcher-acquisition.test.ts | committed actual binary を fresh native fixture へ置換、non-Copilot APM install/local HTTP acquisition/cold offline 診断を追加 |
| packages/cli/test/e2e/apm-install-targets.test.ts | source binary 不在 assertion、既存 target config/Copilot deploy gap 検証を維持 |
| AGENTS.md / docs/guides/release.md / distribution.md | source/dev/consumer 移行、1 dispatch、offline と fallback、切替条件・復旧 |
| scripts/lib/apm-bundle.mjs | LFS と記したコメントのみ更新。処理は変更しない |
| 旧 ADR/SPEC/TSPEC と generated README | reciprocal supersede と doc-maintenance plan/apply による status/index 同期 |

## Implementation Sequence
1. source checker と tracking の contract tests を先に追加し、binary-free fixture と Git index の違反を RED で確認する。
2. checker を構造/metadata 検証へ置換する。3 binary を git rm --cached し、当該 worktree の binary file のみ削除する。ignore/LFS tracking を更新する。
3. source checker focused tests と production checker CLI を GREEN にする。task/IR evidence を記録して論理的変更を commit する。
4. workflow の freshness/legacy publication tests を変更して RED を確認する。
5. build workflow の比較/input/LFS を除去し、release-acquisition の caller を更新する。旧 release は contents: read の refusal job にする。CI と AGENTS、release guide、bundle comment を整合させる。
6. workflow focused tests を GREEN にし、task/IR evidence と共に commit する。
7. native fixture helper と existing consumer tests を移行する。committed binary/実 GitHub acquisition の依存を解消し、real CLI semantic deny を保持する。
8. local HTTP + Node-free PATH の install/acquisition test と cold offline 診断を追加する。consumer focused suite と必要な単独 file を GREEN にする。
9. distribution migration と rollback guide、旧文書の supersede、new test-spec/task/IR traceability を完了する。lint/typecheck/build と scoped exit-audit を実行し、coordinator review gate を ask する。
10. review の修正後、生成 index と IR status を同期し、commitGate で論理的変更を commit する。worker_done は完了時に 1 回のみ。

## Dependencies and Risks
Task A (source tracking/checker) → Task B (workflow/guide) → Task C (consumer verification/closure)。実行は順次。fresh setup は exit 0、baseline acquisition は 45 passed / 5 skipped。
Local native fixture は current mise の Node 26 と pnpm build 済み dist を使う。unique dist directory に build するため parallel CI worker の出力を共有しない。
Copilot source install は JSON gap により非対応。archive/local sibling の除去を勧めない。Cursor の launcher failure は host hook で allow に変換されることを明示する。
checksum 未公開の旧 Release に cold acquisition を期待しない。初回新経路 publish はこの worker では実行せず、全 5 asset と native smoke の確認を consumer 切替条件とする。

## Task Breakdown
| Unit | 作業境界 | Done evidence |
| --- | --- | --- |
| A: Untrack payload and validate source assets | checker/test、attributes/ignore/index | checker CLI と script suite pass、tracked binary 無し、pointer/metadata negative fixtures pass |
| B: Migrate native workflows and release guidance | CI/test、AGENTS、release guide、bundle comment | workflow tests pass、cmp/strict_freshness 無し、native version smoke 維持、legacy publish 拒否 |
| C: Verify consumer acquisition and close migration | native fixture、3 CLI e2e files、distribution/status/index | non-Copilot install/Node-free acquisition、real native hook deny、offline/fallback、lint/typecheck/build と scoped exit-audit |

## Verification
| Step | Runnable command / expected outcome |
| --- | --- |
| source checker fixtures | mise exec -- node --test scripts/test/apm-payload.test.mjs — exit 0 |
| production source checker | mise exec -- node scripts/check-apm-payload.mjs — exit 0、binary 不在を受理 |
| release structure/artifact unit | mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs — exit 0 |
| acquisition unit file | mise exec -- pnpm --filter ./packages/cli exec vitest run test/e2e/launcher-acquisition.test.ts — exit 0 |
| deployed real CLI file | mise exec -- pnpm --filter ./packages/cli exec vitest run test/e2e/deployed-executable.test.ts — exit 0 |
| APM targets file | mise exec -- pnpm --filter ./packages/cli exec vitest run test/e2e/apm-install-targets.test.ts — exit 0 |
| target package focused suite | mise exec -- pnpm --filter ./packages/cli exec vitest run test/e2e/launcher-acquisition.test.ts test/e2e/deployed-executable.test.ts test/e2e/apm-install-targets.test.ts test/e2e/apm-install.test.ts test/e2e/apm-package-layout.test.ts — exit 0 |
| static/build | mise exec -- pnpm lint / mise exec -- pnpm typecheck / mise exec -- pnpm build — 各 exit 0 |
| route/audit | mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --current exit-audit --focus docs/specs/payload-untracking-migration.md --json — scoped terminal route |
full mise run check は merge 時の coordinator serial run に defer。canonical check deferred to coordinator merge run。

## Rollback and Follow-up
consumer は移行前に稼働 manifest/ref と sibling を含む deploy directory を退避する。全 asset の公開前・新 install/取得が失敗した場合は旧 manifest/deploy を復元する。
maintainer は新 workflow の失敗を修正し新 immutable version/tag で fix-forward する。旧 checksum 不在 publication、Git binary 再追加、tag rewrite は復旧経路にしない。
Copilot JSON deploy gap は別変更の blocking 前提（binary-free Copilot を後日有効化する場合のみ）。本 slice の対象 consumer とは区別する。

## Open Questions / Gaps
実 provider と実 publish は未検証で scope 外。foreign OS native build/smoke は workflow contract/unit と CI matrix が担い、Windows focused run を全 OS の実証と呼ばない。

## Self-Review
全 acceptance は A/B/C に対応する。4 既存 payload-dependent test file の disposition と CLI test hot-file 許可を確認済み。coordinator の未 commit 直接修正を保持する。生成 index は手編集しない。

## Implementation Handoff
dispatch 共通ルールの子 run 禁止に従い inline 実行する。review は coordinator が担当する。実装前に each task の IR を開き、失敗/修正/検証と graph checkpoint を記録する。

## Review Handoff
plan-approval を ask し、APPROVED を受けるまで test-spec/task/実装を開始しない。upstream の spec/ADR/design は承認済み。

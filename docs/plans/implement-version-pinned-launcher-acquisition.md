---
id: "PLAN-034zQHj1WyOgBNLGbDaFY8"
type: "plan"
status: "completed"
title: "Implement version-pinned launcher acquisition"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: ["../specs/version-pinned-launcher-acquisition.md"]
  implemented-by:
    - "../tasks/add-launcher-acquisition-contract-tests.md"
    - "../tasks/implement-native-launcher-acquisition-and-hook-routing.md"
    - "../tasks/document-launcher-distribution-and-verify-acquisition.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/version-pinned-launcher-acquisition.md"
    - "../adr/version-pinned-launcher-acquisition.md"
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/version-pinned-launcher-integrity-and-stream-preservation.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Version 固定 launcher acquisition 実装計画

## Goal / Scope

sibling 優先を保持し、不在時に検証済み固定 version の Release binary を user cache へ atomic に取得する。sh/pwsh の対称 contract と hook の互換性が同じ critical path / release intent を持つ単一 stream。#73-B/C の workflow / payload 削除 / version sidecar 補完は含まない。hot file / dependencies / 生成物を変更しない。

## Upstream and Design Inputs

承認済み SPEC-034zIO2lm7gJPz6Gy4JIQP、accepted ADR-034zIOoBXZtluQdBMlJQge、approved DESIGN-034zQkHD65kAeG9xVGSoFX に従う。coordinator design review の修正: sibling 優先は全 host、acquisition のみ3 target、Copilot の version.json 不在は取得せず fail closed。共有 ADR D1/D4/D6 を維持。

## File Map

| Path | Responsibility |
| --- | --- |
| packages/apm/.apm/hooks/cheaper | POSIX acquisition、sibling exec、signal cleanup |
| packages/apm/.apm/hooks/cheaper.ps1 | Windows native acquisition、検証、argv/stdin/exit |
| packages/apm/.apm/hooks/hooks.json | 子 PowerShell launcher 経由と既存 fallback |
| packages/cli/test/e2e/launcher-acquisition.test.ts | HTTP stub / cache / failure / concurrency / stdio / hook e2e |
| packages/cli/test/e2e/fixtures/launcher-fixture.cs（必要時） | Windows の小さい native PE test fixture、Node 不要 |
| docs/guides/distribution.md | proxy、pair preseed、migration、recovery、制限 |
| packages/apm/.apm/skills/cheaper-usage/SKILL.md | acquisition 経路の説明追記のみ |
| 自 chain の test-spec/tasks/impl と既存 spec/ADR/discovery/design | traceability、検証 evidence、終端 status |
| docs の generated README indexes | doc-maintenance plan → apply のみ |

## Implementation Sequence / Task Breakdown

| Step | Action | Depends on | Spec |
| --- | --- | --- | --- |
| A | acquisition contract の failing e2e を作成する | — | R1–R6 |
| B | focused test で現 launcher が新契約を満たさないことを確認する | A | AC1–AC4 |
| C | POSIX launcher acquisition を実装する | B | R1–R6 |
| D | Windows launcher acquisition を実装する | C | R1–R6 |
| E | hooks.json を子 PowerShell launcher 経由に変更する | D | R6 / AC4 |
| F | focused tests を実行し全 acquisition/stream contract を検証する | E | AC1–AC4 |
| G | distribution guide と usage skill 追記を作成する | F | R7 |
| H | canonical check、doc-maintenance、exit audit と evidence を完了する | G | AC5–AC6 / R8 |

task 文書は coherent slice として test harness（A/B）、launcher+hooks（C–F）、guide+close（G/H）に分割し、各 task 内の個別 action を明示する。test red → 実装 → green の順を守る。追加の agent は使わず inline で同じ critical path を進める。

C/D の実装順は fixture に使える runner を考慮して入替可能。ただし取得/検証/公開/exec の各 boundary を通した thin slice を green にしてから failure/concurrency coverage を広げる。既存 sibling は network/version/cache を参照せず、unsupported acquisition host と version.json 不在は download 前に fail closed。LFS pointer は両実装で拒否。

manifest（全行構文、対象1件、64KiB）、hash、stdin を閉じた version probe、same-filesystem directory rename、120秒 bounded exclusive lock、通常失敗 cleanup を実装する。POSIX mv 前は key 存在 guard を置く。download redirect は両 launcher で HTTPS のみ（loopback test origin は HTTP 許容）とする。POSIX curl は `--proto-redir -all,https`（同等の scheme 制限）を指定し、PowerShell は curl.exe の同 protocol restriction または IWR の自動 redirect 無効化+手動検証で実装する。非 loopback plain HTTP への downgrade は受理しない。既存 cache が不正なら再取得で隠さない。cache pair / version の検証を hit 時にも行う。

## Verification Matrix

| Step | Command / observable criteria | Pass |
| --- | --- | --- |
| A/B | mise exec -- pnpm exec vitest run packages/cli/test/e2e/launcher-acquisition.test.ts | red が取得未実装/未対応契約によるものと確認（syntax/harness error は不可） |
| C/D/E/F | 同 focused command | sibling/hit/miss、hash/manifest/version/permission/download失敗、2process各1取得、stdin/argv/exit、hook fallback が pass |
| G | mise exec -- node .agents/skills/doc-status/scripts/doc_status.js lint --type all --json | 自chain/guide に broken-link / invalid-front-matter なし |
| H | mise exec -- pnpm exec vitest run packages/cli/test/e2e/deployed-executable.test.ts | 現行 LFS sibling e2e pass |
| H | mise run check | exit 0、出力末尾を report に保存 |
| H | mise exec -- node .agents/skills/doc-maintenance/scripts/doc_maintenance.js plan --json → apply --json | preview と一致し generated index を再生成 |
| H | mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --current exit-audit --focus docs/plans/implement-version-pinned-launcher-acquisition.md --signal exit-audit-pass --json | focused gate pass、terminal complete |

Windows worktree は pwsh と Git Bash sh が利用可能。Windows PE fixture（実 CLI または C# executable）と loopback HTTP で Node-free launcher を検証する。Linux CI は sh と host executable。利用できない runner は明示 skip。version.json 不在、manifest欠落/重複/不正、binary-only cache、manifest-only cache、wrong version、broken/truncated download、cache parent regular file、unsupported acquisition（Windows ARM64 host 等の native-arch 判定を含む）、LFS pointer、stub 302 → 非 loopback / plain HTTP target の拒否、stdin JSON hook deny exit2 と cache no-network を含む。argv の空文字/空白/記号は fixture に echo させ境界一致を確認する。probe が hook stdin を消費しないことも検証する。

全 e2e case は `CHEAPER_CACHE_DIR` と `HOME`/`XDG_CACHE_HOME`/`LOCALAPPDATA`（host 該当分）を fixture temp dir へ向け、実 user cache / HOME を変更しない。hooks.json の全 PowerShell entry が `powershell -File ./cheaper.ps1` 経由で `cli/bin/win32-x64/cheaper.exe` 直呼びがないことも静的に検証する。

## Risk Register

| Risk | Likelihood / impact | Mitigation |
| --- | --- | --- |
| staging/key 誤削除による data loss | low / high | cleanup は取得した lock/stage に限定。既存 key を上書き/暗黙修復しない |
| PowerShell wrapper の stdin/argv/fallback 破壊 | medium / high | 子 process、null probe、hook entry 実行、PE argv fixture |
| Release manifest がまだ公開されない | high / medium | 現在は fail closed、sibling 維持、stub 検証、#73-B を guide に明記 |
| Copilot deploy が version.json を除外 | high / high | 不在は fail closed、#73-C 前の別変更が必須と記録 |
| 強制kill orphan lock | medium / medium | 自動奪取せず bounded wait、guide の手動復旧 |
| 並列 e2e 20秒 timeout flake | medium / medium | 該当 file 単独 rerun、scope/evidence を report 明記 |
| cached executable user 書換え | low / high | hit hash/version 検証、user-owned cache trust の限界を明記 |

## Rollback Strategy

公開 migration / dependency / Git history 変更は無し。launcher regressions の場合 coordinator が該当 logical commit を revert し sibling 経路に戻す（worker は destructive Git 操作をしない）。既存 cache は自動削除しない。user は process 不在確認後に対象 version/platform key のみ手動削除し sibling または正しい pair を配置する。rollback の確認は deployed-executable e2e と focused sibling case の exit0。

## Self-Review / Review Handoff

R1–R6 → A–F、R7 → G、R8/AC5–6 → H と対応済み。file ownership / fixed version / manifest format は design と一致。外部公開と Copilot sidecar 補完の依存は意図的 defer であり本実装で隠さない。plan は coordinator の承認後に approved にし、その後 test-spec/task を作成する。

## Implementation Handoff / Graph

inline 実行。共有境界を持つため独立 dispatch は不要。design/plan は coordinator 承認済み例外の手動 delegate 順を使用した。plan focus が存在するので承認後は --focus docs/plans/implement-version-pinned-launcher-acquisition.md で router に戻る。implementation commitGate は authorized Conventional Commit で解決し push/PR は行わない。

## 2026-10-04 coordinator verification policy update
4 worker同時canonical checkによるhost contentionのため、canonical check deferred to coordinator merge run。workerはfocused suite GREEN、失敗CLI file単独rerun、lint/typecheck/build passとdocs/IR/commitを完了する。mise run checkの成功を主張しない。

## Completion
focused suite45passed/5skipped、existing e2e1passed/1skipped、lint/typecheck/build pass。owned doc audit blocking0、IR audit findings0。graph exit-audit全prerequisite passを確認。canonical checkは明示的にcoordinator merge runへdefer。

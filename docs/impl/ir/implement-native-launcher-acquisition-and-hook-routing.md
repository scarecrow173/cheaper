---
id: "IMPL-034zR9OQeNgEdCN2OMdt8I"
type: "impl"
status: "completed"
title: "Implement native launcher acquisition and hook routing"
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
    - "../../tasks/implement-native-launcher-acquisition-and-hook-routing.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native launcher acquisition
## Summary
sh/pwshの固定version取得、verified pair cache、atomic publish、exclusive lockとhook entryを実装した。
## Context
approved spec/design/planと先行red、TSPECに従う。
## Implementation
### Changes
cheaper、cheaper.ps1、hooks.json。IWR fallbackの2保証caseを追加。
### Decisions During Implementation
siblingは全host優先。version不在は取得せずfail closed。POSIX mv前にkey不存在guard。curl redirectはHTTPS-only。PS Process APIでargv/stdioを保持しprobe stdinを閉じる。PS5.1 file parseだけで失敗しUTF8 ParseInputでは0errorだったためUTF8 BOMを付けた。
### Runtime Boundaries
same-filesystem staging pairをdirectory rename。hitもmanifest/hash/version検証。不正keyは暗黙修復しない。PS cacheのjunction/symlink祖先はboundaryを隠すため拒否。
## Related Experiments
### Adopted
なし。
### Rejected
なし。
## Validation
focused suite：45passed/5skipped(50)、exit0、181.94s。Windows hostのpwsh+Git Bash sh。5skipはrunner固有cases。hook WindowsPS5.1：1passed、IWR fallback：2passed。既存deployed-executable：1passed/1skipped、exit0、18.79s。pnpm lint/typecheck/build pass。lintの既存12.3MiB bundle size warning1件。
## Risks
native Linux実行はこのWindows runでは未検証（同test fileがLinux CIでshを使用）。公開manifestとCopilot sidecarは#73-B/C。SIGKILL orphanは手動復旧。
## Follow-ups
coordinator方針により canonical check deferred to coordinator merge run。公開Release smokeは本task範囲外。

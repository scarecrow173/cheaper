---
id: "TASK-IYUgfVARCrZwwsvW6aNfFq"
type: "task"
status: "done"
title: "check script + scripts/test 群の新 layout 追従"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/plans/apm-hook-per-target-package-split.md"
  changes:
    added: []
    modified:
      - {type: "file", path: "scripts/check-apm-payload.mjs"}
      - {type: "file", path: "scripts/check-doc-ids.mjs"}
      - {type: "file", path: "scripts/check-version.mjs"}
      - {type: "file", path: "scripts/set-version.mjs"}
      - {type: "file", path: "scripts/test/lib/apm-fixture.mjs"}
      - {type: "file", path: "scripts/test/apm-install-targets.test.mjs"}
      - {type: "file", path: "scripts/test/apm-payload.test.mjs"}
      - {type: "file", path: "scripts/test/apm-package-layout.test.mjs"}
      - {type: "file", path: "scripts/test/launcher-passthrough.test.mjs"}
      - {type: "file", path: "scripts/test/devin-hooks-install.test.mjs"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/apm-hook-per-target-package-split.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/apm-hook-packages-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/apm-hook-per-target-package-split.md"
    - "docs/tasks/apm-hook-packages-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/apm-hook-per-target-package-split.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "scripts/check-apm-payload.mjs"
    - "scripts/test/lib/apm-fixture.mjs"
  defers: []
  deferred-by: []
---

# Task: check script + scripts/test 追従

## やること

1. `scripts/check-apm-payload.mjs`:
   - `packages/*/.apm/hooks/` を glob 走査し、現行検査(asset 非空・
     extension allowlist・PE/ELF sniff・BOM/CRLF/shebang/LF・git mode
     100755・hooks.json↔launcher 整合)を各 package に適用。
   - `packages/apm` に `.apm/hooks/` が無いことを assert。
   - 4 package 間で `cheaper.sh` 同士・`cheaper.ps1` 同士が byte 一致。
   - wiring: 親 `apm.yml` の `dependencies.apm` が 4 sibling を正しい
     `targets:` で宣言していることを検査。
2. `scripts/check-doc-ids.mjs`: glob を `packages/*/.apm/**/*.md` へ。
3. `scripts/check-version.mjs` / `scripts/set-version.mjs`: 対象 manifest
   に `packages/apm-hook-*/apm.yml` を追加。
4. `scripts/test/lib/apm-fixture.mjs`: `makeConsumerLayout` が
   `packages/` 5 package を `pkgs/<name>/` にコピーし、consumer dep は
   `path: ../pkgs/apm`。`BUNDLE_DIRS` を per-target hook package 名
   (`.windsurf/hooks/apm-hook-windsurf` 等)へ更新。
5. `scripts/test/apm-install-targets.test.mjs`: 「native event のみ
   登録・foreign event key 不在・claude の copilot 由来重複不在」へ
   assertion を書き換え。
6. `scripts/test/apm-payload.test.mjs`: fixture source を
   `packages/apm-hook-windsurf/.apm/hooks` 等へ。
7. `scripts/test/apm-package-layout.test.mjs`: `packages/apm` に
   `.apm/hooks` 無し + 4 sibling 存在を assert。
8. `scripts/test/launcher-passthrough.test.mjs`: `cwd` を
   `packages/apm-hook-windsurf/.apm/hooks` へ。
9. `scripts/test/devin-hooks-install.test.mjs`: hooks.json 参照先を
   `packages/apm-hook-pretooluse/.apm/hooks/hooks.json` へ。

## Acceptance

- `node --test scripts/test/` グリーン(apm 利用可能環境)。
- `node scripts/check-apm-payload.mjs`・`check-version.mjs`・
  `check-doc-ids.mjs` グリーン。

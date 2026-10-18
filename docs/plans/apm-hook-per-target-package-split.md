---
id: "PLAN-V0rSMLrOYWCEJaFcsQHZAf"
type: "plan"
status: "approved"
test-spec-skip: "Verified by reworked scripts/test/apm-install-targets.test.mjs (per-target native-only install assertions = AC1/AC2 e2e), check-apm-payload/check-version script checks, and mise run check; no dedicated test-spec doc authored."
title: "APM hook 宣言 per-target サブパッケージ分離 実装計画"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/designs/apm-hook-per-target-package-split.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
    - "docs/designs/apm-hook-per-target-package-split.md"
  implemented-by:
    - "docs/tasks/apm-hook-packages-split.md"
    - "docs/tasks/apm-hook-split-test-follow.md"
    - "docs/tasks/apm-hook-split-docs.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/apm-hook-per-target-package-split.md"
    - "docs/tasks/apm-hook-packages-split.md"
    - "docs/tasks/apm-hook-split-test-follow.md"
    - "docs/tasks/apm-hook-split-docs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/apm-hook-per-target-package-split.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/apm.yml"
    - "packages/apm-hook-pretooluse/.apm/hooks/hooks.json"
  defers: []
  deferred-by: []
---

# Plan: APM hook 宣言 per-target サブパッケージ分離

## スコープ

`packages/` 配下の APM package layout、`scripts/` の check/test、
consumer 向け文書。runtime(`crates/`)の hook 挙動は不変。

## ファイルマップ

- `packages/apm/apm.yml`: `dependencies.apm` に 4 sibling `path:` dep を
  per-dep `targets:` 付きで追加。description 更新。
- `packages/apm/.apm/hooks/` 削除。
- `packages/apm-hook-{windsurf,cursor,copilot,pretooluse}/` 新規:
  `apm.yml`(一意 name・canonical version・target 宣言)、
  `.apm/hooks/hooks.json`・`cheaper.sh`・`cheaper.ps1`。
- `scripts/check-apm-payload.mjs`: glob 走査 + launcher byte 一致 +
  wiring 検査。
- `scripts/check-doc-ids.mjs`: `packages/*/.apm/**/*.md` へ拡張。
- `scripts/check-version.mjs` / `scripts/set-version.mjs`: sibling
  manifest を version contract に追加。
- `scripts/test/`: fixture・4 test file の追従(design §2.5)。
- 文書: `SKILL.md`、`docs/guides/distribution.md`、`AGENTS.md`、
  `CONTRIBUTING.md`、falsified doc set への note(design §4)。

## タスク

1. `docs/tasks/apm-hook-packages-split.md` — package 構造分割
   (sibling 作成・親 deps・旧 hooks 削除)。
2. `docs/tasks/apm-hook-split-test-follow.md` — check 系 script と
   `scripts/test/` 群の追従。
3. `docs/tasks/apm-hook-split-docs.md` — consumer 向け文書・repo 文書・
   falsified doc set の関連付け。

## リスク

- 中。consumer 配布面の構造変更。install 経路は fixture で実証済み
  (local 0.28.0/0.30.0、remote git-daemon)。`mise run check` の
  `node --test scripts/test/` に旧 layout pin が残るため test 追従が
  同一 PR 必須。

## 検証

- `node --test scripts/test/` (apm install e2e 含む)グリーン。
- `node scripts/check-apm-payload.mjs`・`check-version.mjs` グリーン。
- `mise run check` 全段グリーン。
- 実動作: fixture consumer install で各 target config に native event
  のみ・casing 警告が cursor false-positive のみ(実証済み、最終確認を
  e2e test で再実行)。

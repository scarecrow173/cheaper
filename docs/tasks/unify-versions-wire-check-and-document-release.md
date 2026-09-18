---
id: "TASK-034r05iHQeQpDqD0biyV0P"
type: "task"
status: "done"
title: "Unify versions, wire check, and document release"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/unified-version-source-of-truth.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-version-source-of-truth.md"
    - "docs/tasks/add-version-check-and-set-scripts.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/unified-version-source-of-truth.md"
    - "docs/specs/unified-version-source-of-truth.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Unify versions, wire check, and document release

## Work

DESIGN-034r05iHy2gdnoT4LG4KTu どおり、version 宣言を `1.0.0` に統一し、version check
を `mise run check` に組み込み、release 手順を文書化する。

### 1. 宣言の統一

- `packages/cli/package.json`、`packages/core/package.json`、
  `packages/runtime-pi/package.json` の `version` を `0.0.0` →
  `1.0.0` に変更。
- `node scripts/bundle-cli.mjs` で `version.json` を `1.0.0` に
  再生成する。bundle 本体（`cheaper.mjs`）に version 定数は埋め込ま
  れないため、`version.json` のみが変わることを `git diff` で確認。

### 2. `mise.toml` check task への組み込み

`check` の `run` 列に `node scripts/check-version.mjs` を追加
（`pnpm lint` の後）。

### 3. `docs/guides/release.md`（新規）

- canonical source（`packages/cli/package.json`）の明記。
- bump 手順: `node scripts/set-version.mjs <X.Y.Z>` →
  `node scripts/bundle-cli.mjs` → `mise run check` → commit →
  `--version` 目視確認。
- version check の仕組み（check script が `mise run check` / CI で
  自動実行されること）。
- root `apm.yml`（`cheaper-dev`）は製品 version と独立で対象外である
  ことの明記。

### 4. AGENTS.md / README.md の pointer

- AGENTS.md に release 手順は `docs/guides/release.md` を参照する旨の
  最小限の記述を追加（Git 運用節の近く）。
- README の release/version に触れる箇所があれば同様に。

### 5. 最終検証

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm lint
mise exec -- pnpm test
node scripts/check-version.mjs
mise run check
node .claude/skills/doc-status/scripts/audit_docs.js --type all
node packages/cli/bin/cheaper.js --version   # 1.0.0 を返すこと
```

## Acceptance

- [x] 全宣言・`version.json`・`--version` が `1.0.0` を返す。
- [x] `mise run check` が version check を含めて全 pass。
- [x] `docs/guides/release.md` が存在し、bump→検証→commit→確認の
      流れが記されている。
- [x] audit clean。

## Result

実施済み（2026-09-18）。宣言 3 file を `1.0.0` に統一し bundle 再生成
（`version.json` のみ変化、`cheaper.mjs` 非変化 — version 定数非埋込を
確認）。`mise.toml` check task に `check-version.mjs` を `pnpm lint` 後に
組み込み。`docs/guides/release.md` に canonical source・宣言一覧・
bump/release 手順・dev project 対象外を記述。AGENTS.md に pointer 追加
（README には該当節がなく追加せず）。

`mise run check` 全 pass: lint（warning のみ）→ version check OK: 1.0.0
→ typecheck → 656 tests（core 270 / runtime-pi 255 / cli 131）→
bundle 再生成 → freshness clean。`cheaper --version` = 1.0.0。

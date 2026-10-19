---
id: "IMPL-GJ30PMdgql2TCIEuURop04"
type: "impl"
status: "completed"
title: "APM hook 宣言の per-target サブパッケージ分離"
created: "2026-10-18"
updated: "2026-10-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/118"
  changes:
    added:
      - type: "file"
        path: "packages/apm-hook-windsurf/apm.yml"
      - type: "file"
        path: "packages/apm-hook-windsurf/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/apm-hook-windsurf/.apm/hooks/cheaper.sh"
      - type: "file"
        path: "packages/apm-hook-windsurf/.apm/hooks/cheaper.ps1"
      - type: "file"
        path: "packages/apm-hook-cursor/apm.yml"
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/cheaper.sh"
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/cheaper.ps1"
      - type: "file"
        path: "packages/apm-hook-copilot/apm.yml"
      - type: "file"
        path: "packages/apm-hook-copilot/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/apm-hook-copilot/.apm/hooks/cheaper.sh"
      - type: "file"
        path: "packages/apm-hook-copilot/.apm/hooks/cheaper.ps1"
      - type: "file"
        path: "packages/apm-hook-pretooluse/apm.yml"
      - type: "file"
        path: "packages/apm-hook-pretooluse/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/apm-hook-pretooluse/.apm/hooks/cheaper.sh"
      - type: "file"
        path: "packages/apm-hook-pretooluse/.apm/hooks/cheaper.ps1"
    modified:
      - type: "file"
        path: "packages/apm/apm.yml"
      - type: "file"
        path: "scripts/check-apm-payload.mjs"
      - type: "file"
        path: "scripts/check-doc-ids.mjs"
      - type: "file"
        path: "scripts/check-version.mjs"
      - type: "file"
        path: "scripts/set-version.mjs"
      - type: "file"
        path: "scripts/test/lib/apm-fixture.mjs"
      - type: "file"
        path: "scripts/test/apm-install-targets.test.mjs"
      - type: "file"
        path: "scripts/test/apm-payload.test.mjs"
      - type: "file"
        path: "scripts/test/apm-package-layout.test.mjs"
      - type: "file"
        path: "scripts/test/launcher-passthrough.test.mjs"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "docs/guides/distribution.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "CONTRIBUTING.md"
      - type: "file"
        path: "docs/specs/cargo-source-install-distribution-for-apm-consumers.md"
      - type: "file"
        path: "docs/specs/cheaper-usage-skill-hook-path-examples.md"
      - type: "file"
        path: "docs/adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    deleted:
      - type: "file"
        path: "packages/apm/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/apm/.apm/hooks/cheaper.sh"
      - type: "file"
        path: "packages/apm/.apm/hooks/cheaper.ps1"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/apm-hook-per-target-package-split.md"
    - "docs/tasks/apm-hook-packages-split.md"
    - "docs/tasks/apm-hook-split-test-follow.md"
    - "docs/tasks/apm-hook-split-docs.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
    - "docs/designs/apm-hook-per-target-package-split.md"
    - "docs/discovery/apm-hook-per-target-package-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/apm-hook-per-target-package-split.md"
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

# APM hook 宣言の per-target サブパッケージ分離

## Summary

Issue #118。単一 `packages/apm/.apm/hooks/hooks.json` が 4 harness の
イベント名を混在させ、APM が全ファイルを全 target へ merge するため
foreign event registration・Claude の Copilot 由来重複 PreToolUse・
Cursor の casing warning を生じさせていた。hook 宣言を 4 つの
per-target sibling package へ分離し、skills package が transitive
`path:` dep + per-dep `targets:` で引き込む構造へ変更した。

## Context

Issue #118 — `packages/apm/.apm/hooks/hooks.json` 単一宣言が 4 harness
(windsurf/cursor/copilot/claude)の native event 名を混在させており、
APM の全 file merge により各 target config へ foreign/dead registration
が混入していた。Claude には Copilot 由来の重複 `PreToolUse`、Cursor
には event casing warning が発生していた。

## Implementation

- **sibling package 4 件新設**: `apm-hook-windsurf`(`target: windsurf`)、
  `apm-hook-cursor`(`target: cursor`)、`apm-hook-copilot`
  (`target: copilot`)、`apm-hook-pretooluse`(`targets: [claude, codex]`)。
  各 package は `.apm/hooks/` に native event のみの `hooks.json` と
  byte 同一の thin launcher(`cheaper.sh` 100755 / `cheaper.ps1`
  UTF-8 BOM+CRLF)を持つ。manifest name は `cheaper-hook-*` で一意、
  version は workspace canonical(1.0.0)と一致。
- **親 `packages/apm/apm.yml`**: `dependencies.apm` に 4 件の相対
  `path:` dep + edge `targets:` を宣言し `.apm/hooks/` を削除。
- **check script 追従**: `check-apm-payload.mjs` は `packages/*/
  .apm/hooks/` を glob 走査し親 hooks の存在を拒否・全 hook package
  の asset/launcher byte 一致/dep wiring を検証。`check-doc-ids.mjs`
  は `packages/*/.apm/` 配下の tracked Markdown を走査。
  `check-version.mjs`/`set-version.mjs` は `apm-hook-*` manifest を
  version 対象へ追加。
- **test fixture 追従**: `apm-fixture.mjs` は親+4 sibling を `pkgs/`
  へ複写し target→deploy dir マップを package 名基準へ更新。
  `apm-install-targets.test.mjs` は native-only event key・launcher
  path・Claude `PreToolUse` 単一 group を assert。
- **文書**: SKILL.md・distribution.md・AGENTS.md・CONTRIBUTING.md を
  新 layout・移行手順(uninstall→install)・`targets:` narrowing 非伝播
  注意へ更新。superseded contract を記述する spec へ注記、旧 ADR へ
  `superseded-by` を追記。

## Related Experiments

なし。

## Validation

- `mise run check` 全段グリーン(biome・fmt・clippy・version/doc-id・
  cargo test workspace・node --test 43 件・lockfile・payload check)
- apm 0.28.0/0.30.0 で実 install 検証: 5 dep 解決、各 target config に
  native event のみ、Claude `PreToolUse` 単一 group(dead copilot
  entry 解消)、残存 warning は Cursor の upstream casing false
  positive のみ
- remote git ref 経路で sibling `path:` が同一 ref 展開されることを
  fixture で実証

## Risks

- `targets:` narrowing は transitive dep へ伝播しないため、将来 dep
  構造を変える際に edge 宣言の保守が必要(文書に注意として記録済み)。

## Follow-ups

- Cursor の event casing warning は upstream(apm)側の false positive
  であり、本 repo の対応範囲外 — upstream 修正待ち。

---
id: "SPEC-034DjvgZNwy9e9aLXb2alN"
type: "spec"
status: "approved"
title: "cheaper-usage skill hook path examples match APM deploy naming"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/87"
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
  derives-from:
    - "../discovery/cheaper-usage-skill-hook-path-examples.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper-usage skill hook path examples match APM deploy naming

## What Should Be Built

配布 skill `cheaper-usage`
（`packages/apm/.apm/skills/cheaper-usage/SKILL.md`）の "Locating
the CLI" 節を、APM の実 deploy contract に一致する記述へ改訂
する。具体的には (a) deploy dir が `<dep-name>`（consumer の
dependency 解決名）であることの明示、(b) canonical install
（virtual package ref `scarecrow173/cheaper/packages/apm`）で
`hooks/apm/` になる実例、(c) bundle dir が不明な場合の
discovery 手順、(d) target 別 deploy root の記述を追加する。

## Why Now

GitHub issue #87。現行例示 `.windsurf/hooks/cheaper/cheaper` は
deploy dir を package 名 `cheaper` と仮定しているが、canonical
install では `hooks/apm/` になるため、親 agent が例示をそのまま
実行して ENOENT になりうる。release v1.0.0 前の配布物として
doc 精度を是正する。

## Users and Value

- cheaper を APM 経由で install した consumer の親 coding
  agent — skill 記載の path で CLI を 1 発で呼べる
- 本 repo maintainer — issue の再発（別 target・別 install 名で
  同種の doc drift）を防ぐ discovery 手順が文書化される

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の
  "Locating the CLI" 節（:41-101）内の path 例示ブロック
  （:44-73、および関連する path 言及）の改訂。節内の
  :74-100 bullets は変更しない
- windsurf / claude / codex / cursor / copilot 全 target の
  deploy layout 監査結果の反映（実測値、APM 0.28.0）

### Out of Scope

- APM deploy 挙動・`smoke-release.mjs`・generated hooks.json
  自体の変更（deploy contract は APM 側の既存正しい挙動）
- `.agents/skills/`・`.claude/skills/` の skill deploy path
  （別経路で本 issue の範囲外）
- 他 skill（bulk-read / code-write / doc-write）の本文 —
  hook path 例示を持たないため対象外（実装時に再確認）
- local checkout の旧 `.windsurf/hooks.json`（gitignore 対象の
  deploy 生成物）の drift 是正（別 issue として既知、
  deployment drift は本 issue の因果外）
- `docs/discovery/cross-agent-hook-distribution-and-contract-
  matrix.md` の `hooks/cheaper/` 記録 — dep 名 `cheaper` で
  install した時点の point-in-time 実測であり drift ではない。
  dep 名規則の補足追記は今回行わない（SKILL.md 改訂で規則が
  説明されるため）

## Requirements

- R1: SKILL.md の path 例示は「deploy dir =
  `<hooks_root>/<dep-name>/`」規則を正しく表現し、package 名
  `cheaper` が deploy dir になると誤読させる記述を残さない。
- R2: canonical install（virtual package ref
  `scarecrow173/cheaper/packages/apm`）の実 path 例
  `.windsurf/hooks/apm/cheaper`・`cheaper.ps1` を例示する。
- R3: install 名が異なる場合の読み替えだけでなく、bundle dir
  を確実に特定する discovery 手順（generated hook config の
  `command` 参照 — file 名は target 依存で `.windsurf/hooks.json`
  ・`.claude/settings.json`・`.github/hooks/<dep>-hooks.json`
  等 —、または hooks_root 配下の `cheaper` / `cheaper.ps1`
  探索）を記述する。
- R4: target 別 deploy root を記述する（windsurf
  `.windsurf/hooks/`、claude `.claude/hooks/`、codex
  `.codex/hooks/`、cursor `.cursor/hooks/`、copilot
  `.github/hooks/scripts/` — APM 0.28.0 実測値 +
  `scripts/test/lib/apm-fixture.mjs` の BUNDLE_DIRS）。
- R5: 記述は APM の deploy contract を「変更されるべき仕様」
  としてではなく「現行挙動」として記述する（upstream 前提条件
  に依存する将来 path を確定的に書かない）。

## Acceptance Criteria

- [ ] SKILL.md 内に deploy dir を package 名 `cheaper` と
  無条件に結びつける例示が存在しない。
- [ ] canonical install の例 `.windsurf/hooks/apm/cheaper`
  （POSIX）・`.windsurf/hooks/apm/cheaper.ps1`（Windows）が
  記載され、「`apm` は dependency 名由来で install 名により
  変わる」説明が付く。
- [ ] bundle dir が不明な場合の discovery 手順（generated
  hook config 参照 or hooks_root 配下探索）が記載される。
- [ ] target 別 deploy root が R4 の実測値と一致する。
- [ ] temp consumer への `apm install`（APM 0.28.0）で記載
  path・表が実測と一致することを確認する。検証は
  (a) path dep basename `apm` と (b) virtual package ref
  `scarecrow173/cheaper/packages/apm#main` の 2 形態で行い、
  windsurf の `<dep>` dir 実在を確認する（全 5 target の
  hooks_root 表は `apm-fixture.mjs` BUNDLE_DIRS と既存 e2e
  `apm-install-targets.test.mjs` で担保済みのため、再実測は
  windsurf で十分）。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- registry 公開後の registry ref 名で dep 解決名を再確認するか
  | git ref（`#main`）は実測済みで `apm` 確定。registry manifest
  経由の解決名のみ未検証（issue #85 の release 未公開のため）
  | release 公開時 | 記載例が registry install とずれたまま残る
  リスク（低: dep 名規則の説明で吸収可能）

## Sources and Relations

- GitHub issue #87
- discovery: `docs/discovery/
  cheaper-usage-skill-hook-path-examples.md`
- 実測（APM 0.28.0 / temp consumer）:
  - path basename `apm` → `.windsurf/hooks/apm/cheaper`・
    `cheaper.ps1`・`cli/`；`.claude/hooks/apm/`・
    `.cursor/hooks/apm/`・`.github/hooks/scripts/apm/` も同様
  - path basename `mypkg` → `.windsurf/hooks/mypkg/`・
    `.cursor/hooks/mypkg/`（dep 名規則の対照確認）
  - virtual package ref `scarecrow173/cheaper/packages/apm#main`
    （git clone）→ `.windsurf/hooks/apm/`（canonical install の
    dep 名 `apm` を実測確定）
- `scripts/test/lib/apm-fixture.mjs`（BUNDLE_DIRS:
  windsurf/claude/codex/cursor は `<root>/hooks/<dep>`、copilot
  は `.github/hooks/scripts/<dep>`）・
  `scripts/test/apm-install-targets.test.mjs`（実 `apm install`
  での e2e 検証）
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md:41-73`
- `docs/guides/distribution.md:34-42`・
  `docs/guides/parent-agent-integration.md:53-63`

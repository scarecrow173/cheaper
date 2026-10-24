---
id: "DISC-034DjtZ2lFPzDHL1MgR6qu"
type: "discovery"
status: "resolved"
title: "cheaper-usage skill hook path examples diverge from APM deploy path"
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
  derives-from: []
  derived-by:
    - "../specs/cheaper-usage-skill-hook-path-examples.md"
    - "../adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper-usage skill hook path examples diverge from APM deploy path

## Exploration Goal

GitHub issue #87。配布 skill `cheaper-usage`
（`packages/apm/.apm/skills/cheaper-usage/SKILL.md`）の "Locating
the CLI" 節が例示する hook bundle path
（`.windsurf/hooks/cheaper/cheaper`）と、APM 0.28.0 が実際に
deploy する path（`.windsurf/hooks/apm/cheaper`）が一致しない。
親 agent が例示をそのまま実行すると ENOENT になる。
正しい例示・discovery 手順を決める。

Entry Decision: A-4 Direct Start — 現象と対象 file が issue に
明記されており、実 deploy path も temp consumer install で
実測確定済み。残る判断は「例示を何に合わせるか」のみ。

## Key Issues and Assumptions

- **confirmed（実測、APM 0.28.0 / temp consumer install）**:
  `apm install` は hook bundle を
  `.windsurf/hooks/<dep-name>/` へ deploy する。virtual package
  参照 `scarecrow173/cheaper/packages/apm#main`（git clone 実測）
  ・local path `F:/repositry/cheaper/packages/apm` いずれでも
  dep name は `apm` となり、実 path は
  `.windsurf/hooks/apm/cheaper`（POSIX launcher）・
  `.windsurf/hooks/apm/cheaper.ps1`（Windows）・
  `.windsurf/hooks/apm/cli/`（CLI payload）。
- **confirmed（実測）**: deploy dir 名は package `name:`
  （`cheaper`）ではなく consumer の dependency 解決名（path dep
  の basename / git ref の末尾 segment）に由来する。
  `docs/guides/distribution.md` の consumer 例
  （`path: ../cheaper`）では `hooks/cheaper/` になり、
  `parent-agent-integration.md` の virtual package 例
  （`.../packages/apm#main`）では `hooks/apm/` になる — 例示は
  単一固定値を仮定できない。
- **confirmed（code 調査）**: SKILL.md の例示 path が誤っていた
  としても generated `.windsurf/hooks.json` の command 自体は
  APM が正しい deploy path で生成するため hook 機能は動作する。
  影響は「skill を読んだ親 agent が CLI を手動呼出しする時の
  path 発見」のみ。
- **confirmed（code 調査）**: skill 配布は別経路（`.apm/skills/`
  → target 依存の skills root、windsurf 等は
  `.agents/skills/<name>/`、claude は `.claude/skills/<name>/`）
  で、この issue の影響範囲外。
- **confirmed（実測 + repo e2e）**: 他 target も同一の
  `<hooks_root>/<dep-name>/` 規則。windsurf/claude/codex/cursor
  は `<root>/hooks/<dep>`、copilot のみ
  `.github/hooks/scripts/<dep>`（APM 0.28.0 temp consumer
  実測 + `scripts/test/lib/apm-fixture.mjs` BUNDLE_DIRS、
  `scripts/test/apm-install-targets.test.mjs` が実 install
  e2e で担保）。generated hook config の file 名も target
  依存（`.claude/settings.json`、copilot は
  `<dep>-hooks.json`）。
- **confirmed（code 調査）**: `smoke-release.mjs` は path dep
  `../pkg` を使うため `hooks/pkg/` で verify しており（:85,99）、
  SKILL.md 例示との差異は smoke では検出されない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: 例示を `<dep-name>` プレースホルダ＋discovery 手順に改訂 | install 名によらず常に正しい。hooks.json の `command` を読む手順も併記できる | 例が具体的な 1 行でなくなり少し読みにくい | yes |
| B: 例示を canonical install（virtual package ref）の実 path `hooks/apm/` に固定 | 公式の install 手順を辿った consumer には一致する | `path: ../cheaper` 等の別名 install ではずれる。根本の「dir 名 = dep 解決名」を隠す | no |
| C: APM deploy 側を package `name:` 基準に統一するよう upstream 要求 | 全 consumer で path が一意 | upstream 依存で自前解決不可。破壊的変更になり既存 install を壊しうる | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: 誤りの本体は「dir 名を `cheaper`（package 名）
  と仮定した例示」。実際は dep 解決名が入るため、公式案内の
  2 経路（virtual ref → `apm`、local path `../cheaper` →
  `cheaper`）で値が分岐する。
- **hypothesis（→ Option A 採用方向）**: 例示を
  `.windsurf/hooks/<dep-name>/cheaper` 形式に直し、
  「dir 名は install 時の dependency 名。不明なら
  `.windsurf/hooks/` 配下で `cheaper`/`cheaper.ps1` を探すか
  `.windsurf/hooks.json` の `command` を参照」の discovery
  手順を併記するのが safest。
- **hypothesis**: 他 target（claude / cursor / copilot）も同様の
  `<dep-name>` 規則なら、監査結果を skill 本文か guide に
  反映する範囲も spec で決める。

## Open Questions

- registry 公開後の canonical ref 名（registry ref）でも dep
  解決名が `apm` で確定か — git ref は実測済みだが registry
  manifest 経由の解決は未検証（issue #85 の release 未公開と
  関連。registry 名が異なっても規則説明＋discovery 手順で
  吸収可能な低リスク）。

## Promotion Candidates

- [x] spec-doc needed: skill 本文の path 例示・discovery 手順・
  監査対象 target を受入基準として明文化
- [x] adr-doc needed: deploy contract は変えないが「doc が規則を
  どう表現するか」の決定（Option A: 規則＋実例＋discovery 手順
  vs B: 固定例示）を ADR-034Dkj1V1OguoUSXpwCE3e として記録
  （graph の briefing gate も ADR を要求するため当初の「不要」
  判定を訂正）

## Sources and Relations

- GitHub issue #87
- 実測: temp consumer へ `apm install`（APM 0.28.0）→
  `.windsurf/hooks/apm/cheaper`・`cheaper.ps1`・`cli/` 生成、
  generated `.windsurf/hooks.json` の `command` も同 path。
  検証 dep 形態: local path（basename `apm`・対照 `mypkg`）と
  virtual package ref `scarecrow173/cheaper/packages/apm#main`
  （git clone、いずれも `hooks/apm/`）
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md:44-73`
  （"Locating the CLI" 節内の path 例示ブロック・install 名
  読み替え注記。節全体は :41-101）
- `scripts/smoke-release.mjs:99`（path dep `../pkg` →
  `hooks/pkg/` で verify）
- `docs/guides/distribution.md:34-42`、`docs/guides/
  parent-agent-integration.md:53-63`（consumer install 例）

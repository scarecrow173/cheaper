---
id: "ADR-034Dkj1V1OguoUSXpwCE3e"
type: "adr"
status: "proposed"
title: "Document APM dep-name deploy rule in cheaper-usage skill"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "../discovery/cheaper-usage-skill-hook-path-examples.md"
    - "https://github.com/scarecrow173/cheaper/issues/87"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/cheaper-usage-skill-hook-path-examples.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/cheaper-usage-skill-hook-path-examples.md"
    - "../plans/cheaper-usage-skill-hook-path-rewrite.md"
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
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Document APM dep-name deploy rule in cheaper-usage skill

## Context and Problem Statement

配布 skill `cheaper-usage` の "Locating the CLI" 節は hook bundle
path を `.windsurf/hooks/cheaper/cheaper` と例示しているが、APM
の hook deploy dir は `<hooks_root>/<dep-name>/` であり、
`<dep-name>` は consumer の dependency 解決名（path dep の
basename / virtual package ref の末尾 segment）に由来する。
canonical install（`scarecrow173/cheaper/packages/apm`）では
`.windsurf/hooks/apm/cheaper` となり、現行例示は親 agent を
ENOENT に導きうる（issue #87）。

deploy 規則そのものは APM 側の contract であり本 repo では
変更しない。配布 doc がこの規則をどう表現するかを決める。

## Decision Drivers

* 例示が install 方法によらず誤解を与えない（dep 名規則を隠さない）
* 親 agent が実行可能な discovery 手順を提供する
* canonical install（virtual package ref）には具体的な実 path を
  示して即時利用可能にする
* doc 変更のみで完結させ、deploy 挙動・生成 config に手を入れない

## Considered Options

* Option A: deploy dir を `<dep-name>` 規則として説明し、canonical
  install の実例（`hooks/apm/`）＋ discovery 手順（generated hook
  config の `command` 参照 / hooks_root 配下探索）を併記する
* Option B: 例示を canonical install の実 path `hooks/apm/` に
  固定し、規則の説明は省略する
* Option C: APM 側の deploy dir を package 名基準に統一するよう
  upstream へ変更を要求し、doc は現行例示のまま据え置く

## Decision Outcome

Chosen option: "Option A", because `<dep-name>` は install 時の
consumer 側記述に依存する可変値であり、単一の固定例にすると
別名 install（`path: ../cheaper` → `hooks/cheaper/`）で再び
誤例示になる。規則＋実例＋discovery 手順の組合せなら全 install
形態で正しく、upstream 変更要請を待たずに今修正できる。

### Consequences

* Good, because install 名によらず親 agent が実 path を特定
  でき、generated hook config は正しいため discovery 手順の
  信頼できる参照先が常に存在する。
* Good, because 他 target（claude/codex/cursor/copilot）の
  deploy root も同一規則で説明でき、監査結果を一表で示せる。
* Bad, because 例が「そのまま実行できる 1 行」から「規則の
  説明付き例」になり、skill 本文がやや長くなる（許容）。

### Confirmation

SKILL.md 改訂後、temp consumer への `apm install`（APM 0.28.0）
で記載 path の実在を確認する。canonical install 例
（`hooks/apm/`）と dep 名規則の説明が本文に存在することを
review で確認する。

## Implementation Plan

* Affected paths:
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（"Locating the
  CLI" 節、および deploy path を言及する関連記述）
* Patterns to follow: 既存の「install 名が違う場合は読み替え」
  注記スタイルを拡張し、規則・実例・discovery 手順の順に記述
* Patterns to avoid: deploy dir を package 名 `cheaper` と結ぶ
  無条件の例示、`<dep-name>` 規則を隠す固定 path 化
* Dependencies or configuration: なし

## Verification

* [ ] SKILL.md に `hooks/cheaper/` を唯一の例示とする記述が残ら
  ない
* [ ] canonical install 例 `hooks/apm/cheaper`・`cheaper.ps1` と
  dep 名由来の説明が記載される
* [ ] discovery 手順（generated hook config / hooks_root 探索）
  が記載される
* [ ] temp consumer install（APM 0.28.0）で記載 path が実在する
  ことを確認

## Pros and Cons of the Options

### Option A: `<dep-name>` 規則＋実例＋discovery 手順

* Good, because 全 install 形態・全 target で正しく、規則を
  説明するため将来の dep 名変更にも耐える。
* Bad, because 記述量が増え、単純な例示より読解コストがある。

### Option B: `hooks/apm/` 固定例示

* Good, because canonical install には即座に一致する最短の修正。
* Bad, because `path: ../cheaper` 等の別名 install で再び誤例示
  となり、「dir 名 = dep 名」という本質を隠して同種 drift が
  再発する。

### Option C: APM upstream への規則変更要求

* Good, because deploy dir が一意なら doc 側の説明は最小化できる。
* Bad, because upstream の timeline に依存し即時修正にならず、
  既存 consumer の deploy 構成を破壊する breaking change になる。

## More Information

- issue #87: <https://github.com/scarecrow173/cheaper/issues/87>
- discovery:
  `docs/discovery/cheaper-usage-skill-hook-path-examples.md`
- 実測: APM 0.28.0 / temp consumer（path dep basename `apm` →
  `.windsurf/hooks/apm/`、basename `mypkg` → `hooks/mypkg/`）
- `scripts/test/lib/apm-fixture.mjs` BUNDLE_DIRS（全 target の
  deploy root 一覧）

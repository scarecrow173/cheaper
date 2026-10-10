---
id: "DESIGN-034DkkcOv0Sos8KzcEP3oB"
type: "design"
status: "approved"
title: "cheaper-usage SKILL.md hook path section rewrite"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
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
    - "../specs/cheaper-usage-skill-hook-path-examples.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/cheaper-usage-skill-hook-path-examples.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheaper-usage-skill-hook-path-examples.md"
    - "../adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper-usage SKILL.md hook path section rewrite

## Context

`packages/apm/.apm/skills/cheaper-usage/SKILL.md` の "Locating the
CLI" 節（:41-101）は、APM install 済み consumer での CLI 呼出し
方法を説明する唯一の section。現行は節内の path 例示ブロック
（:44-73）で deploy dir を `hooks/cheaper/` と例示するが、実規則は
`<hooks_root>/<dep-name>/`（ADR-034Dkj1V1OguoUSXpwCE3e）。doc のみ
の変更で、改訂は同ブロックの書換え＋ target 別表の追加に閉じ、
節内 bullets :74-100 は位置・内容を維持する。

## Scope

### In Scope

- "Locating the CLI" 節（:41-101）内の path 例示ブロック
  （:44-73 相当）の書換え: 規則説明 → canonical install 実例 →
  discovery 手順 → target 別 deploy root 表 → 既存の launcher
  説明への接続。節内 bullets :74-100 は変更しない
- 同 file 内の他の path 言及の整合確認（`hooks` 参照は本節のみの
  想定だが実装時に再 grep する）

### Out of Scope

- skill の他 section（commands・envelope・exit codes 等）の変更
- 他 skill・guide・ADR/discovery の記述更新（matrix doc の
  point-in-time 記録は触れない）
- APM deploy 挙動・hook script・`smoke-release.mjs` の変更

## Components and Boundaries

- 書換え後の節構成（この順で記述）:
  1. `cheaper` が PATH 上にある場合はそれを使う（現行 :43 を維持）
  2. 導入文の一般化（現行 :44-46）: 「In a Windsurf workspace」
     → target 非依存の「workspace where cheaper was installed via
     APM」へ。また「ships a platform launcher plus standalone
     executables」は Git-source install では executable を含まない
     ため、「launcher ＋（sibling または初回 acquisition による）
     executable」へ訂正（詳細は :53-70 の既存記述が担う）
  3. hook bundle の deploy 規則:
     `<hooks_root>/<dep-name>/cheaper[.ps1]`。`<dep-name>` は
     install 時の dependency 名（path dep の basename、virtual
     package ref `scarecrow173/cheaper/packages/apm` では `apm`）
  4. canonical install の実例:
     `.windsurf/hooks/apm/cheaper <command> [options]` /
     `powershell .windsurf/hooks/apm/cheaper.ps1 <command>
     [options]`（`powershell` 5.1+ で動く方を例示。copilot
     でも同形、例 `.github/hooks/scripts/apm/cheaper` と
     1 行添える）
  5. target 別 `<hooks_root>` 表: windsurf `.windsurf/hooks/`、
     claude `.claude/hooks/`、codex `.codex/hooks/`、cursor
     `.cursor/hooks/`、copilot `.github/hooks/scripts/`
  6. bundle dir 不明時の discovery 手順: generated hook config
     の `command` を読む。file 名は target 依存 —
     windsurf `.windsurf/hooks.json` / `apm-hooks.json`、claude
     `.claude/settings.json`（command は `${CLAUDE_PROJECT_DIR}`
     変数展開を含む点を注記）/ `apm-hooks.json`、codex
     `.codex/hooks.json` / `apm-hooks.json`、cursor
     `.cursor/hooks.json` / `apm-hooks.json`、copilot
     `.github/hooks/<dep-name>-hooks.json`。または hooks_root
     配下で `cheaper`/`cheaper.ps1` を探す
  7. 既存の launcher 説明段落（`cli/bin/<platform>` への
     dispatch・sibling 不在時の Release acquisition 等、
     :53-70）は内容を維持して接続（path 前提の言及だけ
     新規則へ整合）
  8. 「"cheaper <args>" means either form」の橋渡し文
     （現行 :72-73）は維持し、読み替え注記は規則説明へ昇格済みの
     ため新規則を参照する形に書き換える
- 境界: 本 file は APM skill bundle として deploy される
  consumer-facing doc。repo 内 doc ではなく「呼出し手順の
  canonical 案内」であり、upstream APM の将来変更を確定的には
  書かない（spec R5）。

## Data and Control Flow

- なし（doc のみ）。記述の前提となる事実 chain:
  APM install → `.apm/hooks/*` を `<hooks_root>/<dep-name>/` へ
  deploy + hook config 生成（command は実 path へ rewrite 済み）
  → 親 agent が SKILL.md を読む → path 特定 → CLI 起動。

## Non-Functional Constraints

- 記述は APM 0.28.0 の実測挙動に基づく現行挙動の説明とする
- generated hook config の file 名は target 依存であることを
  discovery 手順に含める（`.windsurf/hooks.json` だけを前提にしない）
- 既存注記「install 名が違う場合は読み替え」の意図を規則説明へ
  昇格し、推測ではなく規則として書く

## Risks and Trade-offs

- registry 公開後の canonical ref 名で `<dep-name>` が `apm` で
  なくなる可能性 | 低: 規則説明＋discovery 手順で吸収。
  spec の Deferred Concern として #85 後に再確認を残す
- 記述量増加による skill 読解コスト | 表＋短文構成で抑制

## Deferred Design Concerns

- registry 公開後に canonical ref 名の deploy dir を再実測するか
  | issue #85（release 未公開）待ち | v1.0.0 公開時 | 例示が
  canonical install とずれたまま残る（低リスク、規則説明で吸収）

## Verification Notes

- [ ] 改訂後の SKILL.md に `hooks/cheaper/` を唯一例示とする
  記述が残らない（grep で確認）
- [ ] spec R1-R5・全 AC を満たす
- [ ] temp consumer install（APM 0.28.0）で記載 path・表が実測と
  一致することを確認

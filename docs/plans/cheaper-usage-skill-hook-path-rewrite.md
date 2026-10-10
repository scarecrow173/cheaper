---
id: "PLAN-034DkvfCVX1pOlAvC2zGX2"
type: "plan"
status: "approved"
test-spec-skip: "Documentation-only rewrite; verified by doc audit, scoped grep, and temp consumer install — no executable test-spec."
title: "cheaper-usage SKILL.md hook path section rewrite"
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
  implements:
    - "docs/designs/cheaper-usage-skill-hook-path-section-rewrite.md"
    - "docs/specs/cheaper-usage-skill-hook-path-examples.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/cheaper-usage-skill-hook-path-section-rewrite.md"
  derived-by: []
  verifies:
    - "docs/specs/cheaper-usage-skill-hook-path-examples.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# cheaper-usage SKILL.md hook path section rewrite Implementation Plan

## Goal

`cheaper-usage` SKILL.md の "Locating the CLI" 節を APM の実
deploy contract（`<hooks_root>/<dep-name>/`）に一致する記述へ
改訂し、issue #87 の doc drift を解消する。

## Scope

単一 implementation stream（1 file 改訂 + temp consumer 実測
検証）。task は 1 件で十分。

## Upstream Documents

- `docs/specs/cheaper-usage-skill-hook-path-examples.md`（approved）
- `docs/adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md`
- `docs/discovery/cheaper-usage-skill-hook-path-examples.md`

## Design Inputs

- `docs/designs/cheaper-usage-skill-hook-path-section-rewrite.md`
  （approved、書換え後の節構成 item 1-8 を定義）

## File Map

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` —
  "Locating the CLI" 節内の path 例示ブロック（現行 :44-73；
  節全体は :41-101）を書換える。同節の :74-100 bullets
  （target 別挙動・Codex gap・DISC 参照）は位置・内容とも
  変更しない。他 section・front matter も変更しない。

## Implementation Sequence

- [ ] 1. SKILL.md 内の他の `hooks`/`cheaper.ps1` 言及を grep し、
  書換え範囲が :41-73 のみであることを確定する
- [ ] 2. design item 1-8 の構成で節を書換える:
  - PATH 優先（現行 :43 維持）
  - 導入文を target 非依存へ一般化し "standalone executables"
    記述を sibling/acquisition 実態へ訂正
  - `<hooks_root>/<dep-name>/` 規則 + dep 名由来の説明
  - canonical install 実例（windsurf `hooks/apm/`、copilot
    `scripts/apm/` の 1 行添え）
  - target 別 hooks_root 表（5 target、APM 0.28.0 実測値）
  - discovery 手順（generated hook config の `command` 参照 —
    file 名は target 依存 5 target 分 + claude の
    `${CLAUDE_PROJECT_DIR}` 注記 — / hooks_root 配下探索）
  - launcher 説明段落（現行 :53-70）は維持して接続
  - install 名読み替え注記（現行 :71-72）は規則説明へ昇格
    済みのため削除
  - "either form" 橋渡し文（現行 :72-73）は維持し新規則参照へ
    書き換え（後続の bare `cheaper` 呼出し :75,:97,:179 より
    前に置く）
  - 節内 bullets :74-100 は位置・内容を変更しない
- [ ] 3. `grep -rn "hooks/cheaper" packages/apm/.apm/` で
  path 例示の残存がないことを確認
- [ ] 4. temp consumer `mise exec -- apm install`（APM 0.28.0）で
  記載 path・表が実測と一致することを検証 — path dep basename
  `apm` 形態で `<dep>` dir 実在を確認（virtual package ref
  `scarecrow173/cheaper/packages/apm#main` → `hooks/apm/` は
  briefing 時点で git clone 実測済み。registry ref は #85 後の
  Deferred Concern）

## Dependencies and Risks

- 外部依存なし。APM 0.28.0 の実測値は確定済み
- リスク: registry 公開後の canonical ref 名で `<dep-name>` が
  `apm` 以外になる可能性 — spec/design の Deferred Concern として
  #85 後に再確認（規則説明で吸収可能な低リスク）

## Task Breakdown

- `rewrite-cheaper-usage-skill-hook-path-section` —
  sequence 1-4 を全て含む単一 task（docs-only、分割不要）

## Verification

- [ ] `grep -rn "hooks/cheaper" packages/apm/.apm/` で
  path 例示が残らない
- [ ] spec R1-R5・全 AC の充足を review で確認
- [ ] temp consumer install（APM 0.28.0、path dep）で記載
  path が実在（basename 規則の再確認。virtual ref は briefing
  実測済み、registry ref は #85 後の Deferred Concern）
- [ ] `mise run check` が pass（doc-id check が
  `packages/apm/.apm/**/*.md` の legacy 連番 ID を検査するため
  改訂本文に `TYPE-NNNN` 形式を書かない）

## Open Questions / Gaps

- なし（実測値は全て確定済み）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（doc 改訂 1 file、task 1 件）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 重点: 書換え後の節構成が design item 1-8 と一致するか、
  SKILL.md 内の他 path 言及（launcher 説明内の `cli/bin/` 等は
  bundle 相対で正しい）との整合

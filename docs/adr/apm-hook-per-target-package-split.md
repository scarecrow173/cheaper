---
id: "ADR-1fVxZhjOm4z8ilPCUSXHMI"
type: "adr"
status: "accepted"
title: "APM hook 宣言を per-target サブパッケージへ分離"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "https://github.com/scarecrow173/cheaper/issues/118"
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
  supersedes:
    - "./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  superseded-by: []
  related:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/designs/apm-hook-per-target-package-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/discovery/apm-hook-per-target-package-split.md"
  derived-by:
    - "docs/designs/apm-hook-per-target-package-split.md"
  verifies: []
  verified-by: []
  references:
    - "packages/apm-hook-pretooluse/.apm/hooks/hooks.json"
    - "packages/apm/apm.yml"
  defers: []
  deferred-by: []
---

# ADR: APM hook 宣言を per-target サブパッケージへ分離

## Context

`packages/apm/.apm/hooks/hooks.json` は windsurf/copilot/claude/cursor 向けの
native event 名を 1 ファイルに混在させている。APM の hook integrator は
hook ファイル内の event を **全 effective target の config へ無条件に
merge** し、ファイル内で event を target 別に絞る機構を持たない
(APM 0.28.0/0.30.0 ソースと fixture 実地検証で確認)。

結果として:

- 各 target の hooks.json/settings.json に他 harness の発火しない
  event entry が dead 状態で残る(`.cursor/hooks.json` に
  `pre_read_code` 等)。
- claude では `preToolUse`(copilot flat)が map+変換され `PreToolUse`
  へ入り、正規 entry と **重複登録**される(`--target copilot` を呼ぶ
  死に matcher entry)。
- APM の casing heuristic(非 copilot = PascalCase 期待)が unmapped
  camelCase event を警告対象にするため、claude/codex/cursor/windsurf で
  install/update 毎に警告が出る(issue #118)。

APM の supported model では hook の target 絞り込みは **package 単位**
(`target:`/`targets:` manifest field)でのみ可能であり、deprecated の
filename routing(`hooks-<target>.json`)は警告を伴い将来削除予定。

## Decision Outcome

hook 宣言を harness 別 sibling package に分離する:

- `packages/apm` — skills のみ(`.apm/hooks/` を削除)
- `packages/apm-hook-windsurf`(`target: windsurf`)
- `packages/apm-hook-cursor`(`target: cursor`)
- `packages/apm-hook-copilot`(`target: copilot`)
- `packages/apm-hook-pretooluse`(`targets: [claude, codex]` — event/shape
  が共通なので 1 package に共有)

親 `packages/apm/apm.yml` の `dependencies.apm` に sibling `path:` dep を
per-dep `targets:` 付きで宣言し、consumer の install 対象とする。
remote(virtual subpath)package 内の相対 `path:` dep は同一 repo・同一
ref で解決されるため version skew は生じない(git-daemon fixture で
実証)。local `path:` dep の transitive 解決も APM 0.28.0/0.30.0 で
実証済み(`source_path` anchor、#857)。

各 hook package は `hooks.json` と launcher `cheaper.sh`/`cheaper.ps1` を
自 package に持つ(hook command が参照する script のみ bundle deploy
される規約のため)。manifest の `name:` は `cheaper-hook-*` の一意名、
`version:` は親と同一 canonical version(`check-version.mjs` 対象化)。

## Considered Options

- **B: deprecated filename routing(`hooks-windsurf.json` 等へ分割)**:
  dead entry は減るが「filename routing is deprecated」警告が install 毎に
  出て警告を別物化するだけ。将来削除予定の機能への依存は不可。
- **C: 単一ファイル維持 + 命名規約統一**: 各 harness の native event 名は
  実際に異なり APM map が windsurf/cursor 系をカバーしないため、機能を
  維持したまま統一不可能。`PreToolUse` のみに絞ると windsurf/cursor の
  guard が失われる(機能回帰)。
- **D: hook 登録を CLI `hook-config` へ移行**: 5 harness 分の config
  schema/所有/reconciliation を APM integrator と二重実装。lifecycle
  script は `apm lifecycle trust` 前提で自動配布されず機能回帰。
  hook 配布の責務は APM に残す。
- **E: 現状維持 + 文書で警告を正当化**: dead entry と claude 重複登録が
  残り、厳格に検証する harness での実害リスクを放置する。

## Consequences

- 長所: 各 target config に native event のみが残り dead entry が消える。
  claude `PreToolUse` の重複も解消。casing 警告は cursor の
  false-positive(native camelCase event に対する APM heuristic 誤判定)
  のみ残る。`apm install <dep>` 1 本で従来どおり全分が transitive に
  入る(consumer の dep 宣言は不変)。
- 短所: package 数が 1→5 に増える。launcher script が各 hook package に
  重複配置される(数十行の thin launcher — `check-apm-payload.mjs` で
  copy 間 byte 一致を検査して drift を防ぐ)。
- 短所: `packages/apm` が self-contained でなくなる — dir 単体コピーの
  `path:` install、`apm pack`/`publish` による registry 配布は
  transitive `path:` dep の解決に repo 階層が要るため非対応。
  git ref 経由の配布のみをサポートする(registry 公開は従来から
  deferred 方針)。
- breaking change: consumer が dep-edge `targets:` で cheaper dep を
  narrow していても、APM は祖先 edge の `targets:` を transitive dep
  へ伝播しない(fixture 実証)。multi-harness active な narrowing
  consumer は transitive hook package の deploy を受け取る。
  回避策とともに distribution.md に明記する(SPEC R9)。
- 副作用/移行: 旧 `apm` package が配布した merged entry は
  `apm update` では除去されない(APM に orphan cleanup が無いため)。
  consumer には `apm uninstall` → `apm install` の手順を案内する
  (SPEC の R7 / AC5)。windsurf の旧 entry は dead script path 参照で
  hook failure となり得るため、移行案内は必須。
- 前提 version: transitive `path:` dep と per-dep `targets:` は
  APM ≥0.22 の機能。repo pin の 0.28.0 と 0.30.0 で実動作確認済み。
- upstream 既知問題: cursor native event の camelCase に対する casing
  警告(`_HOOK_EVENT_EXPECTED_CASING` が cursor を PascalCase と
  誤期待)と、hook 喪失 package の merged entry orphan cleanup 欠如は
  microsoft/apm 側の課題(起票候補)。

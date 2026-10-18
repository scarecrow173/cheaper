---
id: "DISC-1mk9ud9KQV9qy4ZC0sxDJa"
type: "discovery"
status: "draft"
title: "APM hook 宣言を per-target サブパッケージへ分離(イベント名 casing 警告と cross-target 死骸エントリの解消)"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
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
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by:
    - "docs/specs/apm-hook-per-target-package-split.md"
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm-hook-claude/.apm/hooks/hooks.json"
    - "packages/apm-hook-codex/.apm/hooks/hooks.json"
    - "packages/apm/apm.yml"
    - "scripts/check-apm-payload.mjs"
    - "docs/guides/distribution.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
  defers: []
  deferred-by: []
---

# APM hook 宣言の per-target 分離

## 背景

Issue #118。`packages/apm/.apm/hooks/hooks.json` が 1 ファイルに
4 ハーネス分の native event 名を混在させている:

| event key | casing | 対象 harness | entry shape |
|---|---|---|---|
| `pre_read_code`, `pre_run_command` | snake_case | windsurf | flat `{command, powershell, show_output}` |
| `preToolUse` | camelCase | copilot | flat + `matcher` |
| `PreToolUse` | PascalCase | claude / codex | nested `{matcher, hooks:[{type:command,...}]}` |
| `beforeReadFile`, `beforeShellExecution` | camelCase | cursor | flat + `|| echo allow` fallback |

## APM integrator の実動作(0.28.0/0.30.0 ソース + 実地 fixture で確認)

- `find_hook_files` は `.apm/hooks/*.json` と `hooks/*.json` を flat に
  glob するだけで、**hook ファイル内の event に per-target 絞り込み機構は
  存在しない**。発見した全 event が全 effective target の config に
  merge される。
- event rename map(`_HOOK_EVENT_MAP`)は copilot/claude/gemini/kiro のみ。
  cursor/codex/windsurf/antigravity は map を持たず verbatim pass-through。
- casing 警告 heuristic(`_HOOK_EVENT_EXPECTED_CASING`): copilot は
  camelCase、それ以外は全て PascalCase を期待。`snake_case`/全小文字は
  casing 検出が None で警告しない。map 無しの不一致 camelCase は警告対象。
- **cursor の native event(`beforeReadFile` 等)は camelCase** であり、
  cursor target 自身に対してすら "expects PascalCase" 警告が出る
  (APM 側 heuristic の false positive — upstream bug)。
- entry shape は harness 毎に異なり、claude/gemini/antigravity への merge
  時のみ flat→nested 変換(`_to_claude_hook_entries` 等)が走る。
  cursor/codex/windsurf には shape 変換が無く raw 配置。
- copilot には event map で `PreToolUse → preToolUse` されるが shape 変換は
  無いため、Claude-shape entry を copilot に置くと壊れる
  (だから現行は flat `preToolUse` entry を別途持つ)。

## fixture での現状再現(consumer に旧 layout を install)

- 全 5 target(claude/codex/copilot/cursor/windsurf)の config に
  **6 個全ての event key が配置**される。
- 警告: claude/codex/cursor/windsurf の 4 target(issue 報告と一致。
  copilot は camelCase 期待なので警告なし)。
- **issue に未記載の追加問題**: claude の `settings.json` の
  `hooks.PreToolUse` に cheaper entry が **2 件**登録される —
  `preToolUse`(copilot flat)が map+変換されて `PreToolUse` に入り、
  `--target copilot` を呼ぶ死に matcher entry(matcher は
  `view|bash|powershell|shell` で claude 工具名に合致せず)と、正規の
  `--target pretooluse` entry が併存する。

## 対応案の検討

### A. per-target サブパッケージ分離(採用候補)

`packages/apm` から hook を取り出し、sibling package へ分離:

- `packages/apm-hook-windsurf` (`target: windsurf`) —
  `pre_read_code`/`pre_run_command`
- `packages/apm-hook-cursor` (`target: cursor`) —
  `beforeReadFile`/`beforeShellExecution`
- `packages/apm-hook-copilot` (`target: copilot`) — `preToolUse` flat
- `packages/apm-hook-pretooluse` (`targets: [claude, codex]`) —
  `PreToolUse` nested(claude/codex で共通形状)

親 `packages/apm/apm.yml` の `dependencies.apm` に sibling `path:` dep +
per-dep `targets:` を宣言。APM doc 明記の「remote package の相対 `path:`
は同一 repo root 内・同一 ref に解決される」規則により **version skew は
構造的に存在しない**。package 側 `target:`/dep 側 `targets:` の両方を
宣言する(直接 install 時の防御 + transitive reach の絞り込み)。

**fixture で実証済み**(0.30.0、local path dep + git daemon 経由の
remote path dep 双方):

- transitive `path:` dep が同一 commit で解決される
- 各 target config に native event のみ配置(dead entry ゼロ)
- claude `PreToolUse` の重複が解消
- 警告は cursor の false-positive のみ残存(`beforeReadFile` は cursor の
  正しい native event 名なので cheaper 側では除去不能 → upstream 案件)

### B. deprecated filename routing(`hooks-<target>.json`)— 却下

- 機能は残るが「filename-based target routing is deprecated」警告を
  install 毎に出す(警告の別物化に過ぎない)+ 将来削除予定の機能に依存。
- 同一 package marker で upsert されるため旧 entry は一部置換されるが、
  書き込まない event key の旧 entry は残存する(dead entry 不滅)。

### C. 命名を統一して 1 ファイル維持 — 不可

各 harness の native event 名は実際に異なり、APM map が
windsurf/cursor の event をカバーしない。「全 target で認識される命名」は
存在しない。`PreToolUse` だけに絞ると windsurf/cursor の機能が失われる
(機能回帰)。

### D. hook 登録を CLI(`hook-config`)へ移す — 却下

devin 向けには既存の実績があるが、5 harness 分の config schema・所有・
reconciliation を APM integrator と二重実装することになり scope 過大。
lifecycle script は `apm lifecycle trust` が前提で自動配布にならず
機能回帰。APM が hook 配布の責務を持つ境界は変えない。

## 移行(upgrade)上の wrinkle — 実証済み

旧 layout で install 済みの consumer が `apm update` で新 layout に
移行すると:

- 旧 package(dep name `apm`)所有の merged hook entry(`_apm_source`
  marker)は `find_hook_files` が空で早期 return するため **除去されず
  orphan 化**する。APM に「hook file を失った package の entry を掃除する」
  経路が無い。
- 結果: 旧 entry(dead script path `.windsurf/hooks/apm/cheaper.sh` 等を
  参照 — bundle は stale file cleanup で消える)と新 entry が併存し、
  native event が **二重発火**する(実証: `.windsurf/hooks.json` の
  `pre_read_code` が 2 件等)。
- windsurf の旧 entry は `exit $LASTEXITCODE` で script 不在 exit≠0 のため
  hook failure として surface し得る(cursor は `|| echo allow` fallback
  で実害小)。
- **remediation**: `apm uninstall <dep>` → `apm install` で全 marker の
  merged entry が完全除去されることを実証(uninstall で hooks dict が空に
  なることを確認)。docs/SKILL.md/issue への明記が要る。
- upstream 側でも orphan cleanup は bug と見なせる → microsoft/apm への
  issue 起票を検討。

## 影響面

- `packages/apm/.apm/hooks/` 削除 → `cheaper.sh`/`cheaper.ps1` は各 hook
  package が独自に持つ(bundle は hook command が参照する script を
  deploy するため、各 package にコピーが必要 — 数十行の thin launcher)。
- dep name = virtual path の末セグメント → bundle root が
  `.windsurf/hooks/apm-hook-windsurf/` 等に変わる。SKILL.md の
  「Locating the CLI」表と `docs/guides/distribution.md` の更新が必要。
- `scripts/check-apm-payload.mjs` は `packages/apm/.apm/hooks/` の
  3 asset を必須検査している → 新 layout(複数 hook package)に合わせて
  書き換え。
- root `apm.yml`(dev project)は自 package に依存していないため無関係。
- `agent-skills` target だけの consumer では hook package が何も
  deploy しない(pkg-level target で全て除外) — 現状と同等以上に正しい。

## 残課題(この issue の scope 外)

- APM upstream: cursor/windsurf/codex の expected-casing 表が native
  event 命名(camelCase/snake_case)と矛盾 → false-positive 警告。
- APM upstream: package が hook file を失った際の merged entry orphan
  cleanup が無い。
- `.cursor/hooks.json` 等に残る旧 marker entry の自動除去は APM 側の
  reconcile 次第;consumer 向けには uninstall→install の手順を案内する。

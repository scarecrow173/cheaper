---
id: "DISC-0005"
type: "discovery"
status: "resolved"
title: "Windsurf Cascade hooks and APM hook deployment for pre-read delegation"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "./0004-v2-productization-initiative-briefing.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0004-v2-productization-initiative-briefing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "./0004-v2-productization-initiative-briefing.md"
  derived-by:
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Windsurf Cascade hooks and APM hook deployment for pre-read delegation

## Exploration Goal

parent agent が file を read する**前**に token count を測定し、large file を
bulk-reader へ redirect する仕組みを、ユーザー選択の initial target
（Windsurf）で実現可能か確認する。あわせて hook の canonical 配布経路
（APM）で実際に deploy される形式を実測で確定する。

## Current State (verified)

- `count-tokens` CLI / `count_tokens` tool が同一 core 実装で稼働（SPEC-0007）
- 本 repo の `apm.yml` は consumer 側。`.apm/` producer layout は未作成
- APM CLI 0.28.0 が mise toolchain で利用可能

## Confirmed Findings

### Windsurf Cascade hooks

出典: <https://docs.devinenterprise.com/desktop/cascade/hooks>

- config は system / user (`~/.codeium/windsurf/hooks.json`) / workspace
  (`.windsurf/hooks.json`) の 3 層で merge される
- event `pre_read_code` は Cascade の code file read **前**に発火し、
  `tool_info.file_path`（絶対 path。directory の場合もあり得る）を stdin
  JSON で受け取る
- pre-hook は **exit code 2 で block** でき、stderr の message が agent に
  見せられる。他の exit code / exit 0 は action を通す
- hook entry shape: `{"hooks": {"pre_read_code": [{"command": "...",
  "powershell": "...", "show_output": false}]}}`

### APM hook deployment（実測）

`/tmp/apm-probe` で local package → consumer `apm install --target windsurf`
を実測した結果:

- `.apm/hooks/hooks.json` の `pre_read_code` entry は windsurf native shape
  のまま `.windsurf/hooks.json` に pass-through で書き込まれる
  （`command`/`powershell`/`show_output` が保持される）
- hook command の `./<script>` は hook file directory から解決され、
  script bundle は `.windsurf/hooks/<pkg>/<script>` へ deploy される。
  generated config は repo-relative path を指す
- `${PLUGIN_ROOT}/hooks/<script>` 形式は script 未検出 warning になるため
  使わない。`./` 相対 path が正
- `${PLUGIN_ROOT}` 自体は path variable として解決されるが、bundle 外の
  file（CLI bin 等）を指す用途には検証が必要（Phase 4 の論点）

## Decisions Recorded

- hook event は windsurf native `pre_read_code` をそのまま使う
  （APM canonical `PreToolUse` への映射は存在しないため）
- hook manifest は `.apm/hooks/hooks.json`、script は `.apm/hooks/` に
  同梱し `node ./pre-read-code.js` で起動する
- block message は stderr で agent へ返し、bulk-read への delegation を
  指示する

## Open Items

- large file threshold の実測根拠（Phase 6 benchmark で決定。
  現段階は env 上書き可能な provisional default を置く）
- `pre_read_code` の `file_path` が directory の場合の扱い
  （初版は pass-through。dir 全体の token 集計は別途検討）
- non-windsurf target への同 hook deploy 挙動
  （event name が未知の target では no-op の見込み。Phase 4/5 で確認）

## Sources and Relations

- DISC-0004: V2 productization initiative briefing（upstream）
- SPEC-0008: 本 discovery から派生する delegation layer 仕様

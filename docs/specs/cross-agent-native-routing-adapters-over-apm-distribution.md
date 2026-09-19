---
id: "SPEC-034rzTJydXfLvQhTIqiDqW"
type: "spec"
status: "approved"
title: "Cross-agent native routing adapters over APM distribution"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/43"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
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
    - "./copilot-pre-read-hook-adapter.md"
    - "./cross-agent-distribution-verification.md"
    - "./separate-dev-apm-project-from-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../designs/apm-skills-and-pre-read-hook.md"
    - "../designs/copilot-pre-read-hook-adapter.md"
  refines: []
  refined-by:
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
  derives-from:
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Cross-agent native routing adapters over APM distribution

## What Should Be Built

APM を hook bundle / generated config の distribution layer として維持したまま、
各 Coding Agent target の native hook event / payload / block contract を
per-target adapter が canonical routing request へ正規化し、target 非依存の
shared routing policy が allow / deny を判定する構造を実装する。

canonical routing request は file-read / shell の 2 種を持ち、policy 層は
target 固有の event 名・payload shape・response contract を知らない。
adapter の責務は native payload の parse、relevant tool 判定、canonical
request への normalize、policy verdict の native response への変換、
infrastructure failure 時の target 固有 fail semantics 吸収に限定される。

## Why Now

ADR-034r05iHrZUE6W9RDF0V73 は verbatim `pre_read_code` deploy を no-op として維持し
per-target adapter を follow-up chain に defer した。Copilot adapter
（SPEC-034r05iHK8bA7gsvijtkou / DESIGN-034r05iHy2gdnoUHGunDtE）がその最初の実装として
機能しているが、Claude Code / Cursor は依然 no-op で、Codex は未再検証だった。

issue #43 が (a) adapter / shared policy 分離の統一アーキテクチャ、(b)
Claude Code・Cursor adapter、(c) Codex の current contract 再検証、
(d) #42 の shell read detection が接続できる canonical shell request の
seam を要求する。加えて 2026-09-19 時点の再検証で、APM が hook entry の
`matcher` / nested `hooks` group を generated config へ verbatim で
pass-through することが実測確認され、Codex native schema
（`PreToolUse` + matcher group + `{type:"command"}` handler）を APM
経由で表現可能であることが判明した。

## Users and Value

- Claude Code / Cursor / Copilot 利用者: Windsurf と同等の large-read
  safety net が native hook contract で機能する。
- Codex 利用者: shell 経由の読み込みは canonical shell request まで
  到達し、file-read が unsupported であることが matrix / docs で
  正確に示される。
- 開発者: large-read 判定が target 非依存の shared policy に集約され、
  #42 の shell candidate extraction が adapter を触らずに接続できる。

## Scope

### In Scope

- canonical routing request の定義（`file-read` / `shell` / `cwd` /
  source tool metadata を policy 層が共通で扱う）。
- shared routing policy module（既存 `pre_read_code` の large-read 判定を
  reusable seam として抽出。candidate extraction → workspace/path
  validation → `count-tokens` → threshold / `FILE_TOO_LARGE` →
  allow / deny + diagnostics）。
- per-target adapter: Windsurf（`pre_read_code` + `pre_run_command`）、
  Claude Code + Codex（`PreToolUse` matcher group、`tool_name` /
  `tool_input` payload、`hookSpecificOutput.permissionDecision` deny）、
  Cursor（`beforeReadFile` + `beforeShellExecution`、
  `{permission: "deny"}` deny）、Copilot（既存 adapter を shared
  architecture へ統合、`view` file-read に加え shell tool を canonical
  shell request へ届ける）。
- `hooks.json` の per-target native event 登録（APM の verbatim
  pass-through を利用。独自 installer は作らない）。
- Codex native contract の再検証結果の反映（file-read は upstream gap で
  unsupported、shell は `PreToolUse` matcher `Bash` で到達可能、hook trust
  review と APM flat-entry shape の制約を matrix に記録）。
- adapter unit 的検証（representative payload fixture 注入）、per-target
  contract test、APM install E2E の更新（target ごとの native event /
  config / adapter deploy / stale `pre_read_code` no-op 記録）。
- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md` と
  `docs/guides/parent-agent-integration.md` の matrix / 対応表更新。

### Out of Scope

- provider 認証 / OAuth / API key、model resolution、worker runtime 本体。
- `cat`/`head`/`tail`/`less`/`more` 等の shell command parsing 本体
  （#42 が canonical shell request の downstream で実装）。
- APM を置き換える独自 installer、target ごとの手動 install 手順。
- 各 agent binary / runtime での実機 E2E（generated config + payload
  contract + stdin 注入 test で検証する従来方針を維持）。
- Copilot `.json` asset 欠落の upstream 修正（既知の制限として維持）。
- hook 経由の network call の追加。

## Requirements

1. **canonical request**: adapter は native payload を
   `{kind: "file-read", path, cwd, sourceTool?}` または
   `{kind: "shell", command, cwd, sourceTool?}` に normalize する。
   policy はこの形だけを解釈し、target 固有 field を参照しない。
2. **shared policy の単一性**: token threshold・workspace 内判定・
   `count-tokens` 呼出し・`FILE_TOO_LARGE` fail-closed・fast path
   （bytes ≤ threshold で CLI spawn しない）を共有 module に集約し、
   adapter へ複製しない。
3. **Windsurf**: 既存 `pre_read_code` の block/allow/fail-open/
   `FILE_TOO_LARGE` fail-closed 挙動を regression させない。
   `pre_run_command` で shell invocation を canonical shell request へ
   normalize する。
4. **Claude Code**: `PreToolUse` payload（`tool_name` / `tool_input`）を
   normalize し、`Read` → file-read、`Bash` / `PowerShell` → shell。
   deny は Claude Code が正式に解釈する
   `hookSpecificOutput.permissionDecision: "deny"` で表現する。
   malformed / unknown payload や adapter failure で通常 tool use を
   壊さない（fail-open）。
5. **Cursor**: `beforeReadFile` → file-read、`beforeShellExecution` →
   shell。deny は `{permission: "deny"}`、allow 経路を含め exit 0 では
   schema 適合の応答を返す（Cursor は permission hook で exit 0 +
   schema 不適合応答を block 扱いするため）。
6. **Copilot**: never-nonzero invariant を維持する。deny は stdout の
   `permissionDecision` JSON + exit 0。`view` → file-read、shell 系
   toolName（`bash` / `powershell` / `shell`）→ shell request。
7. **Codex**: current official contract の再検証結果を反映する。
   APM 経由で到達可能な範囲（`PreToolUse` matcher `Bash` → shell）を
   adapter でカバーし、file-read unsupported は matrix / guide に明記する。
8. **fail semantics 分離**: fail-closed target では intentional deny と
   infrastructure failure を分離する（deny は native response で表現、
   failure は fail-open 側へ倒す）。
9. **deploy**: `apm install --target <t>` 後、各 target の generated
   config が native event 登録を持ち、対応 adapter が deploy される。
   non-windsurf target の stale `pre_read_code` 登録は引き続き no-op と
   して記録する（APM の全 target verbatim deploy 仕様上 unavoidable）。
10. **#42 seam**: `kind: "shell"` request が policy まで到達する構造を
    持ち、shell command からの read candidate 抽出を後続で接続できる。

## Acceptance Criteria

- [ ] `packages/apm/.apm/hooks/` に shared policy module と per-target
      adapter が存在し、adapter に threshold 判定・shell parser の複製がない。
- [ ] Windsurf 既存 test（`pre-read-hook.test.ts`）が全件 pass する。
- [ ] 各 adapter が representative payload fixture で
      file-read / shell normalize、unrelated tool の素通し、malformed
      payload の fail-open、deny → native response 変換を検証する test を持つ。
- [ ] Copilot adapter の全 code path が exit 0 である test が維持される。
- [ ] `apm install --target <t>` の generated config に各 target の
      native event 登録と adapter command が含まれる e2e assert がある
      （claude: `PreToolUse` matcher group、cursor: `beforeReadFile` /
      `beforeShellExecution`、copilot: `preToolUse`、codex: `PreToolUse`
      nested group、windsurf: `pre_read_code` / `pre_run_command`）。
- [ ] contract matrix（DISC-034r05iHy2gdnpdai9KGpW）と parent-agent guide が
      実装・実測結果と一致する（Codex: file-read unsupported / shell-only、
      hook trust review の制約を含む）。
- [ ] `mise run check`（typecheck / lint / test suite 全件）が通る。

## Deferred Design Concerns

- #42 の shell read candidate 抽出（`cat`/`head`/`tail`/`less`/`more`
  parsing）: canonical shell request の seam は本仕様で提供するが、
  parser 本体は #42 の scope。
- Codex file-read interception: upstream（openai/codex#18491・#20204）で
  read 系 tool が `PreToolUse` を emit するまで不可能。再検証トリガは
  upstream issue の close または tool coverage table の更新。
- 各 agent binary での runtime loader 実機検証: 従来どおり generated
  config + payload contract の静的検証に留める。

## Sources and Relations

- Issue #43: 本仕様の起点と完了条件。
- DISC-034r05iHy2gdnpdai9KGpW: canonical contract matrix と target 分類の
  起点。本仕様の Codex 再検証と APM pass-through 実測で更新する。
- ADR-034r05iHrZUE6W9RDF0V73: per-target adapter deferral の終了（本 chain が
  その follow-up）。verbatim deploy 維持の制約は継続する。
- ADR-034r05iHrZUE6WizQ4LgPa / DESIGN-034r05iHy2gdnoUHGunDtE: Copilot adapter の
  never-nonzero / stdout deny contract。shared architecture へ統合する。
- DESIGN-034r05iHy2gdnoFHbdllXV: `pre_read_code` hook と bundle layout の現行設計。
- SPEC-034r05iHK8bA7gky93Dal0 / DESIGN-034r05iHy2gdnoLTC43RP5: per-target install E2E の
  検証基盤（`apm-install-targets.test.ts`）を拡張して使う。
- SPEC-034r05iHK8bA7go4BQtGNS / DESIGN-034r05iHy2gdnoPoUu7BiO: hook bundle は
  `packages/apm/.apm/hooks/` に配置する制約。

---
id: "DISC-gHDyRKX2Xbdm58SDpCFLLt"
type: "discovery"
status: "resolved"
title: "Devin PreToolUse rewrite runtime verification (issue #148)"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/148"
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
  related:
    - "./pretooluse-auto-routing-to-tasks-and-workflows.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../impl/ir/add-hook-route-arm-and-rewrite-verdict.md"
    - "../impl/ir/add-hook-target-rendering-for-rewrite-and-codex-split.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../impl/ir/add-hook-route-arm-and-rewrite-verdict.md"
    - "../impl/ir/add-hook-target-rendering-for-rewrite-and-codex-split.md"
    - "../impl/ir/add-hook-entry-route-loading-and-eval-supply.md"
  verified-by: []
  references:
    - "https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "issue #148 の Rewrite verdict は Devin CLI (SWE-2 無料モデル) の実環境で `hookSpecificOutput.updatedInput.command` を emit し、native approval を維持したまま `cheaper route run` へ委譲できるか"
    scope: "Devin CLI 3000.11.92 / swe-2-max / Windows + Git Bash。`tmp/devin-verify` の隔離 workspace で hook 直接再現と `devin -p` 実機実行の両面を観測。他 target (claude/codex/cursor/copilot/windsurf) はユーザー指示により未検証"
    outcome: "rewrite は実機で発火 — Devin の exec `git status` が `cheaper.exe route run --payload <b64>` へ差し替えられ、routed-exec workflow 経由で実行・route log + usage `kind:route` を記録。`permissionDecision` 非送出で native approval は維持 (auto/smart モードで confirmation gate が残存)。payload cwd と exec cwd の乖離は fail-closed 再検証 (cwd-mismatch) で正しく拒否されることも実機確認"
---

# Devin PreToolUse rewrite runtime verification (issue #148)

## Exploration Goal

issue #148 の残受け入れ条件「実機検証で承認フロー維持確認」を Devin
ターゲットで充足する。具体的には (1) Devin `exec` の PreToolUse で
`hookSpecificOutput.updatedInput.command` が emit される、
(2) `permissionDecision` を付けず native approval が維持される、
(3) rewrite 後の `route run --payload` が実際に routed Workflow を
実行する、の 3 点をこの環境で観測する。他 target はユーザーの
指示により実機検証を省略し contract レベルで通す。

## Environment and Setup

- Devin CLI `3000.11.92`、model `swe-2-max`（無料 tier）、
  Windows + Git Bash、`cheaper` は `target/debug` (HEAD `fd2d814`)
- 検証 workspace `tmp/devin-verify/`（gitignore 済み）:
  - `config/policy.yaml` — grant `probe-git` (`git` / argv `status *`)
  - `config/routes.yaml` — user 層 route `probe-git-status`
    (`git status` → routed-exec workflow)。target path は層内相対
    パス必須のため `config/routed-exec.workflow.json` に同梱
  - `.cheaper/routes.yaml` — project 層の同名 route は `trusted`
    未指定で suppressed（user 層が有効。resolve 出力の
    `"suppressed":"probe-git-status"` がそれを示す）
  - `.devin/config.json` — `hook-config sync` で登録
    (`verify` clean)。登録コマンドは
    `cheaper hook pre-tool-use --target devin`
- **env 継承が必須**: 登録 hook は bare `cheaper hook …` で起動される
  ため、`CHEAPER_CONFIG_DIR` は `devin` 起動プロセスへ export し
  子 hook へ継承させる必要がある。未設定だと default
  `~/.config/cheaper` を参照し grant が見つからず
  viability=`NoGrant` → silent allow（初回実機 run の allow 原因。

## Key Findings

### Confirmed

- **hook 単体の rewrite emit（再現）**: devin shape payload
  (`{"hook_event_name":"PreToolUse","cwd":…,"tool_name":"exec",
  "tool_input":{"command":"git status"}}`) を stdin 投入 →
  ```json
  {"hookSpecificOutput":{"hookEventName":"PreToolUse",
   "updatedInput":{"command":"\"F:\\…\\cheaper.exe\" route run --payload eyJjb21t…"}}}
  ```
  `permissionDecision` も top-level `decision` も存在せず exit 0。
  payload decode → `{command:"git status",executable:"git",
  argv:["git","status"],cwd:…,env:{}}`。
- **実機 Devin 実行で rewrite 発火**: `devin -p … --permission-mode
  dangerous` で、agent の exec `git status` が rewrite されて実行
  され、tool output 末尾に `[cheaper route log:
  1792680858122-27900-18e0e1be100b1b58; truncated: false]` の
  trailer が付いた（`route run` 固有の後置行 — 直接 exec では
  出ない）。usage.jsonl に `kind:"route"` record
  （ts `2026-10-22T14:54:20Z`、route `probe-git-status`、
  originalTokens 88 → returnedTokens 120、exitCode 0）と
  route-logs `<runId>.log`（git status 全量出力）を確認。
- **native approval 維持**: `--permission-mode auto` / `smart` では
  「rejected a tool call that requires confirmation. Running in
  non-interactive mode」で exec が止まる — hook が
  `permissionDecision` を emit しないため Devin 自身の承認ゲートが
  残存することが実機で確認できた（print モードで confirmation を
  表示できないため reject）。
- **fail-closed 再検証**: Devin が git repo root
  （`F:\repositry\cheaper`）を workspace と解釈して exec cwd とした
  一方、hook payload の cwd は起動 dir だった場合、`route run` が
  `rejected: cwd-mismatch` で拒否（run.rs:60-65 —
  `canonicalize(payload.cwd)` と `canonicalize(exec process cwd)`
  の不一致検査）。検証 dir で `git init` して workspace root を
  一致させた後は正常に routed 実行された。この拒否は設計どおり
  （payload の承認範囲外 cwd での実行を防ぐ）であり、Devin 自身も
  再現実験で同一挙動を確認していた。
- **Devin の tool_input 契約**: hook payload に
  `hook_event_name:"PreToolUse"` + `tool_name:"exec"` +
  `tool_input.command` が来ることは normalize 成功（eval が deny を
  返した）で確認。emit shape は docs.devin.ai
  lifecycle-hooks の `hookSpecificOutput.updatedInput` 記載と一致。

### Methodology notes

- Git Bash `echo`/`printf` の `\\` エスケープで JSON payload が
  意図どおり届かず normalize→None→silent allow になる事例があった。
  手動再現は payload を file に書いて `< file` で行うのが確実。
- Rewrite verdict は dedup cache へ persist しない設計
  （`is_shareable=false`）のため、hook-eval store に rewrite の
  痕跡は残らない — rewrite の観測は emit JSON・`route run` 副作用
  （usage/route log/stdout trailer）で行う。

## Open Questions

- Devin の exec cwd が hook payload の `cwd` と異なるケース
  （起動 dir ≠ workspace root）は本番でも起こり得るか。現仕様では
  fail-closed で安全側に倒れるが、rewrite されたコマンドが
  payload cwd で実行される保証は host 側の exec cwd 規約に依存する。
  Devin が「workspace root とは別 dir で exec する」ケースがあるなら
  payload cwd ではなく exec cwd を root とする再検証要否の再考余地
  （別 issue 候補）。
- claude/codex/cursor/copilot/windsurf は本検証の対象外（ユーザー
  指示）。codex/cursor/copilot の rewrite 有効化は #150 の範囲。

## Promotion Candidates

- [ ] #148 close の根拠として本記録を issue へ添付（Devin 検証済み、
      他 target は指示どおり contract 通過扱い）
- [ ] 「exec cwd と payload cwd の乖離」を Open Questions から起票
      するかは所有者判断

## Sources and Relations

- Issue #148: 受け入れ条件の出典。
- `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`:
  rewrite / payload / fail-closed 再検証の設計根拠。
- IR `IMPL-034JjvBItf3G44b0tll2Sh`（route arm + Rewrite verdict）、
  `IMPL-034JjMYRBAtmxiGp1639qW`（per-target render + codex split）、
  `add-hook-entry-route-loading-and-eval-supply`（hook entry の
  routes/policy load）— 本検証が verifies する実装。
- Devin 公式 docs `cli/extensibility/hooks/lifecycle-hooks`:
  `hookSpecificOutput.updatedInput` contract。

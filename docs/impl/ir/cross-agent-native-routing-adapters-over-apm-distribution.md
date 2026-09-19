---
id: "IMPL-034rALLKuPPgsnclaaPRYT"
type: "impl"
status: "completed"
title: "Cross-agent native routing adapters over APM distribution"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/apm/.apm/hooks/routing-policy.js"
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-run-command.js"
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-read-code-pretooluse.js"
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-read-code-cursor.js"
      - type: "file"
        path: "packages/cli/test/e2e/cross-agent-routing-adapters.test.ts"
    modified:
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-read-code.js"
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-read-code-copilot.js"
      - type: "file"
        path: "packages/apm/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/cli/test/e2e/apm-install-targets.test.ts"
      - type: "file"
        path: "packages/cli/test/e2e/copilot-pre-read-hook.test.ts"
      - type: "file"
        path: "packages/cli/vitest.config.ts"
      - type: "file"
        path: "docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md"
    - "docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md"
    - "docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md"
    - "docs/tasks/contract-matrix-and-parent-agent-guide-updates.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/cross-agent-native-routing-adapter-architecture.md"
    - "docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Cross-agent native routing adapters over APM distribution

## Summary

APM 配布を維持したまま、全 target の native hook event を
canonical routing request（`file-read` / `shell`）へ正規化する
per-target adapter 層を実装した。large-read 判定は共有
`routing-policy.js` に集約し、adapter は payload 変換と応答
合成のみを担う。Claude Code / Codex は `PreToolUse` nested
group、Cursor は `beforeReadFile` / `beforeShellExecution`、
Copilot は `preToolUse`（matcher 絞り込み）、Windsurf は
`pre_read_code` + `pre_run_command` で native event に到達する。

## Context

PLAN-034rAaIojIXLbCI0pJGpYg（SPEC-034rzTJydXfLvQhTIqiDqW /
ADR-034rzTJYDRmre6D5OS8HDW / DESIGN-034rA79Uog7JOa7evw6rOL /
DISC-034rA43JiB611pUjLPA4yR）に従う。GitHub issue #43。
DISC-034r05iHy2gdnpdai9KGpW の contract matrix を実装状態へ更新
する task も含む。issue #42 の shell command parser は scope 外で、
canonical `shell` request が adapter まで到達する allow seam のみ
実装した。

## Implementation

### Changes

- `packages/apm/.apm/hooks/routing-policy.js`（新規）: stdin 読取・
  canonical request 評価・workspace 判定・threshold fast path・
  `count-tokens` spawn・`FILE_TOO_LARGE` fail-closed・delegation
  guidance を集約する共有 module。`{kind:"file-read"}` は測定して
  allow/deny verdict、`{kind:"shell"}` は issue #42 が接続する
  まで常に allow を返す seam
- `packages/apm/.apm/hooks/pre-read-code.js`: Windsurf adapter を
  thin wrapper 化し policy へ委譲（deny は stderr + exit 2）
- `packages/apm/.apm/hooks/pre-run-command.js`（新規）: Windsurf
  `pre_run_command` を shell request へ正規化する seam adapter
- `packages/apm/.apm/hooks/pre-read-code-pretooluse.js`（新規）:
  Claude Code / Codex 共用。`tool_name` ∈ `Read|Bash|PowerShell`
  を canonical request へ正規化し、deny は stdout の
  `hookSpecificOutput.permissionDecision: "deny"` JSON（exit 0）。
  Codex では read 系 tool が `PreToolUse` を emit しないため
  shell のみ到達（file-read は unsupported として matrix 明記）
- `packages/apm/.apm/hooks/pre-read-code-cursor.js`（新規）:
  `beforeReadFile` / `beforeShellExecution` を処理し、deny は
  `{permission:"deny", user_message, agent_message}`、allow 経路でも
  `{permission:"allow"}` を emit して schema 適合を保つ
- `packages/apm/.apm/hooks/pre-read-code-copilot.js`: shared policy
  へ移譲。`view`（file-read）に加え `bash|powershell|shell`
  （shell seam）を受理し、`toolArgs` の string/object 揺れと
  foreign shape（Cursor 型 payload 等）への明示 allow を維持。
  全経路 exit 0 の never-nonzero invariant を保持
- `packages/apm/.apm/hooks/hooks.json`: `pre_run_command`、
  `preToolUse`（matcher `view|bash|powershell|shell`）、
  `PreToolUse`（matcher `Read|Bash|PowerShell` の nested
  `{type:"command"}` group）、`beforeReadFile`、
  `beforeShellExecution` を追加登録。stale `pre_read_code` は
  verbatim deploy の no-op として維持
- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts`
  （新規、12 tests）: 各 adapter の deny 応答・bundled CLI
  経由・FILE_TOO_LARGE・fail-open・shell seam・schema 適合を
  stdin 注入で検証
- `packages/cli/test/e2e/copilot-pre-read-hook.test.ts`: shell
  tool 正規化・string toolArgs・foreign shape cross-fire 防止を追加
- `packages/cli/test/e2e/apm-install-targets.test.ts`: bundle
  layout（policy + 4 adapter + bundled CLI）と per-target
  generated config（claude nested group・codex verbatim・cursor
  2 event・copilot merged group・windsurf pre_run_command）の
  assert を追加
- `packages/cli/vitest.config.ts`: full-suite 並列負荷下の
  beforeEach timeout flake 対策として `hookTimeout: 30_000`
- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`:
  final classification を実装後の状態へ更新（claude/cursor
  functional、codex shell-only functional + file-read unsupported）
- `docs/guides/parent-agent-integration.md`: target 別挙動を
  adapter・event・deny 表現の実態に合わせて更新

### Decisions During Implementation

- **verbatim deploy に合わせた matcher + payload 二重防御**: APM は
  hook entry を全 target へ verbatim deploy するため、adapter 側で
  event 名・tool 名・payload shape を検査して cross-fire を防ぐ
  （matcher は generated config 側の第一防御、shape guard は第二）
- **Codex deny の runtime 解釈は実機未検証**: `hookSpecificOutput`
  deny JSON は Claude 契約に倣う。Codex が unsupported field で
  hook を failure 扱いしても fail-open（tool call 継続）側に倒れ
  るため安全側の誤りとして許容。file-read unsupported は
  upstream gap（openai/codex#18491・#20204）に記録済み
- **Cursor は allow でも schema 応答を emit**: Cursor permission
  hook は応答 schema 適合が前提のため、malformed/unrelated 入力でも
  `{permission:"allow"}` を返して exit 0 する
- **shell request は allow seam**: issue #42 の parser 実装前に
  canonical request が全 adapter に到達する経路だけを確立し、
  verdict 自体は常に allow（将来 parser を差し込む注入点）

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `vitest run test/e2e/cross-agent-routing-adapters.test.ts
  test/e2e/pre-read-hook.test.ts test/e2e/copilot-pre-read-hook.test.ts`:
  36/36 pass
- `vitest run test/e2e/apm-install-targets.test.ts`: 6/6 pass
  （実 apm install による全 target generated config 検証）
- `mise run check` 全 pass: lint → version check → doc id check →
  typecheck → 760 tests（core 274 + runtime-pi 315 + cli 171）→
  bundle 再生成 → freshness clean
- 注: 初回の `mise run check` は並列負荷下の `beforeEach` timeout
  で 4 test flake。`hookTimeout` 引き上げ後に全 pass

## Risks

- 実 agent binary での runtime loader 挙動（未知 event の
  accept/ignore、Codex の `hookSpecificOutput` 解釈）は引き続き
  実機未検証 — generated config と documented contract の静的
  検証 + stdin 注入 e2e が evidence の上限
- matcher regex が verbatim deploy されるため target ごとの
  matcher 意味論差（glob vs regex）は drift の可能性がある。
  実機で不一致なら adapter の shape guard が fail-open で吸収
- `pre_run_command` / shell seam は常に allow を返すため、現状
  shell 経由の large-read は検出されない（issue #42 の接続待ち）

## Follow-ups

- issue #42: canonical `shell` request への command parser 接続
- Codex file-read: upstream `PreToolUse` emit gap の修正追跡
  （openai/codex#18491・#20204 が close されたら Read coverage を
  adapter で有効化）
- 実 agent runtime loader の実機検証（全 target 共通の open item）

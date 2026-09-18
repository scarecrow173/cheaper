---
id: "IMPL-03kR39KNEJ13KN9NwesVWy"
type: "impl"
status: "completed"
title: "distributed skill and parent-agent guide sync (PLAN-000004taRcf5XDevo8jYJz)"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../../tasks/sync-cheaper-usage-skill-md-with-current-cli-and-hook-surface.md"
    - "../../tasks/correct-fail-open-fail-closed-contract-in-parent-agent-guide.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/sync-cheaper-usage-skill-md-with-current-cli-and-hook-surface.md"
    - "../../tasks/correct-fail-open-fail-closed-contract-in-parent-agent-guide.md"
  implemented-by: []
  depends-on:
    - "../../plans/synchronize-distributed-skill-and-parent-agent-guide.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
    - "../../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
    - "../../adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/synchronize-distributed-skill-and-parent-agent-guide.md"
  derived-by: []
  verifies:
    - "../../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# distributed skill and parent-agent guide sync

## Summary

Issue #34: `cheaper-usage` SKILL.md と
`docs/guides/parent-agent-integration.md` を現行 CLI surface・hook
contract・credential 配置に同期した。実装コードの変更なし。

## Context

TASK-06WDQr0NHYErusR6Cmye43・TASK-002DRbiVRuFJ5XWHvsWW6i を実施。
上流は PLAN-000004taRcf5XDevo8jYJz / DESIGN-0005kq1c3Wo7rdnwZR8Sp3 /
SPEC-003hd7W00xCikuCXnwHA4C / ADR-066GRjHT22hXEgey2dpq7f。

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  - Commands に `auth status [provider]` / `auth login <provider>` /
    `auth logout <provider>` を `cheaper --help` 実 output と同一表記で追加
  - option 表に `--method <id>` / `--type <oauth|api_key>` を追加
  - 「pre-read hook currently functions on Windsurf only」を target
    matrix 記述に置換: Windsurf functional、GitHub Copilot functional
    （`preToolUse` adapter、`deny` は stdout + exit 0、JSON asset
    deploy gap を併記）、Claude / Codex / Cursor は no-op
  - credential 記述を `<userConfigDir>/pi/auth.json`・project-local
    ではない明示に確定
- `docs/guides/parent-agent-integration.md`:
  - hooks の説明を「計測不能・判定不能は fail-open」の一般化から
    「infra failure = fail-open / parseable `FILE_TOO_LARGE` =
    fail-closed（Windsurf exit 2、Copilot deny + exit 0）」の
    非対称 contract に修正
  - Copilot 箇条の block 条件に `FILE_TOO_LARGE` verdict を追記
  - 「boundary 外・binary・計測不能は fail-open」を envelope 判定
    ベースの記述に修正し file type との直接結合を解消
  - Copilot JSON asset deploy gap の記述は維持

### Decisions During Implementation

- skill の hook matrix 内に Copilot JSON asset gap を併記した —
  parent agent が Copilot 環境で file-schema worker を呼んで失敗する
  前に気づけるよう、no-op target と同じ階層で簡潔に記載した。
- 「binary」表現は削除せず「判定は `count-tokens` envelope のみに
  依存」と count contract 側から記述した（issue の「file type と
  fail-open を直接結び付けない」要求どおり）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `node packages/cli/bin/cheaper.js --help` の実 output と skill の
  command/option 表を突合 — 一致
- `rg` 突合: auth 系 command・`--method`・`--type`・hook matrix・
  credential 記述が skill に存在
- hook script（`pre-read-code.js` / `pre-read-code-copilot.js`）の
  全分岐と guide の fail-open/fail-closed 記述を突合 — 一致
- `usage.md` / `providers.md` / `README.md` / `cheaper-bulk-read`
  の関連記述を rg 突合 — 矛盾なし
- `audit_docs.js --type all` — findings なし
- `mise run check` — lint/build/test 通過を確認（e2e の APM install
  hook timeout は並列負荷下の環境 flake; 単独再実行では pass）

## Risks

- CLI の command/option 追加時に skill 表が drift する可能性は残る。
  providers.md を canonical とし skill は最小 surface のみに留めて
  あるため drift 面積は限定的。

## Follow-ups

- APM が copilot target へ `.json` asset を deploy するようになった
  時点で、skill・guide の gap 記述を外す（upstream 修正待ち）。

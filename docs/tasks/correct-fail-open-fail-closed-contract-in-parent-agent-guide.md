---
id: "TASK-002DRbiVRuFJ5XWHvsWW6i"
type: "task"
status: "done"
title: "Correct fail-open/fail-closed contract in parent-agent guide and verify consistency"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
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
    - "docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md"
  implemented-by: []
  depends-on:
    - "docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md"
    - "docs/tasks/sync-cheaper-usage-skill-md-with-current-cli-and-hook-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synchronize-distributed-skill-and-parent-agent-guide.md"
    - "docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-002DRbiVRuFJ5XWHvsWW6i: Correct fail-open/fail-closed contract in parent-agent guide and verify consistency

## Work

`docs/guides/parent-agent-integration.md` の fail-open/fail-closed
記述を hook 実装の非対称 contract に同期し、関連文書との整合
確認・doc audit・最終検証を行う（Issue #34 /
SPEC-003hd7W00xCikuCXnwHA4C）。

### 1. fail-open / fail-closed contract の修正

現行の「計測不能・判定不能なケースは fail-open」の一般化を、
hook 実装（`pre-read-code.js` / `pre-read-code-copilot.js`）の
実 contract に沿って修正する:

- **fail-open**（read を妨げない）: malformed stdin payload /
  対象外 event・tool / file path 解決不能 / workspace 外 /
  不存在・非 file / CLI 起動失敗 / stdout 非 parseable /
  非 0 exit / `ok:false` 等の infrastructure failure
- **fail-closed**（read を block）:
  - `count-tokens` が parseable な `FILE_TOO_LARGE` envelope を
    返した場合 — 計測不能だが確実に巨大と authoritative に
    判定する verdict（SPEC-0012 / DESIGN-0017）
    - Windsurf: exit 2 + stderr guidance
    - Copilot: `{"permissionDecision":"deny"}` + exit 0
  - measured tokens が threshold 超過の場合も同じく block

### 2. file type と fail-open の直接結合を解消

「boundary 外・binary・計測不能な file は fail-open」の記述を、
file type ではなく count/read contract に沿った説明に修正する
（hook は binary を特別扱いしない；判定は `count-tokens` の
envelope のみに依存する）。

### 3. Copilot JSON asset gap の維持

「APM が copilot target へ `.json` asset を deploy しないため
file-schema worker は利用できない」という既知制限の記述を
削除・弱化しない。

### 4. 整合確認

- `docs/guides/usage.md` / `docs/guides/providers.md` /
  `README.md` / `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md`
  と矛盾がないか rg で確認（矛盾があれば最小修正）
- hook script・E2E test（`apm-install-targets.test.ts` /
  `pre-read-hook.test.ts` / `copilot-pre-read-hook.test.ts`）を
  contract の一次情報源として突合（変更しない）
- `audit_docs.js --type all` が no findings
- `mise run check` が pass

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/synchronize-distributed-skill-and-parent-agent-guide.md`
- `docs/specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md`
- `docs/adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md`
- `docs/designs/synchronize-distributed-skill-and-parent-agent-guide.md`

## Dependencies

- TASK-06WDQr0NHYErusR6Cmye43（SKILL.md 同期）の後に実施し、
  skill/guide 間の相互矛盾を一度に確認する

## Affected Files

- `docs/guides/parent-agent-integration.md`（modify）
- 整合確認で必要なら関連 doc の最小修正

## Done When

- [x] `FILE_TOO_LARGE` fail-closed と infra failure fail-open が
  明確に区別されている
- [x] Windsurf exit 2 / Copilot `permissionDecision: "deny"` +
  exit 0 の surface 形が記述されている
- [x] file type と fail-open の直接結合が解消されている
- [x] Copilot JSON asset gap の記述が維持されている
- [x] providers.md / usage.md / README / cheaper-bulk-read と
  矛盾がない
- [x] `audit_docs.js --type all` no findings
- [x] `mise run check` pass

## Verification

- [x] hook script の各分岐（exit 0 / exit 2 / `deny()`）と guide
  記述の突合
- [x] `rg -i "fail-open|fail-closed|FILE_TOO_LARGE|binary"
  docs/guides/parent-agent-integration.md`
- [x] `rg -i "json.*asset|deploy" docs/guides/parent-agent-integration.md`
  で gap 記述の維持を確認
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all --json`
- [x] `mise run check`

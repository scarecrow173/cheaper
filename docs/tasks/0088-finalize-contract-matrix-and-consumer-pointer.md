---
id: "TASK-0088"
type: "task"
status: "todo"
title: "Finalize contract matrix and consumer pointer"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/0013-cross-agent-distribution-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0013-cross-agent-distribution-verification.md"
    - "docs/tasks/0087-per-target-apm-install-verification-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
    - "docs/adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  verified-by: []
  references:
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by: []
---

# Finalize contract matrix and consumer pointer

## Work

contract matrix を canonical 化し、consumer 向け文書に pointer を
張る（DESIGN-0018 §4）。

### 1. DISC-0009 への最終分類追記

`docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
の per-target matrix 節に、最終分類と本検証の test evidence 参照を
追記して close する。canonical location は DISC-0009 のみ。

最終分類:

| target | classification | evidence |
| --- | --- | --- |
| windsurf | functional | 既存 `apm-install.test.ts`（hook block 実動作確認済み） |
| claude | deploys-but-noop | `apm-install-targets.test.ts` — `pre_read_code` verbatim 登録、native event は `PreToolUse` + matcher `Read` |
| cursor | deploys-but-noop | 同上 — native event は `beforeReadFile`（`failClosed` 未指定時は fail-open） |
| copilot | deploys-but-noop | 同上 — native event は `preToolUse` + `toolName == "view"`。command hook は fail-closed（非 0 exit が deny）で将来 adapter の不変条件 |
| codex | deploys-but-noop かつ unsupported | 同上 — file-read tool が `PreToolUse` を emit しない upstream gap（openai/codex#20204）。adapter 対象外 |
| agent-skills | N/A（skills-only） | 同上 — hook bundle / config 不存在を assert |

evidence 参照として `packages/cli/test/e2e/apm-install-targets.test.ts`
（TASK-0087）と既存 `apm-install.test.ts` を明記する。

### 2. consumer 向け pointer 追加

`.apm/skills/cheaper-usage/SKILL.md` の "Locating the CLI" 節に 1 文
追加する:

> The pre-read hook currently functions on Windsurf only. For the
> per-target contract, see the canonical matrix in DISC-0009
> (`docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`).

matrix 内容（event 名・分類表）を README・skill・spec 等へ複写し
ない — contract 知識の分散は SPEC-0013 の requirement で禁止
されている（DRY）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0013-cross-agent-distribution-verification.md`
- `docs/designs/0018-cross-agent-distribution-verification.md`
- `docs/specs/0013-cross-agent-distribution-verification.md`
- `docs/adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md`

## Dependencies

- PLAN-0013、TASK-0087（matrix の evidence となる test 結果が必要）

## Affected Files

- `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
- `.apm/skills/cheaper-usage/SKILL.md`

変更しない file（明示）: `README.md`、`docs/specs/` 配下、その他
matrix 内容を複写し得る全 doc。

## Done When

- [ ] DISC-0009 matrix に 6 target の最終分類（functional /
      deploys-but-noop / unsupported / N/A）と Codex coverage gap・
      Copilot fail-closed semantics・test evidence 参照が記録されている
- [ ] cheaper-usage SKILL.md "Locating the CLI" 節に matrix への
      pointer があり、contract 知識の複写がない

## Verification

- [ ] `grep -n "functional" docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md` が分類行を返す
- [ ] `grep -n "20204" docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md` が Codex coverage gap 参照を返す
- [ ] `grep -n "DISC-0009" .apm/skills/cheaper-usage/SKILL.md` が
      "Locating the CLI" 節の pointer を返す
- [ ] `mise exec -- pnpm -F cheaper test` が exit 0（doc-only 変更で
      regression なし）

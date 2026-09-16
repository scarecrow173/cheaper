---
id: "PLAN-0013"
type: "plan"
status: "in-progress"
test-spec-skip: "Verified by package unit/e2e test suites and CI; no dedicated test-spec doc authored."
title: "Cross-agent distribution verification"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0018-cross-agent-distribution-verification.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0018-cross-agent-distribution-verification.md"
    - "../specs/0013-cross-agent-distribution-verification.md"
  implemented-by:
    - "../tasks/0086-parameterize-apm-e2e-fixture-by-target.md"
    - "../tasks/0087-per-target-apm-install-verification-e2e.md"
    - "../tasks/0088-finalize-contract-matrix-and-consumer-pointer.md"
  depends-on:
    - "../plans/0008-apm-delegation-layer.md"
    - "../plans/0009-self-contained-cli-bundle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0018-cross-agent-distribution-verification.md"
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  derived-by: []
  verifies:
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  verified-by: []
  references:
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by: []
metadata: {}
---

# PLAN-0013: Cross-agent distribution verification

## Goal

`apm.yml` に宣言された全 target（copilot / codex / claude / cursor /
windsurf / agent-skills）について、`apm install` の deploy 結果と
generated hook config の挙動（functional / documented no-op）を
table-driven e2e test と canonical contract matrix で検証済みの証拠
として固定する。成果物は per-target の機能適応ではなく test と
matrix という証拠である（SPEC-0013 / DESIGN-0018）。

## Scope

単一の implementation stream。test fixture 拡張 → per-target
verification test → matrix 確定・consumer 文書 pointer の serial な
3 task で構成し、分岐・並行可能な独立 subsystem は存在しないため
plan 分割はしない。

### In Scope

- `packages/cli/test/e2e/apm-fixture.ts` の target パラメータ化
  （default `"windsurf"` で後方互換）
- 新規 `packages/cli/test/e2e/apm-install-targets.test.ts` による
  non-windsurf 5 target の table-driven install verification
- DISC-0009 への最終分類（functional / deploys-but-noop /
  unsupported / N/A）と test evidence 参照の追記
- `.apm/skills/cheaper-usage/SKILL.md` "Locating the CLI" 節への
  matrix pointer 1 文の追加（contract 知識の複写はしない — DRY）

### Out of Scope

- per-target hook adapter の実装（event-name mapping・stdin/stdout
  translation・fail-closed 対応）— ADR-0016 で defer 確定
- 実 agent 駆動 e2e（実 binary の実行）
- `apm.yml` targets・`.apm/` 配布物の変更
- 既存 `apm-install.test.ts` / `pre-read-hook.test.ts` /
  `delegation-loop.test.ts` の変更（windsurf regression 防止のため
  無変更を維持）
- dogfooded `.windsurf/hooks.json` の deployment drift 是正
  （DESIGN-0018 deferred concerns 参照）

## Upstream Documents

- `docs/designs/0018-cross-agent-distribution-verification.md`
  （approved — 本 plan の直接の実装対象）
- `docs/specs/0013-cross-agent-distribution-verification.md`
- `docs/adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md`
- `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
  （canonical matrix の記録先）

## Design Inputs

- DESIGN-0018 §1: fixture 拡張方式（option (a) — 既存 helper に
  optional `target` パラメータ + default `"windsurf"`）と target →
  deploy dir / generated config の map
- DESIGN-0018 §2–§3: test file 配置と per-target assertion 一覧
- DESIGN-0018 §4: contract matrix の canonical 化方針（DISC-0009 のみ、
  consumer 文書は pointer のみ）
- ADR-0016: verbatim `pre_read_code` deploy の documented no-op 維持と
  Copilot fail-closed / Codex coverage gap の固定制約

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `packages/cli/test/e2e/apm-fixture.ts` | `makeConsumerLayout` / `apmInstall` / `bundleRoot` / `bundledCli` を `target` パラメータ化、`generatedConfigPaths` を新規追加 | TASK-0086。全引数 `target = "windsurf"` default で既存 call site 無変更 |
| `packages/cli/test/e2e/apm-install-targets.test.ts` | non-windsurf 5 target の table-driven install verification e2e | TASK-0087。新規 file。`apmAvailable()` で skip |
| `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md` | 最終分類 + test evidence 参照の追記（canonical matrix の確定） | TASK-0088 |
| `.apm/skills/cheaper-usage/SKILL.md` | "Locating the CLI" 節に matrix pointer 1 文追加 | TASK-0088。contract 複写禁止 |
| `docs/plans/README.md` / `docs/tasks/README.md` | index 更新 | 本 doc 群の作成時に実施済み |

変更しない file（明示）: `packages/cli/test/e2e/apm-install.test.ts`、
`pre-read-hook.test.ts`、`delegation-loop.test.ts`、`apm.yml`、
`.apm/hooks/**`、`.apm/skills/**`（cheaper-usage への pointer 追加を
除く）。

### Fixture target map（DESIGN-0018 §1、consumer fixture の dep dir 名 = `pkg`）

| target | bundleRoot | generated config |
| --- | --- | --- |
| `windsurf` | `.windsurf/hooks/pkg` | `.windsurf/hooks.json` |
| `claude` | `.claude/hooks/pkg` | `.claude/apm-hooks.json` + `.claude/settings.json` |
| `codex` | `.codex/hooks/pkg` | `.codex/hooks.json` |
| `cursor` | `.cursor/hooks/pkg` | `.cursor/hooks.json` |
| `copilot` | `.github/hooks/scripts/pkg` | `.github/hooks/<dep>-hooks.json`（dep 名由来 → readdirSync で解決） |
| `agent-skills` | （hook deploy なし） | （なし） |

## Implementation Sequence

- [ ] TASK-0086: `apm-fixture.ts` を target パラメータ化し、既存
      windsurf e2e が無変更で pass することを確認
- [ ] TASK-0087: `apm-install-targets.test.ts` を新規作成し、5
      target の assertion を table-driven で実装・実行
- [ ] TASK-0088: DISC-0009 に最終分類と evidence 参照を追記し、
      cheaper-usage SKILL.md に pointer を追加

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| TASK-0086（fixture 拡張） | — | TASK-0087 |
| TASK-0087（per-target e2e） | TASK-0086 | TASK-0088 |
| TASK-0088（matrix 確定 + pointer） | TASK-0087（test evidence が必要） | — |

critical path は serial 1 本（0086 → 0087 → 0088）。並行実行可能な
stream はない — 0087 は 0086 の fixture API に依存し、0088 は 0087
の test 結果を matrix の evidence として参照する。

## Task Breakdown

- TASK-0086: Parameterize apm e2e fixture by target
- TASK-0087: Per-target apm install verification e2e
- TASK-0088: Finalize contract matrix and consumer pointer

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | apm CLI 不在環境では e2e が全 skip され、検証が実行されないまま merge され得る | med | med | `apmAvailable()` skip は既存パターン踏襲。TASK-0088 で matrix に記録する evidence は apm 有環境での実実行結果とし、skip 環境では実行ログを採用しない |
| 2 | agent 側の hook contract drift（event 名・schema 変更）は実機 e2e なしでは検出不能 | med | med | SPEC-0013 が許容済みの trade-off。matrix に分類確定時点の証拠を記録し、将来の adapter chain が再検証する |
| 3 | fixture の `target` default を `"windsurf"` 以外に変える・default を外すと既存 call site が破壊される | low | high | default `"windsurf"` を維持し、新規 code からは必ず明示 target を渡す。既存 test 無変更を CI で確認 |
| 4 | copilot の generated config file 名は dep 名由来（`<dep>-hooks.json`）で hardcode 不能 | high | low | `.github/hooks/` 配下を `readdirSync` で走査し `*-hooks.json` を一意解決する（DESIGN-0018 §3） |
| 5 | agent-skills の「hook 不存在」assert が apm 側挙動変化（空 dir 生成等）で脆く壊れる | med | low | 不存在 assert は generated config file と bundle dir（hook bundle の path）に限定する |
| 6 | generated config 内部 schema への過剰結合で apm upgrade に脆い | med | low | assertion は「存在 + JSON parse + `pre_read_code` event key」レベルに留める（DESIGN-0018 non-functional constraints） |

### External Dependencies

- `apm` CLI（local path dep の install 実行に必須。不在時は test skip）
- Node.js（bundled `cheaper.mjs` の `node <cli> --version` /
  `count-tokens` 実行に使用。`process.execPath` 経由）

## Rollback Strategy

全変更は test file・doc のみで production code（`packages/cli/src`、
`.apm/` 配布物、`apm.yml`）に触れない。不可逆な step は存在しない。

- **Trigger**: 既存 windsurf e2e の regression、または fixture 拡張が
  既存 call site を破壊した場合。
- **Procedure**: 対象 commit を `git revert`。fixture 変更のみ戻す場合は
  TASK-0086 の commit のみ revert すれば後続 task の file は compile
  できなくなるため、revert は serial に遡って行う（0088 → 0087 →
  0086 の逆順でも、plan 全体の revert でもよい）。
- **Verification**: `mise exec -- pnpm -F cheaper test` で既存 e2e
  suite が revert 前状態と同一に pass / skip すること。

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| TASK-0086 | `mise exec -- pnpm -F cheaper test` | 既存 e2e が無変更で pass（apm 不在時は skip）。fixture の型エラーなし |
| TASK-0087 | `mise exec -- pnpm -F cheaper test -- apm-install-targets` | 5 target の `it` が個別に pass（apm 不在時は skip） |
| TASK-0087 (regression) | `mise exec -- pnpm -F cheaper test` | `apm-install` / `pre-read-hook` / `delegation-loop` 含む全 suite green |
| TASK-0088 | `grep -n "functional\|deploys-but-noop" docs/discovery/0009-*.md` | 最終分類行が存在し、Codex に openai/codex#20204 参照がある |
| TASK-0088 | `grep -n "DISC-0009\|contract" .apm/skills/cheaper-usage/SKILL.md` | "Locating the CLI" 節に matrix pointer があり、matrix 内容の複写がない |

### SPEC-0013 acceptance criteria → task/test mapping

| SPEC-0013 acceptance criterion | 対応 task / test |
|--------------------------------|------------------|
| 全 declared target の deploy 内容（hook bundle + generated config path/schema）を証明する automated test | TASK-0087 — `apm-install-targets.test.ts`（non-windsurf 5 target）+ 既存 `apm-install.test.ts`（windsurf） |
| 各 target の deploy config が functional / no-op かと理由が canonical record に記載 | TASK-0087（`pre_read_code` event key verbatim assert で no-op を test 上に固定）+ TASK-0088（DISC-0009 への分類記録） |
| contract matrix が native event・stdin shape・blocking mechanism・Codex coverage gap・Copilot fail-closed を記録 | TASK-0088 — DISC-0009 既存 matrix への最終分類追記（既存記述 + evidence 参照で完結） |
| 既存 windsurf e2e が引き続き pass | TASK-0086/0087 の verification（既存 file 無変更 + suite green） |
| 各 target が functional / deploys-but-noop / unsupported に分類 | TASK-0088 — DISC-0009 最終分類テーブル |

## Open Questions / Gaps

- apm が生成する config の内部 schema 詳細（event key 以外の field
  shape）は assertion 対象外であり、正確な構造は TASK-0087 の実装時に
  実測で確認する（assertion 範囲は DESIGN-0018 で固定済みのため
  blocker ではない）
- per-agent 実機での config validation（未知 event を error なく
  ignore するか）は自動化不能 — SPEC-0013 に従い公式 doc 証拠
  （DISC-0009）で補完する

## Self-Review

- [x] SPEC-0013 の全 acceptance criterion が task/test に map されている
      （上記 matrix 参照）
- [x] placeholder（TBD/TODO/implement later）なし
- [x] file path・relation key・status・command・用語が plan / task /
      upstream doc 間で一致

## Implementation Handoff

- Recommended mode: inline execution（dependency graph が serial 1 本で
  独立 stream がなく、delegated dispatch の利がない）
- Independent work streams: なし（0086 → 0087 → 0088 の serial chain）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: DESIGN-0018 は approved。SPEC-0013 は `proposed` のままだが
  design gate（approved design の存在）は満たしており、spec 内容と
  design 内容の間に矛盾はない
- 番号予約: PLAN-0012・TASK-0083..0085 は並行 branch
  （feature/huge-file-token-guard, PR #4）のため本 plan は
  PLAN-0013 / TASK-0086..0088 を使用
- reviewer の注目点: (1) fixture default `"windsurf"` 後方互換が
  守られているか、(2) copilot config 名解決が readdirSync ベースか、
  (3) matrix への追記が DISC-0009 のみで contract 複写がないか
- 主要 upstream: DESIGN-0018（実装設計）、SPEC-0013（要件・
  acceptance criteria）、ADR-0016（defer 制約）、DISC-0009
  （canonical matrix）

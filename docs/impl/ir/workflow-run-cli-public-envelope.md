---
id: "IMPL-034AOjFln9ikSd8laLlspo"
type: "impl"
status: "completed"
title: "Workflow run CLI と public envelope を統合する"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - { type: file, path: packages/cli/src/workflow-context.ts }
      - { type: file, path: packages/cli/src/workflow-run.ts }
      - { type: file, path: packages/cli/test/workflow-run.test.ts }
      - { type: file, path: packages/cli/test/e2e/workflow-run.test.ts }
    modified:
      - { type: file, path: packages/cli/src/args.ts }
      - { type: file, path: packages/cli/src/cli.ts }
      - { type: file, path: packages/cli/src/envelope.ts }
      - { type: file, path: packages/cli/src/text-format.ts }
      - { type: file, path: packages/cli/src/usage-log.ts }
      - { type: file, path: packages/cli/test/workflow-interface.test.ts }
      - { type: file, path: packages/runtime-pi/src/index.ts }
      - { type: file, path: packages/runtime-pi/src/run.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: ["../../tasks/integrate-workflow-run-cli.md"]
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../../plans/implement-one-shot-workflow-execution-and-cli-integration.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Workflow run CLI と public envelope を統合する

## Summary

`cheaper workflow run <path>` を injected DAG engine、canonical tool runtime、Worker runtime、invocation-local artifact store へ接続した。interface と run は共有 context で loader、policy、Worker metadata を準備する。S5 coordinator review は commit `c7c0e6f` で approved。

## Context

承認済み plan S5 と workflow-run CLI public projection test-spec に従う。workflow 実行は worker 単体 usage record に混ぜず、宣言済み outputs と選択 metadata だけを一つの public envelope に投影する。

## Implementation

### Changes

- args parser は `parsed.tokens` から `--input` omission と明示 `-` を区別し、workflow command 専用 option allowlist を適用する。
- `workflow-context.ts` が interface/run 共通の explicit loader、denylist、policy、compiler、Worker metadata preparation を担う。interface は model/runtime/executor を作らない。
- `workflow-run.ts` は omitted input を `{}` とし、明示 input を既存 JSON reader で取得する。compile 後、admission 時に invocation runtime/store を一度だけ作り、最初の Worker で共有 model promise を初期化する。tool-only workflow は model runtime を作らない。
- direct tool と `runWorker` を同じ invocation/parent lease へ接続し、tool owner と Worker child permission を job ID に結びつける。Pi helper handler の canonical export を追加し、artifact publication producer も owner ID を保つ。
- workflow envelope は success/failure の双方で partial public outputs、job outcomes、execution metadata、unavailable artifact metadata を保持する。declared output 以外の job result は公開しない。text variant は generic error より先に dispatch し、usage logger は workflow discriminator を除外する。
- pre-engine failure は format/quiet を保ち、stdout は一 envelope に限定する。通常 command の error diagnostics も quiet 時に stderr へ出さない。

### Decisions During Implementation

- 既存 `readTaskInput` を使い、未指定 input だけ `{}` にする。token values の default `-` では omission を判定しない。
- artifact reference の preview は宣言 output に選ばれた場合のみ出る。top-level artifact metadata は preview 無し・`availability: unavailable` とする。
- Runtime Pi の helper handler export は設計上の composition seam として公開し、workflow-run module に private handler 実装を複製しない。

## Related Experiments

### Adopted

該当なし。

### Rejected

該当なし。

## Validation

- RED: S5 test 初回実行は workflow-run parser/dispatch 不在、input omission、public envelope で失敗した。
- GREEN: `mise exec -- pnpm --filter cheaper exec vitest run test/workflow-run.test.ts test/workflow-interface.test.ts test/args.test.ts test/input.test.ts test/envelope.test.ts test/text-format.test.ts test/usage-log.test.ts test/main.test.ts` — 8 files / 116 tests passed。
- GREEN: `mise exec -- pnpm build` — all workspace packages built successfully。
- GREEN: `mise exec -- pnpm --filter cheaper exec vitest run test/e2e/workflow-run.test.ts test/e2e/workflow-interface.test.ts` — 2 files / 11 tests passed against built CLI.
- `git diff --check` passed before S5 commit. Canonical `mise run check` remains deferred to coordinator merge run.

## Risks

S5 independent coordinator review passed. Provider smoke and canonical merged check remain out of scope; canonical verification is coordinator-side. S6 verification and plan/task closure are recorded in the dependent S6 task and IR.

## Follow-ups

S5 logical commit and coordinator review are complete. S6 verification, scoped audits, and plan/task closure continue in the dependent S6 IR.

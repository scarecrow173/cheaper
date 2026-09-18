---
id: "DESIGN-OVERVIEW"
type: "design"
status: "draft"
title: "System Design Overview"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "workspace-filesystem-boundary-sandboxed-tools.md"
    - "model-resolver.md"
    - "pi-execution-engine.md"
    - "cli-contract-envelope-exit-codes.md"
    - "bulk-reader-contract.md"
    - "cheap-writer-contract.md"
    - "repository-structure-toolchain.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../adr/typescript-pnpm-monorepo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# System Design Overview

## System Boundaries

- `packages/core`（Pi 非依存）: Worker Definition/Resolver、Runtime Policy、tool vocabulary、path security、model tier 分類、error model
- `packages/runtime-pi`: Pi SDK 統合（session・resource loader・制限・abort・retry・出力抽出・repair・usage）、sandboxed tools、mutation journal
- `packages/cli`: コマンド・envelope・exit code・built-in workers/schemas/policy.yaml 同梱
- 依存方向は `cli → runtime-pi → core` の一方向のみ

## Core Components

| 段階（引継ぎ §4） | 担当 design |
| --- | --- |
| CLI 引数解析・stdin 入力・envelope 出力・exit code | DESIGN-034r05iHrZUE6WvDuYjZB4 |
| Worker 解決・frontmatter 検証・input schema 検証 | DESIGN-034r05iHrZUE6WrdtQIOmE |
| Runtime Policy ロード/マージ・CLI restriction・capability 交差 | DESIGN-034r05iHrZUE6WrdtQIOmE |
| Model tier 分類・available model 選択 | DESIGN-034r05iHrZUE6Wu1cMdeJ1 |
| sandboxed tool 実装・workspace boundary・mutation journal | DESIGN-034r05iHrZUE6WsgJOy6Ny |
| Pi session 実行・制限強制・abort 分類・retry・repair・usage | DESIGN-034r05iHrZUE6WvaQsRRsM |
| built-in Worker 定義（bulk-reader / cheap-writer） | DESIGN-034r05iHrZUE6WxkSJi9GF / DESIGN-034r05iHrZUE6Wy5X0Hc88 |
| リポジトリ構成・ツールチェーン | DESIGN-034r05iHrZUE6Wzyjd1NS8 |

## Data Flow

- 親 Coding Agent → `cheaper run <worker> --input -`（stdin JSON）→ envelope 1 行（stdout）+ 診断（stderr）
- core で Worker/Policy/model 解決 → runtime-pi が Pi session を生成し、sandboxed tools と budget を wrap して実行 → journal と events から deterministic meta を実測 → cli が envelope に詰めて返す

## Runtime Invariants の実装位置

| Invariant | 保証するコード |
| --- | --- |
| stdout は result envelope のみ | `cli/envelope.ts`（runtime-pi/core は stdout に書かない） |
| workspace 外アクセス不許可 | `runtime-pi/tools/operations.ts` + 自前 grep/find の `resolveWorkspacePath` |
| requested tools は宣言済み vocabulary のみ | `core/tools/vocabulary.ts` の alias 正規化と `UNKNOWN_TOOL` |
| policy 超過は拒否 | `core/policy` の capability 交差（tools.allow / maxTier） |
| maxTurns/maxToolCalls/timeout | `runtime-pi` の tool execute wrap・`turn_end` 監視・timer abort |
| abort 時 rollback なし・変更は実測報告 | `runtime-pi/journal.ts` → `meta.changes` |
| LLM 出力は schema 適合 JSON のみ | `runtime-pi/output.ts` + repair session |
| 未認証モデルを選択しない | `ModelRuntime.getAvailable()` のみを候補に使う |

## Non-Functional Constraints

- Node 26 / pnpm 11 / ESM only / `minimum_release_age = 7d`
- `@cheaper/core` は `@earendil-works/*` に依存しない
- Pi SDK の sandbox 不在・maxTurns 不在を cheaper 側コードで補完する（ADR-034r05iHfSMW9SGV4dpcy2/0007）

## Detailed Design Documents

読み順: 0008 → 0001 → 0002 → 0003 → 0004 → 0005 → 0006 → 0007

- [0008 リポジトリ構成とツールチェーン](./repository-structure-toolchain.md)
- [0001 コアドメインモデル](./worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md)
- [0002 workspace filesystem boundary と sandboxed tools](./workspace-filesystem-boundary-sandboxed-tools.md)
- [0003 Model Resolver](./model-resolver.md)
- [0004 Pi 実行エンジン](./pi-execution-engine.md)
- [0005 CLI contract](./cli-contract-envelope-exit-codes.md)
- [0006 bulk-reader contract](./bulk-reader-contract.md)
- [0007 cheap-writer contract](./cheap-writer-contract.md)

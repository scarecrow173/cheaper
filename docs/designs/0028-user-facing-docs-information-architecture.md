---
id: "DESIGN-0028"
type: "design"
status: "approved"
title: "User-facing documentation information architecture"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0023-user-facing-docs-information-architecture.md"
    - "../adr/0026-readme-as-user-entrypoint-with-canonical-guides.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0023-user-facing-docs-information-architecture.md"
  implemented-by:
    - "../plans/0023-user-facing-docs-information-architecture.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0005-cli-contract-envelope-exit-codes.md"
    - "./0021-dev-apm-project-and-distributable-package-layout.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0026-readme-as-user-entrypoint-with-canonical-guides.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# User-facing documentation information architecture

## Context

SPEC-0023 / ADR-0026 の実装設計。README を user-facing に再構成
し、目的別 guide・CONTRIBUTING・docs index を整備する。記載
内容は全て実コードで検証済みの contract のみとする。

## Scope

### In Scope

- `README.md` 書き換え（11 項目構成）
- `docs/README.md`（新規）— 3 分類 index
- `docs/guides/usage.md`（新規）
- `docs/guides/configuration.md`（新規）
- `docs/guides/workers.md`（新規）
- `docs/guides/parent-agent-integration.md`（新規）
- `docs/guides/providers.md`（再編）
- `CONTRIBUTING.md`（新規）

### Out of Scope

- `AGENTS.md` の変更（Coding Agent instruction として現行維持。
  CONTRIBUTING.md から参照される側）
- `docs/handoff/` 等の既存記録の移動（index から参照するだけ）
- license・installation method

## Content Responsibility Map

実コード検証済みの canonical contract と記載先:

| Contract | Canonical source | 検証済み根拠 |
|---|---|---|
| CLI commands/options/envelope/exit codes | `guides/usage.md` | `args.ts`・`envelope.ts`・cheaper-usage SKILL.md |
| config discovery/precedence・env vars | `guides/configuration.md` | `dirs.ts`・`policy/load`・`model/config`・`cli.ts` |
| provider/auth・models.json・tier 割当 | `guides/providers.md` | 現行 providers.md（user-facing 部分）・`auth.ts` |
| worker 概念・built-in・schema・custom | `guides/workers.md` | `packages/cli/workers/*`・`worker/resolve` |
| skills/hooks・target 別挙動 | `guides/parent-agent-integration.md` | `packages/apm/.apm/`・DISC-0009・ADR-0016 |
| repo setup・test・構造・記録導線 | `CONTRIBUTING.md` | `mise.toml`・ADR-0020・AGENTS.md 参照 |
| 記録分類 index | `docs/README.md` | 既存 `docs/*` 構成 |
| usage.jsonl 計測 | `guides/usage.md`（言及）| ADR-0025・`usage-log.ts` |

### README 構成（issue の 11 項目）

1. Header: worker runtime の 1-2 文説明（現行「目的」を維持）
2. Why cheaper?: delegation model の短文 + flow
3. Core capabilities: bulk-read/code-write/doc-write・custom
   worker・count-tokens・provider 解決・parent integration を
   1-2 文ずつ
4. How it works: Parent Agent → cheaper → worker → runtime/
   provider → low-cost model の conceptual flow
5. Basic usage: `cheaper bulk-read` 等の public CLI 最小例 +
   envelope 概要（詳細は usage.md へ）
6. Built-in workers: 3 worker の 1 行表
7. Configuration overview: models.json（provider/auth）・
   models.yaml（tier）・policy.yaml・project/user override の
   役割だけ（詳細は configuration.md・providers.md へ）
8. Parent-agent integration: skills/hooks primitive・target 差・
   guide への link
9. Documentation: 目的別 link 一覧
10. Development: CONTRIBUTING.md への gateway 1 行
11. Project status: experimental / breaking-change expectation を
    短く（test 履歴は書かない）

### `docs/README.md` 分類

```text
User guides       -> guides/{usage,configuration,providers,
                     workers,parent-agent-integration}.md
Contributor       -> ../CONTRIBUTING.md（+ AGENTS.md は agent 用）
Architecture      -> specs/adr/designs/discovery/plans/tasks/
                     impl/test-specs/handoff を「何の記録か」1 文ずつ
```

### `providers.md` 再編

- 残す: config file 場所表・Ollama/OpenAI の models.json・
  models.yaml 例・tier 判定ルール・auth 手順
- 移す: docker compose smoke 手順・NVIDIA Container Toolkit 注記・
  EXP-* 実測メモ（`reasoning_effort` 無効化の実測・4b/9b 分類の
  実測・小モデル repair 収束値）→ CONTRIBUTING.md の「実 provider
  検証」節（smoke 手順）と、EXP/IMPL 記録への 1 行参照。
  repair の内部実装詳細（repair.ts への言及）は削り
  「repairAttempts は余裕を持たせる」等の利用者向け助言だけ残す
- usage.jsonl: `guides/usage.md` に「run・count-tokens が
  `<userConfigDir>/usage.jsonl` に計測 record を残す」旨を
  2-3 行で記載（ADR-0025 参照）

## Validation

- 全 relative link の存在確認（スクリプトまたは rg で検査）
- 記載 command: `cheaper run|bulk-read|code-write|doc-write|
  count-tokens|auth status|auth login|auth logout`・options
  （`--input/--worker-file/--cwd/--timeout/--method/--type/
  --format/--quiet`）が args.ts/HELP と一致
- worker 名: bulk-reader/code-writer/doc-writer + alias
- config path: `PI_CODING_AGENT_DIR`・`CHEAPER_CONFIG_DIR`・
  `XDG_CONFIG_HOME`・`~/.config/cheaper`・`<cwd>/.cheaper`
- env vars: CHEAPER_CONFIG_DIR・CHEAPER_COUNT_MAX_BYTES・
  CHEAPER_READ_TOKEN_THRESHOLD・PI_CODING_AGENT_DIR・
  CHEAPER_E2E_OLLAMA_MODEL（検証系は CONTRIBUTING 側）
- `pnpm lint`（markdown は対象外だが repo の check 全体は通す）
- `audit_docs.js --type all` no findings（新規 doc 系 artifact
  の front matter 整合含む）

## Risks

- guide の内容が実装から drift する: 数値・コマンドは検証時に
  実コードと突合する（Phase 5 の検証を task に明記）。
- providers.md から移した実測知識の参照切れ: 移動先 link を
  index/CONTRIBUTING に残す。
- README 短縮で利用者が「インストール方法」を探す: issue が
  installation method を扱わないと明言しているため、README の
  status/development で「配布形態は未決定」旨を自然に触れる
  （仮決めはしない）。

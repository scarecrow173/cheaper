---
id: "IMPL-0018"
type: "impl"
status: "completed"
title: "User-facing documentation information architecture (PLAN-0023)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/README.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "CONTRIBUTING.md"
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "docs/guides/providers.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0112-create-usage-configuration-guides-and-contributing.md"
    - "docs/tasks/0113-create-workers-integration-guides-and-reorg-providers.md"
    - "docs/tasks/0114-restructure-readme-docs-index-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0023-user-facing-docs-information-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
    - "docs/designs/0028-user-facing-docs-information-architecture.md"
    - "docs/adr/0026-readme-as-user-entrypoint-with-canonical-guides.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0023-user-facing-docs-information-architecture.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# User-facing documentation information architecture (PLAN-0023)

## Summary

README を user-facing entrypoint に再構成し、利用・設定・provider・
worker・parent-agent 連携の 5 guide・CONTRIBUTING.md・docs index
を整備した。Issue #15（TASK-0112/0113/0114、PLAN-0023 /
SPEC-0023 / ADR-0026 / DESIGN-0028）に従う。コード変更は無く、
全てドキュメント再構成。

## Context

旧 README は 68 行で目的・repository setup・source tree 実行・
`.apm` 内部構造・smoke 履歴が混在し、利用者の入口として
機能していなかった。SPEC-0023/ADR-0026 に従い、利用者向けの
入口は README に限定し、contract の canonical source は目的別
guide に分離、開発者向け記録は CONTRIBUTING/docs index に
寄せた。

## Implementation

### Changes

- `README.md`: issue の 11 項目構成（header/why/capabilities/
  how-it-works/basic-usage/workers/config/integration/docs/
  development/status）で再構成。repository build・bundle・
  `.apm` 内部構造・source tree 直接実行・smoke 履歴を
  main flow から外した
- `docs/README.md`（新規）: user guides・contributor・
  architecture/decision records の 3 分類 index。各 directory
  に「何の記録か」1 文を付与
- `docs/guides/usage.md`（新規）: CLI contract の canonical
  guide。command・option・stdin/stdout・envelope・exit code・
  count-tokens・auth・usage.jsonl 計測 log
- `docs/guides/configuration.md`（新規）: config dir 解決・
  優先度・`models.yaml`・`policy.yaml`（built-in 実値）・
  env var の canonical guide
- `docs/guides/workers.md`（新規）: worker の概念・built-in
  3 worker・input schema 要点・custom worker・policy 関係
- `docs/guides/parent-agent-integration.md`（新規）: 役割分担・
  skills/hooks・install・target 別挙動・閾値・連携確認
- `docs/guides/providers.md`: user-facing に再編。docker
  compose・NVIDIA toolkit 手順は CONTRIBUTING へ移し、EXP-*
  実測メモ・repair 内部実装言及を削減
- `CONTRIBUTING.md`（新規）: 人間向け entrypoint。canonical
  setup/check・test・実 provider 検証（Ollama container）・
  repository 構造・architecture docs・AGENTS.md 導線

### Decisions During Implementation

- **記載は全て実コード突合**: command・option・exit code・
  config path・env var・policy 実値・worker schema・hooks/
  skill 名・threshold を args.ts・cli.ts・dirs.ts・policy.yaml・
  worker 定義・hooks.json・pre-read-code.js と突合して記載。
  例文の決定内容も repository 内部語を避けた中立例に変更。
- **tokenizer 例は実 shape に**: `tokenizer` は object
  （`{name:"o200k_base",implementation,version}`）なので、
  count-tokens の例は実 shape に合わせた。
- **providers.md の分離先**: smoke 手順・NVIDIA toolkit 注記は
  CONTRIBUTING.md の「実 provider 検証」節へ（開発者向け
  手順として）。EXP-* の実測メモは削るか 1 行参照に圧縮し、
  `repairAttempts` の助言は user 影響があるため残した。
- **installation method は未決定のまま**: README の status で
  「experimental・contract 変更可能性」を自然に触れ、install
  手順・license は記載しない（issue の out-of-scope）。

## Verification

- `audit_docs.js --type all`: 211 files、no findings
- link 検証: README・docs/README・CONTRIBUTING・5 guide の
  relative link を機械検証、全て実在
- contract 突合: 全記載 command・option・worker・path・env
  var が実装と一致
- `mise run check` exit 0（lint warning 4・typecheck・
  668 tests・real-provider e2e 3 件 live・freshness clean）

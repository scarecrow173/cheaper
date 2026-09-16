---
id: "ADR-0004"
type: "adr"
status: "accepted"
title: "one-shot JSON-first CLI プロセスモデル"
created: "2026-09-15"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
  implemented-by: []
  depends-on:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0002-worker-yaml-frontmatter-markdown.md"
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0001-cheaper-v1-pi-sdk.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 4. one-shot JSON-first CLI プロセスモデル

## Context and Problem Statement

親 Coding Agent から cheaper を呼び出す形態を決める必要がある。V1 では request→実行→structured result→exit の単発処理のみが要件（§4–§6, §32–§34）。

## Decision Drivers

* §4: V1 は one-shot process、daemon/server 化しない
* §5: JSON-first CLI、親 Agent が認識するのは原則 CLI contract のみ
* §2.1: 親 Agent 非依存

## Considered Options

* (1) daemon / REST server として常駐させる
* (2) MCP server として公開する
* (3) one-shot JSON-first CLI

## Decision Outcome

Chosen option: "(3) one-shot JSON-first CLI", because 親 Agent 非依存・プロセス管理不要・stdout/stderr の contract が単純で、V1 要件に過剰なインフラを持ち込まないから。

### Consequences

* Good, because 起動・終了・exit code の契約が明確でテスト容易。
* Good, because stdin/stdout の JSON であらゆる親 Agent から呼べる。
* Bad, because 呼び出しごとのプロセス起動コスト・モデルカタログ初期化コストがかかる。

### Confirmation

* `cheaper run` の stdout が result envelope のみ（Pi イベントやログが混入しない）であることを統合テストで確認。
* exit code が §33 の分類表に従うことを確認。

## Implementation Plan

* Affected paths: `packages/cli/`（予定 path、ADR-0008）— command 定義、result envelope、exit code マッピング
* Patterns to follow: task data → JSON、runtime invocation control → CLI、worker behavior → Worker Definition、global execution policy → Runtime Policy（§5）
* Patterns to avoid: daemon/REST/MCP server、TTY 検出による自動フォーマット切替、stdout へのログ混入
* Dependencies or configuration: なし

確定内容:

* command: `cheaper run <worker>`、エイリアス `cheaper bulk-read` / `cheaper cheap-write`
* task input は JSON（stdin または `--input <file|->`）。共通制御: `--cwd`, `--timeout`, `--format json|text`, `--quiet`, `--help`, `--version`
* stdout は最終 result envelope のみ（`{schemaVersion:1, ok, worker, result, meta}` 概念）、デフォルト JSON。`--format text` は人間向け表示。TTY 検出による自動変更なし
* stderr は warnings/diagnostics/runtime errors/optional progress
* exit code: 0 success / 2 invalid request / 3 configuration error / 4 policy・filesystem violation / 5 provider・model failure / 6 worker・tool execution failure / 7 timeout / 8 aborted / 9 partial mutation / 70 internal error（§33）
* abort（SIGINT/SIGTERM/timeout）: Pi/model request を abort し、rollback はしない（§34）

## Verification

* [ ] stdout に result envelope 以外が出力されない統合テスト
* [ ] 各 error code が正しい exit code にマップされるテスト
* [ ] `--format` 未指定で JSON、TTY であっても自動切替しないテスト
* [ ] SIGINT/timeout 時に `session.abort()` 相当が呼ばれ exit 8/7 となるテスト

## Pros and Cons

### Option 1: daemon / REST server

* Good, because 起動コストを amortize でき、session 再利用も可能。
* Bad, because §4/§39 が daemon・REST を non-goal と明記。プロセス管理・ポート・ライフサイクルが不要な複雑さ。

### Option 2: MCP server

* Good, because MCP 対応 Agent から直接呼べる。
* Bad, because MCP server 化は §39 non-goal。親 Agent 固有プロトコルへの依存は §2.1 にも反する。

### Option 3: one-shot CLI

* Good, because 最小のインフラ・明確な contract・全親 Agent 共通。
* Bad, because 毎回の起動コスト。

## More Information

* 引継ぎ §4–§6, §32–§34

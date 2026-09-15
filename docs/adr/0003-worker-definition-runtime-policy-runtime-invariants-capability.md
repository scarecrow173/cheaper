---
id: "ADR-0003"
type: "adr"
status: "proposed"
title: "Worker Definition / Runtime Policy / Runtime Invariants の三層 capability モデル"
created: "2026-09-15"
updated: "2026-09-15"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0002-worker-yaml-frontmatter-markdown.md"
    - "0005-model-tier-model-resolver.md"
    - "0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "0004-one-shot-json-first-cli.md"
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

# 3. Worker Definition / Runtime Policy / Runtime Invariants の三層 capability モデル

## Context and Problem Statement

Worker が要求する capability（tools・model tier）と、cheaper 全体が許可する上限、そして誰も変えられない絶対制約をどう分離・交差判定するかを決める必要がある（§2.4, §2.5, §11, §20–§31）。

## Decision Drivers

* §2.4: Worker Definition（何をしたいか）と Runtime Policy（どこまで許すか）の分離
* §2.5/§31: Runtime Policy 自体も変更できない Runtime Invariants の存在
* §11: Worker の `tools` は permission grant ではなく capability 要求。silent に削減しない
* §43: fail closed / no silent fallback / no silent privilege reduction/expansion

## Considered Options

* (1) Worker 単位の policy（Worker Definition が自身の制限を持つ）
* (2) 単一の flat config に全制約を集約する
* (3) 三層モデル: Worker Definition ∩ Runtime Policy（∩ CLI restrictions）、Runtime Invariants で下限保証

## Decision Outcome

Chosen option: "(3) 三層 capability モデル", because 責務（WHAT / HOW MUCH / MUST NEVER）を層で分離でき、Worker は環境非依存・policy は運用者調整可能・invariants はコードで保証されるから。

### Consequences

* Good, because Worker Definition が実行環境の制約に依存せず配布可能。
* Good, because Runtime Invariants が policy/CLI/Worker のいずれからも緩められない。
* Bad, because 三層の merge・交差・優先順位の実装が単純な flat config より複雑。

### Confirmation

* §23 の validation 順序（built-in complete → user partial → merge → project partial → merge → effective complete）と §30 の CLI narrow-only がテストで確認されること。
* requested tool が policy allowed に1つでも欠ける場合に `POLICY_VIOLATION` で全体拒否されること（部分実行しない）。

## Implementation Plan

* Affected paths: `packages/core/`（予定 path）— Worker 要求 capability 表現、Runtime Policy parser/validator/merger、Runtime Invariants 実装
* Patterns to follow: `Worker requested ∩ Policy allowed = effective`、`version: 1` 必須
* Patterns to avoid: Worker 個別 policy、silent clamp、silent fallback、unknown field の黙認
* Dependencies or configuration: なし（コアロジックは Pi 非依存）

確定内容:

* 三層: Worker Definition（capability 要求）→ Runtime Policy（built-in `<package>/policy.yaml`、user `<user-config>/cheaper/policy.yaml`、project `<cwd>/.cheaper/policy.yaml`、CLI restrictions）→ Runtime Invariants（コード固定）
* merge: object は deep merge、scalar は後段 replace、array は union せず全体 replace（§22）
* CLI は Runtime Policy を狭めることのみ可能。expansion は `POLICY_VIOLATION`、silent clamp しない（§30）
* 各 policy file に `version: 1` 必須（§21）
* Runtime Invariants（§31）: workspace root = canonicalized cwd、filesystem access は workspace 内のみ、symlink escape 禁止、unknown tool 禁止、shell なし、network tool なし、Worker output は宣言 schema 必須、Worker/CLI は Runtime Policy を昇格できない
* §43 実装原則: fail closed、no silent fallback、no silent privilege reduction、no silent privilege expansion、single source of truth

## Verification

* [ ] merge 規則（deep merge / scalar replace / array replace）の単体テスト
* [ ] invalid user/project policy が `INVALID_RUNTIME_POLICY` で失敗し silent ignore されないテスト
* [ ] CLI narrow 受理・expand 拒否（`POLICY_VIOLATION` exit 4）のテスト
* [ ] requested tool ∩ policy allowed の交差判定で1つでも欠ければ Worker 全体拒否のテスト
* [ ] Runtime Invariants が policy/CLI/Worker から変更不可能であることのコードレビュー

## Pros and Cons

### Option 1: Worker 単位 policy

* Good, because Worker ごとに細かい制御ができる。
* Bad, because §20「Worker 個別 policy は V1 では存在しない」に反し、権限昇格経路が増える。

### Option 2: 単一 flat config

* Good, because 実装が単純。
* Bad, because Worker の意図・運用調整・絶対制約が混ざり、no silent privilege reduction/expansion を保証しにくい。

### Option 3: 三層モデル

* Good, because 責務分離が §45 のアーキテクチャ境界と一致する。
* Bad, because 実装・テスト項目が増える。

## More Information

* 引継ぎ §2.4, §2.5, §11, §20–§31, §43, §45

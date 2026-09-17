---
id: "ADR-0026"
type: "adr"
status: "accepted"
title: "README は user-facing entrypoint に限定し、canonical source は目的別 guide に分離する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0023-user-facing-docs-information-architecture.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/0028-user-facing-docs-information-architecture.md"
  depends-on:
    - "./0020-canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
    - "./0019-separate-dev-apm-project-via-nested-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0023-user-facing-docs-information-architecture.md"
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

# 26. README は user-facing entrypoint に限定し、canonical source は目的別 guide に分離する

## Context and Problem Statement

SPEC-0023 は README・guides・CONTRIBUTING・docs index の責務分離
を要求する。決めるべきこと: (a) README の情報粒度、(b) user
guide の分割軸と canonical source の置き方、(c) contributor
entrypoint の形態、(d) providers.md に混在する開発知識の移動先、
(e) docs index の分類方法。

## Decision Drivers

- Issue #15: 「README を user-facing entrypoint とし、利用・設定・
  integration・開発・設計記録を適切な文書へ分離して導線を作る」
  と 11 項目の README 構成・guide 一覧・docs index 分類を具体的に
  指定する。
- 「同じ canonical information を複数文書へ詳細に複製しない」
  （one canonical source per contract）が設計原則 3 に明示される。
- `AGENTS.md` は Coding Agent instruction であり、人間向け
  contributor guide の代用にしない（issue の問題点 5）。
- license・正式 installation method は未決定で仮決めしない
  （issue の「今回扱わないこと」）。

## Considered Options

### README の粒度

- **A: user-facing entrypoint に限定**（採用 — issue 指定）。
  目的・why・capability・conceptual flow・最小 usage・worker
  一覧・config 概要・integration 概要・docs link・development
  gateway・status の 11 項目。build/bundle/内部 path は外す。
- **B: 現行 + 軽微な整理**。混在問題が解消しない。

### user guide の分割軸

- **i: 目的別 5 guide**（採用 — issue 指定）: usage・
  configuration・providers・workers・parent-agent-integration。
  各 contract の canonical source を 1 文書に固定する。
- **ii: 1 本の大きな user manual**。参照性が低く progressive
  disclosure に反する。

### contributor entrypoint

- **α: `CONTRIBUTING.md`（repo root）**（採用 — issue 指定）。
  OSS 慣例の場所で、setup contract（`mise run setup`/`check`、
  ADR-0020）・test・構造・architecture docs・AGENTS.md への
  導線を持つ。AGENTS.md の複製にしない。
- **β: README の Development 節に詳細を残す**。README を再び
  contributor 向けに戻してしまう。
- **γ: `docs/guides/development.md`**。root の CONTRIBUTING.md
  ほど発見されにくい。issue も CONTRIBUTING.md を指定。

### providers.md の開発知識の移動先

- **A: `docs/discovery/` に実測知識を移す**。EXP-* メモ・
  repair 実装知見・smoke 手順は性質上 discovery/development
  記録（採用 — 新規 `discovery` doc ではなく既存 EXP/IMPL 記録
  への参照 + `CONTRIBUTING.md` の smoke/検証節へ移す）。
- **B: そのまま残す**。user guide が内部知識で肥大化したまま。

### docs index の分類

- **a: 3 分類**（採用 — issue 指定）: user guides /
  contributor・development / architecture・decision records。
  `adr/designs/plans/tasks/impl/specs/discovery/test-specs/
  guides/handoff` を「何のための記録か」1 文ずつ説明する。
- **b: directory 一覧のみ**。issue が明示的に否定。

## Decision Outcome

- README は issue の 11 項目構成で再構成し、repository build・
  bundle 生成・`.apm` 内部構造・source tree 上の直接実行・
  smoke 履歴を main flow から外す。
- canonical source 対応表: CLI contract→`guides/usage.md`、
  config→`guides/configuration.md`、provider/auth→
  `guides/providers.md`、worker→`guides/workers.md`、
  integration→`guides/parent-agent-integration.md`、repo 開発→
  `CONTRIBUTING.md`、記録索引→`docs/README.md`。
- `providers.md` から repository smoke 手順（Docker/NVIDIA
  Toolkit 注記）・EXP-* 実測メモ・repair 内部実装知見を外し、
  CONTRIBUTING.md（検証手順）と既存 EXP/IMPL 記録への参照へ
  移す。user-facing（設定ファイル場所・auth 手順・tier 割当・
  models.json/models.yaml 例）は残す。
- README の usage 例は `cheaper <command>` の public CLI contract
  （bundle 経由で consumer が得る形）を使い、source path 実行は
  書かない。
- `docs/README.md` を新設し 3 分類で index 化する。

## Consequences

- 利用者は README → 目的別 guide の 2 hop で全 public contract に
  到達できる。contributor は README → CONTRIBUTING.md → 各記録へ。
- contract 変更時の更新先が 1 文書に固定される（表現ドリフト防止
  — AGENTS.md 開発原則と整合）。
- guide が増えるため docs/README.md の index が必須になる。
- providers.md の開発知識分離で、実験知識の置き場が discovery
  系へ移る — 将来の類似メモも同じ分類に従う。

## Confirmation

- README・docs index の全 relative link が実在 file を指す。
- README・guide のコマンド・worker 名・config path・env var が
  実装（args.ts・dirs.ts・policy.yaml・worker 定義・hooks.json）
  と一致する。
- README main flow に build/bundle/内部 path が無い。

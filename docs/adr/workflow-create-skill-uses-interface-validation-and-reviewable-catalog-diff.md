---
id: "ADR-034Zub9WUEyiDT9ATk2WAg"
type: "adr"
status: "accepted"
title: "workflow create skill uses interface validation and reviewable catalog diff"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/204"
    - "https://github.com/scarecrow173/cheaper/issues/200"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/workflow-create-skill-issue-204.md"
    - "../specs/workflow-catalog-issue-201.md"
    - "workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-create-skill-issue-204.md"
    - "../specs/workflow-create-skill-issue-204.md"
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

# workflow create skill uses interface validation and reviewable catalog diff

## Context and Problem Statement

`cheaper-workflow-create` skill が Workflow JSON v1 の検証と catalog
登録をどの手段で行うかが設計の分岐点。検証を skill 内の独自 schema
や手動チェックにすると現行 compiler(`WORKFLOW_JSON_SCHEMA`、Rust
literal で外部 export なし)と drift する。登録を新しい書込み CLI に
すると #201 が deferred した catalog write CLI の設計を先取りし、
issue #204 の非目標(CLI generator 本体)に踏み込む。

## Considered Options

* skill は検証を `cheaper workflow interface <path>` に委譲し、
  catalog 登録は agent が manifest を編集した reviewable な差分とし、
  `cheaper workflow list` で登録状態を確認する。定義 file の保存と
  実行は別で、`workflow run` や grant/policy/Route の変更は行わない。
* `cheaper workflow create`/`register` の書込み CLI を新設し、
  schema 強制と catalog 追記を code path にする。
* skill が standalone JSON schema file を配布し、agent が生成物を
  schema 検証してから保存する。

## Decision Outcome

Chosen option: "interface 検証 + reviewable catalog diff"。schema
正本は Rust 実装にのみ存在し、`workflow interface` はその compiler
の definition validation を通す唯一の公開経路(実測で valid fixture
accept・invalid fail closed を確認)。独自 schema file は drift の
温床になる。catalog は #201 の ADR で発見用 index と決定済みで、
登録は単なる manifest file edit — 差分が小さく人が review でき、
重複 id や missing path は `workflow list` の diagnostics で機械的に
検出できるため、書込み CLI の追加価値が現時点で薄い。

## Implementation Plan

* Affected paths:
  `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md`(新規)、
  `docs/guides/workflow-usage.md`・
  `docs/guides/parent-agent-integration.md`(導線の最小変更)。
* Constraints: Rust/`crates/`・hook 実装には触れない。skill 本文の
  例は `workflow interface` が accept する形のみ。
  `--input '{...}'` inline JSON・架空 job/tool/worker・string
  interpolation を記述しない。catalog manifest の bound(64 KiB・
  256 entry・field cap)と entry schema は #201 契約に従う。

## Verification

* [ ] skill 内 template が `cheaper workflow interface` を pass
  し、invalid case が `INVALID_REQUEST`/`INVALID_INPUT` + exit 2 で
  fail する(実実行で裏付け)。
* [ ] catalog 登録 diff が `cheaper workflow list` の
  `status:"ok"`/`diagnostics` で確認できる。
* [ ] `doc_status audit --blocking` が 0、`mise run check` が
  exit 0。

---
id: "TSPEC-034zQvKPjNkXAEgbZlzdcQ"
type: "test-spec"
status: "proposed"
title: "Workflow compiler の契約保証"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/workflow-contract-compiler.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-contract-compiler.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow compiler の契約保証

## Purpose

public interface を関数として再利用できることと、未検証 job input を実行に渡さない境界を維持する。D5 を supersede する仕様が成立した場合のみ対応する保証を廃止できる。

## Feature

Workflow JSON v1 の定義を、default 適用済みの決定的 DAG と最終検証 hooks に変換する。

## Rules

- Rule: default/requiredness は input root default の有無で決定し、入力 mutation と invocation 間の object 共有をしない。unknown/invalid/missing は拒否。
- Rule: binding は exact keys。literal 配下と挿入値は再解釈せず、result-root JSON Pointer は own key のみ。null と missing を区別。
- Rule: needs/from/resource edges は dedupe、unknown/self/cycle は拒否、code-unit 順で決定的。
- Rule: catalog にない Worker/tool と Worker.tools capability は定義段階で拒否。core schema compile error の cause を保ち INVALID_REQUEST に wrap。
- Rule: from 無し input は compile 時に full schema 検査。from 有りは構造のみで compile、materialization hook は必ず Worker/tool 検証。
- Rule: output は全 job terminal 後のみ。単一 hook は独立 output を保持でき、batch は全出力成功が必要。
- Rule: resources 省略と {} を保持して fallback、空配列も port 必須。port throw は成功扱いせず、全 job manifest を保持し欠落を拒否。
- Rule: projection は public input schema/default/requiredness/output schema のみ。invocation と resource port/handler/Worker/process を実行しない。

## Examples

- Example: boolean/number/null/object default の $input はその JSON 型のまま渡り、nested default は補完しない。
- Example: $literal 内の偽 $from と input default 内の偽 $from は依存を生まず、公開 compiled field を変えても hook の元 snapshot は変わらない。
- Example: 空 pointer は result 全体、/details/a~1b/~0 は escape key、/items/0 は array 要素。01/-/missing/prototype key は失敗。
- Example: needs と同じ from は一辺。逆順定義でも order と JSON 表現は同じ。integer-like id は order が code-unit 順。
- Example: producer result が Worker schema と不一致なら job hook は INVALID_INPUT、output の未完了/欠損/schema 不一致は INVALID_OUTPUT。
- Example: failed job の result が無くても参照しない単一 output は成功し、失敗 output と batch は拒否。
- Example: resources:{reads:[]} で port が無い compile は拒否。async port の全 manifest と inferred edges は compiled に保持。

## Guarantees

SPEC AC1–AC7、DESIGN の metadata/snapshot/manifest 義務を保つ。guard は依存方向、compact fixture/guidance は parent-agent contract を保証する。

## Non-goals

engine/provider 実行、policy の許可拡大、real-path/resource 推論実装、Artifact Store、CLI は検証しない。

## Risk

non-scan の喪失は意図しない依存、snapshot の喪失は検証迂回、manifest 欠落の成功扱いは resource safety の誤認を起こす。

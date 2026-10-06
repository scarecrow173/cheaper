---
id: "IDEA-034zIHiBntz3Z0WJ9PdgNU"
type: "idea"
status: "promoted"
title: "再利用可能な Workflow contract を検証済み DAG へ compile する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/64"]
  derived-by: ["../discovery/workflow-compiler-boundaries.md"]
---

# 再利用可能な Workflow contract を検証済み DAG へ compile する

## Summary

親 agent が public inputs と outputs だけを扱える型付き Workflow と、実行前に private DAG を検査する compiler を追加する。

## Problem and Motivation

複数 Worker/tool の連鎖を毎回親 agent が管理すると中間値と依存関係の再送が増える。保存可能な contract に defaults と構造 binding を持たせ、入力の型と依存関係を一度検査する。

## Expected Value

#62 engine と #70 interface が同一の定義検証を使い、private job 情報を公開せずに再利用できる。

## Open Questions

未生成の `$from` 値と #65-B resource 推論を、未実装を成功扱いせずどう境界化するか。

## Next Action

- [x] discovery で境界を整理し spec へ昇格する。

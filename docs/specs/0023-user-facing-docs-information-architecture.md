---
id: "SPEC-0023"
type: "spec"
status: "approved"
title: "User-facing documentation information architecture"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/15"
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
    - "./0016-separate-dev-apm-project-from-distributable-package.md"
    - "./0017-repo-owned-setup-contract-for-fresh-checkouts.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0022-delegation-usage-measurement-log.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# User-facing documentation information architecture

## What Should Be Built

README を「初めて `cheaper` を知った利用者が短時間で目的・機能・
仕組み・基本用法・次に読む文書を理解できる入口」に限定して再構成
し、利用・設定・integration の canonical user guide、人間向け
contributor entrypoint（`CONTRIBUTING.md`）、docs 全体の index
（`docs/README.md`）を整備する。

## Why Now

現行 README は目的説明の直後に repository 開発手順・配布内部構造
（`mise`/`pnpm -r build`/source tree 上の CLI 実行/`.apm` layout/
bundle 生成）を並べており、利用者と contributor の情報が混在して
いる（Issue #15）。#9（dev/distribution 分離）・#7（canonical
setup contract）が完了し、path・command が確定したため IA を
最終化できる。Issue 指定の順序（#9 → #7 → 本 issue）どおり。

## Users and Value

- 新規利用者: README だけで cheaper の価値と基本モデルを理解し、
  目的別 guide へ迷わず移動できる。
- consumer（parent agent 利用者）: 設定・integration・worker の
  canonical な参照先が 1 箇所ずつある。
- contributor: 人間向け entrypoint から setup・test・構造・
  architecture 記録へ辿れる。
- 開発者: 情報の canonical source が 1 箇所になり、変更時の
  更新対象が明確になる。

## Scope

### In Scope

- `README.md` の user-facing 再構成（issue の 11 項目方針に準拠）
- `docs/README.md` 新設（user guides / contributor・development /
  architecture・decision records の 3 分類 index）
- `docs/guides/usage.md` 新設（CLI / basic usage の canonical
  guide）
- `docs/guides/configuration.md` 新設（models.yaml・policy.yaml・
  discovery・precedence・環境変数の canonical guide）
- `docs/guides/workers.md` 新設（worker 概念・built-in・schema・
  custom definition の canonical guide）
- `docs/guides/parent-agent-integration.md` 新設（skills/hooks・
  target 別挙動・確認方法の canonical guide）
- `docs/guides/providers.md` の再編（user-facing 情報を残し、
  repository smoke・実験メモ・内部実装知識を分離）
- `CONTRIBUTING.md` 新設（人間向け contributor entrypoint）
- 全 relative link・コマンド例・config path の実装一致検証

### Out of Scope

- license の選定・`LICENSE` の追加
- 正式な installation method（npm/binary/GitHub Release）の決定・
  記載 — 未定のため仮決めしない
- `AGENTS.md` の本質的な再構成（Coding Agent instruction として
  維持。人間向け guide の代用にしない）
- usage 集計コマンド等新機能のドキュメント（機能自体が未実装）

## Requirements

1. **README の audience**: primary audience は cheaper の利用者。
   冒頭だけで目的・delegation model・core capability が分かる
   構成にする。repository build・bundle 生成・source tree 上の
   直接実行（`node packages/cli/bin/...`）・`.apm` 内部構造・
   smoke 履歴を main flow に置かない。
2. **progressive disclosure**: README から directory 名ではなく
   目的別文書へリンクする（usage / configuration / providers /
   workers / integration / CONTRIBUTING / docs index）。
3. **canonical source**: 同一 contract を複数文書へ詳細に複製
   しない。CLI contract は `guides/usage.md`、config は
   `guides/configuration.md`、provider は `guides/providers.md`、
   worker は `guides/workers.md`、integration は
   `guides/parent-agent-integration.md`、repo 開発は
   `CONTRIBUTING.md` を canonical とし、README は概要 + link。
4. **implementation-verified**: 記載する command・option・worker
   名・config path・env var は実コード（cli.ts・args.ts・
   dirs.ts・policy.yaml・worker definitions・hooks.json）と
   一致させる。古い spec 記述より実装を優先する。
5. **link integrity**: README・docs index の全 relative link が
   存在し、broken link がないこと。
6. **CONTRIBUTING.md**: 人間向け entrypoint として setup
   （`mise run setup`/`mise run check`）・test・構造・
   architecture docs・AGENTS.md への導線を持つ。AGENTS.md の
   複製にしない。
7. **docs/README.md**: `adr/designs/plans/tasks/impl/specs/
   discovery/test-specs/guides/handoff` を「何のための記録か」
   1 文ずつ説明する index にする。
8. **providers.md 分離**: user-facing（provider 設定・認証・
   model 選択）は残し、repository smoke 手順・EXP-* 実測メモ・
   repair 実装知見・test-only tuning は discovery/development
   文書側へ移す。

## Acceptance Criteria

- [ ] README が issue の 11 項目構成（header/why/capabilities/
      how it works/basic usage/workers/config overview/
      integration/docs/development/status）に準拠している
- [ ] README に repository build/bundle 内部手順が無い
- [ ] 5 本の user guide（usage・configuration・providers・
      workers・parent-agent-integration）が存在する
- [ ] `docs/README.md`・`CONTRIBUTING.md` が存在する
- [ ] README・index の全 relative link が有効
- [ ] 記載コマンド・path・worker 名・env var が実装と一致する
- [ ] license・installation method を仮決めしていない

## Sources and Relations

- Issue #15: 要件・構成方針・受け入れ条件の起点。
- SPEC-0016: dev/distribution 分離（README の配布説明の前提）。
- SPEC-0017: canonical setup contract（CONTRIBUTING.md が参照）。
- SPEC-0022: usage.jsonl（configuration/usage guide で言及する
  user-facing contract）。
- DISC-0009・ADR-0016: target 別 hook 挙動（integration guide の
  正確な記述根拠）。

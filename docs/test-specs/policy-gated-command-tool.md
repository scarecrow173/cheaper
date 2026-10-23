---
id: "TSPEC-034EOCRqkrPUFcQLHVN5I3"
type: "test-spec"
status: "approved"
title: "policy-gated command tool の信頼境界保証"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
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
  related:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "../designs/documentation-honesty-sweep-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/policy-gated-command-tool.md"
    - "docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md"
  derived-by: []
  verifies:
    - "docs/specs/policy-gated-command-tool.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# policy-gated command tool の信頼境界保証

## Purpose

`command` tool の認可境界が「trusted layer（builtin + user-scope policy）が authoring した nonempty grants」という一点に集約されることを保証する。`command` は動的 argv を持つ唯一の managed subprocess tool であり、ここの fail-open は任意 executable 起動を意味する。`command` tool が廃止されるか、信頼境界の設計（Option A）が改訂されたときにのみ退役可能。

## Feature

Feature: trusted-authored grants だけが executable を authorize し、それ以外の全経路は fail-closed で拒否される。

## Rules

- Rule: project layer の policy は `command` section を受け付けない（`PolicySchemaMode::ProjectPartial` で schema 拒否、`INVALID_RUNTIME_POLICY`）。user partial・complete では受理する。
- Rule: `names` に `Command` を含む tool runtime の生成は grants 非空かつ全 grant の pin 成功を要求し、空 grants・pin 不能・deny 該当・workspace-resident executable は `POLICY_VIOLATION` で fail-closed する。
- Rule: interpreter/shell/exec-wrapper/loader deny set は declared basename・pin 時 resolved basename・spawn 前再検証の 3 適用点で評価される。
  > （issue #169 注記 — #163 で builtin deny set（interpreter/shell/exec-wrapper/loader・native magic・workspace-resident・metachar）は mandatory 層から除去され、#164 で deny は opt-in `security.execDenyPatterns`（既定 `[]`）宣言時のみ deterministic executable deny-list として再導入。fail-closed gate は空 grants・pin 不能では維持、deny 該当は `execDenyPatterns` 宣言時のみ。3 適用点構造（policy resolve・pin・spawn 前再検証）は opt-in deny の適用点として存続）
- Rule: `command` は writeScope と併存できない（scope 生成時 `POLICY_VIOLATION`）。project は `tools.allow` で exposure を toggle できるが grants を authoring することはできない（exposure ≠ authorization）。
- Rule: model 入力は `{name, args}` のみで、`executable`/`cwd`/`env` 等の余剰 key や非 string は schema 層で `INVALID_INPUT` に落ちる。executable/cwd/env は model 入力経路に乗らない。
- Rule: invocation と worker scope の `command` policy は declared-shape の `PartialEq` parity で一致し、scope は invocation-resolved grants を受け取る。

## Examples

- Example: project policy に `command: {grants: {build: ...}}` を書くと load が `INVALID_RUNTIME_POLICY` で拒否される。user-scope `policy.yaml` の同一記述は受理される。
- Example: grants 空の `command` runtime は `POLICY_VIOLATION "command exposure requires nonempty authorized grants"`。`names` に `Command` を含まない runtime は dormant として許容される。
- Example: grant `node` は bare name でも interpreter deny で `POLICY_VIOLATION`。pin 済み path が deny basename に差し替わると spawn 前再検証で拒否される。
  > （issue #169 注記 — #163 で interpreter/bare-`node` deny は mandatory 層から除去。bare name `node` は既定では PATH 解決され受理される。deny basename 該当・spawn 前再検証による拒否は opt-in `security.execDenyPatterns` 宣言時のみ（#164））
- Example: `{name: "build", args: [...], cwd: "."}` は `INVALID_INPUT`。`{name: "build", args: ["--unknown-flag"]}` は argv pattern 不一致で `POLICY_VIOLATION`。

## Guarantees

- project → trusted の権限昇格経路（schema・merge・normalize のいずれ経由でも）が存在しない。
- grants が空・pin 失敗・deny 該当のとき、実行経路に到達する前に registration gate が閉じる。
  > （issue #169 注記 — 空 grants・pin 失敗は現行でも fail-closed。deny 該当による閉鎖は opt-in `security.execDenyPatterns` 宣言時のみ（#163/#164 — builtin deny set 除去・opt-in deny-list 再導入））
- `process` tool の contract・error shape・details field は変わらない。
- `command` 使用 worker は `savings.directlyComparable == false` で、`meta.changes` は実測 journal のみを報告する。

## Non-goals

- OS sandbox による子 process の containment、PATH 解決の provenance 検証、deny list の網羅性。
- workflow `kind: tool` job の resource inference 詳細（generic tool 分岐の保守的 manifest で担保）。
- interpreter opt-in knob・project 側 narrowing field（deferred design concern）。

## Risk

- これらの保証が落ちると、workspace 内 policy file を編集できる actor（または model 出力）が任意 executable を起動でき、secret 持ち環境での情報窃取・破壊的操作が可能になる。

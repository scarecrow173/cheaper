---
id: "TSPEC-034Jg5JHetTGqIaDM2E2cr"
type: "test-spec"
status: "draft"
title: "Route declaration loading and argv resolution"
created: "2026-10-20"
updated: "2026-10-20"
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
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route declaration loading and argv resolution

## Purpose

route 宣言の schema・3層 load・argv 解決が spec R1-R17 の contract を
満たすことを pin し、hook/run/CLI 3 caller が同一 resolve を共有する
ことを保証する。

## Feature

route.yaml 宣言の load・match・viability 評価。

## Rules

- routes.yaml schema は route id・match.command・target{kind,path,args}・
  passthroughEnv・trusted を検証し、unknown field を拒否する。
- builtin → user → project の順に後勝ち merge し、project 層
  `trusted: true` のみ下位層同名 route を shadow する(suppressed
  provenance を記録)。
- 同一 effective route で同一長 `match.command` の重複は
  `invalid-route` diagnostic として resolve 対象から除外する。
- `target.path` は宣言層 dir 起点の相対パスのみ許可し、絶対パス・
  `..` traversal・invalid encoding を拒否する。
- argv prefix match は token 列一致で longest match を採用し、
  `args`(match.command 以降)と `exec_args`(argv[1..])を区別する。
- viability は self-bypass → INTERACTIVE_COMMANDS →
  check_command_execution(grant 解決・pin・denied・cwd)の固定順。
- env-prefix token(FOO=1)を含む argv は route 対象外とする。

## Examples

- `match.command: ["git","diff"]` は `git diff --stat` に match し、
  `exec_args = ["diff","--stat"]`、`args = ["--stat"]`。
- 同名 route が user(untrusted)と project(trusted)に存在する場合、
  project 側が採用され user 側は suppressed として記録される。
- project 層 `target.path: "../../etc/x.yaml"` は拒否される。

## Guarantees

- hook eval・`route resolve`・`route run` revalidate が同一
  ResolveInput → 同一 verdict を返す。
- viability の grant check が `command::execute` の受理判定と
  一致する(同一 `check_command_execution` seam)。

## Non-goals

- compound shell(&&/pipe)の routing、worker への再委譲。

## Risk

- schema 誤検証の緩さ・層 merge の誤りは trusted workflow への
  不正 routing を許す。

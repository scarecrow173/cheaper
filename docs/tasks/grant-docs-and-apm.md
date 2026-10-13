---
id: "TASK-034FpqCHFW0zJzTo0Kmv24"
type: "task"
status: "done"
title: "T6: grant-docs-and-apm — guides・README・APM skill・impl record"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/grant-docs-and-apm-skill-updates.md"}
    modified:
      - {"type":"doc","path":"README.md"}
      - {"type":"doc","path":"docs/guides/command.md"}
      - {"type":"doc","path":"docs/guides/workers.md"}
      - {"type":"doc","path":"docs/guides/usage.md"}
      - {"type":"doc","path":"docs/guides/configuration.md"}
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-usage/SKILL.md"}
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"}
      - {"type":"doc","path":"docs/plans/implement-invocation-scoped-command-grants.md"}
      - {"type":"doc","path":"docs/plans/README.md"}
      - {"type":"doc","path":"docs/plans/implement-policy-gated-command-tool.md"}
      - {"type":"doc","path":"docs/plans/implement-tool-operator-built-in-worker.md"}
      - {"type":"doc","path":"docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"}
      - {"type":"doc","path":"docs/designs/policy-gated-command-tool.md"}
      - {"type":"doc","path":"docs/designs/tool-operator-built-in-worker.md"}
      - {"type":"doc","path":"docs/discovery/invocation-scoped-command-grants.md"}
      - {"type":"doc","path":"docs/tasks/README.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  implemented-by:
    - "docs/impl/ir/grant-docs-and-apm-skill-updates.md"
  depends-on:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
    - "docs/tasks/grant-e2e-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/invocation-scoped-command-grants.md"
  references: []
  defers: []
  deferred-by: []
---

# T6: grant-docs-and-apm — guides・README・APM skill・impl record

## Scope

issue #105 の公開面 docs（guides・README・APM skill × 2）と実装完了
時の impl record を整備する（design §(j)・AC11）。記述対象の挙動は
T5 で確定済みであることが前提。docs の記述内容そのものは test-spec
の行為保証対象外（Non-goals — 本 task の doc audit が担う）。

- In: `command.md` の `--grant` contract・cmdline 露出 warning・
  trust boundary、`workers.md` の `grantRequests` output contract、
  `usage.md` の `--grant`・`policy grant`・audit surface、
  `configuration.md`（必要最小限）、README commands 表の `policy`
  行、`cheaper-usage` / `cheaper-tool-operate` SKILL.md の
  grant-request → user approval → `--grant` re-run loop（request
  由来 spec の review 必須・untrusted 明記）と `policy grant` UX・
  write caveat（last-wins・comment 非保存）、`docs/impl/ir/` の impl
  record。最終 gate として全 task green 後の `mise run check`
  （AC12）も本 task で通す。
- Out: `--grant-file`・`process.tasks` 同型機構・project policy
  対象化・`usage.jsonl` record shape 拡張（ADR defer / spec Out of
  Scope）。

### 所有ファイル

- `docs/guides/command.md` — `--grant` contract（repeatable・両受理
  形・64KiB・`name` 内包 JSON・非永続・非 run command reject）・
  cmdline 露出 warning（`policy grant add` の spec-json も同種
  caveat）・trust boundary（model 入力・worker prompt・project
  policy は grant を authoring 不能）。
- `docs/guides/workers.md` — `grantRequests` output contract（
  `blocked`/`partial` で emit・`{executable, argvHint?, name?}`・
  advisory / untrusted）。
- `docs/guides/usage.md` — `--grant`・`policy grant`・audit surface
  （stderr `grant:` 行 + `grants.invocation` span field、
  `usage.jsonl` record shape 不変）。
- `docs/guides/configuration.md` — 必要最小限（user `policy.yaml`
  の `command.grants` 管理経路として `policy grant`）。
- `README.md` — commands 表へ `policy` 行。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`・
  `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` —
  grant-request → user approval → `--grant` re-run loop を loop の
  必須手順（request 由来 spec の review 必須・全 field は untrusted）
  として文書化、`policy grant` UX と write caveat。
- `docs/impl/ir/`（新規 1 file）— 実装完了時の IMPL record
  （task-doc → impl-doc の既存規約に従い scaffold で生成）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T6）
- `docs/designs/invocation-scoped-command-grants.md` — §(j) の docs
  必須記載事項
- `docs/specs/invocation-scoped-command-grants.md` — AC11・AC12
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （docs 記述内容は Non-goals — 行為保証対象外）

## Dependencies

- `docs/tasks/grant-e2e-verification.md`（T5 — docs が記述する挙動の
  確定）

## Steps

1. `command.md` に `--grant` contract・cmdline 露出 warning・trust
   boundary を記述する。
2. `workers.md` に `grantRequests` output contract、`usage.md` に
   `--grant`・`policy grant`・audit surface を記述し、
   `configuration.md` へ必要最小限の追記と README commands 表の
   `policy` 行を追加する。
3. `cheaper-usage` / `cheaper-tool-operate` SKILL.md に grant-request
   → approval → `--grant` re-run loop（review 必須・untrusted 明記）
   と `policy grant` UX・write caveat を記述する。
4. 実装完了時に `docs/impl/ir/` へ impl record を scaffold で生成
   する。
5. doc audit・doc-id check・slim payload check が pass し、全 task
   green を確認した上で `mise run check`（AC12 の canonical
   verification）を通す。

## Done When

- [x] design §(j) の 4 項目（`--grant` contract・`grantRequests` →
  approval → re-run loop・`policy grant` UX + write caveat・cmdline
  露出 warning）が guides・README・両 SKILL.md に記載され、実装挙動
  と突合で矛盾がない（AC11）
- [x] impl record が `docs/impl/ir/` に生成されている
- [x] 全 task の green を確認した上で `mise run check` が pipeline
  全段 exit 0（AC12）

## Verification

実績（2026-10-14）:

- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all` exits 0
- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `node scripts/check-apm-payload.mjs` exits 0
- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js` exits 0
- [x] `grep -rn -- "--grant" docs/guides/command.md docs/guides/usage.md packages/apm/.apm/skills/` returns matches
- [x] `grep -n "policy grant" README.md docs/guides/usage.md` returns matches
- [x] `grep -n "grantRequests" docs/guides/workers.md` returns matches
- [x] `mise run check` exits 0（canonical verification — AC12、全 task 完了後の最終 gate）

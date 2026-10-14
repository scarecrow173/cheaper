---
id: "IMPL-034Fs3qmmPJOmrZe4uVW81"
type: "impl"
status: "completed"
title: "grant docs and APM skill updates"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
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
      - {"type":"doc","path":"docs/tasks/grant-docs-and-apm.md"}
      - {"type":"doc","path":"docs/tasks/README.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/grant-docs-and-apm.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# grant docs and APM skill updates

## Summary

T6。issue #105 の invocation-scoped command grants（`--grant` flag・
`grantRequests` output contract・`cheaper policy grant` command）の
公開面 docs を整備した。design §(j) の 4 項目（`--grant` contract・
grant-request → approval → re-run loop・`policy grant` UX + write
caveat・cmdline 露出 warning）を guides・README・両 APM SKILL.md
へ記載し、T1–T5 で確定した実装挙動と突合済み。

## Context

TASK-034FpqCHFW0zJzTo0Kmv24。issue #105 の第 6 task（AC11）。
T1(fcba487)が合成 invocation layer、T2(bca57e6)が `--grant` flag・
plumbing、T3(bbc5615)が `policy grant` command + core write 基盤、
T4(2be077f)が `grantRequests` output contract、T5(8f84d12)が
dev-seam / regression / parity の結合検証を着地済み。本 task は
公開面 docs・APM skill・status 同期と impl record のみで実装
コードには触れない。

## Implementation

### Changes

- `docs/guides/command.md`: 「gate と信頼境界」の trusted layer
  列挙へ合成 invocation layer（invoker の `--grant`）を追加。
  新規 section `## --grant（invocation-scoped grant）` に contract
  を記述 — repeatable・`name` 内包 JSON object・`--grant <v>` /
  `--grant=<v>` 両形・64KiB 上限・非 run command への
  `InvalidRequest` reject・`workflow run` 同一意味論・policy 由来
  grant と同一検査経路（deny list 迂回不能、`details.layer` =
  `"invocation"`）・非永続（policy file・usage.jsonl・`.cheaper/`
  不変）・collision / intra-invocation duplicate の fail-loud・
  `command` 非宣言 worker の warn-and-continue・audit surface
  （stderr `grant:` 行 + `grants.invocation` span field・
  usage.jsonl shape 不変）。subsection `### cmdline 露出と
  secrets` に flag 値の cmdline 露出（`ps`/`/proc`/shell
  history）・`env_clear` + minimal env で ambient env が子へ
  届かない事実・secret は user policy `env`（0600）か子側
  credential file への配置指針・`policy grant add` の同種
  caveat を明記。末尾に trusted-layer-only の信頼境界が不変
  である旨を補足。
- `docs/guides/workers.md`: `tool-operator` 節に `grantRequests`
  output contract を追加 — `blocked`/`partial` で gated tool /
  grant 欠如由来の場合に optional field（`{executable, argvHint?,
  name?}`）を emit、advisory・全 field は model 生成の untrusted
  値、request → spec 組立 → **re-run 前 review** → user approval →
  `--grant` re-run の loop を必須手順として固定（interactive
  prompt は存在しない・re-run 時の再検査で fail-closed）。
- `docs/guides/usage.md`: コマンド構成 block へ `policy grant`
  3 形態を追加（HELP mirror）。共通 option 表へ `--grant <json>`
  行。stderr diagnostics bullet へ `grant:` 行 +
  `grants.invocation` span field の audit surface（usage.jsonl
  record shape 不変）。新規 section `## policy grant（command
  grant の管理）` に add/list/remove の UX・spec shape（`name`
  key 非含有の map value）・同一検査経路・atomic write（0600）・
  write caveat（last-wins・comment/整形非保存）・cmdline 露出
  caveat・utility envelope 規約を記述。計測 log の記録対象外
  列挙へ `policy` を追加。
- `docs/guides/configuration.md`: `command` section 記述へ
  `cheaper policy grant {add,list,remove}` が user policy の
  grants の managed 経路である旨と `--grant` の run 単位非永続
  経路を必要最小限で追記（caveat 詳細は usage/command ガイド
  への pointer）。
- `README.md`: Basic usage の command 例へ `cheaper policy grant
  list` を追加（commands 列挙面への `policy` 反映）。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`: Commands
  block へ `policy grant` 3 形態、options 表へ `--grant` を追加。
  新規 section `## Command grants` に `--grant` contract
  （run-scoped 非永続・repeatable・collision fail-loud）・
  `policy grant` UX + write caveat（atomic・0600・last-wins・
  comment 非保存）・secrets warning（cmdline 露出・env_clear +
  minimal env → secret は policy `env` / child credential file）・
  grant-request loop の必須 4 手順（read → assemble + review →
  approval → re-run / persist-via-add）を記述。Delegation
  guidance の tool-operate 項へ `grantRequests` → loop への誘導
  を追加。
- `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`:
  `Consuming the result` に `grantRequests[]` field を追加。新規
  section `## Grant-request loop` に blocked/partial 時の必須
  loop（assemble → **review（untrusted 明記）** → approval →
  `--grant` re-run、永続化は `policy grant add`）と fail-closed
  再検査・cmdline 露出の注意を記述。
- `docs/tasks/grant-docs-and-apm.md` → done（relations.changes・
  implemented-by・Done When・Verification 実績を同期）、
  `docs/tasks/README.md` の T1–T6 行を done へ同期（index が
  lag していた T1–T5 を含む）、plan
  `implement-invocation-scoped-command-grants.md` および
  `docs/plans/README.md` の status を completed へ。impl
  record 本 file を scaffold（`new_impl_record.js`）で生成し、
  `docs/impl/ir/README.md` へ行追加（scaffold 自動更新）。
  chain 上の reciprocal-relation warning（test-spec ↔ plan/ADR、
  plan ↔ ADR/先行 plan ×2、design ↔ 先行 design ×2/discovery）
  8 件を link-back 追加で解消（d464fd0 の closeout 規約と同型）。

### Decisions During Implementation

- README に literal な commands 表は存在しない — command 列挙面は
  Basic usage の bash block が該当するため、plan の「commands 表の
  `policy` 行」は同 block への `policy grant list` 追加として
  解釈した（#102 T5 で `tool-operate` を同 block へ追加した既存
  規約に倣う）。
- usage.md のコマンド構成 block は HELP と mirror する規約の
  ため、`policy grant` 3 形態は HELP literal と同じ文言で末尾へ
  追加した（既存 block が `hook` を auth の後に置く並びは維持）。
- `--grant` の詳細 contract は command ガイドを正典とし、
  usage.md・SKILL.md では使用面 + pointer に留めて表現ドリフトを
  避けた（同一概念の表現を一箇所に集約）。

## Related Experiments

### Adopted

なし。確定済み design/plan に従う docs task で追加 experiment は
起こさなかった。

### Rejected

なし。

## Validation

- `node scripts/check-doc-ids.mjs` — exit 0。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
  — exit 0（新規 finding 無し）。
- `node scripts/check-apm-payload.mjs` — pass（APM payload は
  SKILL.md × 2 の更新のみで slim payload 規約に適合）。
- `grep -rn -- "--grant" docs/guides/command.md docs/guides/usage.md
  packages/apm/.apm/skills/` — match あり。
- `grep -n "policy grant" README.md docs/guides/usage.md` — match
  あり。
- `grep -n "grantRequests" docs/guides/workers.md` — match あり。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js` —
  findings 0。
- `mise run check` — pipeline 全段 exit 0（AC12）。

## Risks

- なし。docs/APM skill のみの変更で実行経路を持たず、revert は
  単独で可能。

## Follow-ups

- なし。issue #105 の全 task が本 task で完了。

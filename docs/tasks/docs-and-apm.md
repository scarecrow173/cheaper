---
id: "TASK-034EUvSZEQbvfLXkjtnqnF"
type: "task"
status: "todo"
title: "T5: user-facing docs・APM skill・impl record"
created: "2026-10-13"
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
  implements:
    - "../plans/implement-tool-operator-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-tool-operator-built-in-worker.md"
    - "docs/tasks/tool-operator-worker-and-alias.md"
    - "docs/tasks/fixture-dsl-and-verification.md"
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
    - "../test-specs/tool-operator-built-in-worker.md"
  references: []
  defers: []
  deferred-by: []
---

# T5: user-facing docs・APM skill・impl record

## Scope

`tool-operator` worker・`tool-operate` alias・`cheaper.optionalTools`
縮退機構の user-facing 文書を整備し、APM skill `cheaper-tool-operate` を
追加して cheaper-usage SKILL.md の delegation guidance を更新する。
T3 で確定した worker/alias の最終形と T4 で固定した挙動を正確に mirror
する。完了時に impl record（`docs/impl/ir/` 新規 1 file）を task-doc →
impl-doc の既存規約に従い scaffold で生成する。

- In: README built-in workers 表、workers.md（表 + 入力 schema 要点 +
  `cheaper.optionalTools` frontmatter key リファレンス + 縮退挙動 +
  blocked-status 推奨）、usage.md（alias 行・列挙文・per-alias 使用例 +
  `directlyComparable`）、command.md（opt-in caveat + admission /
  content-gate 棲み分け）、APM skill `cheaper-tool-operate` 新規 +
  `cheaper-usage` SKILL.md 更新、impl record。
- Out: `docs/guides/configuration.md`（worker frontmatter key の記述を
  持たないため原則無変更 — `optionalTools` と交差する記述、worker との
  関係節 :201 周辺を発見した場合のみ最小追記し、差分が設計外なら
  review 対象として報告）、実装コード変更、test-spec の変更。

### 所有ファイル

- `README.md` — built-in workers 表（:29-30・:67-70・:84-86）へ
  `tool-operator` / `tool-operate` の行追加。
- `docs/guides/workers.md` — built-in worker 表（:66-68）への行追加、
  「入力 schema（要点）」（:78-85）の per-worker bullet への
  `tool-operator` 項目追加、frontmatter key リファレンス（:36-60）へ
  `cheaper.optionalTools` key 項目（構文: `tools` 宣言名の部分集合・
  gated tool（`process` / `command`）のみ・全 reject は
  `InvalidWorkerDefinition`）+ 縮退挙動の説明 + opt-in worker は
  `blocked` 相当の status を output schema に含める推奨（Risk #8）。
- `docs/guides/usage.md` — コマンド構成の embed 済み alias 行（:13-16）
  へ `tool-operate           Alias for: run tool-operator` 追加、alias
  列挙文（:28）へ `tool-operate` 追加、per-alias 使用例（:90-107）へ
  tool-operate 例追加、`savings.directlyComparable=false` の扱い
  （`process` / `command` 宣言で常に false —
  `is_directly_comparable_worker` の mutating 判定）。
- `docs/guides/command.md` — :9 の「`command` を expose する runtime
  構築は `POLICY_VIOLATION` で fail closed」への opt-in caveat をその場で
  補足 + `tools.allow` admission と content-gate の棲み分け（admin が
  `command` を禁止しつつ縮退実行を許すには grant を定義しない）。
- `packages/apm/.apm/skills/cheaper-tool-operate/`（新規）—
  `cheaper-bulk-read` / `cheaper-code-write` / `cheaper-doc-write` と
  parity の SKILL.md。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — command list に
  `tool-operate` 追加 + tool-operator への delegation guidance
  （multi-step 操作 loop の委譲先として言及）。
- `docs/impl/ir/`（新規 1 file）— 実装完了時の IMPL record（task-doc →
  impl-doc の既存規約に従い scaffold で生成）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-tool-operator-built-in-worker.md` — 実装対象 plan（T5）
- `docs/designs/tool-operator-built-in-worker.md` — Risks and Trade-offs
  （docs 必須記載事項）
- `docs/specs/tool-operator-built-in-worker.md` — AC12
- `docs/test-specs/tool-operator-built-in-worker.md` — 検証 spec
  （公開面 docs の記述内容は行為保証の対象外 — Non-goals 参照）

## Dependencies

- `docs/tasks/tool-operator-worker-and-alias.md`（T3 — 記述対象の
  worker/alias が存在）
- `docs/tasks/fixture-dsl-and-verification.md`（T4 — docs が記述する
  挙動の確定）

## Steps

1. `README.md` の built-in workers 表と `workers.md` の表・入力 schema
   要点・frontmatter key リファレンスを更新する（`cheaper.optionalTools`
   key 項目・縮退挙動・blocked-status 推奨を含む）。
2. `usage.md` に alias 行・列挙文・per-alias 使用例・
   `directlyComparable=false` の扱いを追加し、`command.md` に opt-in
   caveat と admission/content-gate 棲み分けを補足する。
3. `packages/apm/.apm/skills/cheaper-tool-operate/` を既存 delegation
   skill と parity で新規作成し、`cheaper-usage` SKILL.md の command
   list + delegation guidance を更新する。
4. `docs/guides/configuration.md` に `optionalTools` と交差する記述が
   あるか確認し、発見した場合のみ最小追記する（原則無変更）。
5. 実装完了時に `docs/impl/ir/` へ impl record を scaffold で生成する。
6. doc audit と doc-id check が pass することを確認する。

## Done When

- [ ] built-in workers 表（README + workers.md）・workers.md 入力 schema
  要点・`optionalTools` frontmatter key リファレンス・縮退挙動・
  blocked-status 推奨が記載されている
- [ ] command.md の opt-in caveat・admission/content-gate 棲み分け、
  usage.md の alias 行/列挙文/使用例・`directlyComparable` 記述が揃う
- [ ] `cheaper-tool-operate` skill が parity で存在し、cheaper-usage
  SKILL.md の command list + delegation guidance が更新されている
- [ ] impl record が `docs/impl/ir/` に生成されている

## Rollback

- task 単位の coherent commit を `git revert` — docs/APM skill/impl
  record のみの変更であり実行経路を持たないため単独 revert 可能。
  T5 → T4 → T3 → T2+T1 の逆順 unwind か fix-forward を選ぶ際の先頭
  task が本 task。
- revert 後に `node scripts/check-doc-ids.mjs` が pass すること。

## Verification

- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all` exits 0
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `grep -n "tool-operate" README.md docs/guides/workers.md docs/guides/usage.md` returns matches
- [ ] `grep -n "optionalTools" docs/guides/workers.md docs/guides/command.md` returns matches

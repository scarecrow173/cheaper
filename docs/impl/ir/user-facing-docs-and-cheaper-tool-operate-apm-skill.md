---
id: "IMPL-034EWZtjDHdhil5oaHVIvr"
type: "impl"
status: "completed"
title: "user-facing docs and cheaper-tool-operate APM skill"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/user-facing-docs-and-cheaper-tool-operate-apm-skill.md"}
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"}
    modified:
      - {"type":"doc","path":"README.md"}
      - {"type":"doc","path":"docs/guides/workers.md"}
      - {"type":"doc","path":"docs/guides/usage.md"}
      - {"type":"doc","path":"docs/guides/command.md"}
      - {"type":"doc","path":"docs/guides/managed-process.md"}
      - {"type":"doc","path":"docs/guides/parent-agent-integration.md"}
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-usage/SKILL.md"}
      - {"type":"doc","path":"docs/plans/implement-tool-operator-built-in-worker.md"}
      - {"type":"doc","path":"docs/plans/README.md"}
      - {"type":"doc","path":"docs/tasks/README.md"}
      - {"type":"doc","path":"docs/tasks/docs-and-apm.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/docs-and-apm.md"
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

# user-facing docs and cheaper-tool-operate APM skill

## Summary

T5。`tool-operator` worker・`tool-operate` alias・
`cheaper.optionalTools` 縮退機構の user-facing 文書を整備し、APM
skill `cheaper-tool-operate` を parity で追加した。T3/T4 で確定した
worker/alias・縮退挙動を正確に mirror し、fail-closed gate の
記述には opt-in caveat をその場で補足した。

## Context

TASK-034EUvSZEQbvfLXkjtnqnF。issue #102 の tool-operator built-in
worker の第 5 task（AC12）。T1(df540ad)が `optionalTools`
vocabulary・validate、T2(c9921f1)が degrade path + prompt
surfacing、T3(7be4f41)が builtin worker 定義・schema・
`tool-operate` alias、T4(c84781f)が fixture DSL + 行為保証 test
を着地済み。本 task は公開面 docs・APM skill・status 同期のみで
実装コードには触れない。

## Implementation

### Changes

- `README.md`: built-in workers 表へ `tool-operate` 行追加、
  capabilities bullet に `tool-operate` 追加、basic usage の
  command 例へ `cheaper tool-operate < task.json` 追加。
- `docs/guides/workers.md`: built-in workers 表へ `tool-operator` /
  `tool-operate` 行（`outputPerInput: 0.3`、tools 7 件）追加 +
  表直下に縮退挙動の説明段落。frontmatter key リファレンスへ
  `cheaper.optionalTools` 項目を新設 — 構文（`tools` 宣言名の
  真部分集合・gated tool（`process`/`command`）のみ・全 reject は
  `InvalidWorkerDefinition`）、縮退 semantics（content-gate 不成立
  で未登録化・run 継続・unknown-tool fail-safe・capability
  shrinking 専用）、`tools.allow` admission との層分離、opt-in
  worker は `blocked` 相当 status を output schema に含める推奨
  （Risk #8）。「入力 schema（要点）」へ `tool-operator` bullet
  （`task`・`context`・`constraints`・`output`）追加。
- `docs/guides/usage.md`: HELP 準拠の alias 行
  `tool-operate           Alias for: run tool-operator` を追加、
  alias 列挙文へ `tool-operate` 追加、per-alias 使用例を追加。
  `savings.directlyComparable` の記述へ `process`/`command` 宣言
  worker（tool-operator）は常に `false`（宣言 tools を見る保守的
  判定・縮退 run でも変わらない）を明記。
- `docs/guides/command.md`: 「grants 空で `command` を expose する
  runtime 構築は `POLICY_VIOLATION`」への opt-in caveat をその場で
  補足し、content-gate（empty grants/tasks）と `tools.allow`
  admission の層分離を新段落で説明 — admin が「command 禁止 +
  縮退実行許可」を実現するには grant を定義しないこと（
  `tools.allow` から外すと run 全体 fail）。
- `docs/guides/managed-process.md`: `task が空なら expose されない`
  記述へ同趣旨の opt-in caveat を括弧追記。
- `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`（新規）:
  `cheaper-bulk-read` / `cheaper-doc-write` / `cheaper-code-write`
  と同一構造（frontmatter name/description・When to use・
  How to invoke・input field 表・Consuming the result）で作成。
  degrade semantics 節を追加 — grant/tasks 無しでも動作し
  `status: "blocked"` を返し得る点、`tools.allow` 除外は
  縮退ではなく run 全体の fail である点を明記。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`: Commands 一覧へ
  `tool-operate` 行追加、Delegation guidance へ tool-operator への
  委譲指針（multi-step 操作 loop の委譲先・blocked 可能性）追加。
- `docs/guides/parent-agent-integration.md`: skills 件数 4 → 5 に
  更新し `cheaper-tool-operate` を列挙へ追加（plan file map 外だが
  同一記述の突合として必要な最小追記）。
- `docs/tasks/docs-and-apm.md` → done、`docs/tasks/README.md` の
  T1–T5 全行を done へ同期（T1–T4 は task file 側のみ done で
  index が lag していた）、plan
  `implement-tool-operator-built-in-worker.md` および
  `docs/plans/README.md` の status を completed へ。
- `docs/guides/configuration.md` は無変更 — worker frontmatter key
  の記述を持たず、`optionalTools` と交差する記述も :201 周辺は
  workers ガイドへの pointer のみだった（plan の変更基準どおり）。

### Decisions During Implementation

- `docs/guides/parent-agent-integration.md` の「skills（4 件）」は
  file map に無いが新 skill 追加で直ちに不整合になるため、件数と
  列挙の最小追記を本 task の差分に含めた（同じ情報の 2 箇所表現を
  放置しない DRY の範囲内の修正として扱う）。
- usage.md の command 一覧は HELP literal（`crates/cheaper-cli/src/
  cli.rs` HELP）と verbatim で mirror する規約に従い、`tool-operate`
  行は HELP と同じ `doc-write` の直後・`count-tokens` の前に置いた。
- `command.md` の trust boundary 記述（writeScope 非両立・grant の
  みが信頼境界等）は opt-in 後も正確なままのため変更しなかった —
  縮退が緩めるのは登録面のみという 1 文で挙動差分を固定。

## Related Experiments

### Adopted

なし。確定済み design/plan に従う docs task で追加 experiment は
起こさなかった。

### Rejected

なし。

## Validation

- `node scripts/check-doc-ids.mjs` — exit 0。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
  — exit 0。本 diff で新規 finding は出ていないが、#102 関連 doc
  には 5 件の pre-existing warning（`related`/`derives-from` の
  link-back 未記載 — plan/design/ADR/spec/discovery 間、全て
  承認 commit 由来で本 task の変更起因ではない）が残存する。
  repo 全体に同種 warning が散在しており、本 task の file
  ownership 外の doc relation 修正となるため未対処。
- `node scripts/check-apm-payload.mjs` — pass。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js` —
  findings 0。
- `grep -n "tool-operate" README.md docs/guides/workers.md
  docs/guides/usage.md` — match あり。
- `grep -n "optionalTools" docs/guides/workers.md
  docs/guides/command.md` — match あり。

## Risks

- なし。docs/APM skill のみの変更で実行経路を持たず、revert は
  単独で可能。

## Follow-ups

- 全 task green 後の `mise run check`（AC13）を残す。

---
id: "IMPL-03466yClZgunJa94h9lzJV"
type: "impl"
status: "completed"
title: "Workflow discover skill implementation record"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/203"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-workflow-discover-skill.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-workflow-discover-skill.md"
    - "docs/plans/implement-workflow-discover-skill.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/workflow-discover-skill-issue-203.md"
    - "docs/designs/workflow-discover-skill-structure.md"
    - "docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/implement-workflow-discover-skill.md"
    - "docs/plans/implement-workflow-discover-skill.md"
    - "docs/designs/workflow-discover-skill-structure.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-discover-skill-issue-203.md"
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Workflow discover skill implementation record

## Summary

issue #203 の L1 skill `cheaper-workflow-discover` を新設した
（`packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` —
front matter + 本文 7 節）。guide 2 件を更新:
`parent-agent-integration.md` の skills 一覧を 6 件化し L0 節へ
L0→L1 handoff 補足、`workflow-usage.md` に discovery 相互参照節を
追加した。code・`apm.yml`・hook package・他 skill・deploy 写しへの
変更はゼロ。

## Context

TASK-034IUTJoHjZJipMihVLIMX / PLAN-034Y9sqpRWawlMTaY1PIyP /
DESIGN-034ziRfYArWAlH3iiItcAR / SPEC-034LNA8FqEjwFaDnqvIUc5 /
ADR-034aSc75jH9YL83FNQRuwH / DISC-0349RTCACbyKZHJy1Cvnpv の
chain に従う。ADR の Option 1（独立 skill に比較規約を集約、guide
は要約 + pointer）をそのまま実装した。

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md`
  - front matter: `name: cheaper-workflow-discover`（#202 の L0 節
    が参照する確定名）、`description` に比較意図の発火語彙
    （L0 handoff・手組み fan-out/fan-in・長い中間出力・明示的な
    検討要求）+ discovery-only 境界句（file を書かない・
    `workflow run` しない・grant/policy/catalog を変えない）。
  - 本文 7 節: Boundaries（write・`workflow run`・権限変更禁止・
    定義 file を inspect しない・workspace scoped lookup のみ・
    親 context ≠ cheaper telemetry・最小収集）、What to gather
    （observed/user-stated/unknown 分離・捏造禁止）、Comparison
    procedure（7 step: direct adequacy → workers → routes →
    `workflow list` → `workflow interface` → 同一基準比較 →
    economics）、Evidence labels（measured/estimated/unknown・
    parent token 節約の断定禁止）、Output contract（決定語彙・
    alternatives・evidence・conceptual proposal summary・免責文・
    user-facing 例）、Handoff to L2（"if installed" 条件付き・
    未 install 時は manual authoring 経路・negative 決定は終了）、
    Non-goals。
- `docs/guides/parent-agent-integration.md`
  - skills 一覧を 5→6 件へ: `cheaper-usage` に L0 入口の役割を
    補記し、`cheaper-workflow-discover` を L1 skill として追加。
  - L0 節内に `### L0 → L1 handoff で読まれた後の
    cheaper-workflow-discover` を追加 — L1 が読まれた後の
    振る舞い（read-only inspection・reuse-first 比較・
    evidence-labeled 提案・proposal ≠ 作成/実行・L2 または
    manual authoring が永続化を担う）を 1 段落で要約し、canonical
    は skill 本文と明記。既存 fallback 記述は維持。
- `docs/guides/workflow-usage.md`
  - catalog 節の後に `## Workflow 候補の発見と提案（L1）` を
    追加 — L1 が本 guide の選択基準・claim 境界を適用する評価層
    である旨と parent-agent 連携ガイドへの link のみ（規約複写
    なし）。

### Decisions During Implementation

- skill 本文は英語（他 skill と同じ言語）、guide は日本語（各
  file の既存言語に揃える）。
- `history list` は `--cwd` を受理しない（実 binary で確認）ため、
  skill では history 系に `--cwd` を指示しない書き方にした。
- `workflow interface` は workspace 内 path のみ受理（外側は
  `INVALID_REQUEST`）— catalog `path`（workspace 相対）をそのまま
  使う流れで記述し、明示 path 前提を skill に残した。
- `cheaper-workflow-create`（#204・未 merge）への言及は全箇所
  "if installed" / 「install 済みの場合」条件付きに限定 — 未
  install 時は manual authoring を経路として残す。
- decision vocabulary・proposal summary の YAML 形は issue #203 の
  記載どおりの確定語彙をそのまま採用（conceptual shape であり
  schema/CLI input ではない旨を明記）。

## Related Experiments

### Adopted

（experiment log なし — skill 1 file + guide 2 file の直接編集で
完結）

### Rejected

（experiment log なし）

## Validation

- `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js
  audit --type all --blocking --json` → `{"blocking":0}` —
  本 change 追加の 7 doc（discovery/spec/adr/design/plan/task/
  impl record）+ README index row を含む全 document で blocking
  finding なし
- `git diff --check` → whitespace error 0
- `mise run check` → 全 step pass（biome・fmt・clippy・
  check-version・check-doc-ids・cargo test --workspace・
  node --test・check-lockfile・check-apm-payload）
- `git diff --stat` → 変更 file は SKILL.md + guide 2 file + doc
  chain + README index のみ。crates/・`apm.yml`・hook package・
  `.agents/skills/` deploy 写しに差分なし
- CLI spot-check（`target/debug/cheaper.exe`、scratch workspace）—
  skill が指示する全 command を実 binary で検証:
  - `workflow list --cwd <ws>`: catalog 無し → `present:false`・
    空一覧・exit 0。catalog 有り → `workflows[]` に
    `{id,path,summary,tags,status}`（missing file は
    `status:"missing"` + `diagnostics` に `missing-path`）。
    utility envelope `worker:null`
  - `workflow interface <path>`: `{schemaVersion,inputs,outputs}`
    を返す（公開 I/O のみ）。workspace 外 path は
    `INVALID_REQUEST` exit 2
  - `route check`: read-only report（layers/routes/configuration/
    grants/diagnostics）exit 0。`route resolve --command` は
    dry-run `{matched,route,target,grant,viability}`（`no-grant`
    viability を確認）
  - `usage`: cheaper 自身の run のみの集計（`usage.totalTokens`・
    `usage.costUsd`・`measured.sourceTokens/returnedTokens/
    avoidedContextTokens/comparable.costUsd`）
  - `history list --limit --format`: 動作（`--cwd` は非受理 —
    skill は指示しない）
  - `count-tokens <file> --cwd <ws>`: `{path,tokens,bytes,
    tokenizer}`。`models`: `{models:[{provider,id,tier,
    effectiveCost,pricing}]}`
- scoped grep: SKILL.md に Boundaries・What to gather・Comparison
  procedure・Evidence labels・Output contract・Handoff・Non-goals
  節が存在。`workflow run` を指示する記述・workflow 定義 file を
  読む指示・無条件の L2 参照・user-global catalog 参照がないこと
  を確認（`workflow run` は禁止文脈・handoff の承認前提文脈でのみ
  言及）
- guide と skill 本文に verbatim 複写なし（目視）・
  `cheaper-workflow-create` 言及は全箇所条件付き（目視）

## Risks

- `description` が長く truncating host で切れる可能性 → #202 と同じ
  緩和（明示呼出し fallback が残る。design で受容済み）。
- L1 が install されていない consumer では L0 の handoff がこれ
  まで通り「観測結果をユーザーへ報告」に倒れる — #202 で受容済み
  の挙動で本 change で変化なし。
- skill が catalog 不在を「候補なし」と過度に結論づけるリスク →
  procedure step 4/5 で「absent は空一覧の事実・catalog は完全性を
  証明しない」を明示し対処。

## Follow-ups

- #204 `cheaper-workflow-create` merge 時に handoff 文言の確認 —
  "if installed" 条件付き記述のため変更不要の見込み。
- #205 横断 E2E で L0→L1→catalog→L2 の実機導線を検証し、runtime
  implicit firing の限界を実測値で更新する。

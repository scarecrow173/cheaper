---
id: "ADR-034aSc75jH9YL83FNQRuwH"
type: "adr"
status: "accepted"
title: "L1 workflow comparison lives in the discover skill body"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/203"
    - "../discovery/workflow-discover-skill-issue-203.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/workflow-discover-skill-issue-203.md"
  implemented-by:
    - "../designs/workflow-discover-skill-structure.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/workflow-discover-skill-issue-203.md"
    - "../specs/workflow-discover-skill-issue-203.md"
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
    - "../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-discover-skill-issue-203.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# L1 workflow comparison lives in the discover skill body

## Context and Problem Statement

issue #203 は L1 評価層を要求する — L0(#202)が見つけた候補または
ユーザーの明示要求に対し、direct tool・既存 Worker・既存 Route・
catalog 済み Workflow・新規 Workflow を同一基準（再利用価値・
遅延・コスト・不確実性）で比較し、**Workflow 化しない判断を含む**
evidence 付き提案を返す。Epic #200 の不変条件により、提案≠作成≠
実行であり、重い本文は必要時にのみ読む（常時判定コスト極小化）。

問題は、この比較手順・evidence 規約・出力契約をどの配布面に置く
かである。候補は (a) 独立 skill `cheaper-workflow-discover`、
(b) `cheaper-usage` への L1 節追加、(c) read-only 比較 CLI 新設、
(d) guide のみ、の 4 つ。

## Decision Drivers

- issue #203 が `packages/apm/.apm/skills/cheaper-workflow-discover/
  SKILL.md` の新設を明示し、skill 名と配置が固定済み
- epic #200 不変条件 2: 「スキルの重い本文は必要時にのみ読む」—
  L1 の比較本文（procedure・evidence 規約・output contract）は
  L0 より桁違いに大きく、常時面には置けない
- description 一致の implicit firing は全 target で非保証だが、
  明示呼出しはどこでも機能する（#202 で確立）— skill body は唯一
  全 APM target に均一に届く配布面
- CLI 新設は issue の非目標（新規 CLI・独自 cost estimator・
  定義構文の変更は対象外）であり、比較の判断・不確実性の扱いは
  deterministic な command 出力に押し込めない
- guide は agent context に入らないため単独では発火しない
  （#202 option C と同じ棄却理由）

## Considered Options

- Option 1: 独立 skill — `cheaper-workflow-discover/SKILL.md` に
  比較手順・evidence 規約・出力契約・条件付き L2 handoff を集約し、
  front-matter `description` を比較意図（L0 handoff・手組み
  fan-out/fan-in・明示的な検討要求）に一致するよう記述する
- Option 2: `cheaper-usage` への L1 節追加 — hub skill の末尾に
  比較手順を置く
- Option 3: read-only 比較 CLI（例: `workflow discover` 相当）を
  新設し、比較結果を deterministic に返す
- Option 4: guide 記述のみ（skill を増やさず consumer doc に手順を
  載せる）

## Decision Outcome

Chosen option: "Option 1: 独立 skill", because issue が名前と配置を
固定しており、「重い本文は必要時のみ」という L0/L1 分離の設計意図と
一致する。`description` が discovery surface に載り、必要時だけ
procedure 全文が読まれる。guide 側は要約 + pointer（canonical は
skill 本文）の既存規約どおり補完に留める。

### Consequences

- Good, because `cheaper-usage` の L0 節が条件付きで指していた
  `cheaper-workflow-discover` が実在物になり、L0→L1 handoff が
  dangling でなくなる。
- Good, because 比較本文が独立ファイルのため、CLI 契約を読むだけの
  task が L1 本文を一切ロードしない。description の発火語彙は
  「比較意図」に絞れる。
- Good, because code・manifest・hook・payload への変更がゼロで、
  `check-apm-payload` の slim payload 契約に影響しない（skills は
  wiring 検査対象外）。
- Bad, because skill が 5→6 件に増え、`cheaper-usage` 以外の skill が
  「いつ読むか」の判断を description だけに頼る面が 1 件増える —
  `description` に L0 handoff と明示的検討要求の両方を書いて補う。
- Bad, because `cheaper-workflow-discover` が install されていない
  consumer では L0 の handoff がこれまで通り「観測結果をユーザーへ
  報告」に倒れる — これは #202 で受容済みの挙動で、本 ADR で変化
  しない。

### Confirmation

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` が
  存在し、front matter と 7 節（boundaries・inputs・procedure・
  evidence・output contract・handoff・non-goals）を text 検査で
  確認する。
- skill が指示する全 CLI command（`workflow list`・
  `workflow interface`・`route check`/`route resolve`・`usage`・
  `history`・`count-tokens`・`models`）を実 binary に対して実行し、
  flag と出力形状を確認する。
- `doc_status` audit が blocking 0、`mise run check` が exit 0。

## Implementation Plan

- Affected paths: `packages/apm/.apm/skills/cheaper-workflow-discover/
  SKILL.md`（新規）、`docs/guides/parent-agent-integration.md`
  （skills 一覧 + L0→L1 handoff 注記）、`docs/guides/workflow-usage.md`
  （discovery 相互参照節）+ doc chain + README index。
- Patterns to follow: #202 の canonical/pointer 方式（規約正本は
  skill 本文、guide は要約 + matrix/fallback 相当の補完のみ）、
  `workflow-usage.md` の claim 境界（parentUsage unavailable 時は
  節約を主張しない）を evidence 規約へ翻訳、#202 の "if installed"
  条件付き言及（L2 #204 向けにそのまま再利用）。
- Patterns to avoid: skill 本文への CLI 説明の過剰複製（envelope の
  詳細は `cheaper-usage`/usage.md を canonical とする）、workflow
  定義ファイルを読む指示、未測定の parent token 節約主張、未 merge の
  `cheaper-workflow-create` を前提にした無条件 handoff、catalog を
  実行・権限の正本として扱う記述。
- Dependencies or configuration: なし（新規依存・feature flag・
  `apm.yml` 変更なし）。

## Verification

- [x] `workflow list`（catalog 有・無）・`workflow interface`・
      `route check`・`route resolve --command`・`usage`・
      `history list`・`count-tokens`・`models` を
      `target/debug/cheaper.exe` で実行し、skill 記述の flag・出力
      形状が実装と一致することを確認
- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js
      audit --type all --blocking --json` → blocking 0
- [x] `git diff --check` → whitespace error 0
- [x] scoped grep で skill の禁止事項（`workflow run` 指示・定義
      file 読取・無条件 L2 参照・user-global lookup）が無いことを確認
- [x] `mise run check` → 全 step pass

## Pros and Cons of the Options

### Option 1: 独立 skill

- Good, because 重い比較本文が on-demand ロードに収まり、全 target
  に均一に届く唯一の面になる。命名・配置は issue が固定済み。
- Bad, because skill 件数が増え、runtime の implicit firing 限界が
  1 件分増える（#202 の per-target matrix がそのまま適用される。
  明示呼出し fallback で補う）。

### Option 2: cheaper-usage への L1 節追加

- Good, because 新規 primitive なしで L0 と同じ面に載る。
- Bad, because 全 CLI 契約 read が比較本文を引きずり、L0/L1 を分離
  した設計意図（重い本文は必要時のみ）に正面から反する。

### Option 3: read-only 比較 CLI

- Good, because deterministic で testable な比較出力が得られる。
- Bad, because issue の非目標（新規 CLI・独自 estimator）に違反し、
  判断・不確実性・ユーザー context 依存の比較を固定出力に押し込め
  ない。親 agent しか持たない evidence（観測済み反復・context 中の
  中間結果）を CLI は収集できない。

### Option 4: guide 記述のみ

- Good, because 配布物ゼロで最小 diff。
- Bad, because guide は agent context に自動で入らず発火面が残らない
  — L1 が実質 manual に後退する。補完としては使うが単独不可。

## More Information

- Discovery: `../discovery/workflow-discover-skill-issue-203.md`
  （CLI surface の実 binary 検証済み一覧）
- Spec: `../specs/workflow-discover-skill-issue-203.md`
- 関連 ADR: `../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md`
  （L0 配置）、`../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md`
  （catalog の権威境界 — L1 が依拠する）
- Epic: #200、後続: #204（L2、conditional 参照）、#205（横断 E2E）

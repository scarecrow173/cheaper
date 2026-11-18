---
id: "ADR-034Zk1CgFRRcnAwV6sZWqQ"
type: "adr"
status: "accepted"
title: "L0 workflow trigger lives in cheaper-usage skill metadata"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/202"
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
  implemented-by:
    - "../designs/l0-workflow-candidate-trigger-distribution.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/apm.yml"
    - "docs/guides/parent-agent-integration.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# L0 workflow trigger lives in cheaper-usage skill metadata

## Context and Problem Statement

issue #202 は「通常作業中に Workflow 化の兆候を見つけたら L1 skill
(`cheaper-workflow-discover`、#203) を読むか」を決めるだけの極小の常時
判定規約 L0 を配布するよう求める。制約は 3 つある。(1) 独立した大きな
skill 本文は作らず、既存 skill の短い description/発動規約・consumer
向け統合ガイド・opt-in 最小テンプレートのいずれかで提供する。
(2) 毎 turn の CLI/LLM/file scan を追加しない。(3) description 一致の
間接発火は全 target で保証できないため、明示呼出し fallback が要る。

配布経路は `packages/apm/.apm/skills/` で固定済み（epic #200 の不変
条件 7）。残る問題は、5 つの既存 skill のどれに L0 を載せるか、あるいは
載せずにガイド/テンプレートだけにするかである。

## Decision Drivers

- issue #202 が「既存 skill の短い description/発動規約」を提供面の第 1
  候補として明示し、独立 skill 本文を非目標とする
- L0 が発火するには runtime の skill discovery surface（front-matter
  `description`）に載る必要がある — repo 内 `AGENTS.md` や guide 単独は
  consumer の agent context に入らず発火し得ない
- `cheaper-usage` は「Read this before calling any cheaper command」を掲げる
  hub skill で、全 consumer が同じ 1 点から deploy される — 配布対象を
  増やさずに L0 を常時届けられる唯一の既存面
- L0 の本文を別 skill にすると、発火した agent が CLI 契約の 443 行を
  読まずに済む一方で、issue の「独立 skill を作らない」に違反する
- guide と opt-in template は発火面としては機能しないが、skill
  auto-firing が無い環境の fallback と保証限界の文書化として必要

## Considered Options

- Option 1: `cheaper-usage` skill metadata — `description` に短い発火句を
  追加し、`SKILL.md` 末尾に小さな L0 節を置く。guide には matrix・
  fallback・opt-in template を載せる
- Option 2: 新規最小 skill（例: `cheaper-workflow-candidate`）を
  `.apm/skills/` に追加し、description のみで発火させる
- Option 3: skill には触れず、guide 節 + opt-in template だけで配布する

## Decision Outcome

Chosen option: "Option 1: `cheaper-usage` skill metadata", because 唯一の
常時配布面である hub skill の description が runtime の discovery に乗る
唯一の既存手段であり、issue の「独立した大きな skill 本文を作らない」を
厳密に守れる。本文末尾の小さな L0 節は on-demand で読まれるため、無関係な
task が規約本文を常時ロードすることもない。guide 節と opt-in template は
Option 1 を補完する — fallback と発火限界の文書化は guide の責務であり、
重複ではなく相互参照とする。

### Consequences

- Good, because 新規 primitive・`apm.yml` 変更・code 変更が一切不要で、
  slim payload 契約と `check-apm-payload` wiring 検査に影響しない。
- Good, because `description` の発火句が skill-capable な全 runtime で
  L0 シグナルを拾い、明示呼出しでも同じ節に到達できる（fallback が同一
  規約を指す）。
- Bad, because `cheaper-usage` の implicit firing 範囲が CLI 呼出し文脈
  以外（反復作業の検出）へ広がる — issue が意図する効果であり、L0 節の
  抑制条件で誤発火時の振る舞いを束縛する。
- Bad, because `cheaper-usage` を install していない consumer には L0 が
  届かない — ただし cheaper skill を持たない環境で workflow 委譲自体が
  成り立たないため実害はない。

### Confirmation

- `description` に L0 発火句があり、`SKILL.md` に L0 節（シグナル・
  抑制条件・条件付き L1 handoff）が存在することを text 検査で確認する。
- `docs/guides/parent-agent-integration.md` に per-target matrix・
  fallback・opt-in template があることを確認する。
- `doc_status` audit が blocking 0、`mise run check` が exit 0 を返す。

## Implementation Plan

- Affected paths: `packages/apm/.apm/skills/cheaper-usage/SKILL.md`、
  `docs/guides/parent-agent-integration.md`、
  `docs/guides/workflow-usage.md`（相互参照 1 行）
- Patterns to follow: DISC-034r05iHy2gdnpdai9KGpW の per-target 検証
  matrix の誠実な記録方式（保証できる事実と fallback の分離）、
  SPEC-034r05iHK8bA7gky93Dal0 の skill/guide 間 pointer 方式（canonical
  を決めて複写しない）
- Patterns to avoid: CLI 説明の skill 内複製、guide への skill 本文の
  verbatim 複写、存在しない `cheaper-workflow-discover` の即時実行を
  促す記述、hook 意味論の変更
- Dependencies or configuration: なし（新規依存・feature flag・
  `apm.yml` 変更なし）

## Verification

- [x] `grep -n "workflow-candidate\|L0" packages/apm/.apm/skills/cheaper-usage/SKILL.md`
      で description 句と L0 節を確認
- [x] guide に matrix・fallback・template が存在し、skill 本文との verbatim
      複写がないことを目視確認
- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit
      --type all --blocking --json` が blocking 0
- [x] `mise run check` 全 step pass（flaky timing test のみ間欠失敗 —
      impl record 参照）

## Pros and Cons of the Options

### Option 1: cheaper-usage skill metadata

- Good, because 全 consumer に既に deploy される hub skill を再利用し、
  description（auto-invocation surface）と on-demand 本文の両方に
  規約を置ける。発火面・規約面・fallback が 1 つの正本に収束する。
- Bad, because `cheaper-usage` の implicit firing 範囲が広がり、CLI 無関係の
  反復作業でも skill が提示されうる（意図する効果。抑制条件で束縛）。

### Option 2: 新規最小 skill

- Good, because workflow 候補検出だけを目的にした description にでき、
  発火時に CLI 契約本文を読ませない。
- Bad, because issue #202 の「独立した大きな skill 本文を作らない」に
  違反し、#203 の `cheaper-workflow-discover` と責務・命名が衝突する。

### Option 3: guide + opt-in template のみ

- Good, because skill 配布物を一切変更せず最も小さい diff になる。
- Bad, because guide と template は agent context に自動で入らないため
  auto-firing 面が何も残らず、L0 の「常時判定」が実質 opt-in 手動運用に
  後退する — 補完としては必須だが単独では不十分。

## More Information

- Discovery: `../discovery/l0-workflow-candidate-trigger-issue-202.md`
  （target 別 discovery 実測と runtime 公式 doc の evidence）
- Spec: `../specs/l0-workflow-candidate-trigger-issue-202.md`
- Epic: #200（L0/L1/L2 3 層導線）、後続: #203（L1）、#205（E2E）

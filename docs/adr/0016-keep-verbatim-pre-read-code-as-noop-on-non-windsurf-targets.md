---
id: "ADR-0016"
type: "adr"
status: "accepted"
title: "Keep the verbatim pre_read_code deployment as a documented no-op on non-Windsurf targets and defer per-target hook adapters"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0013-cross-agent-distribution-verification.md"
    - "./0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../specs/0013-cross-agent-distribution-verification.md"
  references:
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 16. Keep the verbatim pre_read_code deployment as a documented no-op on non-Windsurf targets and defer per-target hook adapters

## Context and Problem Statement

`cheaper` は APM package として `apm.yml` の targets
（copilot, codex, claude, cursor, windsurf, agent-skills）へ配布される。
`apm install` は hook bundle（`pre-read-code.js` + bundled `cheaper.mjs`、
ADR-0013）と target ごとに生成される hook config を deploy するが、
すべての config が Windsurf 固有の event 名 `pre_read_code` を
verbatim で登録する。

調査（DISC-0009）により、`pre_read_code` event が存在するのは
Windsurf のみであることが確定した。target ごとの実態は以下の通り:

- **Claude Code**: `PreToolUse` hook + matcher `Read` が必要。
  stdin は `tool_input.file_path`、block は exit code 2 または
  stdout の `permissionDecision: "deny"` で行う
- **Cursor**: `beforeReadFile` hook が必要。stdout の
  `{permission: "deny"}` または exit code 2 で block する
- **Copilot**: `view` tool に対する `preToolUse` hook が必要。
  block は stdout の `permissionDecision` で行う。command hook は
  **fail-closed** であり、non-zero exit は deny として扱われる
- **Codex**: file read を intercept する hook が存在しない
  （upstream issue openai/codex#20204）。hook による large-read
  block は原理的に不可能
- **agent-skills**: skills のみの target で hook deploy 対象外

つまり non-windsurf の 4 target では hook config が deploy されるが、
対応する event が発火しないため hook は silent な no-op になる。
この状態を「そのまま残す」「今すぐ直す」「target を削る」のどれで
扱うかが本 ADR の決定対象である。

## Decision Drivers

- 未検証の contract で adapter を ship しないこと。特に Copilot は
  command hook が fail-closed で、誤った adapter（non-zero exit を
  返す実装）は **すべての tool call を deny** し得るため、間違いが
  no-op よりもはるかに有害である
- skills（`cheaper-bulk-read` 等）は agent-agnostic な markdown であり
  全 target で有用。hook が動かないことを理由に配布対象を絞ると
  skills の価値を失う
- 各 target の hook contract（event 名・stdin schema・stdout/block
  semantics）は互いに異なり、Codex のように適合不可能な target も
  含まれる。単一の変更に束ねると検証不能な変更になる
- 検証と分類を先に行い、各 target の適合は独立に reviewable な
  単位で後続させる incremental delivery を守る
- no-op である hook は fail-open かつ harmless であること。
  誤った deny を出さない限り残してよい

## Considered Options

- **Status quo + verification** — verbatim `pre_read_code` deploy を全
  target で維持し、各 target の挙動を test と canonical contract
  matrix で検証・分類する。functional adapter は target ごとの
  follow-up decision chain に defer する（採用）
- **Uniform adapter now** — 1 つの hook script + config layer で
  event 名・stdin・stdout を target ごとに変換し、この変更で
  一括対応する
- **Restrict targets to windsurf** — adapter が存在するまで
  copilot / codex / claude / cursor を `apm.yml` targets から外す

## Decision Outcome

Chosen option: "Status quo + verification"（verify and classify now,
adapt per-target later）。

- `apm.yml` の targets と `.apm/hooks/` の構成は変更しない。
  non-windsurf targets への verbatim `pre_read_code` deploy は
  **documented no-op** として維持する
- 各 target の hook contract（event 名・stdin schema・stdout/block
  semantics・coverage 可否）を canonical contract matrix
  （DISC-0009 / SPEC-0013）に記録し、分類の single source of truth
  とする
- functional adapter は本 ADR では作らない。target ごとに独立した
  follow-up の ADR/spec chain で、matrix の検証済み contract に
  基づいて実施する
- 将来の adapter が必ず守る制約をここに固定する:
  - Copilot の command hook は fail-closed。adapter は deny を
    意図する場合のみ non-zero exit / `permissionDecision` を返し、
    異常系・未対応入力では deny しないこと
  - Codex は file read を intercept できない（openai/codex#20204）。
    adapter を書く対象ではなく、matrix 上の coverage gap として
    記録し続けること

### Consequences

* Good, because 間違った contract の adapter による agent 破壊の
  リスクをゼロにできる。Copilot の fail-closed semantics では
  誤動作する hook が全 tool call を block し得るため、この回避は
  特に重要である
* Good, because 検証済みの evidence trail（canonical contract
  matrix）が残り、各 target の適合を独立した reviewable な単位で
  後続させられる（per-target risk isolation）
* Good, because skills は引き続き全 target へ deploy され、
  agent-agnostic な価値を失わない
* Bad, because non-windsurf の 4 target では hook が silently no-op
  であり、large-read block / `bulk-read` 委譲が働かない。ユーザーは
  skills のみを得る。canonical matrix にこの挙動を明記して緩和する
* Bad, because 「hook が入っているのに効かない」状態は matrix を
  読まない限り発見しにくい。matrix の記録が事実上の要件になる

### Confirmation

DISC-0009 の調査で各 target の hook contract が実測・分類され、
SPEC-0013 の cross-agent distribution verification で verbatim
deploy の挙動（windsurf で機能、non-windsurf で no-op）が検証される。

## Implementation Plan

* Affected paths: `apm.yml`（変更なし — targets 維持）、
  `.apm/hooks/hooks.json` / `.apm/hooks/pre-read-code.js` /
  `.apm/hooks/cli/`（変更なし — verbatim deploy 維持）、
  `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
  および `docs/specs/0013-cross-agent-distribution-verification.md`
  （canonical contract matrix の記録先）
* Patterns to follow: per-target の挙動分類は canonical contract
  matrix を single source of truth として記録する。functional
  adapter は target ごとの別 ADR/spec chain で扱う
* Patterns to avoid: 未検証の contract で adapter を書くこと、
  複数 target の contract を 1 つの変更に結合すること、Copilot
  で意図せず non-zero exit を返す実装（fail-closed で全 tool
  call が deny される）、`apm.yml` targets の削減
* Non-goals: 本 ADR での adapter 実装、Codex に対する read
  interception の実現、non-windsurf targets への配布停止
* Dependencies or configuration: なし（deploy 構成の変更を伴わない
  決定）

## Verification

* [ ] `apm.yml` の targets が copilot / codex / claude / cursor /
      windsurf / agent-skills のまま維持されている
* [ ] 生成される hook config が全 non-windsurf target で
      `pre_read_code` を verbatim で登録し続ける
* [ ] canonical contract matrix に各 target の event 名・stdin
      schema・stdout/block semantics が記録されている
* [ ] Codex の coverage gap（read intercept 不可、
      openai/codex#20204）が matrix に明記されている
* [ ] non-windsurf targets の hook が documented no-op として
      matrix / spec に記述されている
* [ ] 本変更で per-target adapter が実装されていない
      （defer が守られている）

## Pros and Cons of the Options

### Status quo + verification (chosen)

* Good, because 間違った contract で agent を壊すリスクがない。
  Copilot の fail-closed semantics では誤った adapter が全 tool
  call を deny するため、no-op の方が誤 adapter より安全
* Good, because 各 target の適合が独立に reviewable な単位に
  なり、matrix という検証済みの判断材料が先に残る
* Bad, because non-windsurf ユーザーは skills を得るが hook は
  silently dead なまま。block 機能の欠如がユーザーから見えにくい

### Uniform adapter now

* Good, because 全 target で hook が（正しければ）一度に機能する
* Bad, because 4 つの異なる contract（Codex の不可能な target を
  含む）を 1 つの検証不能な変更に結合する。実 agent での検証
  手段がないまま ship することになり、incremental delivery に
  反する

### Restrict targets to windsurf

* Good, because 「deploy されるが機能しない」状態そのものを
  消せる
* Bad, because agent-agnostic で全 target で有用な skills
  （`cheaper-bulk-read` 等）の deploy まで失われる。hook の
  no-op は harmless なので、skills を犠牲にする理由にならない

## More Information

* `docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md`
  — per-target hook contract の調査・分類根拠
* `docs/specs/0013-cross-agent-distribution-verification.md`
  — cross-agent 配布検証仕様（canonical contract matrix）
* ADR-0013 — verbatim `pre_read_code` deploy を含む hook bundle
  配布の前提決定
* ADR-0012 — hook が担う large-read block の threshold 決定
* <https://github.com/openai/codex/issues/20204> — Codex の file
  read interception 欠如を追跡する upstream issue

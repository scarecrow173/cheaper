---
id: "DESIGN-034ZuedUsEd6RnxHqV4FL1"
type: "design"
status: "approved"
title: "L0 workflow candidate trigger distribution"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/202"
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
    - "../plans/l0-workflow-candidate-trigger-plan.md"
  depends-on:
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
    - "../discovery/l0-workflow-candidate-trigger-issue-202.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
---

# L0 workflow candidate trigger distribution

## Context

issue #202 の実装設計。SPEC-034Zk1kCap2wKUvTt21Sil と ADR-034Zk1CgFRRcnAwV6sZWqQ
で確定したとおり、L0 規約は `cheaper-usage` skill metadata（`description`
発火句 + `SKILL.md` 末尾の小さな L0 節）を主配布面とし、
`docs/guides/parent-agent-integration.md` の L0 節（matrix・fallback・
opt-in template）と `docs/guides/workflow-usage.md` の相互参照 1 行で
補完する。code・`apm.yml`・hook・他 skill は一切変更しない。

`packages/apm/.apm/skills/` 配下の `cheaper-usage/SKILL.md` が正本であり、
`.agents/skills/` 配下は APM deploy 写しで編集対象外（DESIGN-giTr4G7NzwSzWJTVgBo3ax
と同じ境界）。guide 2 件は consumer 向け doc であり、agent context へは
直接入らない — 発火面として機能するのは skill metadata のみで、guide は
fallback と発火限界の文書化を担う。

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` への 2 箇所の変更:
  (a) front-matter `description` 末尾への短い発火句の追加、(b) 本文末尾
  （「Delegation guidance」節の後）への L0 節の追加
- `docs/guides/parent-agent-integration.md` への L0 節追加（配置は
  「自動 vs 明示的な委譲」節の直後 — 発火/明示の区別を論じる文脈に
  接続する）
- `docs/guides/workflow-usage.md` の「選択基準」節末尾への相互参照
  1 行追加

### Out of Scope

- `cheaper-workflow-discover`（L1、#203）・`cheaper-workflow-create`・
  catalog（#201）・E2E（#205）
- `packages/apm/apm.yml`・hook package・他 4 skill・`crates/` 全般・
  `.agents/skills/` deploy 写し
- bulk-read hook・route hook・worker skill の意味論
- `cheaper-usage` の既存 CLI 説明の再編（複製しない・構造を変えない）

## Components and Boundaries

### 1. `cheaper-usage` `description`（発火面）

runtime の skill discovery が常時見る唯一の面。末尾に 1 文追加する:

```text
Also read when normal work shows workflow-candidate signals — repeated
identical procedures, manual fan-out/fan-in across tools or workers, or
long intermediate outputs — the L0 check below decides whether the
cheaper-workflow-discover skill is worth loading.
```

- シグナル語彙（repeated / fan-out / fan-in / long intermediate outputs）を
  description に載せることで、CLI 呼出し以外の文脈でも description 一致の
  implicit firing が起き得る。
- `cheaper-workflow-discover` は名前としてのみ言及し、即時実行を促さない
  （未配布 version でも dangling 参照にならないよう、存在を条件付きに
  するのは本文側の責務）。

### 2. `cheaper-usage` 本文 L0 節（規約の正本）

`## Workflow candidate check (L0)` を末尾に置く。内容は SPEC の
requirements の写しではなく、親 agent への行動規約として 4 ブロックで
構成する:

- 目的: 「L1 skill `cheaper-workflow-discover` を読むか」だけを決める。
  Workflow 化判断・重複探索・コスト分析・保存は行わない。判定は観測済み
  context のみに基づき、毎 turn の追加 CLI/LLM/file scan を要求しない。
- Candidate signals: issue #202 の 4 条件（反復・明示的反復要求、
  手組みの fan-out/fan-in・tool→Worker chain、長い中間出力の内部要約
  価値、入力差替えで再利用できる定型処理）
- Not a candidate: 1 回きりの単純 read/grep/command、既存単一 Worker で
  足りる作業、ユーザーが提案不要と示した状況、context 不足
- Suppression rules: 同一根拠は session 中 1 回程度・拒否済みは新事情
  なしに再提案しない。catalog list・`history`・`usage`・file scan を
  この判定のためだけに呼ばない（追加情報は L1 が実際に動くとき初めて
  確認する）。過去 session・他 agent の履歴を知っていると仮定しない。
  L0 は親 agent の判定規約であり hook ではない — PreToolUse route hook
  （#142）とは別経路で、hook から毎回 LLM 推論/提案を起こさない。
- When a signal holds: `cheaper-workflow-discover` skill が install 済み
  なら読んで L1 に任せる — L0 の役目はそこまで。install されていなければ
  呼び出そうとせず、観測した pattern が workflow 候補に見える旨を
  ユーザーに伝えて判断を委ねる。

本文は英語（同 file の言語に揃える）。行数は ~45 行以内に抑え、「極小の
発動規約」の要件を維持する。

### 3. `parent-agent-integration.md` L0 節（consumer 面の canonical）

「自動 vs 明示的な委譲」の直後に `## Workflow 候補の軽量判定（L0）` を
追加する。guide の責務は skill 本文の複写ではなく、contract・matrix・
fallback・template の提供:

- contract の要約: L0 は `cheaper-usage` skill の description/「Workflow
  candidate check (L0)」節に搭載された親 agent 判定規約で、「L1 を読むか」
  までを決める。Workflow 化判断・重複探索・コスト分析・保存はしない。
  シグナルと抑制条件は skill 側を canonical とし、guide は要点だけを
  指して相互参照する。
- per-target matrix（AC 要求の表）:

| target | APM 既定の deploy 先 | runtime の discovery | 発火限界 |
|---|---|---|---|
| claude | `.claude/skills/` | Claude Code が native scan | description 一致は model 判定 — 非保証。明示は `/<skill>` |
| codex | `.agents/skills/` | repo root + cwd 祖先を native scan | 同上。明示は `$<skill>` / `/skills` |
| cursor | `.agents/skills/` | Cursor 2.4+ が `.agents/skills/`（`.cursor/skills/`・`.claude/skills/`・`.codex/skills/` も）scan | 同上。明示は `/` メニュー |
| copilot | `.agents/skills/` | Copilot CLI が `.agents/skills/`（`.github/skills/`・`.claude/skills/` も）scan。IDE agent mode は version 依存 | 同上 |
| windsurf | `.agents/skills/` | Cascade が cross-agent compat で `.agents/skills/` を scan（`.devin/skills/`・`.windsurf/skills/` も） | 同上 |
| agent-skills（Devin CLI 等） | `.agents/skills/` | Devin CLI が session 開始時に読み込む（本 repo で dogfood 検証済み） | 同上 |

- 全 target 共通の限界: description 一致の間接発火は常に model の判断
  であり決定論的保証ではない。runtime version が Agent Skills 非対応
  なら明示呼出しのみになる。`--legacy-skill-paths` install では deploy 先
  が per-client dir へ変わるが、各 runtime は legacy dir も scan するため
  discovery 自体は維持される。
- fallback: runtime が skill を自動発火しない・拾わなかった場合でも、
  ユーザーが「`cheaper-usage` skill の L0 節を読んで現在の作業が
  Workflow 候補か判定して」と明示依頼すれば同じ判定が動く（skill 名の
  明示呼出しは auto-firing が無い環境でも機能する）。L0 が候補と判定
  したら `cheaper-workflow-discover`（L1、#203）を明示的に読む —
  未配布 version では L0 の観測結果をそのままユーザーへ報告する。
- opt-in 最小指示テンプレート: 常時適用を確実にしたい consumer が自前の
  `AGENTS.md`/rules に貼れる短い英文 block（skill auto-firing に依存
  しない L0 相当規約。`cheaper-workflow-discover` への言及は
  "if installed" 条件付き）。

### 4. `workflow-usage.md` 相互参照

「選択基準」節の末尾に 1 行追加し、L0/L1 の導線と本 guide の費用判断
基準を接続する（重複記述なし — 相互参照のみ）。L0/L1 が提案しても
本節の基準で Workflow 不適と判定され得ることを明示する。

## Data and Control Flow

1. 通常作業中、親 agent は runtime の skill discovery 経由で
   `cheaper-usage` の description を見ている。L0 シグナル語彙に一致
   すれば skill が提示され、agent が `SKILL.md` を読むと L0 節に到達
   する（明示呼出しでも同じ経路に合流）。
2. L0 節の規約に従い、agent は観測済み context だけで「L1 を読むか」を
   判定する。追加の CLI/LLM/file scan はここで行わない。
3. 候補なら `cheaper-workflow-discover` が install 済みかを確認し、
   あれば L1 へ引き渡す。なければ観測結果をユーザーへ報告して終了 —
   Workflow 化判断・重複探索・コスト分析・保存は L0 では行わない。
4. 提案抑制は規約側で束縛する（session 中 1 回・拒否済み再提案禁止）—
   L0 が頻出してもノイズにならない。

## Non-Functional Constraints

- 追加の実行コストゼロ: L0 自体は catalog list・`history`・`usage`・
  file scan・新規 hook event を毎 turn 呼ばない。skill 本体を増やさず、
  description + 1 節に収める。
- 後方整合: `cheaper-usage` の既存 section 構造・文言は変更しない
  （description 追加と末尾節追加のみ）。既存の委譲 guidance・exit code・
  envelope 説明は無変更。
- 表現ドリフト防止: L0 の規約正本は `cheaper-usage` L0 節に一元化し、
  guide は contract 要約 + matrix + fallback + template に留める
  （canonical/pointer 方式 — SPEC-034r05iHK8bA7gky93Dal0 先例）。
- 誇張禁止: 「全 target で常時自動起動する」とは書かない。matrix と
  fallback が発火限界を honest に記述する。

## Risks and Trade-offs

- `description` が長くなり truncating host で L0 句が切れる可能性
  → 許容。truncating host では description 全体が対象であり、L0 句を
  前置きすると既存の CLI 発火契約を損なう。fallback（明示呼出し）が
  残るため実害は限定的。
- `cheaper-usage` が CLI 無関係の反復作業でも提示される誤発火
  → 意図する効果（issue が通常作業中の検出を要求）。L0 節の抑制条件が
  提示後の振る舞いを束縛する。
- `cheaper-workflow-discover` 未配布期間に agent が存在しない skill を
  呼ぼうとする → 本文・guide・template のいずれでも "if installed"
  条件付き記述に限定し、未 install 時はユーザーへの報告へ倒す。
- #203 で skill 名が変わる → DISC open question に記録済み。名前が
  決まれば 1 行の同期修正。

## Deferred Design Concerns

- `cheaper-workflow-discover`（L1）skill 本文・比較プロセス・L1→L2
  handoff | #203 が所有 | L0 の参照名確定時に同期 | 名前変更時は L0
  節・guide・template の 3 箇所の同期が必要
- runtime 実機での implicit firing E2E | #205 の横断検証領域 | 実 agent
  環境の整備時 | matrix の unverified 列が残る
- opt-in template と skill L0 節の文言 drift | 本設計では template を
  guide にのみ置き skill には複写しない | template の改訂要求が出た
  場合 | 片方だけ古くなる — template は「最小規約」であって skill 節の
  写しではない旨を guide に明記

## Verification Notes

- [x] `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の
      `description` に発火句、本文に L0 節がある（grep で確認）
- [x] `docs/guides/parent-agent-integration.md` に matrix・fallback・
      template が入り、skill 本文との verbatim 複写がない
- [x] `docs/guides/workflow-usage.md` に相互参照 1 行がある
- [x] `git diff --stat` が上記 3 file + doc chain のみを含む
      （crates/・hook・`apm.yml`・deploy 写しに差分がない）
- [x] `doc_status` audit blocking 0・`mise run check` 全 step pass
      （flaky timing test のみ間欠失敗 — impl record 参照）

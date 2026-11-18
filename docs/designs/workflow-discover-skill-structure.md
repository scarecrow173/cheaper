---
id: "DESIGN-034ziRfYArWAlH3iiItcAR"
type: "design"
status: "approved"
title: "Workflow discover skill (L1) structure and guide integration"
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
    - "../specs/workflow-discover-skill-issue-203.md"
  implemented-by:
    - "../plans/implement-workflow-discover-skill.md"
  depends-on:
    - "../adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/workflow-discover-skill-issue-203.md"
    - "../adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
    - "../discovery/workflow-discover-skill-issue-203.md"
    - "../designs/workflow-catalog-manifest-and-read-only-list.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/workflow-discover-skill-issue-203.md"
    - "../adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
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
---

# Workflow discover skill (L1) structure and guide integration

## Context

issue #203 の実装設計。ADR-034aSc75jH9YL83FNQRuwH で確定した
とおり、L1 比較規約は独立 skill
`packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` に
集約し、guide 2 件は要約 + pointer で補完する。code・manifest・
hook・他 skill は一切変更しない。本 design は SKILL.md の節構成、
front-matter `description` の発火語彙、guide 更新の境界を固定する。

`packages/apm/.apm/skills/` 配下が正本であり `.agents/skills/` は
APM deploy 写し（編集対象外）。catalog 契約（#201）と L0 規約
（#202）は既に main にあり、L1 はその両方の consumer として
設計する。

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md`
  新設 — front matter + 本文 7 節構成（後述）
- `docs/guides/parent-agent-integration.md` — skills 一覧を
  5→6 件へ更新し `cheaper-workflow-discover` を追加 + L0 節への
  L1 handoff 補足（何をする skill かの 1 段落 + canonical pointer）
- `docs/guides/workflow-usage.md` — discovery 相互参照節（選択基準
  の権威を保ったまま L1 導線へ接続）

### Out of Scope

- `cheaper-workflow-create`（#204、未 merge — "if installed"
  条件付き言及のみ）・#205 横断 E2E
- `apm.yml`・hook package・他 5 skill・`crates/` 全般・
  `.agents/skills/` deploy 写し・catalog schema・workflow 定義
  構文
- proposal summary の永続 schema 化（概念形のまま — SPEC の
  deferred concerns 参照）

## Components and Boundaries

### 1. SKILL.md front matter（発火面）

`name: cheaper-workflow-discover` 固定（#202 の L0 節が既にこの名を
参照 — 変更すると 3 箇所の同期が必要になる）。

`description` は比較意図で発火する語彙を持つ: 「direct tools・
existing workers・routes・cataloged workflows・new workflow
candidate の比較」「cheaper-usage L0 check が候補を flag した時」
「手組みの fan-out/fan-in・tool→worker chain の反復」「長い中間
出力」「ユーザーが Workflow 化すべきか聞く時」。末尾に境界句 —
「proposal ≠ creation ≠ execution、file を書かない・workflow を
実行しない・grant/policy/catalog を変えない」を置き、発火時から
discovery-only の責務を明示する。

### 2. 本文構成（7 節）

本文は英語（他 skill と同じ言語）。構成:

1. **Boundaries** — discovery only（write・`workflow run`・
   grant/policy/catalog 変更の禁止）、workflow 定義 file を
   inspect 目的で読まない（`workflow list` + `workflow interface`
   のみ）、workspace scoped lookup のみ（user-global catalog は
   存在しない）、親 context ≠ cheaper telemetry（usage/history は
   cheaper 自身の run）、最小限の収集（巨大 log・会話全文・秘密
   入力を集めない）
2. **What to gather** — 手順要約・反復（observed/user-stated/
   unknown を分離）・代表 I/O・親が運ぶ中間結果・副作用と時間
   制約・既知の比較対象。欠落は 1 問だけ聞くか unknown とする —
   捏造しない
3. **Comparison procedure** — 順序付き 7 step: (1) 自動化自体の
   適性（1 回きり・単一 tool/worker で足りるなら explicit な
   `direct-tool`/`direct-worker`/`not-recommended`）、(2) 既存
   Worker（built-in 4 + custom）、(3) 既存 Route（`route check`・
   `route resolve` — read-only）、(4) catalog（`workflow list`、
   status/diagnostics の正直な報告、absent = 空一覧）、
   (5) 適合候補の `workflow interface`（公開 I/O のみ。reuse/
   parameterize/modify を create より先に比較。catalog は完全性
   を証明しない）、(6) 5 option の同一基準比較（再利用価値・
   latency・cost・uncertainty・constraints — tool-only 単純処理は
   workflow の方が遅くなり得る）、(7) 経済性
   （`initialAuthoringCost + N × workflowPerRunCost` vs
   `N × directPerRunCost`、break-even N は正の差分 + 観測値/価格
   仮定がある時のみ）
4. **Evidence labels** — `measured`（自身の観測・cheaper の
   telemetry・benchmark artifact）/`estimated`（宣言した仮定の
   下での導出）/`unknown`（それ以外）。`sourceTokens`/
   `returnedTokens`・benchmark byte 差から parent token 節約を
   断定しない（#67/#183 の claim 境界 — `workflow-usage.md` と
   同一規約）
5. **Output contract** — 決定（`reuse|modify|create|direct-tool|
   direct-worker|route|not-recommended` の 1 つ）・比較した
   alternatives・evidence labels・conceptual proposal summary
   （issue の YAML 形 — 永続 schema/CLI input ではない旨を明記）・
   「何も作成・保存・実行していない」免責文。user-facing 提案の
   記述例 1 件
6. **Handoff to L2** — `create`/`modify` + `cheaper-workflow-create`
   install 済み → ユーザー承認後に summary を渡す提案。未 install
   → 提案を報告し create skill または手動 authoring を経路として
   示す（自分で JSON を生成・保存しない）。`reuse` → 明示 path と
   公開 interface を示し、実行は別途明示要求のみ。`direct-*`/
   `route`/`not-recommended` → 理由を述べて終了（workflow 提案を
   残さない）
7. **Non-goals** — 常時監視・per-turn scan（L0 の責務）、authoring/
   JSON 生成・保存・catalog 登録・grant/policy 変更（L2 の責務）、
   `workflow run`、親履歴の telemetry からの再構成、独自 cost
   estimator・完全性を証明する重複検出

### 3. `parent-agent-integration.md` 更新

- 「配布 primitive」の skills 一覧を 5 件→6 件へ:
  `cheaper-workflow-discover` — 「反復・orchestration 負荷の高い
  作業の実行選択肢（direct tool・Worker・Route・catalog 済み
  Workflow・新規 Workflow）を比較し evidence 付きの提案を返す L1
  skill。提案のみで作成・実行はしない」
- L0 節に L1 handoff 補足を追加: L1 が install 済みで読まれた後の
  振る舞い（read-only inspection → reuse-first 比較 →
  evidence-labeled 提案 → 承認なしでは永続化・実行しない）を
  1 段落で要約し、canonical は skill 本文と明記する。既存の
  「install 済みなら読む / 未配布では観測結果を報告」の fallback
  記述は維持する（skill 未 install の consumer では依然正しい）

### 4. `workflow-usage.md` 更新

「選択基準」節に既にある L0/L1 言及の直後、または catalog 節の
近傍に小さな discovery 節を追加: `cheaper-workflow-discover` が
本 guide の選択基準・claim 境界を適用する評価層であり、発見
（proposal）は作成・実行と別物である旨の相互参照のみ。判断基準の
複写はしない — canonical は本 guide の選択基準のまま。

## Data and Control Flow

```text
[L0 signal or user request]
  -> load cheaper-workflow-discover
  -> gather inputs (observed / user-stated / unknown)
  -> direct adequacy gate ----> direct-tool / direct-worker / not-recommended
  -> existing workers/routes (route check / route resolve, read-only)
  -> workflow list (catalog metadata)
  -> workflow interface <path> (fitting candidates only, public I/O)
  -> compare (a)-(e) on one scale + economics
  -> recommendation + alternatives + evidence labels
  -> [reuse] explicit path + interface | [create/modify]
     if cheaper-workflow-create installed -> offer on approval
     else -> report proposal (manual authoring / L2 path)
  -> nothing persisted, granted, or executed without approval
```

## Non-Functional Constraints

- 追加の常時コストゼロ: skill は L0/ユーザーが引いた時だけ全文が
  読まれる。per-turn の監視・scan・CLI 呼出しは規約としても禁止。
- 表現ドリフト防止: 規約正本は SKILL.md に一元化し、guide は
  要約 + pointer。CLI envelope の詳細説明は `cheaper-usage`・
  `usage.md` が canonical — skill は flag と result 形状の必要
  最小限のみ記す。
- 誠実性: 未測定の parent token 節約・存在しない user-global
  lookup・catalog による完全な重複証明を主張しない。
- 後方整合: 既存 skill・guide の他節・CLI の振る舞いは不変。

## Risks and Trade-offs

- `description` が長い → 許容。発火語彙と境界句を両立させるため
  で、truncating host では明示呼出し fallback が残る（#202 と同じ
  緩和）。
- skill が `workflow run` を示唆される誤読 → boundaries 節を先頭に
  置き、procedure 内の全 command を read-only に限定、output
  contract に免責文を必須化して対処。
- 未 merge の L2 への期待 → 全言及を "if installed" に限定し、未
  install 時は manual authoring を経路として残す。
- `usage`/`history` を親履歴と誤読 → boundaries と evidence 節の
  両方で「cheaper 自身の run のみ」を明示。

## Deferred Design Concerns

- proposal summary の永続 schema / CLI 経由 handoff | L2 (#204)
  が machine-readable を要求した場合 | 別 issue。本設計では概念形
  のまま
- L1 が収集する代表入力に対する `count-tokens` の粒度 | 実運用で
  必要性が見えた場合 | 現状は optional の measured input として
  記載済み
- runtime 実機での implicit firing | #205 の横断検証領域 | #202 の
  per-target matrix がそのまま適用される

## Verification Notes

- [x] SKILL.md が front matter + 7 節構成で存在（grep・目視）
- [x] skill が指示する全 command を実 binary で実行し flag・出力
      形状を確認（`workflow list` catalog 有/無・`workflow
      interface`・`route check`・`route resolve`・`usage`・
      `history list`・`count-tokens`・`models`）
- [x] guide 2 件の更新が要約 + pointer（規約複写なし）
- [x] `git diff --stat` が SKILL.md + guide 2 件 + doc chain のみ
- [x] `doc_status` audit blocking 0・`mise run check` 全 step pass

---
id: "SPEC-034EStsweRrzPq1Og6oCzl"
type: "spec"
status: "approved"
title: "tool-operator built-in Worker"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/102"
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
    - "cheaper-v1-worker-runtime-cli.md"
    - "./policy-gated-command-tool.md"
    - "./managed-process-canonical-tool.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bulk-reader-built-in-worker.md"
    - "./code-writer-and-doc-writer-built-in-workers.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/tool-operator-worker-built-in.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../guides/workers.md"
    - "../guides/command.md"
  defers: []
  deferred-by: []
---

# tool-operator built-in Worker

## What Should Be Built

目標に向かって tool call を反復する「operation actor」としての built-in Worker `tool-operator` を追加する。operation の実行 → 結果の解釈 → 次の operation → 圧縮された最終結果の返却、という loop を cheap model 側で回し、中間の巨大な tool 出力を親 context に流さない。

CLI alias `cheaper tool-operate` を `cheaper run tool-operator` と等価の入口として公開する。canonical invocation は `cheaper tool-operate --input <file>` または stdin への JSON 入力（`echo '{"task":"..."}' | cheaper tool-operate`）であり、alias は extra positional を受理しない（全 built-in worker と同じ入力規約 — args.rs の alias 解約が positional を reject する）。issue #102 の `cheaper tool-operate "<task>"` 表記は「input が task を運ぶ run」の shorthand であり、positional 引数を解釈する構文を意味しない。

責務分担は次の通り: bulk-reader = 大きな情報の読み取りと圧縮、code-writer / doc-writer = artifact の編集、tool-operator = 目標に向かって tool call を反復する操作 actor。`run`（generic worker 実行）、workflow、plan とは別の、単独の built-in 責務である。

宣言する tools は `read` / `grep` / `find` / `ls` / `count_tokens` / `process` / `command`。`write` / `edit` は含めない。

## Why Now

親 Coding Agent が複数 step の操作（build の実行、結果の確認、次の操作の選択）を委譲する手段が無い。bulk-reader は read-only で操作を実行できず、writer 系は artifact 編集が責務であり、いずれも「操作を反復して目標へ進む」actor ではない。結果、親は frontier model の context で操作 loop を自ら回し、中間の tool 出力ごとコストを支払っている。`command` tool（issue #101）の導入で policy-gated な操作実行の基盤が揃ったため、その consumer として `tool-operator` を定義する。

## Users and Value

- 親 Coding Agent: 複数 step の tool 操作を cheap model へ委譲し、raw な tool log ではなく evidence 付きの compact structured result だけを受け取れる。frontier model の context とコストを節約できる。
- trusted user: `command` の実行は既存の trusted grant 境界の内側に留まり、grant を持たない環境でも worker の挙動が一貫して予測可能（縮退実行または理由付き blocked）である。

## Scope

### In Scope

- built-in Worker 定義 `tool-operator`（worker markdown + YAML frontmatter + input/output JSON Schema）と CLI alias `tool-operate`、HELP・test の追加
- tools 宣言: `read` / `grep` / `find` / `ls` / `count_tokens` / `process` / `command`（`write` / `edit` は宣言しない）
- worker frontmatter への opt-in key `cheaper.optionalTools` の追加（ADR-034ESBcMTWxNCMDI9obdb8 に従う。worker markdown の追加ではなく core の validation 変更を伴う）
- input contract `{task (required), context?, constraints?, output hints?}` と output contract の論理 field: `status`（`success|partial|failed|blocked`）、`summary`、`findings[]`（各要素は evidence: file:line | commitId | exitCode | truncation を運ぶ）、`unresolved[]`、`nextActions[]`、および任意の top-level `evidence[]`（aggregate / tool-execution evidence）。具体的な JSON shape は bulk-reader spec の慣例に倣い design doc で確定する
- 親へ返す結果を compact structured result に限定し、evidence を file:line・commit id・tool exitCode・truncation 情報で表現する contract
- policy が `process` / `command` を許可しない環境での挙動（利用可能な tools での縮退実行、または理由付き早期終了 blocked のいずれか一貫した挙動）
- 既存の Rig adapter loop の再利用による実行（独自 agent loop・追加 agent engine は作らない）
- `execution.maxTurns` / `maxToolCalls` / `timeout` / `output.maxTokens` / command 出力 cap の遵守
- `meta.changes` の honest な報告、`cheaper usage` で計測可能な savings、`directlyComparable` の保守的扱い
- issue #102 の test case をカバーする dev-seam fixture と regression corpus
- 公開面の文書更新: README built-in workers 表、workers guide、command guide、usage guide、APM skill

### Out of Scope

- worker 内からの再帰的 worker delegation（worker が更に worker / run を起動する構造）
- long-running daemon・常駐 process・persistent artifact の生成
- unrestricted shell・interactive TTY
- 新しい agent engine・独自 agent loop の実装
- `write` / `edit` capability（writer 系 worker の scope を侵食しない）
- `command` による filesystem.deny / writeScope 契約の迂回 — `command` の子 process 副作用境界は policy-gated-command-tool spec の契約（grant が唯一の制約であり、FS deny / writeScope は bind しない）のままとする。issue #101 が固定した信頼境界は不変（grant の authoring は trusted layer のみ、call 経路は fail-closed）だが、scope 構築時の失敗 semantics に限り opt-in の例外が加わる — `cheaper.optionalTools` を宣言した worker は gate 不成立で未登録となった gated tool を `POLICY_VIOLATION` ではなく skip して縮退する（ADR-034ESBcMTWxNCMDI9obdb8）
- per-worker の execution 制限 override 機構（global 制限内に収める。不足が判明した場合は別 spec で扱う）
- `cheaper.optionalTools` 機構の内部実装詳細（機構の選択は ADR-034ESBcMTWxNCMDI9obdb8 で確定済み。本 spec が固定するのは外部から観測可能な挙動と frontmatter contract）

## Requirements

### worker 定義・alias

- built-in Worker `tool-operator` を既存 worker 定義機構（resource embedded の markdown + frontmatter + schema）で追加する。Runtime に worker 固有分岐を追加しない
- `cheaper tool-operate` は `cheaper run tool-operator` と等価に動作する。canonical invocation は `cheaper tool-operate --input <file>` または stdin の JSON（`{"task": "..."}`）であり、extra positional は受理しない
- model tier は既存 built-in worker と同じ `cheaper.model.tier` 宣言機構を使い、`tier: low`・`workload.outputPerInput: 0.3` とする（ADR-034ESBcMTWxNCMDI9obdb8 で確定）

### tools・責務境界

- 宣言 tools は `read` / `grep` / `find` / `ls` / `count_tokens` / `process` / `command` で固定とする（ADR-034ESBcMTWxNCMDI9obdb8 で確定。`process` / `command` は `cheaper.optionalTools: [process, command]` で opt-in 宣言する）
- `write` / `edit` は宣言しない。worker からこれらの tool への到達経路は存在せず、undeclared tool として拒否される
- scope 外への partial write を発生させない。tool-operator は write/edit を持たず writeScope を宣言しないため、worker 契約上の書き込み boundary は存在しない

### input / output contract

- input は `{task (required), context?, constraints?, output hints?}` の JSON task input で、宣言した input schema に従う
- output の論理 field は `status`（`success|partial|failed|blocked`）、`summary`、`findings[]`（各要素は evidence: file:line | commitId | exitCode | truncation を運ぶ）、`unresolved[]`、`nextActions[]`、および任意の top-level `evidence[]`（aggregate / tool-execution evidence）とし、宣言した output schema に適合する（schema validation は Runtime Invariant）。具体的な JSON shape は bulk-reader spec の慣例に倣い design doc で確定する
- `status` の意味論: `success` = 目標達成 + evidence 完備、`partial` = 一部達成 + 未達成分を `unresolved` に列挙、`failed` = 達成不能、`blocked` = 権限・制約により開始または継続不能。`blocked` は成功 run envelope 内の `result.status == "blocked"` を意味し、run 全体が失敗する error envelope（`POLICY_VIOLATION` 等）とは区別される。`tools.allow` で宣言 tool が除外された場合は従来通り run 全体が admission gate で fail する（これは設計意図であり、縮退対象ではない）
- `partial` / `failed` / `blocked` では `unresolved` と `nextActions` に次の手がかりを残す
- evidence は file:line・commit id・tool exitCode・truncation 情報で表現し、中間 tool 出力の生テキストを含めない

### 中間出力の圧縮

- 巨大な中間 tool 出力を親 context に surface しない。親へ返るのは compact structured result のみで、envelope サイズは bounded であること
- tool 出力が cap で truncate された場合、その事実を evidence として結果に含める

### `process` / `command` の認可

- `command` は trusted grant が存在し、かつ `tools.allow` が許可する場合にのみ利用可能とする
- policy が `process` / `command` を許可しない環境でも worker は usable でなければならない: 宣言 tools のうち利用可能なもので実行を継続するか、理由付きで早期終了（`blocked`）するかのいずれかを一貫して行う。中途半端な失敗や silent な権限昇格の試行は不可
- 実現機構は ADR-034ESBcMTWxNCMDI9obdb8 で確定済み（worker frontmatter の `cheaper.optionalTools` で opt-in した gated tool を、content-gate 不成立時に「未登録」へ縮退する方式）。本 spec が要求するのは外部から観測可能な挙動の一貫性であり、内部実装詳細は design 側の決定とする

### 実行制限・loop・計測

- `execution.maxTurns` / `maxToolCalls` / `timeout` / `output.maxTokens` / command 出力 cap を守る。制限到達時は規定の打ち切り挙動に従う
- 実行 loop は既存の Rig adapter loop を再利用する。独自 agent loop・追加の agent engine・worker 内からの再帰的 worker delegation は作らない
- `meta.changes` は実測した write/edit change のみを報告する。tool-operator は write/edit を宣言しないため `meta.changes` は常に空であり、`command` / `process` の子 process 副作用を audited change として主張しない（honest reporting）
- mutating tool（`process` / `command`）を宣言するため、`savings.directlyComparable` は tool-operator の run で常に false とする（child mutation が未観測のため estimated-savings 母数に参加させない保守的扱い）
- run record は既存の usage.jsonl 記録機構（`kind:"run"`、worker・model・usage・durationMs）で `cheaper usage` から計測可能でなければならない

## Acceptance Criteria

- [ ] AC1: `cheaper tool-operate --input <file>` および stdin の JSON（`{"task": "..."}`）が `cheaper run tool-operator` と等価に動作し、有効な入力で run できる
- [ ] AC2: 成功 run が `status: success`・summary・evidence 付き findings を含む compact structured result を、宣言した output schema に適合して返す
- [ ] AC3: partial / failed の run が `status`・`unresolved`・`nextActions` を返す
- [ ] AC4: `command`（`process`）が policy で許可されない環境で、worker が利用可能な tools で縮退実行するか理由付きの `blocked`（成功 run envelope 内の `result.status == "blocked"`。run-level `POLICY_VIOLATION` の error envelope とは区別される）を返す — いずれか一貫した挙動を示し、中途半端な失敗・権限昇格試行が無いことを検証する test が通る（既定挙動は ADR-034ESBcMTWxNCMDI9obdb8 の `cheaper.optionalTools` 縮退機構に従う）。`tools.allow` による宣言 tool の除外は縮退対象ではなく、従来通り run 全体を fail させること
- [ ] AC5: 巨大な中間 tool 出力を発生させる run で、親へ返る envelope に中間出力が含まれず、envelope の `result` が宣言した output schema に適合し `output.maxTokens` 以内で生成される
- [ ] AC6: `execution.maxTurns` / `maxToolCalls` / `timeout` / 出力 cap が遵守され、超過時に規定の打ち切りとなる test が通る
- [ ] AC7: `write` / `edit` が宣言されず、worker から利用不可（undeclared）であることが検証される
- [ ] AC8: policy が許可しない `process` / `command` が worker へ公開されず、権限なしに実行されないことが検証される
- [ ] AC9: `command` 経由の実行を含む run で、worker 契約上の書き込み boundary が迂回されず、`meta.changes` が実測 write/edit（= 空）のみを正直に報告する
- [ ] AC10: run record が usage.jsonl に記録され `cheaper usage` で集計可能、`savings.directlyComparable=false` が付く
- [ ] AC11: issue #102 の test case が dev-seam fixture と regression corpus でカバーされる
- [ ] AC12: README built-in workers 表・`docs/guides/workers.md`・`docs/guides/command.md`・`docs/guides/usage.md`・APM skill が更新される
- [ ] AC13: `mise run check` が pass する

## Deferred Design Concerns

以下の deferred concern は ADR-034ESBcMTWxNCMDI9obdb8（accepted）の決定で解決済み:

- `process` / `command` 未許可時挙動の実現機構 → **解決済み**: worker frontmatter の `cheaper.optionalTools` による per-tool opt-in 縮退機構（Option C1）を採用。`process` は `command` とともに初期 tools 集合に含め、`optionalTools: [process, command]` で縮退可能として宣言する
- per-worker execution 制限 → **解決済み**: override 機構は設けず global policy 制限（`execution.maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000`）に従う
- model tier と `workload.outputPerInput` → **解決済み**: `tier: low`・`workload.outputPerInput: 0.3`
- worker 別 APM skill `cheaper-tool-operate` → **解決済み**: `packages/apm` に追加する
- `ollama_e2e.rs` の実 provider 検証 → **解決済み**: 本 iteration の対象外（dev-seam fixture + regression corpus が primary 検証）

引き続き deferred:

- `cheaper tool-operate "<task>"` 形の positional-sugar（alias が task positional を解釈して input JSON へ詰め替える parser 変更）| 理由: 既存 alias は extra positional を reject する規約であり、canonical `--input` / stdin 経路で要件を満たせる | 再着手: positional 入力の需要が実測された場合、parser 規約ごと別 spec で検討
- execution 制限（global `maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000`）が multi-step 反復に実用上足りるかの実測 | per-worker override が要ると判明した場合は strict `cheaper` frontmatter keys の拡張 = core 変更として別 spec へ | 実装・benchmark で実測後に判断

## Sources and Relations

- source: GitHub issue #102（worker 責務・tools 集合・input/output contract・evidence 表現・認可時挙動・test case・acceptance の正本）
- derives-from: `../discovery/tool-operator-worker-built-in.md`（DISC-034ESpIy7zAOucpwBxmjfd）。コードベース探索結果（worker 定義機構・alias 追加面・fail-closed gate・execution 制限・計測面の既存整合）と Option A–D の比較表を内包する
- depends-on: `cheaper-v1-worker-runtime-cli.md`（worker runtime contract）、`./policy-gated-command-tool.md`（`command` の grant / trust boundary の正本）、`./managed-process-canonical-tool.md`（`process` の contract 正本）
- related: `./bulk-reader-built-in-worker.md`・`./code-writer-and-doc-writer-built-in-workers.md`（責務分担の兄弟 worker spec）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（command の grant 信頼境界 decision）
- references: `../guides/workers.md`・`../guides/command.md`（更新対象の user-facing docs）

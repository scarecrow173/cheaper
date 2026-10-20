---
id: "DISC-034J7LwMEdPR5HxFkgOivZ"
type: "discovery"
status: "draft"
title: "PreToolUse auto-routing to tasks and workflows"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/142"
    - "https://github.com/scarecrow173/cheaper/issues/143"
    - "https://github.com/scarecrow173/cheaper/issues/144"
    - "https://github.com/scarecrow173/cheaper/issues/145"
    - "https://github.com/scarecrow173/cheaper/issues/146"
    - "https://github.com/scarecrow173/cheaper/issues/147"
    - "https://github.com/scarecrow173/cheaper/issues/148"
    - "https://github.com/scarecrow173/cheaper/issues/149"
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/grant.rs"
    - "crates/cheaper-cli/src/policy.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/artifact.rs"
    - "crates/cheaper-workflow/src/engine.rs"
    - "crates/cheaper-workflow/src/schema.rs"
  defers: []
  deferred-by: []
---

# PreToolUse auto-routing to tasks and workflows

## Exploration Goal

cheaper の delegation は現状 manual opt-in(親 agent が `cheaper run` /
worker alias を明示呼出する形)に依存しており、実測データで adoption
が極端に低いことが判明した。一方で親 agent の shell command 実行は
巨大な tool output を frontier model の context へそのまま流入させ
続けている。本 discovery が扱う問い: **PreToolUse hook が shell
command を登録済み Task/Workflow へ自動 routing する仕組み —
route 定義・認可境界・実行経路・ログ参照・計測をどう構成するか**
(起点は parent issue #142)。

対象範囲は、route 宣言(`routes.yaml`)と認可(`command.grants`)の
分離、workflow を主実行経路とする構成、rewrite-safe gate、route log
store と scoped read、`kind:"route"` usage record、hook target 別の
rewrite 戦略。設計方針の大半は issue #142 で確定済みであり、本書は
それらを確定入力として記録し、代替案の却下理由・未確定点・昇華先
(spec/ADR/design)を整理する探索記録である。最終決定は ADR、要件は
spec に委ねる。

## Key Issues and Assumptions

### 実測データ(manual opt-in の限界 — confirmed)

- **Devin `sessions.db`(5 週間・238 sessions)**: tool result と
  して frontier context へ流入した bytes は合計 **1.43GB**。内訳は
  exec 出力 **262MB**、edit echo **758MB**、read **362MB**。
  同一 file の冗長な再 read が **8,686 件**観測されている。
- **Codex rollouts(4.3 か月・1,073 sessions)**: 総 input は
  **1.77B tokens**。**>10KB の tool output が 184MB = 全 output
  bytes の 64%** を占め、>10KB output を含む session は
  **788/1,073**。この期間の cheaper の実 invocation は **0 件**。
- **cheaper adoption <1%**: hook が発火したのは Devin 238 session 中
  **12 session** のみ。ただしその 12 session では **13.6M tokens**
  の context 流入を実際に block しており、効果自体は実証済み —
  致命的なのは coverage である。worker failure rate は **31%**、
  `usage.jsonl` は **76% が test pollution**。
- **帰結(confirmed)**: manual opt-in では巨大 output の long tail
  を拾えない。巨大 output が発生する時点で自動的に delegation へ
  routing する hook 機構が必須。

### 確定済み設計入力(issue #142 — 全て settled)

以下は issue #142 で確定した設計判断であり、本 discovery では変更
対象ではなく入力制約として扱い、そのまま spec/ADR へ引き継ぐ。

1. **Workflow が主実行経路** — 専用の `kind:exec` orchestrator は
   作らない。routed 実行は登録済み workflow の起動として表現する。
2. **routing ≠ authorization** — routes は宣言のみ、認可は
   `command.grants` が担う。project は routes を定義できても
   grants を生成できない。grant 不成立なら non-runnable → rewrite
   せず従来挙動へ fallback する。
3. **rewrite-safe gate は single command のみ** — pipe/chain/
   redirect/metachar を含む command は rewrite 対象外。判定は既存の
   `tokenize_shell`(`crates/cheaper-cli/src/hook/routing_policy.rs:1093`
   周辺、issue #42 / ADR-034rBhRB7QyWkAxRiTDXXq 由来)を再利用する。
4. **`route run` の exit code = 元 command の exit code** —
   post-processing(capture・token 計測・worker 呼出)が失敗しても
   command を再実行しない。
5. **`route resolve` は dry-run** — viability check(実 argv に対
   する grant match、target の存在、cwd)は `route resolve` で行い、
   `route run` 側の runtime recheck とは分離する。
6. **workflow `when` は汎用 conditional job semantics** —
   deterministic で LLM を介さない。skip ≠ failure とし、
   dependents/output materialization の規則を定義する。job が skip
   されても workflow は succeed しうる。schema は `$input`/`$from`
   と衝突しないこと。
7. **command tool `exitCodes` は opt-in** — 非 0 exit を result
   data として返すのは opt-in のみ。spawn/timeout/abort は従来どおり
   Err のまま。
8. **Log store は `<userConfigDir>/route-logs/` が候補** — secret
   を含みうるため workspace 外。読み出しは ArtifactReader /
   canonical read に揃えた scoped log-read 機構とし、汎用的な
   workspace 外 file access は開けない。`count_tokens` /
   bulk-reader / `route logs` は 1 つの参照 contract を共有する。
   truncation は flag で明示、streaming write、size/count/TTL の
   retention を持つ。
9. **Env passthrough は明示 allowlist のみ**(`passthroughEnv`)—
   stdin/TTY 依存の command は対象外。hook env ≠ shell env の
   caveat を明記する。
10. **usage record は `kind:"route"` を独立 contract で追加** —
    savings contract とは分離し、originalTokens/returnedTokens/
    reductionRatio/worker usage+duration/refetches/
    postprocessFailures を保持する。
11. **Hook rewrite target は Claude + Devin を先行** —
    `updatedInput` のみを使い `permissionDecision` は書かない
    (上流の approval を温存)。Codex/Cursor/Copilot は
    approval-bypass 検証のため defer(issue #150)。windsurf は
    rewrite 非対応のため Allow fallback。
12. **dedup は verdict sharing のみ** — `VariantInputs` に
    `routes.yaml` hash を追加する。command の二重実行防止ではない。
13. **Trust: project routes が user routes を override できるのは
    明示 trust 規則の下のみ**(詳細は ADR で確定)。builtin routes
    は auto-granted な grants 無しで出荷する。
14. **実行互換** — cwd は hook payload の cwd で復元、env は
    allowlist、timeout は clamp、signal/process-tree 伝播、非 0
    exit の透過性を維持する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (A) Workflow を主実行経路とし、route が登録済み workflow を起動する | tool/engine 機構を再利用できる。`when`・capture・token 計測を workflow の primitive として user workflow にも開放できる | workflow schema に `when`/`exitCodes` 等の拡張が必要 | yes(#142 で確定) |
| (B) 専用 `kind:exec` orchestrator を新設する | routed 実行専用に最適化できる | workflow/tool 機構の重複実装になる。ユーザーの directive でも却下済み | no(rejected) |
| (C) caller 側 two-phase orchestration(workflow `when` の代わりに呼出側が段階制御する) | workflow schema を変えずに済む | 事実上の orchestrator が caller 側に分散し、user workflow からは表現不能になる | no(rejected) |
| (D) 汎用 workspace 外 file grant で log を読む | 実装が単純 | 境界が広すぎる。ArtifactReader 整合の scoped log-read 機構で代替する | no(rejected) |
| (E) `permissionDecision:"allow"` + `updatedInput` を全 target に一律適用する | target 毎の分岐が不要 | 上流 approval を bypass する(Codex は rewrite に allow が必須で approval を素通しする) | no(rejected) |
| (F) route precedence を単純 last-wins にする | 実装が単純 | project が user の grants を使って workflow を黙って差し替え可能になる | no(rejected) |

## Tentative Conclusions and Hypotheses

- **routing pipeline 構成(confirmed — issue #142 で確定)**:
  `PreToolUse` → Route Resolver → 登録済み Task/Workflow → 親
  エージェント。Route は argv-prefix で解決し、shell command を
  `cheaper route run` へ rewrite、`route run` が登録済み workflow
  (command tool + capture + token count + conditional worker)を
  起動し、compact result + log reference を親へ返す。
- **宣言と認可の分離が安全弁(confirmed)**: route 宣言は
  project/user/builtin いずれにも置きうるが、実行可否は
  `command.grants` のみが決める。project 由来の route 持ち込みは
  それだけでは実行不能であり、rewrite されず legacy 挙動に落ちる。
- **rewrite は fail-safe(confirmed)**: tokenize 不能・複合
  command・grant 不成立・viability 不全は全て「rewrite しない =
  従来挙動」に倒す。deny ではなく passthrough で、routing 層の
  不具合が親の command 実行を壊さない。
- **workflow `when`・command `exitCodes`・scoped log-read は本機能
  の前提 primitive として workflow/tool 層へ昇華する(hypothesis —
  詳細規則は spec/ADR 側で確定)**。
- **`kind:"route"` usage record で routing の削減実績を `cheaper
  usage` 系から観測可能にする(confirmed 方向 — schema 詳細は
  spec)**。

## Open Questions

- `when` で skip された job の dependency 伝播規則と output
  materialization の詳細(dependents への引き渡し・失敗扱い境界)。
- scoped log-read 機構の正確な形 — `artifact://` URI 拡張か、
  invocation-scoped read grant か。
- `route run` が workflow を in-process で起動するか subprocess で
  起動するか。
- project route override の trust mechanics — 明示 trust 宣言の形
  (per-route? 宣言 file?)を ADR で確定する。
- Claude `updatedInput` silent-ignore bug(#81340)の version 検出
  方法と doctor verb の扱い。
- builtin routes を enabled で出荷するか opt-in で出荷するか。
- `exitCodes` schema の形(`[0]`/any/list)と、grant-level vs
  task-level のどちらに置くか。
- log retention の default 値、および巨大 log の token 計測方法
  (streaming か cap か)。

## Promotion Candidates

- [ ] spec-doc needed: route 定義(`routes.yaml` schema・argv-prefix
  解決・precedence/trust)、`route resolve`/`route run` の CLI
  contract、`when` conditional job semantics、command `exitCodes`、
  log store + scoped read、`kind:"route"` usage record、hook
  rewrite 応答(target 別)を formalize する
- [ ] adr-doc needed: routing≠authorization の認可境界、workflow
  主実行経路(orchestrator 不採用)、trust 規則、in-process vs
  subprocess、`updatedInput` のみの rewrite 方針を記録する
- [ ] design-doc needed: Route Resolver・rewrite-safe gate・route
  log store・workflow `when`・dedup(VariantInputs)の実装構造

## Sources and Relations

### Issue map(GitHub — parent #142 と実装分割)

- parent: issue #142
  <https://github.com/scarecrow173/cheaper/issues/142>
- #143 Route Resolver + `routes.yaml` + `route resolve`
  <https://github.com/scarecrow173/cheaper/issues/143>
- #144 command tool `exitCodes`
  <https://github.com/scarecrow173/cheaper/issues/144>
- #145 log store + scoped read
  <https://github.com/scarecrow173/cheaper/issues/145>
- #146 workflow `when`
  <https://github.com/scarecrow173/cheaper/issues/146>
- #147 `route run` + routed-exec workflow(deps: #143–#146)
  <https://github.com/scarecrow173/cheaper/issues/147>
- #148 hook Rewrite + Claude/Devin + dedup(deps: #143, #147)
  <https://github.com/scarecrow173/cheaper/issues/148>
- #149 usage `kind:"route"` + `route logs` + retention + docs
  (deps: #145, #147)
  <https://github.com/scarecrow173/cheaper/issues/149>
- #150 Codex/Cursor/Copilot rewrite(deferred、dep: #148)
  <https://github.com/scarecrow173/cheaper/issues/150>

実装順序: **#143 → #144/#145/#146 → #147 → #148 → #149 → #150**。

### 関連 doc(`related` 参照 — 本 chain の lineage ではない)

- `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  (SPEC-034H2WAU6TQG6N9dmmmKNA)— 既存 PreToolUse hook の bounded
  evaluation と登録 lifecycle。rewrite 機構はこの基盤の拡張。
- `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  (ADR-034H2WBNtC4We8dyC5lQZJ)— hook 登録 lifecycle の CLI
  ownership。route rewrite の deploy 形態はこの境界に従う。
- 補足の文脈(front matter link は張らず prose のみ):
  `docs/discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md`
  (dedup/fingerprint の前提)、
  `docs/discovery/invocation-scoped-command-grants.md`
  (command grants の前提)、
  `docs/discovery/usage-and-savings-telemetry-for-delegation-reduction.md`
  (usage record の前提)。

### code evidence

- `crates/cheaper-cli/src/hook/routing_policy.rs` — `tokenize_shell`
  (:1093、rewrite-safe gate の再利用対象)、`RoutingRequest`/`Verdict`
  (:80/:116)、`FingerprintContext`(:360、dedup の fingerprint 構成)
- `crates/cheaper-cli/src/hook/targets.rs` — per-target normalize/
  render(devin :527/:545、claude/codex pretooluse :484-、copilot
  :193-)。Claude `updatedInput` は未実装で本機能で追加
- `crates/cheaper-cli/src/hook/eval_cache.rs` — verdict dedup の
  cache 経路(`VariantInputs` 拡張の挿入点)
- `crates/cheaper-cli/src/usage_log.rs` — `kind:"run"`(:121,138)/
  `kind:"count"`(:107)の record 構築。`kind:"route"` の追加先
- `crates/cheaper-cli/src/grant.rs`・`crates/cheaper-cli/src/policy.rs`
  — `command.grants` の認可判定経路
- `crates/cheaper-tools/src/tools/command/mod.rs` — command tool
  (`exitCode` detail :86-105)。`exitCodes` opt-in の実装面
- `crates/cheaper-tools/src/artifact.rs` — ArtifactReader。
  scoped log-read の揃え先
- `crates/cheaper-workflow/src/engine.rs`・`schema.rs` — workflow
  engine/schema。`when` conditional job の拡張面

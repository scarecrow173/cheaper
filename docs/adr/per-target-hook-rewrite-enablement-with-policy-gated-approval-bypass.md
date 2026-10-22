---
id: "ADR-034KrYGRHZuvOIx1d0S0nE"
type: "adr"
status: "accepted"
title: "Per-target hook rewrite enablement with policy-gated approval bypass"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
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
  depends-on:
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "./command-grant-trust-boundary-and-managed-process-reuse.md"
    - "./invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "./copilot-adapter-separate-script-never-nonzero-exit.md"
    - "./apm-hook-per-target-package-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/codex-cursor-copilot-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/devin-pretooluse-rewrite-runtime-verification.md"
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/mod.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "packages/apm-hook-codex/.apm/hooks/hooks.json"
    - "packages/apm-hook-cursor/.apm/hooks/hooks.json"
    - "packages/apm-hook-copilot/.apm/hooks/hooks.json"
    - "https://developers.openai.com/codex/hooks"
    - "https://github.com/openai/codex/pull/20527"
    - "https://cursor.com/docs/hooks"
    - "https://docs.github.com/en/copilot/reference/hooks-configuration"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Per-target hook rewrite enablement with policy-gated approval bypass

## Context and Problem Statement

issue #148 で導入した `Verdict::Rewrite{command, fallback}` は
claude (`pretooluse`) と devin のみ `updatedInput` を emit し、
codex / cursor / copilot は `fallback` verdict を render する
(ADR-034J8eZHNPqARi25HLFOL6 D3 — 「approval-bypass 検証が必要」
で defer)。issue #150 はこの 3 target の有効化を要求する。

本 session の codex-cli 0.162.1 実機検証
(DISC-034KrLM3o2H1kMylQFlg2U)で確定:

- codex の `updatedInput` は `permissionDecision:"allow"` 併記時に
  のみ適用される (単独・`"ask"` 併記では無視 — 原 command がその
  まま実行パスへ)。**bypassing emit が構造的に必須**。
- hook の `allow` は approval prompt を素通しするが codex
  execpolicy は迂回しない (rewrite 後 command も `blocked by
  policy` 評価を受ける)。
- codex の PreToolUse payload は claude と同一 shape
  (`tool_name`/`tool_input`) — 既存 `pretooluse_normalize` で正規化
  可能、登録 event・matcher の変更は不要。

cursor / copilot は CLI が本環境に存在せず emit contract が未検証
(cursor: `preToolUse` event 移行が必須、`updated_input`/
`updatedInput` と `permission` 併記要否が不確定。copilot:
`modifiedArgs`/`updatedInput` の実効 field と `"ask"` の approval
意味が不確定)。issue の文言規則「実機検証で確認できない挙動は
仕様上の保証として扱わない」に従い、emit shape の確定を実機検証
task へ gate する設計が必要。

残る設計問題は 2 つ: (1) approval を素通しする emit を有効化する
opt-in の配置・粒度・trust 境界、(2) 未検証 emit を flag で
gate しつつ、検証済み emit を段階的に解放する構成。

## Decision Drivers

* **approval bypass は user の明示 opt-in**: `permissionDecision:
  "allow"` 併記は agent の approval prompt を素通しする — user の
  同意なしに emit してはならない (ADR-034J8eZHNPqARi25HLFOL6 D3 で
  「`permissionDecision:"allow"` 併記は approval bypass のため
  不可」と却下済みの経緯を、user opt-in 条件付きで解放する形)。
* **untrusted project が bypass を自己宣言できない構造保証**:
  project 層 (`<workspace>/.cheaper/`) は checkout 可能=untrusted。
  `command.grants` が project partial schema から section ごと除外
  されているのと同型に、bypassing emit の許可も trusted 層
  (builtin/user) のみが authoring できる必要がある
  (ADR-034EMgaatFhsXjR97kH14c の trust 境界を継承)。
* **検証済みと未検証の emit を分離する staged enablement**: codex
  の emit shape は実機確定済み、cursor/copilot は未検証 — 未検証
  shape を flag 有りでも emit しない構造にしないと issue の文言
  規則に違反する。
* **render 層に差分を閉じる**: `Verdict::Rewrite` は `fallback`
  verdict を保持し、flag 不在時は全 target で現行挙動に完全に
  戻る。verdict 生成・eval・dedup・fail-open 規約は変更しない
  (R14-R17 の不変条件)。
* **reuse over duplication**: flag の load は既存の firing あたり
  1 回の `RouteSupply` (`load_or_none`) に乗せ、policy 3 層
  schema (complete/partial/project-partial) の trust model をその
  まま使う。新しい config surface を増やさない。

## Considered Options

* Option A — `policy.yaml` 新 `route` section に
  `route.rewrite.approvalBypass` (target 名 list) として追加し、
  project partial schema から `route` を除外する。
* Option B — `routes.yaml` top-level `rewrite` key として追加する。
* Option C — 各 route 宣言へ `rewrite.approvalBypass` field を
  追加する (per-route 粒度)。
* Option D — env var (`CHEAPER_HOOK_REWRITE_APPROVAL_BYPASS` 等) で
  opt-in する。

## Decision Outcome

Chosen option: "Option A — policy `route` section に
`route.rewrite.approvalBypass` (target 名 list)", because:

- flag の意味は route ではなく **target の応答 contract**
  (approval を素通しする emit を user が許可したか) — per-route
  配置 (Option C) は粒度が誤りで、untrusted project route が
  新規 command に flag を付けられる supply-chain hole になる。
- `routes.yaml` top-level (Option B) は file 単位の trust が無く、
  project routes.yaml が untrusted load される構造では flag を
  trusted 層限定にする merge 規則が複雑化する — 現行 merge は
  per-route のみ (routes.rs:91)。
- env var (Option D) は ambient で audit 性がなく、policy.yaml が
  security-sensitive 設定の canonical surface である既存設計と
  不整合。trusted 層限定の構造保証も得られない。
- Option A は `command` section を project partial schema から
  除外する先例 (schema.rs:289 — 「将来 field が増えても section
  丸ごと除外なら自動的に project から閉じる」fail-closed 規約)
  をそのまま再利用し、`RouteSupply` が既に firing ごとに policy
  を load 済みのため render への供給経路が新規 I/O なしで構成
  できる。

### 個別決定

#### D1 — `route.rewrite.approvalBypass` は target 名 list (per-target 粒度)

schema: `policy.yaml` に `route` object を追加し、
`route.rewrite.approvalBypass` を string array とする。値 domain
は bypassing emit を持ちうる target 名 (`codex`/`cursor`/`copilot`)
— 未知名・非配列・非文字列 item は `InvalidRuntimePolicy` で
fail-loud。default `[]`。`RuntimePolicy` に `route` section の
normalized 表現を追加し、`RouteSupply` → `handle_hook` →
render へ target list を渡す(`eval_input`/`RouteEvalInput` は
不変 — 供給は `evaluate_with_routes` の戻り値経由、D5/R14)。

per-target list を選ぶ理由: bypass の同意は target 単位で意味を
持ち (codex は bypass が構造必須、cursor/copilot は emit 確定
後に bypassing かどうかが分かる)、global boolean では未検証
target を巻き込んで一律 bypass になる。

#### D2 — codex emit は `permissionDecision:"allow"` + `updatedInput` 併記のみ

codex render の bypassing emit は
`{"hookSpecificOutput":{"hookEventName":"PreToolUse",
"permissionDecision":"allow","updatedInput":{"command":<rewrite>}}}`
+ exit 0 に固定する。`updatedInput` 単独・`"ask"` 併記は codex が
実機で無視することが確認済みのため、emit してはならない — 誤 emit
は「rewrite したつもりで原 command が走る」silent no-op に
なる。flag に codex が無い場合は従来どおり `fallback` verdict を
render する (deny → `hookSpecificOutput.permissionDecision:"deny"`、
allow → silent exit 0)。

#### D3 — cursor / copilot の emit は実機検証 gated

cursor の `preToolUse` event 登録 (`packages/apm-hook-cursor/.apm/
hooks/hooks.json` への追加) と normalize の追加は行うが、emit
shape (`updated_input` か `updatedInput` か、`permission` 併記
要否) は実機検証で確定するまで emit しない — flag に cursor が
あっても未検証 shape は fallback render のまま。copilot も同様に
`modifiedArgs`/`updatedInput` の実効 field と `permissionDecision`
併記値を実機検証で確定してから flag gated emit を有効化する。
「検証済みで approval を温存する emit (例: copilot の `ask`
併記が機能すると判明した場合) は flag 不要で後続有効化できる」
余地を残すが、本 issue で新規に無条件化する emit は無い。

#### D4 — cursor event 移行は `preToolUse` 追加で行い、旧 event の
撤去は検証結果依存

現行 `beforeReadFile`/`beforeShellExecution` 登録は cursor が
`preToolUse` で file-read / shell 両方を emit するかの検証結果に
依存する。`preToolUse` が両方をカバーすれば旧登録は撤去して一本化
する (二重 firing は dedup が吸収するが登録面の複雑さは残る)。
`preToolUse` の payload shape が cursor 固有 (`command`/
`file_path` top-level) か claude 型 (`tool_name`/`tool_input`)
かは payload 実測で確定する。

#### D5 — flag の供給経路は `RouteSupply` → render 層 (verdict 不変)

flag は `RouteSupply` (supply.rs:28 — firing ごと 1 回 load) が
保持する effective policy から読み、`handle_hook` が render fn へ
渡す。`Verdict` enum・`evaluate_route_arm` の Rewrite 生成条件・
`eval_cache` の dedup 規約 (Rewrite は persist/serve しない) は
変更しない。`load_or_none` 失敗時は flag 未供給 = bypass 無効
(conservative default) で、flag 読み取り失敗の新 error 経路は
増やさない。

#### D6 — 命名は `route.rewrite.approvalBypass` (issue 案 `allowApprovalBypass` からの確定)

issue が示唆した `route.rewrite.allowApprovalBypass` の `-like`
の範囲で、`approvalBypass` (名詞 list) を採用する — `allow`
prefix が boolean 印象を与えるのに対し、実態は target 名 list で
あり、値 domain を list とした命名の方が schema と整合する。
`route` section の prefix は route 機能群の config 面として既存
`routes.yaml` と語彙を揃える (policy 層の route 設定である点を
routes 宣言そのものと区別)。

### Consequences

* Good, because codex の emit が即座に実装可能になり (shape 実機
  確定済み)、cursor/copilot は検証 gated で flag 構造が既に
  あるため emit shape 確定後の追加が小さい。
* Good, because project 層から `route` を除外する構造保証で
  untrusted checkout が approval bypass を有効化できない
  (`command` grants と同型の trust 境界継承)。
* Good, because flag 不在時の挙動が現行と完全同一で、導入自体が
  既存 contract test を破壊しない。
* Bad, because policy schema の 3 mode (complete/partial/
  project-partial) と `RuntimePolicy`・`normalize_policy` に
  `route` section の追加変更が必要 — schema 面の diff が
  やや大きい。
* Bad, because cursor の `preToolUse` 登録は APM hooks.json の
  変更を伴い、consumer 側の再 deploy が必要になる
  (hook-config の managed lifecycle は devin のみ — cursor は
  apm install 経由)。

### Confirmation

* `route.rewrite.approvalBypass` の schema/load test — builtin/
  user 層で受理、project 層で `route` key 拒否、値 domain 外を
  `InvalidRuntimePolicy` で reject。
* codex render contract test — flag 有りで
  `permissionDecision:"allow"` + `updatedInput.command` 併記 emit、
  flag 無しで fallback render (`route_run.rs` の
  `devin_hook_rewrite` precedent)。
* cursor/copilot は emit 確定後に同型の contract test で固定。
* 実機検証記録 — `DISC-034KrLM3o2H1kMylQFlg2U` の probe 結果
  (codex 3 pattern) を証跡として保持。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-core/src/policy/schema.rs` — `route` section を
    complete/partial schema へ追加、project partial から除外、
    `RuntimePolicy` に normalized 表現を追加。
  - `crates/cheaper-core/src/policy/load.rs` — merge 経路は既存
    layer 規約で追加変更不要 (route は Replace merge に乗る)。
  - `crates/cheaper-cli/src/route/supply.rs` — 変更不要。
    `RouteSupply.policy` が effective `RuntimePolicy` を保持
    済みで、flag は `evaluate_with_routes` の戻り値拡張
    (`(Verdict, Option<RouteSupply>)`) 経由で render 層へ渡る
    (`RouteEvalInput`/`eval_input` は不変 — R14、D5)。
  - `crates/cheaper-cli/src/hook/targets.rs` — `handle_hook` が
    flag を render へ渡し、`codex_render` に bypassing emit 分岐を
    追加。cursor/copilot の render は emit 確定後に追加 (現在は
    fallback のまま)。
  - `packages/apm-hook-cursor/.apm/hooks/hooks.json` —
    `preToolUse` entry 追加 (旧 event の併存/撤去は検証依存)。
  - `crates/cheaper-cli/src/hook/mod.rs` — `KNOWN_EVENTS` は
    `pre-tool-use` を既に含むため変更不要 (cursor への event 名
    mapping は normalize 側で吸収)。
* Patterns to follow: 既存 `hook_specific_output_*` emit の
  byte 互換 JSON 規約 (`stringify_ordered`)、`command` section の
  project partial schema 除外先例、render は verdict のみで副作用
  を持たない現行構造。
* Patterns to avoid: bypassing emit を flag 無しで発行する、
  `updatedInput` を `permissionDecision` 無し・`"ask"` 併記で
  codex に発行する (codex が無視して silent no-op)、env var での
  flag override、routes.yaml への flag 配置。
* Dependencies or configuration: policy.yaml schema 拡張
  (`route` section)、user-facing doc 更新 (記法・trust 境界・
  検証状況)。

## Verification

* [ ] schema test — `route` section が complete/partial で受理、
  project partial で拒否、値 domain 外が `InvalidRuntimePolicy`。
* [ ] codex contract test — flag 有りで `permissionDecision:
  "allow"` + `updatedInput.command` emit、flag 無しで fallback。
* [ ] cursor/copilot — emit 確定後に contract test で固定、未検証
  状態では flag の有無にかかわらず fallback。
* [ ] fail-open — route supply load 失敗で bypass 無効。
* [ ] `cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で全 contract が
  pass — ただし `RuntimePolicy.route` の serialize 追加に伴い
  `expected.policy` を持つ golden fixture
  (`policy/{load,restrict,schema}.json`、
  `crates/cheaper-core/tests/golden/fixtures/`) の再生成と
  `RuntimePolicy{…}` literal 構築 site への `route` field 追記を
  伴う(無変更で pass するのは実行時挙動のみ)。
* [ ] `node scripts/check-apm-payload.mjs` — cursor hooks.json 変更
  が slim payload check に整合。
* [ ] 実機検証 (codex emit が実機で適用されることは本 session で
  確認済み — DISC の probe 証跡参照)。

## Pros and Cons of the Options

### Option A — `policy.yaml` `route` section + project 除外

* Good, because trust 境界が `command` grants と同型で既存の
  schema 機構を再利用できる。
* Good, because `RouteSupply` が firing ごとに policy を load 済み
  で render 供給経路が新規 I/O なしで構成できる。
* Bad, because policy schema 3 mode + `RuntimePolicy` +
  `normalize_policy` の追加変更が必要。

### Option B — `routes.yaml` top-level `rewrite` key

* Good, because route 文脈に近い。
* Bad, because file 単位の trust が無く project routes.yaml が
  untrusted load — flag を trusted 層限定にする merge 規則が
  複雑化する (現行 merge は per-route)。

### Option C — per-route `rewrite.approvalBypass` field

* Good, because route 粒度の制御に見える。
* Bad, because flag の意味は target の応答 contract であり route
  ではない — 粒度が誤り。untrusted project route が新規 command
  に付けられる supply-chain hole。

### Option D — env var

* Good, because schema 変更不要。
* Bad, because ambient で audit 性がなく、policy.yaml が
  security-sensitive 設定の canonical surface である既存設計と
  不整合。trusted 層限定の構造保証が得られない。

## More Information

- issue: <https://github.com/scarecrow173/cheaper/issues/150>
- 実機検証証跡: `docs/discovery/codex-cursor-copilot-rewrite-enablement.md`
  (DISC-034KrLM3o2H1kMylQFlg2U — codex 3 pattern probe、隔離
  CODEX_HOME の再現環境)。
- parent ADR の defer 経緯:
  `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  (ADR-034J8eZHNPqARi25HLFOL6 D3 — `permissionDecision:"allow"`
  併記は approval bypass で不可、3 target を issue #150 へ defer)。
- trust 境界先例: `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`
  (ADR-034EMgaatFhsXjR97kH14c — `command` grants が trusted 層
  のみ)、`docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`。
- copilot 規約先例: `docs/adr/copilot-adapter-separate-script-never-nonzero-exit.md`
  (非 0 = deny)。
- per-target package 構造: `docs/adr/apm-hook-per-target-package-split.md`
  (hook 宣言は target 別 sibling package — cursor hooks.json 変更
  の deploy 経路)。

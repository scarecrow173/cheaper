---
id: "ADR-034KLgkao8SgWWKIIsCqaB"
type: "adr"
status: "accepted"
title: "opt-in security.execDenyPatterns profile — A9 の降格実装"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
    - "../discovery/opt-in-exec-deny-patterns.md"
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
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../designs/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/opt-in-exec-deny-patterns.md"
  derived-by:
    - "../designs/opt-in-exec-deny-patterns.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/exec-contract-content-audit-removal.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../specs/exec-contract-content-audit-removal.md"
    - "../discovery/opt-in-exec-deny-patterns.md"
    - "../discovery/exec-contract-content-audit-removal.md"
    - "../discovery/permission-model-responsibility-redesign.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# opt-in security.execDenyPatterns profile — A9 の降格実装

## Context and Problem Statement

#163 で mandatory 層から除去された A9 executable deny list（旧
`command_denied_executable_re`）を、issue #164 で operator opt-in の
policy profile として再配置する。確定すべき決定は:

1. deny 設定を policy のどこに置くか（schema 配置と authoring
   trust boundary）;
2. pattern 記法を regex にするか glob 等の限定記法にするか;
3. deny 判定をどこに適用し、resolved policy から適用点へどう届け
   るか（`resolve_command_policy` 等の signature 配管か、別経路か）;
4. 不正 pattern・不正値の扱いを load 時 fail-closed にするか、
   warn/ignore にするか;
5. invocation 合成 grant（`--grant`）を deny 対象に含めるか。

## Decision Drivers

* 親 design が定める責務配置 — deny は mandatory 契約層ではなく
  operator 宣言値として降格されるべきものであり、issue #164 が
  `security.execDenyPatterns` を明示している。
* trust boundary — invocation 層は `command.grants` のみ供給でき、
  project 層は `command`/`route` を書けない。deny は operator
  posture であり両者の制約と矛盾しない場所に置く。
* layer-agnostic 評価 — resolved `command.grants` map は layer 由来
  情報を持たず、operator deny は invocation 合成 grant を exempt
  しない（issue AC）。
* #163 除去判断との整合 — 内容審査を mandatory 層に戻さず、
  builtin 定数を復活させず、bypass 構造を正直に文書化する。
* 既存 convention との整合 — `ArgPattern::compile`（regex・
  compile 時 fail-closed）・`route.approvalBypass`（operator のみ
  authoring する配列キー）・merge の配列 wholesale-replace 意味論
  に沿う。

## Considered Options

* **Option A** — `security` トップレベル節に `execDenyPatterns`
  （regex 配列、既定: 空）を置き、resolved `SecurityPolicy` を
  `RuntimePolicy.security` 経由で既存 3 適用点（declared basename /
  resolved basename / spawn 前再検証）の signature に配管する。
* **Option B** — `security` 配置は同じだが pattern 記法を glob
  （`*`/`?` 等の限定ワイルドカード）にする。
* **Option C** — deny 設定を `command` 節内に置く（例:
  `command.denyPatterns`）し、grant と同じ section の中で管理する。
* **Option D** — invocation 合成 grant を deny 評価から exempt する
  か、deny を mandatory/builtin として再導入する（#163 の除去を
  一部巻き戻す方向）。

## Decision Outcome

Chosen option: **Option A** — `security.execDenyPatterns`（regex 配列、
既定: 空）+ 3 適用点への signature 配管。

確定する規約:

* **pattern 記法**: regex（Option B の glob は却下）。
  `ArgPattern::compile` と同じ regex 基盤・同じ compile 時検査
  convention を再利用でき、versioned basename（`python\d+` 系）の
  表現力が必要な唯一の場面はまさに executable 名であるため、
  限定記法に落とす理由が無い。
* **match 対象と正規化**: executable の basename を正規化した stem
  — `/`・`\` 区切りの最終要素 → 小文字化 → `.exe`/`.com` suffix
  strip。旧 builtin の正規化規約をそのまま引き継ぐ。match は
  stem への完全一致（`^(?:pattern)$` 相当）・case-insensitive・
  複数 pattern は OR。
* **適用点**: #163 後に残った 3 か所 — `resolve_command_policy` の
  declared basename 段階（`InvalidRuntimePolicy`）、
  `pin_command_grant` の resolved basename 段階
  （`PolicyViolation` / `grant_violation` 規約）、
  `verify_pinned_executable` の spawn 前再検証段階（同規約）。
  resolved policy の `security` 値を `RuntimePolicy` → runtime/
  invocation options → 各 signature へ単一の経路で届ける。
* **layer-agnostic 適用**: deny は layer 由来情報を持たない
  resolved grants map 全体に及ぶため、invocation 合成 grant も
  対象（Option D の exempt は却下 — 実装上も layer provenance を
  後付けする必要が生じ、operator deny の意味を損なう）。
* **authoring trust boundary**: trusted 層（builtin/user）のみ。
  invocation 層は既存の構造 guard（keys ⊆ `{version,
  command.grants}`）で `security` を `InvalidRequest` 拒否し、
  project partial schema は `command`/`route` と同型に section 丸
  ごと除外して `InvalidRuntimePolicy` 拒否する。
* **不正 pattern**: load 時 fail-closed — compile 不能な pattern を
  含む policy は `InvalidRuntimePolicy` で拒否（Option B/D で
  併せて検討した warn-and-skip は却下）。
* **merge 意味論**: 配列の wholesale replace（union しない）。

### Consequences

* Good, because deny は mandatory 層に戻らず operator 宣言値になり、
  親 design の降格順序と #163 除去判断に整合する。
* Good, because 既定空・opt-in 宣言時のみ deny が効き、#163 後の
  既定挙動を壊さない。
* Good, because 3 適用点は #163 で空いた挿入点であり、deny を
  「認可契約の一部」として局所的に管理できる。
* Good, because resolved grants の layer-agnostic 性により
  invocation 合成 grant に特別扱いを要さない。
* Good, because regex + compile 時 fail-closed は `ArgPattern`
  convention と同型で学習コストが無い。
* Bad, because `RuntimePolicy` への新 field 追加で options 型
  （`ToolRuntimeOptions`・`InvocationToolRuntimeOptions`・
  `CommandToolOptions`・`CommandPolicyView`）と golden fixture・
  `RuntimePolicy` literal 構築 test に churn が生じる。
* Bad, because regex の表現力は operator 側の誤記（過剰 match・
  anchor 忘れ）の余地を残す — stem 完全一致・docs の seed 例で
  緩和する。
* Bad, because 名一覧方式の構造的 bypass（改名・別ツール・子
  プロセス）は残る — containment を主張しない表現で正直に記述
  する前提を引き継ぐ。

### Confirmation

* `security.execDenyPatterns` が schema に存在し既定が空であること、
  宣言時のみ 3 適用点で deny が効くこと、invocation 合成 grant が
  対象になることを test で固定する（spec の Acceptance Criteria
  に対応）。
* invocation 層・project 層からの `security` authoring がそれぞれ
  `InvalidRequest`/`InvalidRuntimePolicy` で拒否されることを test
  で固定する。
* compile 不能 pattern が load 時 `InvalidRuntimePolicy` で拒否
  されることを test で固定する。
* 旧 builtin 一覧と migration seed 例が docs に記録されていること
  を doc review で確認する。

## Implementation Plan

* Affected paths: `crates/cheaper-core/src/policy/schema.rs`・
  `crates/cheaper-core/src/policy/load.rs`・
  `crates/cheaper-core/src/policy/command.rs`・新設
  `crates/cheaper-core/src/policy/security.rs`・
  `crates/cheaper-core/src/policy/mod.rs`・
  `crates/cheaper-tools/src/tools/command/mod.rs`・
  `crates/cheaper-tools/src/registry.rs`・
  `crates/cheaper-tools/src/invocation.rs`・
  `crates/cheaper-cli/src/route/resolve.rs`・
  `crates/cheaper-cli/src/route/supply.rs`・
  `crates/cheaper-runtime/src/run/mod.rs`・
  `crates/cheaper-cli/src/workflow_run.rs`・
  `crates/cheaper-cli/resources/policy.yaml`・
  `docs/guides/command.md`・`docs/guides/configuration.md`
  （discovery が確定した call site inventory に従う — 正確な
  一覧は design で確定）。
* Patterns to follow: `ArgPattern::compile`（regex compile + load 時
  fail-closed）、`route.approvalBypass`（operator のみ authoring
  する配列キー + project partial からの section 除外）、旧
  `command_denied_executable_re` の basename 正規化規約。
* Patterns to avoid: deny の mandatory 復活・builtin 定数の再導入、
  invocation grant への layer provenance 後付けによる exempt、
  `process.tasks` への deny 適用（別 trust boundary）、warn-and-
  skip の silent 無効化、glob/独自記法の新設。
* Dependencies or configuration: 新規依存無し。`security` 節の
  `RuntimePolicy` 追加に伴う serialized shape の変化で golden
  fixture（`policy/schema.json`・`policy/load.json`）と
  `RuntimePolicy` literal 構築 test の更新が必要。

## Verification

* [ ] `mise run check`（format・clippy・doc-id・全 test）が通る。
* [ ] spec の Acceptance Criteria 全項目に対応する test が存在し
  通る（3 適用点 deny・invocation grant 対象・既定空・trust
  boundary 拒否・invalid pattern fail-closed・match 規約）。
* [ ] `node scripts/check-doc-ids.mjs` が通り、`audit_docs` が新
  規文書で 0 error を報告する。
* [ ] docs が「deterministic executable deny-list」の表現を使い、
  「保護/sandbox/containment」を謳わないことを doc review で確認。

## Pros and Cons of the Options

### Option A — `security.execDenyPatterns`（regex）+ 3 適用点 signature 配管

* Good, because issue #164 が指定する配置と一致し、`security` は
  command 認可とは別責務の operator posture 節として成立する。
* Good, because `command` 節に deny を混ぜると invocation が供給
  できる section の中に operator-only の設定が同居し trust
  boundary の語りが曇る — `security` 分離でそれを避けられる。
* Good, because resolved `SecurityPolicy` を `RuntimePolicy` 経由で
  届ける単一経路にでき、scope 構築・route viability・parity
  check の全経路が同じ deny 集合を見る。
* Bad, because `security` module と `RuntimePolicy` への新 field・
  options 配管の実装量が Option C より大きい。

### Option B — `security` 配置 + glob pattern

* Good, because 記法が単純で operator が誤記しにくく、basename
  match には表現力が過剰に見える面がある。
* Bad, because versioned basename（`python\d+`/`python3.12` 系）を
  書けず、旧 builtin 一覧のような interpreter version pattern の
  seed 移行が表現できない。
* Bad, because regex 基盤（`ArgPattern::compile`）が既存で、新
  記法の compile/検査/error 分類を別途作るコストに見合わない。

### Option C — deny 設定を `command` 節内に置く

* Good, because grant と deny が 1 section にまとまり見通しが良く、
  `ResolvedCommandPolicy` への配管が最小差分で済む。
* Bad, because `command` は invocation が `grants` を供給する唯一の
  section であり、operator-only 設定を同居させると authoring
  boundary を key 単位で守る必要が生じ構造 guard が複雑化する。
* Bad, because issue #164 が `security.execDenyPatterns` を明示
  しており、親 design の責務配置（deny は command 契約ではなく
  operator posture）とズレる。

### Option D — invocation grant exempt / mandatory・builtin 再導入

* Good, because exempt にすれば `--grant` の利便性が損なわれず、
  mandatory 復活は機構上最も強い拒否になる。
* Bad, because issue AC（invocation-synthesized grants も対象）に
  反し、resolved map は layer provenance を持たないため exempt の
  実装には provenance 後付けが必要で構造を歪める。
* Bad, because mandatory/builtin 復活は #163 の除去判断（内容審査は
  mandatory 層に置かない）と親 ADR の受理済み降格順序に直接
  反し、opt-in の要件を満たさない。

## More Information

* 起点: issue #164（`https://github.com/scarecrow173/cheaper/issues/164`）。
* 派生元 discovery: `../discovery/opt-in-exec-deny-patterns.md` —
  3 適用点の実測 signature、call site inventory、旧 builtin
  regex（git history から復元）、schema guard 実測の ground truth。
* 兄弟 spec: `../specs/opt-in-exec-deny-patterns.md`。
* 文脈参照（`references` のみ — cross-chain lineage は張らない）:
  `../adr/exec-contract-content-audit-removal.md`（#163 の除去
  判断）、`../adr/enforce-declared-contracts-over-content-audit.md`
  （declared contract 優先）、両 design/discovery/spec。
* 文書正直性 sweep・migration guide 全体は issue #169 の scope。

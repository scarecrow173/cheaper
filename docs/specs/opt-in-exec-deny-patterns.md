---
id: "SPEC-034KLgfJ0Gka7fkRKHpfSH"
type: "spec"
status: "approved"
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
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../designs/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by:
    - "../adr/unified-exec-grant-model.md"
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
---

# opt-in security.execDenyPatterns profile — A9 の降格実装

## What Should Be Built

policy schema に `security` 節を新設し、operator 宣言の executable
名 regex list `security.execDenyPatterns`（既定: 空）を
deterministic executable deny-list として実装する。deny は
`resolve_command_policy`（declared basename）・`pin_command_grant`
（resolved basename）・`verify_pinned_executable`（spawn 前再検証）
の既存 3 適用点で、layer 由来情報を持たない resolved
`command.grants` map 全体に対して効く — invocation 合成 grant
（`--grant` flag 経路）を exempt しない。

```yaml
security:
  execDenyPatterns: []   # 既定: 空。operator が regex 名リストを宣言すると
                         # grant executable の deterministic deny として効く
```

authoring は trusted 層（builtin policy + user-scope `policy.yaml`）に
限る — invocation 層は既存の keys ⊆ `{version, command.grants}`
構造 guard で `security` を書けず、project partial schema も
`command`/`route` と同型に section 全体を受け付けない。

これは #163 で mandatory 層から除去した A9 builtin deny list
（`command_denied_executable_re`）の降格先であり、「既知の危険名
一覧」への fail-fast であって containment 機構ではない。表現は
「deterministic executable deny-list」とし、「保護」「sandbox」
「containment」とは呼ばない。

## Why Now

#163（exec-contract cleanup — 内容審査を mandatory 層から除去、
ADR-034KI3r2xdthh6rKDFY517 accepted）は A9 deny list の mandatory
適用点と builtin predicate を完全削除した。その結果、現行の
cheaper には executable 名に対する deny 能力が一切無く、
`python`/`ssh`/`tar` 等の任意名を宣言した grant は拒否されない。
これは親 design が受理した降格順序（mandatory 除去 → opt-in
再配置）の中間状態であり、#164 が opt-in の補完経路を提供する。

mandatory 層に残さない判断の根拠（親 ADR-034KuqZelggTIb8Z8K9QB8）:
名一覧方式は改名・別ツール・子プロセス経由で構造的に bypass 無限
であり containment を主張できない。一方で「自環境では interpreter
系や wrapper 系の executable へ動的 argv を渡させたくない」という
operator の正当な要請は残るため、builtin 定数ではなく operator
宣言値として opt-in 提供する。

## Users and Value

- **operator / admin** — user-scope `policy.yaml` の
  `security.execDenyPatterns` に regex 名リストを宣言するだけで、
  grant executable への deterministic deny を有効化できる。旧
  builtin 一覧は「旧既定値」として docs に記録され、そのまま
  seed として再利用できる migration 例を持つ。
- **worker / workflow 作者** — 既定（空）では従来通り任意の
  executable 名を grant に宣言でき、opt-in しない限り挙動は
  #163 後の状態と変わらない。
- **runtime 保守者** — deny は mandatory ではなく policy 宣言値に
  なったため、適用点は認可契約の一部として局所的に管理でき、
  「内容審査」責務が mandatory 層へ戻ることはない。

## Scope

### In Scope

- policy schema への `security` 節追加（complete/partial schema の
  property・project partial schema からの除外・
  `validate_policy_schema`/`normalize_policy` の section resolve・
  `RuntimePolicy` への resolved field 追加）。
- operator 宣言 regex を compile する deny predicate と basename
  正規化（basename 抽出 → 小文字化 → `.exe`/`.com` strip → stem
  match）の実装。旧 builtin 正規化規約を引き継ぐ。
- deny 判定の 3 適用点への注入（`resolve_command_policy`・
  `pin_command_grant`/`pin_command_grants`・
  `verify_pinned_executable`/`check_command_execution`）と、
  `RuntimePolicy.security` から適用点へ deny 集合を届ける options
  配管（`ToolRuntimeOptions`・`InvocationToolRuntimeOptions`・
  `CommandToolOptions`・`CommandPolicyView` 等）。
- error 分類: resolve 段階の deny は `InvalidRuntimePolicy`、
  pin/verify 段階の deny は `PolicyViolation`（grant 名を
  `details.grant` に載せる `grant_violation` 規約）。
- 不正 regex（compile 不能）を policy load 時に
  `InvalidRuntimePolicy` で拒否する fail-closed 検査。
- docs: `docs/guides/command.md` の `security.execDenyPatterns`
  意味論・match 規約・「旧既定値」一覧と migration seed 例への
  書き換え（#163 で stale 化した旧 deny list 記述の更新を含む）、
  `docs/guides/configuration.md` の policy.yaml section reference
  への `security` 節追加。

### Out of Scope

- `process.tasks` executable への deny 適用 — 別 trust boundary
  （project も authoring できる fixed-argv task）であり、deny は
  「動的 argv を渡す grant の executable」を縛るもの。旧 F3
  機械は demote ではなく remove 済み。
- glob 等の非 regex pattern 記法、deny の CLI flag・env var 注入
  （`--grant` と同型の invocation 供給はしない — authoring は
  trusted 層のみ）。
- merge 意味論の変更（`execDenyPatterns` は配列の wholesale
  replace — union/差分 merge は行わない）。
- 文書全体の正直性 sweep・「boundary/sandbox/protection」表現の
  全量訂正 — issue #169 の scope。
- `security` 節への他 key の追加（本 issue は
  `execDenyPatterns` のみ）。

## Requirements

- **R1** — `policy_complete_schema`・`policy_partial_schema` は
  `security` section を optional property として受理し、
  `execDenyPatterns` は string 配列（既定: 空）とする。
  `security` は required にしない。
- **R2** — `policy_project_partial_schema` は `security` を
  property に持たず、`security` key を含む project policy は
  `additionalProperties: false` により `InvalidRuntimePolicy` で
  拒否される（`command`/`route` と同型の section 丸ごと除外）。
- **R3** — invocation 層（`--grant` 合成）は既存の merge 前
  構造 guard（top-level keys ⊆ `{version, command}`、`command`
  keys ⊆ `{grants}`）により `security` key を
  `InvalidRequest` + `details.layer="invocation"` で拒否する。
  本 issue による guard 自体の変更は不要。
- **R4** — 各 pattern は regex として compile され、match 対象は
  executable の basename を正規化した stem（`/`・`\` 区切りの
  最終要素 → 小文字化 → `.exe`/`.com` suffix strip）。match は
  stem への完全一致（各 pattern を `^(?:pattern)$` 相当として
  評価）かつ case-insensitive とする。複数 pattern は OR。
- **R5** — compile 不能な pattern を含む `security` 節は policy
  load 時に `InvalidRuntimePolicy` で拒否する（fail-closed —
  invalid policy を silent に ignore しない convention、
  `ArgPattern::compile` の compile 時拒否と同型）。
- **R6** — deny は `resolve_command_policy` の declared basename
  段階・`pin_command_grant` の resolved basename 段階・
  `verify_pinned_executable` の spawn 前再検証段階で評価され、
  deny 集合は `RuntimePolicy.security` から各適用点へ配管される。
  すべての resolve/pin/verify 経路が deny-aware であること（
  scope 構築・route viability・worker parity check を含む）。
- **R7** — deny は layer 由来情報を持たない resolved
  `command.grants` map 全体に及び、invocation 合成 grant
  （`--grant`）も対象とする（operator deny は invocation grant
  を exempt しない）。
- **R8** — 既定（`execDenyPatterns` 未指定・`null`・空配列）は
  deny 無しとし、任意の executable 名の grant を拒否しない。
- **R9** — deny match は grant executable にのみ適用し、
  `process.tasks` の executable・`filesystem.denyPaths`・
  `PathDenylist` 等の別機構には影響しない。
- **R10** — resolve 段階の deny 拒否は `InvalidRuntimePolicy`
  （`command_invalid` 系の message prefix）で、pin/verify 段階の
  deny 拒否は `PolicyViolation`（`grant_violation` 規約 —
  `details.grant` に grant 名）で返す。
- **R11** — `merge_policy_layers` の配列 wholesale-replace 意味論に
  従い、後読み trusted 層の `execDenyPatterns` は前の層の配列を
  丸ごと置換する（union しない）。この挙動を docs で明記する。
- **R12** — docs は旧 builtin 一覧を「旧既定値」として記録し、
  `execDenyPatterns` へそのまま貼れる migration seed 例を提供する。
  表現は「deterministic executable deny-list」とし、改名・別
  ツール・子プロセス経由で bypass し得ること、containment 機構
  ではないことを正直に明記する。

## Acceptance Criteria

- [ ] `security.execDenyPatterns` が policy schema（complete/
  partial）に存在し、既定値が空であること。
- [ ] builtin（complete）・user（partial）層の policy で
  `security` を authoring でき、invocation 層は `security` key で
  `InvalidRequest` + `details.layer="invocation"`、project
  partial は `security` key で `InvalidRuntimePolicy` に拒否
  されることが test で固定されていること。
- [ ] `execDenyPatterns` を宣言した場合のみ、declared basename
  （resolve 段階 `InvalidRuntimePolicy`）・resolved basename
  （pin 段階 `PolicyViolation`）・spawn 前再検証の 3 適用点で
  deny が効くこと、および `--grant` で注入した invocation 合成
  grant も deny 対象になることが test で固定されていること。
- [ ] 既定（空）では `python`/`ssh`/`tar` 等を宣言した grant が
  policy resolve・pin で拒否されないことが test で固定されて
  いること。
- [ ] match が stem への完全一致かつ case-insensitive であること
  — 例えば pattern `sh` は `bash`/`sshd` に match せず、pattern
  `python.*` は `python3`/`PYTHON.EXE` に match することが test
  で固定されていること。
- [ ] compile 不能な pattern を含む policy が load 時に
  `InvalidRuntimePolicy` で拒否されることが test で固定されて
  いること。
- [ ] 旧 builtin 一覧（旧 `command_denied_executable_re` の regex
  内容）と seed 用 migration 例が docs（`docs/guides/command.md`
  または `docs/guides/configuration.md`）に記録されていること。
- [ ] `mise run check` が通ること。

## Deferred Design Concerns

- **deny 集合の options 配管の具体形** — `ToolRuntimeOptions`・
  `InvocationToolRuntimeOptions`・`CommandToolOptions`・
  `CommandPolicyView`・`pin_command_grant(s)`・
  `verify_pinned_executable`・`resolve_command_policy` の各
  signature に deny をどう載せるかは design で確定する。
  discovery が特定した call site inventory（`load.rs`・
  `schema.rs`・`registry.rs`・`invocation.rs`・`route/resolve.rs`・
  `run/mod.rs`・`workflow_run.rs`）に沿って配管する。
- **`security` module の置き場所** — 新 module
  `crates/cheaper-core/src/policy/security.rs` に section schema・
  `SecurityPolicy`・resolver・deny predicate を集約する案を
  ADR で採用方向とし、design で確定する。
- **builtin `policy.yaml` への `security: {execDenyPatterns: []}`
  明示** — schema optional のため不要だが、knob の可視化として
  置くかは実装判断（`route.approvalBypass: []` 先例）。
- **`process.tasks` executable への deny 対象拡大** — 別 trust
  boundary のため本 issue では対象外。将来 grant/task merge
  （issue 3/7）で exec grant モデルが統一された場合に再検討。
- **resolved `SecurityPolicy` の serialized shape** —
  `RuntimePolicy` への新 field は golden fixture
  （`policy/schema.json`・`policy/load.json`）と `RuntimePolicy`
  literal 構築 test（`restrict.rs`・`capability.rs`・
  `dev/fixture.rs`・`run_dev_seam.rs`・`ollama_e2e.rs`）に
  churn を生じる — 実装時に fixture 更新が必要。

## Sources and Relations

- 起点 issue: #164（#151 実装分解 2/7）。deps: #163（完了済み）。
- 本 spec の派生元: `../discovery/opt-in-exec-deny-patterns.md`
  （実測 apply-point inventory・配管 site 棚卸し・旧 builtin
  regex 復元の ground truth）。
- 決定根拠（`references`/`related` のみ — cross-chain lineage は
  張らない）:
  - `../designs/permission-model-responsibility-redesign.md`
  - `../designs/exec-contract-content-audit-removal.md`
  - `../adr/enforce-declared-contracts-over-content-audit.md`
  - `../adr/exec-contract-content-audit-removal.md`
  - `../specs/exec-contract-content-audit-removal.md`
  - `../discovery/opt-in-exec-deny-patterns.md`
  - `../discovery/exec-contract-content-audit-removal.md`
  - `../discovery/permission-model-responsibility-redesign.md`
- 兄弟 ADR: `../adr/opt-in-exec-deny-patterns.md`
- 後続: 文書正直性 sweep・migration guide は issue #169 の scope。

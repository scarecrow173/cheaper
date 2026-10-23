---
id: "DISC-Fad7xt2qdtfpcCHhjMhkVy"
type: "discovery"
status: "resolved"
title: "opt-in security.execDenyPatterns profile — A9 の降格実装（#164）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
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
    - "exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../adr/opt-in-exec-deny-patterns.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/exec-contract-content-audit-removal.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../specs/exec-contract-content-audit-removal.md"
    - "../discovery/exec-contract-content-audit-removal.md"
    - "../discovery/permission-model-responsibility-redesign.md"
  defers: []
  deferred-by: []
---

# opt-in security.execDenyPatterns profile（issue #164）

## Exploration Goal

issue #164（#151 実装分解 2/7）は、#163 で mandatory 層から除去された
A9 `command_denied_executable_re` deny list を、operator opt-in の
deterministic deny `security.execDenyPatterns` として再配置する。
親 design（DESIGN-034KuDkWOmNXiBddEQ4XWK・approved）と #163 の
ADR（ADR-034KI3r2xdthh6rKDFY517・accepted — builtin predicate は
再利用不能として削除済み）で architecture 判断は確定済みであり、
本調査の完了条件は:

1. #163 後の 3 適用点（`resolve_command_policy`・
   `pin_command_grant`・`verify_pinned_executable`）が deny
   predicate の再挿入点として実コード上に存在することを file:line
   で確認する。
2. policy schema への `security` 節の追加位置（complete/partial/
   project partial の 3 schema・validate/normalize の呼出し・
   invocation layer の構造 guard）を確定する。
3. deny 判定を 3 適用点へ届ける配管経路（`RuntimePolicy` →
   tool runtime options → pin/verify）の全 call site を棚卸しする。
4. pattern 意味論（regex vs glob・match 対象の正規化・不正
   pattern の扱い）の決定材料を整理する。
5. docs 移管用に旧 builtin 一覧を git history から復元する。

## Key Issues and Assumptions

- **[確認済] 3 適用点は clean な再挿入点として残っている** —
  #163（merge `2e13031`、除去 commit `981a4f4`/`cce415b`）は旧
  `is_denied_command_executable` 呼出しを if-block ごと除去し、
  seam・stub・flag は残していない（sibling design「#164 境界」
  通り）。現行 code の該当位置は後述 Inventory の通り。
- **[確認済] resolved `command.grants` map は layer 由来情報を
  持たない** — `CommandGrant`
  （`crates/cheaper-core/src/policy/command.rs:35-48`）には layer
  field が無く、`load_policy` は invocation 層を user 層 merge の
  直後・typed re-resolve の前に `MergeMode::Replace` で畳み込む
  （`crates/cheaper-core/src/policy/load.rs:119-188`）。merged map
  に対する deny 評価は invocation 合成 grant（`--grant` flag 経路、
  `crates/cheaper-cli/src/grant.rs:49-104` が
  `{"version":1,"command":{"grants":{..}}}` を合成）にも等しく
  及ぶ — issue が要求する「invocation grant を exempt しない」は
  配管経路を通せば自動的に満たされる。
- **[確認済] invocation 層が `security` を書けない構造保証は既存** —
  `load.rs:124-131` の merge 前 key guard が top-level keys ⊆
  `{version, command}` を強制し、`:132-148` が `command` 内 keys ⊆
  `{grants}` を強制する（`security` は未知 top-level key として
  `InvalidRequest` + `details.layer="invocation"` で拒否される）。
  schema 側の追加作業は不要で、guard がそのまま効く。
- **[確認済] project partial schema は section 丸ごと除外の先例を
  持つ** — `policy_project_partial_schema`
  （`crates/cheaper-core/src/policy/schema.rs:340-355`）は `command`・
  `route` を properties から除外し、`additionalProperties: false`
  が key を含む project policy を `InvalidRuntimePolicy` で拒否する
  （comment「将来 grant field が増えても section 丸ごと除外なら
  自動的に project から閉じる」fail-closed 原則）。`security` は
  同じく properties に追加しないだけで同型の保証になる。
- **[前提] builtin predicate は新規実装** — 旧
  `command_denied_executable_re`・`is_denied_command_executable` は
  完全削除済み。#164 は operator 宣言 pattern を compile する別
  predicate（basename 正規化 + match）を新設する。basename 正規化
  規約（`rsplit(['/', '\\'])` → 小文字化 → `.exe`/`.com` strip →
  stem match）は旧実装を規約として引き継ぐ（後述）。
- **[前提] 表現は「deterministic executable deny-list」** —
  「保護」「sandbox」「containment」とは呼ばない（issue・親
  design）。名一覧方式の構造的 bypass（改名・別 tool・子プロセス
  経由）は docs で正直に明記する（#169 の正直性 sweep と表現を
  揃える）。

## Apply Point Inventory（実測 file:line — 2026-10-23 時点）

### 適用点 1 — `resolve_command_policy`（declared basename）

`crates/cheaper-core/src/policy/command.rs:232-375`。
签名 `resolve_command_policy(input: Option<&Value>)
-> Result<ResolvedCommandPolicy, CheaperError>`。grant loop
（`:285-363`）で `:287-291` が `executable` を raw spec から取り出す
直後が旧 `is_denied_command_executable(&executable)` のあった位置
（旧 `command.rs:377-381` — `command_invalid("interpreter or
exec-capable executable is forbidden")` / `InvalidRuntimePolicy`）。

**deny 判定の届け方が本 issue の主要な配管問題** —
`resolve_command_policy` は `command` section のみを引数に取り、
`security` は sibling の top-level section。現行の呼出し site:

| call site | 文脈 | `security` の有無 |
|-----------|------|-------------------|
| `load.rs:208` | merged `effective` の typed re-resolve | merged policy から取得可 |
| `schema.rs:429-431` | `validate_policy_schema` の layer 別 boundary 検査（`process`/`command` と同型に `security` も呼ぶ形） | layer の raw から取得可 |
| `schema.rs:570` | `normalize_policy`（complete-validated 入力） | merged raw から取得可 |
| `registry.rs:1115` | `create_tool_runtime` — `options.command`（`Partial<CommandPolicy>` raw JSON のみ） | options に新 field が必要 |
| `invocation.rs:234` | `create_worker_scope` parity check（caller 指定 raw の再 resolve） | `inner` に保持する必要 |
| `invocation.rs:398` | `create_invocation_tool_runtime` — `options.command` | options に新 field が必要 |
| `command.rs` 内 unit test・`proc_command.rs:409,430,1551,1560`・`route/resolve.rs:385`・`hook/routing_policy.rs:2068` | test 群 | 签名に追随 |

旧実装は predicate が module 内 const のため引数不要だったが、
operator 宣言値に変わるため deny 集合を引数注入するか、
post-resolve の別 pass にするかの選択が必要（ADR の option）。

### 適用点 2 — `pin_command_grant`（resolved basename）

`crates/cheaper-tools/src/tools/command/mod.rs:185-202`。
签名 `pin_command_grant(_name, grant, root, symlink_policy, denylist)
-> Result<String, ToolError>`。`resolve_process_executable` による
PATH 解決 + canonical realpath の直後（旧 `:230-237` —
`grant_violation("resolved executable is interpreter or
exec-capable")` / `POLICY_VIOLATION`）が再挿入点。
`pin_command_grants`（plural、`:167-179`）が per-scope loop で呼ぶ。
呼出し site:

- `registry.rs:1208-1214` — `create_tool_runtime` 内、
  `options.denylist` と同じ経路で deny 集合を渡す。
- `route/resolve.rs:311-319` — viability 事前評価が scope pin 未
  構築で直接呼ぶ shared seam。`CommandPolicyView`
  （`route/resolve.rs:69-76`）が `policy`/`root`/`symlink_policy`/
  `denylist` を束ねており、deny 参照を同 struct へ加えるのが自然な
  載せ先（`RouteSupply::command_view()` —
  `route/supply.rs:40-47` が `RuntimePolicy` から populate）。
- test: `proc_command.rs:1577`、mod.rs 内 `pin_command_grants`
  呼出し（`:712,730,746,767`）。

### 適用点 3 — `verify_pinned_executable`（spawn 前再検証）

`crates/cheaper-tools/src/tools/command/mod.rs:210-224`（private —
#163 で `root` 引数除去済み、`fn verify_pinned_executable(name,
pinned)`）。pin-equality（`resolved != pinned`、B2）の直後が
旧 `is_denied_command_executable(&resolved)`（旧 `:267-272`）の
位置。唯一の呼出しは `check_command_execution`
（`:391-394` — execute と `route::resolve` viability が共有する
predicate seam）で、deny 集合は `CommandToolOptions`
（`:233-244` — `denylist` と同じく Option field として載せる形）へ
畳み込み、`fn command()`（`registry.rs:317-327` — `ToolCtx` 由来）
と `route/resolve.rs:320-332` が populate する。

注: pin map は scope 構築時点で immutable に固定され
pin-equality が path 同一性を強制するため、適用点 3 の deny 再評価は
構造的には pin 時判定と等価。ただし「全適用点で同一 predicate を
再適用する」旧モデルの uniform 契約を維持するために残す（deny
集合が scope 生存中に変化しない現行構造では観測差分は生じない）。

### deny 集合の runtime 配管経路（`RuntimePolicy` → 適用点）

`RuntimePolicy`（`schema.rs:124-140`）へ新 field
`security: SecurityPolicy`（resolved struct、serde-native —
`CommandGrant.argv` が pattern source string を保持する先例に倣い
`exec_deny_patterns: Vec<String>` を保持する想定）。consumption
site:

- `cheaper-runtime/src/run/mod.rs:335-354`・
  `cheaper-cli/src/workflow_run.rs:236-271` —
  `InvocationToolRuntimeOptions` 構築（`command: Some(to_value(
  &policy.command))` と同型に deny を渡す）。
- `invocation.rs` — `inner` に保持し、scope 構築時に
  `runtime_options` へ再注入（`inner.command` と同型、`:271-274`）。
  parity check（`:232-238`）は `inner` 保持の deny で再 resolve。
- `registry.rs:1115`（`resolve_command_policy`）・`:1209`
  （`pin_command_grants`）・`ToolCtx.command()`（`:317-327` →
  `CommandToolOptions`）— `ToolRuntimeOptions` 新 field 経由。
- `route/supply.rs` + `route/resolve.rs` — `CommandPolicyView` 新
  field 経由で `pin_command_grant`・`CommandToolOptions` 双方へ。
- `cheaper-cli/src/policy.rs`（`policy grant add`）は
  `validate_policy_schema(Partial)` 経由で間接適用 — user policy
  file 自身の `security` pattern に match する spec は add 時点で
  `InvalidRuntimePolicy` になり、authoring 境界でも fail-closed。

`restrict_policy`（`policy/restrict.rs` — CLI restriction）は
`security` を触らない（`RuntimePolicy` clone でそのまま運ぶ）。

## Policy Schema への `security` 節挿入点

- `policy_complete_schema()`（`schema.rs:287-312`）— properties に
  `security` object schema を追加。required には**加えない**
  （`route` 先例 — optional section、`[]` default。既存の
  complete policy との後方互換を保つ）。
- `policy_partial_schema()`（`schema.rs:316-333`）— properties に
  `security` を追加（user 層が authoring する入口）。
- `policy_project_partial_schema()`（`schema.rs:340-355`）—
  properties に**追加しない**（`command`/`route` と同型の
  section-excluded fail-closed。project は deny の追加・
  緩和とも不能 — narrowing さえ受け付けない）。
- `validate_policy_schema()`（`schema.rs:401-433`）—
  `command`/`process` の section 別 resolve 呼出しと同型に、
  `security` key 存在時に section resolver を呼び pattern compile
  検査を行う（per-layer の fail-closed — 不正 regex は layer 単位
  の load で `InvalidRuntimePolicy`）。
- `normalize_policy()`（`schema.rs:501-633`）—
  `RuntimePolicy.security` を `route` 同様に passthrough 的に
  resolve（値 domain は schema が保証、pattern compile 済み前提で
  source string を収集）。
- merge は generic — `merge_policy_layers`（`policy/merge.rs`）は
  object deep-merge + array wholesale-replace のため、user 層の
  `security.execDenyPatterns` は builtin の配列を丸ごと置換する
  （`tools.allow` と同じ replace 意味論。union ではない — 仕様で
  明記要）。
- `security` の置き場所候補: 新 module
  `crates/cheaper-core/src/policy/security.rs`（section properties・
  `SecurityPolicy`・`resolve_security_policy`・deny predicate・
  basename 正規化を集約 — `command.rs` が section schema と
  predicate を同居させる先例）。`schema.rs` inline（`route`
  先例）は 1-field section としても成立するが、cheaper-tools から
  使う predicate + 正規化を持つため専用 module が適切（ADR option）。

## Pattern Semantics — 決定材料

- **regex vs glob** — issue・親 design とも「operator が regex 名
  リストを宣言」と明記（design `:174`）。旧 builtin も regex。
  `regex` crate は cheaper-core の workspace dep 済み
  （`crates/cheaper-core/Cargo.toml:21`）。glob 先例は
  `filesystem.denyPaths`（path 用）に存在するが、executable 名の
  versioned family 表現（`python\d+`）には regex が適切。
- **match 対象の正規化** — 旧 `is_denied_command_executable`
  （除去 commit 親 rev より復元）: `executable.rsplit(['/', '\\'])
  .next()` で basename → `to_lowercase()` → `.exe`/`.com` suffix
  strip → stem へ match。declared（raw spec 文字列）と resolved
  （canonical path）の両方に同一正規化を適用。
- **anchoring** — 旧 builtin は `^(?:…)$` の full-match。operator
  pattern を unanchored `is_match` にすると `sh` が `bash` に
  substring match するような false positive（deny の誤発動 =
  正当な grant の拒否）が生じる。各 pattern を
  `^(?:pattern)$` 相当の full-match として compile する方が
  「executable 名の deny-list」という mental model に合い、
  substring が欲しい operator は `.*` で明示できる（ADR option）。
- **case** — stem は小文字化されるため、case-sensitive compile の
  ままでは `Python` 等の mixed-case pattern が silent に不発に
  なる。match を case-insensitive に揃える（pattern も
  case-insensitive 評価 — `Python` でも `python` に match）。
- **invalid pattern** — policy load 時の fail-closed
  （`InvalidRuntimePolicy`）が convention
  （`load.rs` module doc「An invalid policy is never silently
  ignored」、`ArgPattern::compile` の compile 時拒否と同型）。
  warn-and-skip は deny を silent に弱めるため不採用方向。
- **複数 pattern** — いずれか 1 つが match すれば deny（OR）。
  空配列は deny 無し（既定）。
- **適用 scope** — grant executable のみ。`process.tasks` の
  executable には適用しない（別 trust boundary — project も
  authoring できる fixed-argv task であり、旧 F3 機械は demote
  されず remove 済み。deny は「動的 argv を渡す grant の
  executable」を縛るもので、事前宣言済み fixed-argv task は
  deny の対象概念ではない）。

## 旧 builtin 一覧（docs 移管用 — git から復元）

除去 commit `981a4f4` の親 rev より `command_denied_executable_re`
（旧 `command.rs:160-189`）を復元。単一 anchored regex で、分類は
comment 行の通り:

```
^(?:
  # interpreters（versioned name まで）
  pythonw?|python\d+(?:\.\d+)*[a-z]?|
  pypy\d*(?:\.\d+)*|perl\d*(?:\.\d+)*|ruby\d*(?:\.\d+)*|
  node|nodejs|deno|bun|php\d*(?:\.\d+)*|
  lua|luajit|lua[\d.]+|r|rscript|julia|
  wscript|cscript|mshta|java|javaw|groovy|tclsh|wish|expect|osascript|
  # exec wrapper / launcher（grant は shell の grant と等価）
  env|xargs|find|awk|gawk|mawk|nawk|sed|busybox|toybox|
  nice|nohup|setsid|timeout|watch|stdbuf|flock|taskset|
  sudo|doas|su|pkexec|setpriv|nsenter|unshare|chroot|
  docker|podman|systemd-run|ssh|scp|rsync|screen|tmux|
  # dynamic loader
  ld\.so|ld-linux.*|ld-musl.*|dyld|
  # Windows LOLBin + wsl
  rundll32|regsvr32|msbuild|dotnet|installutil|regasm|regsvcs|
  forfiles|pcalua|msiexec|wsl|jshell|
  # editor / pager / REPL escape
  vim|ex|nvim|less|man|mysql|psql|sqlite3|tar|
  # 追加 shell
  nu|elvish|xonsh|oil|ion
)$
```

旧 builtin regex をそのまま 1 要素として `execDenyPatterns` に
置けば旧 mandatory 挙動と**ほぼ**等価になる（差分: 旧 builtin は
`is_shell_process_executable`（A8 — `sh`/`bash`/`cmd` 系 +
script ext）が別 predicate で除去済みのため、旧 deny 一覧だけでは
shell/shim 名は対象外。shell 系も拒否したい operator は shell 名を
自前で追加する）。migration 例はこの verbatim regex か、各分類を
個別 pattern へ分解した yaml のいずれかを docs に示す。

## Docs 対象

- `docs/guides/command.md` — `:32` の旧「既定で拒否する」記述は
  #163 で既に stale（mandatory deny は存在しない）。本 issue で
  `security.execDenyPatterns` の意味論・match 規約（basename
  正規化・full-match・case-insensitive）・「deterministic
  executable deny-list であり containment ではない」記述・
  旧既定値一覧 + migration seed 例を同 section へ書く。
  `:45`（invocation grant の deny 言及）も opt-in 前提に
  restate。`:80` の「deny list は containment ではない」注意は
  opt-in profile に引き継ぐ。
- `docs/guides/configuration.md` — policy.yaml section reference
  に `security` 節を追加（`route` 同型の trusted-layer 限定
  section として documented）。
- `crates/cheaper-cli/resources/policy.yaml` — builtin に
  `security: {execDenyPatterns: []}` を明示 key として置くかは
  実装判断（schema optional のため不要だが、knob の可視化として
  置く先例は `route.approvalBypass: []`）。

## Test / Fixture Churn 候補

- `RuntimePolicy` literal 構築: `restrict.rs` test・
  `capability.rs` test・`cheaper-runtime/src/dev/fixture.rs`
  `default_policy()`・`cheaper-runtime/tests/run_dev_seam.rs`・
  `ollama_e2e.rs` — `security` field 追加に追随。
- golden fixture: `crates/cheaper-core/tests/golden/fixtures/
  policy/{schema,load,merge}.json` — normalize/load 出力の
  serialized policy に `security` key が現れる（追加）。
- options 签名 churn: `ToolRuntimeOptions`・
  `InvocationToolRuntimeOptions`・`CommandToolOptions`・
  `CommandPolicyView`・`pin_command_grant(s)`・
  `verify_pinned_executable`・`resolve_command_policy` —
  `proc_command.rs`・`proc_invocation.rs`・`proc_degrade.rs`・
  `proc_runtime.rs`・`differential.rs`・`characterization.rs`・
  `cheaper-workflow/tests/process.rs`・
  `filesystem_contracts/artifacts.rs`・`route/resolve.rs`・
  `routing_policy.rs` test 群。
- 新規 test 要求（issue AC）: 既定（空）で `python`/`ssh`/`tar`
  grant が受理されること・declared/resolved basename・spawn 前
  再検証の各 deny 発火・invocation 合成 grant への deny 適用・
  invocation 層 `security` key 拒否（既存 guard test の延長）・
  project partial `security` key 拒否・不正 regex の load 拒否。

## Open Questions

- ~~deny を `resolve_command_policy` の引数で注入するか
  post-resolve 別 pass にするか~~ → ADR で検討（引数注入推奨 —
  5+ call site の呼び忘れ drift を構造的に防ぐ）。
- ~~pattern の anchoring~~ → ADR で検討（full-match 推奨）。
- ~~`security` module 置き場所~~ → ADR で検討（`policy/security.rs`
  推奨）。
- `pin_command_grant` の `_name` 引数 — deny error が grant 名を
  details に載せるため使用に変わる（現行は unused）。

## Promotion Candidates

- [x] spec-doc: `../specs/opt-in-exec-deny-patterns.md`
- [x] adr-doc: `../adr/opt-in-exec-deny-patterns.md`

## Sources and Relations

- 起点: issue #164（#151 実装分解 2/7）。deps: #163（完了済み）。
- 上流: 親 chain（#151）の design・spec・ADR・discovery、および
  sibling chain（#163）の design・spec・ADR・discovery — 全て
  `references`/`related` のみ（cross-chain lineage は張らない）。
- 旧 builtin regex の ground truth: `git show 981a4f4^:crates/
  cheaper-core/src/policy/command.rs`（除去 commit の親 rev）。

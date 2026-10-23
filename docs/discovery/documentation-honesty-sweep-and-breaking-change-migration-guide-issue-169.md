---
id: "DISC-034L67agpaqLudLiaNI822"
type: "discovery"
status: "resolved"
title: "documentation honesty sweep and breaking-change migration guide issue-169"
created: "2026-10-23"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
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
    - "../designs/permission-model-responsibility-redesign.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../adr/unified-exec-grant-model.md"
    - "../adr/route-grant-set-projection.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "../tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "../guides/command.md"
    - "../guides/managed-process.md"
    - "../guides/configuration.md"
    - "../guides/parent-agent-integration.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep and breaking-change migration guide issue-169

## Exploration Goal

issue #169 — 権限モデル再設計(#151)分解の最終 slice(7/7)。
「boundary/sandbox/protection」表現と実強制範囲の整合
(claim ≤ 実強制)を guides・refine 対象 ADR/spec から訂正し、
#163–#168 の破壊的変更一覧と移行手順を migration guide に
列挙する。全実装依存(#163–#168)が完了したため着手可能。

実測 inventory(briefing で詳細化 — 後述「実測 inventory」節):

- 「OS sandbox ではない」「子プロセス fs 副作用は強制不能」
  の明記対象 — canonical file tool の workspace boundary
  (E1/E2)は cheaper 所有 surface で強制可能だが spawn 子
  プロセスの副作用は OS sandbox なしでは強制不能
- spawn API 層の shell-free 記述と ENOEXEC→`/bin/sh` fallback
  (libc 既定動作・保証対象外)の区別
- `security.execDenyPatterns` = deterministic executable
  deny-list の命名規約(「保護」「sandbox」「containment」
  とは呼ばない)
- refine 対象 16 doc(ADR 7 件・spec/design 9 件 — issue 本文
  の §Supersede/Refine 一覧。supersede なし・全件 refine)
- migration guide 収録項: executable deny list 削除・内容
  審査 7 項目除去・統一 grant 形状・**#168 の project 層
  `trusted` key presence 拒否**(修復 = key 削除 1 行 or
  userConfigDir へ vendor — ゼロ編集で shadowing 復活)

## Key Issues and Assumptions

- 「claim ≤ 実強制」の判定には各表現の実装照合が必要
  (掃き出し対象表現の grep inventory + 強制境界の実測)
- refine は文書訂正のみでコード変更を伴わない(コード側の
  不整合が見つかった場合は別 issue 化して本 sweep では
  文書の正直性を優先する)
- migration guide の置き場所・形式は briefing で確定
  (docs/guides/ 配下 or CHANGELOG 系の既存規約に従う)

## 実測 inventory(claim vs 実強制)

### 実強制されているもの(実装照合済み)

| 強制 | 実装接点 | 適用範囲 |
|------|----------|----------|
| workspace boundary(E1) | `crates/cheaper-tools/src/tools/boundary.rs:114-127`(`resolve_workspace_path` + `rethrow_as_denied`)、`crates/cheaper-core/src/workspace/paths.rs` | canonical file tool(write/edit/ls/read/find/grep)の path — cheaper 所有 surface のみ |
| deny_paths(E2) | `boundary.rs:120-126`(`PathDenylist::is_denied`)、ls `:456-459`(deny entry 非表示) | 同上 — declared path のみ |
| writeScope/writeClaims(E3) | `access.rs` の `ResourceAccess::observe` 経由 | canonical tool の op admission |
| spawn viability(executable) | `crates/cheaper-tools/src/tools/proc/executable.rs:67-83`(`native()`: realpath → is_file → win32 `.exe`/`.com` → X_OK) | `command` grant・`process` task の executable。**内容審査なし** — file 実在性+実行 bit+win32 拡張子のみ |
| pin + spawn 前再検証(B1/B2) | `executable.rs:90-92`(`native_executable` wrapper)、`exec/mod.rs` の pin/verify 経路 | pin 済み canonical path、ambient PATH は pin 時点 |
| argv pattern 照合(A6) | `exec/mod.rs:316-331`(`authorize_args` — pattern match のみ。**metachar 検査・内容審査なし**) | `command` の call-time argv |
| call input bounds | `command/mod.rs:48-53`(`args.maxItems:256`・`items.maxLength:4096` — input schema 層で `INVALID_INPUT`) | `command` model 入力 |
| lifecycle(C1-C3) | `proc/spawn.rs:146-170`(`env_clear`・stdin null・piped stdout/stderr・process group・windowsHide) | managed subprocess |
| `security.execDenyPatterns` | `policy/security.rs:99-127`(stem 完全一致・case-insensitive・OR・`^(?:p)$` anchor)、3 適用点(declared spec policy resolve `InvalidRuntimePolicy`・pin 時 resolved basename `POLICY_VIOLATION`・spawn 前再検証) | 全 exec grant(`command.grants` + `process.tasks` 由来 fixed grant)の **executable stem のみ** — opt-in・既定 `[]` |
| 層 gate(route) | `routes.rs:629-633`(`trusted`)、`:481-486`(`grants`)、`:520-525`(`substitute`)— project 層 key presence を `InvalidRuntimePolicy` で拒否 | routes.yaml authoring |
| route trust = 宣言層 provenance | `routes.rs` merge project 分岐(衝突→`suppressed`・非衝突→有効化、`route.trusted` 非読) | route merge precedence |
| grant scope eligibility | `policy/exec.rs:114-116`(`is_route_eligible` = `scope=="trusted" && !fixed`) | route grant 参照全般 |
| rewrite-safe gate(R8) | `hook/routing_policy.rs:1034+`(`tokenize_shell` — unsafe segment/token 分類、fail-open) | routing candidacy — **実行可否ではなく routing 評価を試すかの保守的 gate** |
| viability `unsafe-command` | `route/resolve.rs:284-289`(caller 報告 flag + cwd-changing executable の 2 条件 — tokenize 済み token の metachar 内容は分類しない、#166) | route viability |

### 実強制されていないもの(claim 側に訂正が必要)

| 旧 claim | 実測 | 訂正内容 |
|----------|------|----------|
| 「cheaper が workspace に閉じ込める」 | canonical file tool のみ。spawn 子プロセスの fs 副作用は `command.md:145`・`managed-process.md:47` が正直に記述済みだが全 sweep 対象で均質化が必要 | 「OS sandbox ではない・子プロセスの副作用は強制不能」の canonical 文言を配置 |
| executable の内容・名審査(native magic・shell/shim・script ext・bare `node`・workspace-resident) | 全除去済み(#163)。`executable.rs:6-8`「executable の内容は審査しない」明記、test `script_files_resolve_by_spawn_viability`(:295-323)で shebang/+X text file 受理を実証 | 旧要件を持つ doc から mandatory 表現を除去し opt-in/history 注記へ |
| direct-argv metachar reject(A10)・task argv 同種審査(F3) | `authorize_args` は pattern match のみ。`process.rs` test `accepts_shell_executables_and_argv_metachars`(:246-)で `["bash","-c","x"]`・`["echo","a|b"]` 受理を実証 | 「metachar 拒否」を含む記述を訂正 — spawn は shell 非介在のため metachar は executable の literal argv として渡る |
| `process.tasks` の `command` 要素 shell 審査 | 同上 — 内容審査なし。schema は `command: array minItems 1` の構造のみ | `managed-process.md:23`・`managed-process-canonical-tool.md` 要件の訂正 |
| deny list = mandatory 保護 | #164 以降 opt-in(`security.execDenyPatterns`・既定 `[]`)・stem match のみ。列挙を免れる binary は存在(plugin/extension 機構、改名、別経路) | 「deterministic executable deny-list」呼称 + 「保護/sandbox/containment ではない」但書 |
| ENOEXEC 非発生の含意 | glibc `execvpe` は shebang 無し +X text file で ENOEXEC→`/bin/sh` retry する(`process/tests.rs:242` コメント・`spawn.rs:85` musl 差分) | 「spawn は shell 非介在」と「ENOEXEC 時 libc fallback は OS 既定・保証対象外」の区別を明記 |

### 掃き出し対象の表現(docs grep inventory)

**guides(訂正必須)**:

- `docs/guides/command.md:44` — 「native magic(MZ/ELF/Mach-O)を持つ
  executable のみ実行可能で、shell・script shim(`.cmd`/`.bat`/
  `.ps1`/`.sh`)は `process` baseline の deny で拒否される」—
  **stale**(#163 で除去。同 file :100 の migration 注記と自相矛盾)
- `docs/guides/command.md:133` — 「全 args token … shell metachar
  拒否(`` ` ``・`|`・`&`・`;`・`<`・`>`・`$(`・CR/LF/NUL)が適用
  され … metachar 含有は `POLICY_VIOLATION`」— **stale**(call-time
  metachar 検査は A10 で除去。token ≤4096・args ≤256 は input
  schema 層の `INVALID_INPUT` として残存 — error code も訂正要)
- `docs/guides/managed-process.md:23` — 「任意 command/argv の指定、
  shell、shell 演算子、script shim を拒否する。pipe、semicolon 等は
  正規の引数内でも拒否」— **stale**(task 宣言 argv の内容審査は
  F3 で除去。呼出し input が task 名のみなのは不変)
- `docs/guides/managed-process.md:27` — 「Windows の `.cmd`/`.bat`/
  `.ps1` shim は実行しない … native PE/ELF/Mach-O executable のみ
  解決し」— **stale**(win32 の `.exe`/`.com` 制約は CreateProcess
  の spawn viability であって shim deny ではない。unix は内容無
  審査で +X file を解決 — ENOEXEC fallback 注記もここへ)
- `docs/guides/workers.md:54-65`・`:138-139` — writeScope 記述は
  概ね整合済み(tri-state coverage・`command` 非両立明記)。`:139`
  「`targets[].path` 以外への write を runtime が強制的に禁止する」
  は canonical file tool 文脈で正しいが exec 子プロセスへの非適用
  を補足する余地あり(軽微)

**guides(整合済み — sweep で確認のみ)**:

- `command.md` trust/sandbox 限界節(:141-153)・execDenyPatterns 節
  (:46-100)・`--grant` cmdline 露出(:119-123) — #164/#165 追従済みで
  正直な記述(:63/:148「containment 機構ではない」)
- `managed-process.md:47`(scope provenance・deny 均一適用・宣言は
  OS sandbox/tracer ではない明記)・`:49`(group escape 非対象)
- `configuration.md`(unified exec namespace :198-230・
  approvalBypass :232-268・execDenyPatterns :270-304 — 全て
  opt-in/層境界を正確に記述)
- `parent-agent-integration.md`(:235-258 provenance trust + 破壊的
  変更の修復手順、:296-311 grant eligibility、:386-406 substitute
  層制約 — #168 追従済み)
- `usage.md`(:301-310 record `executed` split・:343-349
  directlyComparable、:207 deny list 検査は opt-in 機構として実在)
- `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`(child
  process side effects not audited 明記)・`cheaper-usage/SKILL.md:353`
  (「deny list」— opt-in 機構として実在するため軽微な条件付記で
  十分。distribution 面の guide 相当として sweep 範囲に含める)

**refine 対象 ADR(7 件 — design §Supersede/Refine 一覧確定)**:

| ADR | ID | 残存 stale 記述(実測) | 訂正方向 |
|-----|-----|----------------------|----------|
| `command-grant-trust-boundary-and-managed-process-reuse` | ADR-034EMgaatFhsXjR97kH14c | :60/:90/:135 native magic・shell/shim 拒否・`argv_shell_re` metachar 拒否を mandatory として記述 | 決定時点の記録は保持し、#163/#164 で opt-in 降格・内容審査除去された旨の refine 注記を追記 |
| `worker-definition-runtime-policy-runtime-invariants-capability` | ADR-034r05iHlWosDpJ3DpG3XW | :96「filesystem access は workspace 内のみ、shell なし」等の invariants 列挙が子プロセスまで及ぶ印象 | invariants の適用範囲は canonical surface + 実行契約のみとする注記(3 層モデル自体は維持) |
| `workspace-filesystem-boundary-cheaper-sandboxed-tool` | ADR-034r05iHlWosDpO699OwYF | title・本文の「sandboxed tool」が canonical tool 越えの containment と読み得る | 「子プロセスの fs 副作用には及ばない」を明記する refine 注記 |
| `pretooluse-auto-routing-via-route-resolver-and-workflow-delegation` | ADR-034J8eZHNPqARi25HLFOL6 | C9 single-grant projection 前提・`trusted` flag 記述 | #166 grant-set 一般化・#168 provenance trust の refine 注記 |
| `per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass` | ADR-034KrYGRHZuvOIx1d0S0nE | approvalBypass gating は整合 — 内容審査除去に伴う表現の軽微な訂正のみ(design 指示) | 軽微な注記 |
| `defer-non-exec-routing-to-permission-model-redesign` | ADR-034KtTzOJGqmzmBW3SwGp5 | deferred sketch の採否が親 design で確定済み(grant-set・条件付き viability 弱化・record split・substitute 採用、Option E open) | 採否結果の refine 注記(実装確定済み) |
| `fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront` | ADR-034zGZpjoQlppwkkOs5gur | :185「`process` は固定 executor・shell 解釈なしで直接 spawn する(glob 展開/pipe/metachar なし)」 | 統一 grant + `tool:"process"` alias 形へ再記述 + spawn shell-free と ENOEXEC→`/bin/sh` fallback(libc 既定・保証外)の区別 |

**refine 対象 spec/design(9 件)**:

| doc | ID | 残存 stale 記述(実測) | 訂正方向 |
|-----|-----|----------------------|----------|
| `specs/policy-gated-command-tool` | SPEC-034EMbEGBOWM4CAM6YSWm6 | :60/:78-80 native-magic/shell-shim/interpreter deny・metachar 拒否が requirement、:94 AC4 | deny 要件を opt-in `execDenyPatterns` 前提へ降格・内容審査分を除去する refine 注記 |
| `specs/invocation-scoped-command-grants` | SPEC-034FlOz4hiN9m0nQW7lBN2 | :85/:93/:118/:133/:138/:150 「deny list」が mandatory 前提で記述 | deny 適用は opt-in profile 設定時の条件付き検査である旨の注記(機構自体は実在) |
| `specs/pretooluse-route-resolver-and-workflow-delegation` | SPEC-034J7SxWvGL3oSbeBpo8a3 | R1/R7/R12/R15/R34 は #166–#168 で更新済み。残は R8 周辺の viability 分類記述の整合確認のみ | 確認 + 残差分の軽微訂正 |
| `specs/managed-process-canonical-tool` | SPEC-034A8q6kmxcBsNF8zS9Cwn | :62「任意 command、追加 argv、shell executable/構文は拒否」・:72 AC の「shell argv」拒否 | 統一 grant schema(`fixed`+`inputs`/`outputs`)と内容審査除去へ写す refine 注記 |
| `specs/workflow-resource-inference` | SPEC-034zSWvgiB9gHQpjguNJ3M | :74 `tool:process` declaration seam・undeclared process INVALID_REQUEST | 統一 grant の resource 宣言へ再配線・fixed grant 前提で再記述 |
| `designs/policy-gated-command-tool` | DESIGN-034EMWNJbQpMB0N3ZTXM5A | §(a) token 検証・§(b) executable 解決・§(d) interpreter deny が mandatory 記述 | deny 部分のみ本再設計に置き換わる旨の refine 注記(pin B1/B2・argv pattern A6・C7 manifest・route-context 注入は引き続き authoritative — supersede しない) |
| `designs/workflow-resource-inference` | DESIGN-034zSWw1LYrRvcB471oQm2 | :57/:65/:89 process declaration の inference 経路 | spec 側と同じ再配線の注記 |
| `designs/rust-rig-runtime-migration-architecture` | DESIGN-034Bbq1qY8WFBdL26tUdBw | :291「`node` 解釈は "node を PATH 解決せず明示 error"」— B5 由来の固定ルール | B5 除去に伴う訂正(bare `node` は通常 PATH 解決) |
| `designs/pretooluse-route-resolver-and-delegated-workflow-execution` | DESIGN-034JbqoCWm85pKC033fgfd | :398-429 viability 分類・:301-310 `trusted` shadow 記述 | 条件付き弱化 + substitute retarget + provenance trust の注記 |

### migration guide 収録項(#163–#168 破壊的変更・挙動変化)

| issue | 変更 | 利用者影響 | 修復/移行 |
|-------|------|-----------|-----------|
| #163 | 内容審査 7 項目除去(A8 shell/shim・B6 script ext・B3 native magic・B4 workspace-resident・B5 bare `node`・A10 metachar・F3 task 同種) | 従来拒否された宣言 exec(python/ssh/tar/.sh/workspace binary/bare node/metachar argv)が受理される | 既定では何も拒否しない — 旧近似の拒否が必要なら #164 の deny profile を宣言 |
| #164 | builtin executable deny list 削除 → opt-in `security.execDenyPatterns` | 旧 mandatory deny が無指定では効かない | seed 例(command.md の旧 regex verbatim + 分類別 seed)を user `policy.yaml` `security.execDenyPatterns` へ。旧 shell 名 predicate は別機構で `sh|bash|…` を別途追加 |
| #165 | `command.grants`/`process.tasks` → 単一 `exec` grant namespace | resolved model は統一 grant(`process` 節・`tool:"process"` は loader 写像+alias で存続 — policy.yaml・workflow は即死しない)。両 section 名衝突は `InvalidRuntimePolicy` + `details.{grant,task}` で fail closed。`--grant`/`policy grant add` は `fixed:true`・明示 `scope` spec を `InvalidRequest` で拒否。project task は `scope:"project"`(route 不可視) | 名衝突の rename、`fixed`/`scope` を含む CLI spec の除去(必要なら trusted YAML で直接 authoring) |
| #166 | route `grants:[name,…]`(trusted 層限定)・viability 条件付き弱化・`kind:"route"` record `{command, executed}` split | record schema additive — 旧 reader は `executed` を無視して集計可。`executed.argv` は平文 | project routes.yaml の `grants` key 除去が必要になった場合の記述(実装上は新 key のため既存 file 影響なし) |
| #167 | route `substitute:{grant,argv}`(trusted 層限定)・exit = 実行した command の exit | substitute mode は semantic substitution を trusted opt-in で受容(新機構 — 既存 route 影響なし) | なし(新機能)。ただし spec R15 再記述済みの意味論確認を guide へ |
| #168 | project 層 `trusted` key presence を `InvalidRuntimePolicy` で拒否(値不問・`false` も拒否) | `trusted` を含む project routes.yaml は load 全滅(strict 経路 exit 3、hook 経路 fail-open で project routing 全体不活性化 — 症状は `route resolve` まで不可視) | 当該 key 削除 1 行、または routes.yaml を `<userConfigDir>/` へ vendor して user 層 route 化(`trusted:true` は user 層で inert 受理・shadow 動作復活) |

## Alternatives and Comparison

migration guide の置き場所と sweep 訂正方式が briefing の主要論点。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| O1a `docs/guides/migration.md` 新設(canonical migration guide) | user-facing guide の規約に一致・`docs/README.md` User guides へ index row を追加・将来の破壊的変更も同じ置き場所で集約できる | 初の migration guide で前例なし(命名は issue の呼称そのまま) | **採用** |
| O1b CHANGELOG.md 新設 | release 単位の変更追跡に一般的 | repo に CHANGELOG 規約が存在しない・guide 形式(症状→修復手順)より entry 列挙になりがち・docs/README.md の 3 層構造に載らない | 不採用 |
| O1c 各 guide へ分散記載 | 読者が tool guide 内で完結 | 破壊的変更一覧の単一入口が無く issue AC「一覧列挙」を満たしにくい・重複 drift | 不採用(一覧は集約、各 guide は訂正のみ) |
| O2a ADR/spec/design に「Refine 注記」追記(履歴保持) | 決定時点の記録を falsify しない・design 表の「追記」指示と一致・差分が追跡可能 | 本文と注記の 2 層読みが必要 | **採用**(ADR) |
| O2b 要件本文を in-place 書換(issue annotation 付き) | #166–#168 の route spec 先例(R15・R1・R12・R34)に一致・読み手が単一の真実を見る | 提案時点の記録が変わる | **採用**(living contract である spec の要件本文・guides) |
| O2c front-matter `refines`/`refined-by` で 16 件へ edge | graph 上で追跡可能 | commit `9d76d40` で確定した cross-chain lineage pollution を再発させる — `refines`/`refined-by` は lineage relation で異 chain 接続は route_graph を focus-required で全 chain block | 不採用(`related`/`references` + 本文内注記) |
| O3a deny profile を「deny-list」とだけ呼ぶ | 簡潔 | 「list が保護する」含意が残り得る | 不採用 |
| O3b 「deterministic executable deny-list」と呼ぶ | issue 指定の呼称・match 対象(executable stem)・確定的性質を名に含む | 長い | **採用** |

## Tentative Conclusions and Hypotheses

- **確定**: migration guide は `docs/guides/migration.md`(新規・front
  matter 無しの guide 規約)とし、`docs/README.md` User guides
  リストに index row を追加する。構成は issue 別(#163–#168)の
  変更要約 + 症状 → 修復手順の形式。
- **確定**: honesty policy の canonical 文言は ADR で固定する —
  (a)「cheaper は OS sandbox ではない。canonical file tool の
  workspace boundary(E1/E2)・denyPaths・writeScope は cheaper
  所有の tool surface で強制されるが、spawn した子プロセスの
  fs 副作用は強制されない」、(b)「spawn は shell を介さず argv
  を直接渡す。ただし POSIX で shebang を持たない +X file の
  ENOEXEC に対し libc が `/bin/sh` fallback するのは OS/libc
  既定動作であり cheaper の保証対象外」、(c)
  `security.execDenyPatterns` = 「deterministic executable
  deny-list」(「保護」「sandbox」「containment」とは呼ばない)。
- **確定**: 16 refine 対象は全件「本文訂正 + refine 注記」で、
  supersede しない。front-matter は `related`/`references` に
  新規 doc を追加する backlink 方式とし、`refines`/`refined-by`
  は cross-chain で使わない(9d76d40 前例 + pretooluse ADR
  :709-710 の明記規約)。per-doc の refine 意図は spec 本文の
  対象一覧表に集約する。
- **確定**: sweep は文書訂正のみ — 実測で stale と判明した記述は
  全て「doc が実装に追いついていない」側であり、code 側の不整合
  は見つからなかった(もし見つかれば別 issue 化する規約 —
  assumptions 参照)。
- **確定**: `command.md`・`managed-process.md` の stale 4 箇所は
  in-place 訂正し、`trust / sandbox 限界`節・`managed-process.md`
  の caveat 節は既に整合済みのため保持。ENOEXEC 注記は
  managed-process/command 両 guide と ADR-034zGZpjoQlppwkkOs5gur
  の refine 注記に配置。
- **仮説(実装時に確認)**: `workers.md:139` の writeScope 記述に
  「canonical file tool の write に限る」補足が必要かどうか —
  現行文脈(writer 系 worker + `command` 非両立の直前記述)では
  誤読の余地が小さいため最小限の補足で十分と判断。

## Open Questions

- なし(全論点は本 discovery で確定。実装詳細 — 各 doc への
  適用文面・migration guide の細目構成 — は spec/plan の責務)

## Promotion Candidates

- [x] spec-doc needed: yes → `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
      (SPEC-034L6i3XmfCaWRHAiw8TTT — sweep 要件・refine 対象
      16 doc の訂正方向一覧・migration guide 要件・AC 対応)
- [x] adr-doc needed: yes → `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
      (ADR-034L6ihibvbjmSnKmSffWH — honesty policy の canonical
      文言・deny-list 命名規約・migration guide 置き場所・
      refine-not-supersede + front-matter 規約の決定記録)
- [ ] design-doc needed: no — 依頼は briefing artifacts のみ
      (spec + ADR + resolved discovery)。適用設計は別 flow で
      扱う。

## Sources and Relations

- issue #169: https://github.com/scarecrow173/cheaper/issues/169
  (権限モデル再設計 7/7 — body は
  `../tasks/file-issue-docs-honesty-and-migration-guide.md`
  TASK-034KvxGAbTTWEsA1RbX7cn に canonical 保存)
- 親 chain: `../designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK — §Supersede/Refine 一覧が
  16 対象 doc の authoritative 一覧・issue 分解 7/7)・
  `../specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5 — R5 正直性要件・R9 破壊的変更
  列挙)・`../adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8 — 責務モデル:宣言契約強制 vs
  内容審査)・`../discovery/permission-model-responsibility-redesign.md`
- 実装済み変更の ADR(告知材料): `../adr/exec-contract-content-audit-removal.md`
  (#163)・`../adr/opt-in-exec-deny-patterns.md`(#164)・
  `../adr/unified-exec-grant-model.md`(#165)・
  `../adr/route-grant-set-projection.md`(#166)・
  `../adr/substitute-exec-route-mode.md`(#167)・
  `../adr/trusted-route-flag-issuer-redesign.md`(#168)
- 実装照合の接点: `crates/cheaper-tools/src/tools/boundary.rs`・
  `crates/cheaper-tools/src/tools/proc/executable.rs`・
  `crates/cheaper-tools/src/tools/proc/spawn.rs`・
  `crates/cheaper-tools/src/tools/exec/mod.rs`(`authorize_args`)・
  `crates/cheaper-tools/src/tools/command/mod.rs`(input schema)・
  `crates/cheaper-core/src/policy/security.rs`(`ExecDenyList`)・
  `crates/cheaper-core/src/policy/process.rs`・
  `crates/cheaper-cli/src/route/routes.rs`・
  `crates/cheaper-cli/src/route/resolve.rs`・
  `crates/cheaper-cli/src/hook/routing_policy.rs`
- refine 規約の前例: commit `9d76d40`(cross-chain `refines` →
  `related` 修正・graph pollution 復旧)・
  `../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md:709-710`
  (`refines`/`refined-by` は chain を跨がない明記)
- user-facing 記述の現状(訂正対象・整合済みの両方):
  `../guides/command.md`・`../guides/managed-process.md`・
  `../guides/configuration.md`・`../guides/workers.md`・
  `../guides/parent-agent-integration.md`・`../guides/usage.md`・
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md`・
  `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`

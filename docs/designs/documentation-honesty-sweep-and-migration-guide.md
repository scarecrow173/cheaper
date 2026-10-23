---
id: "DESIGN-034Lrgl6KUIjrLaU6BBL2U"
type: "design"
status: "draft"
title: "documentation honesty sweep + migration guide — per-doc 訂正計画・refine 注記規約・migration guide 構成"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  changes:
    added:
      - type: "file"
        path: "docs/guides/migration.md"
    modified:
      - type: "file"
        path: "docs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md"
      - type: "file"
        path: "docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
      - type: "file"
        path: "docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
      - type: "file"
        path: "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
      - type: "file"
        path: "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
      - type: "file"
        path: "docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
      - type: "file"
        path: "docs/specs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/specs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/specs/managed-process-canonical-tool.md"
      - type: "file"
        path: "docs/specs/workflow-resource-inference.md"
      - type: "file"
        path: "docs/designs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/designs/workflow-resource-inference.md"
      - type: "file"
        path: "docs/designs/rust-rig-runtime-migration-architecture.md"
      - type: "file"
        path: "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "./permission-model-responsibility-redesign.md"
    - "./trusted-route-flag-issuer-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "./permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../adr/unified-exec-grant-model.md"
    - "../adr/route-grant-set-projection.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../adr/trusted-route-flag-issuer-redesign.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../specs/policy-gated-command-tool.md"
    - "../specs/invocation-scoped-command-grants.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/workflow-resource-inference.md"
    - "./policy-gated-command-tool.md"
    - "./workflow-resource-inference.md"
    - "./rust-rig-runtime-migration-architecture.md"
    - "./pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../guides/command.md"
    - "../guides/managed-process.md"
    - "../guides/workers.md"
    - "../guides/parent-agent-integration.md"
    - "../README.md"
    - "../tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "../../packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "../../crates/cheaper-tools/src/tools/proc/executable.rs"
    - "../../crates/cheaper-tools/src/tools/proc/spawn.rs"
  defers: []
  deferred-by: []
---

# documentation honesty sweep + migration guide — per-doc 訂正計画・refine 注記規約・migration guide 構成

## Context

issue #169 — 権限モデル再設計（#151）分解の最終 slice（7/7）。
SPEC-034L6i3XmfCaWRHAiw8TTT が R1–R8 + AC を、
ADR-034L6ihibvbjmSnKmSffWH が D1（canonical disclaimer 文言）・
D2（deny-list 命名）・D3（refine-only 訂正方式）・D4（migration
guide 置き場所）を確定済み。DISC-034L67agpaqLudLiaNI822 の実測
inventory が訂正対象の file:line を正本として供給する。本 design は
spec/ADR の決定を編集可能な形へ落とす **適用設計** であり、新しい
方針決定は行わない — 決定済みの 3 軸（文言・訂正方式・置き場所）を
per-doc の編集契約へ分解する。

設計で固定する残りの論点:

- **refine 注記の canonical 形と配置規則** — ADR/design 本文へ
  追記する注記の構造（issue #169・変更点・現行真実・governing link）
  と配置位置を一本化する。`docs/adr/cli-owned-devin-hook-registration-
  lifecycle-with-apm-integration.md:166` の `> Amend(…)` blockquote が
  「決定時点の記録を保持したまま後訂正を示す」既存先例。
- **per-doc 訂正表** — 16 refine 対象 + guide stale 4 箇所の各々に
  つき、実測 anchor・旧記述・新記述（または注記内容）・訂正方式
  （in-place vs 注記追記）を固定する。全 anchor は本 branch
  `feature/issue-169-docs-honesty-sweep` で実測済み。
- **migration guide の構成契約** — `docs/guides/migration.md` の
  節構成・各 issue 節の必須項目を固定する（front matter 無しの
  guide 規約 — `release.md`・`distribution.md` と同形）。
- **front-matter backlink 規約** — commit 9d76d40 の確定規約
  （cross-chain の意味 link は `related`）を 16 doc へ適用する
  編集契約として明文化する。

## Scope

### In Scope

- refine 注記の canonical 形・配置規則・front-matter backlink 規約
  の設計（ADR D3・commit 9d76d40 先例の適用）。
- 16 refine 対象 doc（ADR 7 件・spec/design 9 件）の per-doc 訂正表
  — 実測 anchor・旧記述・訂正方向・注記内容。
- `docs/guides/` の stale 4 箇所（`command.md:44`・`:133`・
  `managed-process.md:23`・`:27`）の in-place 訂正文 shape と、
  canonical disclaimer の配置設計（`command.md` trust/sandbox 限界
  節・`managed-process.md` caveat 節・migration guide 前提節）。
- 均質化の軽微対象 — `workers.md:139` の writeScope 補足・
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md:353` の deny
  list 条件付記。
- `docs/guides/migration.md` の構成契約（preamble + #163–#168 の
  issue 別節）と `docs/README.md` User guides index row。
- churn inventory・verification 対応表。

### Out of Scope

- **doc 編集作業そのもの** — 本 doc は設計成果物であり、16 doc ・
  migration guide の編集は後続 plan/task が担う。
- **コード変更** — sweep で code 側の不整合（doc の記述が意図
  どおり正しく、実装がそれを満たさないケース）が見つかった場合は
  別 issue 化する。本 design の調査では発見しなかった — 実測
  inventory（discovery §実測 inventory）が示す stale は全て
  「doc が実装に追いついていない」側。
- **16 対象一覧外の doc 全般の総書換え** — impl/ir・plans・tasks・
  handoff・過去 discovery の history 記録は対象外。一覧外で発見
  した mandatory 内容審査前提の記述は軽微な注記で補ってよいが、
  一覧対象の完遂を優先する（spec Out of Scope 準拠 — 候補は
  §(f) の opportunistic 項目のみ）。
- **Option E exec-less route・OS-sandbox backed mode** — 親 chain
  の open/deferred 事項のまま。
- **release/version bump** — migration guide は「現在の main に
  適用される変更群」の告知であり issue 番号起点で記述する。
- **README index の自動再生成** — `docs/designs/README.md` は
  `<!-- doc-suite:generated-index -->` 管理だが、本 design の
  行追加は手動で同一 change に含める（convention: index は
  filename 昇順）。

## Components and Boundaries

### (a) canonical disclaimer — 文言の正本と配置規則

文言の正本は **ADR-034L6ihibvbjmSnKmSffWH D1** であり、以下の 3
項目を canonical とする（verbatim 引用 — 配置先は意味の等価性を
保つ形で文脈へ埋め込む。逐語の再掲でなくてよいが 3 要素すべてを
含めること）:

> - 「cheaper は OS sandbox ではない」
> - 「canonical file tool(write/edit/ls/read/find/grep)の
>   workspace boundary(E1/E2)・`filesystem.denyPaths`・
>   worker `writeScope` は cheaper 所有の tool surface で強制
>   される。spawn した子プロセスの fs 副作用は、OS sandbox
>   無しでは cheaper が強制・完全観測できない」
> - 「cheaper の spawn は shell を介さず argv を直接渡す(shell
>   metachar は executable の literal argv として渡り、cheaper
>   が解釈・拒否しない)。POSIX で shebang を持たない +X file の
>   実行が ENOEXEC となった際に libc が `/bin/sh` で retry する
>   のは OS/libc 既定動作(glibc `execvpe` 系。musl は retry
>   しない残留差分あり)であり cheaper の保証対象外」

配置規則（spec R1/R2）:

| 配置先 | 項目 | 形態 |
|--------|------|------|
| `docs/guides/migration.md` 前提節（冒頭） | 全 3 項目 | canonical 文言をそのまま箇条書き |
| `docs/guides/command.md` trust / sandbox 限界節（:141-153） | 項目 1・2（OS sandbox 非該当・子プロセス fs 副作用）＋ 項目 3（shell-free / ENOEXEC 区別） | 既存の正直な記述（:143/:145/:148）と重複しないよう、節冒頭の 1 文を canonical 形へ強化 + ENOEXEC 区別を 1 bullet 追加 |
| `docs/guides/managed-process.md` caveat 節（:47-49） | 項目 1・2 ＋ 項目 3 | :47「OS sandbox や tracer ではない」記述を canonical 形へ揃え、:27 訂正箇所へ ENOEXEC 注記を配置（§(d) 参照） |
| `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md` refine 注記 | 項目 3 | 注記本文へ ENOEXEC 区別を含める |

deny-list 命名（ADR D2 — sweep 内の全編集に適用）:
`security.execDenyPatterns` は「deterministic executable
deny-list」と呼ぶ（opt-in・既定 `[]`・executable basename stem への
anchored case-insensitive 完全一致・3 適用点 — policy resolve /
pin / spawn 前再検証）。「保護」「sandbox」「containment」
「防御」とは呼ばない。列挙を免れる経路（改名・代替 binary・
plugin/extension 機構）が残る但書は `command.md:63`・`:148` の
既存記述を維持・再利用する。

### (b) refine 注記 — canonical 形と配置規則

**形状（canonical）**: 対象 doc の末尾に `## Refine 注記（issue
#169）` 節を 1 つ追記する。複数 anchor を持つ doc でも節は 1 つに
集約し、箇条書きで anchor ごとの訂正を列挙する。節の必須要素
（ADR D3）:

```markdown
## Refine 注記（issue #169）

権限モデル再設計（#151 分解 #163–#168）により、本 doc の以下の記述は
現行の真実と異なる。<ADR/design は「決定時点の記録として本文は保持する」>
<箇条書き: `<file:line または節>` <旧記述の要旨> → <現行の真実>（#<issue>）>

governing: [ADR-034L6ihibvbjmSnKmSffWH](<相対 link>)（canonical 文言・
訂正方式）・[SPEC-034L6i3XmfCaWRHAiw8TTT](<相対 link>) R5・
[migration guide](../guides/migration.md)
```

**配置規則**:

| doc 種別 | 訂正構え | 注記位置 |
|----------|----------|----------|
| ADR（決定記録） | 本文は保持し注記追記のみ（履歴非改竄） | ファイル末尾に `## Refine 注記` 節を 1 つ追記（既存の最終節 — `More Information`・`Verification`・`更新履歴`・`Confirmation` 等 doc ごとに異なる — の後に置き、既存節の内部構造を乱さない） |
| design | ADR と同じ（注記追記が基本） | 同上 — `## Verification Notes` の後 = ファイル最終節 |
| spec（living contract） | 要件本文を **in-place** で現在の真実へ訂正。訂正行には `（issue #16X — …）` の inline 注釈を付す（route spec R1/R12/R15/R34 の #166–#168 先例）。残存差分が大きい doc は末尾へ同形の `## Refine 注記` 節を併置してよい | 要件本文中（in-place）＋ 任意の末尾節 |
| guide・skill | 現在の真実へ **in-place** 訂正 | 該当箇所の直接書換（refine 注記節は設けない — 利用者向け単一の真実） |

**front-matter backlink 規約（spec R6・ADR D3）**: 16 対象 doc
すべてに対し、cross-chain の意味 link は `relations.related` へ
追加する（commit 9d76d40 の確定規約 — `refines`/`refined-by` は
chain 内 lineage relation であり cross-chain edge は
route_graph pollution を再発させる）。追加する link:

- `../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
- `../adr/documentation-honesty-sweep-and-migration-guide.md`
- `../guides/migration.md`
- `./documentation-honesty-sweep-and-migration-guide.md`（本 design —
  designs/ 配下の対象 doc のみ相対パスが `./`）

spec/ADR/既存 doc の相対パスは対象 doc の配置 dir に従う
（`docs/adr/`・`docs/specs/`・`docs/designs/` はいずれも `docs/` 直下
のため、adr↔spec↔design は `../<dir>/`、同 dir は `./`）。
`refines`/`refined-by`/`supersedes`/`superseded-by` は新規に一切
張らない。superseded status の変更も行わない。

### (c) ADR 7 件 — per-doc 訂正表（全件「注記追記」方式）

いずれも本文は決定時点の記録として保持し、§(b) 規約の末尾
`## Refine 注記` 節 + `related` backlink を追加する。anchor は
実測 file:line。

#### ADR-1 `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`（ADR-034EMgaatFhsXjR97kH14c）

| anchor | stale 記述 |
|--------|-----------|
| :60 | 「`executable.rs`（native magic MZ / ELF / Mach-O check、shell / shim / bare-node 拒否、PATH search、Windows `.exe` / `.com`、POSIX `X_OK`）」— 内容審査込みの旧 executable 解決記述 |
| :69 | 「異なる native check・env sanitize 規則の分裂」を防ぐ DRY driver（native check 前提） |
| :90 | 「grant の executable 解決は `resolve_process_executable` …と native magic check、shell / shim 拒否規則を再利用する。`argv_shell_re` / `shell_executable_re` / `script_ext_re`…は `pub(crate)` 化」— mandatory 再利用決定 |
| :116 | regex の `pub(crate)` 化を含む affected paths |
| :135 | Verification: 「`command` が … native magic / shell / shim 拒否、`argv_shell_re` の metachar 拒否 … を `process` と同一実装で通る test」 |

注記内容: #163 で内容審査 7 項目（native magic・shell/shim・script
ext・workspace-resident・bare `node`・call-time/task argv metachar）
が mandatory 層から除去され、:60/:90/:116/:135 の mandatory 前提
記述は現行の真実と異なる。#164 で executable deny は opt-in
`security.execDenyPatterns`（deterministic executable deny-list・
既定 `[]`）へ降格。trusted-layer-only の grant 境界（Option A）は
**維持**され本 ADR の核心的決定として依然 authoritative。

#### ADR-2 `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md`（ADR-034r05iHlWosDpJ3DpG3XW）

| anchor | stale 記述 |
|--------|-----------|
| :96 | 「Runtime Invariants（§31）: …filesystem access は workspace 内のみ、symlink escape 禁止、unknown tool 禁止、shell なし、network tool なし…」— spawn 子プロセスまで及ぶ印象 |

注記内容: invariants の適用範囲は **cheaper 所有の canonical
surface + 実行契約**（file tool の workspace boundary・denyPaths・
宣言制約・lifecycle）に限る。spawn した子プロセスの fs 副作用は
OS sandbox 無しでは強制不能（canonical disclaimer 項目 1・2）。
3 層 capability モデル（Worker / Policy / Invariants）自体は維持。

#### ADR-3 `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md`（ADR-034r05iHlWosDpO699OwYF）

| anchor | stale 記述 |
|--------|-----------|
| title・:51・:55 | 「sandboxed tool」・「`cwd` = workspace root = filesystem security boundary」— canonical tool を越える containment と読み得る |

注記内容: 「sandboxed tool」は canonical file tool（write/edit/ls/
read/find/grep）の workspace boundary 強制を指し、spawn した子
プロセスの fs 副作用には及ばない。`command`/`process` の子
プロセスは OS sandbox 無しで動作する（#163–#168 確定後の真実
としては disclaimer 項目 1・2 が governing）。決定自体（boundary
強制を cheaper 側 tool 実装に置く）は有効。

#### ADR-4 `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`（ADR-034J8eZHNPqARi25HLFOL6）

| anchor | stale 記述 |
|--------|-----------|
| :181-190（D2） | route の `grant` field は `command.grants` の既存 grant **名 1 件**への参照（C9 single-grant projection 前提） |
| :280-317（D7） | project 層 route が `trusted: true` 宣言で下位層を shadow する trust モデル |
| :441/:450/:465 | `trusted: true` の pros/cons・shadow 記述 |
| :589-591/:635/:671 | `trusted: true` の検証項・文書化項・option 評価 |

注記内容: #166 で C9 single-grant 投影は route-declared grant-set
投影（`grants:[name,…]` — trusted 層限定）へ一般化され、#167 で
`substitute:{grant,argv}` mode が追加。#168 で `Route.trusted` は
**削除**され、route の trust は宣言値ではなく宣言層 provenance
（builtin/user = trusted、project 層は下位層を shadow しない）へ
再設計 — project 層 `trusted` key presence は `InvalidRuntimePolicy`
で拒否（値不問）。D2（routing ≠ authorization）・層解決順は維持。
:709-710 の cross-chain `refines` 禁止規約は本 sweep でも継続適用。

#### ADR-5 `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`（ADR-034KrYGRHZuvOIx1d0S0nE）

| anchor | stale 記述 |
|--------|-----------|
| （実測 grep で mandatory 内容審査の明確な残留なし — 親 design 指示「表現の軽微な訂正のみ」に該当） | approvalBypass gating 自体は整合 |

注記内容（軽微・1-2 行）: 本 ADR の決定（per-target hook rewrite の
approvalBypass trusted gating）は #163–#168 と整合を保つ。
#163 の内容審査除去・#168 の `trusted` flag 再設計で周辺表現が
変わったが、approval bypass は trusted 層 opt-in のまま変更なし。

#### ADR-6 `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`（ADR-034KtTzOJGqmzmBW3SwGp5）

| anchor | stale 記述 |
|--------|-----------|
| :175-209 | Deferred design sketch — grant-set 投影・viability 弱化・substitute retarget・exit mirror・record split の「#151 で採用する場合」の検討入力（採否未確定の体裁） |

注記内容: 採否は親 design §非 exec routing handoff で確定済み —
grant-set 投影 **採用**（#166・`project_grant` → route 宣言
`grants:[name,…]`、trusted-scoped grant のみ解決）・viability
**条件付き弱化**（#166 — byte-matched mode の argv precheck は維持、
set 内非 exec grant は存在 + pin のみ）・usage record
`{command, executed}` split **採用**（#166）・substitute-exec mode
**採用**（#167・exit mirror は「実行した command」の exit）・
workflow-declared grants **不採用**・Option E exec-less は引き続き
**open**。

#### ADR-7 `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md`（ADR-034zGZpjoQlppwkkOs5gur）

| anchor | stale 記述 |
|--------|-----------|
| :185 | 「`process` は `kind: "tool", tool: "process"` の固定 executor で、policy に宣言された named task の argv を shell 解釈なしで直接 spawn する（glob 展開 / pipe / metachar なし、cwd = workspace root または validated workspace-relative dir、path は workspace boundary 適用）。任意 command 文字列の実行は提供しない」 |

注記内容: #165 で `process.tasks` は unified `exec` grant
namespace の `fixed:true` grant へ loader 写像され、`tool:
"process"` は alias として存続（`command` grant は `fixed:false`）。
「shell 解釈なしで直接 spawn」は**維持**されるが正確には spawn
API 層の shell-free であり、metachar を含む argv は拒否されるの
ではなく executable の literal argv として渡る（#163 — 内容審査
除去）。POSIX で shebang を持たない +X file の ENOEXEC に libc が
`/bin/sh` で retry するのは OS/libc 既定動作（glibc `execvpe` 系・
musl は retry しない残留差分）であり cheaper の保証対象外
（canonical disclaimer 項目 3）。

### (d) spec/design 9 件 — per-doc 訂正表

spec は living contract として要件本文を **in-place** 訂正し、
訂正行へ `（issue #16X — …）` の inline 注釈を付す（route spec
先例）。design は注記追記。全件 `related` backlink 追加。

#### S-1 `docs/specs/policy-gated-command-tool.md`（SPEC-034EMbEGBOWM4CAM6YSWm6）— in-place + 末尾注記

| anchor | stale 記述 | 訂正方向 |
|--------|-----------|----------|
| :60 | 「executable 解決 + native-magic/shell-shim 拒否、per-token argv pattern match、shell metachar 拒否、cwd normalize + denylist + workspace boundary check、sanitized env という実行前検証」 | 「executable 解決（spawn viability: realpath・is_file・win32 `.exe`/`.com`・X_OK — 内容審査なし）、per-token argv pattern match、cwd normalize + denylist + workspace boundary check、sanitized env。executable/argv の内容審査は行わない（issue #163）。opt-in deny は `security.execDenyPatterns` 宣言時のみ（issue #164）」 |
| :78 | 「実行には native-magic check（MZ / ELF / Mach-O）と shell / shim 拒否を通過しなければならない。さらに shell / shim / interpreter 起動による迂回を既定で禁止し、python / perl / ruby / node 等の interpreter executable は拒否対象」 | pin（load 時 1 回 + spawn 前再検証）は維持。native-magic/shell-shim/interpreter/bare-node deny 要件を除去し「executable の内容・名は審査しない — opt-in `security.execDenyPatterns` 宣言時のみ deterministic executable deny-list が適用される」へ（issue #163/#164） |
| :79-:80 | 「shell metachar を含む token は match 前に reject」「shell metachar 拒否は … match 前の独立したチェック」 | 「call-time の内容審査は行わない — metachar を含む token も pattern 適合なら executable の literal argv として渡る（spawn は shell を介さない）。token 長 ≤4096・args 個数 ≤256 は input schema の `INVALID_INPUT`」（issue #163） |
| :94 | AC4「forbidden executable / argv（interpreter executable を含む）、shell injection（metachar を含む token）、cwd escape が実行前に reject」 | AC を現行契約へ: 「argv pattern 不一致・cwd escape・project grant authoring が拒否される。opt-in `security.execDenyPatterns` 宣言時のみ deny 該当 executable が拒否される（issue #163/#164 — 旧 mandatory 拒否 AC は opt-in 前提へ降格）」 |

末尾 `## Refine 注記` 併置可（差分が大きいため推奨）。trusted
boundary・pin・argv pattern・cwd/env 境界・lifecycle 要件は維持
（:81 の OS sandbox 非該当記述は既に正直 — 保持）。

#### S-2 `docs/specs/invocation-scoped-command-grants.md`（SPEC-034FlOz4hiN9m0nQW7lBN2）— in-place

| anchor | stale 記述 | 訂正方向 |
|--------|-----------|----------|
| :60 | 「deny list・argv pattern・executable pinning・env/cwd 規則」を `resolve_command_policy` 系統から継承 | 「deny list」を「`security.execDenyPatterns`（opt-in・既定 `[]`）宣言時の条件付き検査」へ条件化 |
| :64/:64 周辺・:70 | 「schema・deny list・argv pattern の誤り」前提 | 同上の条件化（deny 検査は profile 宣言時のみ） |
| :85 | 「同一の runtime 挙動（deny list・argv pattern・executable pinning…）」 | 「（argv pattern・executable pinning・`security.execDenyPatterns` 宣言時の deny 検査…）」へ |
| :93 | 「静的検査（schema + deny list + argv compile + env/cwd 規則）」 | 「schema + argv compile + env/cwd 規則（+ `execDenyPatterns` 宣言時の deny 照合）」 |
| :105 | 「deny list・argv match・managed-process lifecycle の意味論は不変」 | deny の意味論を opt-in 条件へ（機構は実在・既定では非活性） |
| :118 | 「deny list に該当する executable を含む grant spec は `--grant` 経由でも reject」 | 「`security.execDenyPatterns` 宣言時、deny 該当 executable を含む grant spec は `--grant` 経由でも reject（invocation 経路は deny を迂回しない）」— 機構の実在は維持、条件付き化のみ |
| :133 | 「deny list 検査（必須）」 | 「deny list 検査（`execDenyPatterns` 宣言時のみ・既定 `[]` では検査不発）」 |
| :138 | 「deny list 該当 executable や invalid argv pattern」 | 条件付記（宣言時のみ） |
| :150 | AC3「deny list 該当 executable を含む `--grant` spec は run 開始前に reject」 | AC 書換: 「`security.execDenyPatterns` 宣言時、deny 該当 executable を含む `--grant` spec は reject（issue #164 — deny は opt-in profile 前提）」 |
| :153 | AC6「invalid spec（deny list 該当…）」 | 同上の条件付記 |

機構自体は実在するため「deny は opt-in profile 宣言時の条件付き
検査」への書換であり、deny の存在抹消ではない点に注意。

#### S-3 `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`（SPEC-034J7SxWvGL3oSbeBpo8a3）— 確認 + 残差分の軽微訂正

R1/R7/R12/R15/R34 は #166–#168 で更新済み（実測: :224/:234/:256/
:286-302/:330-350/:497 付近に issue 注釈入り）。残差分の確認結果:

- :303-311（R8 rewrite-safe gate）・:306-307 の `unsafe_token`
  （metachar 等）分類は **routing candidacy 層**の保守的 gate であり
  依然強制されている（`hook/routing_policy.rs` の `tokenize_shell`、
  fail-open）— exec 時の metachar 拒否（#163 で除去）とは別層なので
  **本文訂正不要**。R12 の `unsafe-command` viability 値も維持。
- 差分が小さいため末尾注記は不要と判断 — `related` backlink のみ
  追加。ただし編集時に exec 時 metachar 拒否を残す記述が見つかっ
  た場合は R8 との層区別を 1 行補記する（実施者の確認義務）。

#### S-4 `docs/specs/managed-process-canonical-tool.md`（SPEC-034A8q6kmxcBsNF8zS9Cwn）— in-place

| anchor | stale 記述 | 訂正方向 |
|--------|-----------|----------|
| :62 | 「入力は task 名だけ。任意 command、追加 argv、shell executable/構文は拒否する。」 | 「入力は task 名だけ（model は command/argv を指定できない）。task の `command` 配列は `fixed:true` grant の literal argv として内容審査なしに spawn される — shell executable・演算子を含む要素も executable の literal argv となる（issue #163 — 内容審査除去、issue #165 — unified grant schema `fixed` + `inputs`/`outputs` へ写像）」 |
| :72 | AC「unknown task、command 入力、shell argv、cwd escape、undeclared output scope を拒否する test が通る」 | 「unknown task・`task` 名以外の input key（schema `INVALID_INPUT`）・cwd escape・undeclared output scope を拒否。`command` 配列要素の内容審査（shell argv 等）は行わない（issue #163）」 |

#### S-5 `docs/specs/workflow-resource-inference.md`（SPEC-034zSWvgiB9gHQpjguNJ3M）— in-place

| anchor | stale 記述 | 訂正方向 |
|--------|-----------|----------|
| :74（item 9） | 「process は現行 JSON kind を増やさず tool:process の将来 port source で表す。named task の inputs/outputs は明示必須（[] は可）。undeclared process は推論不可として INVALID_REQUEST」 | `tool:process` の declaration seam を unified grant の resource 宣言（`fixed:true` grant の `inputs`/`outputs` — `process.tasks` は loader 写像で存続）へ再配線し、「undeclared process は INVALID_REQUEST」を fixed grant 前提で再記述（issue #165） |

#### D-1 `docs/designs/policy-gated-command-tool.md`（DESIGN-034EMWNJbQpMB0N3ZTXM5A）— 注記追記

| anchor | stale 記述 |
|--------|-----------|
| :104/:113（§(a)） | token 検証: `argv_shell_re` metachar check・pattern 文字列自体の metachar 検査 |
| :119-123（§(b)） | workspace-resident executable 二段拒否・`native()` candidate 検証・`(d)` interpreter deny 適用 |
| :139-158（§(d)） | interpreter denial 節全体 — deny set 拡張・`command_denied_executable_re()`・3 適用点 |
| :209（§(j)） | error 写像表 — metachar/interpreter/shell/shim/workspace-resident を `POLICY_VIOLATION` |
| :259/:300 | integration test・Verification の metachar/interpreter/shim/workspace-resident 拒否項目 |

注記内容: deny list 部分（native magic・shell/shim・script ext・
interpreter deny・workspace-resident 拒否・metachar check）は #163
で mandatory 層から除去、#164 で opt-in `security.execDenyPatterns`
へ降格。**supersede ではない** — pin 契約（B1/B2）・argv pattern
言語（A6）・Exclusive root manifest（C7）・route-context 注入・
trusted-layer 境界は引き続き authoritative。

#### D-2 `docs/designs/workflow-resource-inference.md`（DESIGN-034zSWw1LYrRvcB471oQm2）— 注記追記

| anchor | stale 記述 |
|--------|-----------|
| :57 | `ResourceInferenceJob.process` の ProcessResourceDeclaration（task/inputs/outputs） |
| :65 | 「Process declaration は inputs/outputs 必須…undeclared process は fallback 成功にしない」 |
| :89 | 「undeclared tool:process の INVALID_REQUEST は port level で行使」 |

注記内容: S-5 と同じ再配線 — `process` declaration の inference
経路（task inputs/outputs→produces/writes）は #165 の unified
grant resource 宣言（`fixed:true` grant）前提へ置き換わる。

#### D-3 `docs/designs/rust-rig-runtime-migration-architecture.md`（DESIGN-034Bbq1qY8WFBdL26tUdBw）— 注記追記

| anchor | stale 記述 |
|--------|-----------|
| :291-293 | 「`node` task の `process.execPath` fast-path は Rust binary に等価物が無いため **`node` 解釈は "node を PATH 解決せず明示 error"(POLICY_VIOLATION 系)に置き換え**」— B5 由来の固定ルール |

注記内容: B5（bare `node` reject）は #163 で除去。bare `node` は
通常の PATH 解決を通り、spawn viability（realpath・is_file・
X_OK）で受理される（`proc/executable.rs:6-8` 明記）。`execPath`
fast-path 非存在の設計背景は記録として保持。

#### D-4 `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`（DESIGN-034JbqoCWm85pKC033fgfd）— 注記追記

| anchor | stale 記述 |
|--------|-----------|
| :301-310 | project route の `trusted: true` shadow 規則・`trusted` field は project 層のみ意味を持つ記述 |
| :398-429 | viability 分類 — `no-grant` が executable pin・denied executable・workspace residency・`prepare_cwd` denylist の事前 check を共有（:409-415）、`exec_args` の metachar（`argv_shell_re`）該当は `unsafe-command` 側に分類（:429-431） |
| :609 | C9 — resolved policy を route の grant 1 件へ投影 |
| :1034-1036 | `trusted: true` は self-declared の限界記述 |

注記内容: #166 で viability は**条件付き弱化**（byte-matched mode
の argv precheck は維持、grant-set member は存在 + eligibility +
pin のみ）＋ `grants:[name,…]` 投影一般化。#167 で substitute
mode の検査対象 retarget。denied executable・workspace residency・
metachar の事前 check は #163 で除去（deny は opt-in
`execDenyPatterns` 宣言時のみ）。`unsafe-command` 分類は caller
報告 flag + cwd-changing executable の 2 条件のみ。#168 で
`Route.trusted` 削除・merge 無条件化・trust は宣言層 provenance —
project 層 `trusted` presence は `InvalidRuntimePolicy` 拒否。

### (e) guides — in-place 訂正（stale 4 箇所 + disclaimer 配置 + 均質化）

guide・skill は利用者向け単一の真実なので in-place 訂正（refine
注記節は設けない）。guide 系 doc に front matter は無いため
backlink 編集も不要。

#### G-1 `docs/guides/command.md:44`

旧: 「…bare name は scope 構築時点の ambient PATH で解決し、
realpath 済み canonical path を pin する。**native magic（MZ /
ELF / Mach-O）を持つ executable のみ実行可能で、shell・script
shim（`.cmd`/`.bat`/`.ps1`/`.sh`）は `process` baseline の deny
で拒否される。**」

新（shape）: 先頭 2 文は維持。末文を「executable の内容・名は
審査しない — spawn viability（realpath・is_file・win32
`.exe`/`.com`・POSIX `X_OK`）のみを判定するため、shebang
script・任意 interpreter・shim 名の file も解決される。拒否する
唯一の executable 審査は opt-in `security.execDenyPatterns`」へ。
同 file :100 の migration 注記（旧 regex seed）との自相矛盾を解消。

#### G-2 `docs/guides/command.md:133`

旧: 「…`argv` 省略時は args 個数無制約。**全 args token（pattern
適合・rest 残余を問わず）には match 前の独立 check として shell
metachar 拒否（`` ` ``・`|`・`&`・`;`・`<`・`>`・`$(`・CR/LF/NUL）
が適用され、token 長 ≤4096・args 個数 ≤256。pattern 不一致・
metachar 含有は `POLICY_VIOLATION`。**」

新（shape）: 「`argv` 省略時は args 個数無制約。args 個数 ≤256・
各 token ≤4096 は input schema 層の bound で、超過は
`INVALID_INPUT`。call-time の内容審査は行わない — metachar を
含む token も pattern 適合なら executable の literal argv として
渡る（spawn は shell を介さない）。pattern 不一致は
`POLICY_VIOLATION`。」

#### G-3 `docs/guides/managed-process.md:23`

旧: 「`command` は固定 argv 配列。呼出し input は `task` のみで、
**任意 command/argv の指定、shell、shell 演算子、script shim を
拒否する。pipe、semicolon 等は正規の引数内でも拒否するため、**
複雑な処理は信頼した script に記述する。」

新（shape）: 「`command` は固定 argv 配列。呼出し input は `task`
のみ — model から任意 command/argv は指定できない。宣言した
`command` 配列は内容審査なしに literal argv として spawn される
— shell・script file・演算子を含む要素も executable の引数として
そのまま渡る（cheaper は shell を介さないため `|`・`;` は制御
構文として解釈されない）。executable 名・argv の正当性は task
作者の責務。」

#### G-4 `docs/guides/managed-process.md:27`

旧: 「**Windows の `.cmd`/`.bat`/`.ps1` shim は実行しない。** pnpm
等は native executable と JS entrypoint の argv にする。
**native PE/ELF/Mach-O executable のみ解決し、**bare command は
host PATH の絶対 directory を検索する（bare `node` も PATH 上の
実 `node` へ解決される — executable 名・内容の審査は行わない）。
見つからなければ `POLICY_VIOLATION` で拒否する。」

新（shape）: 「executable 名・内容の審査は行わない — spawn
viability（realpath・is_file・実行 bit）のみを判定する。win32 では
CreateProcess が直接 spawn できる `.exe`/`.com` のみ解決する
（`.cmd`/`.bat`/`.ps1` は viability で `POLICY_VIOLATION` — 内容
審査ではなく OS の制約）。pnpm 等は native executable と JS
entrypoint の argv にする。unix では +X file を内容無審査で解決
する — bare `node` も PATH 上の実 `node` へ解決される。**POSIX
で shebang を持たない +X file の実行が ENOEXEC となった際に
libc（glibc `execvpe` 系）が `/bin/sh` で retry するのは OS/libc
既定動作であり cheaper の保証対象外（musl は retry しない残留
差分あり）。** 見つからなければ `POLICY_VIOLATION` で拒否する。」

#### G-5 disclaimer 配置 — `docs/guides/command.md` trust/sandbox 限界節（:141-153）

:143「`command` の子 process は **OS sandbox 無し**で動作する」
は既に正直な記述 — canonical 形へ強化: 節冒頭を「cheaper は OS
sandbox ではない。canonical file tool（write/edit/ls/read/find/
grep）の workspace boundary（E1/E2）・`filesystem.denyPaths`・
worker `writeScope` は cheaper 所有の tool surface で強制されるが、
`command` が spawn した子プロセスの fs 副作用は OS sandbox 無し
では強制・完全観測できない」へ揃え（既存 :145 の bullet と重複
しないよう吸収・言い換え）、shell-free/ENOEXEC 区別（canonical
項目 3）を 1 bullet として追加する。

#### G-6 disclaimer 配置 — `docs/guides/managed-process.md` caveat 節（:47-49）

:47「宣言は scheduler/ownership の入力であり OS sandbox や
tracer ではない」を canonical 項目 1・2 の文言へ揃える（canonical
file tool の boundary/denyPaths/writeScope は tool surface で強制
されるが `process` の子プロセスの fs 副作用は強制・完全観測
できない旨）。:49 の group escape 非対象・W1 limitation は保持。
ENOEXEC 注記は G-4（:27）へ配置済みのため caveat 節への重複配置
は不要 — ただし :27 への cross-reference（「spawn viability と
ENOEXEC の区別は executable 解決の節を参照」）を 1 行添えてよい。

#### G-7 均質化 — `docs/guides/workers.md:138-139`

「writer 系 2 worker は `writeScope: targets` で、`targets[].path`
以外への write を runtime が強制的に禁止する。」へ「（canonical
file tool の write に限る — exec 子プロセスの fs 副作用には及ば
ない）」の但書を追加。:54-65 の writeScope tri-state 記述は整合
済みのため保持。

#### G-8 均質化 — `packages/apm/.apm/skills/cheaper-usage/SKILL.md:353`

「`add` runs the same validation as policy load (deny list, argv
pattern compile, env/cwd rules)」の「deny list」へ条件付記 —
「(opt-in deny list — `security.execDenyPatterns` declared のみ,
argv pattern compile, env/cwd rules)」形。distribution 面の guide
相当として sweep 範囲に含める（discovery §掃き出し 確定）。

#### G-9 整合済み（変更不要 — 編集しない）

`command.md` :46-100（execDenyPatterns 節・seed・migration 注記）・
:119-123（cmdline 露出）・:148、:63（containment 非該当の但書）、
`managed-process.md:47` 前半・`:49`、`configuration.md`（:198-304
unified namespace・approvalBypass・execDenyPatterns）、
`parent-agent-integration.md`（:235-258・:296-311・:386-406）、
`usage.md`（:301-310 record split・:343-349・:207 — deny list 検査は
opt-in 機構として実在するため本文維持。必要なら「(opt-in)」の
軽微な条件付記のみ可）、`packages/apm/.apm/skills/
cheaper-tool-operate/SKILL.md`（child side effects not audited 明記）。

### (f) migration guide — `docs/guides/migration.md` 構成契約

front matter 無しの guide 規約（`release.md`・`distribution.md`
同形）。ファイル構成:

```markdown
# Migration guide

<1-2 行: cheaper の破壊的変更と移行手順。issue 番号起点で列挙 —
version 番号への紐付けは release 時の責務。現在の main に適用
される変更群>

## 前提: cheaper が強制するもの・しないもの

<canonical disclaimer 全 3 項目を verbatim 配置（§(a)）>

## #163 — 内容審査 7 項目の除去
### 変更内容
### 利用者影響
### 修復・移行手順

## #164 — builtin executable deny list 削除 → opt-in security.execDenyPatterns
（同 3 小節）

## #165 — 統一 exec grant namespace
（同 3 小節）

## #166 — route grant-set 投影と `kind:"route"` record split
（同 3 小節）

## #167 — substitute-exec route mode
（同 3 小節）

## #168 — project 層 `trusted` key presence 拒否
（同 3 小節）
```

各 issue 節の必須内容（discovery §migration guide 収録項表を正本）:

| issue | 変更内容 | 利用者影響 | 修復/移行手順 |
|-------|----------|------------|----------------|
| #163 | 内容審査 7 項目除去（A8 shell/shim・B6 script ext・B3 native magic・B4 workspace-resident・B5 bare `node`・A10 call-time metachar・F3 task argv） | 従来拒否されていた宣言 exec（python/ssh/tar/.sh/workspace-resident binary/bare `node`/metachar argv）が受理される — 既定では何も拒否しない挙動変化 | 拒否近似が必要なら #164 の `security.execDenyPatterns` を user `policy.yaml` へ宣言（seed は次節） |
| #164 | builtin executable deny list 削除 → opt-in `security.execDenyPatterns`（deterministic executable deny-list・既定 `[]`） | 旧 mandatory deny は無指定では効かない | seed 例を提示: `command.md` :78 の旧 regex verbatim・:84-98 の分類別 seed・旧 shell 名 predicate（`sh|bash|…`）は別機構除去のため別途 pattern 追加が必要な旨の注記（`command.md`:100） |
| #165 | `command.grants`/`process.tasks` → 単一 `exec` grant namespace。`process` 節・`tool:"process"` は loader 写像 + alias で存続（即死しない）。名衝突（`command.grants` ∩ `process.tasks`）は `InvalidRuntimePolicy` + `details.{grant,task}` fail closed。`--grant`/`policy grant add` CLI spec の `fixed:true`・明示 `scope` は `InvalidRequest` 拒否。project task = `scope:"project"`（route 不可視） | authoring の section 分離は維持・grant 名空間は共有 | 名衝突時は一方を rename。`fixed`/`scope` を含む CLI spec は除去（必要なら trusted YAML で直接 authoring） |
| #166 | route `grants:[name,…]`（trusted 層限定の set 投影）・`kind:"route"` record の `{command, executed}` split | `executed.argv` は平文で記録される注意。project 層 `grants` key presence は `InvalidRuntimePolicy` で route load 全滅（record schema は additive — 旧 reader は `executed` を無視して集計可） | project routes.yaml の `grants` key 除去（新 key のため既存 file への影響は宣言時のみ）。`executed` を含む record を扱う集計系は additive field として読み替え |
| #167 | route `substitute:{grant,argv}`（trusted 層限定）・exit は「実行した command」の exit（substitute mode では親 agent が substitute の exit を原 command のものとして解釈 — semantic substitution を trusted opt-in で受容） | 新機構 — 既存 route 影響なし | なし（新機能）。project 層 `substitute` key presence は `grants` と同じく `InvalidRuntimePolicy` で拒否される旨の注意 |
| #168 | project 層 routes.yaml の `trusted` key presence を `InvalidRuntimePolicy` で拒否（`true`/`false`/非 bool 値不問） | `trusted` を含む project routes.yaml は **load 全滅** — strict 経路（`route resolve`/`route run`）は exit 3、hook 経路は fail-open でその workspace の全層 routing が不活性化。**症状は debug log のみに現れ `route resolve` を実行するまで不可視**（routing が黙って不発する症状として観測） | (a) 当該 key の削除 1 行（shadow を意図していないならこれで完了）、または (b) routes.yaml を `<userConfigDir>/` へ vendor して user 層 route 化（`trusted:true` は user 層で inert 受理・ゼロ編集で shadowing 復活）。hook 経路の fail-open 症状は `route resolve --command '…'` で診断 |

`docs/README.md` User guides リストへ index row を追加する
（`managed-process.md`・`command.md` の後・`workflow-usage.md` の
前あたり — 内容説明は「#163–#168 の破壊的変更・症状別修復手順」
系の 1-2 行）。front matter 無し guide のため README index は
手動管理（`release.md` 先例）。

### (g) churn inventory — 本 sweep が触れる file 全一覧

**新規**:
- `docs/guides/migration.md` — §(f) の構成
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  — 本 design（status `draft`）

**編集（docs のみ — code・fixture・test 変更なし）**:

| file | 変更種別 |
|------|----------|
| `docs/README.md` | User guides リストへ `migration.md` row 追加 |
| `docs/designs/README.md` | 本 design row 追加（filename 昇順 — `devin-usage-…` と `exec-contract-…` の間） |
| `docs/guides/command.md` | G-1(:44)・G-2(:133) in-place + G-5 disclaimer 配置 |
| `docs/guides/managed-process.md` | G-3(:23)・G-4(:27) in-place + G-6 disclaimer 配置 |
| `docs/guides/workers.md` | G-7(:138-139) 但書 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | G-8(:353) 条件付記 |
| `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md` | ADR-1 末尾注記 + `related` backlink |
| `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md` | ADR-2 同上 |
| `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md` | ADR-3 同上 |
| `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md` | ADR-4 同上 |
| `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md` | ADR-5 同上 |
| `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md` | ADR-6 同上 |
| `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md` | ADR-7 同上 |
| `docs/specs/policy-gated-command-tool.md` | S-1 in-place + 末尾注記 + backlink |
| `docs/specs/invocation-scoped-command-grants.md` | S-2 in-place + backlink |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | S-3 backlink（残差分見つかれば軽微訂正） |
| `docs/specs/managed-process-canonical-tool.md` | S-4 in-place + backlink |
| `docs/specs/workflow-resource-inference.md` | S-5 in-place + backlink |
| `docs/designs/policy-gated-command-tool.md` | D-1 末尾注記 + backlink |
| `docs/designs/workflow-resource-inference.md` | D-2 同上 |
| `docs/designs/rust-rig-runtime-migration-architecture.md` | D-3 同上 |
| `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md` | D-4 同上 |

**opportunistic（一覧外・発見済み — 軽微注記は許容、完遂優先）**:
`docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md:95`
（「executable 検証（native magic / shell / shim 拒否、
`minimal_env`）は従来通り」— 軽微な注記で補ってよい。一覧対象の
完遂を優先する spec Out of Scope 規約に従う）。

**status 整合確認（編集無し・確認のみ）**: `docs/specs/README.md`
（SPEC-034L6i3XmfCaWRHAiw8TTT = proposed）・`docs/adr/README.md`
（ADR-034L6ihibvbjmSnKmSffWH = proposed）・
`docs/discovery/README.md`（DISC-034L67agpaqLudLiaNI822 =
resolved）は既に front-matter status と整合 — 実測済み。

## Data and Control Flow

- 訂正の制御順序: (1) guides in-place（G-1〜G-8 — 利用者向け単一の
  真実を先に正す）→ (2) spec in-place（S-1〜S-5）→ (3) ADR/design
  注記追記（ADR-1〜7・D-1〜4）→ (4) migration.md 新設 →
  (5) `docs/README.md` + `docs/designs/README.md` index →
  (6) `node scripts/check-doc-ids.mjs` + grep 検証。順序は依存を
  持たない便宜上のものであり、commit 分割は「guides 訂正」・
  「refine 注記 sweep」・「migration guide + index」の 3 塊が自然
  （Conventional Commits — `docs(guides):`/`docs(adr,spec,design):`/
  `docs(guides):` 系）。
- 訂正の参照方向: 全ての注記・訂正は ADR-034L6ihibvbjmSnKmSffWH
  （canonical 文言・命名・方式）→ SPEC-034L6i3XmfCaWRHAiw8TTT
  R5/R7（対象一覧・収録項）→ migration.md（告知先）へ link する
  一方向。逆方向の link（対象 doc 側からの backlink）は
  `related` のみ。
- code 側不整合の flag 機構: 編集中に「doc の記述が意図として
  正しく実装が非対応」な箇所を発見した場合、当該 doc の訂正を
  行わず issue へ follow-up 候補として記録する（doc 側を実装へ
  合わせる訂正と混同しない）。本 design 時点の実測では該当なし。

## Non-Functional Constraints

- **文書のみの差分** — code・fixture・test を一切変更しない
  （spec R8・issue 前提）。doc-suite skill 配下の `.agents/`・
  `apm_modules/` 生成物にも触れない（`packages/apm/.apm/skills/` の
  SKILL.md は配布物として対象内）。
- **claim ≤ 実強制** — 訂正後の文言が実装強制範囲を超えない
  こと。実装接点（`proc/executable.rs:6-8` の「内容は審査しない」
  明記・`security.rs` の stem match・`routes.rs` 層 gate）と照合済み。
- **履歴非改竄** — ADR/design の本文は保持し注記で補正。
  supersede・superseded status 変更なし。
- **graph lineage 保護** — 新規 `refines`/`refined-by`/
  `supersedes`/`superseded-by` edge ゼロ。cross-chain link は
  `related` のみ（9d76d40・ADR-034J8eZHNPqARi25HLFOL6:709-710）。
- **deny-list 命名** — D2 の呼称規約を sweep 全編集に適用
  （「保護」「sandbox」「containment」「防御」と呼ばない）。
- **既存の正直な記述を壊さない** — `command.md`:63/:148・
  `managed-process.md`:47/:49・`parent-agent-integration.md`
  :235-258 等の整合済み記述は維持し、canonical 文言との重複・
  矛盾を避ける均質化に留める。

## Risks and Trade-offs

- **注記品質のばらつき** — 16 doc へ注記を書く作業者の判断幅が
  残る（ADR Consequences も同認識）。対策: §(c)/(d) の per-doc
  表で anchor・訂正方向・注記要点まで固定し、注記節の canonical
  形を §(b) で定義。
- **in-place 訂正の過剰書換** — spec 要件本文を現行真実へ書き
  換える際、維持すべき要件（trusted boundary・pin・argv pattern・
  lifecycle）まで消すリスク。対策: per-doc 表で「維持する項目」を
  明示（S-1 の trusted boundary 等）。
- **一覧外 stale の残存** — `opt-in-soft-gating` ADR :95 のような
  一覧外残留は本 slice で全捕捉できない。対策: 発見済みの 1 件は
  opportunistic 注記を許容、以後は継続 sweep 対象として spec の
  Deferred Design Concerns 通り管理。
- **code 側不整合の発見** — 編集中に「doc が正しく実装が誤り」
  を見つける可能性。本 sweep は doc を正しても code を正さない
  ため、flag → follow-up issue 化の機構を §Data and Control Flow
  で固定。
- **migration guide の形骸化** — issue 別節構成は今回の #163-168
  群には十分だが、将来の破壊的変更で形式が追いつかない可能性。
  対策: 現時点は YAGNI で issue 別構成のまま、継続運用判断は
  spec Deferred へ委譲。

## Deferred Design Concerns

- 16 対象一覧外の stale 表現の補正 | 一覧外 doc で mandatory 内容
  審査前提の記述が残り得る（発見済み: `opt-in-soft-gating` ADR
  :95） | 発見次第の軽微注記 or 継続 sweep 対象 | 範囲を広げると
  本 slice が完遂しない — spec Deferred と同一
- OS-sandbox backed mode | 真の containment 需要が発生した場合 |
  「提供しない」を正直に文書化しておくことで誤期待を防ぐ | 親
  design・spec の Deferred と同一
- migration guide の継続運用 | 将来の破壊的変更ごとの追記先と
  して `migration.md` を canonical にするか version 単位 handoff
  にするか | 次の破壊的変更発生時点で判断 | 現時点では issue 別節
  構成で十分（YAGNI） — spec Deferred と同一

## Verification Notes

spec ACs への対応（grep で検証可能な形）:

- [ ] **issue AC1 / spec AC1** — `docs/guides/migration.md`・
      `docs/guides/command.md`・`docs/guides/managed-process.md` に
      「OS sandbox」非該当・子プロセス fs 副作用の強制不能を含む
      canonical disclaimer 相当文言がある（`grep -n "OS sandbox"`
      で 3 file とも hit・`command.md`・`managed-process.md` の
      所定節に canonical 項目 1-2 の文言）
- [ ] **issue AC2 / spec AC2** — 訂正後の grep 検証:
      `docs/guides/command.md` に「native magic（MZ」・
      「shell metachar 拒否」が**残っていない**、
      `docs/guides/managed-process.md` に「native PE/ELF/Mach-O」・
      「shell 演算子、script shim を拒否」が残っていない、16
      対象 doc それぞれに `## Refine 注記` 節または in-place
      訂正（`issue #16` 注釈）がある（discovery §掃き出し
      inventory の全件処理）
- [ ] **issue AC3 / spec AC3** — `managed-process.md`・
      `command.md`（+ ADR-034zGZpjoQlppwkkOs5gur の注記）に
      「ENOEXEC」・「`/bin/sh`」・「OS/libc 既定動作」・
      「保証対象外」の区別記述がある（`grep -n "ENOEXEC"`
      で両 guide hit）
- [ ] **issue AC4 / spec AC4** — 16 対象 doc すべてに refine
      反映（注記節または in-place 訂正）があり、それぞれの
      front-matter `related` に本 chain doc への backlink 追加。
      かつ新規 `refines:`/`refined-by:`/`supersedes:`/
      `superseded-by:` が 16 doc 差分内に存在しない
      （`git diff` + grep で確認）
- [ ] **issue AC5 / spec AC5** — `docs/guides/migration.md` が
      存在し、`#163`〜`#168` の 6 節すべてと各節の
      「変更内容/利用者影響/修復」小節を持つ。`docs/README.md`
      の User guides リストに `migration.md` の行がある
- [ ] **spec R8 / issue AC（文書のみ）** — 差分が docs/*.md・
      `packages/apm/.apm/skills/*/SKILL.md` のみで code・
      fixture・test 変更なし。`node scripts/check-doc-ids.mjs`
      pass。`docs/designs/README.md` の新 design row（status
      `draft`・filename 昇順位置）と既存 chain 行（spec=
      proposed・ADR=proposed・discovery=resolved）が
      front-matter status と整合

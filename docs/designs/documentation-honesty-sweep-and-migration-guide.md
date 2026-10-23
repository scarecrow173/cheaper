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
        path: "docs/guides/usage.md"
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
        path: "docs/adr/managed-process-named-task-lifecycle.md"
      - type: "file"
        path: "docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
      - type: "file"
        path: "docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
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
        path: "docs/specs/cheaper-v1-worker-runtime-cli.md"
      - type: "file"
        path: "docs/designs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/designs/workflow-resource-inference.md"
      - type: "file"
        path: "docs/designs/rust-rig-runtime-migration-architecture.md"
      - type: "file"
        path: "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
      - type: "file"
        path: "docs/designs/managed-process-canonical-tool.md"
      - type: "file"
        path: "docs/designs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/designs/workspace-filesystem-boundary-sandboxed-tools.md"
      - type: "file"
        path: "docs/designs/overview.md"
      - type: "file"
        path: "docs/test-specs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/test-specs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/test-specs/managed-process-task-authorization.md"
      - type: "file"
        path: "docs/test-specs/route-resolution.md"
      - type: "file"
        path: "docs/test-specs/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/test-specs/route-grant-set-projection.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
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
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../adr/managed-process-named-task-lifecycle.md"
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "./managed-process-canonical-tool.md"
    - "./invocation-scoped-command-grants.md"
    - "./workspace-filesystem-boundary-sandboxed-tools.md"
    - "./overview.md"
    - "../test-specs/policy-gated-command-tool.md"
    - "../test-specs/invocation-scoped-command-grants.md"
    - "../test-specs/managed-process-task-authorization.md"
    - "../test-specs/route-resolution.md"
    - "../test-specs/substitute-exec-route-mode.md"
    - "../test-specs/route-grant-set-projection.md"
    - "../impl/ir/trusted-route-flag-issuer-redesign.md"
    - "../../packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "../../packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
    - "../../crates/cheaper-tools/src/tools/proc/executable.rs"
    - "../../crates/cheaper-tools/src/tools/proc/spawn.rs"
    - "../../crates/cheaper-tools/src/tools/exec/mod.rs"
    - "../../crates/cheaper-core/src/policy/schema.rs"
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
- **per-doc 訂正表** — 16 refine 対象 + guide stale 4 箇所 +
  off-list 追加分（§(d-2) — opportunistic doc 9 件・test-spec
  6 件）の各々につき、実測 anchor・旧記述・新記述（または注記
  内容）・訂正方式（in-place vs 注記追記 vs in-place
  annotation）を固定する。全 anchor は本 branch
  `feature/issue-169-docs-honesty-sweep` で実測済み。
- **migration guide の構成契約** — `docs/guides/migration.md` の
  節構成・各 issue 節の必須項目を固定する（front matter 無しの
  guide 規約 — `release.md`・`distribution.md` と同形）。
- **front-matter backlink 規約** — commit 9d76d40 の確定規約
  （cross-chain の意味 link は `related`）を 16 対象 + off-list
  追加の front-matter 付き doc へ適用する編集契約として明文化
  する（§(b) の path 規約を含む）。

## Scope

### In Scope

- refine 注記の canonical 形・配置規則・front-matter backlink 規約
  の設計（ADR D3・commit 9d76d40 先例の適用）。
- 16 refine 対象 doc（ADR 7 件・spec/design 9 件）の per-doc 訂正表
  — 実測 anchor・旧記述・訂正方向・注記内容。
- off-list 追加分の訂正（§(d-2)）— spec Out of Scope :161-167 の
  opportunistic 機構で発見済みの同型 stale 9 件（ADR 3・spec 1・
  design 4・skill 1）と、#168 IR が本 sweep へ明示 defer した
  test-spec stale `trusted` window 3 file + deny 系同型 test-spec
  3 file（test-spec は charter 内 — §(d-2) 参照）。
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

- **doc 編集作業そのもの** — 本 doc は設計成果物であり、全対象
  doc（16 対象 + §(d-2) 追加分）・migration guide の編集は後続
  plan/task が担う。
- **コード変更** — sweep で code 側の不整合（doc の記述が意図
  どおり正しく、実装がそれを満たさないケース）が見つかった場合は
  別 issue 化する。本 design の調査では発見しなかった — 実測
  inventory（discovery §実測 inventory）が示す stale は全て
  「doc が実装に追いついていない」側。
- **16 対象一覧外の doc 全般の総書換え** — impl/ir・plans・tasks・
  handoff・過去 discovery の history 記録は対象外。一覧外で発見
  した mandatory 内容審査前提の記述は軽微な注記で補ってよいが、
  一覧対象の完遂を優先する（spec Out of Scope 準拠 — 発見済み
  候補は §(d-2) で opportunistic 追加として正式化済み）。
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
| ADR（決定記録） | 本文は保持し注記追記のみ（履歴非改竄） | **汎用規則 = ファイル末尾・既存の最終節の直後**に `## Refine 注記` 節を 1 つ追記（既存の最終節 — `More Information`・`Verification`・`更新履歴`・`Confirmation` 等 doc ごとに異なる — の後に置き、既存節の内部構造を乱さない） |
| design | ADR と同じ（注記追記が基本） | 同じ汎用規則 — 最終節名は doc ごとに異なる（多くは `## Verification Notes` だが、D-2 `workflow-resource-inference.md` のように `## Coordinator review amendments` で終わる doc はその節の**後**に置く — 「最終節の後」が規則であり特定節名の後ではない） |
| spec（living contract） | 要件本文を **in-place** で現在の真実へ訂正。訂正行には `（issue #16X — …）` の inline 注釈を付す（route spec R1/R12/R15/R34 の #166–#168 先例）。残存差分が大きい doc は末尾へ同形の `## Refine 注記` 節を併置してよい | 要件本文中（in-place）＋ 任意の末尾節 |
| test-spec（行為保証の記録） | Rule/Example/Guarantee 本文は保持し、**in-place annotation note** で補正（履歴非改竄 — bookkeeping sweep としての最小介入） | 該当 bullet / Example 行の直下へ `> （issue #169 注記 — <現行の真実>）` の blockquote を挿入。同一 falsified 機構を共有する連続 bullet は cluster 末尾へ 1 本に集約可（§(d-2) 参照 — file-end 節は設けない） |
| guide・skill | 現在の真実へ **in-place** 訂正 | 該当箇所の直接書換（refine 注記節は設けない — 利用者向け単一の真実） |

**front-matter backlink 規約（spec R6・ADR D3）**: 16 対象 doc +
off-list 追加分のうち front matter を持つ全 doc（ADR・design・
spec・test-spec — guide・skill は front matter 無しのため対象外）
に対し、cross-chain の意味 link は `relations.related` へ
追加する（commit 9d76d40 の確定規約 — `refines`/`refined-by` は
chain 内 lineage relation であり cross-chain edge は
route_graph pollution を再発させる）。追加する link（対象 doc の
配置 dir から見た相対パスで書く）:

- spec: `../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
- ADR: `../adr/documentation-honesty-sweep-and-migration-guide.md`
- migration guide: `../guides/migration.md`
- 本 design:
  - `docs/designs/` 配下の対象 → `./documentation-honesty-sweep-and-migration-guide.md`
  - `docs/adr/`・`docs/specs/`・`docs/test-specs/` 配下の対象 → `../designs/documentation-honesty-sweep-and-migration-guide.md`

`docs/adr/`・`docs/specs/`・`docs/designs/`・`docs/test-specs/` は
いずれも `docs/` 直下のため、異 dir 間は `../<dir>/<file>`、同 dir
は `./<file>` が規則。`refines`/`refined-by`/`supersedes`/
`superseded-by` は新規に一切張らない。superseded status の変更も
行わない。

### (c) ADR 7 件 — per-doc 訂正表（全件「注記追記」方式）

いずれも本文は決定時点の記録として保持し、§(b) 規約の末尾
`## Refine 注記` 節 + `related` backlink を追加する。anchor は
実測 file:line。見出しの `ADR1`〜`ADR7` は本 doc 内の行ラベル
（`G-`/`S-`/`D-`/`TS-` と同じ便宜名 — artifact id ではなく、
`ADR-<数字>` 形は check-doc-ids の legacy-id 検出に抵触するため
hyphen を付さない）。実 artifact id は各見出し括弧内。

#### ADR1 `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`（ADR-034EMgaatFhsXjR97kH14c）

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

#### ADR2 `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md`（ADR-034r05iHlWosDpJ3DpG3XW）

| anchor | stale 記述 |
|--------|-----------|
| :96 | 「Runtime Invariants（§31）: …filesystem access は workspace 内のみ、symlink escape 禁止、unknown tool 禁止、shell なし、network tool なし…」— spawn 子プロセスまで及ぶ印象 |

注記内容: invariants の適用範囲は **cheaper 所有の canonical
surface + 実行契約**（file tool の workspace boundary・denyPaths・
宣言制約・lifecycle）に限る。spawn した子プロセスの fs 副作用は
OS sandbox 無しでは強制不能（canonical disclaimer 項目 1・2）。
3 層 capability モデル（Worker / Policy / Invariants）自体は維持。

#### ADR3 `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md`（ADR-034r05iHlWosDpO699OwYF）

| anchor | stale 記述 |
|--------|-----------|
| title・:51・:55 | 「sandboxed tool」・「`cwd` = workspace root = filesystem security boundary」— canonical tool を越える containment と読み得る |

注記内容: 「sandboxed tool」は canonical file tool（write/edit/ls/
read/find/grep）の workspace boundary 強制を指し、spawn した子
プロセスの fs 副作用には及ばない。`command`/`process` の子
プロセスは OS sandbox 無しで動作する（#163–#168 確定後の真実
としては disclaimer 項目 1・2 が governing）。決定自体（boundary
強制を cheaper 側 tool 実装に置く）は有効。

#### ADR4 `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`（ADR-034J8eZHNPqARi25HLFOL6）

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

#### ADR5 `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`（ADR-034KrYGRHZuvOIx1d0S0nE）

| anchor | stale 記述 |
|--------|-----------|
| （実測 grep で mandatory 内容審査の明確な残留なし — 親 design 指示「表現の軽微な訂正のみ」に該当） | approvalBypass gating 自体は整合 |

注記内容（軽微・1-2 行）: 本 ADR の決定（per-target hook rewrite の
approvalBypass trusted gating）は #163–#168 と整合を保つ。
#163 の内容審査除去・#168 の `trusted` flag 再設計で周辺表現が
変わったが、approval bypass は trusted 層 opt-in のまま変更なし。

#### ADR6 `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`（ADR-034KtTzOJGqmzmBW3SwGp5）

| anchor | stale 記述 |
|--------|-----------|
| :175-209 | Deferred design sketch — grant-set 投影・viability 弱化・substitute retarget・exit mirror・record split の「#151 で採用する場合」の検討入力（採否未確定の体裁） |
| :225-229 | 「command deny-list（`command_denied_executable_re`、`command.rs:160-189` の find/sed/awk/tar/docker/python*/node/ssh 等）は grant executable 自体を恒久拒否するため、これら heavy-output command は grant を持てず routing mode に関わらず `no-grant` で unroutable のまま」— builtin mandatory deny-list 前提の恒久 unroutable 主張 |

注記内容: 採否は親 design §非 exec routing handoff で確定済み —
grant-set 投影 **採用**（#166・`project_grant` → route 宣言
`grants:[name,…]`、trusted-scoped grant のみ解決）・viability
**条件付き弱化**（#166 — byte-matched mode の argv precheck は維持、
set 内非 exec grant は存在 + pin のみ）・usage record
`{command, executed}` split **採用**（#166）・substitute-exec mode
**採用**（#167・exit mirror は「実行した command」の exit）・
workflow-declared grants **不採用**・Option E exec-less は引き続き
**open**。:225-229 の「deny-list が python/ssh/tar 等の grant を
恒久拒否し routing mode 不問で `no-grant` unroutable のまま」も
#163/#164 で falsify 済み — builtin deny-list は除去され deny は
opt-in `security.execDenyPatterns`（既定 `[]`）宣言時のみ効くため、
既定ではこれらの executable も grant が宣言・pin 可能なら
route-eligible である（`no-grant` は deny 該当ではなく grant
非宣言・非 eligible・pin 不能・argv 不一致の viability 結果）。
`command_denied_executable_re`・`command.rs:160-189` への参照
自体も #163/#164 で機構除去済みの stale コード参照。

#### ADR7 `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md`（ADR-034zGZpjoQlppwkkOs5gur）

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
| :82 | 「writeScope を宣言した worker に対して `command` を fail-closed で reject するかは Deferred Design Concern とする」— 未決定の体裁 | 決定・強制済みへ訂正: writeScope 宣言 worker が `command`（`!fixed` grant surface）を expose すると scope 生成が `POLICY_VIOLATION`（"command is incompatible with worker writeScope" — `registry.rs:1167`）で fail closed（workers.md:54-67 と整合 — issue #169）。`process` task（`fixed` grant）側の `outputs ⊆ writeScope` coverage tri-state も workers.md:61-67 の規約を写す |
| :104 | Deferred Design Concerns 内の同趣 — 「writeScope を宣言した worker に対して `command` を fail-closed で reject するかは design phase で確定する」 | 同上 — deferred から「確定済み」へ移す（design phase で fail-closed が採用・実装済み — `registry.rs:1167`、guide 反映済み workers.md:54-67。issue #169 注記を付す） |

末尾 `## Refine 注記` 併置可（差分が大きいため推奨）。trusted
boundary・pin・argv pattern・cwd/env 境界・lifecycle 要件は維持
（:81 の OS sandbox 非該当記述は既に正直 — 保持）。

#### S-2 `docs/specs/invocation-scoped-command-grants.md`（SPEC-034FlOz4hiN9m0nQW7lBN2）— in-place

| anchor | stale 記述 | 訂正方向 |
|--------|-----------|----------|
| :60 | 「deny list・argv pattern・executable pinning・env/cwd 規則」を `resolve_command_policy` 系統から継承 | 「deny list」を「`security.execDenyPatterns`（opt-in・既定 `[]`）宣言時の条件付き検査」へ条件化 |
| :64・:70 | :64「`command` tool は default deny で、grant は trusted layer（builtin + user-scope policy）でのみ authoring できる」— 本 spec 自身が :144 で確定した合成 invocation layer（第 3 の trusted 供給元）を Why Now 節で未反映・自相矛盾。:70「手書き YAML は validation 無しで schema・deny list・argv pattern の誤りを実行時まで検出できない」前提 | :64 → trusted 供給元を「builtin + user-scope + 合成 invocation layer（`--grant`）」の 3 源へ訂正（issue #105・ADR-034FlTS3BaiV3WKz8bX1Xv — issue #169 注記付き）。:70 → deny 検査を「`execDenyPatterns` 宣言時のみ」へ条件化 |
| :85 | 「同一の runtime 挙動（deny list・argv pattern・executable pinning…）」 | 「（argv pattern・executable pinning・`security.execDenyPatterns` 宣言時の deny 検査…）」へ |
| :93 | 「静的検査（schema + deny list + argv compile + env/cwd 規則）」 | 「schema + argv compile + env/cwd 規則（+ `execDenyPatterns` 宣言時の deny 照合）」 |
| :105 | 「deny list・argv match・managed-process lifecycle の意味論は不変」 | deny の意味論を opt-in 条件へ（機構は実在・既定では非活性） |
| :118 | 「deny list に該当する executable を含む grant spec は `--grant` 経由でも reject」 | 「`security.execDenyPatterns` 宣言時、deny 該当 executable を含む grant spec は `--grant` 経由でも reject（invocation 経路は deny を迂回しない）」— 機構の実在は維持、条件付き化のみ |
| :133 | 「deny list 検査（必須）」 | 「deny list 検査（`execDenyPatterns` 宣言時のみ・既定 `[]` では検査不発）」 |
| :138 | 「deny list 該当 executable や invalid argv pattern」 | 条件付記（宣言時のみ） |
| :150 | AC3「deny list 該当 executable を含む `--grant` spec は run 開始前に reject」 | AC 書換: 「`security.execDenyPatterns` 宣言時、deny 該当 executable を含む `--grant` spec は reject（issue #164 — deny は opt-in profile 前提）」 |
| :153 | AC6「invalid spec（deny list 該当…）」 | 同上の条件付記 |
| :115 | 「注入された grant はその run の `RuntimePolicy.command` の effective grant 集合に畳み込まれ」 | `RuntimePolicy.exec` へ rename（issue #165 — unified exec grant namespace、`schema.rs:134` の `pub exec: ResolvedExecPolicy`。policy file 側の `command.grants` 記述 shape は有効のまま — rename は resolved struct field のみで、本 file の `command.grants`・`command` section・`command:` layer key への言及は訂正不要） |
| :173 | 「`kind: tool` job は `command: None` で parity check を skip するが `inner.command`（invocation grant 込みの resolved policy）が再注入」 | `exec: None`（`workflow_run.rs:500-503`）・`inner.exec`（`invocation.rs:277-286` — post-#165 rename）へ。`command` section key・`command.grants` shape は file-level では有効のまま |

機構自体は実在するため「deny は opt-in profile 宣言時の条件付き
検査」への書換であり、deny の存在抹消ではない点に注意。同様に
`RuntimePolicy.command` → `.exec` は resolved struct の rename
であって、policy file の `command` section・`command.grants`
shape・invocation layer の `{"version":1,"command":{"grants":…}}`
形状は #165 以降も有効（loader が unified `exec` へ写像）—
file-level 表現の言及は訂正しない（表現 drift 回避の境界:
resolved struct 名のみ更新）。

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

### (d-2) off-list 同型 residual — opportunistic 追加 + test-spec（charter 内）

spec Out of Scope（spec :161-167）の既定機構 —「16 対象一覧外の
doc で mandatory 内容審査を前提に見える箇所は、発見した場合に
軽微な注記で補うが一覧対象の完遂を優先する」— に基づく追加分と、
#168 IR（`docs/impl/ir/trusted-route-flag-issuer-redesign.md` の
residual follow-up 節 — 「既存 test-spec 内の stale `trusted`
記述の bookkeeping sweep … #169 正直性 sweep で回収」）が本
sweep へ明示 defer した test-spec 群。いずれも discovery §実測
inventory の 16 対象一覧外だが同一 honesty class（mandatory 内容
審査・builtin deny・`trusted` self-declared shadowing・canonical
tool を越える boundary 主張・resolved rename/deferred 残余）の
ため回収する。訂正方式は doc 種別に従う（§(b)）— ADR/design は
file-end `## Refine 注記` 追記、spec は要件本文の in-place 訂正
＋ inline 注釈、skill は in-place 書換、test-spec は後述の
in-place annotation note。front matter を持つ対象（ADR/design/
spec/test-spec）は全て `related` backlink 4 件を追加する —
相対 path は §(b) の規約に従い、ADR/spec/test-spec から本 design
へは `../designs/documentation-honesty-sweep-and-migration-guide.md`
の形。

#### test-spec は charter 内 — placement convention

- **charter 根拠**: #168 IR が `route-resolution.md`・
  `substitute-exec-route-mode.md`・`route-grant-set-projection.md`
  の stale `trusted` window を「test-spec chain 側管理・#169
  正直性 sweep で回収」と明示 defer したため、test-spec は spec
  R5 の 16 対象一覧に載らなくても本 sweep の charter 内。deny
  list 系 test-spec（TS-1〜TS-3）は一覧外だが同一 honesty class
  の行為保証 contract として opportunistic 機構で併せて回収する。
- **placement convention（本 design の判断）**: test-spec は
  「issue 時点の行為保証を pin した記録」であり Rule/Example の
  flat bullet 構造を持つ。file-end `## Refine 注記` 節は anchor
  から stale bullet への読み替えを強いるため採らず、**該当
  bullet / Example 行の直下へ `> （issue #169 注記 — <現行の
  真実>）` の blockquote を in-place に挿入する**。連続する複数
  bullet が同一 falsified 機構を共有する場合は cluster 末尾へ
  1 本に集約してよい（substitute-exec-route-mode の merge
  suppressed 群のように同一 window が散在する file で実用化）。
  Rule/Example の本文は保持し注記で補正する（履歴非改竄 —
  bookkeeping sweep としての最小介入）。front matter を持つため
  `related` backlink 4 件も追加する。

#### TS-1 `docs/test-specs/policy-gated-command-tool.md`

| anchor | stale 記述 |
|--------|-----------|
| :52 | 「空 grants・pin 不能・deny 該当・workspace-resident executable は `POLICY_VIOLATION` で fail-closed」— builtin deny・workspace-resident 拒否前提 |
| :53 | 「interpreter/shell/exec-wrapper/loader deny set は declared basename・pin 時 resolved basename・spawn 前再検証の 3 適用点で評価される」— builtin deny set の無条件適用 |
| :62 | 「grant `node` は bare name でも interpreter deny で `POLICY_VIOLATION`。pin 済み path が deny basename に差し替わると spawn 前再検証で拒否」— interpreter/bare-node deny 前提 |
| :68 | 「grants が空・pin 失敗・deny 該当のとき…registration gate が閉じる」guarantee — deny 該当の無条件前提 |

注記内容: #163 で builtin deny set（interpreter/shell/exec-
wrapper/loader・native magic・workspace-resident・metachar）が
mandatory 層から除去され、#164 で deny は opt-in
`security.execDenyPatterns`（既定 `[]`）宣言時のみ deterministic
executable deny-list として再導入。:52/:68 の fail-closed gate は
空 grants・pin 不能では維持、deny 該当は `execDenyPatterns`
宣言時のみ。:53 の 3 適用点構造（policy resolve・pin・spawn 前
再検証）は opt-in deny の適用点として存続。:62 の `node` 例は
既定では PATH 解決され受理される。

#### TS-2 `docs/test-specs/invocation-scoped-command-grants.md`

| anchor | stale 記述 |
|--------|-----------|
| :47 | Purpose —「`command` tool の実行時認可意味論（deny list・argv pattern・executable pin・env/cwd 規則・managed-process lifecycle）は一切緩めない」— deny list 無条件列挙 |
| :51 | Feature —「shared validation 由来の失敗（deny 該当・argv compile・schema 違反）」— deny 該当の無条件前提 |
| :59 | Rule —「`resolve_command_policy` が走り deny list・`ArgPattern::compile`・env/cwd 規則を継承」+「typed re-resolve 後の `RuntimePolicy.command` に畳まれる」— deny 無条件 + `RuntimePolicy.command` は #165 で `RuntimePolicy.exec` へ rename |
| :74 | Rule —「deny list の 3 適用点は既存 test が無変更 green で担保」— builtin deny 除去で対応 test 群は #163/#164 で更新済み（deny 検査は `execDenyPatterns` 宣言時のみ） |
| :82 | Example —「`--grant '{"executable":"python",…}'`（interpreter deny 該当）は `INVALID_RUNTIME_POLICY` で fail」— `python` は既定では deny 該当でない（`execDenyPatterns` 宣言時のみ reject） |
| :95 | Guarantees —「`RuntimePolicy.command` の effective grant 集合」「deny list・argv pattern・executable pin…完全に同一」— rename + deny 無条件の両方 |

注記内容: deny 検査は全 anchor で「`security.execDenyPatterns`
宣言時のみ（既定 `[]` では非活性 — issue #164）」へ条件化。
`RuntimePolicy.command` は `RuntimePolicy.exec` へ rename（#165・
`schema.rs:134`）— file-level `command.grants` shape は有効の
まま。invocation layer の layer ordering・collision・attribution
規約は維持。

#### TS-3 `docs/test-specs/managed-process-task-authorization.md`

| anchor | stale 記述 |
|--------|-----------|
| :50 | Rule —「raw command/shell、未知 task、不正 cwd/env/config を拒否する」— task argv 要素の shell/metachar 内容審査前提 |

注記内容: input-level の raw command 拒否（呼出し input は
`task` 名のみで任意 command/argv を model が供給する経路非存在）
は維持。一方 task 宣言 `command` 配列の内容審査（argv 要素の
shell 名・metachar・script ext 拒否）は #163 で除去 — 宣言要素は
executable の literal argv として内容無審査で渡る。

#### TS-4 `docs/test-specs/route-resolution.md`

| anchor | stale 記述 |
|--------|-----------|
| :52-56 | Rule —「passthroughEnv・trusted を検証し」「project 層 `trusted: true` のみ下位層同名 route を shadow する(suppressed provenance を記録)」— self-declared `trusted` shadowing 前提 |
| :73-74 | Example —「user(untrusted)と project(trusted)に同名 route がある場合 project 側が採用され user 側は suppressed」— project `trusted: true` が下位層を shadow する旧 merge |

注記内容: #168 で `Route.trusted` field は削除・merge は無条件化
— project 層 route は下位層を **shadow せず**、同名衝突は常に
project 側が負けて `suppressed` provenance へ記録される
（builtin/user 層間 shadow は同一 trusted 層の宣言 override と
して従来どおり有効）。project 層 `trusted` key presence は
load 時に `InvalidRuntimePolicy` で拒否（`{label}.trusted:
not declarable in the project layer` — 値不問・strict 経路は
error・hook 経路は fail-open で route arm 不活性）。builtin/user
層の `trusted:` 宣言は受理されるが inert。

#### TS-5 `docs/test-specs/substitute-exec-route-mode.md`

| anchor | stale 記述 |
|--------|-----------|
| :124-126 | Feature —「merge は project `trusted:true` route が下位層 route を置き換えた際 displaced route を `suppressed` provenance へ記録する」 |
| :174-175 | Rule —「`trusted: true` 宣言は判別に使わない(project 自己申告の shadowing gate)」— project 層 `trusted` は判別以前に presence 自体が拒否される |
| :337-361 | Rule —「merge suppressed provenance — project `trusted:true` 置換の displaced 記録」+ 旧 semantics pin 2 test（`user_shadows_builtin_and_project_needs_trusted`・`suppressed_project_provenance_reaches_route_record`）の assert 反転記述 — #168 で merge arm 自体が除去・test も `project_layer_trusted_is_rejected`・`project_trusted_violation_disables_route_arm_fail_open` 系へ置換済み |
| :374-378 | Rule —「project `trusted:true` route が builtin/user substitute route を同一 `match.command` で shadow するとき…」— 到達不能な分岐 |
| :406-407 | Rule —「例外は displaced-trusted provenance の 2 assert 更新のみ」— #168 で当該 test は merge 無条件化へ再 pin 済み |
| :473-478 | Example —「project `trusted:true` route `[git,push]` が builtin substitute route を shadow → effective set は project route」— 現行では load 時拒否または衝突敗北 |
| :513-516 | Guarantee —「project `trusted:true` による trusted substitute route の silent displacement が `suppressed` provenance で追跡可能」 |
| :542-543 | Non-goal —「記録対象は project 層置換のみ」— #168 で `suppressed` は「衝突で負けた project route」のみを運ぶ semantics へ狭義化 |
| :566-571 | Risk —「merge の `suppressed` 記録を untrusted project 敗北だけに残すと project `trusted:true` が trusted substitute を silent に置き換えて追跡不能」— 旧 semantics 前提の risk |

注記内容（cluster 化 — 同一 falsified 機構のため merge 節・
Guarantees・Risk 各 cluster 末尾へ 1 本ずつでよい）: #168 で
`Route.trusted` field は削除され、project 層 routes.yaml の
`trusted` key presence は値不問で `InvalidRuntimePolicy`（strict
経路は load error・hook 経路は fail-open で route arm 不活性）。
merge は無条件化 — project 層 route は下位層を shadow せず同名
衝突は常に project 側が負けて `suppressed` provenance へ記録
（`SuppressedRoute` semantics は「衝突で負けた project route」へ
狭義化）。builtin/user 層の `trusted:` key は受理されるが inert。

#### TS-6 `docs/test-specs/route-grant-set-projection.md`

| anchor | stale 記述 |
|--------|-----------|
| :150-155 | Rule —「project 層の `route.grant` 単数宣言は従来どおり有効…`trusted: true` 宣言は判別に使わない(project 自己申告の shadowing gate — 判別子は宣言層 provenance)」— provenance 判別の言及自体は #168 と整合するが、project 層 `trusted` key は「判別に使わない」のではなく presence 自体が `InvalidRuntimePolicy` で拒否される点が未反映 |

注記内容: TS-4/TS-5 と同じ #168 の真実 — project 層 `trusted`
key presence は load 時拒否・`Route.trusted` field 削除・merge
無条件化。`grants` presence 拒否・宣言層制約の他の記述は維持。

#### off-list opportunistic（spec Out of Scope 既定機構 — 一覧対象の完遂優先）

| doc | anchor | stale 記述 | 訂正方式 |
|-----|--------|-----------|----------|
| `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md` | :95 | 「executable 検証（native magic / shell / shim 拒否、`minimal_env`）は従来通り」— mandatory 内容審査前提 | ADR → file-end `## Refine 注記` + `related` backlink（既存 opportunistic 項目を正式化） |
| `docs/adr/managed-process-named-task-lifecycle.md` | :59 | 「native executable を shell:false で spawn。`node` は process.execPath…Windows .cmd/.bat/.ps1 shim は拒否し…argv は shell/メタ構文を core config validation で拒否」— 内容審査・shim deny・execPath fast-path 前提 | ADR → file-end 注記 + backlink。#163 で内容審査除去（argv metachar/shell は literal argv、shim は win32 viability 制約、bare `node` は PATH 解決）・`execPath` fast-path 不実装 |
| `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md` | :62, :83, :89, :92 | 「`resolve_command_policy` 系統（schema 検査・deny list・`ArgPattern::compile`・env/cwd 規則）」「deny list・argv compile・env/cwd 規則の full validation」「grant spec の validation（deny / ArgPattern / cwd / env）」「shared validation … deny list 該当」— deny 無条件列挙 + `RuntimePolicy.command`/`policy.command` 表記（#165 rename 残余） | ADR → file-end 注記 + backlink。deny は `security.execDenyPatterns` 宣言時のみ・`RuntimePolicy.command`→`.exec` rename（file-level `command.grants` shape は有効のまま） |
| `docs/specs/cheaper-v1-worker-runtime-cli.md` | :71, :75, :105, :115 | :71「`cwd` = workspace root = filesystem security boundary」・:75 Runtime Invariants「workspace 内 access のみ、symlink escape 禁止、shell なし」・:105 AC「workspace 外への filesystem access が常に拒否される」・:115 Security AC「workspace escape…全て拒否される」— exec 子プロセスまで及ぶ falsified normative AC | spec → in-place 訂正 + inline 注釈（+ 末尾 `## Refine 注記` 併置可）: 各 claim を canonical file tool surface（write/edit/ls/read/find/grep の workspace boundary・denyPaths・writeScope）へ限定し、`command`/`process` の spawn 子プロセスの fs 副作用は OS sandbox 無しでは強制されない旨を明記（canonical disclaimer 項目 1・2 — issue #169） |
| `docs/designs/managed-process-canonical-tool.md` | :57, :85, :88 | :57「各引数に NUL/newline、shell 演算子（pipe・ampersand・semicolon・redirection・backtick・`$()`）を許可しない。argv0 の sh/bash/cmd/powershell 等 shell と Windows script shim は拒否」・:85「Windows shim 非対応」risk・:88「argv のメタ文字拒否により正規の引数内に pipe/semicolon 等を含める task も拒否」— task argv 内容審査前提 | design → file-end 注記 + backlink。#163 で task argv の内容審査除去 — 宣言 `command` 配列要素は literal argv として無審査で渡る（metachar・shell 名を含んでも拒否されない）。win32 `.cmd`/`.bat`/`.ps1` 非対応は CreateProcess viability 制約として残るが deny 機構ではない |
| `docs/designs/invocation-scoped-command-grants.md` | :74, :99, :173, :229, :327, :341 | :74「deny list・argv pattern・executable pin・env/cwd 規則・managed-process lifecycle の意味論は不変」・:99「deny list（`command.rs:346-355`）」・:173「deny list 該当」・:229「schema・deny list・argv compile・env/cwd 規則は `resolve_command_policy` 経由」・:327「deny list / argv compile が再検査するため fail-closed」・:341「deny list 該当 executable の `--grant` が load 時に reject」— deny 無条件列挙 + stale コード参照（`command.rs`・`resolve_command_policy`・`inner.command`/`command: None` 表記の #165 rename 残余を含む） | design → file-end 注記 + backlink。deny は `execDenyPatterns` 宣言時のみ・code 参照は `policy/exec.rs`/`security.rs`・`RuntimePolicy.exec`/`inner.exec`/`exec: None` 系へ rename |
| `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md` | :44-56, :85-91 | title/In Scope の「sandboxed tools」命名・:85「shell tool が存在せず全書き込みが write/edit operations を通るため…全域 snapshot は不要」・:91「外部プロセス不実行: sandboxed tool 層は rg/fd 等の外部バイナリ…に依存しない」— canonical file tool を越える containment 印象・exec surface 不在前提 | design → file-end 注記 + backlink。「sandboxed tool」は canonical file tool の workspace boundary 強制のみを指す。#101 以降の `process`・`command` exec surface は外部プロセスを spawn し、その fs 副作用は journal/write/edit operations を通らない（全域 snapshot 不要論は file tool に限定 — canonical disclaimer 項目 1・2）。file tool 層の外部バイナリ非依存（rg/fd 回避）は維持 |
| `docs/designs/overview.md` | :54-67, :79-88 | 「sandboxed tools」命名（:55/:67）・`packages/{core,runtime-pi,cli}` の TS-era 構成名・:82「workspace 外アクセス不許可 — operations.ts + resolveWorkspacePath」の invariant 行・:75「runtime-pi が…sandboxed tools と budget を wrap」— exec 子プロセスまで及ぶ印象 | design → file-end 注記 + backlink。(a) 「sandboxed tools」・「workspace 外アクセス不許可」invariant は canonical file tool surface に限定 — `command`/`process` 子プロセスの fs 副作用は OS sandbox 無しでは強制・完全観測されない（#163-168、disclaimer 項目 1・2）。(b) `packages/*`→`crates/*` の Rust 移行 drift は rust-rig-runtime-migration chain で記録済みの別系統 — 本 sweep では最小の言及に留める |
| `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` | :82-85 | 「Deny-list and argv-pattern checks are re-applied on the re-run, so a malformed or denied spec fails closed」— deny-list 無条件前提 | skill → in-place 書換: 「argv-pattern checks — and the opt-in deny-list when `security.execDenyPatterns` is declared — are re-applied」形へ条件付記（front matter 無し — backlink 不要） |
| `docs/guides/usage.md` | :207 | 「書き込み前に policy load と同一の検査（schema・deny list・argv pattern の compile・env/cwd 規則・grant/task 名衝突）」— deny list 無条件列挙 | guide → in-place: 「deny list」へ `(opt-in — security.execDenyPatterns 宣言時のみ)` の軽微な条件付記（G-9 の optional 項目 — 採用・backlink 不要） |

### (e) guides — in-place 訂正（stale 4 箇所 + disclaimer 配置 + 均質化）

guide・skill は利用者向け単一の真実なので in-place 訂正（refine
注記節は設けない）。guide 系 doc に front matter は無いため
backlink 編集も不要。

#### G-1 `docs/guides/command.md:44`

旧: 「executable は bare name または absolute path（**workspace
相対 path は拒否**）。bare name は scope 構築時点の ambient
PATH で解決し、realpath 済み canonical path を pin する。
**native magic（MZ / ELF / Mach-O）を持つ executable のみ実行
可能で、shell・script shim（`.cmd`/`.bat`/`.ps1`/`.sh`）は
`process` baseline の deny で拒否される。**」

新（shape）: 文頭の受理形列挙を現行真実へ訂正する — 「workspace
相対 path は拒否」の clause は #163 以降 **誤り** であり除去/
書き換える。`resolve_process_executable` は `/`・`\` を含む
workspace 相対 executable を `resolve_workspace_path` で
workspace boundary 内へ解決して pin し（`proc/executable.rs:
108-124` — test `pin_resolves_workspace_relative_executable_path`
`exec/mod.rs:588-608` で pin 済み。denylist 該当・
symlink escape・boundary escape のみ拒否側）、bare name は ambient
PATH 検索、absolute path は直接 viability 判定する 3 形態受理と
書く。PATH 解決（ambient PATH・realpath 済み canonical path の
pin）の記述は維持。末文を「executable の内容・名は審査しない —
spawn viability（realpath・is_file・win32 `.exe`/`.com`・POSIX
`X_OK`）のみを判定するため、shebang script・任意 interpreter・
shim 名の file も解決される。拒否する唯一の executable 審査は
opt-in `security.execDenyPatterns`」へ。同 file :100 の migration
注記（旧 regex seed）との自相矛盾を解消。

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
`usage.md`（:301-310 record split・:343-349 — deny list 検査は
opt-in 機構として実在するため本文維持。:207 の optional「(opt-in)」
条件付記は採用し §(d-2) off-list 行へ移した — 本節の対象外）、
`packages/apm/.apm/skills/
cheaper-tool-operate/SKILL.md`（child side effects not audited 明記 —
deny-list 無条件列挙の :82-85 は §(d-2) off-list 行で条件付記）。

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
| #163 | 内容審査 7 項目除去（A8 shell/shim・B6 script ext・B3 native magic・B4 workspace-resident・B5 bare `node`・A10 `argv_shell_re` metachar（call-time + task 宣言 argv の両適用点を含む）・F3 process task の native/shim 検査 — 親 design :137-144） | 従来拒否されていた宣言 exec（python/ssh/tar/.sh/workspace-resident binary/bare `node`/metachar argv）が受理される — 既定では何も拒否しない挙動変化 | 拒否近似が必要なら #164 の `security.execDenyPatterns` を user `policy.yaml` へ宣言（seed は次節） |
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
| `docs/guides/usage.md` | (d-2) off-list — :207 の「deny list」へ `(opt-in — security.execDenyPatterns 宣言時のみ)` 条件付記（G-9 の optional 項目を採用） |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | G-8(:353) 条件付記 |
| `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` | (d-2) off-list — :82-85 deny-list 条件付記 in-place |
| `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md` | ADR1 末尾注記 + `related` backlink |
| `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md` | ADR2 同上 |
| `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md` | ADR3 同上 |
| `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md` | ADR4 同上 |
| `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md` | ADR5 同上 |
| `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md` | ADR6 同上（:175-209 + :225-229 anchor） |
| `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md` | ADR7 同上 |
| `docs/adr/managed-process-named-task-lifecycle.md` | (d-2) off-list — :59 末尾注記 + backlink |
| `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md` | (d-2) off-list — :62/:83/:89/:92 末尾注記 + backlink |
| `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md` | (d-2) off-list — :95 末尾注記 + backlink（既存 opportunistic 項目の正式化） |
| `docs/specs/policy-gated-command-tool.md` | S-1 in-place + 末尾注記 + backlink（:60/:78/:79-:80/:94 + :82/:104 residual） |
| `docs/specs/invocation-scoped-command-grants.md` | S-2 in-place + backlink（deny 条件化 + :115/:173 の `RuntimePolicy.exec` rename residual） |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | S-3 backlink（残差分見つかれば軽微訂正） |
| `docs/specs/managed-process-canonical-tool.md` | S-4 in-place + backlink |
| `docs/specs/workflow-resource-inference.md` | S-5 in-place + backlink |
| `docs/specs/cheaper-v1-worker-runtime-cli.md` | (d-2) off-list — :71/:75/:105/:115 in-place + backlink（boundary 主張を canonical file tool surface へ限定） |
| `docs/designs/policy-gated-command-tool.md` | D-1 末尾注記 + backlink |
| `docs/designs/workflow-resource-inference.md` | D-2 同上（最終節は `## Coordinator review amendments` — その後に注記） |
| `docs/designs/rust-rig-runtime-migration-architecture.md` | D-3 同上 |
| `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md` | D-4 同上 |
| `docs/designs/managed-process-canonical-tool.md` | (d-2) off-list — :57/:85/:88 末尾注記 + backlink |
| `docs/designs/invocation-scoped-command-grants.md` | (d-2) off-list — :74/:99/:173/:229/:327/:341 末尾注記 + backlink |
| `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md` | (d-2) off-list — :44-56/:85-91 末尾注記 + backlink |
| `docs/designs/overview.md` | (d-2) off-list — :54-67/:79-88 末尾注記 + backlink |
| `docs/test-specs/policy-gated-command-tool.md` | TS-1 — :52/:53/:62/:68 in-place annotation note + backlink |
| `docs/test-specs/invocation-scoped-command-grants.md` | TS-2 — :47/:51/:59/:74/:82/:95 同上 |
| `docs/test-specs/managed-process-task-authorization.md` | TS-3 — :50 同上 |
| `docs/test-specs/route-resolution.md` | TS-4 — :52-56/:73-74 同上（#168 IR deferral — charter 内） |
| `docs/test-specs/substitute-exec-route-mode.md` | TS-5 — :124-126/:174-175/:337-361/:374-378/:406-407/:473-478/:513-516/:542-543/:566-571 同上（cluster 化 — charter 内） |
| `docs/test-specs/route-grant-set-projection.md` | TS-6 — :150-155 同上（charter 内） |

**opportunistic → 正式化**: 従来 opportunistic 扱いだった
`opt-in-soft-gating` ADR :95 を含む off-list 追加分は §(d-2) で
per-doc anchor・訂正方向・配置まで固定済み（spec Out of Scope
:161-167 の既定機構に基づく正式な訂正対象 — 一覧対象の完遂優先
規約は維持）。以後に発見される一覧外残留は spec Deferred と同じく
継続 sweep 対象。

**status 整合確認（編集無し・確認のみ）**: `docs/specs/README.md`
（SPEC-034L6i3XmfCaWRHAiw8TTT = proposed）・`docs/adr/README.md`
（ADR-034L6ihibvbjmSnKmSffWH = proposed）・
`docs/discovery/README.md`（DISC-034L67agpaqLudLiaNI822 =
resolved）は既に front-matter status と整合 — 実測済み。

## Data and Control Flow

- 訂正の制御順序: (1) guides in-place（G-1〜G-8 + (d-2) の
  usage.md/skill 条件付記 — 利用者向け単一の真実を先に正す）→
  (2) spec in-place（S-1〜S-5 + (d-2) の cheaper-v1 spec）→
  (3) ADR/design 注記追記（ADR1〜7・D-1〜4 + (d-2) off-list の
  ADR/design）→ (3.5) test-spec in-place annotation（TS-1〜TS-6 —
  §(d-2) の配置規約）→ (4) migration.md 新設 →
  (5) `docs/README.md` + `docs/designs/README.md` index →
  (6) `node scripts/check-doc-ids.mjs` + grep 検証。順序は依存を
  持たない便宜上のものであり、commit 分割は「guides 訂正」・
  「refine 注記 sweep（off-list・test-spec 含む）」・
  「migration guide + index」の 3 塊が自然
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
- **履歴非改竄** — ADR/design/test-spec の本文・Rule/Example は
  保持し注記（file-end 節または in-place annotation — §(b)）で
  補正。supersede・superseded status 変更なし。
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

- **注記品質のばらつき** — 16 + off-list/test-spec 追加分の doc
  へ注記を書く作業者の判断幅が残る（ADR Consequences も同認識）。
  対策: §(c)/(d)/(d-2) の per-doc 表で anchor・訂正方向・注記
  要点まで固定し、注記節の canonical 形・test-spec の in-place
  annotation 規約を §(b) で定義。
- **in-place 訂正の過剰書換** — spec 要件本文を現行真実へ書き
  換える際、維持すべき要件（trusted boundary・pin・argv pattern・
  lifecycle）まで消すリスク。対策: per-doc 表で「維持する項目」を
  明示（S-1 の trusted boundary 等）。
- **一覧外 stale の残存** — §(d-2) で発見済み 9 件 + test-spec
  6 件を正式化したが、本 slice でも一覧外残留を全捕捉はできない。
  対策: 以後の発見分は継続 sweep 対象として spec の Deferred
  Design Concerns 通り管理（新規発見の処理経路は §(d-2) と同じ
  per-doc anchor 表への追記）。
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
  審査前提の記述が残り得る（発見済み分は §(d-2) で正式化済み —
  `opt-in-soft-gating` ADR :95・off-list 8 件・test-spec 6 件） |
  以後の発見次第の軽微注記 or 継続 sweep 対象 | 範囲を広げると
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
      「shell metachar 拒否」・「workspace 相対 path は拒否」
      が**残っていない**（G-1 — absence-grep）、
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
- [ ] **issue AC4 / spec AC4** — 16 対象 doc + off-list 追加の
      front-matter 付き doc（ADR/design/spec/test-spec — §(d-2)
      一覧）すべてに refine 反映（注記節・in-place 訂正・または
      test-spec の in-place annotation blockquote）があり、
      それぞれの front-matter `related` に本 chain doc への
      backlink 追加（ADR/spec/test-spec から本 design へは
      `../designs/documentation-honesty-sweep-and-migration-guide.md`
      形 — §(b) path 規約）。かつ新規 `refines:`/`refined-by:`/
      `supersedes:`/`superseded-by:` が全対象 doc の差分内に
      存在しない（`git diff` + grep で確認）
- [ ] **§(d-2) 追加 scope の grep 検証** —
      (a) 6 test-spec file すべてに `issue #169` 注記 blockquote
      または節がある（`grep -l "issue #169"
      docs/test-specs/{policy-gated-command-tool,invocation-scoped-command-grants,managed-process-task-authorization,route-resolution,substitute-exec-route-mode,route-grant-set-projection}.md`
      が 6 件を返す）。
      (b) `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`
      の末尾注記が :225-229 の deny-list 恒久 unroutable 主張を
      訂正している（`grep -n "no-grant\|unroutable" <file>` で
      注記節内に hit）。
      (c) `docs/specs/policy-gated-command-tool.md` の :82・:104
      が writeScope+command fail-closed を「決定済み」として記述
      し Deferred 体裁を脱している（`grep -n "Deferred" <file>`
      で当該行に残存しない）。
      (d) `docs/specs/invocation-scoped-command-grants.md` の
      :115・:173 が `RuntimePolicy.exec`・`inner.exec`/`exec: None`
      へ rename 済み（`grep -n "RuntimePolicy.command\|
      inner.command" <file>` が残存しない — file-level
      `command.grants` shape の言及は残ってよい）。
      (e) `docs/designs/overview.md`・`docs/designs/workspace-
      filesystem-boundary-sandboxed-tools.md`・
      `docs/specs/cheaper-v1-worker-runtime-cli.md` の boundary/
      「sandboxed」主張が注記または in-place 訂正で canonical
      file tool surface へ限定されている（`grep -n "Refine 注記\|
      issue #169"` で各 file に hit）。
      (f) `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`
      :82-85 および `docs/guides/usage.md` :207 の deny-list 言及が
      opt-in 条件付き（`grep -n "execDenyPatterns" <file>` で
      当該箇所に条件付記）。
      (g) `docs/adr/managed-process-named-task-lifecycle.md`・
      `docs/adr/invocation-scoped-command-grants-as-a-synthetic-
      trusted-policy-layer.md`・`docs/adr/opt-in-soft-gating-of-
      policy-gated-tools-for-the-tool-operator-worker.md`・
      `docs/designs/managed-process-canonical-tool.md`・
      `docs/designs/invocation-scoped-command-grants.md` に
      `## Refine 注記` 節がある。
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

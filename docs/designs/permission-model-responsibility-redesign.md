---
id: "DESIGN-034KuDkWOmNXiBddEQ4XWK"
type: "design"
status: "draft"
title: "権限・Runtime Policy・Tool 実行制限の責務ベース再設計"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 権限・Runtime Policy・Tool 実行制限の責務ベース再設計

## Context

issue #151 の再設計。ADR-034KuqZelggTIb8Z8K9QB8 で確定した
**Option B(宣言契約モデル)** を実装形に落とす: cheaper は「宣言された
実行契約の強制」と「実行契約の正しさ」のみを責務とし、「宣言された
exec の内容を審査して reject」する 8 項目の制限を mandatory 層から
除去する。全制限の棚卸し・分類は
`docs/discovery/permission-model-responsibility-redesign.md`(46 項目、
分類 A-F)に確定済み。

責務モデル:

- **cheaper が保証する** — 宣言された制約の強制(allowlist・grant・
  argv pattern・writeScope・fs boundary・manifest・deny_paths)、
  実行契約の正しさ(shell 非介在 argv・env_clear・stdin null・cwd・
  timeout/kill・exit 規約)、プロセス lifecycle、observability、
  リソース上限。
- **workflow 作者 / operator の責務** — 「何を走らせるか」の意味的
  正当性。grant で宣言された executable が `python` でも `.sh` でも
  workspace 内 binary でも、cheaper は内容を審査しない。
- **外部の責務** — host agent・hook・OS permission・真の sandbox。
  cheaper は子プロセスの fs 副作用を confine しない(OS sandbox が
  無い限り不可能)。この限界を文書で正直に記述する。

## Scope

### In Scope

- 内容審査 8 項目(A8/A9/A10/B3/B4/B5/B6/F3)の disposition 決定と
  mandatory 層からの除去。
- `command.grants` と `process.tasks` の統合決定(F2)。
- 非 exec routing の deferred sketch(ADR-034KtTzOJGqmzmBW3SwGp5
  §Deferred design sketch)の新モデルへの mapping と採否。
- 既存 ADR/spec/design の supersede/refine 一覧の確定。
- 実装 issue 分解(title + scope + deps)。

### Out of Scope

- OS sandbox(seatbelt/bubblewrap/seccomp)の実装 — 提供しない保護と
  して文書で明記する側。
- workflow の意味的検証・新規 hardening 機構の追加。
- `trusted` route flag の発行主体移行の実装(issue 分解に含めるが、
  本 design では境界定義まで)。
- product コードの変更 — 本 doc は設計成果物であり、実装は後続
  issue 群が担う。

## Components and Boundaries

### 実行経路ごとの保証マトリクス

| 経路 | cheaper が強制する | 作者/外部の責務 |
|------|---------------------|------------------|
| **direct argv**(command tool / `cheaper run --grant`) | grant 存在・argv pattern 照合(A6)・cwd/env 宣言境界(A7)・pin + spawn 前再検証(B1/B2)・env_clear/stdin null/lifecycle(C1-C3) | executable 名の選択・argv の意味的正しさ |
| **workflow**(process/grant job) | 宣言済み task/grant のみ実行・同一契約 + manifest/writeScope 照合(E3/E4)・unknown tool fail-closed(E7) | workflow の意味的正しさ・task 宣言内容 |
| **route**(hook → routed workflow) | viability → `check_command_execution` 委譲(D2)・payload argv/env/cwd 一致(D9)・exec-input byte 一致(D11)・exit mirror(D12)・routed-exec convention(D10) | route 宣言の正当性・route 先 workflow 内容 |
| **canonical file tool** | `resolve_workspace_path` + deny_paths(E1/E2)・writeClaims ⊆ writeScope(E3)・ArtifactReader 境界(E11) | — (cheaper 所有 surface) |
| **spawn した子プロセス** | lifecycle・env・argv のみ | **fs 副作用は強制不能** — OS sandbox なし。文書で明記 |

### 制限タグ(実装上の整理単位)

制限はコード上 4 タグで整理する: `exec-contract`(実行契約、mandatory)・
`operator-grant`(権限宣言、mandatory 機構)・`optional-hardening`・
`content-audit`(除去対象)。タグ付け自体はコメント/モジュール分割レベルで
よく、新規の実行時分類器は作らない(KISS)。

## Disposition: 内容審査 8 項目

| # | 制限 | 出所 | disposition | rationale |
|---|------|------|-------------|-----------|
| A8 | grant executable の shell/shim 判定 | `policy/command.rs:372` + `process.rs:117-148` | **remove** | spawn が shell-free な限り shell 起動自体は合法契約。名前ベースの審査は改名で無限 bypass。宣言した exec の形式を cheaper が覆さない |
| A9 | `command_denied_executable_re` deny list | `policy/command.rs:160-207` | **demote → opt-in profile** | 本再設計の中核。「既知の危険名一覧」であり containment ではない。`security.execDenyPatterns`(policy schema 新節、既定=空)として operator opt-in の deterministic deny へ降格。「保護」と呼ばない |
| A10 | `argv_shell_re` メタ文字 reject | `process.rs:134-137,278`、`command/mod.rs:467-475` | **remove** | shell-free spawn では `;` `|` は単なるデータ。false positive の温床。`tokenize_shell`(D1)は hook 文字列の解析 correctness として別経路で残る(拒否ではなく viability 分類) |
| B3 | native() magic-byte 検査 | `executable.rs:74-117` | **remove** | `#!/usr/bin/env python` 等の shebang script は正当な exec 形。形式審査は実行内容と無関係 |
| B4 | workspace-resident executable reject | `command/mod.rs:214-244` | **remove** | provenance 審査は作者責務。「project の binary を trusted grant が走る」リスクは宣言した operator の責任と割り切る。文書で残余リスクを明記 |
| B5 | bare `node` reject | `executable.rs:163-173` | **remove** | deny list の退化形。旧 Node host executable 名との衝突回避は既に不要 |
| B6 | `.cmd/.bat/.ps1/.sh` 拡張子 reject | `process.rs:127-130` | **remove** | 中身は argv 解釈されるだけで害はない。正当な build script を拒否する |
| F3 | process task の native/shim 検査 | `process.rs:117-148` + `executable.rs` | **remove** | B3-B6 と同種の内容審査。同時に除去 |

**集計**: remove 7 項目(A8・A10・B3・B4・B5・B6・F3)、
opt-in profile へ降格 1 項目(A9)。mandatory 層に内容審査は残らない
(spec R2)。

### opt-in deny profile の形(A9 降格先)

policy schema に `security` 節を新設:

```yaml
security:
  execDenyPatterns: []   # 既定: 空。operator が regex 名リストを宣言すると
                         # grant executable の deterministic deny として効く
```

- 適用点は現行と同じ(grant resolve + pin + spawn 再検証)だが、
  pattern は builtin 定数ではなく operator 宣言値のみ。
- `invocation` 層・`project` 層からは設定不可(trusted 層のみ)。
- 現行の builtin 一覧は `docs` に「旧既定値」として記録し、
  必要な operator が自前で seed できる migration 例を提供する。
- 表現は「deterministic executable deny-list」。「保護」「sandbox」
  「containment」とは呼ばない。

## command.grants vs process.tasks(F2)— 統合決定

**決定: merge(統合する)。** Option D(capability-grant 統合)を採用。

理由:

- 両者は同一概念の二重実装 — 「宣言済み exec を許可する + 固定 argv +
  policy 管理」。schema・resolve・pin・spawn・テスト・文書が全て
  二重で、不整合の余地が構造的に存在する。
- 内容審査の除去で両側の審査分岐が同時に消えるため、統合の
  差分コストが最小になるタイミング。
- 統合後の policy surface・テスト・文書が実質半減する。

統合形:

- 単一の `command.grants` モデルへ `process.tasks` を畳み込む。
  grant = `{name, executable, argv contract, cwd/env 宣言}` で、
  現行 task が持つ情報は grant の部分集合。
- tool surface は `command` に統一。`process` tool は loader 側の
  互換 shim で旧 `process.tasks` 宣言を grant へ読み替え
  (deprecation warning 付き)し、policy schema では一定期間
  `process` 節を受理してから除去する 2 段階 migration。
- route の `project_grant` 投影・viability・record は統一 grant
  モデルだけを見る形になるため、後続の grant-set 投影とも整合する。

## 非 exec routing handoff(deferred sketch の mapping)

ADR-034KtTzOJGqmzmBW3SwGp5 の Deferred design sketch は本再設計の
必須 input。新モデルへの写像と採否:

| sketch 要素 | 新モデル上の扱い | 採否 |
|-------------|-------------------|------|
| grant-set 投影(`project_grant` → route 宣言 `grants:[name,…]` の set 投影) | 統一 grant モデル上で `project_grant` を複数名へ一般化。confused-deputy 半径は declared grant argv union に明示的に閉じる。D2 routing≠authorization を維持 | **採用**(issue 4) |
| viability 再審査(存在・pin 解決可能な grant set 検査へ弱め、argv 照合は job 実行時へ委譲) | 「実行されない command」の認可を調べる形骸化を解消。direct `workflow run --grant` と同じく実行時照合へ移す | **採用**(issue 4) |
| substitute exec(route が `substitute:{grant,argv}` を宣言、viability の `check_command_execution` を原 argv でなく substitute へ向ける) | precheck の strict さを保ったまま「宣言通り実行する command」を検査。exit mirror は substitute の exitCode を mirror するだけで機構維持 | **採用**(issue 5) |
| exit-code mapping(実行した command の exit を mirror) | substitute 宣言あり→ substitute exit、なし→ 原 command exit。D11 mirror 機構はそのまま | **採用**(issue 5) |
| usage record field split(原 command 文字列と実 exec argv を別 field) | `kind:"route"` record を `{command, executed}` に拡張。「実行されなかった command」を command attribution にしない | **採用**(issue 4) |
| workflow-declared grants(project 層が grant を authoring) | trusted-layer-only の grant 境界の迂回であり D2/C9 を壊す | **不採用**(明示拒否) |
| Option E exec-less route(exec job 不要、`outputs.exitCode` の author 宣言値 mirror) | author による成功偽装を opt-in で受容する判断が前提。product 需要が未確認 | **open** — 需要確認まで実装しない。route `grant` field は firing gate として留保される設計だけ確定 |

残る open 事項: Option E の採否、`trusted` flag 発行主体の
operator 層への移行(issue 6 の設計論点)。

## Supersede / Refine 一覧

### refine 対象(既存 ADR)

| ADR | ID | 変更内容 |
|-----|-----|----------|
| `command-grant-trust-boundary-and-managed-process-reuse` | ADR-034EMgaatFhsXjR97kH14c | trusted-layer grant 機構は維持。deny list・workspace-resident reject・メタ文字審査の mandatory 審査部分を除去。opt-in `execDenyPatterns` への降格を追記 |
| `worker-definition-runtime-policy-runtime-invariants-capability` | ADR-034r05iHlWosDpJ3DpG3XW | 3 層モデル維持。invariants に置く制限の中身を再配置 — 実行契約のみ invariants、内容審査は除外 |
| `workspace-filesystem-boundary-cheaper-sandboxed-tool` | ADR-034r05iHlWosDpO699OwYF | canonical tool の boundary 強制は維持。「子プロセスの fs 副作用には及ばない」を明記 |
| `pretooluse-auto-routing-via-route-resolver-and-workflow-delegation` | ADR-034J8eZHNPqARi25HLFOL6 | C9 single-grant 投影 → route-declared grant-set 投影へ一般化 |
| `per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass` | ADR-034KrYGRHZuvOIx1d0S0nE | approvalBypass の trusted gating は整合。内容審査除去に伴う表現の軽微な訂正のみ |
| `defer-non-exec-routing-to-permission-model-redesign` | ADR-034KtTzOJGqmzmBW3SwGp5 | deferred sketch の採否が本 design で確定(grant-set・viability・record split 採用、Option E は open)。refine 注記として採否を ADR 側へ追記 |

### refine/supersede 対象(既存 spec・design)

| doc | ID | 変更内容 |
|-----|-----|----------|
| `specs/policy-gated-command-tool` | SPEC-034EMbEGBOWM4CAM6YSWm6 | deny list・workspace-resident 要件を除去/降格。**refine**(deny 要件の opt-in 化) |
| `specs/invocation-scoped-command-grants` | SPEC-034FlOz4hiN9m0nQW7lBN2 | grant 検証要件から内容審査分を除去。**refine** |
| `specs/pretooluse-route-resolver-and-workflow-delegation` | SPEC-034J7SxWvGL3oSbeBpo8a3 | grant semantics を grant-set 投影前提へ。**refine** |
| `designs/policy-gated-command-tool` | DESIGN-034EMWNJbQpMB0N3ZTXM5A | deny list 部分が本 design に置き換わる。**supersede**(bypass surface 記録は正しかった点を残す) |

## 実装 issue 分解

依存順に 7 issue。各 issue は title + scope + deps。

1. **exec-contract cleanup: 内容審査 7 項目を mandatory 層から除去**
   - scope: A8・A10・B3・B4・B5・B6・F3 の検査分岐と呼出しを
     `check_command_execution`・pin・executable resolve・task resolve
     から除去。`tokenize_shell` の解析用途(D1)は維持。
   - deps: なし(起点)。
2. **opt-in `security.execDenyPatterns` profile(A9 降格)**
   - scope: policy schema に `security` 節追加(trusted 層のみ)、
     deny pattern の grant resolve/pin/spawn 再検証フック、既定=空、
     旧 builtin 一覧の migration 例を docs へ。
   - deps: 1。
3. **command.grants / process.tasks の単一 exec grant モデル統合**
   - scope: schema 統合・tool surface を `command` に統一、
     loader 互換 shim(旧 `process.tasks` → grant 読み替え +
     deprecation warning)、2 段階 migration。
   - deps: 1。
4. **route/hook の審査依存整理 + grant-set 投影 + record split**
   - scope: viability の unsafe 分岐が内容審査を呼ぶ経路の整理、
     `project_grant` → route 宣言 `grants:[name,…]` set 投影、
     `kind:"route"` record の `{command, executed}` field 分離。
   - deps: 1, 3。
5. **substitute-exec route mode**
   - scope: route `substitute:{grant,argv}` 宣言、viability の検査対象を
     substitute へ retarget、exit mirror の remap。Option E(exec-less)は
     本 issue では実装しない(open 事項)。
   - deps: 4。
6. **trusted route flag 発行主体の再設計(D4)**
   - scope: `trusted:true` を project 自身が宣言できる現状の再設計、
     trust 判定主体を operator/user 層へ移す方式の決定と実装。
   - deps: 1。
7. **文書正直性 sweep + breaking-change migration guide**
   - scope: 「boundary/sandbox/protection」表現と実強制範囲の整合
     (guides・refine 対象 ADR/spec)、破壊的変更一覧(deny list 削除・
     process.tasks deprecation・grant 形状)と移行手順の告知。
   - deps: 1-6。

## What stays invariant(変更しない保証)

- **observability** — usage record・route log・ComposedReader/
  ArtifactReader 境界(D13/E11)、condense worker の bounded read。
- **リソース上限** — timeout/killGrace/heartbeat/maxOutputBytes/
  accepted exit codes(C3)、maxToolCalls・abort grace・concurrency(C8)。
- **プロセス lifecycle** — env_clear + minimal_env(C1)、stdin null・
  no TTY・process group・windowsHide(C2)、pin + spawn 前再検証
  (B1/B2)、fail-closed admission(C4)。
- **宣言制約の強制** — argv pattern 言語(A6)、cwd/env 宣言境界(A7)、
  writeScope/writeClaims(E3)、manifest 照合(E4)、capability 交差(E6)、
  deny_paths(E2)、canonical tool の workspace containment(E1)。
- **信頼境界機構** — 層順序 + fail-loud load(A1)、invocation 層の
  key 制限(A2)、ProjectPartial の command/route 節排除(A3)、
  RestrictConcurrency(A4)、CLI `--restrict` narrow-only(A5)、
  trusted-only approvalBypass(D14)、dup match 隔離(D5)、
  `target.path` 層起点相対(D6)、passthroughEnv allowlist(D7)。
- **routing ≠ authorization** — viability → `check_command_execution`
  委譲(D2)。payload 検証(D9)・exec-input 一致(D11)・exit mirror(D12)。
- **routing ≠ sandbox** — `prepare_exec_cwd`(D15)が workspace 外
  cwd を許容する唯一の経路であることは文書で明示継続。

## Data and Control Flow

- grant resolve: `command.grants[name]` → executable PATH 解決 →
  canonical realpath pin(B1) → argv pattern 照合(A6)・cwd/env 宣言境界
  (A7) → spawn(C1-C3)。内容審査分岐は存在しない(A9 を opt-in で
  有効化した場合のみ `execDenyPatterns` 照合が resolve/pin/spawn 前に
  挿入される)。
- route run: hook payload → route resolve(viability = grant set 存在 +
  pin 解決可能性の検査) → route-declared grant-set 投影 →
  routed workflow 実行(exec-input byte 一致 D11) → exit mirror(D12)
  → `kind:"route"` record `{command, executed}`。
- loader: 旧 `process.tasks` 節 → deprecation warning → grant 形状へ
  読み替え → 統一 grant 集合として resolved policy へ merge。

## Non-Functional Constraints

- **後方互換**: deny list 削除・process.tasks deprecation は破壊的
  変更。migration guide(issue 7)で告知。旧 `process` 節は loader
  shim で読み替えのため既存 policy.yaml が即死しない。
- **正直性**: 「提供しない保護」を文書で誇張しない。deny profile は
  deterministic deny-list であり containment ではない。
- **回帰防止**: 契約側の既存テスト(pin・env_clear・stdin null・
  lifecycle・fs boundary・writeScope)は無変更で pass し続けること。

## Risks and Trade-offs

- **「既定で危険を弾く」網の喪失** — opt-in profile(A9 降格)で
  補完可能。元々 bypass 無限で完全な防御ではなかったため実損は小さい。
- **workspace-resident binary の宣言実行** — 「宣言した以上 operator
  の責任」と割り切る。残余リスクは migration guide + docs で明記。
- **project が自ら `trusted` を宣言できる現状** — issue 6 で発行主体を
  operator 層へ移すまで残存。現行と同じ曝露であり悪化はしない。
- **統合の migration コスト** — loader shim で 2 段階化し、単一
  リリースでの強制書換えを避ける。

## Deferred Design Concerns

- Option E exec-less route | product 需要が未確認 | lint-only/analysis
  route の需要が確認された時点で再検討 | author 宣言 exit を信用する
  設計が前提になり、未決のまま実装すると成功偽装の hole を開ける
- `trusted` flag 発行主体 | issue 6 へ分割 | route 信頼モデル設計と
  同時に扱う | 放置すると project 自身が trust を宣言し続ける
- OS-sandbox backed mode | 本再設計の非目標 | 真の containment 需要が
  発生した場合 | 「提供しない」を正直に文書化することで対処

## Verification Notes

- [ ] 内容審査項目が mandatory 層に残っていないこと(discovery 分類表
      との照合)
- [ ] `python`/`ssh`/`tar`/`.sh`/workspace binary を宣言した grant が
      opt-in deny 未指定で拒否されない統合テスト(issue 1/2 の AC)
- [ ] 既存契約テスト(pin・env_clear・stdin null・lifecycle・fs
      boundary・writeScope)が回帰なく pass
- [ ] viability が authorization(grant/allowlist)を呼ぶが内容審査を
      呼ばない構造(issue 4 の AC)
- [ ] 破壊的変更一覧が migration guide に列挙されていること
      (issue 7 の AC)

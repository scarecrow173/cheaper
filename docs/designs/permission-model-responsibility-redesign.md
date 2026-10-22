---
id: "DESIGN-034KuDkWOmNXiBddEQ4XWK"
type: "design"
status: "approved"
title: "権限・Runtime Policy・Tool 実行制限の責務ベース再設計"
created: "2026-10-22"
updated: "2026-10-23"
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
  related:
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "../specs/policy-gated-command-tool.md"
    - "../specs/invocation-scoped-command-grants.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/workflow-resource-inference.md"
    - "./policy-gated-command-tool.md"
    - "./workflow-resource-inference.md"
    - "./rust-rig-runtime-migration-architecture.md"
    - "./pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "./managed-process-canonical-tool.md"
    - "./invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 権限・Runtime Policy・Tool 実行制限の責務ベース再設計

## Context

issue #151 の再設計。ADR-034KuqZelggTIb8Z8K9QB8 が選択した
**Option B(宣言契約モデル)** を実装形に落とす: cheaper は「宣言された
実行契約の強制」と「実行契約の正しさ」のみを責務とし、「宣言された
exec の内容を審査して reject」する 8 項目の制限を mandatory 層から
除去する。なお同 ADR は status `proposed` であり ratification 未了 —
本 design の結論は ADR の ratify を前提とし、issue 1 の着手前に
accepted へ進める必要がある。全制限の棚卸し・分類は
`docs/discovery/permission-model-responsibility-redesign.md`(53 項目 —
36 required / 7 operator-choosable / 1 optional-hardening /
8 content-audit + F2 構造的重複、分類 A-F)に確定済み。

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
- `command.grants` と `process.tasks` の統合決定(F2)— Merge
  Fidelity(inputs/outputs・fixed argv・project 層経路・
  writeScope coverage・`tool:"process"` 互換)の個別決定を含む。
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
| A10 | `argv_shell_re` メタ文字 reject | `process.rs:134-137,278`、`command/mod.rs:467-475`、`command.rs:249` | **remove** | shell-free spawn では `;` `|` は単なるデータ。false positive の温床。`command.rs:249` は `ArgPattern::compile` が宣言済み argv pattern へ同一 regex を適用する適用点で、同一 audit class として同時除去。`tokenize_shell`(D1)は hook 文字列の解析 correctness として別経路で残る(拒否ではなく viability 分類) |
| B3 | `native()` magic-byte 検査 | `executable.rs:74-117` | **remove** | `#!/usr/bin/env python` 等の shebang script は正当な exec 形。形式審査は実行内容と無関係。除去後の正直な記述は下記補足参照 |
| B4 | workspace-resident executable reject | `command/mod.rs:214-244,273-278` | **remove** | provenance 審査は作者責務。「project の binary を trusted grant が走る」リスクは宣言した operator の責任と割り切る。文書で残余リスクを明記。`verify_pinned_executable` の `within_workspace` 再検査(273-278)も同一審査として除去するが、pin-equality 検査(261-266)は B2 の TOCTOU 対策として残る |
| B5 | bare `node` reject | `executable.rs:163-173` | **remove** | deny list の退化形。旧 Node host executable 名との衝突回避は既に不要 |
| B6 | `.cmd/.bat/.ps1/.sh` 拡張子 reject | `process.rs:127-130` | **remove** | 中身は argv 解釈されるだけで害はない。正当な build script を拒否する |
| F3 | process task の native/shim 検査 | `process.rs:117-148` + `executable.rs` | **remove** | B3-B6 と同種の内容審査。同時に除去 |

**集計**: remove 7 項目(A8・A10・B3・B4・B5・B6・F3)、
opt-in profile へ降格 1 項目(A9)。mandatory 層に内容審査は残らない
(spec R2)。

**B3 補足(正直な restatement)**: `native()` の magic-byte 検査を
除去すると、shebang を持たない +X の text file を execve した場合に
ENOEXEC となり、glibc `execvpe` 系の既定動作として `/bin/sh` での
fallback が起こり得る。shebang 付き script は kernel の interpreter
機構で直接実行されるため問題ないが、「shell-free spawn」の invariant
は **spawn API 層でのみ真** であると明記する: cheaper が使う spawn
API は shell を介さず argv を直接渡す。ただし ENOEXEC 時に libc が
`/bin/sh` fallback する挙動は OS/libc 側の既定動作であり cheaper の
保証対象外。narrow 化した is-native gate を残す選択肢もあるが、
形式審査は実行内容と無関係かつ bypass 可能であり、「保護らしき
何か」を残すより OS の既定動作として正直に記述する方を採る
(issue 方向性は pseudo-protection より正直性を優先)。

**A10 補足(観測差分)**: `\0` を含む argv は現行では metachar reject
の一環として `POLICY_VIOLATION` で事前拒否されるが、除去後は
spawn 層の OS/NUL エラーへ縮退する。拒否自体は維持されるが
error 分類が変わる軽微な観測差分として記録する。

### opt-in deny profile の形(A9 降格先)

policy schema に `security` 節を新設:

```yaml
security:
  execDenyPatterns: []   # 既定: 空。operator が regex 名リストを宣言すると
                         # grant executable の deterministic deny として効く
```

- 適用点は現行と同じ(grant resolve + pin + spawn 再検証)だが、
  pattern は builtin 定数ではなく operator 宣言値のみ。
- `security` 節の authoring は trusted 層(builtin/user)に限る —
  invocation 層は keys ⊆ `{version, command.grants}` の構造 guard
  (`load.rs:119-150`)で `security` を書けず、project partial
  schema も同様に受け付けない。
- **ただし deny の効果は invocation 合成 grant(`--grant` flag
  経路)にも等しく及ぶ** — deny 評価は `resolve_command_policy`
  (declared basename)・`pin_command_grant`(resolved basename)・
  `verify_pinned_executable`(spawn 前再検証)の既存適用点で
  layer 由来情報を持たない resolved `command.grants` map 全体に
  対して効くため、operator deny は invocation grant を exempt
  しない。
- 現行の builtin 一覧は `docs` に「旧既定値」として記録し、
  必要な operator が自前で seed できる migration 例を提供する。
- 表現は「deterministic executable deny-list」。「保護」「sandbox」
  「containment」とは呼ばない。

## command.grants vs process.tasks(F2)— 統合決定

**決定: merge(統合する)。** Option D(capability-grant 統合)を採用。

理由:

- 両者は同一概念の二重実装 — 「宣言済み exec を許可する + argv
  契約(固定 or pattern)+ policy 管理」。schema・resolve・pin・
  spawn・テスト・文書が全て二重で、不整合の余地が構造的に存在する。
- 内容審査の除去で両側の審査分岐が同時に消えるため、統合の
  差分コストが最小になるタイミング。
- 統合後は resolve/pin/spawn/manifest/テストの実装系が単一化する
  (authoring spelling は A3 境界のため `process` 節を残す —
  後述 M3)。

統合形:

- 単一の `command.grants` モデルへ `process.tasks` を畳み込む。
- tool surface は `command` に統一し、`tool:"process"` は merged
  grant を解決する compat alias として残る(M5)。
- route の `project_grant` 投影・viability・record は統一 grant
  モデルだけを見る形になるため、後続の grant-set 投影とも整合する。

### Merge Fidelity — task 情報の保存(前提訂正)

初稿の「現行 task が持つ情報は grant の部分集合」は**誤り**。
実際は逆で、`ProcessTask` は grant が持たない情報を持つ:

- `ProcessTask.command` は **固定 argv 全体**(`command[0]` が
  executable、`command[1..]` が argv)— model は task 名以外を
  供給しない。
- `ProcessTask.inputs` / `outputs` は
  `process_task_resources`(`process.rs:392-396`)で per-path の
  `reads` / `writes` / `produces` manifest へ変換され、
  declared-path deny 検査(`prepare.rs:71-95`)・`produces`
  ownership・writeScope coverage 検査(`prepare.rs:96-122`)を
  駆動する。
- `outputs` が空の task は `ClaimMode::Shared`
  (`process/mod.rs:108-112`)、非空なら `Exclusive`。

対する grant 側は workspace root subtree の monolithic
reads+writes(`produces` 無し — `command.rs:488-498`)と常時
`ClaimMode::Exclusive`(`command/mod.rs:135-143`)のみ。
単純な畳み込みは task 側の情報を落として manifest・writeScope・
ClaimMode の semantics を壊すため、以下を個別に決定する。

**(M1) 統一 grant schema は optional `inputs`/`outputs` を持つ —
採用。** task の resource 宣言を保持し、manifest 生成・writeScope
coverage・ClaimMode 導出の semantics を統一モデル上で維持する。
`inputs`/`outputs` を持つ grant は per-path manifest
(`process_task_resources` 同等)を生成し、持たない grant は従来の
conservative root manifest を使う。merge は descope しない。

**(M2) fixed-argv は `fixed: true` marker で表現 — 採用。** 統一
grant に `fixed: true`(仮名)を設け、fixed grant は task 由来の
宣言 argv のみを実行し、model が供給する tool input は `{name}`
のみ(`args` 不許可)。「task が固定 argv」「grant が argv
pattern + model 供給 args」という 2 種の契約を同一 schema・同一
exec surface で表現する。`process` tool を独立した固定 argv 実装
surface として残す案は、統合目的(実装の単一化)に反して二重経路を
残すため不採用(tool 名自体は M5 の compat alias として残る)。

**(M3) project 層の exec 宣言経路 — `process` 節を
project-authorable に維持し、project 宣言 `process.tasks` を
project-scoped grant namespace へ写像。** 現状
`policy_project_partial_schema`(`schema.rs:340-355`)は `process`
節を許し `command` 節を除外する — task を trusted-only な grant
へ畳むだけだと project の唯一の exec 宣言経路が消える破壊的変更
になる。かといって project task を route grant 参照を満たす通常
grant へ写すと A3 の trusted-layer invariant を弱める。決定:
project 層は引き続き `process` 節でのみ exec を宣言し
(`command` 節は引き続き除外)、loader がそれを provenance
`project` の grant へ写像する。route の grant 参照
(`route.grant`・grant-set `grants[]`・`substitute.grant`)は
trusted-scoped grant のみを解決し、project-scoped grant は
route の firing/authorization を満たせない — 両経路を保全する。

**(M4) writeScope × exec — coverage check 経路を維持。** 現行は
`writeScope` × `command` が runtime 構築時に fail-closed
(`registry.rs:1133-1142`)、`process` 側のみ task outputs ⊆
writeScope の per-task coverage check(`prepare.rs:96-122`)で
許可される。統合後は writeScope 下の worker が exec できるのは
**`fixed: true` かつ宣言 `outputs ⊆ writeScope` の grant のみ**
とする — coverage check を呼び出し grant 単位で適用し、
argv を供給する通常 grant は writeScope 下で引き続き拒否
(C5 の「resource 層が観測不能な副作用を宣言させない」invariant
を per-call に写す。migrated task は現行 process semantics を
保持)。

**(M5) `tool:"process"` workflow 互換 — alias として維持。**
`{"kind":"tool","tool":"process","input":{"task":…}}` を持つ
既存 workflow(`benchmarks/scenarios/scenarios.json`・golden
fixture)が `UnknownTool` fail-closed(E7)に落ちてはならない。
`process` は canonical tool vocabulary に残し、`input.task` を
merged grant 集合の fixed grant 名へ解決する compat surface
として統一 exec 機構を通す。

これらにより、統合後の grant モデルは
`{name, executable, fixed?: argv固定, argv?: pattern,
inputs?, outputs?, cwd/env 宣言, lifecycle 宣言,
scope: trusted|project}` を保持する。`process` 節は loader 側で
`fixed: true` + provenance scope 付き grant へ写像される
authoring 経路として存続し(deprecate しない)、`tool:"process"`
は exec surface の alias として存続する — authoring/workflow
面の破壊的変更は発生せず、破壊的変更は内容審査除去側に限定
される。

## 非 exec routing handoff(deferred sketch の mapping)

ADR-034KtTzOJGqmzmBW3SwGp5 の Deferred design sketch は本再設計の
必須 input。新モデルへの写像と採否:

| sketch 要素 | 新モデル上の扱い | 採否 |
|-------------|-------------------|------|
| grant-set 投影(`project_grant` → route 宣言 `grants:[name,…]` の set 投影) | 統一 grant モデル上で `project_grant` を複数名へ一般化。grant 参照は trusted-scoped grant のみを解決する(M3 — project-scoped grant は route を満たせない)。confused-deputy 半径は declared grant argv union に明示的に閉じる。D2 routing≠authorization を維持 | **採用**(issue 4) |
| viability 再審査(**条件付き弱化**) | byte-matched mode では原 argv = 実行 argv(D11)なので `check_command_execution` による argv precheck(`resolve.rs:304-335`)は形骸化しておらず**維持する** — 無条件に弱めると argv-mismatch の現行挙動(viability=`no-grant` → route 不発 → 原 command が直接実行)が「route 発火 → exec job 失敗(原 command は走らない)」へ変わる後退になる。弱めるのは (a) substitute mode 宣言時 — 検査対象を `substitute.{grant,argv}` へ retarget(issue 5)、(b) grant set 内の非 exec grant — 存在・pin 解決可能性のみ viability で見て argv 照合は job 実行時へ委譲(direct `workflow run --grant` と同一)に限定する | **採用**(issue 4) |
| substitute exec(route が `substitute:{grant,argv}` を宣言、viability の `check_command_execution` を原 argv でなく substitute へ向ける) | precheck の strict さを保ったまま「宣言通り実行する command」を検査。`expected_input`(`route/run.rs:176` の `{name:grant,args:exec_args}`)は substitute mode では **`{name:substitute.grant,args:substitute.argv}` へ再 anchor** し、exec-input byte 一致(D11)を substitute argv へ適用する。exit mirror は substitute の exitCode を mirror するだけで機構維持 | **採用**(issue 5) |
| exit-code mapping(実行した command の exit を mirror) | substitute 宣言あり→ substitute exit、なし→ 原 command exit。D11 mirror 機構はそのまま。**ただし spec R15 の記述を修正**: route-run exit は「原 command の exit」ではなく「実行した command の exit」であり、substitute mode では親 agent が substitute の exit を原 command のものとして解釈する — semantic substitution を trusted 層 opt-in で受容する帰結として `specs/pretooluse-route-resolver-and-workflow-delegation.md` R15 を再記述済み(要件 ID は維持) | **採用**(issue 5) |
| usage record field split(原 command 文字列と実 exec argv を別 field) | `kind:"route"` record を `{command, executed}` に拡張。「実行されなかった command」を command attribution にしない | **採用**(issue 4) |
| workflow-declared grants(project 層が grant を authoring) | trusted-layer-only の grant 境界の迂回であり D2/C9 を壊す | **不採用**(明示拒否) |
| Option E exec-less route(exec job 不要、`outputs.exitCode` の author 宣言値 mirror) | author による成功偽装を opt-in で受容する判断が前提。product 需要が未確認 | **open** — 需要確認まで実装しない。route `grant` field は firing gate として留保される設計だけ確定 |

**grant anchor の規約(grant-set / substitute 下の帰属)**: 複数
grant が関わる route で帰属を一意にするため、次の anchor を固定
する。

- `expected_input` の `name` は **exec job が実際に走らせる
  grant** — byte-matched mode(grant-set route 含む)では
  `route.grant`、substitute mode では `substitute.grant`。
- deadline/timeout 等の lifecycle bound は **expected_input が
  指す実行 grant** の宣言値を使う(`command::execute` が named
  grant の `timeout`/`killGrace`/`exitCodes` を読む既存規約)。
  grant-set route では `route.grant`、substitute mode では
  `substitute.grant` が供給する。set 内の非 exec grant はそれを
  呼ぶ追加 job で自身の宣言 bound を使う。
- usage record の route 帰属(route × firing gate の認可 grant)
  は **`route.grant` に anchor** する — grant set は実行範囲の
  宣言であり attribution anchor を構成しない。`{command,
  executed}` の `executed` field が実 exec argv(byte-matched
  では原 argv、substitute では substitute argv)を記録する。

残る open 事項: Option E の採否、`trusted` flag 発行主体の
operator 層への移行(issue 6 の設計論点)。

## Supersede / Refine 一覧

本再設計で supersede する doc は無し — 影響を受ける既存 doc は
すべて refine(部分更新)で扱う。

### refine 対象(既存 ADR)

| ADR | ID | 変更内容 |
|-----|-----|----------|
| `command-grant-trust-boundary-and-managed-process-reuse` | ADR-034EMgaatFhsXjR97kH14c | trusted-layer grant 機構は維持。deny list・workspace-resident reject・メタ文字審査の mandatory 審査部分を除去。opt-in `execDenyPatterns` への降格を追記 |
| `worker-definition-runtime-policy-runtime-invariants-capability` | ADR-034r05iHlWosDpJ3DpG3XW | 3 層モデル維持。invariants に置く制限の中身を再配置 — 実行契約のみ invariants、内容審査は除外 |
| `workspace-filesystem-boundary-cheaper-sandboxed-tool` | ADR-034r05iHlWosDpO699OwYF | canonical tool の boundary 強制は維持。「子プロセスの fs 副作用には及ばない」を明記 |
| `pretooluse-auto-routing-via-route-resolver-and-workflow-delegation` | ADR-034J8eZHNPqARi25HLFOL6 | C9 single-grant 投影 → route-declared grant-set 投影へ一般化 |
| `per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass` | ADR-034KrYGRHZuvOIx1d0S0nE | approvalBypass の trusted gating は整合。内容審査除去に伴う表現の軽微な訂正のみ |
| `defer-non-exec-routing-to-permission-model-redesign` | ADR-034KtTzOJGqmzmBW3SwGp5 | deferred sketch の採否が本 design で確定(grant-set・条件付き viability 弱化・record split 採用、Option E は open)。refine 注記として採否を ADR 側へ追記 |
| `fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront` | ADR-034zGZpjoQlppwkkOs5gur | 「`process` は固定 executor・shell なし」の invariant 記述を統一 grant + `tool:"process"` alias 形へ再記述。spawn API 層の shell-free 記述と ENOEXEC→`/bin/sh` fallback の OS 既定動作の区別を反映 |

### refine 対象(既存 spec・design — supersede 対象なし)

| doc | ID | 変更内容 |
|-----|-----|----------|
| `specs/policy-gated-command-tool` | SPEC-034EMbEGBOWM4CAM6YSWm6 | deny list・workspace-resident 要件を除去/降格。**refine**(deny 要件の opt-in 化) |
| `specs/invocation-scoped-command-grants` | SPEC-034FlOz4hiN9m0nQW7lBN2 | grant 検証要件から内容審査分を除去。**refine** |
| `specs/pretooluse-route-resolver-and-workflow-delegation` | SPEC-034J7SxWvGL3oSbeBpo8a3 | grant semantics を grant-set 投影前提へ。R15 を「実行した command の exit」(substitute mode 反映)へ再記述済み。**refine** |
| `specs/managed-process-canonical-tool` | SPEC-034A8q6kmxcBsNF8zS9Cwn | task の fixed-argv 契約・inputs/outputs・「shell executable/構文拒否」要件を統一 grant schema(`fixed` + `inputs`/`outputs`)と内容審査除去へ写す。**refine** |
| `specs/workflow-resource-inference` | SPEC-034zSWvgiB9gHQpjguNJ3M | §9 の `tool:process` declaration seam(named task の inputs/outputs 明示・outputs→produces/writes 推論)は統一 grant の resource 宣言へ再配線され、undeclared process の INVALID_REQUEST 契約は fixed grant 前提で再記述。**refine** |
| `designs/policy-gated-command-tool` | DESIGN-034EMWNJbQpMB0N3ZTXM5A | deny list 部分のみ本 design に置き換わる。pin 契約(B1/B2)・argv pattern 言語(A6)・Exclusive root manifest(C7)・route-context 注入は引き続き authoritative — **refine**(supersede ではない) |
| `designs/workflow-resource-inference` | DESIGN-034zSWw1LYrRvcB471oQm2 | process declaration の inference 経路(task inputs/outputs→produces)が統一 grant 化で再配線。**refine** |
| `designs/rust-rig-runtime-migration-architecture` | DESIGN-034Bbq1qY8WFBdL26tUdBw | bare `node` reject(B5)の由来である「旧 Node host executable との衝突回避」固定ルール記述を、B5 除去に伴い訂正。**refine** |
| `designs/pretooluse-route-resolver-and-delegated-workflow-execution` | DESIGN-034JbqoCWm85pKC033fgfd | viability 分類(no-grant precheck の semantics)が条件付き弱化 + substitute retarget で変わる。**refine** |

## 実装 issue 分解

依存順に 7 issue。各 issue は title + scope + deps。

1. **exec-contract cleanup: 内容審査 7 項目を mandatory 層から除去**
   - scope: A8・A10・B3・B4・B5・B6・F3 の検査分岐と呼出しを
     `check_command_execution`・pin・executable resolve・task resolve
     から除去。A10 の範囲には `command/mod.rs:467-475`(runtime argv)
     と `process.rs:278`(task argv)に加え `command.rs:249`
     (`ArgPattern::compile` が宣言済み pattern に適用する同種審査)
     を含む。B4 の範囲には `command/mod.rs:214-244`(pin 時 2 段
     検査)と `273-278`(`verify_pinned_executable` の workspace
     再検査)を含むが、pin-equality 検査(261-266)は B2 として残す。
     `tokenize_shell` の解析用途(D1)は維持。golden fixture
     `crates/cheaper-core/tests/golden/fixtures/policy/process.json`
     の shell/script/metachar 系 case・`crates/cheaper-tools/tests/
     fixtures/process.json`・`run_dev_seam.rs` の deny-list case
     (`run_fake_stream_grant_deny_listed_executable_rejected` 等)は
     本 issue の churn 対象として AC に含める。
   - deps: なし(起点)。**前提: ADR-034KuqZelggTIb8Z8K9QB8 の
     ratification(proposed → accepted)を本 issue 着手前に完了
     すること。**
2. **opt-in `security.execDenyPatterns` profile(A9 降格)**
   - scope: policy schema に `security` 節追加(authoring は
     trusted 層のみ、deny の効果は invocation 合成 grant を含む
     resolved grant map 全体に及ぶ)、deny pattern の grant
     resolve/pin/spawn 再検証フック、既定=空、旧 builtin 一覧の
     migration 例を docs へ。
   - deps: 1。
3. **command.grants / process.tasks の単一 exec grant モデル統合**
   - scope: 統一 grant schema(optional `inputs`/`outputs` +
     `fixed` flag — M1/M2)・`process.tasks` → grant の loader
     写像(provenance scope 付き — project 宣言 task は
     project-scoped grant namespace へ写像し route grant 参照を
     解決しない、M3)・tool surface を `command` に統一 +
     `tool:"process"` alias 維持(M5)・writeScope 下の per-call
     coverage check 経路(M4)・pin/spawn/manifest/pool 機構の
     単一化。
   - deps: 1。
4. **route/hook の審査依存整理 + grant-set 投影 + record split**
   - scope: viability の unsafe 分岐が内容審査を呼ぶ経路の整理、
     `project_grant` → route 宣言 `grants:[name,…]` set 投影
     (grant 参照は trusted-scoped のみ)、viability の条件付き弱化
     (byte-matched mode では argv precheck を維持し、set 内の
     非 exec grant は存在・pin 解決可能性のみ検査)、
     `kind:"route"` record の `{command, executed}` field 分離
     (record attribution は `route.grant` anchor)。
   - deps: 1, 3。
5. **substitute-exec route mode**
   - scope: route `substitute:{grant,argv}` 宣言、viability の検査
     対象を substitute へ retarget、`expected_input` の
     `{substitute.grant, substitute.argv}` 再 anchor(D11 mirror
     の機構維持)、exit mirror の remap(実行した command の
     exit — spec R15 再記述済み)。Option E(exec-less)は
     本 issue では実装しない(open 事項)。
   - deps: 4。
6. **trusted route flag 発行主体の再設計(D4)**
   - scope: `trusted:true` を project 自身が宣言できる現状の再設計、
     trust 判定主体を operator/user 層へ移す方式の決定と実装。
     route の trust model は grant 参照の scope(誰の宣言を route
     が使えるか)と直に交差するため、grant-set/substitute の形が
     見えてから決める。
   - deps: 1, 4, 5。
7. **文書正直性 sweep + breaking-change migration guide**
   - scope: 「boundary/sandbox/protection」表現と実強制範囲の整合
     (guides・refine 対象 ADR/spec — spawn API 層の shell-free
     記述と ENOEXEC→`/bin/sh` fallback の区別を含む)、破壊的
     変更一覧(deny list 削除・内容審査除去・grant 形状)と
     移行手順の告知。
   - deps: 1-6。

## What stays invariant(変更しない保証)

- **observability** — usage record・route log・ComposedReader/
  ArtifactReader 境界(D13/E11)、condense worker の bounded read。
- **リソース上限** — timeout/killGrace/heartbeat/maxOutputBytes/
  accepted exit codes(C3)、maxToolCalls・abort grace・concurrency(C8)。
- **プロセス lifecycle** — env_clear + minimal_env(C1)、stdin null・
  no TTY・process group・windowsHide(C2)、pin + spawn 前再検証
  (B1/B2)、fail-closed admission(C4)。spawn API は shell を
  介さない — ただし POSIX で ENOEXEC 時に libc が `/bin/sh`
  fallback するのは OS/libc 既定動作であり cheaper の保証対象外
  (B3 補足)。
- **宣言制約の強制** — argv pattern 言語(A6)、cwd/env 宣言境界(A7)、
  writeScope/writeClaims(E3)、manifest 照合(E4)、capability 交差(E6)、
  deny_paths(E2)、canonical tool の workspace containment(E1)。
  writeScope 下で exec 可能なのは `fixed` かつ `outputs ⊆
  writeScope` の grant のみ(M4 — C5 invariant の per-call 形)。
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
- route run: hook payload → route resolve(viability = byte-matched
  mode では `route.grant` × 原 argv の `check_command_execution`
  precheck、substitute mode では substitute ペアへの retarget、
  set 内非 exec grant は存在 + pin 解決可能性の検査) →
  route-declared grant-set 投影(trusted-scoped grant のみ) →
  routed workflow 実行(exec-input byte 一致 D11 — `expected_input`
  は実行 grant `{route.grant|substitute.grant, argv}` に anchor) →
  exit mirror(実行した command の exit、D12) → `kind:"route"`
  record `{command, executed}`(route.grant anchor)。
- loader: `process.tasks` 節 → `fixed: true` + provenance scope
  (trusted/project)付き grant 形状へ読み替え → 統一 grant 集合
  として resolved policy へ merge。`process` 節は deprecate
  せず project の exec 宣言経路として存続(M3)。

## Non-Functional Constraints

- **後方互換**: deny list 削除・内容審査除去は破壊的変更。
  migration guide(issue 7)で告知。`process` 節・`tool:"process"`
  は loader 写像 + alias で存続するため、既存 policy.yaml・
  `{"kind":"tool","tool":"process"}` workflow は即死しない(M3/M5)。
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
- **統合の migration コスト** — `process` 節・`tool:"process"` は
  存続するため authoring/workflow 面の強制書換えは発生しない。
  コストの主体は resolved model・pin/spawn/manifest 機構・
  golden fixture の移行側(issue 3)に寄る。

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
      呼ばない構造(issue 4 の AC)。byte-matched mode の argv
      precheck は維持され、argv-mismatch 入力が route 不発 →
      原 command 直接実行の fallback を保つこと(条件付き弱化の
      AC)
- [ ] `{"kind":"tool","tool":"process","input":{"task":…}}` を持つ
      既存 workflow(`benchmarks/scenarios/scenarios.json`・
      fixtures)が統合後も `UnknownTool` で落ちずに動く後方互換
      テスト(issue 3 の AC — M5)
- [ ] writeScope 下の worker が `outputs ⊆ writeScope` の fixed
      grant を exec でき、outputs が writeScope を超える task・
      argv 供給 grant は拒否される統合テスト(issue 3 の AC — M4)
- [ ] golden fixture `crates/cheaper-core/tests/golden/fixtures/
      policy/process.json` の shell/script/metachar 系 case・
      `crates/cheaper-tools/tests/fixtures/process.json`・
      `run_dev_seam.rs` の deny-list case が issue 1/3 の churn
      対象として更新または削除されていること(AC 範囲に含める)
- [ ] 破壊的変更一覧が migration guide に列挙されていること
      (issue 7 の AC)

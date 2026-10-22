---
id: "DISC-034KuleA9g82Ir4xWfmTYq"
type: "discovery"
status: "active"
title: "権限・Runtime Policy・Tool 実行制限の責務ベース再設計 (issue #151)"
created: "2026-10-22"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/151"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
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
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 権限・Runtime Policy・Tool 実行制限の責務ベース再設計 (issue #151)

## Exploration Goal

issue #151 が問うのは「cheaper が現在抱える実行制限の総体を、責務ベースで再分類・
再設計できるか」である。背景となる責務分担は issue 本文で確立済み:

- **cheaper の責務** — 宣言された制約の強制(allow list・grant・writeScope・
  fs boundary)、実行契約の正しさ(shell 非介在 argv・環境・cwd・exit 規約)、
  プロセス lifecycle、自前 tool のデータアクセス制御、observability、
  リソース上限、cleanup。
- **workflow 作者の責務** — workflow の意味的正しさ・内容の信頼性
  (作者はユーザーまたは上流 agent)。
- **外部の責務** — host agent・hook・OS permission・真の sandbox。

本調査の完了条件(issue 成果物):

1. 全既存制限の inventory — source file:line・実行経路・保証する性質・
   bypass surface・keep/relax/configurable/remove の予備判定。
2. 信頼/権限モデルの選択肢 + 評価基準(トレードオフを残す)。
3. 既存 ADR/spec の supersede/refine 一覧。
4. 将来実装の issue 分解・依存 map。

「実行時 contract の強制」と「内容審査」を分離する視点が骨格。
deny list や provenance 審査は「宣言された制約の強制」ではなく
「cheaper が内容を審査する」挙動であり、責務分担と整合するかが争点。

## Key Issues and Assumptions

- **[確認済] 全制限が同一レイヤに混在している** — 実行契約(強制すべき
  性質)・権限(operator が選ぶ allow/deny)・optional hardening・内容審査が
  `policy/`・`tools/`・`route/` に区別なく実装されている。分類なしに
  「緩める/残す」を議論できない。
- **[確認済] deny list は宣言強制ではなく内容審査** —
  `command_denied_executable_re`(python/node/docker/ssh/tar/find…)は
  「grant が明示的に宣言した executable」を便宜上の危険度で reject する。
  これは workflow 作者責務への踏み込み。
- **[確認済] canonical tool の fs boundary と子プロセスの副作用は別物** —
  `resolve_workspace_path` + deny_paths は cheaper 所有 tool にのみ効く。
  grant で spawn した子プロセスのファイル副作用は OS sandbox が無い限り
  強制不能(既に `designs/policy-gated-command-tool.md` が bypass を認める)。
  文書上の「boundary」記述と実強制範囲の不整合は要修正。
- **[仮説] command grants と process.tasks は同一概念の二重実装** —
  双方とも「宣言済み exec を許可する + 固定 argv + policy 管理」。
  統合できれば policy surface・テスト・文書が半減する(後述 Option D)。
- **[確認済] routing ≠ authorization は既に正しく分離済み** —
  `route/resolve.rs` の viability は routing 決定で、exec 可否は
  `check_command_execution` に委譲されている。維持すべき構造。
- **[前提] shell 非介在 spawn は契約として維持** — argv 直渡し・env_clear・
  stdin null・process group は「実行契約の正しさ」(契約 A)であり、
  内容審査(契約 B)ではない。分離の基準点。

## Restriction Inventory(全制限棚卸し)

分類は issue の 4 区分: **(1) required** = 実行契約/correctness、
**(2) operator-choosable** = 権限として宣言可能、
**(3) optional hardening** = 任意の追加防御、
**(4) unenforceable/excessive** = 強制不能または内容審査。
予備判定は `keep` / `relax` / `configurable` / `remove`。

### A. Policy スキーマ・レイヤ(`crates/cheaper-core/src/policy/`)

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| A1 | 層順序 builtin→user→invocation→project + fail-loud load | `schema.rs` MergeMode, `load.rs` merge 順 | policy load 全体 | 決定的な優先順位・黙らない設定不備 | なし(純粋 merge) | (1) required | keep |
| A2 | invocation 層は `command.grants` key のみ許容 | `load.rs:119-188` | `--grant` CLI → load | 起動時の一時付与が恒久 policy を汚さない | なし | (1) required | keep(機構)。再設計で invocation key が増えた場合は拡張余地 |
| A3 | ProjectPartial が `command`/`route` 節を排除 | `schema.rs:340-355` | project `policy.yaml` load | 信頼境界 — project は grant/route authoring 不可 | project 層で trusted 宣言を書けば route のみ可(§D4) | (2) operator-choosable | keep(機構)。境界の形そのものが再設計対象 |
| A4 | RestrictConcurrency — project は数値/duration の narrow のみ | `merge.rs:20-36` | project merge | project が並列度・timeout・上限を緩和できない | なし | (1) required | keep |
| A5 | CLI `--restrict` 系は narrow のみ | `restrict.rs` | CLI 適用 | caller が policy を緩和できない | なし | (1) required | keep |
| A6 | argv pattern 言語(literal/glob/regex の 3 形式) | `command.rs:225-302` | grant resolve → args 照合 | 宣言した argv 形のみ許可 | glob の表現力に依存 | (2) operator-choosable | keep(契約 A の宣言制約そのもの) |
| A7 | grant cwd は workspace 相対のみ・env key は allowlist 形 | `command.rs:393-419` | grant resolve → prepare | cwd/env の宣言境界 | なし | (1) required | keep |
| A8 | grant executable の shell/shim 判定 | `command.rs:372` + `process.rs:117-148` | grant resolve | 「declared executable が shell/shim でない」= 内容審査 | grant 作者が exe を改名/別名で回避可能 | (4) content audit | relax(spawn が shell-free な限り shell 起動自体は合法契約) |
| A9 | `command_denied_executable_re` deny list | `command.rs:160-207` | grant resolve + pin + spawn 再検証 | 「既知の危険 executable を reject」= 内容審査 | 改名・同一機能の別ツール・子プロセス呼出しで無限に bypass 可 | (4) unenforceable content audit | **remove** または optional profile へ降格(本再設計の中核) |
| A10 | `argv_shell_re` メタ文字 reject(direct argv) | `process.rs:134-137`, task argv `process.rs:278`, grant args `command/mod.rs:467-475` | task/grant argv 検証 | 「argv に shell メタ文字が無い」= 内容審査 | なし(但し拒否自体が false positive) | (4) content audit | relax — shell-free spawn では `;` `|` は単なるデータ。tokenizer 側の解析用途(§D1)には残る |

### B. executable 解決・pin(`crates/cheaper-tools/src/tools/`)

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| B1 | PATH 検索 → canonical realpath pin | `executable.rs:138-190`, `command/mod.rs:183-213` | runtime 構築時 pin | 「実行するのは宣言名が解決した binary」= 決定性 | PATH 変更は pin 以降無効化済み | (1) required | keep |
| B2 | spawn 直前の pinned path 再検証 | `command/mod.rs:254-280` | 各 spawn | TOCTOU 対策(pin 後の差替検出) | pin→spawn 間の race は残存(文書化済) | (1) required | keep |
| B3 | `native()` magic-byte 検査 — MZ/ELF/Mach-O 以外を reject | `executable.rs:74-117` | process task resolve | 「shebang script/テキストを executable にしない」 | 実行内容には無関係な形式審査 | (4) content audit | relax/configurable — `#!/usr/bin/env python` script は正当な exec 形 |
| B4 | workspace-resident executable を grant で reject | `command/mod.rs:214-244`(2 段検査) | pin 時 | 「project の binary を trusted grant で走らせない」= provenance 審査 | workspace 外へ copy・インストール済み shim 経由で同等 | (4) content audit | relax — 宣言で許可する設計へ(spec 参照) |
| B5 | bare `node` の明示的 reject | `executable.rs:163-173` | process task resolve | 旧 Node host executable 名の衝突回避 | 該当名のみ | (4) excessive | remove(deny list の退化形) |
| B6 | `.cmd/.bat/.ps1/.sh` script 拡張子 reject | `process.rs:127-130` | process task resolve | 「script shim を task executable にしない」 | 中身は argv 解釈されるだけで害はない | (4) content audit | relax |

### C. 共有 proc lifecycle(`proc/`)+ registry/admission

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| C1 | `env_clear` + 宣言 env のみ(minimal_env) | `proc/mod.rs:175-217`, `spawn.rs:154` | 全 spawn | ambient credential/env 漏洩防止 + env 決定性 | route の passthroughEnv/extra_env は宣言経路 | (1) required | keep |
| C2 | `stdin(Stdio::null)`・no TTY・process group・windowsHide | `spawn.rs:151-160` 周辺 | 全 spawn | 対話不可能な managed 契約 | なし | (1) required | keep |
| C3 | timeout/killGrace/heartbeat/maxOutputBytes/accepted exit codes | `proc/monitor.rs:225-265`, ProcLimits | 全 spawn | bounded lifecycle・bounded output | 子プロセスが detach/daemon 化すれば tree kill 外に逃げ得る(既知) | (1) required | keep |
| C4 | command/process の ∅grants/∅tasks は fail-closed | `registry.rs:1102-1132` | runtime 構築 | 「allowlist 無しに exec tool を露出しない」= admission 正直性 | なし | (1) required | keep |
| C5 | writeScope × command 同時露出を拒否 | `registry.rs:1133-1142` | runtime 構築 | 「resource 層で観測不能な副作用を宣言させない」= 正直性 | なし(仕様上の整合) | (1) required | keep(再設計で command 意味が変わるなら見直し) |
| C6 | invocation parity check — caller policy と宣言形の一致 | `invocation.rs:219-238` | runtime 構築 | caller が policy 形を偽装しない | なし | (1) required | keep |
| C7 | Exclusive pool + conservative root manifest(command) | `command/mod.rs:126-145`, `command.rs:488-498` | 各 command 実行 | scheduler 直列化 + 監査用 claim | 実副作用の confinement ではない | (1) required | keep(但し「副作用を防ぐ」とは文書化しない) |
| C8 | maxToolCalls・abort grace・concurrency 上限 | policy execution 節 | tool 呼出し | 資源上限 | なし | (1) required | keep |

### D. route/hook(`crates/cheaper-cli/src/route/`,`hook/`)

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| D1 | `tokenize_shell` — unsafe segment/token 検出 | `routing_policy.rs:1041-1258` | hook eval・route run prepare | 「hook 文字列を正確に token 化できる」= 解析 correctness | なし(拒否ではなく viability 分類) | (1) required | keep(解析用)。policy 判定とは分離維持 |
| D2 | viability preflight = `check_command_execution` 委譲 | `resolve.rs:304-335` | route resolve/run | routing 決定と exec 可否の分離 | なし | (1) required | keep |
| D3 | `INTERACTIVE_COMMANDS` → StdinUnsupported | `resolve.rs:109-169` | viability | stdin 使用コマンドを route 不可として検出 | 対話性の heuristic(網羅不能) | (3) optional hardening | configurable — 検出不能な interactive もあり得るが早期診断として有用 |
| D4 | project route は `trusted:true` のみ下位層を shadow、非 trusted は suppressed provenance | `routes.rs:134-146` | routes load | 「project が user/builtin route を黙って乗っ取れない」 | `trusted:true` を project が書けば shadow 可 — 現状 project 自身が trust を宣言できる構造 | (2) operator-choosable | keep + 要検討: trust 宣言の発行主体(谁が trusted を認めるか)は再設計論点 |
| D5 | 同一層内 dup `match.command` → invalid 隔離 | `routes.rs:274-303` | routes load | 曖昧 match の fail-loud | なし | (1) required | keep |
| D6 | `target.path` は層起点相対のみ | `routes.rs:466-480` | routes load | 層外 escape の構造拒否 | symlink 経由は routes 側で未検査 | (1) required | keep |
| D7 | `passthroughEnv` env-name allowlist | `routes.rs:429-450`, `run.rs:107-119` | route run | hook env から宣言名のみ通過 | なし | (1) required | keep |
| D8 | `project_grant` 単一 grant 投影 | `command.rs:74-80`, `run.rs:121` | route run prepare | confused-deputy 防止 — route が他人の grant を使えない | なし | (1) required | keep(現モデル)。grant-set 投影は deferred sketch §F |
| D9 | argv-mismatch / env-not-allowed / cwd-mismatch(payload 検証) | `run.rs:60-130` | route run | payload が宣言 route に一致 | なし | (1) required | keep |
| D10 | routed-exec convention — `exec` job + whole `execResult` output 必須 | `workflow_run.rs:897-940` | routed workflow load | 「route 先 workflow が exec 契約を満たす」 | workflow 内での任意 tool 使用は自由 | (1) required(現 routing mode) | keep。非 exec route は deferred sketch §F |
| D11 | `exec-input-mismatch` — exec job input が prepared command と byte 一致必須 | `workflow_run.rs:465-482` | tool job 実行 | route が宣言外の argv を注入できない | なし | (1) required | keep |
| D12 | exit-code mirror — `execResult.details.exitCode` を route exit へ写像 | `run.rs:436-611` | route finish | 呼出し側 exit 規約の維持 | なし | (1) required | keep(非 exec route では mapping が課題 — §F) |
| D13 | ComposedReader — route log への bounded ArtifactReader | `run.rs:300-317` | condense worker 等 | route log を worker が宣言上限内で read | inner reader への fallback | (1) required | keep |
| D14 | approvalBypass は trusted 層のみ | `schema.rs` route 節 + A3 | hook policy | project が承認迂回を宣言できない | なし | (2) operator-choosable | keep |
| D15 | payload.exec_cwd の workspace containment 迂回(`prepare_exec_cwd`) | `proc/mod.rs:264-290` | route run | route 起動元 cwd をそのまま再現 | **意図的に workspace 外 cwd を許す** — denylist は適用されるが containment は迂回 | (1) required(現契約) | keep + 文書で明示 — workspace 外実行を許容する唯一の経路として audit 対象 |

### E. filesystem・resource・worker(`paths.rs`,`deny_paths.rs`,`access.rs`,worker 定義)

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| E1 | `resolve_workspace_path` — canonicalize・containment・symlink policy | `paths.rs:126-203` | 全 canonical file tool | workspace 外 path を tool が触らない | **canonical tool のみ** — spawn した子プロセスの fs 副作用には無効 | (1) required | keep + 文書で範囲明確化 |
| E2 | `DEFAULT_DENY_PATHS` 認証情報 pattern + policy `denyPaths` | `deny_paths.rs` | 全 canonical file tool + proc cwd 検査 | credential/key/.ssh/.aws 等を tool が読まない | canonical tool のみ。子プロセスは自前で読める | (1) required | keep |
| E3 | writeScope → writeClaims 宣言 + manifest ⊆ writeScope | `access.rs:319-382` | worker scope 構築 + 各 operation | worker が宣言外に書けない | canonical write tool のみ | (1) required | keep |
| E4 | `operation()` per-call manifest 照合(claims contains) | `access.rs:137-152`, `406-479` | 各 tool 呼出し | 宣言 manifest の範囲内の read/write のみ | 同上 | (1) required | keep |
| E5 | readStrictness(Record 等) | `access.rs` scope options | tool 呼出し | 読取りの記録義務 | なし | (1) required | keep |
| E6 | tools.allow ∩ worker.tools capability 交差 | `capability.rs`, registry | worker 構築 | worker が policy 未許可 tool を持てない | なし | (1) required | keep |
| E7 | unknown tool fail-closed・語彙閉集合 | `ToolName::from_canonical`, `workflow_run.rs:450-464` | tool job 実行 | 未定義 tool は reject | なし | (1) required | keep |
| E8 | worker output schema 必須 | worker validate | worker 定義 load | 構造化結果の契約 | なし | (1) required | keep |
| E9 | `optionalTools` で縮退可能な opt-in 露出 | worker `cheaper.optionalTools`, tool-operator.md | worker 構築 | policy 不成立時に未登録へ縮退 | なし | (2) operator-choosable | keep |
| E10 | writeScope(worker 定義側の宣言窓) | worker `cheaper.writeScope` | worker 定義 load | worker の副作用を事前宣言 | 同上 | (2) operator-choosable | keep |
| E11 | ArtifactReader 境界(route log・artifact への read のみ) | `run.rs:300-317`, artifact tool | condense/bulk-reader | worker が raw fs ではなく curate 済み store を読む | なし | (1) required | keep |

### F. process.tasks の固定 argv + 二重構造

| # | 制限 | 出所 | 実行経路 | 保証する性質 | bypass surface | 分類 | 予備判定 |
|---|------|------|----------|--------------|----------------|------|----------|
| F1 | process.tasks は固定 argv task 集合 | `process.rs`(tasks 節), `tools/process/prepare.rs` | process tool 実行 | 「宣言済み task のみ実行」= allowlist | task 名指定以外の argv 不可 | (2) operator-choosable | keep — grant と同種の宣言制約 |
| F2 | command.grants と process.tasks の二重実装 | `command.rs` vs `process.rs`, `tools/command/` vs `tools/process/` | 全 exec | なし(構造的重複) | メンテ負担・不整合余地 | —(構造) | **merge 候補** — 単一 exec grant モデルへの統合を spec で検討 |
| F3 | process task executable の native/shim 検査 | `process.rs:117-148`, `executable.rs` | task resolve | B3-B6 と同種の内容審査 | 同上 | (4) content audit | relax — B3-B6 と同時に見直す |

### 分類カウント

- **(1) required(実行契約/correctness)**: 31 項目 — A1,A2,A4,A5,A7,B1,B2,C1-C8,D1,D2,D5-D13,E1-E8,E11
- **(2) operator-choosable**: 6 項目 — A3,A6,D4,D14,E9,E10,F1
- **(3) optional hardening**: 1 項目 — D3
- **(4) unenforceable/excessive(内容審査)**: 8 項目 — A8,A9,A10,B3,B4,B5,B6,F3
- 構造的重複(分類外): F2

### 内容審査クラスタ(再設計の中核)

A8・A9・A10・B3・B4・B5・B6・F3 はすべて「宣言された exec の**内容**を
cheaper が審査して reject」する制限。いずれも:

- 宣言強制ではなく author の意図を cheaper が覆す
- bypass が構造的に無限(改名・別ツール・子プロセス経由)
- false positive が正当な workflow を拒否する(例: `git filter-branch` に
  必要な `find`、deploy script の `ssh`、build 内の `.sh`)

これらを「宣言された実行契約の強制」層から「optional hardening profile」層へ
移すか削除するかが、本再設計の第一決定。

## Alternatives and Comparison(信頼/権限モデルの選択肢)

評価基準: 正直性(claim≦実強制)・false positive 率・責務明確性・
operator 制御性・confused-deputy 耐性・監査性・移行コスト。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **A. 現状維持 hardening** — deny list・provenance・メタ文字審査を mandatory のまま保持・拡張 | 保守的。変更不要。「危険ツールを既定で弾く」直感に合致 | 内容審査は責務外。false positive が正当 workflow を拒否。bypass 無限で claim と実強制が乖離。deny list を「保護」と呼ぶのは誤称 | **no** — issue 前提と矛盾 |
| **B. 宣言契約モデル** — cheaper は「宣言された制約の強制」のみ担う。内容審査(8 項目)を削除または opt-in profile へ降格。grant=operator が宣言した実行許可 | 責務分担と一致。claim=実強制で正直。false positive 解消。worker 作者に意図通りの exec を許す | grant 作者の能力に依存。「既定で危険物を弾く」網が無くなる(但し元々完全ではない)。移行時の doc 修正量 | **yes** — 推奨方向(ADR 参照) |
| **C. tiered trust zone / profile** — 現行制限を全て configurable profile(`execProfile: strict/declared` 等)として層別に選択可能化 | 後方互換。operator が hardening 強度を選べる | knob ごとに semantics+test+doc が必要。組合せ爆発。複雑性が「単純に削る」より高い | **maybe** — B の「optional profile への降格」がこれに相当。B+C の複合が実現案 |
| **D. capability-grant 統合** — `command.grants` と `process.tasks` を単一「exec grant」モデル+単一 tool へ統合。allowlist・argv contract・lifecycle を一元化 | F2 の二重実装解消。policy surface 半減。route/process/command の契約が一本化 | 大規模 refactor。migration が必要。B とは独立の構造変更 | **maybe** — B と直交。spec で scope 決定(同時 or 後続 issue) |
| **E. OS-sandbox backed mode**(参考) — seatbelt/bubblewrap/seccomp 等で真の isolation を提供する任意モード | 真の containment が可能 | OS 依存・導入コスト大・「sandbox を提供する」と言える条件が厳しい。issue 範囲外 | **no** — 本再設計の非目標。「提供しない」を正直に文書化する側 |

**推奨の複合**: B(宣言契約モデル)を基底とし、A9 の deny list を
opt-in `security.execDenyPatterns` 的な profile knob として残すかは
spec で決定(C の要素)。D(grant/task 統合)は別 issue へ切り出すか
同 spec の scope に含めるかを ADR で記録する。

## Tentative Conclusions and Hypotheses

- **[confirmed] 責務境界は「宣言強制 vs 内容審査」で引ける** — 8 項目の
  内容審査制限(A8,A9,A10,B3,B4,B5,B6,F3)はすべて「宣言を覆す審査」
  であり、残り 38 項目は「宣言の強制」または「契約の正しさ」に分類できる。
  境界は構造的に明瞭。
- **[confirmed] deny list の「保護」表現は誤称** — `command.rs` の
  deny list は deterministic deny であり containment ではない。
  `designs/policy-gated-command-tool.md` 自身が bypass surface を認めている。
  「default で危険を弾く」期待は「既定で拒否する既知の悪い名前一覧」に
  訂正すべき。
- **[confirmed] E1/E2 の workspace boundary は canonical tool 限定** —
  現行実装もこれを正しく制限している(子プロセスを装う主張は無い)が、
  ADR・guide の文面上で「cheaper が workspace に閉じ込める」表現が
  読める箇所があり要訂正(成果物#5 の対象)。
- **[hypothesis] grant/task 統合は価値あるが独立決定** — F2 の統合は
  責務再設計と直交する構造改善。同時実施は spec の scope で決める。
- **[hypothesis] D4 の `trusted` 宣言主体が課題** — project が自ら
  `trusted:true` を書けば shadow できる現状は、trust の発行が project
  自身にある。真の trust 判定主体(operator/user)への移行は
  route 信頼モデルの再設計論点として残す。
- **[confirmed] routing ≠ authorization は既に正しい** — D2 で確認済み。
  再設計でもこの分離は維持。viability が内容審査(A9 等)を呼んでいる
  点のみ、審査削除と共に自然に整理される。

## Open Questions

- deny list は「削除」か「opt-in profile(`security.execDenyPatterns`)」か?
  opt-in に残すなら既定値は空か現行一覧か?
- workspace-resident executable(B4)を grant で許可する場合、
  「project が書いた binary を trusted grant が走る」リスクを
  「宣言した以上 operator の責任」と割り切れるか?
- メタ文字 reject(A10)は exec 側では全廃でよいか(tokenizer D1 の
  解析用途は残る前提で)?
- command.grants と process.tasks の統合(F2/D)は本 spec の scope か、
  後続 issue か?
- deferred non-exec routing sketch(ADR-034KtTzOJGqmzmBW3SwGp5 —
  substitute command・grant-set 投影・非 exec workflow・exit mirror 変種)
  を本再設計に組み込むか?
- `trusted` route flag の発行主体を operator 層へ移すか?
- 文書上の「boundary」「sandbox」「protection」表現の棚卸し範囲
  (guides/ADR/spec のどこまでを成果物#5 で修正するか)?

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5、proposed) — 再設計の product intent・
  責務分担・受入基準・scope(統合/後続の境界)
- [x] adr-doc needed: `docs/adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8、proposed) — 「宣言契約の強制を責務とし
  内容審査を分離」の architecture decision(Option A/B/C/D の比較記録)

### 将来実装の issue 分解(成果物#4)

1. **exec-contract 整理** — A8/A9/A10/B3-B6/F3 の内容審査を削除または
   optional profile 化。`check_command_execution`・pin・resolve の
   審査分岐を整理。**依存**: 本 ADR。
2. **grant/task 統合 or 保持決定** — F2。spec scope 決定後に着手。
3. **route/hook の審査依存整理** — viability の unsafe 分類が内容審査を
   呼ぶ箇所の整理(D1 の解析用途は残す)。deferred sketch の grant-set 投影
   検討。**依存**: 1。
4. **trusted route 発行主体** — D4 の再設計。**依存**: 1・spec。
5. **文書正直性 sweep** — E1/E2/C7 等の「boundary/sandbox/protection」
   表現と実強制範囲の整合(guides・ADR・spec)。**依存**: 1。
6. **移行 guide + migration** — 既存 user policy.yaml への影響・
   deny list 削除による破壊的変更の告知。**依存**: 全て。

関連既存 issue: #142-#150(route/hook 系)、#160(hook worker parity)。

## Sources and Relations

- 起点: issue #151(責務分担の定義・4 分類・棚卸し範囲 A-D・完了条件)
- 前提 ADR(検証対象):
  - `../adr/command-grant-trust-boundary-and-managed-process-reuse.md`
    (ADR-034EMgaatFhsXjR97kH14c, accepted) — trusted-layer grant モデル・
    deny list・workspace-resident reject を定義。本再設計で **refine** 対象
  - `../adr/worker-definition-runtime-policy-runtime-invariants-capability.md`
    (ADR-034r05iHlWosDpJ3DpG3XW, accepted) — 3 層モデル(Worker ∩
    Policy ∩ Invariants)。invariants の中身再配置で **refine** 対象
  - `../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md`
    (ADR-034r05iHlWosDpO699OwYF, accepted) — canonical tool の fs boundary。
    「子プロセスは対象外」の明確化で **refine** 対象
  - `../adr/defer-non-exec-routing-to-permission-model-redesign.md`
    (ADR-034KtTzOJGqmzmBW3SwGp5, accepted) — 非 exec routing 判断を本
    issue へ委譲。deferred sketch(substitute・grant-set・非 exec)は
    本再設計の必須 input
- 関連 ADR(影響確認済み):
  - `../adr/invocation-scoped-command-grants.md` 系(spec SPEC-034FlOz4hiN9m0nQW7lBN2)— grant 形状変更で refine
  - `../adr/managed-process-bounded-lifecycle-policy.md` (ADR-034AtSJVcROd9oyMi4ksua) — 影響なし(keep)
  - `../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md` (ADR-034ESBcMTWxNCMDI9obdb8) — 直交、keep
  - `../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md` (ADR-034J8eZHNPqARi25HLFOL6, proposed) — C9 投影が grant-set 化で refine
  - `../adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md` (ADR-034KrYGRHZuvOIx1d0S0nE) — approvalBypass の trusted gating は整合、軽微な refine
- 関連 spec:
  - `../specs/policy-gated-command-tool.md` (SPEC-034EMbEGBOWM4CAM6YSWm6, approved) — deny list 要件を含むため **supersede/refine**
  - `../specs/invocation-scoped-command-grants.md` (SPEC-034FlOz4hiN9m0nQW7lBN2, approved) — grant 検証要件の refine
  - `../specs/pretooluse-route-resolver-and-workflow-delegation.md` (SPEC-034J7SxWvGL3oSbeBpo8a3, proposed) — grant semantics の refine
- design: `../designs/policy-gated-command-tool.md` (DESIGN-034EMWNJbQpMB0N3ZTXM5A, approved) — bypass surface を正しく記録。deny list 部分が再設計で更新
- 調査した実装: `crates/cheaper-core/src/policy/{schema,load,merge,restrict,command,process,capability,filesystem}.rs`、`crates/cheaper-tools/src/tools/{command,process,proc}/`、`registry.rs`、`access.rs`、`invocation.rs`、`crates/cheaper-core/src/workspace/{paths,deny_paths}.rs`、`crates/cheaper-cli/src/route/{routes,resolve,run,log_store}.rs`、`crates/cheaper-cli/src/hook/routing_policy.rs`、`crates/cheaper-cli/src/workflow_run.rs`、`crates/cheaper-cli/resources/{policy.yaml,workers/}`

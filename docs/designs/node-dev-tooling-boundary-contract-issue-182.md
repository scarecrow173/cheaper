---
id: "DESIGN-BSEuqAM7QQUmRoTUGOMxbH"
type: "design"
status: "approved"
title: "Node dev-tooling boundary contract — keep-Node operationalization (issue #182)"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
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
    - "../specs/node-dev-tooling-scope-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  refines: []
  refined-by:
    - "docs/specs/remove-ambient-node-test-coupling-issue-188.md"
  derives-from:
    - "../specs/node-dev-tooling-scope-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
  derived-by:
    - "../plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    - "remove-ambient-node-from-rust-tests-issue-188.md"
    - "../test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  verifies: []
  verified-by:
    - "../test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "../../mise.toml"
  defers: []
  deferred-by: []
metadata: {}
---

# Node dev-tooling boundary contract — keep-Node operationalization (issue #182)

## Context

issue #182 の toolchain 決定として、
`docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
(ADR-034LRjX5iYZHrubmKgKAL1)は **Option A — Node.js を pin した
dev-tooling runtime として維持**を採択した。Option A は
decision-as-record(コード変更を伴わない決定)だが、「維持」を形骸化
させないためには、何を・どの境界で・どう観測して維持するかを契約として
固定する必要がある。

本 design は keep-Node 決定が operationalize すべき 5 点を canonical 参照
として規定する:

1. **非退行契約** — 動き続けなければならない Node entry point の完全集合と、
   各 entry が今日検出している失敗(保存契約)の固定。
2. **境界 policy** — product runtime の node-free 維持と、repo 内のどの
   領域が新規 node 結合を導入してよいかの明文化。
3. **環境結合 policy** — Rust test 3 site の process/tool contract と、
   issue #188 による Node-free fixture coverage。mise が repository tooling
   用 Node を供給する運用も記録する。
4. **Revisit trigger の観測点** — ADR が挙げる見直し条件を、機械的に
   確認可能な観測手順へ落とす。
5. **Follow-up issue 計画** — 本決定が先送りする implementation 候補と
   順序制約(issue #183 benchmark redesign との関係を含む)。

上流 chain: inventory(`docs/discovery/node-dev-tooling-inventory-issue-182.md`)
→ scope spec(`docs/specs/node-dev-tooling-scope-issue-182.md`)→ ADR。
本 doc の記述対象は dev-tooling のみで、product runtime への変更は
一切伴わない。

**Implementation update (2026-10-26):** Follow-up F1 was completed under issue
#188. The three Rust test sites no longer depend on ambient Node; the Node pin
remains required by repository tooling described in §1.1–§1.2 and by upstream
deployed doc-suite skills. This dated status update does not change the accepted
keep-Node decision or the separate T1/T3 removal gates.

## Scope

### In Scope

- 現行 Node entry point 集合(check 6 件 + manual 3 件)と Rust test 3 site
  の Node-free fixture coverage の宣言的固定。
- 新規 node 結合の導入可否を領域ごとに定める境界 policy。
- issue #188 で移行した Rust test 3 site の required contract と
  bare-name PATH proof の責務分担。
- revisit trigger ごとの観測点(コマンド・diff 対象)の手順化。
- 先送り implementation issue 候補の scope・受入 sketch・順序制約。

### Out of Scope

- 実装変更そのもの(Option A は現行実装をそのまま維持する決定であり、
  本 design 作成時点では file 変更を要求しなかった。F1 は issue #188 の
  独立 follow-up として実装済み)。
- upstream 所有 script(`.agents/skills/`・`.claude/skills/` の deploy 物)
  の移行 — `scarecrow173/apm-packages`・`obra/superpowers` 側の決定事項。
- `mise.toml` の `node` pin 除去 — required entrypoint が全て node-free
  になるまで gate 未達(issue 指定条件)。
- benchmark harness の設計変更 — issue #183 の管轄。

## 1. 非退行契約 — 維持すべき Node entry point 集合

「Node 維持」とは、次の集合が現在の検証意味論を保って動き続けることを
意味する。いずれかが失われる・弱められる場合は本契約の違反であり、
ADR の revisit ではなく **回帰**として扱う。

### 1.1 required entrypoint — `mise run check` の node 呼出し 6 件

`mise.toml` `[tasks.check]` の node 経由 step(mise.toml:69-70,:72-75 —
:71 は `cargo test`)。pipeline
順序・各 script の検出対象は変更してはならない(check pipeline 契約は
ADR-034r05iHrZUE6WeDAi63F3 の canonical task が所有する)。

| # | mise.toml | 呼出し | 保存契約(今日検出している失敗) |
| --- | --- | --- | --- |
| 1 | :69 | `node scripts/check-version.mjs` | `Cargo.toml [workspace.package].version` と `packages/apm/apm.yml` + `packages/apm-hook-*/apm.yml` の version 宣言 drift → CI fail |
| 2 | :70 | `node scripts/check-doc-ids.mjs` | tracked markdown(`docs/`・`AGENTS.md`・`README.md`・`packages/*/.apm/**/*.md`)への legacy `TYPE-NNNN` artifact ID の再混入 |
| 3 | :72 | `node --test scripts/test/` | repo tooling test 6 file(+`lib/apm-fixture.mjs`)の coverage: `apm install` 実実行 deploy matrix・package layout 分離・payload contract・bulk-reader skill drift・devin hooks install・launcher passthrough の回帰 |
| 4 | :73 | `node --test benchmarks/test/` | benchmark harness unit test 4 file の coverage: aggregate・measurements・result-schema・scenario-manifest の回帰 |
| 5 | :74 | `node scripts/check-lockfile.mjs` | committed `Cargo.lock` の manifest 不整合 + 先行 step による working-tree drift(consumer `cargo install --locked` 契約) |
| 6 | :75 | `node scripts/check-apm-payload.mjs` | slim APM payload 契約(検査範囲は `packages/*/.apm/hooks/`): 必須 asset 欠落・`cli/` 残存・extension 違反・PE/ELF payload・launcher byte 契約・copy 間 drift・`dependencies.apm` wiring 欠落・hooks.json↔launcher 不整合・bare `./cheaper` 参照 |

### 1.2 manual entrypoint — 手動 repo tooling 3 件

| # | 呼出し | 用途 | 保存契約 |
| --- | --- | --- | --- |
| 7 | `node scripts/set-version.mjs`(+`scripts/lib/release-artifacts.mjs`) | release version bump(`docs/guides/release.md` 手順) | Cargo.toml canonical + 全 apm.yml の一括更新 + 更新後の check-version 自己検証(bump 漏れ防止) |
| 8 | `node benchmarks/run.mjs`(+`benchmarks/lib/`・`fixtures/`・`scenarios/`) | workflow arm vs scripted direct arm の計測 harness(手動、`mise run check` 非配線) | workflow vs direct の orchestration overhead 計測。`scenarios.json` の `{node}` → `process.execPath` 置換で emit.mjs を spawn する scenario(scenarios.json:194・263)は「計測対象 workload としての node」を含む。結果 record の `environment.node` field を保持(`environmentSnapshot()` が record に emit する field であり、`result-schema.mjs` の `validateBenchmarkRecord` では未宣言・未検証) |
| 9 | `node crates/cheaper-tools/gen-glob-tables.cjs <out-path>` | `crates/cheaper-tools/src/glob/tables.rs` の glob POSIX class Unicode range table 再生成 | Node bundled ICU の `/^[\p{...}]$/u` regex を全 code point に試行する oracle。generated output に `process.version` を埋め込み node-version-sensitive な再現性を追跡(tables.rs header は `Node v26.10.0` を記録) |

### 1.3 Rust test 内の process contract — issue #188 で Node-free 化

issue #188 (2026-10-26) で以下の 3 site から ambient Node 依存を除去した。
テストは従来の process / differential / workflow contract を維持し、Node
不在でも実行される。詳細な実施記録は
`docs/designs/remove-ambient-node-from-rust-tests-issue-188.md` を参照する。

- `crates/cheaper-tools/tests/proc_process.rs::node_task_resolves_via_path`:
  test binary を再実行し、fixture binary directory のみを持つ child PATH
  で bare `diff_fixture_child` を resolve → grant pin → spawn する。
  process-global PATH は変更しない。
- `process.json` の既存 literal `node` case と `runtime.json` の 6 個の
  literal `node` declarations は frozen JSON を変更せず、test 専用
  translator が absolute `diff_fixture_child` + explicit `bare` mode へ
  変換する。期待出力 `bare node\n` を保つが、この differential translation
  自体は PATH lookup を検証しない。
- `cheaper-workflow/tests/process.rs` は `current_exe` を exact filter で
  再実行し、explicit marker 下で `out.txt` を生成する。workflow/resource
  assertions は維持され、Node 不在時も silent skip しない。
- acceptance evidence は明示的 Node-free PATH 下の三つの test 成功と、
  frozen fixture JSON が基準 revision と byte-identical であること。

### 1.4 維持すべき invariant の一覧

上記 entry が担保する invariant(どれかが弱まると回帰):

- **version 宣言一致**: `Cargo.toml` canonical version と全 `apm.yml` の
  version が常に一致していること(check-version が検出、set-version が
  一括更新で予防)。
- **Cargo.lock committed-state**: committed `Cargo.lock` が manifest と
  整合し、`mise run check` 実行後に working-tree drift を残さないこと
  (consumer の `cargo install --locked` が前提とする)。
- **APM launcher byte/encoding/mode/wiring 契約**: `.ps1` の BOM+CRLF、
  `.sh` の shebang+LF、git index mode `100755`、hooks.json↔launcher
  wiring、`dependencies.apm` の transitive 参照、payload 内 executable・
  `cli/`・PE/ELF・bare `./cheaper` 参照の不存在。
- **doc-ID check coverage**: tracked markdown 全体への旧連番 ID 走査が
  継続されること(対象集合が狭められない)。
- **unit-test coverage/failure 意味論**: `node --test` 両 suite が現行の
  assertion 集合を保ち、fail すべき条件で fail し続けること(test 数の
  削減・skip 化による無音の coverage 低下を認めない)。

## 2. 境界 policy — node 結合の導入可否

### 2.1 原則

- **product runtime(CLI・Runtime・Hook 実行経路)は node-free を維持**。
  これは Option によらず不変の制約であり、本 design の変更対象でもない。
- **Node は pinned mise dev-tool(`mise.toml` `[tools] node = "26"` +
  `mise.lock`)としてのみ存在する**。`mise install` / `mise run` /
  `mise exec` 経路で供給される runtime であり、product 配布物・consumer
  install 経路・hook runtime からは一切参照されない。
- 新規 `.mjs` tooling を追加する場合も同一境界に従う: **repo tooling =
  Node 可、product path = Node 不可**(AGENTS.md・CONTRIBUTING.md の既存
  記述を本契約として codify する)。

### 2.2 領域別の node 結合導入可否

| 領域 | 新規 node 結合 | 理由・enforcement |
| --- | --- | --- |
| `crates/*/src/**` | **不可** | product runtime。`git grep -w 'node' -- 'crates/*/src/*'` で node 参照を走査し、spawn 相当箇所が 0 件であることを review で維持(comment・文字列の false positive は目視で除外 — `Command::new("node"` 単一 signature では `.arg("node")`・`format!` 構築・`node.exe`・`sh -c "node"`/`cmd /c node` 形を拾えない) |
| `cheaper.sh` / `cheaper.ps1`・`packages/**` | **不可** | 配布 payload。`packages/*/.apm/hooks/` 配下は `check-apm-payload.mjs` が extension allowlist + magic byte で機械的に排除するが、それ以外の `packages/**`(skills payload `packages/apm/.apm/skills/**` を含む)は走査対象外のため違反は human review 依存(§2.3) |
| hook 実行経路(`packages/apm-hook-*/.apm/` 等) | **不可** | consumer 環境に node を要求してはならない。hook launcher は PATH 上 `cheaper` を解決する thin wrapper のまま |
| `.github/workflows/ci.yml` | **不可**(直接 node 呼出しを増やさない) | CI は canonical task(`mise run setup`/`mise run check`)経由のみ。node 呼出しの追加は mise task 側で行う |
| `crates/*/tests/**`・test fixture | **原則不可** — ambient PATH への暗黙結合を増やさない。tool contract の検証上やむを得ない場合のみ、mise 供給前提をコメントで明示し required/skip 契約を本 §3 と同型に定めること | 新規 ambient 結合は §5 F1(fixture binary repoint)の方向と矛盾するため追加しない |
| crate 非 src の codegen script(`*.cjs`/`*.mjs`) | **条件付き可** — generated artifact の oracle が node semantics に依存する場合のみ、manual entrypoint として(`gen-glob-tables.cjs` 先例)。生成物 header に generator version を記録する契約を継続 | codegen は check pipeline に配線しない |
| `scripts/*.mjs`・`scripts/lib/`・`scripts/test/` | **可** | repo tooling 領域。ただし platform-sensitive な処理(byte-level 検査・git mode・lockfile・permission 系)を新設する場合は Rust(xtask・test helper)を優先検討する — POSIX/Windows 差分を node に任せるか Rust で制御するかは script 単位で判断 |
| `benchmarks/` | **可** | 計測 harness は repo tooling。ただし harness 全体の構造変更は issue #183 に委ね、本 policy 下での散発的な拡張は最小限に留める |
| `.agents/skills/`・`.claude/skills/`・`apm_modules/` | **repo 管理外** | upstream deploy 物。本 repo での node 結合追加は不可(上書きされる)かつ upstream の責務 |
| `mise.toml` `[tools]` | node pin の version bump は運用として継続可(lifecycle 詳細は §4 の運用 note)。**`node` 行の除去は removal gate(required entrypoint 全 node-free + §1 維持 entry の移行完了)未達の間は不可** — §4 の revisit trigger は ADR 再検討の発火条件であり除去条件ではない。別 runtime の追加は toolchain 決定として ADR 級の review を要する | toolchain 宣言は issue の removal gate が縛る |

### 2.3 enforcement

- 機械的 enforcement は既存のまま: `check-apm-payload.mjs`(payload・
  launcher・wiring)が product 側の node 混入を排除し、`biome check` が
  `.mjs` 系の lint を担う。本 design は新たな検査を追加しない。
- 機械的 enforcement の既知 gap(受容済みの human review scope):
  - `check-apm-payload.mjs` の extension allowlist + magic byte 走査は
    `findHookPackages` が発見する `packages/*/.apm/hooks/` のみを対象
    とし、skills payload `packages/apm/.apm/skills/**` は走査されない。
    同 subtree は既に
    script(`cheaper-usage/scripts/install-devin-hooks.ps1`)を含み、
    `.js`/`.mjs` を混入させても CI は検出せず pass するため、当該
    subtree の node 結合禁止は review 依存とする。
  - `biome.jsonc` の includes は `scripts/**/*.mjs`・
    `benchmarks/**/*.mjs|json`・`packages/**/*.json` のみで、
    `*.cjs`(`gen-glob-tables.cjs`)と将来の crate 側 codegen script
    は lint coverage 外。
- 人間側の enforcement は review 時の本 §2.2 参照と、§1 の非退行表に
  対する差分確認で行う。

## 3. 環境結合 policy — `cargo test` の Node-free fixture 契約

### 3.1 site ごとの意図契約

| site | Node-free 実装 | 保存する contract |
| --- | --- | --- |
| `proc_process.rs` `node_task_resolves_via_path` | Exact test re-execution sets PATH only on the child process to the Rust fixture binary directory. | Real bare filename → PATH resolution → grant pin → spawn remains covered; assert exit 0 and exact `bare node\n`. No process-global PATH mutation. |
| `process.json` literal bare-Node case + six `runtime.json` declarations | Differential translator maps only the known literal Node command to the absolute fixture binary and explicit mode. | Keep frozen JSON byte-identical and preserve all golden outputs. This absolute-path translation does not claim to test PATH resolution. |
| `cheaper-workflow/tests/process.rs` producer/consumer test | Exact-filter re-execution of the test binary writes the declared output under a marker. | Compile/materialize/execute/output/resource assertions run without Node and without silent skip. |

### 3.2 `mise run check` による node 存在保証

- `mise install` / CI の `jdx/mise-action`(`install_args: --locked`)が
  mise.toml + mise.lock から toolchain を復元し、`mise run` が `[tools]`
  を PATH へ解決するため、`mise run check` 経路では mise 管理の
  node(lock 上は 26.12.1)が ambient PATH に必ず存在する。
- Node remains in mise for repository tooling. The three Rust test sites above
  no longer consume that ambient Node entry, so both `mise run check` and bare
  Cargo test runs execute their Rust fixture coverage rather than relying on
  Node availability or silently skipping a site.

### 3.3 新規 ambient 結合の扱い

`cargo test` から ambient `node` への新規結合は追加しない(§2.2)。issue
#188 で既存 3 site は fixture binary 化済み(F1 complete)。contract を変える
場合は本 policy と test spec を更新し、Node-free acceptance を維持する。

## 4. Revisit triggers と観測点

ADR の revisit 条件を、発火を機械的に確認可能な観測点へ落とす。

| # | trigger(ADR) | 観測点(確認手順) |
| --- | --- | --- |
| T1 | deploy 済み node-requiring skill 群(doc-driven-dev package + `obra/superpowers` 経由 brainstorming)が全て node-free 化されるか `apm.yml` dependencies から drop された | `apm.lock.yaml` の `deployed_files` diff で `scripts/*.js`・`*.mjs`・`server.cjs` エントリが 0 になるか(後述の前提 note 参照)。補助: `apm install` 後に `find .agents/skills .claude/skills \( -name "*.js" -o -name "*.mjs" -o -name "*.cjs" \)` が空を返すか。発火時は repo 側 Option C 相当(F4)が node 除去の最後のピースとなるため本 ADR を再検討。brainstorming は upstream が third-party のため、skill の drop/replace(F5)自体が別 unblock path |
| T2 | benchmark harness の大改修(scenario・計測面の変更)が必要になった | issue #183(benchmark redesign)の着手。改修機会に Rust 化(F2)を畳み込むか再評価する。`{node}` workload scenario と `environment.node` field の再設計が焦点 |
| T3 | glob-table codegen の再エンジニアリングが必要になった(design 追加 trigger — ADR の revisit 列挙は T1・T2・T4 に相当する 3 条件のみ) | `tables.rs` の再生成需要(POSIX class semantics の変更・Node 版間の ICU drift が問題化)が生じた時点で、`gen-glob-tables.cjs` の Node regex oracle 依存を再設計する(F3)。再生成需要がない間は manual entrypoint として維持 |
| T4 | `node` pin の security bump 頻度や 2-runtime 管理コストが顕在化した | `git log -p -- mise.lock` を `[[tools.node]]`/`version` 行で filter して node bump 履歴の頻度と伴随する toolchain 更新コストを観測(定性的判断。pin lifecycle は後述の運用 note) |
| — | 上記いずれかの発火を疑うとき | inventory §Sources の走査を再実行する(scripts inventory re-run): `git grep -w 'node' -- 'crates/*/src/*'`(word-boundary 走査 + false positive の目視除外 — spawn signature は `Command::new("node"` 単独では捕捉できない多形)・`find crates \( -name "*.js" -o -name "*.mjs" -o -name "*.cjs" \)`・deployed script 数・`mise exec -- node --version` で現況を固定してから判断する |

観測手順の前提と node pin の運用 lifecycle:

- `apm.lock.yaml` の `deployed_files` diff は apm dep bump
  (`resolved_commit`/`version` の変更)のたびに**双方向**で確認する —
  upstream bump は node-coupled file を除去するとは限らず、追加も
  し得る。
- T1 の補助 `find` は fresh `apm install` 済み checkout で実行する。
  `.worktreeinclude` 経由で worktree へコピーされた `.agents/`・
  `.claude/` 資産は lock 変更後 `mise run setup` で再生成されるまで
  古いまま残り得る。
- `find` 系コマンドは POSIX shell 前提(Windows では Git Bash / WSL
  で実行)。pattern は `*.js`・`*.mjs`・`*.cjs` の 3 拡張子を網羅する
  — upstream が script を `.mjs` へ移行しても node 依存は残るため。
- node pin の bump は手動 `mise upgrade` + `mise.lock` 再生成で行い、
  renovate/dependabot 相当の自動化は存在しない。`mise.toml:2` の
  `minimum_release_age = "7d"` により security release も publish
  から最大 7 日遅延して供給される点は受容済み caveat。`node = "26"`
  は major line pin であり、Node の LTS 移行(Active → Maintenance)
  に合わせて major bump する policy。bump 時は Node bundled ICU の
  変更で `gen-glob-tables.cjs` の oracle 出力が drift し得るため、
  `tables.rs` の再生成 diff を併せて確認する(§1.2 #9 の
  node-version-sensitive 契約)。

## 5. Follow-up issue 計画

本決定が先送りする implementation 候補。いずれも Option A 下では
**即時対応不要**で、各 ordering 制約が整った時点で起票する。

| ID | 候補 issue | scope(1 行) | acceptance sketch | ordering |
| --- | --- | --- | --- | --- |
| F1 (完了: 2026-10-26, issue #188) | `cargo test` の ambient `node` 結合を fixture binary に置換 | proc_process は controlled child PATH で bare fixture name を resolve/pin/spawn。`process.json` と six `runtime.json` declarations は test translator で fixture child に変換。workflow test は test binary child を実行 | Node-free WSL PATH で三つの対象 test が通過し、frozen process/runtime JSON は基準 revision と byte-identical。differential absolute-path translation を PATH proof と誤認せず、bare lookup proof は proc_process が所有 | 独立の hygiene issueとして完了。Node pin と他 follow-up は変更しない |
| F2 | benchmark harness の Rust 化 | `run.mjs`+`lib/*.mjs`+`test/`+fake-stream fixture を Rust harness へ移行し `{node}` workload scenario・`environment.node` field を再設計 | workflow vs direct の計測意味論が維持され、結果 record schema の変更が version/比較可能性として明示される。`node --test benchmarks/test/` が cargo test 系へ置換される | **issue #183 への結合が前提** — benchmark redesign が scenario/計測面を変える際に畳み込む。#183 単独で node 化 workload を残す判断なら F2 は後送り継続。required entrypoints 6→5 に減るだけで removal payoff なし(ADR Option B 却下と同根拠)のため #183 なしでは起票しない |
| F3 | `gen-glob-tables.cjs` codegen oracle の再設計 | Node bundled ICU `\p{...}` regex oracle への依存を除去し、`tables.rs` の別生成経路(Rust 側 oracle 実装 or 検証付き固定 snapshot)を設計 | `node` 不在で tables.rs を再生成可能、または「再生成不要の固定 artifact」として contract 化される。POSIX class semantics の drift 検出手段を持つ | trigger T3 発火時のみ。それまでは manual entrypoint 維持(単純移植不可 — oracle の再エンジニアリングが前提) |
| F4 | check-*.mjs・`node --test` 系・release script の Rust/native 移行(Option C repo 側) | `check-version`/`check-doc-ids`/`check-lockfile`/`check-apm-payload`/`set-version` + `scripts/lib/release-artifacts.mjs` → xtask/mise 行、`scripts/test`・`benchmarks/test` → cargo integration test。ambient fixture 化は issue #188 の F1 で完了し、この follow-up の対象外 | `mise run check` pipeline の node step が 0 件で全 pass に加え、release 経路(`set-version` による bump + `check-version` 自己検証)が node なしで動くこと — pipeline 0 件は下限でしかなく、removal gate は §1.1+§1.2 の維持 entry 全体(benchmarks 側は F2・codegen は F3 を含む)の移行完了を要求する。§1.4 の各 invariant(version drift・lockfile・launcher byte/wiring・doc-id・test coverage)が parity 維持される | **trigger T1 発火後** — upstream deploy script が残る間は removal payoff を得られないため実施しない。T1 達成時はこれが node 除去の最後のピース |
| F5 | brainstorming skill の node 依存 drop/replace | transitive dep として到達する brainstorming(`apm.lock.yaml`: `repo_url: obra/superpowers`・`is_virtual: true`・`depth: 2`・`resolved_by: scarecrow173/apm-packages` — root `apm.yml` に `obra/superpowers` の direct 宣言は無い)の `helper.js`/`server.cjs` node 依存を除去、または node-free な skill へ置換する。実 lever は (a) maintainer 所有の upstream `scarecrow173/apm-packages` の package 宣言で brainstorming を drop/replace し本 repo 側で apm dep bump、(b) apm の exclusion/override 機構で transitive skill を除外(存在する場合 — 未確認のため前提としない)、(c) 親 dep 自体の drop(同 package の他 skill も失う) | `apm.lock.yaml` deployed_files に brainstorming の `.js`/`.cjs` が出ない。skill を落とす場合は doc workflow への影響が案内される | 独立。`scarecrow173/apm-packages` は同一 maintainer 所有のため third-party(`obra/superpowers`)の対応を待たない — T1 の unblock path の一部を単独で前進させる小規模 issue |
| F6 | stale deploy drift の運用 note + stale comment 修正 | `.agents/skills/cheaper-usage/SKILL.md` の旧 `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` 参照が repo source より古い件を `apm install` 再実行で解消する旨を記録し、併せて発見済みの comment-level mismatch 2 件を修正する — `scripts/check-lockfile.mjs` header の「check pipeline の最後尾」claim(実際は 10 step 中 9 番目)と `scripts/lib/release-artifacts.mjs` header の「check-apm-payload との共有」claim(同 file は import していない) | `apm install` 後に該当参照が消え、deploy drift が実依存でないことが確認できる。両 header comment が実態と一致する | trivial。即時可(他と無関係) |

順序サマリ: **F1 は完了**、**F6** は引き続き独立。**F5** は third-party
upstream(`obra/superpowers`)非依存の unblock path(`scarecrow173/
apm-packages` は同一 maintainer 所有)。**F2 は issue #183 との結合が
前提**。**F3・F4 は trigger T3・T1 の発火が前提**で、発火までは
起票しない。

> **Correction(2026-10-25 実測 — F6 前提の訂正)**: F6 は
> 「`.agents/skills/cheaper-usage/SKILL.md` の旧参照は `apm install`
> 再実行で解消する」を前提としたが、実測では解消しない — deploy file
> の sha256 が `apm.lock.yaml` `local_deployed_file_hashes` の pin
> 値と一致し、upstream `scarecrow173/apm-packages` の lock-pinned
> copy が再 deploy されるため。解消には upstream 側の SKILL.md 修正
> + apm dep bump が必要(operationalize plan の F6 deviation・T1
> Notes 参照)。F6 の stale comment 修正半分は本訂正とは無関係に
> 実施可能。

## Risks and Trade-offs

- **2 runtime 併存の認知コスト**: 「Node は repo tooling に限定」の境界は
  AGENTS.md・CONTRIBUTING.md・本 §2.2 で明文化して緩和。新規 ambient
  結合の禁止により結合面が広がるリスクを抑える。
- **fixture translator の対象範囲**: differential test は既知の literal
  `node -e console.log('bare node')` case を明示的に変換する。新しい literal
  Node command は自動的に任意の JS として実行せず、必要な case ごとに
  fixture mode と期待出力を追加する。
- **bare-name proof の責務**: differential fixtures は absolute fixture
  path を使う。PATH lookup/pin/spawn の保証は proc_process の isolated
  child PATH test が担うため、この test を削除・弱化しない。

## Verification Notes

- `mise run check` が現行 pipeline(node 経由 step を含む)で全 pass する
  こと — Option A は変更なしの sanity 確認。
- `git grep -n -w 'node' -- 'crates/*/src/*'` で走査し、node spawn に
  相当する箇所が 0 件であること(comment・文字列の false positive
  は目視で除外。pathspec は `src/*` まで書くこと — `'crates/*/src'`
  は `src` directory 自体にしか match せず、git の generic pathspec
  意味論で 0 match に縮退する)。
- `node scripts/check-apm-payload.mjs` が slim payload 契約を pass し、
  `packages/*/.apm/hooks/` 内に `.js`/`.mjs` payload が無いこと
  (走査範囲は hooks/ のみ — skills payload を含むそれ以外の
  `packages/**` は human review 依存、§2.3)。
- `mise.toml` の `node = "26"` pin が `mise.lock` と整合して残っている
  こと(除去提案をしていないことの確認)。
- 本 design の file 登録: `docs/designs/README.md` index への行追加と、
  `audit_docs.js --type design` で本 file に findings が無いこと。

## Addendum(2026-10-25 — issue #183 実施済み)

§5 F2(benchmark harness の Rust 化)は issue #183
(`docs/designs/benchmark-harness-redesign-issue-183.md`)へ absorb
され実施済み(harness = `crates/cheaper-bench`、manifest v2・
result schema v2 — check pipeline 整合は manifest v2 flip と同一
change で着地済み)。本契約への影響を日付付きで記録する:

- **§1.1 row 4(`node --test benchmarks/test/`)は migrated** —
  `mise.toml [tasks.check]` から行が削除され、benchmark harness の
  unit test coverage(manifest・fixture・schema・aggregate)は
  `cargo test --workspace` 内の `cargo test -p cheaper-bench` が
  担う。required node entrypoint は 6→5 件に縮小。
- **§1.2 row 8(`node benchmarks/run.mjs`)も migrated** — 手動
  計測 entrypoint は `cargo run -p cheaper-bench` へ置換。
  `{node}` workload scenario は `{fixtureChild}`(bench bin の
  自己再実行)へ、`environment.node` field は schema v2 で削除
  (再混入は validation が reject)。旧 `.mjs` file 群
  (`run.mjs`・`lib/`・`fixtures/`・`test/`)は pipeline から外れた
  未参照 file で、file 削除は #183 の T6 が担う。
- **§4 trigger T2 は発火済み** — 「改修機会に Rust 化(F2)を
  畳み込むか」の判断は畳み込み採用で確定。F2 acceptance sketch
  (計測意味論の維持・record schema 変更の version 化・
  `node --test benchmarks/test/` の cargo test 系への置換)は
  manifest v2・result schema v2・`cargo test -p cheaper-bench`
  で充足された。
- **§2.2 `benchmarks/` row** — 「harness 全体の構造変更は issue
  #183 に委ねる」前提が解消し、`benchmarks/` に残るのは
  言語中立資産(`scenarios/` manifest v2・`results/`・README)のみ。
  領域 policy 自体(新規 node 結合「可」)は不変だが、live な
  `.mjs` entrypoint は存在しない(`biome.jsonc` の
  `benchmarks/**/*.mjs` glob も除去済み — `benchmarks/**/*.json`
  include は残存 scenarios 対象で維持)。
- **§1.4「unit-test coverage/failure 意味論」invariant** —
  `node --test` suite は scripts 側のみに縮小したが、benchmark
  coverage は `crates/cheaper-bench/tests/` の Rust test へ移管
  されており、無音の coverage 低下(削減・skip 化)には該当しない
  — row 4 の保存契約は Rust 側 4 test target が継承する。

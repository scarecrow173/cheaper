---
id: "DISC-034LRjLlw4UAOWypGTWGJu"
type: "discovery"
status: "resolved"
title: "Node.js dev-tooling inventory (issue #182)"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
    - "mise.toml"
    - ".github/workflows/ci.yml"
    - ".pre-commit-config.yaml"
    - "biome.jsonc"
    - "../plans/phase-6-remove-typescript-node-pi.md"
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
    - "../adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "../adr/canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
    - "../adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/node-dev-tooling-scope-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
  verifies: []
  verified-by: []
  references:
    - "docs/guides/release.md"
    - "CONTRIBUTING.md"
    - "AGENTS.md"
    - "benchmarks/README.md"
  defers: []
  deferred-by: []
---

# Node.js dev-tooling inventory (issue #182)

> Snapshot note: the site counts, usage table, and H3 finding below record the
> inventory observed on 2026-10-25. The current disposition as of 2026-10-26 is
> recorded after section D; issue #188 removed ambient Node from those Rust
> test paths while leaving the frozen fixture corpus unchanged.

## Exploration Goal

issue #182 は、開発環境に残る **全ての Node.js 使用箇所を棚卸し**し、
証拠に基づいて (A) Node 維持 / (B) benchmarks のみ Rust 化 /
(C) repo tooling 全てを Rust/native へ移行、のいずれかを決定することを
求めている。product runtime は既に Rust(ADR-034BbhP6hJV7DFtv7k7AGs)であり、
Node が product path へ戻ることは許容されない。

なぜ今か: Phase 6(TS 撤去)では「Node は repository tooling の実行基盤として
残す」が補足判断として記録されたが(`docs/plans/
phase-6-remove-typescript-node-pi.md` §Deferred 判断の補足)、全使用箇所の
棚卸しと契約保存表を伴う正式な再検討は未実施。mise.toml から `node` を
外せるかどうかは、この inventory が前提となる。

制約:

- product path(CLI・hook・配布 payload・consumer install 経路)は
  node-free を維持すること。現行は `check-apm-payload.mjs` の
  executable 排除・slim payload 契約と Rust src 内 node spawn 不在で
  強制済み。
- `mise.toml` の `node` 除去は「全 required entrypoint が node-free に
  なったとき」に限る(issue 指定の gate)。
- 外部 contract(`mise run setup` / `mise run check` の canonical
  entrypoint、ADR-034r05iHrZUE6WeDAi63F3)は維持。

## Key Issues and Assumptions

- `.mjs` harness は既に存在し稼働している — 移行は「動く資産の書き換え +
  契約 parity 再検証」であり、新規構築ではない。
- `mise run check` の required entrypoint と、agent が doc ワークフローで
  呼ぶ optional script は別物として分類する必要がある。
- `.agents/skills/`・`.claude/skills/` 配下は `apm install` の deploy 物
  (gitignore 対象)で、upstream repo(`scarecrow173/apm-packages`・
  third-party `obra/superpowers` 等)が所有する — 本 repo の
  変更境界の外側。
- Node の removal 判定は「required entrypoint が node-free になるか」だけで
  なく、「toolchain から node を外したとき何が壊れるか」で評価する。

## Inventory Findings

### カテゴリ別集計

| カテゴリ | 件数 | required/optional | 所有 |
| --- | ---: | --- | --- |
| A. toolchain 宣言(mise.toml `[tools] node`) | 1 | required(以下全ての供給源) | repo |
| B. `mise run check` の node entrypoint | 6 | required | repo |
| C. 手動 repo tooling(set-version・benchmark harness・glob table codegen) | 3 + 付随 lib/fixture | optional-manual | repo |
| D. Rust test 内の ambient `node` 依存 (2026-10-25 snapshot) | 3 site + dormant fixture 宣言 | 2 required / 1 silent-skip at snapshot time | repo |
| E. APM deploy 済み doc-suite `.js`/`.cjs` script | 42 `.js` + 1 `.cjs` / 14 skill(≈86 deployed copies) | agent 実行時 required・upstream 所有 | upstream |
| F. node 非依存と確認済みの隣接領域 | (一覧は後述) | — | — |

### A. Toolchain 宣言

| # | 場所 | 用途 | 呼出し元 | required? | 移行性 | 残す理由 / 保存契約 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `mise.toml:10` `node = "26"` + `mise.lock`(node 26.12.1 全 platform) | 以下全 entrypoint の実行 runtime | `mise install` / `jdx/mise-action`(`install_args: --locked`) | yes | n/a(宣言) | required entrypoint が全て node-free になるまで除去不可 |

### B. `mise run check` の node entrypoint(required・全 6 件)

| # | 場所 | 用途 | 呼出し元 | 移行性 | 保存契約(検出する失敗) |
| --- | --- | --- | --- | --- | --- |
| 2 | `scripts/check-version.mjs`(65 行) | `Cargo.toml [workspace.package].version` と `packages/apm/apm.yml` + `packages/apm-hook-*/apm.yml` の version 宣言一致 | `mise run check`(4 番目) | 低 — xtask crate または数行の shell(Rust なら regex/fs のみ) | version 宣言 drift(CI fail)。canonical source は ADR-034CpwNUNIDqrR92x7VaUR(superseded — canonical-version 決定自体は継続。現行の運用規範は `docs/guides/release.md`) |
| 3 | `scripts/check-doc-ids.mjs`(53 行) | tracked markdown(docs/・AGENTS.md・README.md・`packages/*/.apm/**/*.md`)の legacy `TYPE-NNNN` ID 検出 | `mise run check` | 低 — xtask(`git ls-files` + regex 走査) | 旧連番 artifact ID の再混入 |
| 4 | `scripts/check-lockfile.mjs`(48 行) | `cargo metadata --locked` + `git diff --exit-code -- Cargo.lock` | `mise run check`(10 項目中 9 番目・payload check 直前 — ただし script 自身の header comment は「最後尾に置く」と主張する stale 記述であり、実際の最後尾は `check-apm-payload.mjs`。doc/code comment 不整合として要修正) | 極低 — mise task の run 行 2 行に畳める(script 自体不要) | committed Cargo.lock の manifest 不整合・先行 step による working-tree drift(consumer `cargo install --locked` 契約) |
| 5 | `scripts/check-apm-payload.mjs`(275 行) | slim APM payload 契約。`scripts/test/apm-payload.test.mjs` から library import もされる二役 | `mise run check`(最後尾) | 中 — xtask(fs walk・magic byte・BOM/CRLF・git index mode・line parse JSON/YAML) | 必須 asset 欠落・`cli/` 残存・extension 違反・PE/ELF payload・launcher byte 契約(BOM+CRLF / shebang+LF / mode 100755)・copy 間 drift・`dependencies.apm` wiring 欠落・hooks.json↔launcher 不整合・bare `./cheaper` 参照 |
| 6 | `node --test scripts/test/`(6 file + `lib/apm-fixture.mjs`、計 ≈1,236 行) | repo tooling test: `apm install` 実実行の deploy matrix・package layout 分離・payload contract・bulk-reader skill drift・devin hooks install・launcher passthrough | `mise run check` | 中 — cargo integration test へ(spawn `apm`/sh/pwsh は `std::process::Command` で等価) | 配布 contract の回帰(deploy layout・per-target event・launcher 透過・payload byte) |
| 7 | `node --test benchmarks/test/`(4 file、計 ≈312 行) | benchmark harness の unit test(aggregate/measurements/result-schema/scenario-manifest) | `mise run check` | 中 — cargo test(harness を Rust 化する場合は自然に付随) | 集約・計測・schema・manifest coverage の回帰 |

### C. 手動 repo tooling(optional)

| # | 場所 | 用途 | 呼出し元 | required? | 移行性 | 保存契約 |
| --- | --- | --- | --- | --- | --- | --- |
| 8 | `scripts/set-version.mjs`(61 行) + `scripts/lib/release-artifacts.mjs`(54 行) | release version bump: Cargo.toml canonical + 全 apm.yml を一括更新し check-version を自己呼出 | 手動(`docs/guides/release.md` の release 手順) | optional(ただし release 契約と結合) | 低 — xtask | bump 漏れを防ぐ一括更新 + 更新後自己検証。同 lib を共有するのは `check-version` のみ(`check-apm-payload` は node builtin のみを import — `release-artifacts.mjs` の header comment にも「check-apm-payload と共有」との stale 主張があり、code comment 側の修正が望ましい) |
| 9 | `benchmarks/run.mjs`(74 行)+ `benchmarks/lib/*.mjs`(6 file ≈1,076 行)+ `benchmarks/fixtures/fake-worker-stream.mjs`(102 行)+ `benchmarks/scenarios/*.json` | workflow arm vs scripted direct arm の計測 harness。cargo build 済み `target/{release,debug}/cheaper` を spawn し、`CHEAPER_TEST_FAKE_STREAM` dev-seam 向け RunFixture JSON を spawn 毎に生成 | 手動 `mise exec -- node benchmarks/run.mjs`(`mise run check` には非配線 — mise.toml:61 コメント) | optional(計測時のみ) | 中〜大 — Rust harness(crate/xtask ≈1,560 行相当)。`scenarios.json` の `{node}` → `process.execPath` 置換と `manual.processScript` spawn が「計測対象 workload としての node」を含む点に注意 | workflow vs direct の orchestration overhead 計測。結果 schema の `environment.node` field(process.version)を持つ |
| 10 | `crates/cheaper-tools/gen-glob-tables.cjs`(62 行、CommonJS `require`) | `crates/cheaper-tools/src/glob/tables.rs`(生成物)の glob POSIX class Unicode range table を、Node の `/^[\p{...}]$/u` regex を全 code point に試行した結果を oracle として生成 — Node bundled ICU の `\p{...}` semantics を mirror する codegen oracle。generated output に `process.version` を埋め込み、checkin 済み `tables.rs` header は `Node v26.10.0` を記録(mise.lock pin は 26.12.1 — bundled ICU 経由で codegen が node-version-sensitive) | 手動 `node crates/cheaper-tools/gen-glob-tables.cjs <out-path>` — maintainer が table 再生成時に実行(`tables.rs:9` の Regenerate 手順) | optional(再生成時のみ) | **単純移植不可** — regex-semantics oracle(Node bundled ICU の `\p{...}` 解釈)の再エンジニアリングが必要。glob-table 生成自体を再設計しない限り、full Option C でもこの entry point は残る | table 再生成の再現性(Node version 間の ICU drift を generated header の `process.version` で追跡) |

### D. Rust test 内の ambient `node` 依存(隠れた結合)

| # | 場所 | 用途 | 呼出し元 | required? | 移行性 | 保存契約 |
| --- | --- | --- | --- | --- | --- | --- |
| 11 | `crates/cheaper-tools/tests/proc_process.rs` `node_task_resolves_via_path` | bare `node` task が ambient PATH で実 exe へ解決されることを実 spawn で検証 | `cargo test --workspace`(check 内) | required(不在時 **fail**) | 小 — fixture child binary / PATH 上の別 exe へ repoint | 「bare 名 → PATH 解決 → spawn 成功」の tool contract。コメントで「mise toolchain の node が PATH 上にある前提は `mise run check` 経路で保証」と明記済み |
| 12 | `crates/cheaper-tools/tests/fixtures/process.json` case `"bare node task resolves via PATH"` | differential golden。literal `node -e` task を process tool で実行 | `cargo test`(differential `process_fixtures`) | required(不在時 diverge → **fail**) | 小 — 同上 | 期待 stdout `"bare node\n"` exitCode 0 の golden。`<NODE>` task は `diff_fixture_child` へ翻訳済みだが literal `node` は翻訳対象外 |
| 13 | `crates/cheaper-workflow/tests/process.rs` `resolve_node()` | process task compile + 実行 e2e(`node child.mjs` で file 生成) | `cargo test` | optional(node 不在時 **silent skip** で coverage 低下) | 小 — fixture binary へ repoint | policy process task の materialize→execute 検証。skip は fail ではないため見落としやすい |

* `crates/cheaper-tools/tests/fixtures/runtime.json` にも literal
  `"node"` task 宣言が 6 件残る(約 L398・633・918・1158・1361・
  1842)。今日の op は `<NODE>` 翻訳 task のみを使い、これらは一度も
  invoke されない dormant 宣言だが、将来 `task:"node"` を呼ぶ op の
  追加や task validation の eager 化があれば latent coupling になる。

**Current disposition (2026-10-26):** issue #188 completed F1. The dedicated
`node_task_resolves_via_path` test now resolves a bare Rust fixture executable
through a controlled child PATH, pins it, and spawns it. Differential process
and runtime tests translate the known literal Node fixture command to the
absolute Rust fixture binary; this translation preserves output but is not
the PATH proof. The workflow process test re-executes its own test binary and
does not skip when Node is absent. All three tests passed under an explicit
Node-free WSL PATH. The six runtime JSON declarations remain historical frozen
inputs and are intercepted by the test translator, so they no longer create
ambient Node coupling. The Node tool pin remains for repository tooling and
deployed upstream doc-suite scripts; this update does not claim the broader
Node removal gate is complete.

### E. APM deploy 済み doc-suite `.js`/`.cjs`(upstream 所有・agent 実行時)

| # | 場所 | 用途 | 呼出し元 | required? | 移行性 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 14 | `.agents/skills/*/scripts/` — 42 `.js` + 1 `.cjs` / 14 skill(`adr-doc` 8・`doc-driven-dev-graph` 7・`skill-discovery-protocol` 6・`impl-doc` 6・`doc-maintenance` 4・`doc-status` 3・`test-spec-doc`/`task-doc`/`spec-doc`/`plan-doc`/`idea-doc`/`discovery-doc`/`design-doc` 各 1 = doc-suite 41 `.js`、+ brainstorming の `helper.js` 1・`server.cjs` 1)。`.agents/skills/` と `.claude/skills/` の **双方** へ deploy され、合計 ≈86 deployed copies | doc-driven 開発 workflow の生成・audit・routing script(`new_*.js`・`audit_docs.js`・`doc_status.js`・`route_graph.js`・`inspect_graph.js`・`migrate_ids.js`・`review_adr.js` 等)。doc-suite script は bundled 単一 file(`#!/usr/bin/env node`)。brainstorming は別形態 — `start-server.sh` → `node server.cjs` で local server を起動する(`node <script>` の doc-workflow pattern ではない) | agent が SKILL.md 記載の `node .agents/skills/.../scripts/*.js`(または `.claude/skills/` 側)を実行(briefing/spec/adr/graph routing)。brainstorming は上記の local server 経由。`apm install` で deploy(apm.lock.yaml の `deployed_files`) | agent doc workflow では実質 required。`mise run check` の entrypoint ではない | **本 repo からは移行不可** — upstream 所有の deploy 物で、per-package の帰属は: 41 `.js` が `scarecrow173/apm-packages`(doc-driven-dev package)、brainstorming の `helper.js` + `server.cjs` が third-party `obra/superpowers`(`resolved_by` は `scarecrow173/apm-packages`)。`addyosmani/agent-skills`・`github/awesome-copilot` は `.js` を deploy しない(所有ゼロ)。移行は upstream issue | `mise.toml` から node を外すとこれら全てが `command not found` になる。AGENTS.md:81 も「doc-suite skills」を node 残存理由として明記済み |

### F. node 非依存と実確認した隣接領域(inventory の負側)

| 場所 | 確認方法 | 結果 |
| --- | --- | --- |
| `.github/workflows/ci.yml` | 全文読了 | `jdx/mise-action` + `mise run setup`/`mise run check` のみ。node 直接呼出なし |
| `.pre-commit-config.yaml` | 全文読了 | `mise exec -- biome check --write` + `cargo fmt` + generic hygiene。node なし |
| `biome` tool | mise.toml:11-14 コメント + backend | `aqua:biomejs/biome` の standalone Rust binary(npm 非経由)。node 不要。lint 対象は `.mjs` + `.json` |
| `packages/apm-hook-*/.apm/hooks/cheaper.{sh,ps1}` | grep | PATH 上 `cheaper` を解決する thin launcher。node 参照なし |
| `packages/` 全体 | `find packages -name "*.mjs" -o -name "*.js"` | 0 件 — slim payload 契約どおり |
| `crates/cheaper-cli/build.rs` | 全文読了 | `CARGO_PKG_VERSION` → `version.json` 生成のみ。node 呼出なし |
| `crates/**`(src・test・crate root 等 非 src 含む) | `Command::new("node")` grep + `find crates \( -name "*.mjs" -o -name "*.cjs" \)` 全量走査 | src に node spawn なし(`route/resolve.rs` の `INTERACTIVE_COMMANDS` 語彙と `executable.rs` の fake-file unit test のみ — 実 node 不要)。非 src の `.cjs` は `gen-glob-tables.cjs` 1 件のみ → §C row 10 で正面記載 |
| `docker-compose.yml` | 全文読了 | ollama コンテナのみ |
| `cheaper run` 実行経路 | src grep + binary 確認 | Rust binary(`target/debug/cheaper.exe` 存在)。node spawn なし |
| `docs/guides/managed-process.md`・`migration.md`・`command.md` | grep | `node` は consumer 側 workload/deny vocab の記述例 — product 仕様の話で dev-env 依存ではない |
| `.agents/skills/cheaper-usage/SKILL.md` | grep | deploy 済み copy が `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` の旧経路を参照 — repo source(`packages/apm/.apm/skills/`)は除去済みの **stale deploy drift**(実依存ではない。`apm install` 再実行で解消) |

### 呼出し graph

```text
CI (jdx/mise-action --locked)
 └─ mise install → node 26.12.1 on PATH
     ├─ mise run setup → apm install → .agents/skills/*/scripts/*.js deploy
     │                                 └─ agent doc workflow → node <script>   [E]
     └─ mise run check
         ├─ biome check                              (native — node 不要)
         ├─ cargo fmt --check / cargo clippy         (native)
         ├─ node scripts/check-version.mjs           [B2]
         ├─ node scripts/check-doc-ids.mjs           [B3]
         ├─ cargo test --workspace                   → Rust fixture coverage (F1 complete) [D]
         ├─ node --test scripts/test/                [B6]
         ├─ node --test benchmarks/test/             [B7]
         ├─ node scripts/check-lockfile.mjs          [B4]
         └─ node scripts/check-apm-payload.mjs       [B5]
manual: node scripts/set-version.mjs                  [C8]
manual: node benchmarks/run.mjs                       [C9]
        └─ spawn target/{release,debug}/cheaper
           └─ scenario {node} → process.execPath emit.mjs ×2 scenario
manual: node crates/cheaper-tools/gen-glob-tables.cjs <out>  [C10]
        └─ glob tables.rs 再生成(Node \p{...} regex oracle)
```

## Alternatives and Comparison

| Option | 内容 | 利点 | 欠点 | 傾き |
| --- | --- | --- | --- | --- |
| A: Node 維持 | `node = "26"` pin を残し、repo tooling は `.mjs` のまま | 変更コスト 0。全 check 契約が実績のある実装で維持される。doc-suite・benchmarks・`cargo test` ambient 依存は全て現行で整合(mise が node を保証)。CI cold-start 増分 0 | toolchain に node pin が残る(security bump 随伴、26.12.1 のような定期更新)。「2 runtime 併存」の認知コスト。D の ambient 結合が残る(ただし意図的・文書化済み) | **yes** |
| B: benchmarks のみ Rust 化 | `run.mjs` + `lib/` + `test/` を Rust harness へ | 最大の単一 .mjs 面(≈1.6k 行)を減らせる | required entrypoints は 6→5 に減るが(`benchmarks/test` は 6 件の required check entrypoint の 1 つ)、残り 5 entrypoint + deployed doc-suite script が node を要するため `mise.toml` の node pin 除去には寄与しない。稼働中 harness の書き換えリスクと結果 schema churn(`environment.node`)。`{node}` workload scenario は結局 node を spawn するか workload 再設計が必要 | no |
| C: repo tooling 全て Rust/native 化 | `scripts/*.mjs` → xtask/薄い mise 行、`scripts/test`・`benchmarks/test` → cargo test、benchmarks → Rust harness、D の ambient 結合 → fixture binary | `mise run check` が node-free になる。単一 runtime の開発環境。cargo test の toolchain 独立性が上がる | ≈3.4k 行の rewrite(repo `.mjs` ≈3,356 + codegen `.cjs` 62)+ contract parity 再検証。**upstream 所有の doc-suite `.js`/`.cjs` が残る限り node は toolchain に必要**(agent doc workflow が壊れる)→「node 除去」の主目標は本 repo の境界内では達成不能。さらに `gen-glob-tables.cjs` は regex-semantics oracle の再設計なしには移植できず、glob-table 生成の再設計なしに entry point として残る。security-sensitive な `check-apm-payload` byte 契約の移植リスク。CI では xtask compile が乗るが `mise run setup` の all-targets build に吸収されるため軽微 | no(現時点) |

### コスト・CI・cross-platform の補助分析

- **rewrite 規模**: repo 所有 `.mjs` は scripts 6 file(entrypoint 5 +
  lib 1)+ scripts/test 7 file + benchmarks 12 file(run + lib 6 +
  fixture + test 4)= 25 file・計 ≈3,356 行。codegen
  `gen-glob-tables.cjs`(62 行)を含めると ≈3.4k。check-* 系は小〜中、
  benchmarks が最大面(≈1.6k)。
- **CI**: 現行 CI は `mise run setup` + `mise run check` のみ。Rust 化しても
  xtask/integration test の compile は setup の `cargo build
  --workspace --all-targets` に吸収される。cold-start 悪化は軽微だが、
  node tarball の download を外せる程度の gain に留まる。
- **cross-platform**: `.mjs` は node が POSIX/Windows の差を吸収する。
  Rust 移行では `check-lockfile` が抱える Windows shell 経由 spawn
  (cargo shim `.bat`)や `git ls-files -s` mode 参照・byte 契約を
  `std::process::Command`/fs で書き直す必要がある — 設計上問題ないが
  parity 検証は必須。doc-suite は upstream 側で同様の問題を既に解決済み。
- **upstream 境界**: E(42 `.js` + 1 `.cjs`)は `apm.lock.yaml` pin の
  deploy 物であり、本 repo での書き換えは `apm install` で上書きされる。
  node 完全除去は upstream migration 待ち(doc-suite は
  `scarecrow173/apm-packages`、brainstorming は third-party
  `obra/superpowers`)か doc-suite 運用自体の放棄が前提 — 後者は
  repo の document-driven 開発系を捨てることを意味し現実的でない。

## Tentative Conclusions and Hypotheses

- **H1(confirmed)**: product path は完全に node-free。CLI・hook
  launcher・配布 payload・build.rs・src に node spawn/参照はない
  (slim payload check が executable 混入を機械的に排除)。
- **H2(confirmed)**: required entrypoint(`mise run check`)の node 面は
  check-* 4 script + 2 node:test スイートの 6 件に集約される。
  いずれも小〜中規模で、Rust(xtask/cargo test)への移植は技術的に容易。
- **H3(confirmed as of 2026-10-25)**: `cargo test` には ambient `node` への暗黙結合が
  3 site ある(proc_process 必須・process.json golden 必須・
  workflow process silent-skip)。`mise run check` 経路では mise が
  node を供給するため現行では整合している。
- **H4(confirmed)**: `mise.toml` から node を外しても壊れないためには、
  B+C+D 全てが node-free であることが必要だが、**それでも E(doc-suite
  42 `.js` + 1 `.cjs`・upstream 所有)が残る**。agent doc workflow
  (`new_*`・`audit_docs`・`route_graph` 等)は node 前提であり、
  本 repo の変更境界内では node の完全除去は達成不能。
- **H5(hypothesis)**: したがって現時点の最適解は **Option A(Node 維持)**。
  B は required entrypoint を 5 に減らすものの node pin 除去に寄与せず
  optional 面に rewrite コストを払い、
  C は upstream 境界に阻まれて removal payoff を得られない。
  `node` 除去が意味を持つのは upstream doc-suite が node-free 化した
  時点以降であり、そのとき repo 側 C 相当の移行が最後のピースになる。

## Open Questions

- upstream の node 依存 script を node-free 化する構想があるか/起こせるか
  — doc-suite(`scarecrow173/apm-packages`)と brainstorming
  (`obra/superpowers`、third-party で maintainer 管理外)の双方。
  本 repo の issue ではなく upstream 側の意思決定事項。node 除去の
  revisit trigger(brainstorming は drop/replace も別の unblock path)。
- benchmark scenario の `{node}` workload(emit.mjs)を node-free に
  再設計する際の比較意味論 — C を選ぶ未来でのみ問いになる。
- `cargo test` を ambient-node 非依存にする fixture binary 化は
  Option A でも独立して有益か(小コストの衛生改善として扱うか) —
  follow-up issue の候補。

## Promotion Candidates

- [x] spec-doc needed: issue #182 の done 定義(inventory・契約保存表・
  ADR・follow-up issue 群の受入条件)— `docs/specs/node-dev-tooling-scope-issue-182.md`
- [x] adr-doc needed: (A)/(B)/(C) の toolchain 決定 —
  `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
- [ ] design-doc: 移行(Option C 系)を選ばない限り不要。

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/182>
- 実測(2026-10-25 実施): `mise.toml`/`mise.lock`・`scripts/`(6
  entrypoint + lib + 7 test 関連 file の全文)・`benchmarks/`(run.mjs +
  lib 6 + fixture + scenarios.json の全文)・`.github/workflows/ci.yml`・
  `.pre-commit-config.yaml`・`biome.jsonc`・`crates/*/build.rs`・
  `git grep "node"` による crates src/test・docs/・packages/ の
  横断走査・`find crates \( -name "*.mjs" -o -name "*.cjs" \)` による
  crate 非 src 走査(`gen-glob-tables.cjs` を検出)・
  `find .agents/skills .claude/skills \( -name "*.js" -o -name "*.cjs" \)`
  (各 dir 42 `.js` + 1 `.cjs`、合計 ≈86 copies)・
  `mise exec -- node --version`(v26.12.1、mise shim 経由で稼働中)
- 先行判断: `docs/plans/phase-6-remove-typescript-node-pi.md`
  §Deferred 判断の補足(「Node は repository tooling の実行基盤として
  残す」— security-sensitive な検証ロジックの移植コスト・リスクは
  製品上の利益に見合わない)
- 関連 ADR: `migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  (product の node 撤去)・`benchmark-harness-placement-and-accounting.md`
  (benchmarks を root `benchmarks/` の plain Node runner とした決定)・
  `canonical-setup-check-entrypoints-as-mise-toml-tasks.md`(check
  pipeline 契約)・`distribute-cli-via-cargo-source-install-and-thin-path-launchers.md`
  (cargo install + thin launcher、slim payload)

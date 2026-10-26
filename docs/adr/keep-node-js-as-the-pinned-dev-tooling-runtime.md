---
id: "ADR-034LRjX5iYZHrubmKgKAL1"
type: "adr"
status: "accepted"
title: "Keep Node.js as the pinned dev-tooling runtime"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/node-dev-tooling-scope-issue-182.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/node-dev-tooling-scope-issue-182.md"
    - "benchmark-harness-placement-and-accounting.md"
    - "canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
    - "distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
    - "../specs/node-dev-tooling-scope-issue-182.md"
  derived-by:
    - "../plans/remove-ambient-node-from-rust-tests-issue-188.md"
    - "../specs/remove-ambient-node-test-coupling-issue-188.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../designs/remove-ambient-node-from-rust-tests-issue-188.md"
    - "../plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    - "../test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Keep Node.js as the pinned dev-tooling runtime

## Context and Problem Statement

issue #182 は dev 環境に残る全 Node.js 使用箇所の棚卸しと、(A) Node
維持 / (B) benchmarks のみ Rust 化 / (C) repo tooling 全て Rust・
native 化 の toolchain 決定を求める。product runtime は既に Rust であり
node を含まない — 確認済みの実測: launcher(`cheaper.sh`/`cheaper.ps1`)は
PATH 上 `cheaper` を解決する thin wrapper、`crates/cheaper-cli/build.rs`
は version.json 生成のみ、src に node spawn なし、`packages/` に
`.js`/`.mjs` payload 0 件、CI(`ci.yml`)は mise-action + `mise run
setup`/`mise run check` のみ。

棚卸しの結果(詳細は
`docs/discovery/node-dev-tooling-inventory-issue-182.md`):

- **required entrypoint**: `mise run check` 内の node は
  `check-version.mjs`・`check-doc-ids.mjs`・`node --test
  scripts/test/`・`node --test benchmarks/test/`・
  `check-lockfile.mjs`・`check-apm-payload.mjs` の 6 件に集約される。
- **optional/manual**: `set-version.mjs`(release bump)、
  `benchmarks/run.mjs` + lib(benchmarks 全体 ≈1.6k 行、手動実行のみ)、
  `crates/cheaper-tools/gen-glob-tables.cjs`(62 行 — glob table
  codegen。Node `\p{...}` regex semantics を oracle とし、generated
  output に `process.version` を埋め込む node-version-sensitive な
  手動 `node` 呼出し)。
- **ambient 結合**: `cargo test` は ambient `node` に 3 site で依存
  (`proc_process::node_task_resolves_via_path`・`process.json`
  "bare node task resolves via PATH" は必須、`cheaper-workflow`
  `tests/process.rs` は不在時 silent skip)。
- **upstream 境界**: `.agents/skills/`・`.claude/skills/` の双方へ
  deploy される `.js` 42 file + `.cjs` 1 file(14 skill、≈86
  deployed copies)は `apm install` で deploy される upstream 所有物
  — 41 `.js` は `scarecrow173/apm-packages`(doc-driven-dev
  package)、brainstorming の `helper.js` + `server.cjs` は
  third-party `obra/superpowers`(`resolved_by` は apm-packages)。
  agent の doc workflow(`new_*`・`audit_docs`・`route_graph` 等)が
  node 前提でこれらを呼び、brainstorming は `start-server.sh` →
  `node server.cjs` の local server として起動される。AGENTS.md:81
  も「doc-suite skills」を node 残存理由として明記済み。
- 先行判断: Phase 6 plan は「security-sensitive な検証ロジックを別
  runtime へ移植するコスト・リスクは製品上の利益に見合わない」として
  Node を repo tooling に残すことを補足決定済み。

## Decision Drivers

- product path は node-free を維持すること(既達成・契約で強制済み)。
- `mise.toml` の `node` 除去は全 required entrypoint が node-free に
  なったときに限る(issue 指定の gate)。
- upstream 所有の doc-suite `.js` は本 repo の変更境界の外側であり、
  ここでの書き換えは `apm install` で上書きされる。
- `.mjs` harness は既に存在し、test 付きで稼働している — 移行は
  「動く資産の書き換え + contract parity 再検証」であり、その
  コストに見合う便益が必要。
- KISS/YAGNI: 除去の payoff を得られない rewrite はしない。

## Considered Options

- **Option A: Node.js を pin した dev-tooling runtime として維持する**。
  `mise.toml` の `node = "26"`(mise.lock = 26.12.1)を残し、
  `scripts/`・`benchmarks/` の `.mjs` と doc-suite `.js` は現行の
  まま運用する。
- **Option B: benchmark harness のみ Rust 化する**。`run.mjs` +
  `lib/*.mjs` + `test/*.test.mjs` + fake-stream fixture generator を
  Rust harness へ移植する。
- **Option C: repo 所有 tooling を全て Rust/native へ移行する**。
  `scripts/*.mjs` → xtask crate または mise run 行、
  `scripts/test`・`benchmarks/test` → cargo integration test、
  benchmarks → Rust harness、`cargo test` の ambient `node`
  結合 → fixture binary。なお `gen-glob-tables.cjs`(62 行)は
  Node regex-semantics oracle の再設計を要し、glob-table 生成自体を
  再設計しない限り node entry point として残る。

## Decision Outcome

**Option A** を採用する。

理由は次のとおり。

- **node 完全除去の payoff は本 repo の境界内では達成不能**。
  `.agents/skills/`・`.claude/skills/` の script(42 `.js` + 1
  `.cjs`)は upstream(`scarecrow173/apm-packages` と third-party
  `obra/superpowers`)所有の deploy 物で、agent の doc workflow が
  それらを node で呼ぶ。required entrypoint を全て Rust 化しても、
  `mise.toml` から node を外すと doc-driven 開発
  workflow(briefing/spec/adr 生成・`audit_docs`・`route_graph`)が
  `command not found` で壊れる。つまり Option B/C を実施しても
  node は toolchain に残り、removal の主目標を達成できない。
- **Option B は required entrypoints を 6→5 に減らすだけで、
  node pin 除去に寄与しない**。`benchmarks/test` は 6 件の required
  check entrypoint の 1 つであり、Rust 化でそれは 1 件減るが、残り
  5 entrypoint + deployed doc-suite script が node を要するため
  `mise.toml` の node pin は残る。harness 本体は optional(手動
  実行)であり、その ≈1.6k 行の書き換えは removal に無関係な
  コストとなる。結果 schema の `environment.node` field・`{node}`
  workload scenario(emit.mjs を `process.execPath` で spawn する
  2/7 scenario)の再設計も伴い、稼働中 harness の回帰リスクに対し
  得るものがない。
- **Option C の repo 側 rewrite(≈3.4k 行: `.mjs` ≈3,356 +
  codegen `.cjs` 62)は技術的に容易だが、payoff が upstream に
  阻まれる**。`check-*` 系は xtask/薄い mise 行で、`node --test`
  系は cargo test で等価に書けるが、最終目標(node 除去)は
  doc-suite が残る限り達成されない。さらに `gen-glob-tables.cjs`
  は Node regex semantics oracle の再設計を要し、glob-table 生成の
  再設計なしに entry point として残る。security-sensitive な
  `check-apm-payload` の byte 契約(BOM/CRLF・magic byte・git
  mode・wiring)を書き換えるリスクだけが先に確定する。
- **Option A の残余コストは小さい**。node pin の定期 security bump
  (例: 26.12.1)は mise.lock の機械的更新で済み、biome のような
  npm 非経由 binary とは別種の保守対象として管理可能。`cargo test`
  の ambient `node` 結合は「mise toolchain が node を供給する」
  前提で意図的に設計されており(proc_process.rs:959-963
  コメント)、Option A の下では一貫している。

### Consequences

* Good, because 全ての check/test/doc-workflow 契約が実績のある実装で
  維持され、移行リスク・parity 再検証コストが発生しない。
* Good, because upstream doc-suite が node 前提で動き続けるため、
  doc-driven 開発系(briefing/spec/adr/plan/task の生成・audit・
  graph routing)が無改変で利用できる。
* Good, because `mise run check`・`cargo test` の ambient `node`
  依存が「mise が toolchain として供給する」形で整合したままになる。
* Bad, because toolchain に `node` pin が残り、security bump 随伴の
  mise.lock 更新が継続する(機械的作業)。
* Bad, because `cargo test` は mise env 外(node 非存在 PATH)では
  `proc_process::node_task_resolves_via_path` と `process.json`
  golden case が失敗する — mise 前提の開発経路では問題ないが、
  隠れた環境結合として残る(軽減策は follow-up 候補)。
* Bad, because repo に Rust + Node の 2 runtime が残り、「Node は
  repo tooling に限定」という境界の認知コストが残る
  (AGENTS.md・CONTRIBUTING.md で明文化済みで緩和)。
* Neutral, because product path の node-free は Option によらず不変
  であり、本決定は dev-tooling のみを対象とする。

### Confirmation

- `mise.toml` の `[tools] node` pin が残り、`mise run check` が
  現行 pipeline(biome → fmt → clippy → check-version →
  check-doc-ids → cargo test → node --test ×2 → check-lockfile →
  check-apm-payload)を変更なく維持すること。
- product path が node-free であること: `check-apm-payload.mjs`
  (executable payload 排除)・`git grep 'Command::new("node"'`
  による src 走査・launcher の `cheaper` 参照のみ、で確認可能。
- revisit trigger(下記)発火時に本 ADR を見直すこと。

## Implementation Plan

* Affected paths: 変更なし(decision-as-record)。参考対象は
  `mise.toml`(`[tools] node` pin 維持)・`scripts/`・`benchmarks/`・
  `.agents/skills/`・`.claude/skills/`(upstream deploy 物)・
  `crates/cheaper-tools/{gen-glob-tables.cjs,src/glob/tables.rs}`・
  `crates/cheaper-tools/tests/{proc_process.rs,fixtures/process.json}`・
  `crates/cheaper-workflow/tests/process.rs`。
* Patterns to follow: Node は repo tooling 用途のみとする AGENTS.md /
  CONTRIBUTING.md の記載を維持する。新規 `.mjs` tooling を追加する
  場合も同じ境界(repo tooling=Node 可、product path=Node 不可)に
  従う。
* Patterns to avoid: `mise.toml` から `node` を部分除去すること
  (required entrypoint・benchmarks・doc-suite が残っている間は
  破壊的)。product path・配布 payload・hook 経路への node 再導入。
  benchmarks harness の部分 Rust 化(契約 churn のみが残る)。
* Dependencies or configuration: `node = "26"` pin + `mise.lock`
  継続。biome・prek・apm 等の他 tool は変更なし。
* Follow-up issue 候補(optional・本決定と独立):
  - `cargo test` の ambient `node` 結合(proc_process
    `node_task_resolves_via_path`・`process.json` bare-node case・
    `cheaper-workflow/tests/process.rs`)を fixture binary /
    PATH 制御へ repoint し、test suite の toolchain 独立性を上げる
    衛生 issue(小規模)。
  - stale deploy drift: `.agents/skills/cheaper-usage/SKILL.md` の
    `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` 参照が repo
    source より古い — `apm install` 再実行で解消する運用 note。
    **Correction(2026-10-25 実測)**: `apm install` 再実行では解消
    しない — deploy file の sha256 が `apm.lock.yaml`
    `local_deployed_file_hashes` の pin 値と一致する lock-pinned
    stale copy で、upstream `scarecrow173/apm-packages` の
    SKILL.md 修正 + apm dep bump が解消に必要。
* Revisit triggers: (1) deploy 済みの node-requiring skill 群(現行は
  `scarecrow173/apm-packages` の doc-driven-dev package +
  `obra/superpowers` 経由の brainstorming)が全て node-free 化されるか、
  `apm.yml` dependencies から drop されたとき — repo 側の
  Option C 相当(`.mjs` → xtask/cargo test・ambient node → fixture)
  が node 除去の最後のピースとなるので再検討する。なお
  brainstorming の upstream は third-party(maintainer 管理外)のため、
  brainstorming skill の drop/replace 自体が別の unblock path に
  なる。(2) benchmark harness の大改修(scenario・計測面の変更)が
  必要になったとき — その機会に Rust 化を畳み込むか再評価する。
  (3) `node` pin の security bump 頻度や 2-runtime 管理コストが
  顕在化したとき。

## Verification

* [ ] `mise run check` が `node` 経由 step を含む現行 pipeline の
  まま全 pass すること(変更なしの sanity)。
* [ ] `git grep -n 'Command::new("node"' -- 'crates/*/src'` が
  0 件であり、product src に node spawn が無いこと。
* [ ] `node scripts/check-apm-payload.mjs` が slim payload 契約を
  pass し、packages 内に `.js`/`.mjs` payload が無いこと。
* [ ] inventory document(`docs/discovery/node-dev-tooling-inventory-issue-182.md`)
  の各 entry が「場所・用途・呼出し元・required・移行性・保存契約」を
  持ち、A-F カテゴリを網羅していること。
* [ ] `mise.toml` に `node` pin が残り、`mise.lock` と整合している
  こと(除去提案をしていないことの確認)。

The option comparison below records the decision-time findings from
2026-10-25. Subsequent implementation evidence is appended under
`Implementation follow-up` at the end of this ADR.

## Pros and Cons of the Options

### Option A: Node.js を pin した dev-tooling runtime として維持 — 採用

* Good, because 変更コスト・回帰リスクが 0 で、全 check 契約
  (version/lockfile/payload/doc-id/test)が既存実装で維持される。
* Good, because upstream 所有の deploy script(42 `.js` + 1 `.cjs`)
  もそのまま動き、agent doc workflow を壊さない。
* Good, because `cargo test` の ambient `node` 結合が mise toolchain
  供給という明示前提で整合する。
* Bad, because toolchain に node pin が残り、security bump の定期
  更新と「2 runtime 併存」の認知コストが継続する。
* Bad, because `cargo test` が mise env 外(node 不在 PATH)では
  2 件 fail + 1 件 silent skip となる結合が残る。

### Option B: benchmark harness のみ Rust 化 — 不採用

* Good, because 最大の単一 `.mjs` 面(benchmarks 全体 ≈1.6k 行)を
  Rust に揃えられる。
* Bad, because required entrypoints は 6→5 に減るが(`benchmarks/test`
  が cargo test 化するため)、残り 5 entrypoint + deployed doc-suite
  が node を要し、`mise.toml` の node pin 除去に寄与しない。
* Bad, because 結果 schema の `environment.node` field と `{node}`
  workload scenario(2/7 scenario が `process.execPath` で emit.mjs
  を spawn)の再設計を伴い、稼働中 harness の計測比較意味論を
  churn させる。

### Option C: repo 所有 tooling を全て Rust/native 化 — 不採用(現時点)

* Good, because `mise run check` が node-free になり、`cargo test`
  も ambient node 非依存にできる。tooling が単一 runtime に統一される。
* Good, because check-* script は小規模(48-275 行)で xtask/mise
  行への移植が素直であり、test 系も cargo integration test に
  自然に写る。
* Bad, because upstream 所有の deploy script(42 `.js` + 1 `.cjs`)が
  残る限り `node` は toolchain に必須であり、removal の payoff を
  得られない — ≈3.4k 行の rewrite + contract parity 再検証コスト
  だけが確定する。`gen-glob-tables.cjs` も regex oracle の再設計
  なしには残る。
* Bad, because `check-apm-payload` の byte-level 契約(BOM/CRLF・
  PE/ELF sniff・git index mode・hooks.json wiring)のような
  security-sensitive な検証の移植リスクを product 上の利益なしに
  負う。

## More Information

* 上流 inventory: `docs/discovery/node-dev-tooling-inventory-issue-182.md`
  (全使用箇所・call graph・options 比較表)。
* spec: `docs/specs/node-dev-tooling-scope-issue-182.md`(issue #182
  の done 定義と受入条件)。
* 先行判断: `docs/plans/phase-6-remove-typescript-node-pi.md`
  §Deferred 判断の補足 — Node を repo tooling の実行基盤として残す
  こと、および security-sensitive 検証ロジックの移植が利益に
  見合わないという根拠(本 ADR は inventory をもって同結論を
  正式な決定として refine する)。
* 関連: `benchmark-harness-placement-and-accounting.md`(benchmark
  を plain Node runner とした配置決定)・
  `canonical-setup-check-entrypoints-as-mise-toml-tasks.md`
  (canonical check contract)・
  `distribute-cli-via-cargo-source-install-and-thin-path-launchers.md`
  (product 配布の node-free 化)。

## Implementation follow-up (2026-10-26)

Issue #188 completed follow-up F1: the three Rust test sites identified in the
original inventory no longer depend on ambient Node. The dedicated
`node_task_resolves_via_path` test preserves bare-name PATH resolution, grant
pinning, and spawn with a controlled Rust fixture executable; differential
fixtures use the explicit Rust child mode while preserving frozen JSON and
golden output; the workflow process test executes its fixture child without a
silent skip. The three tests passed under an explicit Node-free WSL PATH.

This implementation does not change the accepted decision. Node remains
pinned for repository tooling and upstream deployed doc-suite scripts. The
separate upstream T1 and glob-codegen T3 work remains outstanding, so this note
does not claim that the broader Node removal gate is complete.

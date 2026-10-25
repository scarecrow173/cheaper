---
id: "SPEC-034LRjQlZwIik0Fp58p5hV"
type: "spec"
status: "approved"
title: "Node dev-tooling inventory and toolchain decision (issue #182)"
created: "2026-10-25"
updated: "2026-10-25"
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
  implements: []
  implemented-by:
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
  derived-by:
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    - "../test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  verifies: []
  verified-by:
    - "../test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "../plans/phase-6-remove-typescript-node-pi.md"
    - "docs/guides/release.md"
    - "CONTRIBUTING.md"
  defers: []
  deferred-by: []
---

# Node dev-tooling inventory and toolchain decision (issue #182)

## What Should Be Built

issue #182 の成果物として、次の 3 点を canonical document として残す。

1. **dev 環境の Node.js 使用箇所の全量 inventory**
   (`docs/discovery/node-dev-tooling-inventory-issue-182.md`)。
   各 entry は 場所・用途・呼出し元(どの task/CI step/hook/agent flow が
   起動するか)・required/optional 区分・移行性(工数と移行先の形:
   xtask crate / cargo test / mise run 行 / upstream のみ可)・
   今日検出している失敗(保存すべき契約)を持つ。
2. **契約保存表** — 移行・維持のいずれを選んでも失ってはならない
   検証契約の一覧: `check-version`(version 宣言 drift)・
   `set-version`(bump 一括性)・`check-lockfile`(committed
   Cargo.lock の整合+drift)・`check-apm-payload`(slim payload
   byte/wiring 契約)・`check-doc-ids`(旧連番 ID 再混入)・
   `node --test` 両スイート・`cargo test` ambient node 依存の
   各 fail 条件。
3. **ADR による toolchain 決定** —
   `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`。
   (A) Node 維持 / (B) benchmarks のみ Rust 化 / (C) repo tooling
   全て Rust・native 化、の証拠ベース選択と revisit trigger。

## Why Now

Phase 6(TS 撤去)で「Node は repository tooling の実行基盤として残す」が
補足判断として記録されたが、全使用箇所の棚卸しと契約保存表を伴う
正式な再検討は未実施だった。mise.toml の `node` pin を維持・撤去する
判断には inventory が前提となるため、issue #182 として証拠を固定する。

## Users and Value

- maintainer: toolchain の責務境界と removal gate を明文化された
  判断材料として使う。
- Coding Agent: `mise run check`・`cargo test`・doc workflow のどこが
  node を必要とするかを inventory から直接確認できる(ambient 結合の
  見落とし防止)。
- 将来の移行実施者: revisit trigger が発火した際に、契約保存表と
  移行性評価をそのまま実装 input として使える。

## Scope

### In Scope

- dev 環境の全 Node 使用箇所の棚卸し(mise tool 宣言・check
  entrypoint・手動 script・benchmark harness・glob table codegen・
  `cargo test` の ambient 依存・APM deploy 済み doc-suite `.js`/`.cjs`)。
- product path が node-free であることの実証的確認(launcher・
  build.rs・src spawn・配布 payload・CI)。
- (A)/(B)/(C) の選択と、その場合の follow-up issue 候補の列挙。
- `mise.toml` `node` 除去 gate の明示(required entrypoint が全て
  node-free になったとき)。

### Out of Scope

- いずれの移行の**実装そのもの**(本 spec は調査・決定の完了を定義する;
  実装は採用された option の follow-up issue に委ねる)。
- product runtime への変更(node は product path に存在しないことを
  確認する対象であり、変更対象ではない)。
- upstream が所有する deploy 済み script(`scarecrow173/apm-packages`
  の doc-driven-dev が 41 `.js`、third-party `obra/superpowers` が
  brainstorming の `helper.js` + `server.cjs`)の移行(本 repo の
  変更境界外 — inventory で upstream 所有として分類するのみ)。
- `mise.toml` の `node` pin 除去(benchmarks・doc-suite が残る現状で
  提案しない — issue の gate 条件)。

## Requirements

- inventory は「全ての」使用箇所を対象とし、node 非依存と確認した
  隣接領域(CI・prek・biome・launcher・build.rs・packages
  payload・product src)も証拠付きで列挙すること。
- 各 check/test について「今日何の失敗を検出するか」を保存契約として
  明記し、移行時に失われる検証が無いことを確認可能にすること。
- 決定は証拠ベースであること — `.mjs` harness が既に稼働している
  こと、upstream 所有の `.js`/`.cjs` が本 repo の境界外であること、
  CI cold-start の Rust compile 時間などを考慮に入れること。
- `mise.toml` から `node` を外す提案は、required entrypoint が全て
  node-free になった場合に限る。

## Acceptance Criteria

- [ ] `docs/discovery/` に inventory document が存在し、上記
  In Scope の全カテゴリ(A: toolchain 宣言 / B: check entrypoint /
  C: 手動 tooling / D: Rust test ambient 依存 / E: APM-deployed
  doc-suite / F: 非依存確認済み領域)を網羅した entry 表を持つ。
- [ ] 各 inventory entry が 場所・用途・呼出し元・required/optional・
  移行性・保存契約 を含む。
- [ ] 契約保存表で check-version / set-version / check-lockfile /
  check-apm-payload / check-doc-ids / `node --test` スイート /
  `cargo test` ambient node case の各 fail 条件が列挙されている。
- [ ] `docs/adr/` に Considered Options ≥2 の ADR が存在し、
  (A)/(B)/(C) の比較と証拠ベースの推奨・revisit trigger を含む。
- [ ] 決定が migration 系 option を選ぶ場合は follow-up
  implementation issue(対象・依存順序・受入条件)が列挙されている;
  維持を選ぶ場合は revisit trigger が明記されている。
- [ ] `mise.toml` の `node` 除去が提案されていないこと(benchmarks・
  doc-suite script が node を必要とする間は gate 未達)。
- [ ] 3 document がそれぞれの dir の README index に登録され、
  `node .agents/skills/doc-status/scripts/audit_docs.js --type
  discovery --type spec --type adr` で findings が無い。
- [ ] product path への node 再導入がない(slim payload check・
  src の node spawn 不存在が変わらない)。

## Deferred Design Concerns

- repo 所有 `.mjs` の Rust 化(scripts → xtask・test → cargo
  integration test・benchmarks → Rust harness・ambient node
  fixture 化): Option A 採用時は即時対応不要。re-engagement trigger
  は ADR の revisit conditions(upstream doc-suite の node-free 化 /
  benchmark harness の大改修機会 / node pin 維持コストの顕在化)。
- `cargo test` の ambient `node` 結合(proc_process・process.json・
  workflow process.rs)を fixture binary へ repoint する衛生改善は
  決定と独立して実施可能 — 小規模 follow-up issue 候補。

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/182>
- 上流 discovery:
  `docs/discovery/node-dev-tooling-inventory-issue-182.md`
  (inventory・call graph・options 比較)。
- 決定 ADR: `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`。
- 先行判断: `docs/plans/phase-6-remove-typescript-node-pi.md`
  (Node を repo tooling に残す補足判断)・
  `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  (product の node 撤去)・
  `docs/adr/benchmark-harness-placement-and-accounting.md`
  (benchmark harness を plain Node runner とした決定)。

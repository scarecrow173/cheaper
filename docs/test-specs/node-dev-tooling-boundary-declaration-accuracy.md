---
id: "TSPEC-034LSvGcYkAp0ImKqx2Alh"
type: "test-spec"
status: "draft"
title: "Node dev-tooling boundary declaration accuracy (issue #182)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/node-dev-tooling-scope-issue-182.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  derived-by: []
  verifies:
    - "docs/specs/node-dev-tooling-scope-issue-182.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  verified-by:
    - "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
    - "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
  references:
    - "docs/discovery/node-dev-tooling-inventory-issue-182.md"
  defers: []
  deferred-by: []
---

# Node dev-tooling boundary declaration accuracy (issue #182)

## Purpose

issue #182 の Option A(Node.js を pinned dev-tooling runtime として維持)
決定を operationalize する変更が、「宣言(doc/comment)と実態の一致」を
壊さないことを pin する検証契約。Option A は実装変更を伴わない
decision-as-record だが、AGENTS.md の Node 段落・script header comment・
follow-up 起票記録が実際の Node entry 集合・pipeline 順序・import 関係・
ordering gate とずれたまま残ると、維持決定の根拠自体が腐る。

本 test spec は将来 AGENTS.md の Node 段落・`scripts/check-lockfile.mjs`・
`scripts/lib/release-artifacts.mjs` の header comment・F-plan 追跡を
書き換える・削除する変更が、宣言の正確性を維持しているかを判定するための
契約である。廃止・supersede の検討タイミングは、boundary contract design
(DESIGN-BSEuqAM7QQUmRoTUGOMxbH)が supersede されたとき、または宣言の
canonical 表現が別 doc/tooling 検査へ移ったとき。

## Feature

repo の Node 結合に関する宣言 — AGENTS.md の段落・check script の header
comment・deferred follow-up の起票/非起票記録 — は、実際の Node entry
point 集合・`mise run check` pipeline 順序・script 間 import 関係・
design §5 の ordering 制約と常に一致する。

## Rules

- AGENTS.md の Node 段落は repo 所有の Node entry カテゴリを全て列挙する
  — `scripts/`・`benchmarks/`・`crates/cheaper-tools/gen-glob-tables.cjs`
  (glob table codegen の Node regex oracle)・deploy 済み doc-suite
  skills(`.agents/skills/`・`.claude/skills/`、upstream 所有)— の 4
  系統を欠いてはならない。あわせて product・distribution・hook 実行経路が
  Node 非依存であること、および `mise.toml` `node` 除去が removal gate
  (required entrypoint 全 node-free + design §1 維持 entry の移行完了)
  に縛られることを述べる。
- AGENTS.md の Node 段落は canonical contract への参照を持つ —
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`(領域別
  policy・非退行契約・revisit trigger)と
  `docs/discovery/node-dev-tooling-inventory-issue-182.md`(全量
  inventory)への pointer を含む。
- `scripts/check-lockfile.mjs` の header comment は自身の pipeline 位置を
  実態どおりに述べる — 「(check pipeline の)最後尾」を名乗ってはならない
  (最後尾は `check-apm-payload.mjs`、check-lockfile は mise.toml:74 の
  10 step 中 9 番目・payload check 直前)。drift 検出の根拠(先行
  build/test による lockfile 再生成の捕捉)の記述は維持する。
- `scripts/lib/release-artifacts.mjs` の header comment は実 importer のみ
  を名指す — `check-version.mjs`・`set-version.mjs` の 2 file。
  `check-apm-payload.mjs` は同 lib を import しない(node builtin のみを
  import)ため名指し対象外。
- 本 operationalize 変更は doc/comment のみ — `mise run check` pipeline・
  `mise.toml` node pin・product runtime・test 意味論を変えない。
  `git diff` に実行 code の差分を持たない。
- F-plan 追跡は ordering 制約どおり — 起票されるのは F1・F5 のみ
  (F6 は本 PR で消費)。gate 未達の候補(F2: issue #183 前提、
  F3: trigger T3、F4: trigger T1)は起票せず gate 条件と共に記録される。
- 起票される follow-up issue は `## Scope`・`## Acceptance Criteria`・
  `## Dependencies`・`## 参照` の 4 節を持ち、filed issue body は
  task doc 収録の body と verbatim で一致する(再解釈しない —
  `gh issue view <n> --jq .body` で diff 可能な機械的照合)。
- product path の node-free は本変更の前後で不変 — `crates/*/src` に
  node spawn 相当箇所を持ち込まず、`packages/*/.apm/hooks/` に
  `.js`/`.mjs` payload を持ち込まない。

## Examples

- `grep -n "最後尾" scripts/check-lockfile.mjs` が 0 件を返し、header が
  `check-apm-payload` 直前(9 番目/10)という実位置を述べている —
  mise.toml `[tasks.check]` run 順(:74 lockfile → :75 payload)と整合。
- `grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs` が 0 件
  — 実 importer(`grep -rn "release-artifacts" scripts/check-version.mjs
  scripts/set-version.mjs` で確認)のみが header に残る。
- `grep -n "gen-glob-tables.cjs" AGENTS.md` が match し、同段落に design・
  discovery への link と「新規 ambient 結合は原則不可」
  「node 行除去不可」の policy 要約がある。
- `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1"
  in:title --state all --json number,title` / `"Node dev-tooling F5"
  in:title` で F1・F5 の title を持つ issue が返り、F2-F4 の
  issue は存在しない(追跡表で gated と記録)。
- `mise run check` が exits 0 — biome → fmt → clippy → check-version →
  check-doc-ids → cargo test → `node --test` ×2 → check-lockfile →
  check-apm-payload の現行 10 step が変更なく pass。
- `.agents/skills/cheaper-usage/SKILL.md:43` の旧
  `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` 参照は
  upstream lock-pinned の stale deploy(`apm install` 再実行でも解消
  しないことを 2026-10-25 実測)として記録され、repo の実 node 依存と
  しては計上されない。

## Guarantees

- repo の Node 結合に関する宣言(AGENTS.md 段落・script header
  comment・F-plan 追跡表)は、実態(entry 集合・pipeline 順・import
  関係・ordering gate)と一致している。
- 本変更は doc/comment のみであり、check pipeline・toolchain・product
  runtime の実行挙動は変更前後で同一(`mise run check` が不変 pass
  することで確認)。
- 起票された F1・F5 issue は design §5 の scope・acceptance sketch・
  ordering を逸脱しない内容を持つ。
- deferred 候補(F2-F4)は gate 条件付きで追跡可能であり、未達 gate の
  issue が混在しない。

## Non-goals

- 各 Node entry が検出する fail 契約そのものの保証 — それは design §1
  非退行契約と既存の check/test(`check-version`・`check-doc-ids`・
  `check-lockfile`・`check-apm-payload`・`node --test` 両 suite)が担う。
  本 spec は宣言の正確さのみを対象とする。
- upstream deploy 物(`.agents/skills/`・`.claude/skills/`)の内容検証 —
  upstream 所有であり観測記録の対象であって enforcement 対象ではない。
- F2-F4 の実施可否判断・upstream doc-suite の node-free 化進捗・
  `check-apm-payload.mjs` の走査範囲(skills payload は design §2.3 の
  既知 gap で human review 依存)・`biome.jsonc` の `*.cjs` coverage gap。
- `cargo test` の ambient `node` 結合 3 site の存在自体 — それは現行
  contract(§3 required/skip)で意図的に維持されており、repoint は
  F1 issue の仕事。

## Risk

- 宣言と実態が drift したまま残ると(pipeline 順変更後に comment が
  古い等)、inventory・ADR・design の証拠が腐り、次回の toolchain
  再検討(revisit trigger 発火時)の入力を誤らせる。
- AGENTS.md の境界宣言が不完全だと、contributor が product path・
  `packages/**`・CI へ node 結合を持ち込んだり、crate 非 src の
  `.cjs` codegen や `cargo test` ambient 結合を見落としたりする —
  「Node は repo tooling に限定」という 2-runtime 境界の認知コストが
  実害になる。
- follow-up の起票/非起票記録が design §5 の ordering とずれると、
  gated 候補の早すぎる起票(removal payoff の無い rewrite 提案)や
  独立候補の起票漏れで決定の運用が形骸化する。

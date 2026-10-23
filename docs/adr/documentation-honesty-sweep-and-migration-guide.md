---
id: "ADR-034L6ihibvbjmSnKmSffWH"
type: "adr"
status: "proposed"
title: "documentation honesty sweep + migration guide — claim ≤ 実強制・refine-only 訂正方式"
created: "2026-10-23"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "./enforce-declared-contracts-over-content-audit.md"
    - "../designs/permission-model-responsibility-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "./enforce-declared-contracts-over-content-audit.md"
    - "./exec-contract-content-audit-removal.md"
    - "./opt-in-exec-deny-patterns.md"
    - "./unified-exec-grant-model.md"
    - "./route-grant-set-projection.md"
    - "./substitute-exec-route-mode.md"
    - "./trusted-route-flag-issuer-redesign.md"
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../tasks/file-issue-docs-honesty-and-migration-guide.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# documentation honesty sweep + migration guide — claim ≤ 実強制・refine-only 訂正方式

## Context and Problem Statement

権限モデル再設計(#151)の最終 slice(#169)は、docs の
「boundary/sandbox/protection」表現を実強制範囲へ整合させる
文書正直性 sweep と、#163–#168 の破壊的変更を告知する
migration guide を要求する。実測 inventory
(DISC-034L67agpaqLudLiaNI822)で、実装がすでに変わった後も
以下の誇張・stale 表現が残っていることが確定した:

- `docs/guides/command.md:44`(native magic/shim deny 残留)・
  `:133`(call-time metachar 拒否・error code)、
  `docs/guides/managed-process.md:23`(宣言 argv 内容審査)・
  `:27`(native PE/ELF/Mach-O 限定)が現行実装と矛盾
- refine 対象 16 doc(ADR 7 件・spec/design 9 件、親 design
  §Supersede/Refine 一覧)に mandatory 内容審査・
  mandatory deny list・workspace containment を前提にした
  記述が残る
- 旧 builtin executable deny list(#164 で opt-in 降格)を
  「保護」と読ませ得る命名が残る

同時に、訂正の**方式**自体を固定する必要がある:履歴 doc
(ADR)と living contract(spec・guide)で訂正の構えが異なり、
cross-chain の `refines`/`refined-by` front-matter edge は
commit 9d76d40 で修正済みの graph pollution(route_graph が
focus-required で全 chain block)を再発させる。また migration
guide の置き場所には既存規約が複数あり得る(guides 新設・
CHANGELOG・各 guide 分散)。これらを一括で決める。

## Decision Drivers

* **claim ≤ 実強制**:文書は実装が強制する範囲を超える保護を
  主張しない(親 spec R5・親 ADR の責務モデルに由来)
* **履歴の非改竄**:ADR は「決定時点の記録」であり、後の変更で
  本文を黙って書き換えると decision log としての価値を失う
* **living contract の単一の真実**:spec の要件本文と user
  guide は現在の真実を読者へ届ける責務があり、旧記述の残留は
  誤設定の温床(#166–#168 で route spec R1/R12/R15/R34 を
  in-place 訂正した先例あり)
* **graph lineage の保護**:`refines`/`refined-by` は chain 内の
  lineage relation であり、異なる chain(別 issue 系統)への
  edge は route_graph の cycle/pollution を引き起こす
  (ADR-034J8eZHNPqARi25HLFOL6 :709-710 の明記規約・9d76d40
  復旧済み)
* **発見可能性**:migration guide は upgrade する利用者が
  「壊れた症状」から修復手順へ辿る入口であり、単一の
  canonical な置き場所が必要
* **比例性**:文書のみの差分。コード側の不整合は別 issue 化
  (本 sweep では文書の正直性を優先する規約 — discovery
  assumptions)

## Considered Options

**正直性文言の置き場所:**

* O1: migration guide に集約し、各 guide は link するだけ
* O2: canonical 文言を本 ADR で固定し、guide・migration
  guide・refine 注記がそれぞれ自前の形で反映
* O3: 共通 include/snippet を導入

**16 対象 doc の訂正方式:**

* O4: front-matter `refines`/`refined-by` edge で graph 上の
  訂正関係を張る
* O5: front-matter は `related`/`references` backlink に留め、
  訂正意図は対象 doc 内の注記本文 + spec の対象一覧表で表す
* O6: 対象 doc を supersede して新 doc に責務を移す

**migration guide の置き場所:**

* O7: `docs/guides/migration.md` 新設 + `docs/README.md` index
* O8: CHANGELOG.md 新設
* O9: 各 guide への分散記載

## Decision Outcome

Chosen option: **O2 + O5 + O7**。

### D1 — canonical disclaimer(本 ADR が文言の正本)

以下の文言形を canonical とし、SPEC-034L6i3XmfCaWRHAiw8TTT
R1/R2 の配置先(migration guide 前提節・`command.md`
trust/sandbox 限界節・`managed-process.md` caveat 節)がこれを
反映する:

- 「cheaper は OS sandbox ではない」
- 「canonical file tool(write/edit/ls/read/find/grep)の
  workspace boundary(E1/E2)・`filesystem.denyPaths`・
  worker `writeScope` は cheaper 所有の tool surface で強制
  される。spawn した子プロセスの fs 副作用は、OS sandbox
  無しでは cheaper が強制・完全観測できない」
- 「cheaper の spawn は shell を介さず argv を直接渡す(shell
  metachar は executable の literal argv として渡り、cheaper
  が解釈・拒否しない)。POSIX で shebang を持たない +X file の
  実行が ENOEXEC となった際に libc が `/bin/sh` で retry する
  のは OS/libc 既定動作(glibc `execvpe` 系。musl は retry
  しない残留差分あり)であり cheaper の保証対象外」

### D2 — deny profile の命名規約

`security.execDenyPatterns` は「deterministic executable
deny-list」と呼ぶ(opt-in・既定 `[]`・executable basename
stem への anchored case-insensitive 完全一致・3 適用点)。
「保護」「sandbox」「containment」「防御」とは呼ばない。
列挙を免れる経路(改名・代替 binary・plugin/extension 機構)
が残るため、exec grant は shell 相当の信頼として扱う既存記述
(`command.md`:63/:148)を維持する。

### D3 — refine-only 訂正方式(supersede しない)

16 対象 doc すべてに refine 反映を行い、supersede edge・
superseded status は一切張らない。訂正の構えは doc 種別で
分ける:

- **ADR(決定記録)**: 本文は当時の決定として保持し、
  「Refine 注記」(issue #169・変更点・現行真実・governing
  doc への link)を追記する
- **spec(living contract)**: 要件本文を in-place で現在の
  真実へ訂正してよい(#166–#168 の route spec 先例)。注記を
  併置するかは各 doc の残存差分の大きさで判断
- **design**: ADR と同じく注記追記を基本とする
- **guide・skill**: 現在の真実へ in-place 訂正(利用者向けの
  単一の真実)

front-matter では cross-chain の `refines`/`refined-by` を
**張らない**。意味 link は対象 doc の `related`/`references`
への backlink(本 ADR・SPEC-034L6i3XmfCaWRHAiw8TTT・
migration guide への参照)で表し、per-doc の refine 意図は
SPEC-034L6i3XmfCaWRHAiw8TTT R5 の対象一覧表と対象 doc 内の
注記本文に集約する。

### D4 — migration guide の置き場所

`docs/guides/migration.md` を新設する(front matter 無しの
guide 規約 — `release.md`・`distribution.md` と同じ形)。構成は
issue 別(#163–#168)の「変更内容 → 利用者影響 → 修復/移行
手順」で、D1 の canonical disclaimer を冒頭の前提節とする。
`docs/README.md` User guides リストへ index row を追加する。

### Consequences

* Good, because canonical 文言が 1 箇所(本 ADR)に固定され、
  sweep 適用者と今後の文書変更が同じ表現基準を使える
* Good, because ADR の履歴非改竄と spec/guide の単一の真実を
  両立し、9d76d40 で復旧した graph lineage を再び汚さない
* Good, because migration guide が user guide 規約の中に単一
  入口として置かれ、issue 別構成で症状から修復手順へ辿れる
* Bad, because 16 doc への注記反映は文量が多く、各注記の
  品質(どこまで本文を残しどこを訂正するか)に実施者の
  判断が入る — spec R5 の訂正方向一覧で判断幅を縮める
* Bad, because 一覧外 doc の stale 表現は本 slice では捕捉
  しきれない(発見次第の軽微注記または継続 sweep 対象)

### Confirmation

* `node scripts/check-doc-ids.mjs` pass(README index 行の
  status/title 整合を含む手動確認)
* `docs/guides/migration.md` 存在 + `docs/README.md` index row
* 16 対象 doc それぞれに refine 注記または in-place 訂正 +
  `related`/`references` backlink があり、`refines`/
  `refined-by`/`supersedes`/`superseded-by` が新規に張られて
  いないこと(grep で確認)
* stale 4 箇所(command.md×2・managed-process.md×2)の訂正

## Implementation Plan

* Affected paths: `docs/guides/`(migration.md 新設・command・
  managed-process・workers 確認)、`docs/adr/`(7 件注記)、
  `docs/specs/`(5 件)、`docs/designs/`(4 件)、
  `packages/apm/.apm/skills/`(確認・軽微訂正)、
  `docs/README.md`(User guides index)
* Patterns to follow: D3 の doc 種別別訂正構え・route spec
  R1/R12/R15/R34 の in-place 訂正先例・9d76d40 の `related`
  化前例・guide の front matter 無し規約
* Patterns to avoid: cross-chain `refines`/`refined-by`・
  supersede による責務移譲・deny profile を「保護」と呼ぶ
  表現・「workspace に閉じ込める」の無条件化
* Dependencies or configuration: なし(文書のみ)

## Verification

* [ ] `node scripts/check-doc-ids.mjs` pass
* [ ] 16 対象 doc の grep で mandatory 内容審査・mandatory deny・
      無条件 containment の残存がない(または注記で補正済み)
* [ ] `migration.md` に #163–#168 の全収録項(spec R7)がある
* [ ] `docs/README.md` User guides に `migration.md` の行がある
* [ ] 差分が文書のみ(コード・fixture・test 変更なし)

## Pros and Cons of the Options

### O1(migration guide に文言集約)

* Good, because 修正点が 1 箇所で済む
* Bad, because guide を読む利用者が canonical 文言に辿る前に
  誤読する余地が残り、issue AC1 の「各所定節に明記」を
  満たしにくい

### O2(ADR で文言を固定・各所定節が反映)— 採用

* Good, because 正本が 1 つで、配置先は利用者文脈に即した
  形で反映できる
* Bad, because ADR 文言と各所の表現に微差が生じ得る(許容 —
  canonical は意味の等価性)

### O3(include/snippet 導入)

* Good, because 完全な単一ソース
* Bad, because docs build 機構がなく include を解決できない
  (YAGNI)

### O4(`refines`/`refined-by` edge)— 不採用

* Good, because graph 上で追跡可能
* Bad, because cross-chain lineage で route_graph pollution を
  再発させる(9d76d40 の復旧事案を反復)

### O5(`related`/`references` backlink + 本文注記)— 採用

* Good, because 意味 link を保ちつつ lineage を汚さない
* Bad, because graph 上の「訂正」関係としては弱い(意図は
  本文と spec 一覧で担保)

### O6(supersede で責務移譲)— 不採用

* Good, because 読者が新 doc だけ見ればよい
* Bad, because 親 design が「全件 refine・supersede なし」と
  確定しており、16 doc の superseded status 変更は履歴を
  断ち切る

### O7(`docs/guides/migration.md`)— 採用

* Good, because user guide 規約・単一入口・将来の追記先
* Bad, because 初の migration guide で前例なし

### O8(CHANGELOG.md)— 不採用

* Good, because release 変更追跡に一般的
* Bad, because repo に CHANGELOG 規約がなく、症状→修復の
  guide 形式より entry 列挙になりがち

### O9(各 guide へ分散)— 不採用

* Good, because tool guide 内で完結
* Bad, because 破壊的変更一覧の単一入口が無く issue AC5
  「一覧列挙」を満たしにくい・重複 drift

## More Information

* issue #169: https://github.com/scarecrow173/cheaper/issues/169
* 実測 inventory・掃き出し表現・migration guide 収録項の正本:
  `../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md`
  (DISC-034L67agpaqLudLiaNI822)
* 適用要件の正本:
  `../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT)
* 16 対象一覧の authoritative input:
  `../designs/permission-model-responsibility-redesign.md`
  §Supersede/Refine 一覧(DESIGN-034KuDkWOmNXiBddEQ4XWK)
* 責務モデルの親 ADR:
  `./enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8)
* graph pollution の復旧 commit: `9d76d40`、cross-chain
  `refines` 禁止の明記:
  `./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`:709-710

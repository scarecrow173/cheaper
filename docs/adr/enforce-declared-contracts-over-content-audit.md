---
id: "ADR-034KuqZelggTIb8Z8K9QB8"
type: "adr"
status: "accepted"
title: "宣言された実行契約の強制を責務とし内容審査を分離する"
created: "2026-10-22"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/151"
    - "../discovery/permission-model-responsibility-redesign.md"
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
    - "../discovery/permission-model-responsibility-redesign.md"
    - "./command-grant-trust-boundary-and-managed-process-reuse.md"
    - "./worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "./workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "./defer-non-exec-routing-to-permission-model-redesign.md"
    - "./opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "./route-grant-set-projection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/permission-model-responsibility-redesign.md"
  derived-by:
    - "../plans/file-permission-model-redesign-implementation-issues.md"
    - "../test-specs/permission-model-redesign-issue-decomposition.md"
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

# 宣言された実行契約の強制を責務とし内容審査を分離する

## Context and Problem Statement

cheaper の実行制限は現在、4 種の異なる性質の制約が同一レイヤに混在している
(inventory は `../discovery/permission-model-responsibility-redesign.md`):

1. **実行契約** — shell 非介在 argv・env_clear・stdin null・timeout/kill・
   fs boundary・writeScope 等、「宣言した通りに・契約内で動く」正しさ。
2. **operator-choosable 権限** — tools.allow・grants/tasks・trusted flag 等、
   宣言によって allow/deny を選ぶ仕組み。
3. **optional hardening** — interactive command 検出等の補助的防御。
4. **内容審査** — executable deny list(python/node/docker/ssh/tar/find…)・
   shell/shim 判定・script 拡張子判定・native magic-byte 検査・
   workspace-resident 判定・direct-argv メタ文字 reject 等、
   「宣言された exec の内容を cheaper が審査して reject」する制限。

issue #151 の責務分担では「何を走らせるか」の意味的正しさは workflow
作者/operator の責務であり、cheaper の責務は宣言された制約の強制と
実行契約の正しさに限る。しかし現状、(4) の内容審査が mandatory 層に
存在し、正当な workflow を拒否する false positive を生む一方、
bypass は構造的に無限(改名・別ツール・子プロセス経由)で containment
としては成立しない。「deny list が保護」「workspace に閉じ込める」
という表現と実強制の乖離も確認された。

## Decision Drivers

* 責務分担との整合 — cheaper は宣言強制に専念し、内容審査は作者/operator に返す
* 正直性 — 「保護する」と言うのは実際に強制できる範囲のみ
* false positive の解消 — 正当な `ssh`/`tar`/script/workspace binary を通す
* bypass 構造の認識 — 改名・代替ツール・子プロセスで抜ける審査は
  「mandatory 防御」として維持する意味が薄い
* routing ≠ authorization の維持 — route の viability が内容審査を
  呼ぶ構造は整理する
* 信頼境界の保持 — trusted 層 vs project 層の authoring 権は機構として残す
* 後方互換と移行コスト — 破壊的変更を明示的に扱う

## Considered Options

* Option A: 現状維持 — 内容審査を mandatory のまま保持・拡張
* Option B: 宣言契約モデル — 内容審査を mandatory 層から除去し、
  cheaper は宣言強制のみ担う(opt-in profile への降格を含む)
* Option C: tiered trust zone — 全制限を configurable profile 化し、
  層別に hardening 強度を選択可能にする
* Option D: capability-grant 統合 — command.grants/process.tasks を
  単一 exec grant モデルへ統合(直交する構造改善)

## Decision Outcome

Chosen option: "**Option B(宣言契約モデル)を基底とし、内容審査の
opt-in profile 化を Option C の要素として許容する複合**"。

理由:

- Option A は issue 前提(workflow 内容は作者責務)と矛盾し、
  false positive と claim/実強制の乖離を維持する。
- Option B は責務分担と一致し、cheaper の claim を実強制範囲に
  揃える。grant は「operator が宣言した実行許可」であり、
  executable の中身を cheaper が審査しない。
- ただし「既定で既知の危険名を弾く」既定値としての価値を
  完全に捨てるかは spec の判断に委ね、opt-in profile
  (`security.execDenyPatterns` 等)として残す選択肢を残す
  (Option C の局所適用)。
- Option D の grant/task 統合は本 ADR の責務決定と直交する
  構造改善であり、採否は spec(R7)で別途決定する。

この ADR が決めるのは **責務モデル** であり、deny list を残すか
profile 化するか等の実装形態は spec・後続 issue に委譲する。

### Consequences

* Good, because — claim≦実強制となり文書が正直になる。false positive
  が解消し、宣言した exec が通る。責務境界が明確になり今後の
  制限追加の判断基準ができる。routing≠authorization が維持される。
* Bad, because — 「既定で危険を弾く」網が mandatory ではなくなる
  (但し元々完全な防御ではなかった)。grant 作者の判断に依存する度合いが
  上がる。deny list 削除は破壊的変更であり、既存 user policy に
  依存している利用者への移行コストがある。
* Neutral — trusted 層の機構(A3 ProjectPartial)は残るため、
  「誰でも grant を書ける」わけではない。内容審査の除去は
  「declaration があれば通る」のであり「誰でも宣言できる」ではない。

### Confirmation

- spec `../specs/permission-model-responsibility-redesign.md` の
  AC(R1-R9)で検証。特に「内容審査が mandatory 層に残らない」
  「opt-in profile の schema 明記」「既存契約テストの回帰なし」。
- 既存 ADR の refine 対象:
  - `command-grant-trust-boundary-and-managed-process-reuse.md` —
    trusted-layer grant 機構は維持。deny list・workspace-resident・
    メタ文字等の mandatory 審査部分を refine。
  - `worker-definition-runtime-policy-runtime-invariants-capability.md` —
    3 層モデルは維持。invariants に置く制限の中身を再配置する
    形で refine(実行契約は invariants、内容審査は除く)。
  - `workspace-filesystem-boundary-cheaper-sandboxed-tool.md` —
    canonical tool の boundary 強制は維持。「子プロセスには及ばない」
    ことを明記する形で refine。
- 非 exec routing の deferred sketch
  (`defer-non-exec-routing-to-permission-model-redesign.md`)は
  本決定の下で採否を spec 側で判断する。

## Implementation Plan

* Affected paths:
  `crates/cheaper-core/src/policy/command.rs`(deny list・shim 判定)、
  `crates/cheaper-core/src/policy/process.rs`(script ext・metachar)、
  `crates/cheaper-tools/src/tools/executable.rs`(native/magic/bare-node)、
  `crates/cheaper-tools/src/tools/command/mod.rs`(workspace-resident 2 段検査・
  spawn 再検証)、route viability の審査依存箇所、関連 docs
* Patterns to follow: 制限を「実行契約」「権限」「optional hardening」
  「内容審査(除去対象)」でタグ付け・層分けする。契約側の既存
  lifecycle/fs/manifest 実装は維持。
* Patterns to avoid: 新たな mandatory 内容審査の追加。「保護」と
  呼べない制限を保護と表現する文書。deny list の単純な拡張で
  誤魔化すこと。
* Dependencies or configuration: opt-in profile を残す場合は
  policy schema に `security`(仮)節の追加が必要。破壊的変更は
  migration/changelog で告知。

## Verification

* [ ] discovery の 4 分類で全制限が整理され、内容審査項目が
      mandatory 層から除去または opt-in 化されていること
* [ ] `ssh`/`tar`/`.sh`/workspace binary を宣言した grant が
      (opt-in deny 未指定で) 拒否されない統合テスト
* [ ] pin・env_clear・stdin null・timeout/kill・fs boundary・
      writeScope の既存契約テストが回帰なく pass すること
* [ ] viability が authorization(grant/allowlist)を呼ぶが
      内容審査を呼ばない構造であること
* [ ] 文書上の「sandbox/boundary/protection」表現と実強制の
      整合(guides・refine 対象 ADR)
* [ ] 破壊的変更一覧(deny list 削除等)の列挙

## Pros and Cons of the Options

### Option A — 現状維持 hardening

* Good, because 保守的で変更不要。「危険ツールを既定で弾く」直感に合致。
* Bad, because 内容審査は workflow 作者責務への踏み込み。false positive
  が正当 workflow を拒否。bypass 無限で containment 不成立。
  「保護」claim と実強制が乖離する。

### Option B — 宣言契約モデル

* Good, because 責務分担と一致。claim=実強制で正直。false positive 解消。
  制限追加時の判断基準(宣言強制か内容審査か)ができる。
* Bad, because grant 作者の能力に依存。「既定で既知の危険名を弾く」
  網が mandatory でなくなる。deny list 削除は破壊的変更。

### Option C — tiered trust zone / profile

* Good, because 後方互換。operator が hardening 強度を選べる。
* Bad, because knob ごとに semantics+test+doc が必要で複雑。
  「単純に削る」より総コストが高い。全制限の profile 化は過剰。
* 局所適用としては有効(deny list の opt-in profile 化のみ採用余地)。

### Option D — capability-grant 統合

* Good, because command/process の二重実装を解消。policy surface 半減。
* Bad, because 大規模 refactor で migration 必要。責務決定とは直交。
* 本 ADR では採否を保留し spec R7 に委譲する(別 issue 化も可)。

## More Information

- 起点: issue #151(責務分担の定義・棚卸し範囲・4 分類)
- inventory・選択肢比較・open questions:
  `../discovery/permission-model-responsibility-redesign.md`
- product intent・AC: `../specs/permission-model-responsibility-redesign.md`
- deferred input: `defer-non-exec-routing-to-permission-model-redesign.md`
  (grant-set 投影・substitute・非 exec workflow の sketch)
- refine 対象 ADR は Decision Outcome §Confirmation を参照。

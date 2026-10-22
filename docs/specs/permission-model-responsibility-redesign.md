---
id: "SPEC-034KuqPkTK2b4mHsykH9A5"
type: "spec"
status: "approved"
title: "権限・Runtime Policy・Tool 実行制限の責務ベース再設計"
created: "2026-10-22"
updated: "2026-10-22"
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
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "./policy-gated-command-tool.md"
    - "./invocation-scoped-command-grants.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/permission-model-responsibility-redesign.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 権限・Runtime Policy・Tool 実行制限の責務ベース再設計

## What Should Be Built

既存の実行制限を「**宣言された実行契約の強制**」と「**内容審査**」に分離し、
cheaper が責務を持つのは前者のみであることを policy モデル・tool 実装・
route/hook・文書の全層で一貫させる再設計。

具体的な product intent:

- cheaper は **宣言された制約の強制**(tools.allow・grants/tasks・writeScope・
  fs boundary・manifest・リソース上限)と**実行契約の正しさ**
  (shell 非介在 argv・env_clear・cwd・lifecycle・exit 規約)を保証する。
- cheaper は **宣言の内容を審査しない**。executable が `python` でも
  `.sh` でも workspace 内 binary でも、grant で宣言された以上
  「何を走らせるか」の正当性は作者/operator の責任。
- 「提供できない保護」(子プロセスの OS 級 containment・deny list による
  containment)を文書で誇張しない。
- 信頼境界(trusted 層 vs project 層の authoring 権)は機構として残すが、
  その中身(何を trusted に認めるか)を再整理する。

詳細な制限棚卸しと選択肢の比較は
`../discovery/permission-model-responsibility-redesign.md` を参照。

## Why Now

- issue #151 が現行制限の棚卸しと責務ベース再分類を要求。
- `command_denied_executable_re` 等の内容審査が「既知の危険名一覧」に
  留まり、正当な workflow を拒否する false positive を生んでいる
  (例: deploy に必要な `ssh`、archive 処理の `tar`)。
- deny list は構造的に bypass 無限(改名・別ツール・子プロセス経由)であり、
  「保護」の名で維持すると claim と実強制が乖離する。
- `command.grants`/`process.tasks` の二重構造や、
  「workspace boundary が子プロセスに及ぶ」かのような文書表現など、
  責務のあいまいさが複合的に蓄積している。
- 非 exec routing の deferred sketch(ADR-034KtTzOJGqmzmBW3SwGp5)が
  本再設計の結論を input として待っている。

## Users and Value

- **workflow 作者(ユーザー・上流 agent)** — 宣言した exec が
  cheaper 都合の内容審査で拒否されない。正当な `ssh`/`tar`/script が通る。
- **operator/policy 管理者** — 「どの制約が強制でどれが任意 hardening か」が
  明示され、opt-in hardening を選べる。
- **route/hook 設計者** — routing ≠ authorization の境界が明確化し、
  非 exec route の設計余地が開く。
- **読者/監査者** — 「cheaper が何を保証し、何を保証しないか」の記述が
  実装と一致し、sandbox 的誤解を防ぐ。

## Scope

### In Scope

- 内容審査制限の再分類と扱い(削除または opt-in profile 化):
  executable deny list(A9)・shell/shim 判定(A8)・script 拡張子判定(B6)・
  native magic-byte 検査(B3)・workspace-resident 判定(B4)・
  bare `node` reject(B5)・direct-argv メタ文字 reject(A10)・
  process task 側の同種審査(F3)
- 実行契約制限の保持・明文化(B1/B2 pin、C1-C8 lifecycle・env・stdin、
  E1-E5 fs/writeScope、D 系 routing 契約)
- 信頼境界機構(A3 ProjectPartial・D4 trusted flag・D14 approvalBypass)の
  保持と「発行主体」の再定義方針
- route/hook 側で内容審査に依存している分岐(viability の unsafe 判定が
  deny list を参照する経路等)の整理
- 文書の正直性修正 — 「boundary/sandbox/protection」表現と実強制範囲の
  整合(guides・対象 ADR の refine 注記)
- 非 exec routing deferred sketch(ADR-034KtTzOJGqmzmBW3SwGp5)の
  受け入れ判断 — grant-set 投影・substitute command・非 exec workflow・
  exit mirror 変種
- command.grants/process.tasks の統合可否決定(F2 — 本 spec scope 内で
  判断するか、後続 issue へ defer するかを本 spec 自体が決める)

### Out of Scope

- OS sandbox(seatbelt/bubblewrap/seccomp 等)の実装 —
  提供しない保護として明記する側であり、本 spec で実装しない
- workflow の意味的検証(「この workflow は安全か」等の内容評価) —
  作者責務として cheaper は行わない
- deny list 以外の新規 hardening 機構の追加
- model tier・credential・provider 等 exec 制限と無関係の policy 領域
- 即時の実装 — 本 spec は intent/AC の定義であり、実装は後続 issue 群へ

## Requirements

- R1. 「宣言された実行契約の強制」と「内容審査」をコード・文書上で
  識別可能に分離すること(制限ごとの分類タグまたは整理された層)。
- R2. 内容審査クラスタ(discovery §A8/A9/A10/B3-B6/F3)を削除するか、
  opt-in の hardening profile へ降格すること。mandatory 層には残さない。
- R3. 実行契約制限(pin・env_clear・stdin null・lifecycle・fs boundary・
  writeScope・manifest・routing contract)は維持すること。
- R4. 信頼境界機構(trusted 層 / project 層の authoring 権)は維持し、
  「誰が trusted を認めるか」を再定義すること。
- R5. 「canonical tool の fs boundary」と「子プロセスの副作用は
  OS sandbox 無しでは強制不能」を文書で明確に分離すること。
- R6. routing ≠ authorization を維持すること。viability 判定が
  内容審査へ依存する経路は整理すること。
- R7. command.grants/process.tasks の統合可否を本 spec の中で
  決定すること(統合する場合は migration 方針を含む)。
- R8. 非 exec routing deferred sketch の受け入れ可否を
  本 spec で決定すること。
- R9. 変更は破壊的変更として changelog/migration へ記載できる粒度で
  識別されること(deny list 削除等)。

## Acceptance Criteria

- [ ] 全既存制限が discovery の分類表に対応する形で
      policy/tool/route 各層に反映されている(内容審査が
      mandatory 層に残っていない)
- [ ] 内容審査を opt-in profile に残す場合、その profile が
      policy schema に明示され、既定値と override 方法が文書化されている
- [ ] `python`・`ssh`・`tar`・`.sh` script・workspace 内 binary を
      宣言した grant が、(opt-in deny profile を明示しない限り)
      内容審査で拒否されないことが統合テストで確認できる
- [ ] shell-free spawn・env_clear・stdin null・timeout/kill・
      fs boundary・writeScope の既存テストが引き続き pass する
      (契約側の回帰なし)
- [ ] routing viability が grant/allowlist 検査(authorization)を
      呼ぶが内容審査を呼ばない構造になっている
- [ ] 「cheaper が workspace に閉じ込める」「deny list が保護」等の
      実強制を超える表現が、対象 guide/ADR/spec から訂正されている
- [ ] grant/task 統合の是非と、採用する場合の移行方針が
      本 spec または後続 issue で確定している
- [ ] 非 exec routing sketch の採否が本 spec で記録されている
- [ ] 破壊的変更一覧(deny list 削除等)が列挙されている

## Deferred Design Concerns

- OS-sandbox backed mode | 本再設計の非目標。真の containment が
  必要になった時点で別途検討 | 「提供しない」記述のまま放置すると
  ユーザーが過剰に期待するリスク(→ R5 の正直性要件で対処)
- `trusted` route flag の発行主体移行 | route 信頼モデルの再設計として
  後続化可能 | project 自身が trusted を宣言できる現状が残る
- deny list の opt-in profile 化 | R2 で「削除 vs profile」の確定が
  前提 | 決定未了のまま実装に入ると設計が揺れる

## Sources and Relations

- 起点 issue: https://github.com/scarecrow173/cheaper/issues/151
- 上流 discovery: `../discovery/permission-model-responsibility-redesign.md`
  (制限棚卸し・選択肢・open questions)
- 責務モデル決定 ADR: `../adr/enforce-declared-contracts-over-content-audit.md`
- refine 対象 spec:
  - `./policy-gated-command-tool.md`(deny list・workspace-resident 要件)
  - `./invocation-scoped-command-grants.md`(grant 検証要件)
  - `./pretooluse-route-resolver-and-workflow-delegation.md`(grant semantics)
- 関連 ADR:
  - `../adr/command-grant-trust-boundary-and-managed-process-reuse.md`(refine)
  - `../adr/worker-definition-runtime-policy-runtime-invariants-capability.md`(refine)
  - `../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md`(refine)
  - `../adr/defer-non-exec-routing-to-permission-model-redesign.md`(input)

---
id: "ADR-0007"
type: "adr"
status: "accepted"
title: "workspace filesystem boundary は cheaper 提供の sandboxed tool 実装で強制する"
created: "2026-09-15"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "../specs/0002-bulk-reader-built-in-worker.md"
    - "../specs/0003-cheap-writer-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0001-cheaper-v1-pi-sdk.md"
  derived-by: []
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

# 7. workspace filesystem boundary は cheaper 提供の sandboxed tool 実装で強制する

## Context and Problem Statement

`cwd` = workspace root = filesystem security boundary（§7）。Worker はこの workspace 外のファイルを読み書きしてはならない。これをどう強制するか。

## Decision Drivers

* §0: security boundary を LLM prompt だけに依存させない
* §2.5/§31: workspace 外 access 禁止・symlink escape 禁止は Runtime Invariant
* §26: filesystem policy `symlinks: deny | within-workspace`
* Pi 公式 docs: Pi に built-in sandbox は存在しない（security.md 明記）

## Considered Options

* (1) Pi 組み込み tool + prompt 制約で workspace 内に留める
* (2) Pi 組み込み tool + OS レベル sandbox（Docker / Gondolin micro-VM / OpenShell）
* (3) cheaper が `defineTool` で read/grep/find/ls/write/edit を自前実装し、全 path を canonicalize して workspace 内判定・symlink policy を適用する

## Decision Outcome

Chosen option: "(3) cheaper 提供の sandboxed tool 実装", because prompt 制約は §0 が禁止し、OS sandbox は追加インフラとポータビリティ低下を招くため。in-process で portable に boundary を強制できるのは cheaper 側 tool 実装のみ。

### Consequences

* Good, because Runtime Invariant をコードで保証できる（LLM の判断に依存しない）。
* Good, because OS/コンテナ非依存で portable。
* Bad, because path canonicalize・symlink・TOCTOU・write atomicity を自前で正しく実装する責任がある（§40 未確定設計）。
* Bad, because Pi 組み込み tool の機能（glob・grep 実装等）を（注入で再利用できない場合）再実装する必要がある。

### Confirmation

* §42 Security カテゴリ（`../` escape、絶対パス escape、workspace 外/内 symlink、workspace 外 write/read、unknown tool）の全テストが pass すること。

## Implementation Plan

* Affected paths: `packages/runtime-pi/`（予定 path）— sandboxed tool 実装（`defineTool` + TypeBox schema）、`packages/core/`（予定 path）— path security primitives・portable tool vocabulary・alias 規則
* Patterns to follow: `defineTool({name, parameters: Type.Object({...}), execute})` で `customTools` 登録。全 path は canonicalize → workspace 内判定 → symlink policy 適用
* Patterns to avoid: Pi 組み込み `read`/`write`/`edit`/`grep`/`find`/`ls`/`bash`/`powershell` の直接使用（`noTools: "builtin"` 相当で無効化、ADR-0001）、prompt ベースの制約のみ
* Dependencies or configuration: `@earendil-works/pi-coding-agent` の `defineTool`/`customTools`、`typebox`

確定内容:

* portable tool vocabulary（§10）: `read`/`grep`/`find`/`ls`/`write`/`edit`。安全な alias のみ許可（`Read→read`, `Grep→grep`, `Glob→find`, `Write→write`, `Edit→edit`）。`Bash`/`Shell`/`Execute`/`WebFetch`/`Agent`/`Search` 等の推測変換は禁止。unknown tool は fail closed
* shell tool・network tool は提供しない（§31）
* symlink policy: `within-workspace` は target を realpath/canonicalize して workspace 内なら許可・外なら拒否（§26）
* **検証項目**: 本 ADR の決定は「workspace boundary は cheaper が制御する tool 層のコードで強制する」であり、Pi 組み込み tool を無条件で使うことは選ばない。tool 本体（grep/find 等のアルゴリズム）を完全自前実装するか、Pi 組み込み tool が filesystem `operations` 等の注入点を持つ場合にそこへ cheaper の boundary 付き実装を注入して再利用するかは、design-doc で実機確認の上決める。いずれの場合も path canonicalize・workspace 内判定・symlink policy は cheaper 側コードが担う

## Verification

* [ ] workspace 外 path への read/write/edit が tool 層で拒否されるテスト
* [ ] workspace 外を指す symlink 経由のアクセスが拒否され、内側 symlink は許可されるテスト
* [ ] `../`・絶対パス escape が拒否されるテスト
* [ ] unknown tool 名が fail closed するテスト
* [ ] Pi 組み込み tool が有効化されないこと（session に cheaper tool のみ登録）をテストで確認

## Pros and Cons

### Option 1: Pi 組み込み tool + prompt 制約

* Good, because 実装不要。
* Bad, because §0「security boundary を prompt だけに依存させない」に反し、Pi に built-in sandbox がないため boundary は守られない。

### Option 2: Pi 組み込み tool + OS sandbox

* Good, because OS レベルで強い隔離が得られる。
* Bad, because Docker/QEMU/OpenShell 等の追加インフラ・環境依存が入り、one-shot CLI の軽量さと portable 性を損なう。

### Option 3: cheaper sandboxed tool

* Good, because in-process・依存なし・portable で Invariant をコード保証できる。
* Bad, because セキュリティ実装の責任が cheaper に移る。

## More Information

* Pi security（no built-in sandbox）: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md
* Pi custom tools（defineTool/customTools/TypeBox）: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md
* 引継ぎ §2.5, §7, §10, §26, §31

## 更新履歴

### 2026-09-15: ハイブリッド方式の確定（実機プローブ結果）

* `read`/`write`/`edit`/`ls` は Pi の tool factory（`createReadTool`/`createWriteTool`/`createEditTool`/`createLsTool`、いずれも package root export）に cheaper の boundary 付き `operations` を注入して再利用する。注入される path は `resolveToCwd` 済みの絶対パスであることを確認。
* `grep`/`find` は cheaper 自前実装（`defineTool` + TypeBox + sandboxed walker）とする。理由: Pi grep は検索本体が `spawn(ensureTool("rg"))` で `GrepOperations` に search 差し替え点がなく、rg 子プロセスが operations 境界を迂回する。rg 未導入時の自動ダウンロードも発生する。find は `operations.glob` で fd を回避可能だが、grep と共通 walker を持つため自前で統一する。
* boundary 強制（path canonicalize・workspace 内判定・symlink policy）はいずれの場合も cheaper 側コードが担う、という本 ADR の決定は変更しない。

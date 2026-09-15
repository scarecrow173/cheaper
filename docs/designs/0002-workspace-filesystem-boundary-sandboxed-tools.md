---
id: "DESIGN-0002"
type: "design"
status: "approved"
title: "workspace filesystem boundary と sandboxed tools"
created: "2026-09-15"
updated: "2026-09-15"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "0004-pi-execution-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "../adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  derived-by:
    - "../plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# workspace filesystem boundary と sandboxed tools

## Context

ADR-0007 のハイブリッド方針を実装レベルに落とす。Pi 実機プローブ（tmp/pi-probe、@earendil-works/*@0.85.1）で確定した事実: `createReadTool`/`createWriteTool`/`createEditTool`/`createLsTool`（および `*ToolDefinition` 版）が package root export で `operations` 注入を受け付ける。注入される path は `resolveToCwd`（`~` 展開・Unicode 空白正規化・`@` 接頭辞除去）済みの**絶対パス**。`grep` は `spawn(ensureTool("rg"))` で検索本体が operations で差し替え不能かつ rg 自動ダウンロードが走る。`find` は `operations.glob` 注入で `fd` を完全に回避可能だが、自前実装とする（walker を grep と共有するため）。

## Scope

### In Scope

- workspace root の canonical 化と `resolveWorkspacePath` アルゴリズム
- read/write/edit/ls: Pi tool + boundary 付き `operations` 注入
- grep/find: cheaper 自前 `defineTool` + TypeBox + sandboxed walker
- 書き込み原子性（tmp + rename）
- mutation journal（`meta.changes` の唯一の情報源）

### Out of Scope

- tool 実行回数の budget 判定そのもの（DESIGN-0004 が wrap 層で実施）
- OS レベルの sandbox（脅威モデル外）
- `.env` 等 workspace 内ファイルへの denylist（V1 は持たない）

## Components and Boundaries

- workspace root: `--cwd`（省略時 `process.cwd()`）→ 存在確認（無ければ `INVALID_REQUEST`）→ `fs.realpath` で canonical 化。以後 root は realpath 値
- `workspace/paths.ts`（core）: `resolveWorkspacePath(root, input, symlinkPolicy)`:
  1. NUL 文字を含む input は拒否
  2. `abs = path.resolve(root, input)`（絶対 input もそのまま扱い、内側判定に委ねる）
  3. `rel = path.relative(root, abs)`。`rel` が `..` で始まる／絶対なら ESCAPE。win32 では比較を大文字小文字非依存で行う
  4. root から abs へ component を順に辿り、存在する component ごとに `lstat`。symlink なら: policy `deny` → 拒否、`within-workspace` → `realpath` が root 配下でなければ拒否、配下なら realpath 側から辿り直す。存在しない末尾は許容（create 用）。ただし存在する最深の祖先の realpath は root 配下であること
  5. 戻り値 `{ real: string（canonical 絶対）, display: string（POSIX 区切り workspace 相対）}`
- `tools/operations.ts`（runtime-pi）: Pi tool に注入する boundary 付き `ReadOperations`/`WriteOperations`/`EditOperations`/`LsOperations`。各関数は受け取った絶対 path（Pi が cwd 解決済み）を `resolveWorkspacePath` に通し、違反時は `Error("Access denied: path is outside the workspace")` を投げる。Pi tool がそれを tool error result としてモデルへ返す
- `tools/walker.ts`: 自前 grep/find 共通の再帰 walker。symlink policy 適用、`.git`・`node_modules` を skip、1 ファイル 1MB 超・先頭 8KB に NUL を含むバイナリを skip
- `tools/grep.ts`: `{ pattern: string(JS RegExp), path?: string = ".", glob?: string, ignoreCase?: boolean, maxResults?: int(default 100, max 500), context?: int(0–3) }` → `path:line: text` 行
- `tools/find.ts`: `{ pattern: string(glob, `path.matchesGlob` 使用), path?: string = ".", maxResults?: int(default 200, max 1000) }` → 相対 path 行
- `journal.ts`: write/edit operations 内でファイルごとに初回書き込み前の内容（存在しなければ null）を記録し、書き込み後の内容を更新。実行終了時に `before` と disk 上の最終内容を比較、差が無いものは除外、`kind = before===null ? "created" : "modified"`、行差分は `diff` パッケージ `diffLines` で added/deleted を算出

## Data and Control Flow

- 全 tool の `execute` は runtime-pi が wrap し、`maxToolCalls` 予算判定を実行前に行う（DESIGN-0004）
- 書き込み原子性: 同ディレクトリに `.<name>.cheaper-tmp-<rand>` を書いて `rename`。tmp path も boundary 判定を通す
- journal → `meta.changes` が唯一の変更情報源（ADR-0006 の「wrapper 実測」の具体化）。shell tool が存在せず全書き込みが write/edit operations を通るため、実行前後の全域 snapshot は不要

## Non-Functional Constraints

- TOCTOU: check-then-act の残余リスクを認める。脅威モデルは「モデルの誤動作・逸脱」であり同時実行の敵対プロセスではない。単一プロセス・one-shot のため許容
- secret/env: 環境変数・workspace 外は tool から到達不能。workspace 内の `.env` 等は読める（workspace は信頼境界内）。V1 では denylist を持たない
- 外部プロセス不実行: sandboxed tool 層は rg/fd 等の外部バイナリ・ネットワークダウンロードに依存しない（ensureTool の自動 DL 経路を排除する設計）

## Risks and Trade-offs

- Pi tool 注入の注意点: `resolveToCwd` は `~` 展開と `@` 接頭辞除去を行うため、モデルが `~/x` を渡すと home 配下の絶対 path が operations に届く → boundary 判定で拒否されるため安全だが、エラーメッセージで home path が漏れないよう `display` は workspace 相対のみを使う
- read tool は `resolveReadPathAsync` でファイル名綴り変種（NFD・curly quote 等）を試す。変種は同一ディレクトリ内の別綴りであり boundary 判定には影響しない
- grep/find 自前は rg/fd より低速・機能限定。portable vocabulary（§10）の範囲に絞ることで許容

## Deferred Design Concerns

- workspace 内 denylist（`.env` 等） | 脅威モデルがモデル誤動作止まり | 機密保持要件が変わった時点 | 無視すると secret 漏洩リスクが残る
- bash/powershell tool の提供 | V1 では Worker vocabulary に含めない | 委譲タスクに shell が必須になった時点 | 導入時は boundary 設計の再考が必要

## Verification Notes

- [ ] workspace 外 path（`../`, 絶対, `~`）は全 tool で拒否され model へ error result として返る
- [ ] symlink: `deny` で拒否、`within-workspace` で workspace 外指しは拒否・内は許可
- [ ] 存在しない file への write は create として成功、親 dir 外への mkdir は拒否
- [ ] grep/find が `.git`/`node_modules`/バイナリ/1MB 超を skip し、外部プロセスを spawn しない
- [ ] write/edit が tmp+rename で原子書き込みし journal が before/after を正確に記録
- [ ] `meta.changes` が journal のみから生成され、差分なしファイルを除外する

# Workers ガイド

worker は `cheaper` が実行する「仕事の定義」。親 Coding Agent が
機械的・大 context の処理を委譲する対象となる、独立した
read/write unit。

## worker とは

worker は Markdown file で定義する。front matter に
name/description/tools/cheaper section を持ち、本文は worker へ
渡す instruction（prompt）になる。

```markdown
---
name: my-worker
description: What this worker does.
tools:
  - read
  - grep
cheaper:
  model:
    tier: low
    workload:
      outputPerInput: 0.3
  input:
    schema:
      file: ./schemas/my-worker.input.schema.json
  output:
    schema:
      file: ./schemas/my-worker.output.schema.json
---

You are a worker that ...（instruction 本文）
```

- `name`: `run <worker>` で参照する名前（`[a-z0-9][a-z0-9-]*`）。
- `tools`: worker が使える tool。`policy.yaml` の `tools.allow`
  でさらに制限される（両方で許可された tool のみ有効）。
  `command` を使う worker はこれに trusted layer の
  `command.grants` が空でないことが加わる三重ゲート（`tools`
  宣言 ∩ `tools.allow` ∩ `command.grants`）を通る。`tools.allow`
  は exposure toggle であり executable の authorization ではない —
  project が flip しても executable を認可することはできない
  （[command ガイド](./command.md)）。
- `cheaper.model.tier`: 要求する model tier（`low`/`medium`/
  `high`）。`policy.yaml` の `model.maxTier` が上限。
- `cheaper.model.workload.outputPerInput`: 平均 output token /
  input token の比（省略時 `1.0`）。auto tier 分類の
  `effectiveCost = input + output * outputPerInput` に使われ、
  worker ごとに同じ model 集合でも tier 境界が変わり得る。
  read 偏重なら小さく（例: `0.05`）、生成偏重なら大きくする。
- `cheaper.input.schema` / `cheaper.output.schema`: JSON Schema
  file への path（worker file からの相対 path）。
- `cheaper.writeScope`: `targets` を指定すると、runtime が
  `targets[].path` 以外への write を禁止する（writer 系 worker の
  標準設定）。`command` tool（`!fixed` grant surface）とは非両立で、
  writeScope を宣言した worker が `command` を expose すると scope
  生成が `POLICY_VIOLATION`（"command is incompatible with worker
  writeScope"）で fail closed する（子 process の副作用を
  resource 層が観測できず coverage が vacuous になるため。詳細は
  [command ガイド](./command.md)）。`process` tool の task（`fixed`
  grant）は writeScope 下でも実行できるが、task の `outputs` 宣言が
  `targets` を coverage することが実行時に要求される — `outputs`
  宣言（nonempty）は `outputs ⊆ writeScope` 必須、`outputs: []` は
  「副作用を宣言しない」契約で coverage 成立、`outputs` 未宣言は
  coverage を証明できず `POLICY_VIOLATION` で拒否される（tri-state
  規約 — [managed process ガイド](./managed-process.md)）。
- `cheaper.optionalTools`: `tools` に宣言した scope-gated tool
  （現状 `process` と `command` のみ）の **真部分集合** を列挙する
  opt-in key。列挙した tool は content-gate（`process.tasks` /
  `command.grants` が空）不成立時に `POLICY_VIOLATION` で scope
  構築を失敗させる代わりに **未登録へ縮退** し、run は残りの
  tool で継続する。併せて、degradable 宣言された exec surface
  （`process` / `command`）に到達可能な grant の executable pin
  （PATH 解決・deny 判定）が失敗した場合も、scope 構築を
  `POLICY_VIOLATION` で fail させず当該 tool のみ未登録へ縮退
  する — pin-fail×degradable の規則は `process` と `command` の
  両 surface に適用される。縮退した tool は system prompt に
  利用不可注記として列挙され、model が呼んでも登録経路が
  存在しないため unknown-tool として安全に失敗する。opt-in は
  capability を縮める方向にのみ働き、権限を広げることには
  使えない。`tools.allow` による admission 除外・grant 解決・
  argv pattern・spawn 前 executable 再検証等の call 経路の
  認可機構は一切変わらず、`tools.allow` で除外された宣言
  tool は opt-in worker でも従来通り run 全体を
  `POLICY_VIOLATION` で fail させる（admission と
  content-gate は別の層 — [command ガイド](./command.md)）。
  検査 rule: 配列でない /
  空配列 / `tools` 未宣言の名 / vocabulary に無い名 / gated で
  ない tool / 重複 / `tools` 全体を含む宣言は全て
  `InvalidWorkerDefinition` で拒否される。opt-in worker は出力
  schema の `status`（相当 field）に `blocked` 相当の終了状態を
  含めることを推奨する — runtime は `status` enum に `"blocked"`
  がある worker にのみ「利用不可 tool が必要なら blocked で
  終了せよ」の誘導行を prompt へ追加し、schema が受理しない
  終了状態を促す矛盾を防ぐ。

## built-in worker

| worker | alias | 目的 | `outputPerInput` | tools |
| --- | --- | --- | --- | --- |
| `bulk-reader` | `bulk-read` | 大規模 read・抽出・圧縮された構造化 findings を返す（read-only） | `0.05` | read, grep, find, ls, count_tokens |
| `code-writer` | `code-write` | 機械的・pattern ベースのコード生成・変更を workspace に適用する | `0.3` | read, grep, find, ls, write, edit, count_tokens |
| `doc-writer` | `doc-write` | decisions/facts から document を生成する | `1.0` | read, grep, find, ls, write, edit, count_tokens |
| `tool-operator` | `tool-operate` | 目標に向かって tool call を反復する操作 worker — evidence 付きの compact structured result を返す | `0.3` | read, grep, find, ls, count_tokens, process, command |

`outputPerInput` の built-in 値は暫定の推定値であり、
telemetry（usage log）による検証は follow-up 予定。

`tool-operator` は `process` / `command` を `cheaper.optionalTools`
で opt-in 宣言している（上記 frontmatter key 参照）。policy が
tasks / grants を定義しない環境ではこれらは未登録へ縮退して
run が継続し、worker は残りの tool で実行を続けるか、必要な
capability が欠けていれば `status: "blocked"` で終了する。
grants / tasks が定義された環境では通常通り登録され、宣言
task 名・grant 名に限定して利用できる。

`status` が `blocked` / `partial` で、gated tool（`process` /
`command`）の欠如または必要 executable をカバーする grant の
不在が原因の場合、output の optional field `result.grantRequests`
に machine-readable な request が列挙される。各 entry は不足した
`executable`（必須）と、意図した argv の形状 hint `argvHint`
（optional）、提案する grant 名 `name`（optional）を運ぶ。

`grantRequests` は **advisory** であり grant の実行ではない —
invoker へ表面化する request であり、権限昇格の試行ではない。
全 field（`executable`・`argvHint`・`name`）は model 生成・
prompt-injectable な **untrusted 値**であり、cheaper は request
の正しさを保証しない。invoker（user / parent agent）が採る
loop は: `result.grantRequests` を読む → grant spec を組み立て
**re-run 前に必ず review する** → user approval を得る →
`--grant` 付きで re-run する（永続化するなら
`cheaper policy grant add`）。approval・review は invoker の責務
であり、cheaper 側に interactive prompt は存在しない。re-run 時に
deny list（opt-in — `security.execDenyPatterns` 宣言時のみ）・
argv pattern の compile 等が再検査されるため、不正な
request 由来の spec は fail-closed で拒否される。

writer 系 2 worker は `writeScope: targets` で、`targets[].path`
以外への write を runtime が強制的に禁止する（canonical file tool
の write に限る — exec 子プロセスの fs 副作用には及ばない）。

### 入力 schema（要点）

- `bulk-reader`: `task`（必須）・`paths`・`hints`・`maxFindings`
- `code-writer`: `task`（必須）・`targets[].path` + `intent`
  （`create`/`modify`/`create-or-modify`、必須）・`references`・
  `constraints`
- `doc-writer`: `task`・`documentType`（`adr`/`spec`/`design`/
  `plan`/`task`/`readme`/`guide`/`reference`/`release-notes`/
  `other`）・`targets`（必須）・`decisions`・`facts`・
  `references`・`constraints`
- `tool-operator`: `task`（必須）・`context`・`constraints`・
  `output`（`focus`・`maxFindings` の hint）

schema の完全な定義は CLI パッケージ同梱の
`workers/schemas/*.schema.json` を参照。

## custom worker

```bash
# named resolution（built-in / user / project の discovery）
echo '{"task": "..."}' | cheaper run my-worker

# 明示 file 指定
echo '{"task": "..."}' | cheaper run --worker-file ./my-worker.md
```

worker の discovery 順: project（`<cwd>/.cheaper/workers/`）→
user（`<userConfigDir>/workers/`）→ built-in。同名の worker は
project が user、user が built-in を上書きする。

## policy との関係

runtime は `policy.yaml` の以下を参照して worker を制約する
（詳細は [configuration ガイド](./configuration.md)）:

- `tools.allow`: worker の `tools:` と intersect して許可。
  `command` については exposure のみを制御し、executable の認可は
  trusted layer の `command.grants` が担う
  （[command ガイド](./command.md)）
- `model.maxTier` / `model.tierFallback`: worker の
  `cheaper.model.tier` 要求の上限と fallback 挙動
- `filesystem.symlinks` / `filesystem.denyPaths`: file access の
  boundary
- `output.maxTokens` / `output.repairAttempts`: output schema
  不適合時の repair 挙動

## 関連

- CLI 呼び出し: [usage ガイド](./usage.md)
- `command` tool の三重ゲート・grant・信頼境界:
  [command ガイド](./command.md)
- worker 定義の設計根拠: `docs/designs/writer-worker-contracts.md`・
  `docs/specs/code-writer-and-doc-writer-built-in-workers.md`

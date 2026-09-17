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
- `cheaper.model.tier`: 要求する model tier（`low`/`medium`/
  `high`）。`policy.yaml` の `model.maxTier` が上限。
- `cheaper.input.schema` / `cheaper.output.schema`: JSON Schema
  file への path（worker file からの相対 path）。
- `cheaper.writeScope`: `targets` を指定すると、runtime が
  `targets[].path` 以外への write を禁止する（writer 系 worker の
  標準設定）。

## built-in worker

| worker | alias | 目的 | tools |
| --- | --- | --- | --- |
| `bulk-reader` | `bulk-read` | 大規模 read・抽出・圧縮された構造化 findings を返す（read-only） | read, grep, find, ls, count_tokens |
| `code-writer` | `code-write` | 機械的・pattern ベースのコード生成・変更を workspace に適用する | read, grep, find, ls, write, edit, count_tokens |
| `doc-writer` | `doc-write` | decisions/facts から document を生成する | read, grep, find, ls, write, edit, count_tokens |

writer 系 2 worker は `writeScope: targets` で、`targets[].path`
以外への write を runtime が強制的に禁止する。

### 入力 schema（要点）

- `bulk-reader`: `task`（必須）・`paths`・`hints`・`maxFindings`
- `code-writer`: `task`（必須）・`targets[].path` + `intent`
  （`create`/`modify`/`create-or-modify`、必須）・`references`・
  `constraints`
- `doc-writer`: `task`・`documentType`（`adr`/`spec`/`design`/
  `plan`/`task`/`readme`/`guide`/`reference`/`release-notes`/
  `other`）・`targets`（必須）・`decisions`・`facts`・
  `references`・`constraints`

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

- `tools.allow`: worker の `tools:` と intersect して許可
- `model.maxTier` / `model.tierFallback`: worker の
  `cheaper.model.tier` 要求の上限と fallback 挙動
- `filesystem.symlinks` / `filesystem.denyPaths`: file access の
  boundary
- `output.maxTokens` / `output.repairAttempts`: output schema
  不適合時の repair 挙動

## 関連

- CLI 呼び出し: [usage ガイド](./usage.md)
- worker 定義の設計根拠: `docs/designs/0011-writer-worker-contracts.md`・
  `docs/specs/0006-code-writer-and-doc-writer-built-in-workers.md`

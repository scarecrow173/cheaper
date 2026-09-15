---
id: "DESIGN-0005"
type: "design"
status: "approved"
title: "CLI contract・result/error envelope・exit code"
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
    - "../adr/0004-one-shot-json-first-cli.md"
    - "../adr/0006-wrapper-measured-deterministic-metadata.md"
  derived-by:
    - "../plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# CLI contract・result/error envelope・exit code

## Context

ADR-0004 の one-shot JSON-first プロセスモデルを CLI contract に落とす。親 Coding Agent が機械的に解釈できる単一 envelope を stdout に出し、診断は stderr に限定する。

## Scope

### In Scope

- コマンド・引数解析（`util.parseArgs`）
- stdin 入力規則
- success/error envelope schema
- `--format text` 出力
- exit code 適用と未捕捉例外の envelope 化

### Out of Scope

- Worker 実行そのもの（DESIGN-0004）
- usage telemetry のファイル記録（envelope の meta のみ）

## Components and Boundaries

- `args.ts`: commands `run <worker>` / `bulk-read` / `cheap-write`（後者 2 つは `run` のエイリアスで追加引数なし）。options: `--input <file|->`（既定 `-`）、`--worker-file <path>`（`run` 専用、`<worker>` と併用不可）、`--cwd`、`--timeout <duration>`、`--format json|text`（既定 json）、`--quiet`、`--help`、`--version`。`cheaper run ./foo.md` のような path 風 name は `INVALID_WORKER_NAME`
- `envelope.ts`: success/error envelope の組み立てと stdout 出力
- `text-format.ts`: `--format text` の人間向け整形
- `main.ts`: 引数解析 → stdin 読取 → core/runtime 呼び出し → envelope 出力 → exit code。未捕捉例外は `INTERNAL_ERROR`（70）で envelope 化

## Data and Control Flow

- stdin: TTY で stdin 未指定でも待たず空入力として扱い → `INVALID_REQUEST`。入力は JSON object 必須
- success envelope:

```json
{
  "schemaVersion": 1,
  "ok": true,
  "worker": "bulk-reader",
  "result": {},
  "meta": {
    "durationMs": 0,
    "model": { "provider": "", "id": "", "tier": "low" },
    "usage": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0, "costUsd": 0 },
    "turns": 0,
    "toolCalls": 0,
    "rejectedToolCalls": 0,
    "repairs": 0,
    "transientRetries": 0,
    "changes": {
      "files": [{ "path": "src/a.ts", "kind": "created", "linesAdded": 10, "linesDeleted": 0 }],
      "totals": { "files": 1, "linesAdded": 10, "linesDeleted": 0 }
    }
  }
}
```

- error envelope: `{ "schemaVersion": 1, "ok": false, "worker": "<name>|null", "error": { "code", "message", "exitCode", "retryable", "details"?, "cause"?: {"code","message"} }, "meta": <取得できた範囲。未取得は null> }`
- stdout は envelope 1 つ + 改行のみ。stderr は診断（model 選択行・警告）に限定し progress は出さない。`--quiet` は stderr の警告/診断を抑止（error は出す）。TTY 判定なし
- `--format text`: 1 行目 `ok`/`error <code>`、続けて worker・model・usage・changes の表、result/error を整形 JSON
- exit code は core の対応表（DESIGN-0001）のみを参照

## Non-Functional Constraints

- stdout の byte 単位決定性: envelope 以外を一切書かない。Pi SDK の内部イベントやログが stdout に混入しないこと
- `schemaVersion: 1` で envelope の後方互換を管理

## Risks and Trade-offs

- `bulk-read`/`cheap-write` エイリアスは Worker 名固定のため将来の名前変更で壊れやすい。V1 は built-in 名を固定する方針で許容

## Deferred Design Concerns

- `--worker-file` と named Worker の併用禁止の緩和 | 引継ぎの現行仕様 | 要求が出た時点 | 両方指定時の優先順位を誤る UX リスク

## Verification Notes

- [ ] stdout が envelope 1 行のみで、内部イベント/ログが混入しない
- [ ] error envelope が `error.code`/`exitCode`/`retryable` を含み exit code と一致する
- [ ] stdin 未指定の TTY 実行でブロックせず `INVALID_REQUEST` exit 2
- [ ] `--format text` が機械可読でないことを明示しつつ envelope 相当の情報を出す

---
id: "DESIGN-0001"
type: "design"
status: "draft"
title: "コアドメインモデル: Worker Definition・Resolver・Runtime Policy・tool vocabulary・error model"
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
    - "0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "0003-model-resolver.md"
    - "0004-pi-execution-engine.md"
    - "0005-cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "../adr/0002-worker-yaml-frontmatter-markdown.md"
    - "../adr/0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# コアドメインモデル: Worker Definition・Resolver・Runtime Policy・tool vocabulary・error model

## Context

`packages/core` が担う Pi 非依存のドメイン層を確定する。Worker Definition のパース/検証/解決、Runtime Policy のスキーマ・マージ・CLI 制限、tool vocabulary の正規化、capability 交差判定、error model の code→exitCode 対応表をここに閉じ込める。上流は SPEC-0001（§8–§33）と ADR-0002/0003。

## Scope

### In Scope

- 設定ディレクトリ解決（user / project / built-in / explicit file）
- Worker Definition（YAML frontmatter + Markdown）のパースと検証
- tool vocabulary と alias 正規化
- Worker Resolver（origin 優先順位と no-fallback 規則）
- Runtime Policy の型・ロード・マージ・バリデーション・CLI restriction
- capability 交差判定（tools × policy.allow、tier × maxTier）
- `CheaperError` と code→exitCode の唯一の対応表

### Out of Scope

- Pi SDK への接続、session 実行（DESIGN-0004）
- filesystem boundary の強制（DESIGN-0002）
- CLI 引数解析と envelope 出力（DESIGN-0005）
- APM integration（別 spec に deferred）

## Components and Boundaries

- `config/dirs.ts`: `resolveConfigDirs({ cwd })` → `{ userConfigDir, projectDir, builtinDir }`。
  - user config dir: `$CHEAPER_CONFIG_DIR` > `$XDG_CONFIG_HOME/cheaper` > `~/.config/cheaper`（全 OS 共通）。配下に `workers/*.md`, `policy.yaml`, `models.yaml`
  - project: `<cwd>/.cheaper/{workers/, policy.yaml, models.yaml}`
  - built-in: `packages/cli/workers/*.md`, `packages/cli/workers/schemas/*.json`, `packages/cli/policy.yaml`
- `worker/parse.ts`: 先頭行 `---` から次の `---` までを `yaml` パッケージで parse、残りを body。body は trim 後 non-empty 必須
- `worker/validate.ts`: frontmatter 型検証・name 規則・tool 正規化・schema 参照解決・Ajv `validateSchema`
- `worker/resolve.ts`: `resolveWorker(name, { cwd, userConfigDir, builtinDir })` と `--worker-file` 経路
- `policy/{schema,load,merge,restrict}.ts`: complete/partial 両スキーマ、§23 のロード順、merge 規則、CLI restriction
- `tools/vocabulary.ts`: `ToolName` union、alias 表、正規化
- `errors.ts`: `CheaperError` と対応表
- `duration.ts`: `parseDuration`（duration 文字列→ms 変換の唯一の箇所）
- `model/` は本 design の型のみ共有し、実体は DESIGN-0003

```ts
type ToolName = "read" | "grep" | "find" | "ls" | "write" | "edit";
type Tier = "low" | "medium" | "high";
type SchemaRef = { file: string } | { inline: object }; // exactly one
interface WorkerDefinition {
  name: string;
  description: string;
  tools: ToolName[];
  cheaper: {
    model: { tier: Tier };
    input?: { schema: SchemaRef };
    output: { schema: SchemaRef };
  };
  instructions: string;
  sourcePath: string;
  origin: "project" | "user" | "builtin" | "explicit";
  inputSchema?: object;
  outputSchema: object;
}
```

## Data and Control Flow

- Worker パース: frontmatter 不在・`---` 未閉鎖・YAML エラー・body 空はすべて `INVALID_WORKER_DEFINITION`
- `cheaper:` 配下は `model` / `input` / `output` のみ、各内部も additionalProperties false。`cheaper.input.schema` はユーザー承認により引継ぎ §12 を拡張して追加した任意項目。ある場合は実行前に入力 JSON を検証し、不一致は `INVALID_INPUT`（exit 2）
- top-level 未知 field は ignore（警告も出さない）
- tool alias: `Read→read, Grep→grep, Glob→find, Write→write, Edit→edit` の完全一致のみ。それ以外の未知名は `UNKNOWN_TOOL`。正規化後の重複は `INVALID_WORKER_DEFINITION`。`tools` は non-empty
- name: `^[a-z0-9][a-z0-9-]*$`（不一致は `INVALID_WORKER_NAME`）、ファイル stem と一致（不一致は `WORKER_NAME_MISMATCH`）
- schema ref: `file` は Worker ファイルのディレクトリ基準で解決、拡張子 `.json/.yaml/.yml`、欠落は `OUTPUT_SCHEMA_NOT_FOUND`/`INPUT_SCHEMA_NOT_FOUND`。JSON Schema は draft 2020-12 を正とし `ajv/dist/2020`（`strict: true, allErrors: true`）で `validateSchema`。他 draft の `$schema` 指定・コンパイル失敗は `INVALID_OUTPUT_SCHEMA`/`INVALID_INPUT_SCHEMA`
- Resolver: name 検証 → project path が存在すれば parse（失敗は即エラー、fallback なし）→ user → builtin → `WORKER_NOT_FOUND`。`--worker-file` は cwd 基準で解決（絶対 path 可、workspace 外可）、存在しなければ `WORKER_NOT_FOUND`、stem と name の一致も要求、named resolution を一切行わない
- Runtime Policy: 内部表現は `timeoutMs: number` に正規化。complete schema（全 field 必須）と partial schema（`version` 以外任意）、両方 additionalProperties false（全階層）。ロード順は §23。merge: object deep merge / scalar replace / array 全体 replace。CLI restriction は V1 で `--timeout` のみ、`--timeout > policy.timeout` は `POLICY_VIOLATION`
- capability 交差: requested tools ⊄ policy.tools.allow → `POLICY_VIOLATION`（欠けた tool 名を details に）。effective tools = requested 全部。tier > maxTier → `POLICY_VIOLATION`

### Error model

`CheaperError { code, message, exitCode, retryable, details?, cause? }`。code→exit の対応表は core で唯一定義し、cli は参照するだけ。

| exitCode | codes |
| --- | --- |
| 2 | `INVALID_REQUEST`（CLI 引数・不正 JSON 入力・cwd 不存在）, `INVALID_INPUT`（input schema 不一致） |
| 3 | `WORKER_NOT_FOUND`, `INVALID_WORKER_DEFINITION`, `INVALID_WORKER_NAME`, `WORKER_NAME_MISMATCH`, `UNKNOWN_TOOL`, `INVALID_RUNTIME_POLICY`, `OUTPUT_SCHEMA_NOT_FOUND`, `INVALID_OUTPUT_SCHEMA`, `INPUT_SCHEMA_NOT_FOUND`, `INVALID_INPUT_SCHEMA`, `INVALID_MODEL_CONFIG` |
| 4 | `POLICY_VIOLATION` |
| 5 | `MODEL_TIER_UNAVAILABLE`, `PROVIDER_FAILURE` |
| 6 | `TOOL_EXECUTION_FAILURE`, `INVALID_OUTPUT`（repair 尽きても不適合）, `MAX_TURNS_EXCEEDED`, `MAX_TOOL_CALLS_EXCEEDED` |
| 7 | `TIMEOUT` |
| 8 | `ABORTED` |
| 9 | `PARTIAL_MUTATION` |
| 70 | `INTERNAL_ERROR` |

- `retryable: true` は `PROVIDER_FAILURE`, `TIMEOUT`, `MODEL_TIER_UNAVAILABLE` のみ
- tool 層での workspace escape は fatal ではなく tool error としてモデルへ返し、`meta.rejectedToolCalls` に計上（Runtime Invariant はコードで守られているため実行継続可）

## Non-Functional Constraints

- `packages/core` は `@earendil-works/*` に依存しない（依存は `ajv`, `yaml` のみ、DESIGN-0008）
- 表現ドリフト禁止: duration 文字列はファイル形式のみで、内部は `timeoutMs` 一本（`parseDuration` 一箇所）
- fail closed: project Worker が invalid でも user/builtin へ fallback しない

## Risks and Trade-offs

- alias を完全一致のみに絞るため、Worker 作者が `Bash` 等を書くと `UNKNOWN_TOOL` になる。一覧を error details に含めて自己修正を促す
- additionalProperties false は将来の field 追加を後方互換で行えない。`version` field を policy/models.yaml 側で担保し、Worker frontmatter の `cheaper:` 配下の厳格性と引き換えに誤記を早期検出する

## Deferred Design Concerns

- Worker の `cheaper:` 配下への将来拡張（環境変数許可リスト等） | V1 スコープ外 | 要求が出た時点 | 現状は厳格 schema のため追加時に互換性判断が必要

## Verification Notes

- [ ] Worker resolution: project invalid → fallback せず `INVALID_WORKER_DEFINITION` exit 3
- [ ] Worker resolution: project → user → builtin の優先順位と `--worker-file` の独立経路
- [ ] Validation: name 規則・stem 一致・alias・重複・`cheaper:` strict・schema file/inline exactly one
- [ ] Runtime Policy: ロード順・merge 規則・additionalProperties false・`--timeout` restriction
- [ ] capability: tools ⊄ allow → `POLICY_VIOLATION` exit 4、tier > maxTier → `POLICY_VIOLATION`
- [ ] error model: 全 code が対応表に存在し exitCode・retryable が一貫

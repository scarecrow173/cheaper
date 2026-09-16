---
id: "DESIGN-0012"
type: "design"
status: "approved"
title: "Token counter contract (core module, count_tokens tool, count-tokens CLI)"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0007-canonical-token-counter.md"
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
    - "0005-cli-contract-envelope-exit-codes.md"
    - "../adr/0011-canonical-tokenizer-js-tiktoken.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0007-canonical-token-counter.md"
    - "../adr/0011-canonical-tokenizer-js-tiktoken.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Token counter contract

## Context

SPEC-0007 の canonical Token Counter と 2 つの入口（`count_tokens` tool・
`count-tokens` CLI）の contract を確定する。tokenizer は ADR-0011 で
`js-tiktoken` / `o200k_base` に決定済み。

## Scope

### In Scope

- `@cheaper/core` の token count モジュール（`src/token/count.ts`）
- `TOOL_NAMES` への `count_tokens` 追加（core `tools/vocabulary.ts`）
- runtime-pi `tools/registry.ts` の `count_tokens` ToolDefinition
- `packages/cli` の `count-tokens` command
- builtin `policy.yaml` の `tools.allow` への `count_tokens` 追加と、
  built-in Worker frontmatter への tool 追加

### Out of Scope

- 複数 file / glob の cumulative count
- threshold 判定ロジック（CLI は測定のみ返し、判定は呼び出し側）
- worker 実行中の model 解決・session 構築への変更

## Components and Boundaries

### core: `countFileTokens`

```ts
export interface TokenizerIdentity {
  name: "o200k_base";
  implementation: "js-tiktoken";
  version: string; // 導入 version と一致させる定数
}

export interface FileTokenCount {
  path: string;      // workspace-relative POSIX display path
  tokens: number;
  bytes: number;
  tokenizer: TokenizerIdentity;
}

export function countFileTokens(opts: {
  root: string;            // canonical 化済み workspace root
  path: string;            // workspace 相対または絶対（boundary 解決する）
  symlinkPolicy: SymlinkPolicy;
}): Promise<FileTokenCount>
```

処理: `resolveWorkspacePath`（boundary + symlink policy 適用）→ `readFile` →
`Buffer` を utf8 decode → `o200k_base` encode → `tokens` 数。file 本文は返さない。

- **決定性**: 同一 file bytes → 同一 count。encoding は module 単一 instance。
- **binary/無効 utf8**: utf8 lossy decode（U+FFFD 置換）してそのまま count する。
  binary 特別扱いはしない（判定に使う呼び出し側は拡張子等を見ればよい）
- **不存在 file / directory**: `readFile` の失敗をそのまま上位へ（CLI 側で
  `INVALID_REQUEST` 等に分類。tool 側は tool error result）
- tokenizer version は core 内の定数として export し、導入 version と一致を
  test で保証する

### tool vocabulary / registry

- `TOOL_NAMES` に `"count_tokens"` を追加。alias は追加しない
- registry の tool 名は `count_tokens`。parameters は
  `{ path: string }`（typebox、workspace 相対 path）。戻りは text content に
  `{ path, tokens, bytes, tokenizer }` の JSON。巨大な本文は返さない
- tool 実装は `createSandboxedTools` の opsOpts と同じ
  `{ root, symlinkPolicy }` で `countFileTokens` を呼ぶ（journal 不要、read-only）
- built-in Worker frontmatter（bulk-reader・code-writer・doc-writer）の
  `tools:` と builtin `policy.yaml` の `tools.allow` に `count_tokens` を追加

### CLI: `count-tokens`

- `cheaper count-tokens <path> [--cwd <path>] [--format json|text] [--quiet]`
- `ParsedCli` に `{ kind: "count-tokens", path, cwd, format, quiet }` を追加。
  `--input`/`--worker-file`/`--timeout` は受理しない
- 処理: `canonicalizeWorkspaceRoot(cwd)` → `loadPolicy`（builtin→user→project
  merge で effective `filesystem.symlinks` を得る）→ `countFileTokens` →
  envelope 出力
- LLM/model/provider/worker 解決には一切触れない（`createModelRuntime`・
  `resolveWorker`・`loadModelConfig` を呼ばない）
- stdout: success envelope `ok: true`・`worker: null`・
  `result: { path, tokens, bytes, tokenizer }`。error は従来どおり
  error envelope（`INVALID_REQUEST` / boundary 系）

## Data and Control Flow

```
APM Hook / Parent        Worker (Pi tool call)        CLI
      │                        │                        │
      └──── cheaper count-tokens ──┘              count-tokens <path>
                        │                             │
              (tool) countFileTokens ◄──── core ──────┘
                        │
              resolveWorkspacePath → readFile → o200k_base encode → count
```

## Non-Functional Constraints

- 同一 file に対し CLI と tool が同一 count を返すことを test で保証
- tokenizer instance は module scope で 1 つ（rank table の再ロードを避ける）
- version 定数と `package.json` の js-tiktoken version の一致を test で保証

## Risks and Trade-offs

- lossy utf8 decode は binary file に無意味な count を返し得るが、測定値として
  決定的であり contract は一貫する。呼び出し側が file 種別を判定する
- `o200k_base` 固定は provider billing との乖離を許容するが、threshold は同じ
  metric で決めるため問題にならない

## Verification Notes

- [ ] CLI と tool が同一 fixture で同一 count を返す
- [ ] `../`・絶対 path・境界外 symlink が拒否される
- [ ] 空 file・不存在 file・binary file の振る舞いが test される
- [ ] `count-tokens` が model runtime/discovery を起動しない

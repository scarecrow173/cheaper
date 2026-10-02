---
id: "TASK-034z52uhywljSnrFmqDWjE"
type: "task"
status: "todo"
title: "port routing-policy.js to packages/cli/src/hook/routing-policy.ts"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# port routing-policy.js to packages/cli/src/hook/routing-policy.ts

## Work

`packages/apm/.apm/hooks/routing-policy.js`（393 行）を
`packages/cli/src/hook/routing-policy.ts` に挙動不変で TS 移植する。

- workspace lexical containment・`statSync().isFile()` gate・byte fast
  path（`CHEAPER_READ_TOKEN_THRESHOLD`）の順序を verbatim 維持
- `count-tokens` 子 process spawn を in-process `countFileTokens` に
  置き換え（count-tokens command と同一経路: canonicalize workspace
  root → policy load → maxBytes）。`countMaxBytes` を cli.ts から
  export して共有
- `FileTooLargeError` → fail-closed deny（文言は JS 版と同一）、
  `WorkspaceEscapeError`・ENOENT・その他全 error → allow
- measured path 全体を `AbortSignal.timeout(30_000)` で囲み旧
  subprocess cap を復元（timeout → allow）
- count/policy 系 import は byte fast path 通過後の dynamic import
  に限定（tokenizer eager init を避ける）
- shell tokenizer・`extractReadCandidates`・`CWD_CHANGING_COMMANDS`・
  deny 文言生成をそのまま移植
- symlinked cwd の quirk（raw cwd resolve → canonical root 不一致 →
  `WorkspaceEscapeError` → allow）を parity として再現。`relPosix`
  は raw cwd 基準の `path.relative`

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md
- docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md

## Dependencies

- なし（最初の task）

## Affected Files

- `packages/cli/src/hook/routing-policy.ts`（新規）
- `packages/cli/src/cli.ts`（`countMaxBytes` の export）
- `packages/cli/test/`（移植 unit test）

## Done When

- [ ] `evaluate(request)` が canonical request（file-read / shell）を
      受理し JS 版と同一の verdict を返す
- [ ] `CHEAPER_READ_TOKEN_THRESHOLD` の解釈（default 8192・正の整数
      以外は default）が一致
- [ ] `FILE_TOO_LARGE` が fail-closed deny、infra error が fail-open
      allow になる
- [ ] measured path が `AbortSignal.timeout(30_000)` で囲まれている
- [ ] `pnpm bundle` 再生成済み（freshness 維持）

## Verification

- [ ] 移植 unit test（tokenizer・read candidate・file-read verdict）
      が JS 版と同一 fixture で pass
- [ ] `mise exec -- pnpm -F @cheaper/cli test` 緑
- [ ] `pnpm bundle` 再生成後 `mise run check` の freshness diff が
      clean

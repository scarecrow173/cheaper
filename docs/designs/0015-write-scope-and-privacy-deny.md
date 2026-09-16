---
id: "DESIGN-0015"
type: "design"
status: "approved"
title: "Write-scope and privacy deny enforcement design"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0010-write-scope-and-privacy-deny.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0010-write-scope-and-privacy-deny.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0011-writer-worker-contracts.md"
    - "../designs/0002-workspace-filesystem-boundary-sandboxed-tools.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-0015: Write-scope and privacy deny enforcement design

## 構成

```
core
  workspace/deny-paths.ts   DEFAULT_DENY_PATHS + createPathDenylist(patterns)
                            → { isDenied(displayPath): boolean }
  worker/validate.ts        cheaper.writeScope?: string を受理
  policy/schema.ts          filesystem.denyPaths?: string[] を受理・merge
runtime-pi
  run.ts                    writeScope field → input から allowlist を抽出・正規化
  tools/operations.ts       resolve() に deny check、write 系に scope check
  tools/walker.ts           walk 結果・dir 潜行を deny filter
  tools/registry.ts         opts に denylist/writeScope を plumb
```

## Write scope

### frontmatter

```yaml
cheaper:
  writeScope: targets   # input 内の field 名
```

`WorkerDefinition.cheaper.writeScope?: string`。`CHEAPER_KEYS` に追加。

### allowlist 抽出（run.ts）

`worker.cheaper.writeScope` があれば `deps.input` の当該 field を読む:

- `undefined` / array でない → `INVALID_INPUT`（fail closed）
- item は string、または `{ path: string }` の object。それ以外 → `INVALID_INPUT`
- 各 path を `resolveWorkspacePath(root, join(root, p), symlinkPolicy)` で
  正規化し display path の `Set` にする（in-workspace 限定に閉じる効果もある）

### 強制（operations.ts）

- `resolveForWrite(abs)`: boundary resolve 後、display が scope Set に
  含まれなければ `SCOPE_DENIED_MESSAGE`（"Access denied: path is outside
  the write scope"）で throw
- `atomicWrite`: target path は `resolveForWrite`、同 dir tmp file は plain
  `resolve`（target 認可済みの実装詳細のため）
- `write.mkdir`: dir の display が `"."` か、いずれかの scope path の
  祖先 dir（`scope.startsWith(dir + "/")`）でなければ deny
- `edit.writeFile` は `atomicWrite` 経由で自動的に scope 適用。
  `edit.readFile`/`access` は scope 対象外（read は制限しない）

## Privacy deny

### pattern → matcher（core）

`createPathDenylist(patterns)`:

- pattern を `/` で segment 化。`**` は 0+ segment、その他 segment 内の
  `*`/`?` を regex に展開
- 先頭 `/` なしは `(?:^|.*/)` prefix を付けて任意深さ match
- 末尾 `/**` は `(?:/[^/]+)*$` ではなく dir 自体にも match するよう
  `(?:/.*)?$` 相当に変換
- `DEFAULT_DENY_PATHS`（built-in、policy で解除不可）:

```
.env, .env.*, *.pem, *.key, *.p12, *.pfx, *.keystore,
id_rsa, id_ed25519, id_ecdsa, id_dsa,
.netrc, .npmrc, .pypirc,
credentials.json, secrets.json, secrets.yaml, secrets.yml,
.ssh/**, .aws/**, .gnupg/**
```

### policy

`filesystem.denyPaths: string[]`（optional、default `[]`）。
merge は array replace（既存規則）。effective patterns =
`DEFAULT_DENY_PATHS + policy.filesystem.denyPaths`。

### 適用点

| 経路 | 動作 |
|---|---|
| operations.resolve()（read/write/edit/ls の path 引数） | `DENIED_MESSAGE` throw → tool error |
| walkFiles（grep/find） | deny file を yield しない・deny dir に潜らない |
| ls.readdir | deny entry を結果から除外 |
| count_tokens tool | deny → tool error |
| CLI count-tokens | 対象外（parent 側 utility） |

deny error message は boundary とは別の `"Access denied: path is denied by
policy"`。`isAccessDeniedError` を両 message に対応させ rejectedToolCalls
へ計上する。

## エラー・meta

- scope/deny の拒否は tool error result（model が retry/修正可能）。
  run は失敗しない
- `meta.rejectedToolCalls` で拒否回数を観測可能

## 非対象

- journal 側の事後検知強化（現状 `meta.changes` で既に観測可能）
- secret の content scanning

---
id: "SPEC-0010"
type: "spec"
status: "proposed"
title: "Write-scope enforcement and in-workspace privacy deny list"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0006-code-writer-and-doc-writer-built-in-workers.md"
    - "./0007-canonical-token-counter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0015-write-scope-and-privacy-deny.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0004-v2-productization-initiative-briefing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# SPEC-0010: Write-scope enforcement and in-workspace privacy deny list

## 背景

writer worker（code-writer / doc-writer）の `targets` 外への書き込みは現在
prompt 命令にのみ依存している（DESIGN-0011 の known limitation）。
また workspace 内の credential 系 file（`.env` 等）を cheap model が read し、
内容が output へ混入する経路を塞ぐ機構がない。

## 要求

### Write scope

1. Worker frontmatter が `cheaper.writeScope: <inputField>` を宣言できる。
   input の当該 field は string array または `{path}` を持つ object array。
2. 宣言された worker の実行で、write/edit/mkdir による mutation は
   scope path（= 宣言 field の path 群を workspace 相対に正規化したもの）
   にのみ許可される。scope 外の write/edit は tool error として拒否され、
   `meta.rejectedToolCalls` に計上される。
3. `mkdir` は scope path の祖先 dir に限り許可される（write tool が
   parent dir を先に作るため）。
4. 宣言 field が input に存在しない・array でない場合は `INVALID_INPUT` で
   run 開始前に失敗する（fail closed）。
5. `writeScope` 未宣言の worker は従来どおり workspace 内への自由な write が
   可能（bulk-reader 等は対象外）。
6. scope は mechanism による強制であり、prompt の禁止事項は defense in depth
   として残す。

### Privacy deny

1. core に built-in の deny pattern 集合（credential/key/secret 系:
   `.env*`, `*.pem`, `*.key`, `id_rsa` 等）を定義し、policy の
   `filesystem.denyPaths` で pattern を追加できる。built-in は常に有効で
   policy からは外せない。
2. deny pattern に match する workspace 内 path への直接 access
   （read/write/edit/ls の path 引数、`count_tokens`）は tool error で拒否。
3. grep/find の walk 結果および ls の dir listing からは deny path を
   除外する（存在の秘匿）。deny dir 配下は walk しない。
4. deny は workspace boundary check とは独立の第 2 層で、workspace 内
   path にのみ意味を持つ。
5. CLI `count-tokens` は deny 対象外（parent 側 utility。hook が deny を
   受けると fail-open して read が進んでしまうため）。

### Pattern 構文

- セグメント内 wildcard `*`・`?`、複数セグメント `**`。
- 先頭 `/` なしは tree 内任意の深さに match（implicit `**/`）。
- 先頭 `/` は workspace root anchored。
- 末尾 `/**` は dir 自体にも match（`.ssh/**` は `.ssh` dir も拒否）。

## 非要求

- deny path の policy 側からの除去・negation（`!`）
- file 内容ベースの secret 検出（pattern は path のみ）
- hook 側での deny 強制（worker tool 層の責務）

## Acceptance Criteria

- [ ] code-writer/doc-writer に `writeScope: targets` が宣言され、targets 外の
      write が e2e で tool error になる
- [ ] `.env` 等の built-in deny path が read/grep/find/ls/count_tokens で
      拒否または除外される
- [ ] `filesystem.denyPaths` で custom pattern を追加できる
- [ ] 全既存 test が pass し、新規 test が scope/deny の各経路を cover

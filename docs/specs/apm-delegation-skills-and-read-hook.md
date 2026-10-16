---
id: "SPEC-034r05iHK8bA7gexEML9oZ"
type: "spec"
status: "implemented"
title: "APM delegation skills and pre-read delegation hook"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/windsurf-cascade-hooks-and-apm-hook-deployment.md"
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
    - "./canonical-token-counter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/canonical-tokenizer-js-tiktoken.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "./case-insensitive-native-tools-and-devin-registration.md"
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/windsurf-cascade-hooks-and-apm-hook-deployment.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM delegation skills and pre-read delegation hook

## What Should Be Built

本 repo を APM package として、parent Coding Agent へ cheaper の委譲経路を
提供する 4 つの skill と、file read 前に token count を測定して large file
を bulk-reader へ redirect する `pre_read_code` hook を実装する。

- `.apm/skills/cheaper-usage/` — cheaper CLI の呼び方（command、stdin
  contract、envelope、exit code、provider/model 設定）を教える usage skill
- `.apm/skills/cheaper-bulk-read/` — いつ・なぜ bulk-reader へ委譲するか、
  input の組み立て方、圧縮結果の消費方法
- `.apm/skills/cheaper-code-write/` — code-writer への機械的 code 変更の
  委譲方法
- `.apm/skills/cheaper-doc-write/` — doc-writer への機械的 doc 作成・編集の
  委譲方法
- `.apm/hooks/hooks.json` + `pre-read-code.js` — Windsurf `pre_read_code`
  で `count-tokens` を実行し、threshold 超過時に exit 2 + stderr guidance
  で read を block して bulk-read へ誘導する

## Why Now

token counting（SPEC-034r05iHK8bA7gdbCbWupr）が入り、delegation の前段判定が可能になった。
cheaper の価値は「large context が parent に入る前に止める」ことなので、
その interception point を最初に実装する。

## Users and Value

- parent agent（Windsurf Cascade など）: skill を読んで cheaper を正しく
  呼び、hook により large file の direct read を未然に防ぐ
- ユーザー: frontier model context/cost の削減

## Scope

### In Scope

- 4 skill の SKILL.md（`name`/`description` frontmatter + 本文、英語）
- `.apm/hooks/hooks.json`（windsurf native `pre_read_code` event）
- `.apm/hooks/pre-read-code.js`（zero-dep CommonJS、node で実行）
- threshold: `CHEAPER_READ_TOKEN_THRESHOLD` env で上書き可能な
  provisional default 8192 tokens
- CLI 解決: `CHEAPER_CLI` env → `cheaper` on PATH の順
- hook script の e2e test（exit code / stderr contract）

### Out of Scope

- threshold の実測根拠と確定値（Phase 6 benchmark）
- directory 再帰 read の token 集計（初版は pass-through）
- `pre_write_code` / `pre_run_command` 等の他 event
- APM pack 後の CLI 同梱と `${PLUGIN_ROOT}` 経由の CLI 解決（Phase 4）
- non-windsurf target での hook 動作検証（Phase 5 以降）
- secret/sensitive file の read deny（privacy は別 phase）

## Requirements

- hook script は依存ゼロの Node.js（APM が deploy する sidecar は commonjs
  default）で書き、stdin の hook JSON を読む
- `agent_action_name !== "pre_read_code"`、file 非存在、directory、
  count 失敗はすべて exit 0（fail-open。通常 read を壊さない）
- tokens > threshold のみ exit 2 + stderr に「直接読むな、bulk-read へ
  委譲せよ」の guidance を出す。message は agent が次の行動を取れる
  具体的 command を含む
- skill 本文は英語（skill は機械処理対象の技術仕様。AGENTS.md の言語規約
  に従う）
- skills は相互を名前参照し、CLI 細部は `cheaper-usage` に集約して
  重複を避ける

## Acceptance Criteria

- `apm install --target windsurf` で `.windsurf/hooks.json` に
  `pre_read_code` entry と script が deploy される（DISC-034r05iHy2gdnp9dPPZ28N の実測形式）
- 閾値超過 file の pre_read_code で exit 2 + stderr guidance、
  閾値以下・非 file・count 失敗で exit 0
- 4 skill が APM skill layout（`.apm/skills/<name>/SKILL.md`）として
  deploy 可能
- hook script の test が green

## Deferred Design Concerns

- threshold 確定値・benchmark 方法論（Phase 6）
- `${PLUGIN_ROOT}` 同梱 CLI による自己完結 hook（Phase 4 の pack 設計と
  一体で決める）
- privacy deny rules を同 hook に載せるか別 hook にするか

## Sources and Relations

- DISC-034r05iHy2gdnp9dPPZ28N: hook contract / APM deploy 形式の実測
- SPEC-034r05iHK8bA7gdbCbWupr: count-tokens CLI（hook が呼ぶ判定器）
- ADR-034r05iHrZUE6W3U1rbJq4: canonical tokenizer

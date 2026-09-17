---
id: "SPEC-0020"
type: "spec"
status: "implemented"
title: "Copilot per-target pre-read hook adapter"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/10"
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/0025-copilot-pre-read-hook-adapter.md"
  depends-on:
    - "./0013-cross-agent-distribution-verification.md"
    - "./0016-separate-dev-apm-project-from-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../designs/0018-cross-agent-distribution-verification.md"
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Copilot per-target pre-read hook adapter

## What Should Be Built

GitHub Copilot target 向けに、native `preToolUse` event（`toolName ==
"view"` で file read を intercept）で動作する pre-read blocking hook を
提供する。`toolArgs` の型揺れ（`string | object`）を normalize し、
copilot の fail-closed 意味論（非 0 exit = 全 deny）の下で意図した
block 以外で tool call を止めない契約を定める。

## Why Now

ADR-0016 は非 windsurf target では verbatim `pre_read_code` を no-op
として維持し、per-target adapter を follow-up chain に defer した。
DISC-0009 の contract matrix により、copilot は (a) native event・
block 表現が判明済み、(b) fail-closed 意味論の反転という最大の設計
リスクを持ち、(c) `.json` asset 欠落という deploy 破損も確認済みで
ある。issue #10 がこの adapter の着手を指示する。

## Users and Value

- Copilot 利用者: 巨大 file の直接 read が agent の context を
  圧迫する前に、bulk-read 委譲へ誘導される（windsurf と同等の
  safety net）。
- 開発者: per-target adapter の最初の実装例として、残りの target
  （claude・cursor）への pattern が確立される。

## Scope

### In Scope

- copilot 向け adapter script（`preToolUse` stdin payload の処理、
  `toolArgs` normalize、block 出力、fail-open 維持）。
- `hooks.json` への copilot event 登録（verbatim deploy 経由）。
- adapter の unit 的検証（stdin payload 注入）と deploy/e2e 検証
  （generated config・実ファイルに対する block/allow）。
- `.json` asset 欠落への対処方針の決定と文書化。
- contract matrix（DISC-0009）の copilot 行の更新。

### Out of Scope

- claude・cursor 向け adapter（同一 pattern で後続 issue/chain に分離）。
- codex（upstream が read event を emit しないため対象外、DISC-0009）。
- `.json` stripping の upstream 修正自体、および asset 名変更・
  schema 埋め込み等の deploy 形態変更（方針決定は scope 内）。
- 実 copilot binary を使った runtime loader 検証（実機 E2E は
  引き続き open item。generated config + payload 契約での検証に留める）。

## Requirements

1. **event 適合**: adapter は copilot の `preToolUse` stdin payload
   （`toolName` / `toolArgs`）を解釈し、`toolName == "view"` のみを
   file read として扱う。それ以外の tool・event では素通しする。
2. **toolArgs normalize**: `toolArgs` が object・JSON 文字列のどちらで
   渡されても file path を取り出せる。string を `JSON.parse` 前提に
   しない。path の key は doc 揺れに備え `path` / `file_path` /
   `filePath` を許容する。
3. **fail-closed 安全**: copilot の command hook は非 0 exit がすべて
   deny になるため、adapter は **いかなる経路でも非 0 exit しない**。
   意図した block は stdout の
   `{"permissionDecision": "deny", "permissionDecisionReason": ...}`
   JSON + exit 0 で表現する。内部 error・想定外入力・CLI 不在は
   exit 0 で素通し（windsurf hook の fail-open contract と同じ意味論）。
4. **block 条件の parity**: windsurf hook と同じ判定を使う — workspace
   内の file、`CHEAPER_READ_TOKEN_THRESHOLD`（default 8192）超過、
   または `count-tokens` の `FILE_TOO_LARGE` verdict。byte 数が
   threshold 以下なら CLI を spawn しない fast path も同じ。
5. **guidance の parity**: deny 理由には測定結果と bulk-read 委譲
   guidance、workspace-relative path を含め、実行可能な shell command
   を埋め込まない（windsurf hook と同じ non-executable guidance 契約）。
6. **deploy 形態**: adapter は APM package の hook bundle に含め、
   `hooks.json` の event 宣言経由で copilot の generated config に
   登録される。windsurf 側の `pre_read_code` 登録とその動作を
   変えない。non-copilot target では adapter 登録が no-op である
   ことを確認する（ADR-0016 の verbatim + no-op pattern）。
7. **`.json` asset 欠落への対処方針**: upstream 修正待ち +
   expected-failure assert を drift signal として維持するか、
   `.json` に依存しない deploy 形態へ変えるかを ADR で決定する。
   adapter 自体が `.json` asset に依存しないこと（`count-tokens` は
   `.json` なしで動作済み）。

## Acceptance Criteria

- [ ] `{toolName: "view", toolArgs}` の payload で、threshold 超過の
      file に対し stdout の deny JSON + exit 0 が返る。`toolArgs` は
      object・JSON 文字列の両方で同じ結果になる。
- [ ] `toolName != "view"`・非 JSON stdin・欠落 field・workspace 外
      path・存在しない file・CLI 不在の全経路で exit 0（非 0 exit が
      一度も発生しないことを assert する test がある）。
- [ ] `apm install --target copilot` の generated config に adapter の
      `preToolUse` 登録が含まれ、`.json` 欠落 assert（drift signal）が
      維持される。
- [ ] windsurf の既存 hook test 全件が pass し、non-copilot target の
      generated config でも adapter 登録は no-op event として扱われる。
- [ ] `.json` 欠落への対処方針が ADR・matrix・consumer 向け文書に
      反映されている。
- [ ] `pnpm typecheck`・`pnpm lint`・test suite 全件が通る。

## Sources and Relations

- Issue #10 / DISC-0009: contract matrix と本仕様の起点。
- ADR-0016: verbatim `pre_read_code` no-op 維持と follow-up chain の
  決定。本仕様はその follow-up。
- SPEC-0013 / DESIGN-0018: cross-agent 配布検証と `.json` strip の
  expected-failure 記録。
- DESIGN-0013: hook 配布・`pre_read_code` 登録の現行契約。
- SPEC-0016 / DESIGN-0021: hook bundle は `packages/apm/.apm/hooks/`。

block 判定の詳細数値（threshold・byte cap）は既存 hook と共有する。
`.json` 欠落への対処方針・adapter の deploy 形態詳細は後続 ADR /
design で確定する。

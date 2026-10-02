---
id: "IMPL-034z57jQrqVgbebkytTkwt"
type: "impl"
status: "completed"
title: "hook contract tests and adapter e2e port"
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
    - "docs/tasks/add-hook-contract-tests-and-port-existing-adapter-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# hook contract tests and adapter e2e port

## Summary

`cheaper hook` の target 別 contract test を新設し、JS adapter を
spawn していた既存 e2e 3 file を `node bin/cheaper.js hook …`
呼び出しに移植した。SEA executable での Node 非依存 hook 動作も
実機確認した。

## Context

- task: docs/tasks/add-hook-contract-tests-and-port-existing-adapter-e2e.md
- test-spec: docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md

## Implementation

### Changes

- `packages/cli/test/hook/hook-contract.test.ts`（新規）: TSPEC の
  Rules/Examples を網羅 — 引数 parse 失敗・未知 target/event、
  byte fast path（8192B 境界）、token deny、FILE_TOO_LARGE、
  outside/missing/dir/FIFO（POSIX のみ）、malformed stdin、
  policy 破損 fail-open、symlinked cwd quirk、usage.jsonl 非記録、
  bundled `cheaper.mjs` parity、cursor の常時 schema-valid JSON、
  copilot の非 0 exit 禁止、dual-schema allow
- `test/e2e/pre-read-hook.test.ts`・`copilot-pre-read-hook.test.ts`・
  `cross-agent-routing-adapters.test.ts`: spawn target を
  `node <adapter>.js <cli>` から `node <cheaper.js|cheaper.mjs> hook
  <event> --target <t>` に移植。`CHEAPER_CLI`/argv[2] 依存を除去
- `test/e2e/apm-install.test.ts`・`delegation-loop.test.ts`: hook
  runtime smoke を bundled CLI の `hook` subcommand 呼び出しに変更
  （hooks.json 内容の assert は task 4 で file 切替と同時に更新）

### Decisions During Implementation

- **「CLI 不在」fail-open の代替**: 子 process 経路が消えたため、
  infrastructure failure 系は破損した user policy.yaml
  （`CHEAPER_CONFIG_DIR` 配下）を measured path で踏ませる形に
  変換（loadPolicy throw → catch → allow を経る）
- **CLI spawn marker test の置換**: `cheaper.mjs` を spawn させない
  fake CLI marker は in-process 化で意味を持たないため、byte fast
  path の allow を破損 policy と組み合わせた形に変更（fast path は
  policy を load しないため exit 0 が維持される）
- **spawn 直列 loop の timeout**: `node bin/cheaper.js` の起動が
  ~2s/回のため、9 case loop が vitest 既定 20s を超過。複数 spawn
  を行う test に `120_000` timeout を設定

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `test/hook/` : 30 pass / 1 skipped（FIFO は win32 で skip）
- ported e2e 3 file: 55 pass
- `dist/sea/cheaper-windows-x64.exe hook …`: PATH から Node を除去
  した環境で windsurf deny exit 2・cursor allow JSON を確認
- `pnpm bundle` 再生成済み

## Risks

- 逐次 spawn の e2e は suite 時間を引き延ばす（3 file で ~175s）。
  CI budget に余裕がある現状は許容
- `testTimeout` 延長は経路の遅さを覆い隠す面がある。hook 起動時間
  自体の改善（tokenizer lazy load 等）は別課題

## Follow-ups

- task 4: launcher deploy 実測 + `hooks.json` の executable 切替
  （apm-install 系の hooks.json assert を同時に更新）

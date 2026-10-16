---
id: "SPEC-034r05iHK8bA7gjwZTji8t"
type: "spec"
status: "implemented"
title: "Huge-file token count guard with fail-closed hook blocking"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
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
    - "../adr/default-large-read-threshold.md"
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/huge-file-token-count-guard-and-fail-closed-hook-behavior.md"
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

# Huge-file token count guard with fail-closed hook blocking

## What Should Be Built

`countFileTokens` に byte size 上限（byte cap）を導入し、上限超過の
file を token count せず専用の typed error で失敗させる。CLI
`count-tokens` はその error を専用 error code の envelope に map し、
`pre-read-code.js` hook はそれを閾値超過と同等に扱って exit 2 で
block + bulk-read 委譲 guidance を返す（fail-closed）。

- core: `countFileTokens` が read 前に `stat` で size を確認し、
  `maxBytes` 超過なら typed error（例: `FileTooLargeError`）を投げる。
  `maxBytes` は option 注入で default は 16MB
- CLI: `count-tokens` が cap error を専用 error code（envelope）に map。
  `CHEAPER_COUNT_MAX_BYTES` env で cap を上書き可能（CLI/hook 経路のみ。
  runtime `count_tokens` tool は core default のまま）
- hook: core が決定的に `FILE_TOO_LARGE` と判定した巨大 file、および
  正常 count で threshold 超過の file を block（exit 2 + 委譲
  guidance）として扱う。spawn error / timeout / stdout parse 不能 /
  `FILE_TOO_LARGE` 以外の `ok:false` などの infra 障害は従来どおり
  fail-open（exit 0）を維持する
- bytes ≤ token threshold の file は `tokens ≤ bytes` の性質から encode
  せず即座に「閾値以下」と判定できる（実装の最適化余地として記録）

## Why Now

v2 productization の review で特定された後続課題。現状 `countFileTokens`
は file 全体をメモリに展開して encode するため、巨大 file で
OOM/timeout する。hook は測定失敗をすべて fail-open（exit 0）で扱う
ため、**最も block すべき file ほど測定に失敗し parent が直接 read
する** 構造的な穴がある。

## Users and Value

- parent agent: 巨大 file の直接 read による context 破壊を防ぐ
- consumer: 測定不能な巨大 file は安全側に倒れる（fail-closed）。
  一方で CLI の spawn error / timeout 等の infra 障害では従来どおり
  fail-open を維持し、通常 read を阻害しない

## Scope

### In Scope

- core `countFileTokens` の byte cap + typed error
- CLI `count-tokens` の error code map と `CHEAPER_COUNT_MAX_BYTES` env
- `pre-read-code.js` hook の fail-closed 扱い（cap error → block）
- 上記の unit / e2e test

### Out of Scope

- bulk-reader 側の巨大 file 読み込み対策（別 chain で検討）
- streaming / sampling による近似 token count（canonical deterministic
  count 契約を維持するため採用しない）
- hook の spawn timeout 値変更（現行 30s のまま）

## Requirements

- bytes > `maxBytes`（default 16MB）の file は read/encode せず typed
  error で失敗する。CLI は専用 error code を envelope で返す
- `CHEAPER_COUNT_MAX_BYTES` に正の整数を設定すると cap が上書きされる。
  不正値は default にフォールバック。この env は CLI `count-tokens`
  （hook が spawn する経路）にのみ適用され、runtime `count_tokens`
  tool は core default 16MB を使い続ける
- hook は cap error を検出したら閾値超過と同じ exit 2 block + 委譲
  guidance を出す。guidance には workspace-relative POSIX path を含める
- hook の fail-closed は「`FILE_TOO_LARGE` または正常 count で
  threshold 超過」に限定される。file 不在・workspace 外・input 異常、
  および CLI spawn error / timeout / parse 不能 / その他の infra
  error は fail-open（既存契約維持）であり、infra 障害で存在する
  file が block されることはない
- cap 未満の file の count 結果は一切変わらない（canonical count 契約
  維持）

## Acceptance Criteria

- [ ] `maxBytes` 超過 file に対し `count-tokens` が専用 error code の
      envelope で非0 exit する（OOM/timeout しない）
- [ ] `CHEAPER_COUNT_MAX_BYTES` による cap 上書き・不正値フォールバック
      が test で検証される
- [ ] cap error を受けた hook が exit 2 + bulk-read 委譲 guidance を返す
      e2e が存在する
- [ ] cap 未満 file の count-tokens 出力（tokens/bytes/tokenizer）が
      従来と一致する parity test が pass する
- [ ] `count_tokens` tool 経路でも同じ guard が効く（同一 core 実装）

## Sources and Relations

- DISC-034r05iHy2gdnpcXzmUKaV: failure mode の調査・alternatives・確定方針
- SPEC-034r05iHK8bA7gdbCbWupr: canonical token counter 契約（本 cap は count の決定性を
  変更しない）
- ADR-034r05iHrZUE6W4XOqUBih: threshold env 上書きパターン（cap env も同型）

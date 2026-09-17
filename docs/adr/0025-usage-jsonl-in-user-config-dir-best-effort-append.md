---
id: "ADR-0025"
type: "adr"
status: "accepted"
title: "delegation 計測は userConfigDir の usage.jsonl への best-effort append とする"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0022-delegation-usage-measurement-log.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/0027-delegation-usage-measurement-log.md"
  depends-on:
    - "./0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0022-delegation-usage-measurement-log.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 25. delegation 計測は userConfigDir の usage.jsonl への best-effort append とする

## Context and Problem Statement

SPEC-0022 は delegation ごとの usage 継続記録を要求する。決める
べきこと: (a) 記録の置き場（workspace `.cheaper/` vs user
config dir vs opt-in flag）、(b) 記録対象（run のみ vs
count-tokens も）、(c) 書き込み失敗時の semantics、
(d) 集計機能を入れるか。

## Decision Drivers

- Issue #13: 「計測の置き場を決める（JSONL log / opt-in summary
  等。YAGNI に従い最小構成から）」「delegation 単位で記録し、
  parent 側で参照・集計できる形に」「count_tokens との整合」を
  明示する。
- `.cheaper/` は consumer の workspace 配下 — 無断で file を
  生成すると consumer repo の git status を汚す。
- `userConfigDir` は `CHEAPER_CONFIG_DIR` で隔離可能で、
  fake-stream e2e・real-provider e2e が既に tmpdir に向けている
  — 記録経路の test 隔離がそのまま使える。
- envelope contract: stdout は envelope 1 JSON のみ。計測の
  失敗がこの contract を壊してはいけない（hook 経由の caller が
  stdout を parse する）。

## Considered Options

### 記録の置き場

- **A: workspace `.cheaper/usage.jsonl`**。project 単位で自然に
  scoped されるが、consumer の repo に未許可で file を生成する
  （git status 汚染・ignore 未設定の repo で untracked が出る）。
- **B: `<userConfigDir>/usage.jsonl`**（採用）。consumer repo を
  汚さず、config SoT と同じ canonical 場所に置ける。record に
  `root`（workspace path）を含めれば project 単位の集計も可能。
  `CHEAPER_CONFIG_DIR` で test 隔離が既存のまま機能する。
- **C: opt-in env flag / config option**。「continuous」計測の
  趣旨に反し、flag を知らなければ永久に未計測になる。always-on
  の 1 行 append はコストが無視できるほど小さい。

### 記録対象

- **i: run envelope のみ**。最小だが、count-tokens の事前見積が
  記録されず実測との突合ができない。
- **ii: run + count-tokens**（採用）。issue が count_tokens との
  整合を「更に有用」と求めており、同じ file に `kind` 違いで
  記録するだけで事前見積 vs 実測の比較が可能になる。追加コスト
  は record builder の分岐のみ。

### 書き込み失敗時

- **α: 失敗を握り潰す**。最も安全だが計測が壊れても気づけない。
- **β: stderr diagnostic + exit/stdout 不変**（採用）。stdout
  contract は保持しつつ、`--quiet` でない限り 1 行の診断を出す。
  既存の `model:` stderr 行と同じ扱い。
- **γ: error に昇格**。計測失敗で worker 実行を fail にするのは
  本末転倒。

### 集計機能

- **δ: `usage-summary` コマンドを同時実装**。便利だが YAGNI
  違反 — JSONL 自体が parent の読める形であり、集計要件が具体化
  してから追加すべき（follow-up 候補として記録）。
- **ε: JSONL のみ**（採用）。

## Decision Outcome

- **置き場**: `resolveConfigDirs` が返す `userConfigDir` 配下の
  `usage.jsonl` に append-only で記録する。workspace・`.cheaper/`
  には何も書かない。
- **record**: 1 行 JSON。`schemaVersion:1`・`ts`(ISO)・`kind`
  （`"run"|"count"`）・`worker`・`root`・`ok`・`durationMs` を
  共通必須。run は `model`・`usage`・error 時 `error.code`、
  count は `result`（path/tokens/bytes）を含む。
- **記録対象**: `run` 系（success + meta を持つ error envelope）
  と `count-tokens` の success。auth・help・version は対象外。
- **best-effort**: 書き込み失敗は exit code・stdout を変えず、
  `--quiet` 以外では stderr に 1 行 diagnostic。
- **append 実装**: `mkdir -p` + 1 行単位の `appendFile`。複数
  process の同時 append は行単位では実害が小さいとして許容
  （risk として明記）。

## Consequences

- CLI の実行ごとに userConfigDir 配下へ file I/O が増える（1 行
  append、無視できるコスト）。`userConfigDir` が存在しなくても
  `mkdir -p` で作る — config 未作成の環境でも計測は機能する。
- `usage.jsonl` は無限に伸びる（rotation なし）。1 行あたり数百
  byte・1 run 1 行なので短期では問題にならないが、長期運用では
  rotation/集計の follow-up が必要になり得る。
- test では `CHEAPER_CONFIG_DIR` を tmpdir に向けるため、実 user
  の log を汚さない。real-provider e2e でも同じ seam で隔離済み。
- JSONL の schema は `schemaVersion` で将来の拡張に備える
  （summary 機能追加時に consumer が識別可能）。

## Confirmation

- fake-stream e2e: `CHEAPER_CONFIG_DIR` 配下に `kind:"run"` の
  record が append されることを assert。
- count-tokens e2e: `kind:"count"` の record が append される
  ことを assert。
- real-provider e2e: record の `model.provider==="ollama"`・
  `usage.input > 0` を assert。
- 書き込み不可 dir（permission 拒否 or 無効 path）でも exit
  code・stdout が不変であることを unit test で assert。

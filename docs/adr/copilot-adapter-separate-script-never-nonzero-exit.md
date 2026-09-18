---
id: "ADR-034r05iHrZUE6WizQ4LgPa"
type: "adr"
status: "accepted"
title: "copilot adapter は self-contained script で stdout deny JSON + 常に exit 0 とし、.json gap は upstream 待ちとする"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/copilot-pre-read-hook-adapter.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/copilot-pre-read-hook-adapter.md"
  depends-on:
    - "./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/apm-skills-and-pre-read-hook.md"
    - "../designs/cross-agent-distribution-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/copilot-pre-read-hook-adapter.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
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

# 23. copilot adapter は self-contained script で stdout deny JSON + 常に exit 0 とし、.json gap は upstream 待ちとする

## Context and Problem Statement

SPEC-034r05iHK8bA7gsvijtkou は copilot 向け pre-read blocking hook を要求する。DISC-034r05iHy2gdnpdai9KGpW
の検証済み contract: event `preToolUse`、stdin `{toolName, toolArgs}`
（`toolArgs` は `string | object` で doc 揺れあり）、`toolName ==
"view"` で read intercept、block は stdout
`{"permissionDecision": "deny", ...}`。command hook は **fail-closed**
（非 0 exit = 全 deny、timeout は fail-open）。加えて `apm install
--target copilot` は `.json` asset を全て欠落させる（version.json /
package.json / worker schemas）。

決めるべきこと: (a) adapter の形態（単一 script 分岐 vs per-target
script）、(b) fail-closed 意味論での block/素通しの表現、(c) `hooks.json`
の event 宣言方法、(d) `.json` asset 欠落への対処。

## Decision Drivers

- issue #10 は「per-target risk isolation を保つ incremental delivery」
  を要求する。
- fail-closed 反転は最大の設計リスク（DISC-034r05iHy2gdnpdai9KGpW §Copilot の
  fail-closed 意味論反転）: 内部 error や想定外 payload で非 0 exit
  すると、正常な read がすべて deny される。
- APM は hooks.json の event 名を verbatim で全 target config に
  書き込む（変換しない — DISC-034r05iHy2gdnpdai9KGpW 確認済み）。native でない event
  名は no-op（ADR-034r05iHrZUE6W9RDF0V73 の前提）。
- `.json` gap の影響範囲: `count-tokens`（adapter が使う唯一の機能）は
  `.json` なしで動作確認済み。`--version` と file-schema worker は
  copilot deploy で壊れる。
- `.json` gap 回避策（asset 名変更・schema 埋め込み）は bundle/worker
  契約への侵襲が大きく、upstream 修正を待つ方が合理的。

## Considered Options

### adapter 形態

- **A: 単一 script が payload shape を判別**（windsurf と copilot を
  1 file で処理）。deploy は単純だが、payload 判別ロジックのバグが
  両 target に波及し、risk isolation に反する。
- **B: per-target self-contained script**（採用）。
  `pre-read-code-copilot.js` を `pre-read-code.js` と並置。共有
  module を作らず各 adapter を自己完結させ、片側の変更・破損が他に
  波及しない。

### `.json` asset 欠落への対処

- **i: upstream 修正待ち + drift-signal test 維持**（採用）。adapter は
  `.json` を必要としない（`count-tokens` のみ使用）。file-schema
  worker が copilot で不動である点は matrix・consumer 文書に既知の
  制限として明記する。
- **ii: `.json` に依存しない deploy 形態へ変更**（schema を bundle
  .mjs へ埋め込む、asset を非 `.json` 名に変える等）。upstream bug を
  側面から恒久回避するが、worker definition 契約か bundle 生成への
  侵襲が大きく、upstream 修正と衝突した際に legacy 負債になる。

## Decision Outcome

- **adapter は `pre-read-code-copilot.js`**（`packages/apm/.apm/hooks/`
  内の self-contained file。共有 module を導入しない — asset 欠落や
  load 失敗が全 adapter に波及する経路を作らないため）。
- **block は stdout deny JSON + exit 0**。`{"permissionDecision":
  "deny", "permissionDecisionReason": "<guidance>"}` を stdout に
  書いて exit 0 する。deny 理由には測定結果・bulk-read 委譲
  guidance・workspace-relative path を含める（windsurf の guidance
  parity、non-executable）。
- **adapter はいかなる経路でも非 0 exit しない**。非 JSON stdin・
  `toolName != "view"`・path 欠落・workspace 外・stat 失敗・CLI
  spawn 失敗・timeout・想定外 envelope — すべて exit 0 で素通し。
  これにより platform の fail-closed 意味論の下でも hook 自身の
  fail-open contract（SPEC-034r05iHK8bA7gsvijtkou Req.3）を維持する。意図した block
  のみが deny を意味し、その表現は stdout JSON に限定される。
- **`hooks.json` に `preToolUse` event を追加宣言する**（`pre_read_code`
  と併記）。APM は event 名を verbatim で全 target config に書き込む
  ため、copilot の generated config には `preToolUse` 登録が入り
  native に発火し、他 target では no-op となる。**実測による
  追記**: apm 0.28.0 の claude adapter は event 名を native casing
  に正規化するため、`preToolUse` は claude config では PascalCase
  `PreToolUse`（matcher `*`）として **live 登録される**（no-op 想定
  は誤りだった）。ただし claude の stdin shape（`tool_name` /
  `tool_input`）は本 adapter の `toolName` / `toolArgs` と一致
  しないため、adapter は全 payload を exit 0 で素通しし、実質
  no-op として安全に振る舞う。
- **`toolArgs` normalize**: string なら `JSON.parse`（失敗時 exit 0）、
  object なら直接使用。path key は `path` / `file_path` / `filePath`
  の順で探索する。
- **`.json` asset gap は upstream 修正待ち**とし、
  `apm-install-targets.test.ts` の expected-failure assert を drift
  signal として維持する。adapter は `.json` に依存しない。
  file-schema worker が copilot deploy で不動である点は matrix と
  consumer 向け文書の既知の制限として明記する。

## Consequences

- copilot consumer install で、巨大 file の `view` が stdout deny JSON
  + guidance で block される（count-tokens が `.json` なしで動く
  ため `.json` gap の影響を受けない）。
- 全 target の generated config に `preToolUse` 登録が入る。native
  event を持たない target では dead registration（現行の
  `pre_read_code` と同じ no-op 状態）。claude では `PreToolUse` に
  正規化され live 登録されるが、payload shape 不一致で実質
  no-op（tool call 毎に node spawn が走る overhead のみ）。
- adapter 誤動作（内部 error）は deny ではなく素通しになる —
  fail-open のため大 file が読まれるが、agent 全体が stop することは
  ない。
- claude・cursor 用の adapter は同じ per-target script pattern で
  後続 chain に分離できる。
- `.json` gap が upstream で修正されると expected-failure assert が
  fail し、matrix の更新が必要になる（意図した drift signal）。

## Confirmation

- copilot shape の stdin payload（object/string 両 toolArgs）に対し
  threshold 超過 file で stdout deny JSON + exit 0、それ以外の全経路
  で exit 0（非 0 exit の不存在を assert する test）。
- `apm install --target copilot` の generated config に `preToolUse`
  登録が含まれる e2e assert。
- windsurf・他 target の既存 e2e 全件 pass。

## Open Questions

- ~~claude での `preToolUse` 登録の扱い~~ → 実測で解決: apm が
  `PreToolUse`（PascalCase、matcher `*`）に正規化して live 登録する。
  claude の payload shape は adapter の contract と不一致のため全て
  exit 0 素通し（実質 no-op）。apm-install-targets.test.ts の claude
  case で assert 済み。
- copilot `view` tool の `toolArgs` の実際の key（`path` 等）は
  runtime 未検証 — normalize が複数 key を許容するため網羅されるが、
  実機検証で確定したい。

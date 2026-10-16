---
id: "TSPEC-034Dak4h2lD4Kk5dB1y5q8"
type: "test-spec"
status: "draft"
title: "Devin provider Windows credential contract behavior"
created: "2026-10-09"
updated: "2026-10-17"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/devin-provider-windows-credential-snapshot-sync.md"
  derived-by: []
  verifies:
    - "docs/specs/devin-provider-windows-credential-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Devin provider Windows credential contract behavior

## Purpose

Devin CLI 3000.x の Windows credential 解決が global
`%APPDATA%\devin\credentials.toml` に固定される制約の下で、cheaper
の credential 供給源を cheaper-owned snapshot に保ち、global
session への副作用を防ぐ contract を pin する。

## Feature

Devin provider auth contract on Windows — login snapshot・reuse
判定・logout 非破壊。

## Rules

- Rule: `devin auth login` が exit `0` の場合のみ global toml が
  cheaper data dir へ temp+rename で atomic copy され、copy 後に
  parse 検証される。
- Rule: reuse 判定は file 上の `api_key` 一致のみで行い、
  `devin_auth_status`（global session の CLI 判定）は使わない。
- Rule: key 不一致（global parse 可能時のみ）は re-login 前に
  `interaction.notify` で告知する。
- Rule: Windows の logout は `devin auth logout` を spawn せず
  cheaper toml のみ削除し、global session 継続を notify する。
- Rule: global toml path は `devin auth status` の `File:` 行
  （exit code 非 `0` でも parse）→ `%APPDATA%` fallback の順で
  解決する。
- Rule: Windows contract は注入 platform flag で動作し、全 AC は
  Linux CI で実行可能。
- Rule: resolver の fixture は、実行 OS の `PathBuf` が構成要素を解釈できる native path を使う。Windows workflow の注入は foreign path syntax の変換を意味しない。Windows の実行では native fixture 自体が Windows 形式となる。

## Examples

- Example: `windows=true` + `devin auth login` exit `0` + global
  toml 存在 → `<dataDir>/devin/credentials.toml` に copy され
  `api_key` が一致。
- Example: `windows=true` + login exit 非 `0` + 既存 global toml
  → copy されず error（abort した login が stale global を
  snapshot しない）。
- Example: cheaper toml key A + global toml key A →
  `devin auth login` を spawn せず cheaper credential を返し、
  `auth status` spawn は `File:` parse の 1 回のみ。
- Example: cheaper key A + global key B → 置換/採用の notify が
  記録され `devin auth login` が実行される。
- Example: `windows=true` の logout → argv に `auth logout` を
  含まず、cheaper toml 削除 + global 継続 notify。
- Example: `auth status` 出力に `File:` 行なし + `APPDATA` env
  あり → `%APPDATA%\devin\credentials.toml` に fallback。
- Example: 空白を含む native credential path を持つ複数 `File:` 行・CRLF 出力は最後の有効行を採用し、APPDATA fallback より優先する。
- Example: CLI が exit 非 `0` でも、stderr に有効な native `File:` 行があれば採用する。

## Guarantees

- 非 Windows 経路の argv 列・env 注入・reuse 判定は既存 test
  （`tests/cli_credentials.rs`）で非 regression。
- `PluginAuth::logout` signature 拡張後も `logout` を override
  しない impl（`FakePluginAuth`）は default impl で compile する。
- global toml は freshness 判定（path 特定 + key 比較）にのみ
  使われ、API key 供給源は常に cheaper toml。

## Non-goals

- 非 Windows contract の再検証（既存 test が担保）。
- logged-out `devin auth status` の `File:` 行有無（fallback が
  安全側に働くため挙動不問）。
- `devin models list` 等の global creds 依存経路の検証。

## Risk

本 spec が無いと、Windows で global session 破壊（`devin auth
logout` spawn）や stale global の silent snapshot（exit code 未
gate copy）が再導入されても検出できない。platform flag 注入の
test seam を失うと Windows contract が CI で検証不能になる。

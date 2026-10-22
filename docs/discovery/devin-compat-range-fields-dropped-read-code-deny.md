---
id: "DISC-t3TdC7pUJUGdYgfkpkmhMU"
type: "discovery"
status: "capturing"
title: "Devin compat 経由の read/grep/exec が範囲指定を無視して全文 token 数で拒否される（#161, #107 再発）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/161"
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
  related:
    - "../specs/tool-operator-built-in-worker.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Devin compat 経由の read/grep/exec が範囲指定を無視して全文 token 数で拒否される

## Seed（briefing 前の最小記録）

Issue #161（#107 再発）。code-knowledge（Windows・Devin CLI）で、
threshold 超過ファイルへの範囲指定 read・単一ファイル grep・shell 経由
読み取りが、いずれも「ファイル全体の token 数」で拒否される。

### 既知の事実（issue 本文より）

- 発火経路は `.windsurf/hooks.json` の legacy 登録（`pre-read-code`
  `--target windsurf`）— `.devin/config.json` が無い環境では native
  `--target devin` 経路が使われず windsurf 互換変換に縮退する
- 各 target の bounded 評価自体は正常（windsurf/devin/pretooluse で
  `limit`/`offset` 付き入力は allow、無しは全文 deny — 直接検証済み）
- 推定原因: Devin の windsurf 互換変換が `tool_info` に adapter が読む
  範囲 field（`offset`/`line_offset`/`limit`/`line_limit`）を載せていない、
  または別名（`start_line`/`end_line`/`view_range`）で送っている
- 現行の windsurf 正規化は `offset`/`line_offset`/`limit`/`line_limit`
  のみを読み `start_line`/`end_line`/`view_range` は無視する
  （`crates/cheaper-cli/src/hook/targets.rs`）

### 候補対応（issue 本文より）

1. windsurf adapter が `start_line`/`end_line`/`view_range` も
   `ReadRange` へ正規化する防御
2. Devin compat が実際に `tool_info` へ送る field 集合を調査し contract
   として文書化
3. native `hook-config sync` 未登録環境での縮退経路の扱い
   （legacy 登録の検出・案内）

詳細は briefing で確定する。

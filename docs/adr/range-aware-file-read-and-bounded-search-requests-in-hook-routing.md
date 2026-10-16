---
id: "ADR-034H2WBlp8VXoEE7KVlGt5"
type: "adr"
status: "accepted"
title: "Range-aware file read and bounded search requests in hook routing"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  depends-on:
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bounded-hook-read-request-contract.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
  derived-by:
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# Range-aware file read and bounded search requests in hook routing

## Context and Problem Statement

issue #107 の根因: canonical `RoutingRequest` が
`FileRead { path, cwd, source_tool }` / `Shell` のみで、native
payload の `offset`/`limit`/`pattern`/`max_results`/`glob_pattern`
等の bounds が adapter の normalize 段階で捨てられる。
`evaluate_file_read` は常にファイル全文を計測するため、要求
ウィンドウが小さくても 8192 token 超のファイルは一律 deny になり、
`grep` は `path` をそのまま FileRead へ flatten して評価される
（dir path なら非 file で allow、file path なら全文計測 — どちらも
「返却内容」ではない）。

評価ポリシーの防御対象は「agent へ返却される内容のサイズ」であり、
ファイルの物理サイズではない。request が保持する bounds を
canonical model まで届け、評価も要求内容単位に揃える必要がある。

## Decision Drivers

* SPEC-034H2WAU6TQG6N9dmmmKNA の R1–R8（bounded evaluation）を
  満たすこと。
* ADR-034rzTJYDRmre6D5OS8HDW の「per-target adapter が canonical
  request へ正規化 → shared policy が判定」の構造を維持し、
  target 固有例外を作らない。
* fail-open/fail-closed 原則（infra 失敗 → allow、測定済み超過 →
  deny）と workspace containment 規則を既存と同一に保つ。
* 計測コスト: bounded read で全文 tokenize しない。search 評価の
  scan は上限を持つ。

## Considered Options

* Option 1: `RoutingRequest::FileRead` に `range: Option<ReadRange>`
  を追加し、`Search { path, pattern, max_results, context, glob,
  output_mode }` variant を新設する。`evaluate` は kind ごとに
  全件/範囲/検索の評価へ分岐する。
* Option 2: `FileRead` をそのままに、adapter が `offset`/`limit` を
  path 文字列へ encode する等の「表現の借用」で済ませる。
* Option 3: bounds を policy へ渡さず、deny reason で tool 側へ
  再指示する（skill 経由の手順で代替）。

## Decision Outcome

Chosen option: "Option 1: canonical request の拡張"。

`ReadRange { offset: u64, limit: Option<u64> }`。offset は
1-based 開始行（wire 観測 `offset:334, limit:81` と Claude/Devin
系 line-window 慣行に基づく前提、一次資料で未確認のため前提として
記録）。`limit` 省略（`None`）は「`offset` からファイル末尾まで」
を評価対象とする（`offset==1` かつ省略のみ全文評価と同値）。無効値
（非整数・負値・`offset < 1`・`limit < 1`・非数値）は normalize
段階で `range = None` に縮退し、全文評価へ倒す。

範囲評価 `evaluate_ranged_read` は既存 `evaluate_file_read` と同じ
resolve/containment/非 file/missing 規則を使い、byte fast path も
共有する。閾値超過 file では行 iterator で `[offset,
offset+limit)` のみ収集する。skip（`offset` までの前行）を含む走査
byte が `count_max_bytes` cap を超過した場合は測定不能の deny
ではなく FileTooLarge 相当の deny とし（cap は「計測可能な上限」の
意味）、収集 byte 超過・収集テキストの `count_text_tokens` が
threshold 超過なら deny。行読みは `take()` で cap 超過読み込みを
抑止する。EOF 超過は空ウィンドウとして allow。

`Search { path, pattern, max_results, context, glob, output_mode,
case_insensitive }` は返却内容を上限付きで実体化する（`head_limit`
は `max_results` の alias として normalize で小さい方へ畳み込む）。`path` 省略は `"."`、cwd anchor は payload `cwd` → target
固有 env（devin は `DEVIN_PROJECT_DIR`）→ プロセス cwd の順。
containment は FileRead と同一規則だが、解決結果が workspace root
そのもの（`rel` 空）の場合は dir scan 対象として継続する差分を持つ
（workspace 外は既存どおり allow）。file path は単一 file のみ、
dir は `glob`/`glob_pattern` で file を絞って走査する。scan は
`SEARCH_SCAN_MAX_FILES = 10,000` file / `count_max_bytes` byte の
cap を持ち、`.git` を除く全 regular file（hidden 含む、gitignore
非適用 — over-scan が安全方向）を対象とする。正規表現（
`case_insensitive`/`-i` を反映）は一致行 + 前後 context 行
（`context`/`-C` と `-A`/`-B`）を `max_results`（`head_limit`
同値受理、両方ある場合は小さい方、省略時 250）組まで収集し、
収集テキストを tokenize して threshold と比較する。
`output_mode` 省略・未知値は content として評価する。
`files_with_matches`/`count` 等の非 content も file path 一覧を
`max_results` まで実体化して同じ比較を行い、無条件 allow はしない。
scan cap 到達は「`path`/`glob`/`max_results` で範囲を絞る」
guidance 付き deny。`pattern` 欠落/非 string や compile 失敗は
評価不能 → allow。`type` は絞り込みを無視して over-scan（安全
方向）、`offset`（結果 skip）は無視して over-count（安全方向）、
`multiline` は per-line 近似として記録する。

### Consequences

* Good, because 評価が「返却内容」のサイズに基づき、#107 報告の
  short range read / bounded grep が過剰 deny されなくなる。
* Good, because target adapter は normalize のみ変更し、判定ロジック
  は shared policy に残る。
* Good, because 無効 bounds・計測不能は既存の fail-open/fail-closed
  原則にそのまま写像できる。
* Bad, because Search の実体化は tool 実出力の厳密な再現ではなく、
  formatting 差分・tool 側の独自 cap は近似として残る。
* Bad, because dir scan の file 数/byte cap は新しい規模閾値であり、
  巨大 workspace での grep は「範囲を絞る」deny になる可能性が
  ある。

### Confirmation

* 8192 token 超 file への `read{offset,limit}` で小さい window が
  allow、全文 `read` は deny、無効値が全文評価へ縮退する test。
* `grep{path:file,pattern,max_results}` が FileRead を経ず、小さい
  結果は allow・大きい結果は deny・`max_results` 存在だけで
  allow しない test。
* offset 基数・EOF・scan cap・`output_mode` 分岐が test-spec と
  一致すること。

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/hook/routing_policy.rs`、
  `crates/cheaper-cli/src/hook/targets.rs`、
  `crates/cheaper-cli/tests/integration/hook.rs`、必要に応じて
  `crates/cheaper-cli/src/count.rs`（範囲計測 entry）。
* Patterns to follow: `evaluate_file_read` の resolve/containment/
  fast path と `measure_tokens` の timeout/env snapshot 構造。
* Patterns to avoid: adapter 側での target 固有 verdict、`path`
  文字列への bounds encode、全文 tokenize のままの範囲 read。
* Dependencies or configuration: 既存 `count_text_tokens`、
  `count_max_bytes`、regex による pattern 照合（workspace 既存依存の
  範囲内）。

## Verification

* [ ] 範囲 read: threshold 超 file + 小 window → allow、window が
      threshold 超 → deny、全文 read → 従来判定の test が pass。
* [ ] search: file/dir/省略 path、max_results・context・glob 反映、
      結果実体化が threshold 内 allow / 超過 deny の test が pass。
* [ ] 無効 bounds・scan cap・計測失敗の挙動が test で固定される。

## Pros and Cons of the Options

### Option 1

* Good, because 要求内容ベースの評価になり、#107 の根因を構造で
  除去できる。canonical request が wire の bounds を保持するため
  将来の tool/target 追加にも流用できる。
* Bad, because Search の実体化ロジックと scan cap という新しい
  評価経路・閾値が増える。

### Option 2

* Good, because request 型を増やさず diff が最小。
* Bad, because bounds が path 文字列に混入して表現 drift を起こし、
  search の返却内容評価は依然不可能。

### Option 3

* Good, because 実装がほぼ不要。
* Bad, because #107 の deny が解消されず、review の必須条件を
  満たせない。

## More Information

* wire 証拠・`max_results` が byte 上限ではない点は
  `docs/discovery/bounded-hook-read-request-contract.md` に記録済み。
* `count_text_tokens` は `cheaper_core::token::count` に既存。

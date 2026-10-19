---
id: "DESIGN-034H3c448Lvz0CfBjd5Xlz"
type: "design"
status: "approved"
title: "Bounded hook request evaluation and hook-config registration lifecycle"
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
  implements:
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/devin-native-hooks-and-apm-bootstrap.md"
    - "../designs/cross-agent-native-routing-adapter-architecture.md"
    - "../adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "../specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
  defers: []
  deferred-by: []
---

# Bounded hook request evaluation and hook-config registration lifecycle

## Context

現行の hook 経路は `targets.rs` の per-target adapter が native
stdin payload を `RoutingRequest`（`FileRead` / `Shell` のみ）へ
正規化し、`routing_policy.rs` の `evaluate` が workspace
containment・byte fast path・in-process token 計測・threshold・
fail-closed deny を行う（DESIGN-034z1EhuztlLSb2x2XXxgD /
DESIGN-034rA79Uog7JOa7evw6rOL）。issue #107 で報告されたとおり、
native `read`/`grep` の bounds（`offset`/`limit`/`max_results`/
`glob_pattern`/context 系）は normalize で消失し、全文計測で一律
deny になる。

また Devin native registration は PowerShell script の preview/apply
だけが存在し、CLI 側の管理・検査・除去・APM lifecycle 統合・POSIX
経路がない。

本 design は SPEC-034H2WAU6TQG6N9dmmmKNA の要求を
ADR-034H2WBlp8VXoEE7KVlGt5（request 拡張）と
ADR-034H2WBNtC4We8dyC5lQZJ（CLI 所有の registration lifecycle）
の選択肢として実装する。

## Scope

### In Scope

- `RoutingRequest::FileRead` への `range: Option<ReadRange>` 追加と
  `Search` variant の新設。`evaluate` の kind 分岐。
- `evaluate_ranged_read`（範囲ウィンドウの byte/token 評価）と
  `evaluate_search`（上限付き実体化 + token 評価）。
- `tool_use_normalize` の `read`/`grep`/`exec` 正規化拡張
  （pretooluse: `Read`/`Grep`/`Bash`/`PowerShell`、devin:
  `read`/`grep`/`exec`）。
- `cheaper hook-config {sync|verify|remove}`（project/user scope）、
  `--event-stdin`、旧 canonical matcher の収束、legacy 登録の scan。
- `install-devin-hooks.ps1` の CLI 委譲化、distribution guide /
  skill docs / 先行 docs の追随、APM per-dep `targets:` 移行手順。
- Rust integration + Node e2e の拡張（POSIX container を含む）。

### Out of Scope

- APM 本体への `devin` target 追加。
- unverified Devin tool 名の matcher 追加（`exec` 以外）。
- `.devin/hooks.v1.json` の生成、`hooks.json` との precedence 変更。
- global/ancestor scope の完全走査。
- tool 実出力 byte-level の厳密再現（grep formatter の完全互換は
  近似で済ませる）。

## Components and Boundaries

### `hook::routing_policy`

- `pub enum RoutingRequest`
  - `FileRead { path, cwd, source_tool, range }` — `range` は
    `Option<ReadRange>`。`ReadRange { offset: u64, limit: Option<u64> }`
    で、`offset` は 1-based 開始行（wire 観測 + Claude/Devin 系
    line-window 慣行に基づく前提）、`limit` は最大行数（None =
    `offset` から末尾まで）。
  - `Search { path, cwd, source_tool, pattern, max_results, context,
    glob, output_mode, case_insensitive }` — `pattern` は String
    （normalize で `as_str`）、`max_results`/`head_limit` は
    `Option<u64>`（両方ある場合は小さい方、省略時 250）、`context`
    は u64（`-C`/`context` と `-A`/`-B` の max）、`glob` は
    `Option<String>`、`output_mode` は enum（`Content`/
    `FilesWithMatches`/`Count`/`Unknown` — 省略・Unknown は Content
    として評価）、`case_insensitive` は `tool_input` の
    `-i`/`--ignore-case`/`caseInsensitive` を正規化した bool。
  - `Shell` は既存どおり。
- `evaluate` は `FileRead` で `range.is_some()` → ranged read 評価、
  `Search` → search 評価、それ以外は既存分岐。

### `hook::targets` — normalize のみ拡張

- `read`: `file_path` を必須、`offset`/`limit` を `Option<u64>` に
  正規化（非整数/負値/`offset<1`/`limit<1`/非数値 は `range: None`
  へ縮退）。windsurf `pre_read_code` の `tool_info.offset`/`limit`
  と copilot `view` の `view_range` など、wire が bounds を運ぶ他
  target でも同じ抽出を行う。
- `grep`: `pattern`（必須・as_str）、`path`（省略時 `"."`、
  非 string は request 自体 `None` → 評価不能 allow）、
  `max_results`/`head_limit`、`context` 系（`-C`・`-A`・`-B`）、
  `glob`/`glob_pattern`、`output_mode`、`-i`/`--ignore-case` を
  `RoutingRequest::Search` へ。`type`・`offset`（結果 skip）・
  `multiline` は近似として over-scan/over-count 方向へ倒す。
- `exec`（devin のみ）: `tool_input.command` を Shell へ。payload
  `cwd` → `DEVIN_PROJECT_DIR` env → プロセス cwd の順で request
  cwd を anchor する。

### `hook::hook_config`（新規 module）

- `sync`: apply 前に既知の登録元を scan — 対象 file は
  `<root>/.windsurf/hooks.json`・`<root>/.devin/hooks.json`・
  `<root>/.devin/hooks.v1.json`・`<root>/.devin/config.local.json`・
  `<root>/.claude/settings.json`・`<root>/.claude/settings.local.json`
  — cheaper 管理外に cheaper 互換 read hook command
  （`pre-read-code`/`pre-tool-use` を含む command 文字列）があれば
  書かず error（旧 PS script の `$conflictRegex` gate を引き継ぐ —
  Devin は `read_config_from` で `.windsurf/`・`.claude/` を既定
  import し、旧登録残存時は bounded 修正が発動しないため）。通過後は
  canonical entry の upsert（旧 matcher のみは現行へ収束）、
  重複/改変/曖昧な own entry は error。temp+rename で atomic
  replace、`config.json.*.tmp`/`.bak` の orphan sweep を起動時と
  失敗時に行う。
- `verify`: 同じ scan を読み取りのみで実行し、登録の有無・drift・
  既知 legacy cheaper hook の残存を報告。`.devin/hooks.v1.json`
  /`.devin/hooks.json` が `config.json` と併存する場合は shadow の
  可能性を warn で報告（優先順位は公表未確認）。問題があれば
  exit 非ゼロ + 修復案内。
- `remove`: `.devin/config.json` の own entry のみ除去。無い場合は
  no-op。曖昧/改変は error。他の登録元に残る非 APM 経由の cheaper
  登録は verify で報告する（自動除去しない）。
- scope 解決: `--scope project`（既定、`--project-root` または
  cwd の `.devin/config.json`）、`--scope user`（POSIX:
  `~/.config/devin/config.json`、Windows: `%APPDATA%\devin\
  config.json`）。その他は明示 error。config file が無い場合:
  `sync` は作成、`verify` は非ゼロで未登録、`remove` は no-op。
  JSONC コメント有りは strict parse 失敗として「コメント除去か
  手動編集」error（書き込みでコメントは保持されない制限）。
- `--event-stdin`: stdin JSON の `event` で
  `post-install`/`post-update` → `sync`、`post-uninstall` →
  `remove`（payload の `packages`/`scope` に cheaper —
  name `cheaper` / slug `scarecrow173/cheaper` — が含まれる場合
  のみ; 判定不能は remove せず verify 出力）、`pre-*` → no-op、
  未知 event → error、malformed payload → error。project root は
  payload `cwd` → 呼出し cwd の順。
- `cheaper hook targets --json` の capability line に
  `"hookConfig": true` を追加し、wrapper/script が `hook-config`
  対応 binary を判別できるようにする（schemaVersion は 1 のまま
  additive）。

### `install-devin-hooks.ps1`

- preview/`-Apply` の UX は残すが、実際の config 編集・能力判定は
  `cheaper hook-config` へ委譲する。`hook-config` の出力を preview
  表示に流用する。POSIX consumer は CLI を直接実行する（wrapper
  script は作らない）。

## Data and Control Flow

### Ranged read

1. adapter が `tool_input.offset`/`limit` を `ReadRange` へ正規化。
2. `evaluate` → `FileRead { range: Some(r) }` →
   `evaluate_ranged_read`。
3. `evaluate_file_read` と同じ resolve/containment/非 file/missing
   規則 + `md.len()` byte fast path（file 全体が小さいなら従来通り
   early allow）。
4. 閾値超過 file のみ: `BufRead` で行を順に読み、`[offset,
   offset+limit)` 相当の行を `String` window へ収集。EOF 超過・
   offset > 総行数は空 window → allow。`offset` までの skip を
   含む走査 byte が `count_max_bytes` cap を超えた場合も
   FileTooLarge 相当の deny（cap は計測可能上限）。収集テキストの
   `count_text_tokens(window)` が threshold 超過 → deny。
   infra error / timeout → allow（既存契約どおり）。

### Search

1. adapter が `pattern`/`path`/`max_results`/`context`/`glob`/
   `output_mode`/`case_insensitive` を `Search` へ正規化。
   `path` 省略は `"."` — 解決は cwd anchor（payload `cwd` →
   `DEVIN_PROJECT_DIR` → プロセス cwd）に従う。
2. `evaluate_search` は `path` を resolve。workspace root そのもの
   （`rel` 空）を指す場合は FileRead とは異なり dir scan 対象として
   継続する。workspace 外・非 file/dir・missing は従来どおり allow。
3. `output_mode` が `files_with_matches`/`count` の非 content は
   「返却が file path/件数のみ」なので、file path 一覧を
   `max_results` まで実体化して同じ token 比較を行う（無条件
   allow はしない）。`Unknown`/省略は content として評価。
4. content mode: file 単体ならその file、dir は `glob` で絞って
   `.git` 以外の全 regular file を走査。走査は
   `SEARCH_SCAN_MAX_FILES = 10,000` file / `count_max_bytes` byte
   の cap を持ち、いずれかに到達した時点で評価完了不能とみなし
   「`path`/`glob`/`max_results` で範囲を絞る」guidance 付き deny
   とする（部分結果での評価は行わない — under-measurement を
   防ぐ）。pattern を compile（`case_insensitive` 反映）し、
   一致行 + `context` 前後行を `max_results`（両指定は小さい方、
   省略時 250）組まで収集。収集テキストを tokenize し threshold
   と比較。超過は同じ guidance 付き deny。compile 失敗・評価不能
   は allow。`type` は無視（over-scan）、`offset` は無視
   （over-count）、`multiline` は per-line 近似。

### Registration lifecycle

1. `cheaper hook-config sync` が config を読み、own entry の識別
   （matcher+command の canonical 集合）と既存内容の保持を行い、
   atomic write で upsert。
2. `verify` は read-only で own entry の有無・drift・既知 legacy
   登録を scan し、問題があれば exit 非ゼロ + 手順を返す。
3. `remove` は own entry のみ除去。
4. APM lifecycle では consumer が `lifecycle:` script に
   `cheaper hook-config ... --event-stdin` を登録。APM の payload
   JSON が stdin に届き、event で sync/remove を振り分ける。
   script 失敗が APM を abort しない仕様のため、`verify` は CLI
   側の検出・案内を担う。
5. per-dep `targets:` 移行（2経路のうちどちらか）:
   (a) consumer の cheaper dep を `targets: [agent-skills]` に絞る。
       project `targets` に `agent-skills` が必須 — 含まれないと
       dep 全体が skip され skills も除去される。APM 0.28.0 の
       install/update reconcile が cheaper 所有の Windsurf hook
       artifact を除去する（実機検証済み）。
   (b) Devin の `read_config_from` で `windsurf`/`claude` を明示
       `false` にし、Devin が旧登録を import しないようにする。
   `verify` は残存を検出してこの手順を案内する。

## Non-Functional Constraints

- すべての評価経路は `MEASURE_TIMEOUT`（30s）相当の bound を持ち、
  timeout・infra error は allow。
- ranged read の行走査は要求ウィンドウの終端で打ち切る。`offset`
  までの skip を含む走査 byte が `count_max_bytes` cap を超えた
  場合は deny（cap = 計測可能上限）。`take()` で cap 超過読み込み
  を抑止する。
- Search の走査は `SEARCH_SCAN_MAX_FILES = 10,000` file /
  `count_max_bytes` byte・`.git` 除外・hidden file 含む・
  gitignore 非適用で bounded とする。
- `hook-config` の書き込みは temp+rename で atomic replace、外部
  変更を capability 確認との間で検出して中断し、`config.json.*.tmp`/
  `.bak` の orphan を sweep する。
- symlink / reparse point は設定 file の対象にしない（PowerShell
  script の既存規則を継承）。
- 新規 runtime 依存は入れず、regex・JSON・path 処理は workspace
  既存依存で済ませる。

## Risks and Trade-offs

- Search 実体化は tool 実出力の近似であり、formatter 差分・tool 側の
  独自 cap は再現しない。判定は「返却内容の上限」という conservative
  な bound であり、tool がさらに絞る場合は過剰 deny にはならない。
- dir scan cap で巨大 workspace の grep は deny になる可能性がある。
  guidance で `path`/`glob`/`max_results` の絞り方を案内する。
- APM lifecycle script は trust 前提・fail-soft のため、全 consumer で
  自動収束を保証できない。`verify` による検出と手動手順を残す。
- `install-devin-hooks.ps1` の wrapper 化は既存 test が script の
  細かい振る舞いに依存するため、CLI 側の挙動を script contract に
  揃える必要がある。

## Deferred Design Concerns

- `glob` tool・`write`/`edit` 系の hook 評価 | 今回の範囲外・wire 未
  観測 | Devin docs/実機で確定した時点。
- `.devin/hooks.v1.json` と `.devin/config.json` の precedence | 公式
  未明記、verify で併存を warn 検出 | Devin docs/実機で確定した時点。
- `.apm/hooks.json`（APM PR #1798 の file-based lifecycle hook）が
  package 側から lifecycle script を供給できるか | 未検証 | APM の
  package-level lifecycle 機構が確立した時点。
- `remove` が `.devin/config.json` 以外の cheaper 登録を扱わない
  残余リスク | verify の検出と手動除去案内で代替 | Devin 側の
  precedence/import が確定した時点。
- `search` の `multiline`・`type`・結果 `offset` 等の近似 | over-
  scan/over-count 方向で安全側 | tool 実動作の wire 観測が確定した
  時点。
- lifecycle event の untrusted script フォールバック | verify 経路で
  代替済み | APM 側で package が強制実行可能な lifecycle を提供する
  場合。

## Implementation Plan

1. `RoutingRequest` に `range`/`Search` を追加し、`evaluate` の kind
   分岐を作る。
2. `evaluate_ranged_read` + `evaluate_search` を実装。
3. `tool_use_normalize` を bounds 抽出へ拡張、`exec` を devin shell
   tool に接続、matcher を `read|grep|exec` へ更新。
4. `hook-config` module + CLI dispatch + `--event-stdin`。
5. PS wrapper 化・skill/docs/distribution guide・spec/ADR の追随。
6. Rust/Node test の拡張・POSIX 検証。

## Verification

- [ ] bounded read/search の Rust integration test が pass。
- [ ] hook-config sync/verify/remove と lifecycle stdin が pass。
- [ ] per-dep `targets:` 移行 e2e が旧 hook の再配布を防ぐ。
- [ ] Windows + POSIX で同じ contract が通る。

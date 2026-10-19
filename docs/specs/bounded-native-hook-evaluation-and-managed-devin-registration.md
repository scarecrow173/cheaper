---
id: "SPEC-034H2WAU6TQG6N9dmmmKNA"
type: "spec"
status: "implemented"
title: "Bounded native hook evaluation and managed Devin registration"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
    - "https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6183636932"
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
    - "https://docs.devin.ai/cli/extensibility/hooks/overview"
    - "https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks"
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
    - "../specs/case-insensitive-native-tools-and-devin-registration.md"
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../specs/apm-delegation-skills-and-read-hook.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "../specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bounded-hook-read-request-contract.md"
  derived-by:
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
  defers: []
  deferred-by: []
---

# Bounded native hook evaluation and managed Devin registration

## What Should Be Built

issue #107 の根本修正と Devin native hook 登録の lifecycle 完備。

1. canonical `RoutingRequest` が要求範囲を保持するように拡張し、
   bounded `read {file_path, offset, limit}` は要求ウィンドウだけを
   token/byte 評価する。bounded `grep` は全文 `FileRead` に潰さず
   専用の Search request で、実返却内容の上限付き実体化を評価する。
2. Devin `exec` tool を検証済みの一次情報に基づき既存の shared
   Shell policy へ接続する。
3. `.devin/config.json` の cheaper 管理 hook registration を CLI
   (`cheaper hook-config`) が sync / verify / remove で一元的に管理
   する。install / update / uninstall は同一の収束モデルを通り、
   Windows / POSIX 両対応。APM lifecycle event・per-dependency
   `targets:` と統合し、旧 Windsurf hook の再配布を防ぐ移行経路を
   提供する。

## Why Now

- PR #109 review（comment-6221192905）が #107 の範囲評価・登録
  lifecycle の残課題をこの PR の必須条件とした。これらは旧 spec
  `SPEC-034GssA0IjLKSiDdIHpAC8` の Out of Scope に置かれていたが、
  reviewer の判断で本 increment へ昇格した。
- 現状のままでは 8192 token を超えるファイルへの `offset`/`limit`
  付き read・`max_results` 付き grep が必ず deny され、delegation
  skill（cheaper-bulk-read）が案内する short-range read の解除経路が
  Devin/Claude 系 consumer で事実上機能しない。
- 登録の追加だけが Windows 限定 PowerShell script に存在し、削除・
  検査・再収束・POSIX 経路が無いため、consumer は一度登録されると
  安全に外せない。

## Users and Value

- **Devin CLI consumer**: 巨大ファイルの範囲 read / 上限付き grep が
  hook で不当に deny されず、過大な全文 read・無制限 search は
  引き続き guard される。hook 登録の導入・確認・除去がクロス
  プラットフォームの CLI で一貫して行える。
- **APM/Windsurf 併用 consumer**: 旧 Windsurf cheaper hook の
  再配布を構造的に防ぐ manifest 手順と、stale 登録を検出して
  収束する verify/sync を得る。
- **cheaper メンテナ**: 評価ポリシーが「要求内容」単位で判定する
  canonical request に揃い、登録管理が単一の CLI 実装に集約される。

## Scope

### In Scope

- `RoutingRequest` に範囲 read（`ReadRange`）と `Search` kind を
  追加し、`evaluate` が要求範囲・実返却内容ベースで判定する実装。
- pretooluse / devin adapter の `tool_input` から
  `offset`/`limit`/`pattern`/`path`/`glob`/`max_results`/`head_limit`/
  `context`（`-A`/`-B`/`-C` 相当）/`output_mode`/`-i`・
  `--ignore-case`・`caseInsensitive` を正規化する処理。
- Devin `exec` → `RoutingRequest::Shell` への接続と登録 matcher への
  `exec` 追加（一次資料で確認済み tool 名のみ）。
- `cheaper hook-config` 相当の CLI verb 群（sync / verify / remove）、
  project scope と user scope、atomic write、malformed JSON error、
  無関係設定の保全、冪等、concurrent modification 検出、POSIX/Windows
  両対応。
- APM lifecycle event との統合経路（stdin event payload の受理と
  event→操作の写像）と、per-dependency `targets: [agent-skills]`
  による旧 Windsurf hook の再配布防止・reconcile 移行手順。
- 登録・payload の ownership 識別と cheaper 所有分のみの cleanup。
- Rust integration・Node e2e（consumer scenario・lifecycle・POSIX）の
  拡張、および `cheaper-bulk-read` skill・distribution guide・先行
  spec/ADR/design の contract 追随。

### Out of Scope

- APM 本体への `devin` target 追加（外部 repo の変更であり、この
  repository では実装不可 — review 指摘3の外部依存として明記済み）。
- Devin `exec` 以外の unverified tool 名の matcher 追加。
- hook verdict の fail-open/fail-closed 原則自体の変更
  （infrastructure 失敗 → allow、測定済みの規模超過 → deny は維持）。
- `.devin/hooks.v1.json` の生成（既存 ADR の方針維持 — 本 spec は
  `.devin/config.json` の `hooks.PreToolUse` のみを管理対象とする）。
- APM project 外（global/ancestor）にある既存登録の完全な走査。
- per-dependency `targets:` による絞り込みを consumer 側で強制する
  機構（consumer manifest は consumer の所有物であり、cheaper は
  手順・検出・verify で支援する）。

## Requirements

### Bounded evaluation

- R1: `read` 系 tool（pretooluse `Read`・devin `read`、および
  wire が bounds を運ぶ他 target — windsurf `pre_read_code` の
  `tool_info.offset`/`limit`、copilot `view` の `view_range`）の
  `offset`/`limit` を canonical request へ保持する。`offset` は
  1-based の開始行（wire 観測 `offset:334, limit:81` と
  Claude/Devin 系 line-window 慣行に基づく前提）、`limit` は最大
  行数とする。`offset` 省略は 1、`limit` 省略は「`offset` から
  ファイル末尾まで」を評価対象とする（`offset==1` かつ `limit`
  省略のみ全文評価と同値）。
- R2: 範囲指定がある場合、評価対象は要求ウィンドウ
  `[offset, offset+limit)` に切り出した行集合とし、ファイル全体を
  tokenize してはならない。EOF 超過・総行数超過の offset は空
  ウィンドウ（allow）として扱う。
- R3: 無効値の扱い — `offset`/`limit` が非整数・負値・
  `offset < 1`・`limit < 1`・非数値の場合は範囲情報を捨てて全文
  評価へ縮退する（guard を弱めない方向へ倒す）。`limit: 0` も
  無効として全文評価となる。
- R4: 範囲評価の計測失敗・timeout は既存契約どおり allow
  （fail-open）、ウィンドウが byte 計測 cap 超過・token threshold
  超過と測定できた場合は deny（fail-closed）。
- R5: `grep` 系 tool は `path` が file・dir・省略のいずれでも
  `Search` request となり、全文 `FileRead` へ flatten しない。
  `pattern` が非 string・欠落の場合は評価不能として allow。
- R6: Search 評価は `max_results`（`head_limit` 同値として受理、
  両方ある場合は小さい方）・context 行数（`context`/`-C` と
  `-A`/`-B` の max — 前後対称 window）・`glob`/`glob_pattern`・`output_mode`・
  `-i`/`--ignore-case` を反映した返却内容を上限付きで実体化し、
  その token 数を threshold と比較する。`max_results` の存在だけで
  allow しない。実体化結果が threshold を超える場合は deny。
  `output_mode` 省略・未知値は content として評価する。
  `files_with_matches`/`count` の非 content でも返却一覧を
  `max_results` まで実体化して同じ比較を行う（無条件 allow は
  しない）。
- R7: Search の scan は上限を持つ — 走査 file 数は
  `SEARCH_SCAN_MAX_FILES = 10,000`、走査総 byte は
  `count_max_bytes`（`CHEAPER_COUNT_MAX_BYTES`、既定
  16 MiB）と同じ cap。いずれか到達で評価完了不能とみなし、
  「`path`/`glob`/`max_results` で範囲を絞る」guidance 付きの
  deny とする。`max_results`/`head_limit` 省略時の実体化既定 cap
  は 250 match。走査は `.git` を除く全 regular file（hidden file
  を含む、gitignore は適用しない — over-scan が安全方向）。
  収集する行は truncate せず実内容を計測する。
- R8: workspace containment・相対パス解決は FileRead と同じ規則
  を使うが、Search に限り `path` が workspace root そのもの
  （`rel` が空）を指す場合は dir scan 対象として継続する
  （FileRead では `rel` 空は allow になる差分）。workspace 外を
  指す path は既存どおり allow（policy の防御境界外）。
  `path` 省略は `"."` = 解決後の cwd（下記 anchor 規則）。
  cwd anchor は payload の `cwd` → target 固有の env
  （devin は `DEVIN_PROJECT_DIR`）→ hook プロセス cwd の順。

### Devin shell interception

- R9: devin `exec`（`tool_input.command`）を `RoutingRequest::Shell`
  へ正規化し、登録 matcher を `read`/`grep`/`exec` の ASCII
  case-insensitive exact とする。一次資料で確認できない tool 名は
  追加しない。

### Managed registration

- R10: `cheaper hook-config <verb>` を通常 CLI dispatch の新規
  top-level command として提供する（`hook` prefix の早期分岐とは
  別 — dispatch の第1引数が `"hook"` と完全一致する場合のみ早期分岐
  されるため衝突しない）。verb は
  `sync`（install/repair の収束）、`verify`（状態検査・drift
  検出）、`remove`（own entry の除去）。対象は `.devin/config.json`
  の `hooks.PreToolUse`。`sync` は apply 前に既知の登録元
  （`.windsurf/hooks.json`・`.devin/hooks.json`・
  `.devin/hooks.v1.json`・`.devin/config.local.json`・
  `.claude/settings*.json` 等）を scan し、cheaper 管理外に
  cheaper 互換 read hook command（`pre-read-code` / `pre-tool-use`
  を含む command 文字列）があれば書き込まず error — Devin は
  `read_config_from` で `.windsurf`/`.claude` の設定を既定で
  import するため、旧登録が残ると bounded 修正が発動しない。
- R11: scope は `--scope project`（既定、`<root>/.devin/config.json`、
  root 既定は cwd）と `--scope user`（POSIX:
  `~/.config/devin/config.json`、Windows: `%APPDATA%\devin\
  config.json`）を扱う。未対応の scope 指定は明示 error。
  config file が存在しない場合: `sync` は `.devin/` または user
  config dir を作成して書き込み、`verify` は「未登録」で非ゼロ、
  `remove` は no-op 成功。config 内にコメント等の JSONC 記法が
  ある場合は strict parse 失敗として「コメントを除去するか手動で
  編集する」error を明示する（書き込み時にコメントは保持
  されない既知の制限）。
- R12: 既存 config の無関係 key・他 event・他 hook group は保持し、
  malformed JSON・非 schema・重複/改変済み own entry・symlink/
  reparse point は書き込まず明示 error とする。書き込みは
  temp+rename の atomic 更新で、capability 確認との間の外部変更を
  検出したら中断する。
- R13: own entry の識別は command 完全一致
  （`cheaper hook pre-tool-use --target devin`）+ matcher が既知
  canonical 集合 `{^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$,
  ^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Ee][Xx][Ee][Cc])$}` に
  含まれることで行う。旧 matcher の entry は sync で現行
  canonical へ収束する。command は一致するが shape が未知の
  entry は「改変済み」として error、自動修復しない。capability
  line `cheaper hook targets --json` は `"hookConfig": true` を
  追加し、script が `hook-config` 対応 binary を判別できるように
  する（schemaVersion は 1 のまま additive）。
- R14: `remove` は own entry のみ除去し、own entry が無い場合は
  no-op 成功、曖昧な改変済み entry は error。`.devin/` 配下の
  cheaper 管理外ファイル（`hooks.v1.json` 等）には触れない。
- R15: `--event-stdin` は stdin から APM lifecycle payload JSON
  を読み、`event` で写像する: `post-install`/`post-update` →
  `sync`、`post-uninstall` → `remove`（ただし payload の
  `packages`/`scope` が示す対象に cheaper package — name `cheaper`
  または slug `scarecrow173/cheaper` — が含まれる場合のみ;
  含まれない・判定不能な場合は no-op で `verify` 結果を報告）、
  `pre-*` → no-op、未知 event → error。project root は payload
  `cwd` → 呼出し cwd の順で解決する。malformed payload は明示
  error。lifecycle script は consumer `apm.yml` の `lifecycle:`
  に consumer が記述し `apm lifecycle trust` が前提（fail-soft で
  script 失敗は APM を abort しない — 強制機構ではなく収束
  トリガ）。untrusted・`APM_NO_SCRIPTS=1` 環境では CLI を直接呼ぶ
  手順を提供する。
- R16: consumer migration 手順 — (a) cheaper dep を
  `targets: [agent-skills]` に絞り project `targets` に
  `agent-skills` を含める（APM が reconcile で cheaper 所有の
  Windsurf hook artifact を除去する。project `targets` に
  `agent-skills` が無いと dep 全体が skip され skills も消える
  ため必須条件）、または (b) Devin の `read_config_from` で
  `windsurf`/`claude` の import を明示的に `false` にする。
  どちらも「Devin が旧 cheaper hook を読み込まなくする」ことを
  実現する。Windsurf IDE と Devin の両方で cheaper の guard を
  維持したい混合 consumer は (a) を選び（Windsurf target 自体は
  残るため IDE 側は維持され、Devin 側は `.devin/config.json` の
  native 登録で guard される）、Devin のみなら (b) でもよい。
  `verify` は残存する旧 cheaper 登録を検出してこの手順を案内
  する。
- R17: POSIX（Linux/macOS）で R10–R14 が動作し、PowerShell installer
  script は CLI 呼び出しへの薄い wrapper へ縮退する。POSIX consumer
  には CLI を直接呼ぶ手順を提供する。`hook-config` の書き込みは
  temp file + atomic replace（`fs::rename` は dest 存在時に
  置換）で、capability 確認との間の外部変更検出・`config.json.*.tmp`/
  `.bak` の orphan sweep を持つ。

## Acceptance Criteria

- [ ] `read {file_path, offset, limit}`（offset 1-based、limit 行数）
  で 8192 token 超のファイルに対し、ウィンドウが threshold 内なら
  allow される（全文 tokenize しない）。
- [ ] `offset`/`limit` 省略時は従来どおり全文評価で、大きい file は
  deny される（後退なし）。
- [ ] offset が総行数超過・EOF 超過の range は空ウィンドウとして
  allow、無効値は全文評価へ縮退することが test で固定される。
- [ ] `grep {path: 単一 file, pattern, max_results: 小}` が全文 read
  として扱われず、実返却内容が threshold 内なら allow、超過なら
  deny される。`output_mode` 非 content・`glob` 絞り込み・省略
  path(cwd)・context 行の扱いが test で固定される。
- [ ] devin `exec` payload が shared Shell policy（read candidate
  抽出を含む）で評価され、登録 matcher が `exec` を含む。
- [ ] `cheaper hook-config sync` → `verify` → `sync`（冪等）→
  `remove` が Windows と POSIX で動作し、無関係な key・event・
  hook group が保持される。
- [ ] malformed JSON・改変/重複 own entry・symlink 経路が明示 error
  で書き込まれず、concurrent modification を検出して中断する。
- [ ] APM consumer で cheaper dep の `targets: [agent-skills]` 化 →
  再 install が旧 Windsurf hook・sidecar を reconcile で除去し、
  再 install / update で再配布されない。他 package・他 target・
  ユーザー定義 hook が残る。
- [ ] lifecycle event stdin 受理が install/update/uninstall を同一の
  収束モデルへ写像し、uninstall で own registration が除去される。
- [ ] `cheaper-bulk-read` skill・`distribution.md`・先行
  spec/ADR/design/plan/task/impl-record が本 contract と矛盾しない
  記述へ更新されている。
- [ ] 計測失敗・timeout は allow（fail-open）、byte cap 超過・
  threshold 超過・scan cap 到達は deny（fail-closed）の境界が
  test で固定される。
- [ ] `--scope user` が `%APPDATA%\devin\config.json`（Windows）
  / `~/.config/devin/config.json`（POSIX）を対象にし、POSIX と
  Windows の path 解決が動作する。
- [ ] `hook-config` の書き込みが atomic replace・concurrent
  変更検出・orphan cleanup を持つ。
- [ ] post-uninstall の packages 判定と unknown event error、
  malformed payload error、payload `cwd` 起点の root 解決が
  test で固定される。
- [ ] 実 E2E scenario: new Devin-only consumer・existing
  Windsurf+Devin consumer・mixed consumer（other targets/deps
  あり）・reinstall・update・uninstall・trust 有無・旧 binary・
  failure/recovery・Windows・POSIX・`read(offset,limit)`/`grep
  (path,max_results)` の native stdin 受信経路が列挙されており、
  mock-only と real-CLI coverage が区別される。
- [ ] `.devin/hooks.v1.json` または `.devin/hooks.json` と
  `config.json` が併存する場合、`verify` が shadow の可能性を
  warn として報告する。

## Deferred Design Concerns

- Devin `glob` tool・`write`/`edit` 系 tool の hook 評価 | 現 review
  スコープ外・wire 証拠未取得 | Devin wire 観測 or 一次資料で tool
  名・payload shape が確定した時点 | 該当 tool が hook を素通りする
  gap が残る。
- global/ancestor scope の既存登録の完全検出 | 走査境界が無限に
  なり得る | Devin 公式に global hooks の確定的な探索規則が提供
  された場合 | 古い登録が気付かず残る可能性。
- `.devin/hooks.v1.json` と `.devin/config.json` の Devin 側優先
  順位 | 公式資料に未明記の shadow 挙動 | Devin docs/実機で
  precedence が確定した場合 | 同時存在時の発火順が未定義。

## Sources and Relations

- PR #109 owner review（comment-6221192905）— 10 項目の必須化。
- `docs/discovery/bounded-hook-read-request-contract.md` — wire
  証拠・APM 実機検証・tool 名一次資料の記録。
- `SPEC-034GssA0IjLKSiDdIHpAC8` — 先行 increment。本 spec はその
  Out of Scope だった #107 範囲評価・POSIX 登録経路・lifecycle/
  uninstall を引き継いで要求化する。
- `ADR-034rzTJYDRmre6D5OS8HDW` / `ADR-034GssAufIiId3hgh4jrSQ` —
  canonical request 構造と APM 配布 bootstrap の先行決定。本 spec の
  構造決定は新規 ADR で扱う。

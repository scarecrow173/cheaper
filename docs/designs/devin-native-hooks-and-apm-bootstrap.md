---
id: "DESIGN-034Gt74p3BYG40NTwu2kST"
type: "design"
status: "approved"
title: "Devin native hooks and APM bootstrap"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6183636932"
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
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../discovery/bounded-hook-read-request-contract.md"
    - "./overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/case-insensitive-native-tools-and-devin-registration.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
  derived-by:
    - "../plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  verifies: []
  verified-by:
    - "../test-specs/native-tool-name-normalization-and-devin-hook-contract.md"
    - "../test-specs/apm-distributed-devin-registration-preservation.md"
  references: []
  defers: []
  deferred-by: []
---

# Devin native hooks and APM bootstrap

## Context

approved spec SPEC-034GssA0IjLKSiDdIHpAC8 と accepted ADR
ADR-034GssAufIiId3hgh4jrSQ の要件を、既存 per-target adapter +
shared policy 構造（ADR-034rzTJYDRmre6D5OS8HDW）へ落とす機械
設計。#107 の fixture 観測（native `tool_input` に bounds、legacy
`tool_info` は `file_path` のみ、22419 tokens deny）は frozen input
で、本 design は native wire 側の前提を再検証しない。確定するのは
Rust interface、capability query、配布 matcher、Windows 登録補助
script の algorithm、および後段 plan へ渡す検証設計。

## Scope

### In Scope

- `crates/cheaper-cli` の `HookTarget::Devin` + `--target devin`、
  共有 `tool_use_normalize`、`devin_render`、capability query。
- `packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher
  更新（ASCII case-insensitive exact char class）。
- `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
  （Windows PowerShell 5.1、preview 既定 / `-Apply`）。

### Out of Scope

- #107 の範囲評価本体（`offset`/`limit` の policy 入力化、bounded
  grep の保証）、threshold 緩和、policy bypass。
- policy API / `RoutingRequest` / token count / threshold の変更。
- `.devin/hooks.v1.json` の生成（legacy `hooks.json` を shadow する
  既観測）、native exec / shell tool の hook 登録。
- POSIX installer、consumer 設定の自動移行、global/user 設定変更。

## Components and Boundaries

- `HookTarget::Devin`: `pub const ALL: [Self; 5]` を現行順
  `windsurf, copilot, pretooluse, cursor, devin` で定義。`parse` は
  `ALL` の `as_str` と比較して解決（`as_str` match が文字列
  authority）。既存 render 契約は不変。
- `targets.rs` 共有 normalizer:
  `fn tool_use_normalize(input: &Value, cwd: &str, source_prefix: &str, shell_tools: &[&str]) -> Option<RoutingRequest>`。
  root `tool_name` は string 必須、`tool_input` は object 必須。
  `tool_name` を `to_ascii_lowercase` で一度だけ正規化し、元 label は
  `source_prefix:original` として `source_tool` へ保持。
  `read` は `tool_input.file_path.as_str()`、`grep` は `path` 欠落で
  `"."`・非 string で `None`、case exact の `read`/`grep` →
  `FileRead`。`shell_tools.contains(normalized)` の時のみ
  `command.as_str()` → `Shell`。`cwd` は既存 policy が相対 path を
  解決する root で、`grep` の欠落 `path` の `"."` もそれに従う。
- Pretooluse adapter: prefix `pretooluse`、`shell_tools =
  &["bash","powershell"]`。Devin adapter: prefix `devin`、
  `shell_tools = &[]`（native exec 登録外）。Devin に限り CLI event
  `pre-tool-use` かつ payload `hook_event_name == "PreToolUse"` のみ
  受理、その他は silent allow。
- `devin_render(Verdict)`: `Deny` → `decision:"block"` + 共有
  `Verdict::Deny.reason` の passthrough を ordered JSON 化 + `'\n'`、
  exit 0、stderr なし。`Allow` / fallback は silent exit 0。
- `execute_hook` 入口の capability 分岐: argv 完全一致
  `["targets","--json"]` のみ stdin を読まず `ALL` から
  `{"schemaVersion":1,"targets":[...]}` を stdout 1 行 exit 0 で返す。
  通常 hook execution・usage 記録へ入らない native event ではない
  utility。他 argv は既存 parse → fail-open 維持。
- `install-devin-hooks.ps1`: `param([string]$ProjectRoot =
  (Get-Location).Path, [switch]$Apply)`。出力先は常に
  `$ProjectRoot/.devin/config.json`。script 自身は APM install /
  copy を行わない（APM が skill resource として verbatim 配布）。

## Data and Control Flow

1. wire JSON → root/`tool_input` 型検査（不正は `None` → fail-open）
   → `tool_name` lowercase → `source_tool = prefix:original` →
   tool 別 path 抽出 → `RoutingRequest`（`FileRead` または `Shell`）
   → 共有 evaluate → target render。
2. script: root を既存 directory の絶対 path へ解決 → root / `.devin` /
   `config.json` の各 path component の reparse・symlink 検査 →
   既存 config schema 検査（top object / `hooks` object /
   `PreToolUse` array / group object）→ known project config の
   conflict scan → preview 出力、または `-Apply` 時 capability
   handshake → temp file + atomic `File.Replace`（既存）/
   `File.Move`（新規）。
3. canonical entry:
   `{matcher:'^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$', hooks:[{type:'command',command:'cheaper hook pre-tool-use --target devin'}]}`。
   同一 command+matcher の比較は case-sensitive 文字列比較で no-op、
   own command の複数/異 matcher/追加 attributes/改変は非破壊 error。
4. conflict scan は既知 file のみ（`.windsurf/hooks.json`、
   `.devin/hooks.json`、`.devin/hooks.v1.json`、
   `.devin/config.local.json`、`.claude/settings.json`、
   `.claude/settings.local.json` + primary `config.json` の own
   canonical 以外）。nested `command`/`powershell`/`bash`/`windows`/
   `linux`/`osx` を辿り known cheaper read command を検出。shell-only
   の legacy `pre-run-command` は conflict としない。conflict 時は
   apply 前 error + 相対 config path 表示のみ。unparseable な対象も
   非破壊 error。
5. capability handshake（apply のみ）: `Get-Command cheaper/cheaper.exe
   -CommandType Application` で resolve → その exe の
   `hook targets --json` を捕捉 → exit 0・JSON object・
   `schemaVersion` が integer `1`・`targets` が全 string の array で
   exact `devin` を含むこと（判定は case-sensitive の `-ccontains`）。
   旧 CLI / empty / malformed / missing devin / unknown schema /
   nonzero は書込みなし error + install/update 案内。version 判定
   なし、capability の生出力はログしない。

## Non-Functional Constraints

- ASCII source/output、PowerShell 5.1、新規 dependency なし、
  APM 0.28.0 pin 維持。既存 comment の追加削除禁止（新 code にも
  comment を書かない）。
- preview は file/directory を一切作らず変更概要のみ、CLI 不在でも
  可。apply は全検証通過後にのみ `mkdir`/temp 書込み。
- `ConvertTo-Json` は `-InputObject` + `-Depth 100` で配列
  flatten/depth truncation を回避。既存 config 値・raw stdin・
  capability 生出力をログしない。
- 既存 config の未知 key/値/構造は semantic に保持するが、初回
  write での UTF-8 JSON format 正規化（key 順序・空白）は許容する。
  canonical entry 適用済みの 2 回目以降は bytes まで no-op。
- 書込み直前に原 bytes unchanged と reparse components を再検査
  （bytes 等価比較は `Convert.ToBase64String` 等を使い、PS 配列
  `-eq` の罠を避ける）、`finally` で自作 temp のみ cleanup。既存
  JSON parse error・競合 fail 時は bytes 全不変。

## Risks and Trade-offs

- **PS 5.1 の JSON 保全 gate（解消済み、probe 採用）**: PS 5.1 に
  既存の .NET `System.Runtime.Serialization.Json.
  JsonReaderWriterFactory` で、元 raw JSON と `ConvertFrom-Json` →
  `ConvertTo-Json -InputObject -Depth 100` の再 serialize 結果を
  それぞれ `XmlDocument.Load(reader).OuterXml` へ投影し `-ceq`
  比較する。失敗・差異があれば書かず保全 error。重複 key・
  escaped duplicate・precision loss・case collision を検出する。
  指数表記のように値が等価でも差異となる config は保守的に拒否
  する（known limitation）。native JSON parse が失敗する資料は
  当然拒否。exact helper は
  `tmp/issue107-json-runtime-probe.ps1` の `Get-JsonShape` を参照
  （同 function をそのまま利用可）。roundtrip 検査は既存 config へ
  apply し preview でも検査可。比較値はログしない。
  `ConvertFrom-Json` 同士の二度比較は禁止。
- **旧 CLI の silent allow**: executable 存在だけでは devin 対応を
  保証できないため capability handshake を apply 条件化済（F8）。
- **matcher 拡大で hook 発火頻度が増える**: `bash`/`powershell` も
  Pretooluse で発火するが、normalized + exact match + fail-open で
  早期 return。新規判定ロジックは増やさない。

## Deferred Design Concerns

- `offset`/`limit` の範囲評価・bounded grep 保証（#107 本体）|
  本 design は wire 上の bounds 保持まで | bounded output semantics
  の確認 + 別 spec/ADR | 件数上限を容量保証と誤用するリスク。
- POSIX installer・native exec/shell 登録 | 初版は Windows + read/
  grep のみ | 別 consumer の需要 | platform 差の放置。
- global/ancestor hook の完全検出 | project 既知 file の限定 scan |
  `/hooks` での利用者確認 | 見落としによる重複 deny。

## Verification Notes

- [ ] `crates/cheaper-cli/tests/hook.rs` の既存 `Fixture`/`run_hook`
      で read/grep/shell の各 case + huge/small/invalid/unknown、
      `grep` file/dir/omitted/nonstring、native event/response/
      reason・malformed fallback、`ALL` roundtrip、capability
      command の stdin 非読取・usage 非記録を assert。
      `mise exec -- cargo test -p cheaper-cli --features dev-seam
      --test hook`。
- [ ] `node --test scripts/test/devin-hooks-install.test.mjs`（Windows
      のみ実行・他 OS は明示 skip、fixture は repo `tmp` 下、PS 5.1
      full path spawn）。capability stub は `fixturebin/cheaper.cmd`
      （`Get-Command` が Application として認識する batch）で固定
      出力、PATH は fixturebin + 必要 systemdir のみ。native profile
      valid/invalid preflight、preview no-files、apply 追加 +
      既存 object/array/null/deep data 保持 + idempotent bytewise、
      legacy duplicate 各 source、shell-only legacy 非 conflict、
      changed own entry、bad JSON/schema、reparse point（junction）
      と secret-sentinel 非出現を assert。
- [ ] source matcher test を node:test で crossplatform 実行（各 case
      一致 + prefix/suffix 未知 tool 不一致）。
- [ ] 手動 targeted integration gate を 1 回: local-path dependency
      の isolated consumer から APM 0.28.0 `apm install --target
      agent-skills` → skill helper 到達 + hash 一致 + `.windsurf`/
      `.claude` hook 未生成を確認（network/model 不要、CI へは足さ
      ない）。

## Review Record

- 2026-10-15: review-protocol の role-separated pass（Reviewer A →
  Reviewer B → Validator → Judge）を実施。targeted correction（JSON
  保全 gate 採用・semantic 保持/format 正規化の明記・capability
  判定の case-sensitivity 等）適用後に P0/P1 なし、ACCEPT 条件を
  達成。修正後の文書・probe 結果・監査を lead が確認し、ユーザー指定の
  承認代替として approved へ更新。
- JSON 保全 probe `tmp/issue107-json-runtime-probe.ps1` の frozen
  結果: `ordinary` / `specialKey` = True（roundtrip equal）、
  `duplicate` / `escapedDuplicate` / `hugeNumber` / `float` /
  `exponent` = False（差異を検出。指数表記は保守的な拒否も含む）、
  `caseCollision` = rejected。

## JSON Preservation Helper

`install-devin-hooks.ps1` の保全 gate で使う canonical helper。
lead-authored `tmp/issue107-json-runtime-probe.ps1`（frozen 証跡）
から `Get-JsonShape` のみを一字変えず機械 copy したもので、実装は
この function をそのまま用いる（上位の probe 呼出・test case は
含めない）。比較値はログしない。

```powershell
function Get-JsonShape([string]$Text) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $reader = [System.Runtime.Serialization.Json.JsonReaderWriterFactory]::CreateJsonReader($bytes, [System.Xml.XmlDictionaryReaderQuotas]::Max)
    try {
        $document = New-Object System.Xml.XmlDocument
        $document.Load($reader)
        return $document.OuterXml
    } finally {
        $reader.Dispose()
    }
}
```

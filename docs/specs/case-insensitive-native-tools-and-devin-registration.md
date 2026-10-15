---
id: "SPEC-034GssA0IjLKSiDdIHpAC8"
type: "spec"
status: "implemented"
title: "Case-insensitive native tools and Devin registration"
created: "2026-10-15"
updated: "2026-10-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
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
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "./apm-delegation-skills-and-read-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bounded-hook-read-request-contract.md"
  derived-by:
    - "../designs/devin-native-hooks-and-apm-bootstrap.md"
    - "../plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  verifies: []
  verified-by:
    - "../test-specs/native-tool-name-normalization-and-devin-hook-contract.md"
    - "../test-specs/apm-distributed-devin-registration-preservation.md"
  references: []
  defers: []
  deferred-by: []
---

# Case-insensitive native tools and Devin registration

## What Should Be Built

issue #107 の partial prerequisite として、次の 3 つの変更を定義する。

1. **既存 Pretooluse adapter の tool 名正規化**: `tool_name` のみを
   ASCII lowercase へ一度だけ正規化してから exact match する
   （`toolName` 等の別 key は本 adapter の対象にしない）。各 tool の
   case variant（`read`/`Read`/`READ`/`rEaD`、`grep`/`Grep`/`GREP`/
   `gReP`、`bash`/`Bash`/`BASH`、`powershell`/`PowerShell` 等）は
   その tool 固有の既存 routing へ寄せ、`read`/`grep` は `FileRead`、
   `bash`/`powershell` は既存 shell routing を使う（tool 間で判定を
   統一しない）。元の tool label は source metadata へ保持してよい。
   `read` は `tool_input.file_path`（欠落・非 string は fail-open）、
   `grep` は `tool_input.path`（省略時は cwd の directory 要求として
   扱う。明示的に不正な型の場合は fail-open）を抽出する。`grep` も
   現在の `FileRead` policy へ接続し、単一 file は全文 token guard、
   directory は現行 allow とする。`pattern` / `max_results` /
   `context` / `glob` の再実行や出力量推定はしない。`read` の
   `offset`/`limit` も今回の評価には入れず全文 policy を維持する。
   未知 tool / malformed payload は現行どおり fail-open。配布元
   `packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher も
   ASCII case-insensitive な exact matcher
   （`^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Bb][Aa][Ss][Hh]|[Pp][Oo][Ww][Ee][Rr][Ss][Hh][Ee][Ll][Ll])$`）
   を登録する。他 target の契約は維持する。
2. **Rust `HookTarget::Devin`（`cheaper hook pre-tool-use
   --target devin`）の追加**: Devin CLI の native `PreToolUse` の
   `read`/`grep` を受け、共有 normalize へ入力して既存 evaluate へ
   接続する。native deny は `exit 0` + stdout へ
   `{"decision":"block","reason":"..."}` 1 行（Devin 公式 overview
   contract）。`reason` には共有 policy の `Verdict::Deny.reason` を
   改変なく passthrough し、Devin 用の第二の guidance は作らない。
   allow / error fallback は silent `exit 0`。
   source metadata の prefix は `devin`。CLI の enum parse / `as_str` /
   dispatcher 追加は既存 pattern に従う。共有 policy は複製せず、
   native response rendering のみ別にする責務境界を維持する。
   normalizer 構成の詳細は design 段階で決定する。
3. **read-only capability query `cheaper hook targets --json`**: exit
   0 + stdout へ `{"schemaVersion":1,"targets":[...]}` の 1 行 JSON
   object を返す。stdin / model / usage / policy を触らず、`targets`
   は `HookTarget` の canonical supported list を単一 source として
   生成する（version string では capability を判定しない。APM skill
   と Cargo binary は独立 install のため旧 binary が残り得る）。

加えて、上記 Devin target を consumer project へ登録する補助 script
`packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
を APM の skill bundle resource として配布する。消費側の操作は
consumer の状態で分ける:

- **新規 Devin-only consumer**: `apm install --target agent-skills`
  の後に `powershell -NoProfile -ExecutionPolicy Bypass -File
  .agents/skills/cheaper-usage/scripts/install-devin-hooks.ps1
  [-ProjectRoot <repo-root>] [-Apply]` を実行する。
- **既存 consumer（manifest で他 target を所有）**: 既存 targets を
  保持したまま通常の `apm install` を実行し、deploy 済みの
  cheaper-usage helper を実際の deploy path から呼ぶ。target を
  置き換えて helper の場所を得ようとしない。`--legacy-skill-paths`
  を選んでいる場合も既存 target の deploy path を使う。

## Why Now

#107 の実測で、native `read`/`grep` の bounds（`offset`/`limit`/
`max_results`）は native `PreToolUse` wire の `tool_input` に存在するが、
Windsurf 互換 `pre_read_code` payload は `file_path` のみで Devin 側の
compat 変換で消失することが確定した（
`docs/discovery/bounded-hook-read-request-contract.md`）。範囲評価の本体
とは別に、まず（a）lowercase `read`/`grep` が現行 Pretooluse adapter で
未認識（silent allow）になる欠陥と、（b）Windsurf 互換を介さない Devin
native 登録経路の不在を解消する必要がある。

## Users and Value

- Devin CLI consumer（code-knowledge 等）: `read`/`grep` の tool 名
  case に依存せず large-file guard が適用され、Windsurf 互換 config を
  置かなくても native `PreToolUse` で cheaper の guard が動く。
- 親 Coding Agent / ユーザー: `.devin/config.json` への登録が
  preview 既定・冪等・非破壊の補助 script で行える。

## Scope

### In Scope

- Pretooluse adapter の `tool_name` ASCII lowercase 正規化 + exact
  match（各 tool の case variant をその tool 固有の既存 routing へ
  寄せる: read/grep → `FileRead`、bash/powershell → 既存 shell）。
- `grep` を `FileRead` policy へ接続（`tool_input.path`、単一 file は
  全文 token guard、directory は現行 allow、省略時 cwd directory、
  不正型は fail-open）。
- 配布元 `packages/apm/.apm/hooks/hooks.json` の `PreToolUse`
  matcher を ASCII case-insensitive exact（`^(read|grep|bash|
  powershell)$` の char class 版）に更新。
- `HookTarget::Devin` / `--target devin` の追加（native `PreToolUse`
  の `read`/`grep` → 共有 normalize → 既存 evaluate。deny は exit 0 +
  `{"decision":"block","reason":"..."}` 1 行、allow/error は silent
  exit 0）。
- read-only capability query `cheaper hook targets --json`
  （`{"schemaVersion":1,"targets":[...]}` 1 行、stdin/policy/model
  なし、`HookTarget` canonical list を単一 source とする）。
- `install-devin-hooks.ps1`（Windows PowerShell 5.1 用初版）を
  `.apm/skills/cheaper-usage/scripts/` 配下に追加し、APM skill bundle
  経由で配布する。default は preview、`-Apply` でのみ登録。
- `.devin/config.json` の `hooks.PreToolUse` へ
  `command: "cheaper hook pre-tool-use --target devin"` を
  case-insensitive exact matcher（read/grep、同様の char class）・
  hook `type: "command"` で追加。既存の無関係 config / 他 event /
  他 hook group は保持し、同一 command+matcher の再実行は no-op、
  改変済み・曖昧な既存 own entry があれば非破壊 error とする。
- 既知の legacy cheaper read hook 登録元（`.windsurf/hooks.json` /
  `.devin/hooks.json` / `.devin/hooks.v1.json` /
  `.devin/config.local.json` / `.claude/settings*.json` 等）を限定
  scan し、cheaper 互換 read / 他 target の重複があれば apply を停止
  して場所と解消手順を通知する（削除や provider 一括無効化はしない）。
- script 内の notice: global/ancestor の完全検出は保証しないため
  `/hooks` builtin での確認を促すこと、この登録は APM lockfile の
  ownership に記録されない補助登録であり、APM uninstall 後の解除は
  この command group の手動削除が必要であること。

### Out of Scope

- #107 の範囲評価本体（native `read` の `offset`/`limit` を policy
  入力とする評価、bounded grep の保証）。
- threshold 緩和、policy bypass、consumer 設定の自動移行。
- global / user-level Devin 設定の変更、`.devin/hooks.v1.json` の生成
  （legacy `hooks.json` を shadow する既観測があるため書かない）。
- 既存 legacy shell hook の削除、`.windsurf` / `.claude` config の新規
  生成。
- APM 本体への Devin target 追加・package resolver/copier/download の
  新規実装、APM internals の monkeypatch。APM 0.28.0 pin 維持・新規
  dependency なし。
- POSIX installer、native exec / shell tool の hook 登録。
- issue #107 の close（`Refs #107` のみ、`Closes` 不使用）。

## Requirements

- tool 名の正規化は `tool_name` のみを対象とし、ASCII lowercase への
  単一変換で行い、判定は normalized 値への exact match とする。部分
  一致・regex 判定を adapter 側へ持ち込まない。判定先は tool 固有の
  既存 routing とする（`read`/`grep` → `FileRead`、`bash`/
  `powershell` → 既存 shell routing。tool 間で判定を統一しない）。
- `read` は `tool_input.file_path`（欠落・非 string は fail-open）、
  `grep` は `tool_input.path` を使う。`path` 省略時は cwd の
  directory 要求として扱い、directory は現行の `is_file() == false`
  allow 枝を維持する。
- `HookTarget::Devin` の応答 contract は Devin 公式の native hook
  overview に従い、deny は exit 0 + stdout `{"decision":"block",
  "reason":"..."}` 改行、allow / error は silent exit 0（exit 2 を
  使わない）。`reason` には共有 policy の `Verdict::Deny.reason` を
  改変なく passthrough し、Devin 用の第二の guidance を作らない。
- 共有 policy・判定ロジックを Devin 用に複製しない。差分は payload
  normalize と response rendering のみ。
- `cheaper hook targets --json` は read-only の capability query と
  し、exit 0 + stdout 1 行 JSON object `{"schemaVersion":1,
  "targets":[...]}` を返す。stdin / model / usage / policy を触らず、
  `targets` は `HookTarget` の canonical supported list を単一
  source から生成する。
- installer の `-Apply` は executable 存在だけでなく、
  `cheaper hook targets --json` の exit 0・JSON object・
  `schemaVersion: 1`・`targets` が string array で exact `"devin"` を
  含むことを書込み前の条件とする。CLI 不在・nonzero exit・empty /
  malformed 出力・unknown `schemaVersion`・`devin` 欠落は非破壊
  error とし、Cargo install/update 案内を出して config を一切書か
  ない。version string では判定しない。default preview は CLI 不在
  でも「何を追加するか」を表示できてよい（capability 検査が必須な
  のは apply 時のみ）。
- config 書込みは temp file + atomic replace とし、既存 JSON の
  parse/schema 異常や symlink / reparse point の path には書かず
  error とする。preview は既存 config の値を出力せず変更概要のみ
  表示する。
- matcher 登録は read/grep の ASCII case-insensitive exact とする
  （bash / powershell の matcher 登録は hooks.json 側の Pretooluse
  matcher 更新のみで、Devin installer が書く matcher 対象は read /
  grep）。
- 消費手順は consumer の状態で分ける。新規 Devin-only consumer は
  `apm install --target agent-skills`、既存 consumer は manifest の
  targets を保持して通常の `apm install` を行い、deploy 済み helper
  を実際の deploy path（`--legacy-skill-paths` 時は既存 target の
  path）から呼ぶ。`apm install` step 自体の reconciliation 副作用
  （dropped target の cleanup）は helper の非破壊保証とは別の責務と
  して扱い、helper の記述に混同しない。legacy read hook の競合は
  検出・通知までで自動解消しない。

## Acceptance Criteria

- [ ] Pretooluse adapter が `Read`/`read`/`READ`（および `Grep`/`grep`/
      `GREP`、`Bash`/`bash`、`PowerShell`/`powershell`）の全 case
      variant で、その tool 固有の同一 routing・同一判定を返す
      （単体 test で検証）。
- [ ] `tool_input {file_path, offset, limit}` を持つ lowercase `read`
      payload が deny/allow 判定を受ける（silent allow でない）こと。
      `file_path` 欠落・非 string は fail-open であること。
- [ ] `tool_input {path, pattern, max_results}` の `grep` で、単一
      file は全文 token guard、directory は allow、path 省略は cwd
      directory 要求、非 string path は fail-open になること。
- [ ] `packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher が
      read/grep/bash/powershell の ASCII case-insensitive exact
      形式であること。
- [ ] `cheaper hook targets --json` が exit 0 + stdout 1 行 JSON
      object（`schemaVersion: 1`、`targets` が string array で
      `devin` を含む）を返し、stdin / model / usage / policy を起動
      しないこと。
- [ ] `cheaper hook pre-tool-use --target devin` に native `PreToolUse`
      の read payload（bounds 含む）を与えると、threshold 超過 file で
      exit 0 + stdout `{"decision":"block","reason":...}` 1 行
      （`reason` は共有 `Verdict::Deny.reason` の passthrough）、
      threshold 未満 / malformed / 未知 tool で silent exit 0 になる
      こと。
- [ ] `install-devin-hooks.ps1` が default で preview のみ（config
      値を出力しない変更概要。CLI 不在でも表示可）を行い、`-Apply` で
      `.devin/config.json` の `hooks.PreToolUse` に指定 entry を追加
      し、同一 command+matcher の再実行で no-op、無関係な既存 config
      を保持すること。
- [ ] `-Apply` が `cheaper hook targets --json` の成功（exit 0・
      `schemaVersion: 1`・`targets` に exact `"devin"`）を書込み条件
      とし、CLI 不在・旧 binary・nonzero・malformed・`devin` 欠落で
      非破壊 error + Cargo install/update 案内となり config を書か
      ないこと。version string で判定しないこと。
- [ ] 既存 JSON の parse 異常 / symlink・reparse point / 改変済み
      own entry で非破壊 error で停止すること。
- [ ] `.devin/hooks.v1.json` が生成されないこと、既知 legacy 登録元の
      重複検出時に apply が停止して場所と解消手順を通知すること。
- [ ] 既存の全 hook 契約 test suite（windsurf / copilot / claude /
      codex / pretooluse の deny / allow / fail-open 出力契約）が
      変更後も pass すること。
- [ ] script / managed output が repo 内 test fixture に対してのみ
      実行されること（検証は fixture に限定）。

## Deferred Design Concerns

- native `read` の `offset`/`limit` 要求範囲を policy 入力として評価
  する範囲評価（#107 本体。wire に bounds が存在することは確定済み、
  bounded output semantics の保証は未確認）| 本 spec は partial
  prerequisite のみ | bounded output semantics の確認・別 spec/ADR の
  approval | 早期接合を強行すると fail-open 経路を残したまま誤判定を
  生むリスク
- `grep` の `max_results` は件数であり byte/token 上限ではない
  （設計制約として記録済み）| 出力量ベースの評価は本 spec 対象外 |
  native grep の実 output 契約の確定 | 件数を容量保証と誤用するリスク
- 実 consumer における他 provider（`.claude` 等）との重複 deny |
  installer の重複 scan は検出して停止するが自動解消しない | consumer
  環境での実測 | 複数 hook からの重複 deny/干渉が残るリスク

## Sources and Relations

- Issue #107 と comment `issuecomment-6183636932`（fixture 再現結果の
  一次情報: native `tool_input` に bounds あり、legacy `tool_info` は
  `file_path` のみ、22419 tokens で deny）。
- `docs/discovery/bounded-hook-read-request-contract.md`（`derives-from`、
  wire 観測・消失地点確定の canonical discovery）。
- `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`
  （`related`、本 spec の登録経路の構造決定）。
- `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`
  （`related`、per-target adapter + shared policy の継承元）。
- `docs/specs/apm-delegation-skills-and-read-hook.md`（`related`、APM
  skill bundle 配布の既存契約）。

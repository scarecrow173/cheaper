---
id: "DISC-034FRguX1hjhDZ7ZMEBnbm"
type: "discovery"
status: "resolved"
title: "Bounded hook read request contract"
created: "2026-10-14"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
    - "https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6172569369"
    - "https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6182313362"
    - "https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6183636932"
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
    - "https://docs.devin.ai/desktop/cascade/hooks"
    - "https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks"
    - "https://docs.devin.ai/cli/extensibility/hooks/overview"
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
  derives-from: []
  derived-by:
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "../specs/case-insensitive-native-tools-and-devin-registration.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../specs/apm-delegation-skills-and-read-hook.md"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "issue #107 の short range read / bounded grep が hook で deny される問題について、評価境界（要求範囲 vs ファイル全文）を確定する"
    scope: "Devin CLI consumer の native hook wire と既存 Rust adapter の突合、RoutingRequest の範囲表現の有無、比較案の棚卸し"
---

# Bounded hook read request contract

## Exploration Goal

issue #107: Devin CLI consumer（code-knowledge、Windows、APM commit
`047165bb`）で、native `read` が `offset 334` / `limit 81` を指定しても
ファイル全文（11809 tokens）として deny され、`grep` の
`max_results 85` 指定も deny されたと報告されている。2026-10-15 に
最小 fixture での native wire 観測・拒否再現を完了した
（issuecomment-6183636932）。

本 discovery の目的は、bounded read / bounded grep 要求に対して hook が
何を評価すべきか — 要求された範囲か、ファイル全文か — の境界を確定
すること。

Entry Decision: A-3 (Combined Discovery) — 報告された consumer 入力と
native hook 仕様と Rust adapter の突合が必要なため。

Active stack: systematic-debugging / discovery-doc。spec-doc / adr-doc は
stack に含まれ、wire 境界の証拠取得済みのため次の step として要求・
方針の formalize / review に進む状態（approval review 未実施、本文書
では設計・実装を決定しない）。

## Key Issues and Assumptions

### コード確認（confirmed: 実読）

- `crates/cheaper-cli/src/hook/targets.rs` — 全 adapter が
  `RoutingRequest::FileRead` へ `path` / `cwd` / `source_tool` のみを渡す。
  Windsurf `pre_read_code` は `tool_info.file_path` のみ、Pretooluse
  `Read` は `tool_input.file_path` のみ、Copilot `view` は `path` のみを
  抽出し、`offset` / `limit` / `max_results` 等の bounds はすべて捨てる。
- `crates/cheaper-cli/src/hook/routing_policy.rs` — `RoutingRequest` に
  範囲を表現する field は存在しない。`evaluate` → `evaluate_file_read` →
  `measure_tokens` はファイル全文の token 数でのみ判定する。
- `targets.rs` の `PRETOOLUSE_READ_TOOLS = &["Read"]` は case-sensitive
  であり、小文字 `read`・`grep` に対応する adapter mapping は存在しない
  （grep を解釈する adapter は全 target に無い）。

### 合成再現（confirmed: 一時 integration test で実測）

`run_cli` を直接駆動する一時 test で、以下を確認済み:

- windsurf `pre_read_code` `tool_info {file_path, offset:334, limit:81}`
  → exit 2 deny（`20002 tokens (threshold 8192)` — offset/limit を
  無視して全文 count）
- pretooluse `Read` `tool_input {file_path, offset:334, limit:81}`
  → `hookSpecificOutput.permissionDecision: "deny"`（同上）
- pretooluse `read`（小文字）
  `tool_input {file_path, offset:334, limit:81}` → silent allow
  （exit 0、stdout/stderr とも空）
- pretooluse `grep` `tool_input {path, pattern, max_results:85}`
  → silent allow（exit 0、stdout/stderr とも空）

ただし合成 fixture は `"lorem ipsum dolor sit amet "` の 4000 回反復 +
末尾改行のみの単一長大行であり、`offset 334` は EOF 後を指す。報告さ
れた「実際に短い 81 行だけを読む」ケースの再現ではない点に注意。

#### 合成再現 round 2（2026-10-15 追加、複数行 fixture）

複数行 fixture（`("lorem ipsum dolor sit amet " × 8) + "\n"` の 500 行
反復、`ws/src/big.txt`）で再実測:

- 要求範囲に相当する 81 行（`lines().skip(333).take(81)`）は
  **3402 tokens ≤ 8192**、ファイル全文は **21000 tokens > 8192**。
- windsurf `pre_read_code` path-only `tool_info {file_path}` →
  exit 2 deny（`21000 tokens (threshold 8192)`）。
- 合成 `tool_name "read"` + `tool_info {file_path, offset:334, limit:81}`
  → **同一 deny reason**（bounds と tool_name は adapter が捨てる）。
- 合成 `tool_name "grep"` +
  `tool_info {file_path, pattern, max_results:1}` → **同一 deny reason**。
- 同 `tool_info` で `file_path` を非 root directory（`src`）に変更 →
  allow（`metadata.is_file() == false` branch、code + synthetic で
  confirmed。real consumer でこの branch か no-hook かは未確定）。
- `agent_action_name` 無し・`tool_info` 無し・`file_path` 無し → allow。

注: metadata を持つ合成 payload は raw native wire の再現ではない。
現行 adapter がそのような field をどう扱うかを示すだけである。

### 実 consumer 観測（confirmed: reporter 実測、2026-10-15）

issue #107 comment `issuecomment-6182313362` の独立観測窓より（Devin
3000.11.3、cheaper.exe version 1.0.0）:

- native `read {file_path, offset 334, limit 81}` と native
  `grep {path, pattern "^class WikiGraphProjectionFacade", max_results 1}`
  の双方で、実 process として
  `.windsurf/hooks/apm/cheaper.ps1 hook pre-read-code --target windsurf`
  および shim `cheaper.exe` の同 args 起動が観測され、両方とも
  **11809 tokens で deny**。つまり read/grep の実 command は Windsurf
  `pre_read_code` 経路を通った（reporter 実測として confirmed）。
- 対照: native directory + `glob_pattern projection.py`（同 pattern、
  `max_results 1`）は 1 行 allowed。ただしこの対照では hook 発火・
  stdin は未観測（directory branch 到達か no-hook かは未確定）。
- Devin startup で `.claude/settings.json` の unknown variant
  `pre_read_code` parse error を観測。これは別の配布/互換境界の観測と
  して区別する — 「parse error が Windsurf 経路へ fallback させた」と
  いう因果は未証明（provider 独立 load の可能性がある）。
- hook discovery: global Devin 8 件 / project Windsurf 2 件。
- **raw stdin は未取得**（この comment 時点の観測としての履歴。後述の
  fixture 再現で取得済み）。native の引数名は wire の field 名とは
  扱わない。

### 実 native wire 観測（confirmed: 最小 fixture 再現、2026-10-15）

実 Devin CLI 3000.11.3（9c803229faa4）で拒否を再現し、raw stdin の
sanitized record を初取得した（issuecomment-6183636932）。保存済み
証跡: `tmp/issue107-native-repro/legacy-wire.jsonl`・
`native-wire.jsonl`・`tmp/issue107-native-run1.log`。

再現条件:

- 場所: gitignore 済み `tmp/issue107-native-repro`（独立した一時 Git
  root — 親 repo `F:\repositry\cheaper` の `.windsurf/hooks.json`
  混入を除外するため）。`code-knowledge`・global 設定・閾値・policy
  は無変更。
- fixture: 500 行 synthetic `src/agents/projection.py`（22419 tokens
  > 8192 threshold）。
- hook 設定は `.devin/` 内のみ: legacy-form `.devin/hooks.json` に
  `pre_read_code` → 元 canonical `cheaper.ps1 hook pre-read-code
  --target windsurf`（先頭に capture 挿入）、`.devin/config.local.json`
  に native `PreToolUse` matcher `^(read|grep)$` の metadata-only
  observer（child は no-op `pass`、deny 経路を上書きしない）。
  `/hooks` builtin で双方が `pre_tool` event に登録されることを確認。
- 実測で判明した登録 semantics: 同一 `.devin/` 内では
  `hooks.v1.json` が legacy `hooks.json` を shadow する（両者共存時は
  v1 のみ load。rename 判別済み）。そのため observer は
  `config.local.json` の `hooks` key へ配置した。
- 実行: `devin --permission-mode auto --prompt-file ... --print` の
  1 invocation のみ（18.8s、exit 0）。prompt は逐次 3 calls のみを
  指示した lead-authored diagnostic。
- privacy: log は field 名・型・数値 bounds・既知 label・対象種別
  （fixture-file/directory）・元 hook の exit code のみ。path・
  pattern・prompt・credential・session ID の値は記録していない
  （記録した `session_id`/`prompt_id`/`execution_id` 等は key 名と
  type のみ）。

結果:

| native call | native `tool_input` keys | native bounds | legacy `tool_info` keys | 元 hook exit |
|---|---|---|---|---|
| read | file_path, limit, offset | limit=81, offset=334 | file_path のみ | 2（deny: 22419 tokens） |
| grep（単一 file） | max_results, path, pattern | max_results=1 | file_path のみ | 2（deny: 22419 tokens） |
| grep（directory+glob） | glob_pattern, max_results, path, pattern | max_results=1 | file_path のみ（directory） | 0（allow） |

**native wire には bounds が存在し、legacy `pre_read_code` wire には
`file_path` のみしか存在しない。** legacy record の root および
`tool_info` の `otherKeyCount` はともに **0** — 未記録の未知 key に
範囲情報が隠れていたのではなく、互換 payload 自体が `file_path`
のみである。native `tool_input` の `otherKeyCount` も 0。native root
には未 whitelist の key が 1 件存在するが（`otherKeyCount:1`、name/
value 未観測）、範囲情報が `tool_input` に存在することと互換側の key
集合は観測できている。

保存済み record（sanitized 値のみ。canonical 保持として原 record の
構造部分を転記）:

legacy 3 records 共通の payload 構造:

```json
{
  "payload": {
    "type": "object",
    "knownKeys": ["agent_action_name", "execution_id", "timestamp", "tool_info", "trajectory_id"],
    "otherKeyCount": 0,
    "fieldTypes": {"agent_action_name": "string", "execution_id": "string", "timestamp": "string", "tool_info": "object", "trajectory_id": "string"},
    "labels": {"agent_action_name": "pre_read_code"}
  }
}
```

legacy `tool_info`（3 records すべて同一）と各 call の exit/target:

```json
{"tool_info": {"type": "object", "knownKeys": ["file_path"], "otherKeyCount": 0, "fieldTypes": {"file_path": "string"}, "labels": {}, "bounds": {}, "containers": {}}}
```

- read → `exitCode: 2`, `target: "fixture-file"`
- grep（file）→ `exitCode: 2`, `target: "fixture-file"`
- grep（directory）→ `exitCode: 0`, `target: "fixture-directory"`

native 3 records 共通の payload 構造（3 record 共通部分を示す図で
あり、完全な raw record 引用ではない。`labels.tool_name` は call 別に
`read` / `grep` / `grep`）:

```json
{
  "payload": {
    "type": "object",
    "knownKeys": ["hook_event_name", "prompt_id", "session_id", "tool_input", "tool_name"],
    "otherKeyCount": 1,
    "fieldTypes": {"hook_event_name": "string", "prompt_id": "string", "session_id": "string", "tool_input": "object", "tool_name": "string"},
    "labels": {"hook_event_name": "PreToolUse"}
  }
}
```

native `tool_input`（call 別）と exit/target（observer child は全件
`exitCode: 0`）:

```json
{"tool_input": {"type": "object", "knownKeys": ["file_path", "limit", "offset"], "otherKeyCount": 0, "fieldTypes": {"file_path": "string", "limit": "number", "offset": "number"}, "bounds": {"limit": 81, "offset": 334}}}
```

```json
{"tool_input": {"type": "object", "knownKeys": ["max_results", "path", "pattern"], "otherKeyCount": 0, "fieldTypes": {"max_results": "number", "path": "string", "pattern": "string"}, "bounds": {"max_results": 1}}}
```

```json
{"tool_input": {"type": "object", "knownKeys": ["glob_pattern", "max_results", "path", "pattern"], "otherKeyCount": 0, "fieldTypes": {"glob_pattern": "string", "max_results": "number", "path": "string", "pattern": "string"}, "bounds": {"max_results": 1}}}
```

各 target は read/file-grep が `fixture-file`、directory grep が
`fixture-directory`。

確定した責任境界:

- **metadata 消失地点は Devin 側の native → Windsurf compatibility
  変換** — cheaper の Rust adapter が受信する前の境界で、tool の種類・
  範囲・検索上限が失われる（この fixture で実測）。受信していない
  metadata は復元できないため、Rust の Windsurf adapter に
  offset/limit の読み取りを足すだけでは今回の native consumer の症状
  は直らない。
- directory + glob の対照では legacy hook も発火し exit 0 —
  `metadata.is_file() == false` で allow する code の枝と整合
  （この fixture では confirmed。reported consumer 側の同一経路かは
  別観測）。
- config 読込場所の差異: 本 fixture の legacy config は
  `.devin/hooks.json`、報告元は `.windsurf/hooks.json`（APM deploy）。
  同一 CLI version・同一 Windsurf-compatible event・同一 canonical
  launcher/target で元の 3 症状と metadata 消失を実測したが、報告元
  環境そのものの raw stdin を取得した主張ではない。
- global hooks 8 件は無変更。fixture では legacy + native observer の
  2 つの `pre_tool` hook が同一 3 calls に発火したが、実 consumer の
  provider 重複（`.claude` 等）や重複 deny は依然未観測（注意事項
  として保持、blocker とはしない）。

### wire 観測 probe（利用可能）

`tmp/issue107_hook_probe.py` — one-off debug probe（製品修正ではない）。
raw stdin bytes を元 command へ `subprocess.run(input=raw)` でそのまま
forward し、stdout/stderr/exit code（0/2）を不変で継承する。log は
whitelist 化された既知 field 名・値の type・認識済み event/tool label・
有限数値 bounds のみ。file path・pattern・prompt・credential・
session ID・未知 key の value は記録せず、knownKeys 以外は
`otherKeyCount` のみ。depth 2 container まで。

privacy 上の限界: 未知 key が `otherKeyCount > 0` として残る・未観測
container がある場合、「wire に bounds が無い」とは断定できない。
既存 CLI log（`mod.rs`）は event/target/verdict・出力 byte length のみ
で payload field は診断不能。

probe の検証は `tmp/test_issue107_probe.py`（6 test）で PASS:
既知 field の knownKeys/bounds/labels 抽出、invalid-json marker、
unknown label の `<other>` 化、非数値 bounds の bounds 除外、
`view_range` の viewRange 化、array root の type のみ記録、log 書込
失敗時の fail-open、raw stdin の byte-for-byte forward と
exit/stdout/stderr 不変（`python -c` stub 経由と、絶対 python `-I -S`
+ `powershell -NoProfile -File` receiver chain で、日本語・CRLF 含有
UTF-8 bytes の byte-for-byte 同一に加え、valid JSON + Unicode + CRLF
metadata の passthrough と log 側の bounds 抽出・sentinel 非出現を
確認）。

consumer 側での利用例（一時的な hook command 置換案。`mise exec` を前置
すると PATH/env が変わり `Get-Command cheaper` の解決先が変わり得る
ため禁止。`--` 以降は元 command を一切変更しない）:

```powershell
& "C:\Users\akiyoshi\AppData\Local\mise\installs\python\3.12.14\python.exe" -I -S "F:\repositry\cheaper\tmp\issue107_hook_probe.py" --log "F:\repositry\cheaper\tmp\issue107-hook-wire.jsonl" -- powershell -NoProfile -ExecutionPolicy Bypass -File ".windsurf/hooks/apm/cheaper.ps1" hook pre-read-code --target windsurf; exit $LASTEXITCODE
```

実際の取得では consumer 側採取は不要になり、fixture 内 variant
`tmp/issue107-native-repro/wire_capture.py`（同 formatter を import し
`exitCode`/`target` を追加記録、child 実行後に log）を使用した。
その smoke test は `tmp/test_issue107_wire_capture.py` で PASS
（byte-for-byte stdin・exit 2・stdout/stderr 不変・record の
exitCode/target/bounds・sentinel 非出現）。

### 未確認事項

2026-10-15 の fixture 再現で以下は解消済み（後述の履歴として保持）:

- stdin の wire field 名と bounds の有無: native `PreToolUse` では
  `tool_input` に `offset`/`limit`/`max_results` 等が存在し、legacy
  `pre_read_code` では `tool_info` が `file_path` のみ（root・
  tool_info とも `otherKeyCount:0`）と確認。
- 消失地点: Devin 側の native → Windsurf compat 変換と確認
  （fixture 実測。Rust 正規化より上流）。

残る未確認:

- 指定範囲を実 tool が尊重する保証。directory + glob_pattern 対照の
  1 行 allowed は bounds 尊重 semantics の保証としては不十分。また
  `max_results` は行数/件数であり byte/token 上限ではないため、
  その存在だけで安全とは断定しない（設計論点として保持）。
- 報告元 consumer 環境そのものの raw stdin（fixture は同一 CLI
  version・同一 event/launcher で再現したが、報告元環境の wire を
  取得した主張ではない）。
- 実 consumer での provider 重複発火・重複 deny（`.claude` 等）。
  fixture では legacy + observer の dual firing を実測したが、reported
  環境での emit 重複は未知。

初期 hypothesis（2026-10-14）の履歴: 「`grep` deny は現行 adapter
の silent allow と矛盾し別経路の可能性」としていた。初期の silent
allow 対照は native read/grep を Pretooluse に入れた合成例であり、
実際の Windsurf 経路の拒否とは比較対象が異なる。判明した Windsurf
adapter の全文 guard は read/grep 双方の拒否を説明し、消失地点も
compat 変換と確定した。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (1) native hook で本来の tool_input と requested-output bounds を保持し、共通 policy が要求量を評価する | 範囲指定された read/grep を要求量だけで判定でき、正当な bounded read を許可できる。native wire の field 構成（`tool_input.offset/limit/max_results`、matcher `^(read|grep)$` での登録可否）は fixture で実測済み | 実 tool が範囲を尊重する semantics（bounded output 保証）の証明は依然必要。`max_results` は件数であり byte/token 上限ではない | maybe |
| (2) path-only legacy は従来どおり全文 guard を維持し、bulk-read skill 側で suggestedReads の使用可能条件を明示する | 現行の fail-closed 契約を維持でき安全 | native exact-read の可用性は解決しない | maybe |
| (3) threshold 緩和 / bypass | 実装最小 | huge-file token count guard の契約を損なう | no（採用候補外） |

## Tentative Conclusions and Hypotheses

- **confirmed**: 範囲 metadata は既存 read adapter で保持されない
  （`targets.rs` が offset/limit 等を拾わず、canonical request にも
  範囲 field が無い）。
- **confirmed（2026-10-15、reporter 実測）**: read/grep の実 command は
  Windsurf `pre_read_code` hook 経路を通り、両方全文 count で deny。
- **confirmed（2026-10-15、fixture 実測）**: 拒否の機構は
  operation/bounds を区別しない全文 guard であり、metadata の消失
  地点は Devin 側の native → Windsurf compat 変換（Rust adapter が
  受信する前の境界）。native wire の `tool_input` には bounds が存在
  するため、native `PreToolUse` 経路で受ければ要求範囲を policy 入力
  として利用できる。Rust adapter への bounds 読み取り追加だけでは
  この native consumer の症状は解消しない。
- **未確定（hypothesis のまま保持）**: 実 tool が要求範囲を尊重する
  semantics（bounded output 保証）— 設計時の論点。
- **設計制約**: `max_results` は件数上限であり byte/token 上限では
  ない。
- 最終決定はここでは置かない。次の検討対象（lead 判断）は:
  metadata を保持する native `PreToolUse` 経路で受けること、
  legacy path-only guard との重複拒否を発生させないこと、native
  read の要求範囲を共通 policy の入力として表現すること、grep の
  結果上限を全文サイズと混同しないこと（`max_results` だけで安全と
  断定しない）、path-only legacy payload では suggestedReads を
  自動許可できない制限を配布 skill に明示すること。これらは
  doc-driven-dev graph の文書ゲートで設計・review してから修正する。

## Open Questions

2026-10-15 の fixture 再現で wire 関連の blocker は解消（下記履歴）。
残る pre-implementation の open item:

- 指定範囲を実 tool が尊重する保証。directory + glob_pattern 対照の
  1 行 allowed は bounds 尊重 semantics の保証としては不十分。
  `max_results` は件数であり byte/token 上限ではない。
- 実 consumer での provider 重複発火・重複 deny（fixture では
  legacy + native observer の dual `pre_tool` firing を実測したが、
  reported 環境の `.claude` 等他 provider との emit 重複は未知。
  global hooks 8 件は無変更・今回の 3 calls で追加 deny は観測
  されなかったため、注意事項として保持するが blocker としない）。

解消した項目（履歴）:

- hook command の経路 → Windsurf `pre_read_code`（reporter 実測 +
  fixture で再確認）。
- stdin の field names / bounds の wire 上の表現 → fixture で raw
  stdin を sanitized record として取得。native `tool_input` に
  bounds あり、legacy `tool_info` は `file_path` のみ。
- metadata の消失地点 → Devin 側 compat 変換（fixture 実測）。

## Promotion Candidates

- [ ] spec-doc needed: bounded read/grep 要求に対する評価境界の要件
      （wire 確定済み — formalize / review は次 step）
- [ ] adr-doc needed: `RoutingRequest` への範囲表現の追加または
      adapter contract の構造決定（wire 確定済み — 同上）

wire 境界の証拠は取得済み。spec / ADR / design / plan への promotion
自体は briefing gate・approval review 待ちで、本文書では設計・実装を
決定しない。新機能や consumer 設定変更は行わない。

## Sources and Relations

- Issue #107 およびその comment（consumer 観測の一次情報）。
- Devin hooks 公式ドキュメント 3 件（native wire 仕様の一次情報）。
- 関連 canonical docs は `relations.references` を参照:
  `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`
  （canonical routing request + per-target adapter の構造決定）、
  `docs/specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md`
  （全文 token guard + fail-closed の現行契約）、
  `docs/specs/apm-delegation-skills-and-read-hook.md`
  （read hook → bulk-read 委譲の現行契約）。

## Graph checkpoint

`doc-driven-dev` graph の route 結果と checkpoint 状態の記録。

### Route（確定入力）

```json
{
  "schemaVersion": 2,
  "graphId": "doc-driven-dev",
  "current": "probe",
  "next": "briefing",
  "edgeId": "probe-to-briefing",
  "condition": "bootstrap",
  "status": "edge",
  "delegate": "briefing-flow",
  "requiredAudits": ["adr", "spec"],
  "blockers": [],
  "taskGraph": null,
  "commitGate": false
}
```

### Pending 状態（checkpoint 未完了、2026-10-15 更新）

- focus: `docs/discovery/bounded-hook-read-request-contract.md`。
- `requiredAudits` = `["adr", "spec"]`: audit は実行済み（log は
  `tmp/issue107-audit-adr.log` / `tmp/issue107-audit-spec.log`）。結果を
  確認し、既存の非 blocking warning のみで今回の新規 finding はない。
  delegate が未完了のため edge checkpoint 自体は未完了。
- `delegate: briefing-flow`: 依然 pending。state は「awaiting external
  input」から「wire 境界の証拠取得、次は spec/ADR の要求・方針を
  formalize / review」へ変更。input-required blocker は解消したが、
  briefing gate / spec・ADR の作成・approval review は未実施であり
  completed とはしない。
- 再開入口: `probe` node の pending edge `probe-to-briefing`。edge は
  選択済みで delegate は引き続き `briefing-flow` — spec/ADR へ進める
  状態（briefing node への遷移はまだ完了していない）。

### input-required（2026-10-15 解消 — fixture で wire 取得済み）

当初 user/consumer のみ提供可能としていた wire 情報 3 項目のうち、
fixture 再現で item 1（stdin field names / bounds）は取得済み、
item 3 の発火重複は fixture 内で dual `pre_tool` firing を実測
（reported 環境の他 provider 重複は依然未知）。item 2（bounded
output semantics 保証）は open item として Open Questions に残す。

### 派生 chain 完了 checkpoint（2026-10-16 追記）

本 discovery から派生した partial chain は完了:
spec `SPEC-034GssA0IjLKSiDdIHpAC8`（implemented）→ ADR（accepted）
+ design（approved）→ plan `PLAN-034GtndRTaEqU3lA6Tnquu`（completed）→
task `TASK-034GtBzfyOaeVS6nlm1ZkC` / `TASK-034GtBzKmwPf5z06kAahym`
（done）→ T1/T2 実装・`mise run check` 全 stage pass。ただしこの
chain が扱うのは native tool の case-insensitive normalization と
Devin 登録 bootstrap であり、**issue #107 本体の bounded
`offset`/`limit`/`max_results` 範囲評価は依然未対応** — wire の
bounds 消失点（native → compatibility 変換）は特定済みのまま、
bounded-read semantics の修正は後続の spec/plan が担う。本
discovery の status は `active` のまま、凍結 wire 証拠本文は
変更しない。`task-graph-to-implementation` edge は実装 commit
`1ef366c` 後の clean worktree を確認し commit gate を通過した。

`implementation-to-followup-triage` の required audits は
`impl-record` → `task` の順に実行した。IR は findings なし、task は
既存の reciprocal-relation warnings のみで blocking findings なし。
今回の 2 task / 2 IR に finding はない（保存済みログ:
`tmp/issue107-followup-audit-impl.log` /
`tmp/issue107-followup-audit-task.log`）。本 partial chain の follow-up は
`followup-terminal` と分類し、#107 の範囲評価は既存の open item として
引き続き保持する。

`followup-triage-terminal` の `doc-status --type all` 監査も
blocking/error なし、今回の chain への finding なしで Completable
（`tmp/issue107-exit-audit-all.log`）。lead が実結果を確認して
`implementation-verified` / `followup-terminal` / `exit-audit-pass` を
与え、`exit-audit-to-complete` 後の router 出力が `status: terminal`、
`current: complete`、`blockers: []` となったことを確認した。

実行 trace は `probe-to-briefing` → `briefing-to-design` →
`design-to-planning` → `planning-to-task-graph` →
`task-graph-to-implementation` → `implementation-to-followup-triage` →
`followup-triage-terminal` → `exit-audit-to-complete`（8 hops、固定
`taskBudgetCount: 2` / `maxHops: 11`、commit waiver なし）。これは
今回の partial plan の terminal であり、#107 全体の resolved 宣言ではない。

## Review-driven scope expansion（2026-10-17、PR #109 comment-6221192905）

PR #109 の owner review が #107 の範囲評価を「out of scope」から
この PR の必須条件へ昇格させた。10 項目の指摘をコード・一次資料・
実機検証と突き合わせて確認した事実を記録する（設計・実装は決定
しない）。

### 確認済み: 指摘の事実基盤

- `RoutingRequest` は `FileRead { path, cwd, source_tool }` と
  `Shell { command, cwd, source_tool }` のみ。`targets.rs` の
  `normalize_pretooluse` / `normalize_devin` は `tool_input` の
  `offset`/`limit`/`max_results`/`glob_pattern` を一切読まず、
  `FileRead` へ flatten する。全文 token guard（既定 8192）が
  要求範囲ではなくファイル全体を評価する構造が #107 の根因。
- Devin adapter の `shell_tools` は現行 `&[]` — `bash`/`exec` 系は
  Shell 評価へ届かない（`hook.rs` fixture でも `bash` は silent
  allow で通る）。
- cheaper 管理 registration は PowerShell script
  `install-devin-hooks.ps1`（preview / `-Apply`、Windows のみ）
  のみで、CLI 側に inspect/remove/lifecycle を持たない。
- APM uninstall・再 install での cheaper-native 登録の converge /
  stale 登録検出は未提供。

### 一次資料・実機で確認した事実（2026-10-17）

- **Devin tool 名**: 公式 docs.devin.ai の hooks / tools 資料で
  shell 実行 tool は `exec`（`tool_input.command`）、file read は
  `read`、search は `grep`・`glob` と確認。`bash` 名の native tool
  は確認できない。matcher へ追加する場合は `exec` のみ。
- **APM per-dependency `targets:`**: APM 0.28.0 実機で検証済み。
  project `targets` が `[agent-skills, windsurf]` で dep が
  `targets: [agent-skills]` の場合、cheaper の Windsurf hook は
  deploy されず skills のみ配置される。既存 consumer が per-dep
  targets に移行して再 install すると、APM が cheaper 所有の
  `.windsurf/hooks/apm/*`・`apm-hooks.json`・`.windsurf/hooks.json`
  内の cheaper entries を reconcile で除去する（実測）。ただし
  project `targets` に `agent-skills` が無いと dep 全体が skip
  され skills も消える — 移行手順の必須条件。
- **APM lifecycle**: consumer `apm.yml` の `lifecycle:` key に
  `pre-install` / `post-install` / `pre-update` / `post-update` /
  `pre-uninstall` / `post-uninstall` を登録可能。project script は
  `apm lifecycle trust` による明示 trust が必須で、script 失敗は
  install/update 操作自体を abort しない（fail-soft）。
  command script は stdin に `event`/`packages`/`scope`/
  timestamp/CLI version/cwd の JSON payload を受け取る。
  `APM_NO_SCRIPTS=1` で抑制可能。したがって lifecycle script だけで
  登録の強制はできず、CLI 側の収束コマンド + 検出・案内と組み合
  わせる設計が必要。
- **`count_text_tokens(&str)`**: `cheaper_core::token::count` に
  文字列 slice を計測する API が既存。範囲評価は読み込んだ範囲
  slice をこの API で計測でき、全文 tokenize を避けられる。

### Promotion 判断に影響する open item の解消

- 「実 tool が範囲を尊重する保証」: native wire で `offset`/`limit`
  が adapter へ届くことは fixture 済み。評価が「要求範囲の token
  数」を行う限り、tool 側の実効範囲との完全一致は block 条件と
  しない（評価は防御境界であり tool 実行結果の pin ではない）。
- Devin `read` の offset 基数は 1-based line number として扱う
  （Claude/Devin 系の line-window 慣行と wire 観測 `offset:334,
  limit:81` と整合）。 EOF 超過・offset が総行数超過は空範囲として
  許容し、tool 側の実挙動は範囲外。

Promotion: 本 expansion の要件・構造決定は新規 spec / ADR へ派生
させる（Graph checkpoint の新規 chain を参照）。

## Evidence 保存状態（2026-10-21 追記）

本 discovery の既存 section は当時の観測報告と選定された
sanitized excerpt を保持しており、完全な原本 wire 全体ではない。
tmp/ の raw artifact は参照前提ではない（tmp path は履歴
label）。保存済みの durable section:

- 2026-10-15 実測 setup・version・test matrix —
  「実 consumer 観測」「実 native wire 観測」「合成再現 round 2」。
- sanitized record の構造例・bounds・exit outcome —
  「保存済み record」以下の wire/table 抜粋。
- privacy 境界と未検証 causal claim の明記 —
  上記各 section の privacy 注記。

2026-10-21 の確認時点では `tmp/issue107-native-repro/` は空で、
当時の wire/probe 原本はこの checkout に存在しない。今回、
再取得・復元・native agent の再実行は行っていない。
上記 durable section が正本であり、同一 byte の原本は本記録
から復元できない。追試には本文の隔離条件・観測項目と
committed tests を参照するが、当時の one-off script 自体の
復元を保証しない。

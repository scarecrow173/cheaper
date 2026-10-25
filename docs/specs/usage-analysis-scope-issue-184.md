---
id: "SPEC-034M0QMBJ1LQRGDdB2wnKl"
type: "spec"
status: "approved"
title: "Usage analysis scope — Route/Error breakdowns and period filtering (issue #184)"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
    - "https://github.com/scarecrow173/cheaper/issues/192"
    - "../discovery/usage-analysis-audit-issue-184.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
    - "../plans/implement-usage-route-error-analysis-issue-184.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/usage-analysis-audit-issue-184.md"
  derived-by:
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
    - "../plans/implement-usage-route-error-analysis-issue-184.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../specs/run-error-record-observability.md"
    - "../specs/delegation-usage-measurement-log.md"
    - "../adr/usage-error-record-field-contract.md"
    - "../adr/usage-jsonl-contract-canonical.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../specs/route-grant-set-projection.md"
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
---

# Usage analysis scope — Route/Error breakdowns and period filtering (issue #184)

## What Should Be Built

2026-10-26 改訂: #192 との責務境界を明確化した amendment は独立 review と
Judge 判定を経て承認済み（記録は plan の approval checkpoint）。
先行版の承認と既存 ADR の O2 採択を維持する。実装の検証完了は意味しない。

issue #184 の成果物は次の 5 点。現行監査の根拠は
`docs/discovery/usage-analysis-audit-issue-184.md`、方式採択は
`docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md` を
参照。

1. **`--since` の route 適用修正** — `build_usage_report` が
   `route_bucket` を `--since` filter 済み集合へ掛けるよう直し、
   `kind:"route"` / `kind:"route-refetch"` record が期間 window
   (`ts >= now - sinceMs`)に従うようにする。
2. **Route 別分析 section** — route 名を key とする per-route 集計
   (executions・成功/失敗・originalTokens/returnedTokens/
   reductionRatio・postprocessFailures・refetch 回数/bytes の
   logRef join・durations `{exec,analyze,total}`・計測不能数)。
   `--route <name>` filter で対象 route を絞れる。
3. **Worker 失敗分析 section** — run error record の `error.code`
   別件数・全 run および失敗 run に占める比率(shareOfRuns /
   shareOfFailures の双方)・worker 名別・provider/model 別内訳・
   失敗の `durationMs` と `execution`
   echo(`timeoutMs`/`maxTurns`/`maxToolCalls`)の集計。
4. **`--group-by route|error` flag と text/JSON parity** —
   加算的 flag のみで既存 report field・filter の後方互換を保ち、
   `--format text` でも同じ集計結果が見える section を追加する
   (現行は route bucket 自体が text に出ない — parity gap を解消)。
5. **テストとガイド更新** — `tests/integration/usage_report.rs` に
   固定 JSONL fixture の境界 test(`--since` 反映・新旧 schema
   混在・0 件・refetch のみ・失敗のみ・二重計上なし)を追加し、
   `docs/guides/usage.md` に route bucket・新 flag・母数と
   actual/measured/estimated の区別を追記する。

## Why Now

- `route_bucket(&all)` が filter 前に全量へ掛かる構造
  (usage_report.rs:552)で、`--since` が route に効かない bug が
  確定している。Route 導入(#149)・`executed` attribution(#166)・
  error.code diagnostics(#141)で record 側の観測値は揃ったが、
  集計側が追いついていない。
- #141 で計測した失敗率(158 run / 48 error)の要因分析を恒常化
  するには、`error.code` 別・worker/provider/model 別の継続集計が
  要る。
- 既存 `cheaper usage` 利用者を壊さず additive に拡張する必要が
  ある(record schema は additive-only、report field も同様)。

## Users and Value

- cheaper を運用する親 agent / 開発者 — Route 処理の圧縮効果
  (original→returned の実測減少)と失敗傾向を期間・route 名・
  error code 軸で把握し、route 定義や worker の改善判断に使う。
- 本 repo の開発者 — #141 のような失敗率調査を ad-hoc 集計なしで
  再現できる。

## Scope

### In Scope

- `crates/cheaper-cli/src/usage_report.rs` の集計・report 構築
  (route bucket の `--since` 適用、per-route 集計、error.code 集計)。
- `crates/cheaper-cli/src/args.rs` の `UsageCliArgs` への additive
  flag 追加(`--group-by route|error`、`--route <name>`。他 command
  では従来どおり reject — usage-only ガードへの編入を含む)。
- `crates/cheaper-cli/src/text_format.rs` の `format_usage_report`
  への additive section 追加(route 集計の text 表示を含む parity
  解消)。
- help テキスト(cli.rs)と `docs/guides/usage.md` の更新。
- `tests/integration/usage_report.rs` への fixture test 追加
  (新旧 schema 混在・境界値・二重計上なし)。
- malformed record の扱い規則(既存の JS-parity fail-hard は
  run 集計に限定して維持し、新しい grouped section では二段ルール
  — 構造的 invalid は skip+計数、otherwise-valid record 内の
  malformed scalar は `or_zero` 加算 + `unmeasurable` family 計数
  — で可視化する。Req 7・「出力 contract」節参照)の確定。

### Out of Scope

- usage.jsonl の writer 変更・record schema 変更(read-only 集計の
  まま。additive-only schema を崩さない)。
- 新しい trace 収集・機密 command/argv/prompt の収集(issue 非目標)。
- 親 agent の未観測 token savings の断定(issue 非目標)。
- `parse_duration` の `d`/`w` suffix 拡張(`DURATION_PATTERN` は
  policy JSON schema — `policy/command.rs:62-92` 等 — と共有。
  本 issue では `168h`/`24h` 表記を guide に示すに留める)。
  なお先行 spec `usage-and-savings-telemetry-aggregation.md`(:148)
  の `--since 7d` 記述は `d` suffix が実装されたことがない既知の
  不正確な記述であり、当該 doc は実装より先行して書かれたため
  superseded-by-practice として扱う(他 chain の approved doc は
  改訂せず、分岐は本 spec に記録する)。issue 例示の `7d` は
  `168h` で充足する。`d`/`w` の追加は `DURATION_PATTERN` と
  usage 側受理値の分離を前提とする将来判断で、本 issue では
  行わない。
- streaming/bounded aggregation への移行(全量 `Vec<Value>` 展開は
  維持。実測で閾値超過が判明した場合のみ別 issue で段階移行)。
- `--worker`/`--provider`/`--model` の route record への拡張適用
  (route record の `worker` は analyze worker の object で意味が
  違う — route 軸は `--route` のみで絞る)。
- usage 集計結果を残す永続化・外部送信。

### #192 Execution History との互換性境界

- #184 自体は Usage writer/schema を変更しない。将来
  `executionId`・`parentExecutionId`・`traceId` が optional field として
  入っても、read 側は受理して無視し、既存 aggregate/group/filter/dedup
  の結果を変えない。ID の有無を record の有効性条件としない。
  同一 trace の別 run はそれぞれ計数し、ID による統合/重複排除をしない。
- 失敗割合の母数は filter 後の **記録された run**（`shareOfRuns`）と
  その失敗 run（`shareOfFailures`）であり、全実行数ではない。History に
  将来記録される usage 発生前の失敗・skip はいずれの母数にも加えない。
- History は不要・非参照であり、History record は集計入力ではない。
  window 外 Route owner を History で補完せず、window 内 refetch は
  既定の `logRef` join/orphan 規則に従う。
- [#192](https://github.com/scarecrow173/cheaper/issues/192) が ID の
  発行・伝搬・History store と、**適用可能な新規 Usage record への
  executionId 追加および History 実行イベントとの一致（必須）**を所有する。
  `parentExecutionId`/`traceId` の適用・整合も #192 の契約であり、
  #184 の optional consumer tolerance はその必須要件を弱めない。
  旧 record の読取互換性を維持し、ID 表現や store 形式はここでは決めない。
- `usage --execution-id` は #192 の optional candidate に留め、
  #184 の flag・受入条件には追加しない。hard dependency はなく別 PR とし、
  提案順は #184 → #192 → #185（副作用なし診断）である。

## Requirements

1. **期間 filter の route 適用**: `--since` 指定時、route 集計
   (global `result.route` bucket・`byRoute` 群)と route-refetch
   計数は `ts >= now - sinceMs` を満たす record のみを対象とする。
   `ts` が無い・parse 不能な route 系 record は `--since` 指定時に
   除外する(run record と同一規約)。issue の「期間単位」はこの
   `--since` window 絞り込みを意味し、日次/週次等の期間 bucket
   行の出力は行わない(解釈として固定)。**これは既存 field
   `result.route` の値の意味論変更**で、純粋な additive ではない
   — `--since` を付けた呼出で route bucket の値が変わる旨を
   guide に明記する。
2. **kind 分離と二重計上禁止**: `kind:"run"`・`"route"`・
   `"route-refetch"` の集計意味論は互いに独立とし、route record の
   `worker.usage`・token field を run 集計(`records`/`usage`/
   `measured`/`comparable` 母数)へ混入させない。route の refetch
   計数は `route-refetch` record の `logRef` join を唯一の計数源
   とし、route record の `refetches` snapshot field は集計しない。
3. **per-route 集計**: `--group-by route` で route 名別に
   executions・succeeded・failed・originalTokens・returnedTokens・
   reductionRatio(= `1 - Σreturned/Σoriginal`、Σoriginal=0 時は
   `0.0` — record writer と同一規約値)・postprocessFailures・
   refetch `{count,bytes}`・durationsMs `{exec,analyze,total}` の
   **和**・計測不能件数を報告する(shape は「出力 contract」節)。
   duration の mean は `executions` で除算すれば導出できるため
   avg field は出さない。group key は route 名(`route` field)
   のみ — `executed.argv`・平文 command を key・出力に含めない
   (必要なら `commandHash` のみ可)。`route` field 欠損/非文字列の
   route record は group 化不能として skip し `unkeyedRecords` に
   計数する。
4. **error.code 集計**: `--group-by error` で `kind:"run"` かつ
   `!truthy(ok)` の record(= `errors` 母数と同一集合)を
   `error.code` 別に集計し、件数・全 run に占める比率
   (shareOfRuns)・失敗 run に占める比率(shareOfFailures)・
   worker 別・`model.provider`/`model.id` 別内訳・`durationMs` 和・
   `error.reason`(abort 語彙)副集計・`execution` echo
   (`timeoutMs`/`maxTurns`/`maxToolCalls`)の distinct 値別件数を
   含める。`error` object 欠損・`code` 欠損/非文字列/明示 `null`
   は `"unknown"` bucket に畳む。出力 field は code・reason・
   各種 count・worker 名・model ref・duration 和・execution echo
   のみで、`error.message`・`schemaErrors`・`checkpoint`・生の
   prompt/errorMessage・`root`/path 系を集計・出力しない
   (allowlist 外の error 詳細も同様)。
5. **CLI flag**: `--group-by <route|error>` と `--route <name>` は
   `usage` command 専用。`--group-by` は単値(既存 string option
   の last-wins 規約)で、`route,error` のような複合値・未知値は
   INVALID_REQUEST とする(guide にも明記)。他 command で使われた
   場合も INVALID_REQUEST で reject する — 新 flag を
   `usage_only_option_used` ガード(args.rs:759)へ編入し、その
   error message(:945 — 対象 flag 名を列挙する形式)にも追加する。
   `--route` は route-scope 集計(既存 `result.route` global
   bucket と `byRoute` section の**両方**)のみを絞り、run record
   集計(records/usage/measured 等)と `--group-by error` の
   `byError` section には影響しない(run/route record は集計 key
   を共有しない)。`--route` + `--group-by error` の組合せは合法
   で、error section は通常通り run record を集計する。
   `--group-by route --worker <name>` では run section のみ
   filter され、route section は無 filter のまま(route record の
   `worker` は object で文字列一致に常に不適合)。
6. **text/JSON parity**: 新しい集計結果は `--format text` と JSON
   envelope で同じ値を出す。text 側は `result.route` 相当の route
   section を常時描画し(既存 parity gap の解消 — 想定内の
   snapshot churn、`usage_command_format_text_writes_report` の
   更新を伴う)、`--group-by` 指定時は byRoute/byError section を
   0 件でも無条件に描画する(JSON が空 array を emit する規約と
   同一)。既存行の書式は維持する。`result.route`(既存 global
   bucket)は残し、per-route/error の新 field は flag-gated
   additive として `--group-by` 指定時のみ emit する。
7. **欠損耐性**: malformed JSONL 行・未知 kind・optional field
   欠損・旧 schema record・期間混在・logRef join 未達
   (orphan refetch)・データ 0 件を正しく扱う。grouped section の
   malformed 扱いは二段ルール: **構造的 invalid**(record 非
   object・`kind` 不正は `is_record` gate で除外済み、`route`
   field 欠損/非文字列は group 不可)は skip+計数
   (`unkeyedRecords`)、**otherwise-valid record 内の malformed
   scalar**(token・duration subfield の欠損/非数値)は `or_zero`
   で 0 加算しつつ対応する `unmeasurable` family を計数する
   (Req 8 parity — `durationMs` subfield は field 毎に `or_zero`
   適用)。既存 run 集計の明示 null fail-hard 契約は変えない。
8. **非有限・負値の扱い**: 既存の JS number 意味論
   (`plus`/`or_zero`/`truthy`、非有限 → JSON `null`)を新集計でも
   踏襲する。

## 出力 contract(新規 `result` field の固定)

新規 `result` key は全て **flag-gated additive** — `--group-by` 指定
時のみ emit し、flag 無しの既存出力は shape・値ともに変えない
(これにより regression golden `cli-usage-*.json` は再生成不要で
green のまま。無条件 emit の新 field を将来追加する場合は golden
再生成が要る)。key 挿入位置は既存 `route` key の**直後**・
`estimated` の前で `byRoute` → `byError` の順とする。design doc
は field 名を refine してよいが意味論は変えない — 名前を変える
場合は本 spec を amend する。

### `--group-by route` 指定時

- `result.route`(既存 global bucket)に `orphanRefetches`・
  `unkeyedRecords` を additive 追加:
  - `orphanRefetches: {count, bytes}` — window 内の
    `route-refetch` record のうち、`logRef` が空・欠損・非文字列
    (`""` で書かれる absent logRef 含む — route/run.rs:630)か、
    window 内 route record の非空 `logRef` のどれにも一致しない
    もの。route record 側も空・欠損・非文字列 `logRef` では join
    対象にならない。orphan 判定は **window 全体**で行い、
    `--route` 絞り込みでは隠れない(orphan はどの route にも帰属
    しないため)。
    join owner として扱う route record は `route` が文字列のものに
    限る。route 名が欠損/非文字列の record の非空 `logRef` は owner
    map に登録せず、それを参照する refetch は orphan とする。
  - `unkeyedRecords` — `route` field 欠損/非文字列で group 化不能な
    route record の件数。これは route window 全体で数え、`--route`
    filter で減らさない。global `executions` 等には従来通り計上するが、
    route 名がないため特定 route の scope には含めない。
- `result.byRoute: []` — route 名昇順(deterministic)。entry shape:
  `{route, executions, succeeded, failed, originalTokens,
  returnedTokens, reductionRatio, refetches{count, bytes},
  durationsMs{exec, analyze, total}, postprocessFailures,
  unmeasurable{tokens, durations}}`。
  - `refetches` は join した `route-refetch` record のみ計数。
  - `durationsMs` は 3 subfield それぞれの和(mean は `executions`
    での除算で導出 — Req 3)。
- group 0 件でも flag 指定済みなら `byRoute: []` を emit する
  (key 省略しない)。

### `--group-by error` 指定時

- `result.byError: []` — `count` 降順・同数は `code` 昇順。entry
  shape: `{code, count, shareOfRuns, shareOfFailures,
  workers{<worker名>: n}, models{<provider>/<id>: n}, durationMs,
  reasons{<reason値>: n}, execution{timeoutMs{<値>: n},
  maxTurns{<値>: n}, maxToolCalls{<値>: n}}}`。
  - `code` — `error.code` の文字列値。`error` object 欠損・`code`
    欠損・明示 `null`・非文字列は全て `"unknown"` に畳む。
  - `shareOfRuns` = count / filter 後 run record 数、
    `shareOfFailures` = count / filter 後 `ok:false` run 数。
    分母 0 のときは `null`(非有限 → `null` の既存規約)。
  - `workers`/`models` — `worker` 文字列 / `model.provider` +
    `model.id` 結合 key 別の件数。該当 field 欠損・非文字列の
    record は当該内訳にのみ含めない(`count` 本体には入る)。
  - `reasons` — `error.reason`(record には abort 語彙のみ載る)の
    値別件数。
  - `execution` — `execution` echo を持つ record のみ計数し、3
    subfield 固定で **distinct 値 → 件数** の map を出す(`null`
    値は key `"null"`)。
- error 0 件でも flag 指定済みなら `byError: []` を emit する。

### 計測不能(unmeasurable)の定義

otherwise-valid(`is_record` 合格・`route` 名あり)な route record
で、下記 field が欠損・非期待型のとき、その metric family の
`unmeasurable` を 1 加算する。値自体は `or_zero` で 0 として和に
参加する(Req 7/8 — 和は「計測できた分の合計」で、unmeasurable が
過少計上を可視化する)。計数は metric family 別であり、単一の
合計 counter ではない。

- `tokens`: `originalTokens` または `returnedTokens` が欠損/非数値。
- `durations`: `durationMs` が欠損/object でない、または
  `exec`/`analyze`/`total` のいずれかが欠損/非数値(欠損
  subfield は当該和へ 0 加算)。
- `worker` — route record の `worker` は analyze job 未実行時に
  `{}` になる**正常形**(route/run.rs:602-612)で、unmeasurable に
  計上しない。analyze worker の usage は本 section の集計対象外。

### `--route <name>` の scoping

`--route` は route-scope 集計のみを絞る — `byRoute` entry と既存
`result.route` global bucket の両方が対象 route の record に限定
される(route-scope の refetch は対象 route の非空 `logRef` に
join したもののみ)。run record 集計と `byError` には一切効かない
(run/route record は集計 key を共有しない)。`orphanRefetches` と
`unkeyedRecords` は window 全体の判定値/件数を維持する。unkeyed
route record は `--route` scoped な `result.route` の executions 等
には含めず、診断用 `unkeyedRecords` だけに表示する。

## Acceptance Criteria

- [ ] optional 相関 ID の有無で aggregate/group/filter/refetch 出力が同一。
      同一 trace の別 run は両方計数され、refetch の独立 executionId は
      `logRef` join を妨げない。
- [ ] History 不在でも report が成立し、History 相当の外部情報があっても
      window 外 Route owner を補完せず orphan のまま。History record は
      入力とせず、usage 発生前の失敗/skip で recorded-run 母数を変えない。

- [ ] `cheaper usage --since <期間>` で route/route-refetch 集計が
  window 内 record のみを反映する(fixture で期間内外を混ぜた
  検証 — issue 受入条件「期間・Route 名・Worker・error 分類に沿って
  集計可能」)。
- [ ] `--group-by route` で route 名別の実行数・成功/失敗・
  original/returned tokens・reductionRatio・postprocess failures・
  refetch 回数/bytes・durations・計測不能数が出る。
- [ ] `--group-by error` で error.code 別件数/比率と worker・
  provider/model 別内訳が出る。
- [ ] `--route <name>` で route 集計が指定 route に限定される。
- [ ] run/route/refetch の件数と token が group 間で二重計上
  されない(test で固定 — issue 受入条件「二重計上しない」)。
- [ ] malformed 行・旧 schema(optional field 欠損)・0 件・
  route 実行 zero・refetch のみ・失敗のみ・orphan refetch
  (join 未達・空 `logRef`)の各 fixture が正しい表示になる
  (issue 受入条件「missing/legacy/corrupt・0件・fake recordで
  正しい表示」)。
- [ ] 構造的に valid な fake record(fixture で捏造した
  `is_record` 合格の record)が通常 record として集計される
  ことを fixture test で固定する。
- [ ] `--format text` と JSON で同じ集計結果が出る(text/JSON
  parity)。既存フィルタ・report field の後方互換を test で固定
  (issue 受入条件「`--format text`/JSON の契約、既存フィルタの
  後方互換」)。新 field は全て flag-gated のため flag 無しの
  regression golden(`cli-usage-*.json`)は無変更のまま green、
  text snapshot(`usage_command_format_text_writes_report`)は
  route section 常時描画で更新が発生する(想定内)。
- [ ] group key・出力に `executed.argv`・平文 command・生の
  prompt/errorMessage が現れない(test で grep 不能性または
  allowlist 検査で固定 — issue 受入条件「sensitive な argv /
  prompt / error detail を意図せず集計結果へ漏出させない」)。
- [ ] `docs/guides/usage.md` に route bucket・`--group-by`・
  `--route` の説明と、失敗割合・圧縮率の母数と
  actual/measured/estimated の違いの説明が追記される
  (issue 受入条件「母数と actual/measured/estimated の違いを
  ガイドで説明」)。あわせて `--since` が `result.route` にも効く
  ようになった意味論変更・`--group-by` が単値であること
  (`route,error` は INVALID_REQUEST)・`168h`/`24h` 期間表記を
  明記する。
- [ ] `mise run check` が pass する(issue 受入条件。check
  entrypoint は `mise.toml` の canonical task)。

## Deferred Design Concerns

- streaming/bounded aggregation への移行 | 現行 full-file parse の
  ピークメモリを代表的な大規模 JSONL fixture で測定・記録してから、
  この issue の範囲では全量 `Vec<Value>` 展開を維持する | 実測で
  file サイズ・parse 時間・ピークメモリが運用上の閾値を超えたとき |
  長期運用で集計応答・メモリが劣化しうる。
- `--worker`/`--provider`/`--model` の route record
  (`worker.model`)への拡張適用 | run と route で `worker` の意味が
  違い混同を避ける | route の analyze model を絞りたい需要の確認 |
  route を model 軸で分析できない(別軸 flag 追加で後追い可能)。
- `d`/`w` duration suffix の受理 | `DURATION_PATTERN` は policy
  JSON schema と共有で波及面が広い | `--since 7d` のような
  自然な期間表記の要求が上がったとき | 利用者が `168h` への
  換算を強いられる。
- workflow 由来の usage 観測 | workflow run は usage.jsonl に
  記録されない現行契約 | workflow 経由の route/run 分析需要 |
  workflow 内実行の傾向が usage 分析から見えないまま。

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/184>
- 上流 discovery: `docs/discovery/usage-analysis-audit-issue-184.md`
  (record inventory・bug 位置・gap 分析・採択空間)。
- 関連 chain(`references` のみ): usage telemetry chain、#141
  run-error-record chain、#149 route resolver chain、#166
  route-grant-set-projection chain。
- 方式採択: `docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md`
  (O2 — `--since` fix + additive `--group-by`/`--route` + sectioned
  output を採択)。

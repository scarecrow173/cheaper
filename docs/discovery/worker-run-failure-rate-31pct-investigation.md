---
id: "DISC-034Kqi8LcVZaxgz7wxHfg4"
type: "discovery"
status: "resolved"
title: "worker run 失敗率 31% の要因調査 (issue #141)"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-runtime/src/run/boundary.rs"
    - "crates/cheaper-runtime/src/run/abort.rs"
    - "crates/cheaper-runtime/src/run/output.rs"
    - "crates/cheaper-runtime/src/run/repair.rs"
    - "crates/cheaper-runtime/src/run/session.rs"
    - "crates/cheaper-rig-adapter/src/error.rs"
    - "crates/cheaper-tools/src/accounting.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/run.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/resources/policy.yaml"
    - "crates/cheaper-cli/resources/workers/bulk-reader.md"
    - "crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json"
    - "crates/cheaper-provider/src/devin/rig_provider.rs"
    - "crates/cheaper-provider/src/devin/jwt.rs"
    - "crates/cheaper-provider/src/devin/cli.rs"
    - "crates/cheaper-provider/src/devin/plugin.rs"
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
    - "./bulk-reader-low-tier-no-result-turn-budget-and-timeout.md"
    - "./bulk-reader-output-limits-and-invalid-output-guidance.md"
    - "../adr/execution-timeout-as-progress-reset-stall-window.md"
    - "../adr/nullable-execution-maxturns-unbounded-by-default.md"
    - "../adr/budget-exhaustion-checkpoint-and-resume.md"
    - "../adr/invalid-output-raw-output-persistence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/run-error-record-observability.md"
    - "../adr/usage-error-record-field-contract.md"
    - "../designs/run-error-record-observability.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# worker run 失敗率 31% の要因調査 (issue #141)

> Entry Decision: **A-3 (Combined Discovery)** — 複数の発火機構(TIMEOUT/
> MAX_TURNS/INVALID_OUTPUT/PROVIDER_FAILURE)を跨ぐ調査で、コード読解と
> 既存 discovery(#111/#108 系)の統合が必要なため。

## Exploration Goal

`~/.config/cheaper/usage.jsonl` の 158 run → 48 error(30.6%:
TIMEOUT 18 / MAX_TURNS_EXCEEDED 16 / INVALID_OUTPUT 9 / PROVIDER_FAILURE 5、
全て bulk-reader・provider ≒ devin)について、4 つの error code の発火機構を
コードから確定する。最大の未解明点は「TIMEOUT duration が正確に ~120s に
密集(16×120s、2×121-122s、1×137s)する出所」— 現行 builtin policy は
`execution.timeout: 300s` の stall window、Devin CLI の既定 timeout は
30s で、120s の直接的な定数が見当たらない。

## Key Issues and Assumptions

- TIMEOUT の発火点は run 経路では
  `crates/cheaper-runtime/src/run/boundary.rs:122-136` の watcher
  **のみ**。窓は `policy.execution.timeout_ms`、発火は
  `last_progress + timeout` 経過時の `RunAbortReason::Timeout` →
  `ErrorCode::Timeout`(`abort.rs:46-53`)。~120s の TIMEOUT が出るには
  **effective `timeout_ms` が ~120s でなければならない**。
- MAX_TURNS_EXCEEDED は `policy.execution.max_turns: Some(int)` の
  場合にしか発火しない(`mod.rs:707` の `is_some_and`)。
  builtin 既定は `maxTurns: null`(`policy.yaml:26`)なので、観測には
  「何らかの層で整数が設定されていた」ことが必要条件。
- error record は `usage_log.rs:151-158` で `error: {"code": ...}` のみ
  を書き、`message`/`details`/`meta.repairs`/version/policy を捨てる。
  **原因分類に必要な field が record 側に構造的に存在しない**。

## 発火機構の確定(コード読解結果)

### F-1: ~120s TIMEOUT の出所 — 現行 run 経路に 120s 定数は存在しない

`execution.timeout_ms` を書く箇所の完全列挙:

| 供給源 | 値 / 機構 | 本件との適合 |
| --- | --- | --- |
| **旧 builtin 既定**(a348d89 / issue #111 fix 以前) | `execution.timeout: 120s` の**絶対期限** | duration が run 開始から正確に ~120s に密集する観測と最も整合。grace(≤5s)を足して 120-125s 台が説明できる |
| user `~/.config/cheaper/policy.yaml` | Replace merge で `execution.timeout` を任意値へ | あり得るが「user が policy を書いた」前提が要る |
| project `.cheaper/policy.yaml` | RestrictConcurrency merge だが `execution.timeout` は `RESTRICT_DURATION_MIN_FIELDS` 対象外(`merge.rs:32-36`)→ 自由に置換可能 | 同上 |
| `--timeout <dur>` flag | `restrict_policy`(`run.rs:219-229`、`restrict.rs:32-53`)で narrowing のみ(> policy は POLICY_VIOLATION) | skill は flag を案内していない(`cheaper-bulk-read`/`cheaper-usage`)ので低確度 |
| route `deadline_ms` | `route/run.rs:325-332` — grant timeout + 120s | record は `kind:"route"`、本件は `kind:"run"` で対象外 |
| `process.timeout`/`command.timeout: 2m` | tool-level timeout | bulk-reader は read/grep/find/ls/count_tokens のみ宣言(`bulk-reader.md:4-9`)で不使用 |
| devin CLI `DEFAULT_TIMEOUT` | 30s(`cli.rs:23`)、JWT mint 30s(`jwt.rs:23`)、models 20s、`auth status` 15s、`version` 8s | completion 経路ではない — devin completion は `DevinTransport::send` の `send_streaming` → `ReqwestClient::default()` で **timeout 未設定**(`rig_provider.rs:501-505`) |
| hook `eval_cache` lock stale 120s | hook 評価内部 | run 記録経路ではない |

**判定(確度: 高)**: 120s は「旧版 binary の builtin 既定 120s 絶対期限」
か「user/project policy による `execution.timeout: 120s|2m` 設定」の
いずれか。stall 意味論(現行)で進捗があれば duration は最終 event+120s に
散るはずで、「run 開始から正確に ~120s」の集中は **絶対期限** を示唆 —
旧版説を支持する。**決定的裏付けは MAX_TURNS_EXCEEDED×16 の共存**:
現行 `maxTurns: null` では policy 由来の MAX_TURNS は発火不能であり、
旧既定 `maxTurns: 8` は両者を一つのバージョン説明で同時に説明する。

ただし record には binary version も resolved policy echo も無いため、
**telemetry 単独では旧版説と「新版 + policy 固定」説を識別不能** —
これ自体が本調査の主要 finding(F-5)。

### F-2: MAX_TURNS_EXCEEDED の発火点

- `mod.rs:707-712` — turn 計上 event(`ModelTurnFinished` 系)毎に
  `snap.turns >= maxTurns && snap.last_turn_tool_calls && pending_tool_calls == 0`
  で `RunAbortReason::MaxTurns`。over-limit turn の pending tool batch
  完了までは deferred(F1 コメント)。
- `mod.rs:737-738` — stream 終端 fallback(同条件)。
- `maxToolCalls: 64` は**別 code** `MAX_TOOL_CALLS_EXCEEDED`:
  `accounting.rs:108-115` の `on_limit` → `mod.rs:502-505` で
  `RunAbortReason::MaxToolCalls`。本件では観測されていない。
- 病態は DISC-x1vmG1ECubKZNQ0QMyUyi5 の F-A と同一: 「最終回答は
  tool call を出さない turn としてしか存在せず、残 turn 予算は
  model に通知されない」。旧既定 8 turn + 低 tier model の未収束
  read/grep 探索は容易に枯渇する。

### F-3: INVALID_OUTPUT の生成点

`extract_output`(`output.rs:55-106`)の 3 失敗分岐 →
`repair_output`(`repair.rs:84-159`)を `output.repairAttempts`(既定 1)
だけ回し、全滅で `invalid_output_error`(`repair.rs:190-218`)→
`ErrorCode::InvalidOutput`(exit 6、non-retryable)。

失敗分岐と原因候補:

| reason | 意味 | bulk-reader + 低 tier devin での典型因 |
| --- | --- | --- |
| `no-assistant-message` | assistant の final text 無し | abort なしで stream が text 無し終端、tool call のみで finish |
| `invalid-json` | JSON.parse 失敗 | **`output.maxTokens: 2000` で出力打ち切り → 未終端 JSON**(最大 suspect)。prose/fence 逸脱 |
| `schema-mismatch` | object だが schema 非適合 | closed object(`additionalProperties:false`)・required 5 keys(`summary`/`findings`/`relevantFiles`/`suggestedReads`/`openQuestions`)欠落・型違反・`$defs/path` pattern(絶対 path・`..` 禁止)違反 |

- `max_output_tokens` は main session(`mod.rs:602`)にも repair
  session(`repair.rs:102`)にも同じ 2000 が適用される — bulk-reader は
  findings を詰めた JSON が 2000 output token を超えやすく、
  **打ち切り → repair も同じ上限で打ち切り → INVALID_OUTPUT** の
  連鎖が機構的にあり得る。
- 旧版 schema には magnitude 上限(excerpt 600 文字等、issue #108 /
  DISC-NXzrf…)があったが、現行 schema は構造制約のみ
  (`builtin_workers.rs:132`「output magnitude 撤廃」)。
  record が旧版由来なら magnitude 超過も失敗源候補。
- envelope の `error.details` には `errors`[:20]・`rawOutput`[:2000文字]・
  `rawOutputPath`(`<userConfigDir>/invalid-outputs/`、issue #114)が
  **残っている** — 原因確定は consumer 側 envelope/保存 file で可能。
  usage.jsonl record はこれらを捨てる(F-5)。

### F-4: PROVIDER_FAILURE の写像面

`cheaper-rig-adapter/src/error.rs`:

- `provider_error_to_cheaper`(:85) — `PromptError::CompletionError` 由来。
  **details は `{errorMessage}` のみ** — HTTP status・transient 分類は
  retry 判定で消費され envelope に残らない(:80-84)。
- `report_to_cheaper`(:98-115) — ErrorKind Http/Provider/ProviderResponse/
  Request/Url/Json/Response → PROVIDER_FAILURE(同じ details contract)。
- `prompt_error_to_cheaper`(:158-181) — `UnknownToolCall`(invalid
  tool retries 尽き後も model が tool 名を外し続ける)→ PROVIDER_FAILURE。

devin plugin(`crates/cheaper-provider/src/devin/`)の具体的到達面:

- `GetChatMessage` 非 2xx — `chat_send_error`(rig_provider.rs:374-386)、
  message は `GetChatMessage HTTP <status>: <body 300字>`。
- transport 失敗(timeout/reset/接続不可)— transient 分類 →
  `transientAttempts: 1` retry 後も失敗で PROVIDER_FAILURE。
- Connect stream が EOS trailer 未到達で自然終端 →
  `ProviderError::Truncated`(rig_provider.rs:332-334)。
- Connect trailer `"error"` → `ProviderError::Provider`(:291-293)。
- JWT mint 失敗 — `GetUserJwt` 非 2xx・30s `MINT_TIMEOUT`・no JWT →
  `jwt_error`(:338-350)。
- credential 未解決 — `to_auth`(plugin.rs:119-125)→ PROVIDER_FAILURE。

`codex-sub` = `openai-codex` provider(ChatGPT subscription backend、
OAuth login)も同じ generic 写像 — 非 2xx / transport / truncation が
PROVIDER FAILURE へ畳まれる。

### F-5: usage.jsonl error record は `{"code"}` のみ(構造的 observability gap)

`build_usage_record`(usage_log.rs:151-158)が error record に書くのは
`error: {"code": <code>}` のみ。捨てられるもの:

- `error.message`(PROVIDER_FAILURE なら `provider error: GetChatMessage
  HTTP 500: …` — status・原因を含む唯一の記述)
- `error.details` — abort の `reason`、INVALID_OUTPUT の `errors[]
  (instancePath)`・`rawOutputPath`、budget 枯渇の `checkpoint.path`
- `meta.repairs`(repair attempt 消費数 — INVALID_OUTPUT が repair 尽き
  由来か判別する材料)
- binary version(旧版/新版の識別子)と effective policy
  (`execution.timeoutMs`/`maxTurns`/`maxToolCalls`/`output.*`/`retry.*`)

**本 issue の 4 問が telemetry から答えられないのは仕様通りの構造上の
制約であり、これ自体が修正対象の defect。**

## Alternatives and Comparison

### ~120s TIMEOUT の出所

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: 旧版 binary(a348d89 以前)の 120s 絶対期限 | MAX_TURNS_EXCEEDED(旧 maxTurns=8)を同じ説明で同時に説明。「正確に 120s」密集は絶対期限と整合。issue #111 でこの層は既に機構的対策済み | record に version が無く直接証明不能 | **yes** |
| B: user/project policy が `execution.timeout: 120s|2m` (+ `maxTurns` 整数) | 新版でも 120s が再現可能 | consumer が policy を書いた前提。timeout と maxTurns を両方書いた想定が要る | maybe |
| C: `--timeout 120s|2m` flag | narrowing は容易 | skill が flag を案内していない。maxTurns を説明しない | no |

### 失敗率の構造

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 大半は既存 defect(issue #111 chain で対策済み)の残滓 | TIMEOUT+MAX_TURNS=34/48 が #111 で機構的対策済みの病態と一致 | INVALID_OUTPUT/PROVIDER_FAILURE は対象外 — 残 14 件の原因は未確定のまま | yes |
| 新規の別 bug が混在 | — | 4 code 全てが既知機構で説明可能で、新規 bug を示す兆候は code 側に無い | no |

## Tentative Conclusions and Hypotheses

- **C1(確度: 高)**: ~120s TIMEOUT の正体は effective `execution.timeout_ms`
  ≈ 120s の run — 最確度高は **a348d89(issue #111 fix)以前の binary の
  120s 絶対期限**。MAX_TURNS_EXCEEDED×16 の共存が裏付け。user/project
  policy 説は排除不能で、識別には record への version/policy echo が必要。
- **C2(確度: 高)**: MAX_TURNS_EXCEEDED は回答 turn 保証の不在 + 有限
  maxTurns の枯渇(DISC-x1vm… F-A と同病態)。issue #111 chain で機構的
  対策済み(`maxTurns: null` + stall timeout + checkpoint/resume)。
- **C3(確度: 中)**: INVALID_OUTPUT の最大 suspect は
  `output.maxTokens: 2000` での出力打ち切り → 未終端 JSON(`invalid-json`)。
  repair session も同じ 2000 cap のため repair も打ち切られ得る。
  確定には `details.errors`/`rawOutputPath` が必要 — record には無い。
- **C4(確度: 中)**: PROVIDER_FAILURE は transient retry 尽き後の
  transport/HTTP 失敗・EOS 未到達・JWT mint・UnknownToolCall のいずれか。
  `errorMessage` 非記録で判別不能。
- **C5(確定・confirmed)**: 本調査の真の blocker は **usage record が
  `error.code` しか持たないこと**。版識別(version)・発火条件(policy
  echo)・原因分類(details subset)のどれも記録されないため、同種の
  「なぜ失敗したか」調査が今後もデータだけでは完結しない。→ spec/ADR で
  「run record の診断 field 拡充」を formalize する。

## Open Questions

- consumer の install version(`cheaper --version`)— 旧版説の直接検証。
  issue reporter 確認事項。
- `~/.config/cheaper/policy.yaml`・`.cheaper/policy.yaml` の有無と内容
  (`execution.timeout`/`maxTurns` の pin)。issue reporter 確認事項。
- `~/.config/cheaper/checkpoints/`・`~/.config/cheaper/invalid-outputs/` に
  sidecar file が残っているか — 残っていれば MAX_TURNS/INVALID_OUTPUT の
  原因を consumer 側 artifact から確定可能。
- 観測期間と a348d89 相当 version への upgrade 時期の前後関係 —
  record 群が版混在かどうか。

## Promotion Candidates

- [x] spec-doc needed: `kind:"run"` record への `version`・実効 policy
  echo・`error` 診断 field(reason/message/schemaErrors/errorMessage/
  checkpoint)・`repairs` の追加 →
  `docs/specs/run-error-record-observability.md`
- [x] adr-doc needed: error record の field contract(固定 allowlist vs
  details passthrough vs 計測据え置き)→
  `docs/adr/usage-error-record-field-contract.md`

## Sources and Relations

- issue #141(2026-10-22)の usage.jsonl 集計: 158 runs → 48 errors。
- `crates/cheaper-runtime/src/run/{mod,boundary,abort,output,repair,
  session}.rs` — TIMEOUT/MAX_TURNS/INVALID_OUTPUT 発火点、stall watcher、
  session factory の内部 guard(1024)・invalid_tool_retries。
- `crates/cheaper-rig-adapter/src/error.rs` — PROVIDER_FAILURE 写像と
  `{errorMessage}` details contract。
- `crates/cheaper-provider/src/devin/{rig_provider,jwt,cli,plugin}.rs` —
  devin completion は Connect streaming HTTP(CLI 委譲ではない)。
- `crates/cheaper-cli/src/usage_log.rs:80-160` — record 構築・error code
  のみ記録。
- `crates/cheaper-cli/src/run.rs:219-229` + `cheaper-core/src/policy/
  {merge,restrict}.rs` — `timeout_ms` の供給源(policy 層・`--timeout`)。
- `crates/cheaper-cli/src/route/run.rs:325-332` — route 専用の
  grant+120s deadline(本件外)。
- DISC-x1vmG1ECubKZNQ0QMyUyi5 — 同病態(turn 枯渇・120s 絶対期限)の
  機構分析と a348d89(issue #111 fix)の設計根拠。
- DISC-NXzrfJIbmACuqdlsNsxtWN — INVALID_OUTPUT details contract・
  schema magnitude 上限(旧版の失敗源)。
- commit `a348d89`(2026-10-17)— `execution.timeout` 120s→300s stall 化、
  `maxTurns` 8→null 化、budget 枯渇 checkpoint/resume の導入。

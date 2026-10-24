---
id: "DISC-034ESpIy7zAOucpwBxmjfd"
type: "discovery"
status: "resolved"
title: "tool-operator worker を built-in 追加する"
created: "2026-10-12"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/102"
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
    - "../discovery/policy-gated-command-tool.md"
    - "../discovery/invocation-scoped-command-grants.md"
    - "../specs/policy-gated-command-tool.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/tool-operator-built-in-worker.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "../designs/tool-operator-built-in-worker.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# tool-operator worker を built-in 追加する

## Exploration Goal

issue #102 は、汎用 built-in worker `tool-operator`（CLI alias `tool-operate`）を追加する。cheap-model worker 内で tool call を反復し（operation 実行 → 結果解釈 → 次 operation → 圧縮された最終結果の返却）、中間の巨大出力を parent context に流さず目標へ向かって操作を進める「operation actor」を担う。

責務分担の整理: bulk-reader = 大きな情報の抽出・圧縮、code-writer / doc-writer = artifact 編集、tool-operator = 目標に向かって反復する操作 actor。

この discovery が答える問い: 既存の worker 定義機構・policy gate・execution 制限の枠内で `tool-operator` をどう定義するか。特に、`command` / `process` が scope 構築時に fail-closed で gate される現行挙動と、issue の「無許可時は利用可能なツールの範囲で実行するか blocked を返す」要求をどう両立させるかが核心である。

## Constraints

issue #102 由来の要求:

- tools: `read`, `grep`, `find`, `ls`, `count_tokens`, `process`, `command` — `write` / `edit` は含めない（writer 系 worker の scope を侵食しない）
- input: `{task required, context?, constraints?, output hints?}`
- output: `{status: success|partial|failed|blocked, summary, results/findings with evidence, evidence, unresolved, nextActions}`
- 巨大な中間出力を parent に dump しない。evidence は file:line・commit id・tool exitCode・truncation 情報で表す
- `meta.changes` の honest さ: 子プロセス副作用を監査済みと主張しない
- Rig adapter loop を再利用する。独自 agent loop・再帰的 worker delegation は作らない
- `execution.maxTurns` / `maxToolCalls` / `timeout` / `output.maxTokens` / command 出力 cap を守る
- savings は `cheaper usage` で計測可能、`directlyComparable` は保守的に

## Key Issues and Assumptions

確認済み（コードベース探索）:

- built-in worker 定義: `crates/cheaper-cli/resources/workers/{bulk-reader,code-writer,doc-writer}.md` — markdown + YAML frontmatter。strict keys: `name`（file stem と一致必須）・`description`・`tools`（nonempty）・`cheaper{model{tier, workload.outputPerInput}, input.schema{file|inline}, output.schema, writeScope}`。schema は `resources/workers/schemas/<name>.{input,output}.schema.json`（JSON Schema draft 2020-12 + strict 検査）。discovery は project → user → builtin 順で `include_dir!` embedded。code 側の一覧は不要。
- CLI alias: `crates/cheaper-cli/src/args.rs:39-46` の `alias_worker()` static table に `"tool-operate" => "tool-operator"` を追加。HELP は `cli.rs:512-550`。test は `tests/args.rs` の `run_alias` table + 任意の regression case。
- **critical**: `command` を `tools` に宣言した worker は、`command.grants` が空の環境では scope 構築時に fail-closed で `POLICY_VIOLATION` になる（`crates/cheaper-tools/src/registry.rs:1075-1085`、builtin `policy.yaml` は `command.grants: {}`）。
- **critical**: `process` にも同型の gate がある（`registry.rs:1066-1071`: `names` に `Process` を含み `process.tasks` が空なら `POLICY_VIOLATION` "process exposure requires nonempty authorized tasks"）。builtin `policy.yaml` は `process.tasks: {}`。**両方を宣言すると、grants と tasks の双方が nonempty でない限り tool-operator は常に POLICY_VIOLATION で起動不能になる。**
- execution 制限は policy-global のみ（builtin `policy.yaml`: `execution.maxTurns=8`・`maxToolCalls=64`・`timeout=120s`・`output.maxTokens=2000`）。per-worker の frontmatter override 機構は存在せず、strict な `cheaper` keys が新規 field を reject するため追加は core 変更になる。多段 tool-operator はより多くの turn を欲しがりうる — global 制限内に収めるか（issue は「守る」と明記）、per-worker override を新設するかは未決定。
- `directlyComparable`: `command` / `process` は mutating tool のため、tool-operator の run は常に `directlyComparable=false`（issue の保守的要求と整合）。
- `meta.changes` は `write` / `edit` のみを journal する。tool-operator は両方を宣言しないため `changes` は常に空 — `command` / `process` の子プロセス副作用が未計測であることを文書化する。
- test 表面: dev-seam fake-stream fixture（`run_dev_seam.rs` の `FIXTURE_COMMAND_CALL` / `command_user_policy` 先例）、regression corpus（`tests/regression/cases/run-max-turns.json` 等）、`ollama_e2e.rs` の gated 実 provider 検証。
- APM 表面: `packages/apm` の `cheaper-usage` skill（command 一覧 + delegation 指針）、worker 別 skill（`cheaper-bulk-read` 等）— `cheaper-tool-operate` skill を追加するかは未決定。docs: `README.md` built-in workers 表（~:80-89）、`docs/guides/workers.md`（~:62-85）、`docs/guides/usage.md`、`docs/guides/command.md`（#101 の新 guide）。

仮定:

- worker 定義の追加自体は markdown + schema の resource 追加で完結し、code 側の worker 一覧変更是非は無い（discovery 機構が dir を走査するため）。
- `tool-operate` alias 追加は args.rs の static table 1 行 + HELP + test で完結する。

## Alternatives and Comparison

論点: `command` / `process` の fail-closed gate（grants / tasks が空なら scope 構築時 `POLICY_VIOLATION`）と、issue の「無許可時は利用可能なツールの範囲で実行するか blocked を返す」の両立方法。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: `command`（と `process`）を宣言したままにする | 実装変更不要。gate が既存の安全契約をそのまま適用する | grants / tasks 未設定の環境では worker が model 開始前に必ず失敗し事実上使えない。builtin 環境（両方 `{}`）では即 dead | 候補 |
| B: `command` / `process` を宣言しない | gate に触れず常に起動可能。read 系 tools だけで縮退運用できる | issue が要求する tools 集合を満たせず、grant 環境でも `command` を永久に使えない。worker の売り（操作 actor）が大きく痩せる | 候補 |
| C: `command` / `process` を optional（soft）tool 化する新機構 | issue の「利用可能な範囲で実行」に最も忠実。tool list / prompt へ「利用不可」を surface できる | scope 構築の fail-closed 契約を変更する runtime / core 変更が必要。既存 worker への影響範囲と後方互換の設計が要る | 候補 |
| D: `command` を宣言し、grant-less 環境では他 worker を使う旨を文書化。run の `POLICY_VIOLATION` を「blocked」の結果とみなす | 最小差分。既存の fail-closed 意味論を blocked の表現として再利用できる | 「blocked」が run 自体の起動失敗 error であり worker output の `status: blocked` ではない。partial 実行への graceful 縮退は無い | 候補 |

## Tentative Conclusions and Hypotheses

- 仮説: Option C が issue の意図（無許可時に利用可能な tools で縮退 or blocked 返却）に最も近いが、scope 構築 gate の semantics 変更を伴うため runtime / core 改修が必要。Option D は既存契約の解釈で済む最小案であり、Option A と実質同じ機構を「blocked の意味づけ」で補強したもの。
- 仮説: `process` を初期 tools に含めるか否かは Option 選択と連動する。fail-closed を避ける方向なら初期 tools から `process` を外す判断もあり得る（`command` だけの宣言なら tasks 側 gate は発火しない）。
- 確認済み: worker 定義・schema・alias の追加面積は小さい。本 issue の実質的な設計負荷は policy gate との整合（上記 Option）と、global-only の execution 制限で multi-step 反復が足りるかの 2 点に集中する。
- 確認済み: `directlyComparable=false` 固定・`meta.changes` 常に空・`cheaper usage` での savings 計測という計測面の要求は、既存機構と整合しており追加設計はほぼ不要。

## Open Questions

- command-grant / process-tasks 不在時の挙動: Option A/B/C/D のどれを採るか（spec / ADR で確定）。C を採る場合の soft-tool 機構の形状（scope 構築時に対象 tool を skip して tool list / prompt で明示するか、worker-level marker とするか）。
- `process` を初期 tools に含めるか。含める場合、`command` と `process` 両方の gate を同時に回避する環境条件が厳しくなる点をどう扱うか。
- execution 制限: global 制限（maxTurns=8 等）内に収めるか、per-worker override 機構を新設するか（後者は strict `cheaper` keys の拡張 = core 変更）。
- `workload.outputPerInput` の暫定値（既存 worker は 0.05 / 0.3 / 1.0 等を使用）。
- worker 別 APM skill（`cheaper-tool-operate`）を追加するか。
- `ollama_e2e.rs` での実 provider 検証を tool-operator にも拡げるか。

## Promotion Candidates

- [x] spec-doc needed: `tool-operator` worker の input / output schema、tools 集合、evidence 表現、status 意味論、alias、計測要件の製品要件。 → `../specs/tool-operator-built-in-worker.md`
- [ ] adr-doc needed: `command` / `process` の fail-closed gate と「無許可時は縮退 or blocked」の両立方式（Option A/B/C/D の決定。core 変更の要否を含む）。

## Sources and Relations

source は issue #102。`command` tool の grant / trust boundary の背景は `policy-gated-command-tool` discovery・spec を参照（`command` は #101 で導入済みで、本 worker はその consumer となる）。

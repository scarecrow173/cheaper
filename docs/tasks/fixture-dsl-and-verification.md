---
id: "TASK-034EUvOR5z2gLPUgzBappw"
type: "task"
status: "todo"
title: "T4: fixture DSL・dev-seam/regression/golden test 実装"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-tool-operator-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-tool-operator-built-in-worker.md"
    - "docs/tasks/optional-tools-core-machinery.md"
    - "docs/tasks/registry-degrade-and-prompt-surfacing.md"
    - "docs/tasks/tool-operator-worker-and-alias.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/tool-operator-built-in-worker.md"
  references: []
  defers: []
  deferred-by: []
---

# T4: fixture DSL・dev-seam/regression/golden test 実装

## Scope

approved test-spec `docs/test-specs/tool-operator-built-in-worker.md`（plan
作成時点で存在済み — 本 task で作成・変更しない）が pin する行為保証を
検証する test を実装する。前提として `FixtureWorker.optional_tools` の
fixture DSL knob（snake_case DSL key）と `to_markdown()` の
`optionalTools:` emit を先行実装する — regression harness が
`f.worker_markdown()` を無条件で `.cheaper/workers/<name>.md` へ書き込む
ため、file seed は clobber されて degraded case を表現不能であり、全
regression/dev-seam case を DSL 経由で表現するのが必須（design §(i)、
Risk #4）。

- In: `FixtureWorker.optional_tools` field + `to_markdown()` emit、
  run_dev_seam.rs の全 fixture test（degraded / granted / capability-denied /
  mixed / UNKNOWN_TOOL / undeclared write-edit / status 各値 / changes-empty /
  directlyComparable / usage record）、runtime 層 `FakeSessionFactory` の
  prompt assert、regression corpus case + expected、worker_validate golden
  accept/reject fixture。
- Out: `optionalTools` の parse/validate 実装（T1）、degrade path 本体・
  prompt surfacing（T2）、worker 定義・schema・alias（T3）、test-spec
  自体の変更、docs/APM（T5）、`tools.degraded` span field の自動 assert
  （accepted trade-off — `degraded_names()` unit test + prompt 注記固定で
  代替検証済み）、`ollama_e2e.rs` 実 provider e2e。

### 所有ファイル

- `crates/cheaper-runtime/src/dev/fixture.rs` —
  `FixtureWorker.optional_tools: Vec<String>`（struct 既存の
  `#[serde(default)]` に乗る field 追加 — fixture DSL key は他 field と
  同じく snake_case `optional_tools`。rename は不要）+ `to_markdown()`
  （:291-312）が非空時のみ `cheaper:` 配下へ frontmatter contract key
  `optionalTools:` を emit。
- `crates/cheaper-cli/tests/run_dev_seam.rs` — degraded run（builtin
  default policy で `POLICY_VIOLATION` 無しに model 到達）・grant 定義環境
  run（`command_user_policy` + `FIXTURE_COMMAND_CALL` で実 spawn — marker
  file 到達で spawn と grant env 伝播を証明）・capability-denied run
  （`tools.allow` 除外は従来通り run 全体 fail — `missing` details 付き
  `POLICY_VIOLATION`）・mixed-policy 片側縮退（`process.tasks` 非空 ×
  `command.grants` 空 / その逆）・scripted degraded-name call →
  `UNKNOWN_TOOL`・scripted undeclared `write` / `edit` call の reject・
  `status` 各値（`success` / `partial` / `failed` / `blocked`）の出力・
  `meta.changes` 空・`directlyComparable=false`・usage record の fixture
  test。
- `crates/cheaper-runtime/tests/`（`FakeSessionFactory` handle 層 —
  cli 層ではない）— 縮退 run の system prompt に実登録集合 +
  `- Declared but unavailable under current policy: ...` 注記が含まれ、
  `status` enum に `"blocked"` を持たない output schema では blocked
  誘導行が emit されない conditional 分岐を assert。
- `crates/cheaper-cli/tests/regression/{cases,expected}/` — degraded run
  の regression corpus case（`FixtureWorker` DSL 経由 — file seed は
  harness に clobber されるため不可）。expected は
  `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression`
  で生成して diff review する。
- `crates/cheaper-core` worker_validate golden fixture — `optionalTools` の
  accept（`dump_worker_def` conditional emit で正規化結果を比較）/
  reject（reason 固定）fixture。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-tool-operator-built-in-worker.md` — 実装対象 plan（T4）
- `docs/designs/tool-operator-built-in-worker.md` — §(i) file map・
  Verification Notes
- `docs/specs/tool-operator-built-in-worker.md` — AC2–AC11
- `docs/test-specs/tool-operator-built-in-worker.md` — 検証 spec
  （全 Rules / Guarantees の正本）

## Dependencies

- `docs/tasks/optional-tools-core-machinery.md`（T1 — `optionalTools`
  parse/validate + `dump_worker_def` conditional emit）
- `docs/tasks/registry-degrade-and-prompt-surfacing.md`（T2 — degrade
  path + prompt surfacing が検証対象）
- `docs/tasks/tool-operator-worker-and-alias.md`（T3 — degraded-run
  fixture test が実 worker file `tool-operator.md` に依存）

## Steps

1. `fixture.rs` に `FixtureWorker.optional_tools`（snake_case DSL key）を
   追加し、`to_markdown()` が非空時のみ `cheaper:` 配下へ
   `optionalTools:` を emit するようにする（先行 item — 以降の全
   regression/dev-seam case は DSL 経由で表現する）。
2. `run_dev_seam.rs` に test-spec の Rules が要求する fixture test を追加
   する: degraded run・grant-env run・capability-denied run・mixed-policy・
   scripted `UNKNOWN_TOOL`・undeclared `write`/`edit` reject・`status` 4 値
   （非 success は `unresolved` + `nextActions` populated・`summary` 非空）・
   `meta.changes` 空・`directlyComparable=false`・`Fixture::usage_log()`
   の run record assert。
3. runtime `FakeSessionFactory` で縮退 run の system prompt（実登録集合 +
   利用不可注記 + conditional blocked 分岐）を assert する。
4. AC5 用に巨大中間 tool 出力を発生させる scripted run で envelope
   `result` が中間生出力を含まず `output.maxTokens` 以内の bounded result
   であることを assert する。AC6 は既存 generic regression case
   （`run-max-turns.json` / `run-max-tool-calls.json` /
   `run-timeout-delay.json`）を coverage として引用し、縮退 run 固有の
   経路差分がある場合のみ tool-operator 向け case を追加する。
5. regression corpus case を DSL 経由で追加し、
   `CHEAPER_REGRESSION_CAPTURE=1` で expected を生成して diff review
   する。worker_validate golden の accept/reject fixture を追加する。
6. design Verification Notes の未チェック項目を確認する — `optionalTools`
   validate 規則の unit test（T1 で実装済みの確認）、grant 定義環境の
   `command` 登録 + 実 spawn、`command` × writeScope の hard reject 維持、
   workflow `kind: worker` job の同一縮退挙動。

## Done When

- [ ] degraded run の model 到達・grant-env の実 spawn・capability-denied
  の run 全体 fail・mixed-policy の片側のみ列挙・`UNKNOWN_TOOL`・
  `status` 4 値（blocked は envelope `result.status == "blocked"` で
  run-level error envelope と区別）・prompt 注記 assert・`meta.changes`
  空・`directlyComparable=false`・usage record の test が全て pass
- [ ] regression corpus case + expected（capture 生成・diff review 済み）
  と worker_validate golden accept/reject fixture が pass
- [ ] `run_fake_stream_command_worker_denied_without_grants` が無変更で
  green — 非 opt-in worker の fail-closed 意味論不変（Risk #6）
- [ ] `optionalTools` は strictly capability-shrinking — admission
  （`tools.allow` / `check_capabilities`）と call 経路の認可機構（grant
  解決・argv pattern・executable pin/検証）は一切緩めていない

## Rollback

- task 単位の coherent commit を `git revert` — fixture DSL field・test・
  corpus/golden fixture を unit として除去する。T5 以降は docs 記述が
  挙動を参照するため、T4 revert 時は T5 も先に unwind する（逆順 unwind
  か fix-forward）。
- revert 後に
  `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  と `mise exec -- cargo test -p cheaper-cli --test regression --features cheaper-cli/dev-seam`
  が pass すること。

## Verification

- [ ] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --test regression --features cheaper-cli/dev-seam` exits 0（`regression` target は `required-features = ["dev-seam"]` で gate 済みのため flag 必須）
- [ ] `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression` で expected を生成し diff review 済み（capture 自体は検証でなく生成手順 — 生成物の commit 前 review が必須）

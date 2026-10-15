---
id: "ADR-034rzTJYDRmre6D5OS8HDW"
type: "adr"
status: "accepted"
title: "Normalize native hook payloads to a canonical routing request via per-target adapters and a shared policy module"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/43"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
    - "https://docs.devin.ai/desktop/cascade/hooks"
    - "https://docs.github.com/en/copilot/reference/hooks-configuration"
    - "https://developers.openai.com/codex/hooks"
    - "https://github.com/openai/codex/issues/18491"
    - "https://github.com/openai/codex/issues/20204"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "./copilot-adapter-separate-script-never-nonzero-exit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/apm-skills-and-pre-read-hook.md"
    - "../designs/copilot-pre-read-hook-adapter.md"
    - "../specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "../specs/case-insensitive-native-tools-and-devin-registration.md"
    - "../designs/devin-native-hooks-and-apm-bootstrap.md"
  refines: []
  refined-by:
    - "./apm-distributed-devin-native-hook-bootstrap.md"
  derives-from:
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Normalize native hook payloads to a canonical routing request via per-target adapters and a shared policy module

## Context and Problem Statement

cheaper は APM package の `.apm/hooks/` bundle を全 target に verbatim
deploy し、generated hooks config 経由で各 Coding Agent の native hook
event に接続する（ADR-034r05iHrZUE6W9RDF0V73 / ADR-034r05iHrZUE6WizQ4LgPa）。
現状の adapter 実装は Windsurf の `pre_read_code` と Copilot の `preToolUse`
だけが機能し、Claude Code / Cursor / Codex では native event が登録されて
いないため large-read routing が到達しない。

per-target adapter が増えると、判定ロジック（workspace 内 path 解決、token
threshold、`count-tokens` spawn、`FILE_TOO_LARGE` fail-closed、delegation
guidance）が adapter ごとに複製されるリスクがある。実際、既存 Windsurf /
Copilot adapter は既に同一ロジックを複製しており、issue #43 はその集約を
含む unified adapter architecture を要求する。また issue #42 が要求する
shell 経由の read 検出は、shell invocation を target 非依存で policy 層へ
届ける seam がないと接続できない。

## Decision Drivers

* APM を distribution layer として維持し、独自 installer に置き換えない。
* large-read 判定は target 非依存に単一化し、判定重複を排除する。
* block response / exit semantics / fail semantics は target ごとに異なる
  （exit code 2、`permissionDecision` JSON、`hookSpecificOutput`、
  `{permission: "deny"}`、never-nonzero）。adapter がそれぞれを吸収する。
* Copilot の never-nonzero invariant を維持する（ADR-034r05iHrZUE6WizQ4LgPa）。
* `kind: "shell"` の canonical request により #42 が adapter を触らずに
  downstream policy へ接続できる。
* APM の hook entry pass-through（matcher・nested `hooks` group の
  verbatim deploy を 2026-09-19 に実測確認）で Codex native schema を
  表現できるが、shell command field が target によって分岐する
  （Windsurf `command_line`、Claude `tool_input.command`、Cursor `command`、
  Copilot `toolArgs.command`、Codex `tool_input.command`）ため正規化が必要。

## Considered Options

* Option 1: canonical routing request + shared policy module + per-target
  adapter script。各 adapter は native event の parse / normalize /
  response 変換のみを持ち、判定は `routing-policy.js`（仮）に集約する。
* Option 2: 単一の universal adapter script が全 target の payload shape を
  sniff して分岐する。`hooks.json` の event 登録は増やすが adapter は 1 つ。
* Option 3: per-target の install script / installer を別途用意し、APM から
  生成される config を補正する。

## Decision Outcome

Chosen option: "Option 1: canonical routing request + shared policy module +
per-target adapter script"。

判定ロジックを `routing-policy.js` に集約し、adapter は native event の
parse / canonical request 化 / verdict の native response 変換 / fail
semantics に限定する。これにより large-read policy が target 非依存で単一
になり、#42 が shell request の downstream に接続できる seam も得られる。
Option 2 は payload shape の分岐を 1 script に集約するため、target 固有の
fail semantics（Copilot never-nonzero、Claude/Codex JSON deny、Cursor
permission JSON、Windsurf exit 2）が 1 つの責務過多な module に混ざり、
unit test の fixture も肥大化するため不採用。Option 3 は APM による
verbatim deploy の前提を崩し、生成 config と source `hooks.json` の差分管理を
追加するため不採用。

Codex は source inspection で `PreToolUse` payload（`tool_name` /
`tool_input`）と matcher group config（`{hooks: {"PreToolUse": [{matcher,
hooks: [{type:"command", command}]}]}}`）が確認でき、APM 経由で nested
group を deploy できることが実測された。ただし file-read tool が
`PreToolUse` を emit しない upstream gap は残存するため、Codex adapter は
shell request（`Bash` tool の `tool_input.command`）までをカバーし、
file-read は unsupported として matrix に記録する。Windsurf `pre_run_command`
は shell request を policy へ届ける seam のみ提供し、本 chain では常に
allow を返す（command 内の read candidate 抽出は #42）。

### Consequences

* Good, because large-read 判定が単一 module に集約され、target 追加時の
  adapter は parse / normalize / response 変換のみを実装すればよい。
* Good, because APM verbatim deploy を維持したまま Claude Code / Cursor /
  Codex の native schema に到達できる。
* Good, because `kind: "shell"` request が #42 の shell read detection の
  接続点になる。
* Bad, because `pre_read_code` 登録は全 target に残り、non-windsurf では
  no-op event として維持される（既存 ADR どおり unavoidable）。
* Bad, because Codex file-read は upstream が emit しない限り unsupported
  のまま残る。

### Confirmation

* `pnpm test` の adapter contract test で、representative payload の
  normalize / deny / fail-open が target ごとに検証される。
* `pnpm test` の `apm-install-targets.test.ts` で generated config の
  event 名 / adapter deploy / `.json` 欠落 drift assert が検証される。
* `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md` の
  target matrix が implementation と実測に一致する。

## Implementation Plan

* Affected paths:
  * `packages/apm/.apm/hooks/` — shared policy module、新規 adapter script、
    `hooks.json` の event 登録を更新。
  * `packages/cli/test/e2e/` — adapter contract test、
    `apm-install-targets.test.ts` の per-target assertion 追加。
  * `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`、
    `docs/guides/parent-agent-integration.md` — matrix / guide 更新。
* Patterns to follow:
  * `pre-read-code.js` / `pre-read-code-copilot.js` の stdin JSON parse、
    workspace containment、fail-open 方針、guidance shape。
  * Copilot adapter の never-nonzero invariant（exit 0 + deny JSON）。
  * APM verbatim deploy と `.json` asset 欠落 drift assert の既存方針。
* Patterns to avoid:
  * 判定ロジックを adapter ごとに複製する。
  * `hooks.json` を target ごとに補正する installer 側 patch。
  * `pre_read_code` 以外の Windsurf event 名や payload 形を勝手に拡張する。
* Dependencies or configuration:
  * 新規 dependency は追加しない（`node` builtin のみ）。
  * `CHEAPER_CLI` / `CHEAPER_READ_TOKEN_THRESHOLD` /
    `CHEAPER_COUNT_MAX_BYTES` の既存 env contract を継続する。

## Verification

* [ ] `pnpm test` で adapter contract test 全件 pass。
* [ ] `pnpm test` の `apm-install-targets.test.ts` で各 target の
      generated config に native event / adapter command が含まれる。
* [ ] `mise run check`（typecheck / lint / test）が通る。
* [ ] matrix 更新により Codex file-read unsupported / shell-only の記述が
      実装と一致している。

## Pros and Cons

### Option 1

* Good, because 判定と adapter の責務を分離でき、target ごとの差分が
  response contract と payload normalize に限定される。
* Good, because #42 が `kind: "shell"` request へ直接接続できる。
* Good, because adapter 単位の contract test が書きやすい。
* Bad, because adapter script が増える分、APM bundle の file 数は増える。

### Option 2

* Good, because hook script が 1 つで済み、`hooks.json` の event だけ増やせばよい。
* Bad, because fail semantics が target ごとに異なるため 1 script では
  never-nonzero（Copilot）と JSON deny（Claude/Codex/Cursor）を分岐
  させる複雑さが残り、test fixture が肥大化する。
* Bad, because native payload shape の sniff 判定が脆弱になりやすい。

### Option 3

* Good, because target ごとの config shape を完全に制御できる。
* Bad, because APM の verbatim deploy を前提とする現在の配布層を崩し、
  generated config との差分管理・drift 検出を別途必要とする。
* Bad, because issue の scope（APM 維持）と矛盾する。

## More Information

* Issue #43 と SPEC-034rzTJydXfLvQhTIqiDqW が要求と完了条件の正。
* DISC-034r05iHy2gdnpdai9KGpW に Codex re-verification と APM pass-through
  実測の詳細を追記する。
* Windsurf docs: https://docs.devin.ai/desktop/cascade/hooks
* Copilot docs: https://docs.github.com/en/copilot/reference/hooks-configuration
* Codex docs: https://developers.openai.com/codex/hooks
* Codex upstream gap: https://github.com/openai/codex/issues/18491 /
  https://github.com/openai/codex/issues/20204

---
id: "ADR-034r05iHrZUE6W71rbZ6aR"
type: "adr"
status: "accepted"
title: "Absorb provider OAuth and API-key login into the cheaper CLI (cheaper auth)"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/v2-productization-initiative-briefing.md"
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
    - "./distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/auth-login-command.md"
    - "../designs/auth-command.md"
    - "../plans/auth-command.md"
    - "./one-shot-json-first-cli.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/v2-productization-initiative-briefing.md"
  derived-by:
    - "../designs/auth-command.md"
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

# 14. Absorb provider OAuth and API-key login into the cheaper CLI (cheaper auth)

## Context and Problem Statement

production provider E2E（DISC-034r05iHy2gdnp7GkO7QDK Phase 9）の target は OpenAI だが、
ユーザーは API key を持たず ChatGPT サブスクリプションのみを持つ。
Pi runtime は `openai-codex` provider の ChatGPT OAuth flow を built-in
で持つが、login 実行は `pi` CLI の TUI を前提とする。`pi` 本体の
install を consumer に要求すると、APM package のみで完結するという
self-contained 配布方針（ADR-034r05iHrZUE6W5HVbwRSz）が崩れる。

`ModelRuntime.login(providerId, type, interaction)` は public API であり、
credential を Pi auth store（`~/.pi/agent/auth.json`）へ永続化するため、
cheaper CLI が interaction bridge を提供してこの flow を直接吸収できる。

本 ADR は SPEC-034r05iHK8bA7ghSWRbNU8 / DESIGN-034r05iHy2gdnoJ5j3v9Nk で確定した判断を遡及的に記録する
ものであり、実装は PLAN-034r05iHE1FbkoZibpg86T / TASK-034r05iHQeQpDqapXCLs3G で完了済みである。

## Decision Drivers

- `pi` CLI の install を要求しない（self-contained 配布の維持）
- OAuth flow そのものは pi-ai の provider-owned 実装を再利用し、
  再実装しない
- credential 等の secret は stdout / stderr / envelope に出力しない
- bundled CLI（`.apm/hooks/cli/bin/cheaper.mjs`）でも login が動作する
- 非対話環境（stdin 非 TTY）では分かりやすく失敗させる

## Considered Options

- `pi` CLI install を要求 — self-contained 配布方針と矛盾し、bundled
  CLI のみで動く環境を壊す
- OAuth flow を cheaper 側で再実装 — pi-ai が provider-owned で保守する
  flow（client_id・token exchange・refresh）の重複になり、drift と
  保守負荷を生む
- `ModelRuntime.login()` を cheaper CLI に吸収（`cheaper auth`）—
  provider-owned flow を public API 経由で再利用し、interaction のみ
  readline bridge で実装（採用）

## Decision Outcome

Chosen option: "`ModelRuntime.login()` を cheaper CLI に吸収"。

- `cheaper auth status|login <provider>|logout <provider>` を CLI
  コマンドとして提供。`--method browser|device` で OAuth method の
  select prompt を自動回答、`--type api_key` で API key login を処理
- credential は Pi auth store（`~/.pi/agent/auth.json`）に永続化。
  cheaper 独自の credential store は作らない
- interaction は terminal readline bridge で実装し、`auth_url` /
  `device_code` / `progress` / `info` event は stderr 表示。secret
  prompt（API key・manual code）は raw-mode no-echo で読み取る
- stdin 非 TTY では prompt が読めないため明示的に失敗させ、device
  method 等の代替を案内する
- bundled CLI では `registerBunOAuthFlows` で OAuth flow を登録し、
  Windows の browser 起動は `rundll32` 経由で行う

### Consequences

* Good, because `pi` install なしに ChatGPT OAuth / API key login が
  bundled CLI を含む全 install 経路で動く。
* Good, because OAuth flow は pi-ai の provider-owned 実装に追随する
  ため、provider 側の flow 変更に cheaper 独自実装が drift しない。
* Bad, because credential の永続化先が Pi auth store に固定される。
  cheaper 独自の store 抽象は持たない。
* Bad, because login は interactive コマンドであり one-shot JSON-first
  の CLI process model（ADR-034r05iHlWosDpKs4eBT59）の例外ケースになる。envelope は
  `{provider, authenticated}` 等の最小 result に留める。

### Confirmation

SPEC-034r05iHK8bA7ghSWRbNU8 の acceptance criteria（status / device login / logout /
未知 provider の error / bundled CLI での動作）は全て完了済み。
EXP-0005 で bundled CLI 経由の OAuth 付き `bulk-read` 実動作を確認済み。

## Implementation Plan

* Affected paths: `packages/cli/src/auth.ts`, `packages/cli/src/cli.ts`,
  `packages/cli/src/args.ts`, `packages/runtime-pi/src/auth.ts`
* Patterns to follow: secret は envelope / stdout / stderr に含めない。
  secret prompt は raw-mode no-echo。非 TTY は明示 error
* Patterns to avoid: OAuth flow の自前実装、`pi` CLI の subprocess 呼び
  出し、独自 credential store
* Dependencies or configuration: `registerBunOAuthFlows`（bundle 互換）、
  Windows browser 起動は `rundll32`

## Verification

* [x] `cheaper auth status` が credential 一覧（secret 値なし）を
      envelope/text で返す
* [x] `cheaper auth login openai-codex --method device` が device code
      を表示して待機する
* [x] `cheaper auth logout` で credential が削除される
* [x] 未知 provider / 未対応 type で適切な error + exit code
* [x] bundle 再生成後も `auth` コマンドが deploy 済み bundle で動く
      （EXP-0005）

## Pros and Cons of the Options

### `pi` CLI install を要求

* Good, because login UX（TUI）を pi 本体に任せられ、cheaper 側の
  実装が不要になる。
* Bad, because APM package のみで完結する self-contained 配布方針を
  否定し、bundled CLI のみの環境で provider login 不能になる。

### OAuth flow の再実装

* Good, because pi-ai への依存を auth 面だけ切れる。
* Bad, because provider-owned の client_id / token exchange / refresh
  flow を重複実装することになり、provider 側変更への追随コストと
  secret 取り扱いの監査面が増える。

### `ModelRuntime.login()` の吸収 (chosen)

* Good, because provider-owned flow を public API 経由で再利用しつつ、
  `pi` install なし・bundled CLI 対応を両立できる。credential store
  も Pi 側の既存機構に委譲できる。
* Bad, because cheaper CLI に interactive コマンドが増え、auth store
  の配置が Pi の実装詳細に結合する。

## More Information

* `docs/specs/auth-login-command.md` — 要求・acceptance criteria
* `docs/designs/auth-command.md` — コマンド面と interaction
  bridge の設計
* `docs/plans/auth-command.md` — 実装計画
* ADR-034r05iHlWosDpKs4eBT59 — one-shot JSON-first CLI process model（auth はこの model
  の例外として interactive を許容）
* ADR-034r05iHrZUE6W5HVbwRSz — bundled CLI 配布（auth が bundled 環境で動く前提条件）

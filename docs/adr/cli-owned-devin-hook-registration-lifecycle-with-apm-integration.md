---
id: "ADR-034H2WBNtC4We8dyC5lQZJ"
type: "adr"
status: "accepted"
title: "CLI-owned Devin hook registration lifecycle with APM integration"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
    - "https://microsoft.github.io/apm/reference/cli/lifecycle/"
    - "https://microsoft.github.io/apm/enterprise/lifecycle-scripts/"
    - "https://github.com/microsoft/apm/issues/2485"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  depends-on:
    - "./apm-distributed-devin-native-hook-bootstrap.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./apm-distributed-devin-native-hook-bootstrap.md"
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "./restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bounded-hook-read-request-contract.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# CLI-owned Devin hook registration lifecycle with APM integration

## Context and Problem Statement

現行の Devin native hook 登録は
`packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
— Windows PowerShell 5.1 限定・install のみ・APM lockfile の
ownership 外・uninstall/verify/repair を持たない ad-hoc script に
閉じている。PR #109 review はこれを「CLI が管理する安定した
registration lifecycle」へ要求した:

- install/update/uninstall が同一の収束モデルを通る
- POSIX を含むクロスプラットフォーム
- `inspect` 相当の状態確認と `remove` の実装
- 既存 consumer が旧 Windsurf cheaper hook の再配布を防ぐ移行経路
- APM lifecycle event との統合（consumer apm.yml の `lifecycle:`）

## Decision Drivers

* SPEC-034H2WAU6TQG6N9dmmmKNA の R10–R17 を満たすこと。
* ADR-034GssAufIiId3hgh4jrSQ の「APM 未対応 target への config
  登録補助」は継承するが、登録処理の所有を CLI binary へ移す
  （設定 file の読み書きロジックが script と CLI で二重化される
  状態を作らない）。
* APM lifecycle script は project trust が必須で失敗も abort しない
  — lifecycle は「収束のトリガ」であり強制機構にはしない。
* consumer manifest の per-dependency `targets:` は consumer 側の
  所有物。cheaper は手順と verify による検出で支援する。
* cheaper 管理外の hook / config entry は一切変更しない。

## Considered Options

* Option 1: `cheaper hook-config` に `sync`/`verify`/`remove` を実装
  し、PowerShell script は CLI 呼び出しの薄い wrapper へ縮退、
  POSIX は CLI 直接実行。APM lifecycle への接続は stdin event
  payload 受理オプションで提供する。
* Option 2: PowerShell script を保持して install のみ担わせ、
  別途 POSIX 用 script・uninstall script を追加する。
* Option 3: APM 本体へ `devin` target を追加して lifecycle ごと
  upstream に任せる。

## Decision Outcome

Chosen option: "Option 1: CLI が所有する hook-config lifecycle"。

`cheaper hook-config <verb>` を通常 CLI dispatch へ追加する:

- `sync`: project scope（既定 root = cwd）で `.devin/config.json` の
  `hooks.PreToolUse` へ canonical entry を冪等に upsert。旧 canonical
  matcher（`read|grep`）を含む own entry は現行 canonical
  （`read|grep|exec`）へ収束。`--scope user` は per-user config —
  POSIX `~/.config/devin/config.json` / Windows
  `%APPDATA%\devin\config.json` — を対象にする。未対応 scope は
  明示 error。apply 前に既知の登録元 scan を行い、cheaper 管理外に
  cheaper 互換 read hook command（`pre-read-code`/`pre-tool-use`
  を含む command 文字列）が検出された場合は書き込まず error —
  Devin は `read_config_from` で `.windsurf/`・`.claude/` の設定を
  既定 import するため、旧登録が残ると bounded 修正が発動しない
  （旧 PowerShell script の `$conflictRegex` gate を引き継ぐ）。
- `verify`: own entry の存在/一意性/旧 canonical drift、および既知
  legacy cheaper hook 登録元（`.windsurf/hooks.json`・
  `.devin/hooks.json`・`.devin/hooks.v1.json`・
  `.devin/config.local.json`・`.claude/settings*.json` 等）の scan
  を行い、問題があれば非ゼロ exit + 修復手順（`sync`・per-dep
  `targets:` 移行・`read_config_from` 無効化・APM reconcile）を
  出力。`.devin/hooks.v1.json` または `.devin/hooks.json` が
  `config.json` と併存する場合は shadow の可能性を warn として
  報告する（優先順位は Devin 公表仕様上未確認のため検出のみ）。
  書き込みは行わない。
- `remove`: own entry のみ除去（`.devin/config.json` のみ対象）。
  無い場合は no-op。改変/曖昧な entry は非破壊 error。
  `.devin/hooks.json`・`.windsurf/hooks.json` 等に残る非 APM 経由
  cheaper 登録は除去せず、`verify` で報告・手動除去を案内する。
- `--event-stdin`: APM lifecycle payload（`event`/`packages`/
  `scope`/timestamp/`cwd` の JSON）を stdin から読み、
  `post-install`/`post-update` → `sync`、`post-uninstall` →
  `remove` として写像する。`post-uninstall` は payload の
  `packages`/`scope` が cheaper package（name `cheaper` または slug
  `scarecrow173/cheaper`）を含む場合に限り remove する —
  consumer の任意の uninstall で cheaper 登録が消えないよう gate。
  判定不能な payload は remove せず `verify` 出力を返す。`pre-*` は
  no-op、未知 event は明示 error、malformed payload も error。
  project root は payload `cwd` → 呼出し cwd の順で解決する。

書き込み規則は PowerShell script の既存 contract を引き継ぐ:
malformed JSON（Devin config は JSONC を許容するが、本 CLI は
strict parse — コメント有りは「コメント除去か手動編集」の明示
error。書き込みでコメントは保持されない既知の制限）・非 schema・
重複/改変 own entry・symlink/reparse point は書かず error、
temp+rename の atomic replace（`fs::rename` は dest 存在時に
置換）、capability 確認との間の外部変更検出、`config.json.*.tmp`/
`.bak` の orphan sweep（起動時 + 失敗時）、無関係 entry の保持、
own entry の識別は command 完全一致 + matcher canonical 集合
（`{read|grep, read|grep|exec}` の 2 形を受理し現行へ収束）。
判定不能な変更は自動修復しない。capability 確認は PATH 解決の
`cheaper` binary に対し `cheaper hook targets --json` を実行する
（登録 command が PATH 経由で走るため）。capability line は
`"hookConfig": true` を追加し wrapper が対応 binary を判別できる
ようにする。

`install-devin-hooks.ps1` は Windows 向けの preview/`-Apply` UX を
残したまま、実際の config 編集を `cheaper hook-config` へ委譲する
wrapper にする。POSIX consumer には CLI を直接呼ぶ手順を提供する
（wrapper script の POSIX 版は作らず、CLI が実装本体）。

移行経路（2経路のうちどちらか）: (a) consumer が cheaper dep を
`targets: [agent-skills]` に絞る — project `targets` に
`agent-skills` が必須で、含まれないと dep 全体が skip され skills
も除去される。APM 0.28.0 の再 install/update が cheaper 所有の
`.windsurf/hooks/apm/*`・`apm-hooks.json`・`.windsurf/hooks.json`
内 cheaper entries を reconcile で除去する（実機検証済み —
discovery 記録）。Windsurf IDE と Devin の両方で guard を維持
したい混合 consumer はこちら。(b) Devin の `read_config_from` で
`windsurf`/`claude` を明示 `false` にする。`verify` は残存を検出
してこの手順を案内する。
APM lifecycle の `lifecycle:` 定義は consumer apm.yml に記述して
`apm lifecycle trust` する必要があり、`APM_NO_SCRIPTS=1` や
script 失敗でも APM 本体の操作は abort しない — これらは保証
不能な強制条件として docs に明記する。

### Consequences

* Good, because registration の read/write/verify/remove が単一の
  Rust 実装に集約され、PowerShell と POSIX で同一の収束ロジックが
  使える。
* Good, because APM lifecycle との接続が「stdin event → 収束 verb」
  の薄い写像になり、script 失敗でも consumer の APM 操作を壊さない。
* Good, because per-dep `targets:` という APM 標準機構で旧 hook の
  再配布を構造的に止められる。
* Bad, because `hook-config` の全部の機能は CLI binary の更新でしか
  届かず、skill 配布のみでは修正できない領域が増える。
* Bad, because lifecycle 統合は consumer の trust 前提であり、
  trust しない consumer では自動収束に依存できない（verify で
  検出して手順を案内するフォールバックが残る）。

### Confirmation

* `hook-config sync` → `verify` → `sync` → `remove` の連続操作が
  Windows と POSIX で冪等・非破壊であることの integration test。
* APM 0.28.0 実機で dep `targets:[agent-skills]` への移行 → 再
  install が cheaper Windsurf hook を除去し他は保持されること。
* `hook targets --json` capability line に `devin` が含まれ続け、
  `--event-stdin` の event 写像が test で固定されること。

## Implementation Plan

* Affected paths: `crates/cheaper-cli/src/args.rs`、
  `crates/cheaper-cli/src/cli.rs`、`crates/cheaper-cli/src/hook/`
  （`hook_config` module）、`crates/cheaper-cli/tests/integration/`、
  `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`、
  `scripts/test/devin-hooks-install.test.mjs`、
  `scripts/test/apm-install-targets.test.mjs`、
  `docs/guides/distribution.md`。
* Patterns to follow: `hook targets --json` の capability line、
  PowerShell script の config 読み書き contract（matcher+command
  ownership・atomic write・legacy scan）。
* Patterns to avoid: APM lockfile の直接改変、consumer manifest の
  自動書き換え、script 失敗で APM 操作を abort する期待。
* Dependencies or configuration: `cheaper hook-config` は既存 CLI
  dispatch/env と同じ binary に入るため新規 runtime 依存なし。

## Verification

* [ ] sync/verify/remove の integration test（project/user scope・
      無関係 entry 保持・malformed/改変/重複 error・concurrent
      変更検出）が Windows と POSIX で pass。
* [ ] `--event-stdin` が lifecycle event を sync/verify/remove へ
      写像し、unknown event を明示 error にする test が pass。
* [ ] per-dep `targets:` 移行の e2e（consumer fixture）が旧 hook の
      再配布を防ぐことを示す。
* [ ] distribution guide・skill docs が新 contract を反映する。

## Pros and Cons of the Options

### Option 1

* Good, because 判定・書き込み・検査・除去が単一実装になり、CLI
  binary があれば全 OS で同じ契約が使える。APM lifecycle とも
  stdin event という小さい接点で統合できる。
* Bad, because CLI の実装面積が増え、PS script の既存 test 契約
  （capability handshake・concurrent 検出等）を Rust へ移植する
  必要がある。

### Option 2

* Good, because 現行の PowerShell contract を残せる。
* Bad, because POSIX は別 script になり、install/update/uninstall が
  同一収束モデルを通らない — review の必須条件を満たせない。

### Option 3

* Good, because APM 本体が config 管理を担えば最も正統。
* Bad, because APM 本体はこの repository の変更境界外であり、
  PR の必須条件としては実装不能。

## More Information

* APM lifecycle/payload・trust・fail-soft の実測は discovery の
  review-driven scope expansion に記録。
* mixed `agent-skills` + MCP の既知問題: microsoft/apm#2485 —
  配布 docs の注意事項として言及する。

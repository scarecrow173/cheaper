---
id: "SPEC-003hd7W00xCikuCXnwHA4C"
type: "spec"
status: "implemented"
title: "Synchronize distributed cheaper-usage skill and parent-agent integration guide"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/34"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
  depends-on:
    - "./provider-auth-docs-runtime-sync.md"
    - "./auth-login-command.md"
    - "./huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "./apm-delegation-skills-and-read-hook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../designs/copilot-pre-read-hook-adapter.md"
    - "../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Synchronize distributed cheaper-usage skill and parent-agent integration guide

## What Should Be Built

parent Coding Agent に実際に配布される `cheaper-usage` skill
（`packages/apm/.apm/skills/cheaper-usage/SKILL.md`）と、user-facing の
`docs/guides/parent-agent-integration.md` を、現行の CLI surface・
hook contract・credential 保存先に同期させる。

## Why Now

Issue #34: main の parent-agent integration 実装と documentation を
突き合わせた結果、4 つの drift が確認された。

1. `cheaper-usage` が「pre-read hook currently functions on Windsurf
   only」と記述しているが、現在は GitHub Copilot 向け `preToolUse`
   adapter（`pre-read-code-copilot.js`）も実装・E2E 済みであり、
   Claude / Codex / Cursor は functional hook ではなく no-op target
   という canonical matrix と一致していない。
2. distributed skill の command / option 一覧に `auth status` /
   `auth login` / `auth logout` / `--method` / `--type` がなく、
   parent agent が CLI surface を完全に把握できない。
3. provider credential が「configured in the project」と読める記述が
   あったが、実装では credential は user-scope の
   `<userConfigDir>/pi/auth.json` に保存され、project-local
   `.cheaper/` には置かない（#33 で最小修正済み、本 spec で全面同期）。
4. `parent-agent-integration.md` が「計測不能・判定不能は fail-open」
   と一般化しているが、parseable な `FILE_TOO_LARGE` envelope は
   SPEC-034r05iHK8bA7gjwZTji8t / DESIGN-034r05iHy2gdnoLt52txFG で
   意図的に fail-closed（Windsurf: exit 2、
   Copilot: `permissionDecision: "deny"` + exit 0）としている。

`cheaper-usage` は parent agent が実際に読む instruction なので、
通常の説明文書より優先して現行 contract に同期する必要がある。

## Users and Value

- parent Coding Agent: 配布 skill から CLI surface・hook contract・
  credential 配置を正確に把握できる。
- `cheaper` の利用者: parent-agent integration guide が実装 contract
  と一致し、fail-open/fail-closed の挙動を誤解しない。
- リポジトリ開発者: canonical guide（providers.md）と配布物の
  記述が drift しない。

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の修正:
  - command / option 表に `auth status [provider]` /
    `auth login <provider>` / `auth logout <provider>` /
    `--method` / `--type <oauth|api_key>` を追加する
  - hook の target 別挙動を現行 contract に同期する
    （Windsurf: functional `pre_read_code`、Copilot: functional
    `preToolUse` adapter、Claude / Codex / Cursor: no-op）
  - credential が user-scope（`<userConfigDir>/pi/auth.json`）で
    project-local ではないことを #33 の canonical guide と矛盾なく
    記述する
- `docs/guides/parent-agent-integration.md` の修正:
  - fail-open / fail-closed contract を実装に合わせて修正する
    （infra failure = fail-open、parseable `FILE_TOO_LARGE` =
    fail-closed）
  - file type（binary 等）と fail-open を直接結び付けない
  - Copilot の JSON asset deploy gap（APM が copilot target へ
    `.json` asset を deploy しない既知制限）を維持する
- 整合確認（矛盾があれば最小限の修正）:
  - `docs/guides/usage.md` / `docs/guides/providers.md` / `README.md`
  - `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md`
  - `packages/apm/.apm/hooks/pre-read-code.js` /
    `pre-read-code-copilot.js`（contract の一次情報源として参照のみ）
  - `packages/cli/test/e2e/` の hook/install 系 test（contract の
    一次情報源として参照のみ）

### Out of Scope

- implementation behavior の変更（hook script・CLI・test code は
  変更しない）
- `docs/guides/providers.md` の再修正（#33 で完了、本 spec では
  canonical source として参照するのみ）
- Copilot JSON asset deploy gap の解消（upstream APM の修正待ち）

## Requirements

- `cheaper-usage` の command surface が `cheaper --help` の実 output
  と一致すること
- auth の保存先・provider setup の説明が #33 で修正した
  `docs/guides/providers.md`（canonical guide）と矛盾しないこと
- target 別 hook behavior が現行 E2E contract
  （`apm-install-targets.test.ts` / `pre-read-hook.test.ts` /
  `copilot-pre-read-hook.test.ts`）と一致すること
- `FILE_TOO_LARGE` が通常の infra failure（fail-open）と明確に
  区別されること
- Copilot の JSON asset deploy gap が引き続き明記されること

## Acceptance Criteria

- [ ] `cheaper-usage` の command surface が current CLI と一致する
- [ ] auth status / login / logout と relevant options が
  agent-facing skill から確認できる
- [ ] credential を project-local に置くような誤解を生む記述がない
- [ ] Windsurf と Copilot の functional hook support が正しく記述される
- [ ] Claude / Codex / Cursor の現状 no-op contract が既存 E2E /
  discovery と一致する
- [ ] `FILE_TOO_LARGE` は fail-closed、それ以外の hook infrastructure
  failure は fail-open と明確に区別される
- [ ] Copilot の JSON asset deploy gap が引き続き明記される
- [ ] #33 の provider/auth guide と矛盾しない
- [ ] implementation behavior は変更しない（docs のみの差分）

## Sources and Relations

- Issue #34（approved requirement）:
  https://github.com/scarecrow173/cheaper/issues/34
- 前提: Issue #33 / SPEC-034r2qodmutbif9L0DLskN（provider/auth
  canonical guide の修正）
- hook contract の一次情報源: `packages/apm/.apm/hooks/pre-read-code.js`、
  `packages/apm/.apm/hooks/pre-read-code-copilot.js`
- 関連: `cross-agent-hook-distribution-and-contract-matrix`
  （target matrix）、`huge-file-token-count-guard-with-fail-closed-
  hook-blocking` + `huge-file-byte-cap-guard-and-fail-closed-hook`
  （fail-closed 仕様）、`copilot-adapter-separate-script-never-
  nonzero-exit` + `copilot-pre-read-hook-adapter`（Copilot adapter
  contract）、`auth-login-command`（auth command surface）

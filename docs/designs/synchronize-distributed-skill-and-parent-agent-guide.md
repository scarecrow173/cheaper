---
id: "DESIGN-0005kq1c3Wo7rdnwZR8Sp3"
type: "design"
status: "approved"
title: "Synchronize distributed cheaper-usage skill and parent-agent integration guide"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  implemented-by: []
  depends-on:
    - "./provider-auth-guide-runtime-sync.md"
    - "./huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "./copilot-pre-read-hook-adapter.md"
    - "./apm-skills-and-pre-read-hook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md"
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Synchronize distributed cheaper-usage skill and parent-agent integration guide

## Context

SPEC-003hd7W00xCikuCXnwHA4C（Issue #34）の実装設計。
parent Coding Agent に配布される `cheaper-usage` skill と
`docs/guides/parent-agent-integration.md` が現行 CLI surface・
hook contract・credential 配置から drift している。両文書を実装
contract に同期する。実装コードは変更しない。

一次情報源:

- CLI surface: `cheaper --help` の実 output（`packages/cli`）
- auth contract: `packages/cli/src/auth.ts`、canonical guide は
  #33 で修正済みの `docs/guides/providers.md`
- hook contract: `packages/apm/.apm/hooks/pre-read-code.js`
  （Windsurf）、`packages/apm/.apm/hooks/pre-read-code-copilot.js`
  （Copilot adapter）
- E2E contract: `packages/cli/test/e2e/apm-install-targets.test.ts`、
  `pre-read-hook.test.ts`、`copilot-pre-read-hook.test.ts`

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  - Commands ブロックと option 表へ auth 系 command
    （`auth status [provider]` / `auth login <provider>` /
    `auth logout <provider>`）と `--method` /
    `--type <oauth|api_key>` を追加する
  - 「The pre-read hook currently functions on Windsurf only」を
    現行 matrix（Windsurf + Copilot functional、Claude /
    Codex / Cursor no-op）に修正する
  - credential 記述を `<userConfigDir>/pi/auth.json` の
    user-scope 記述で確定させる（#33 で最小修正済みの箇所を
    全面同期）
- `docs/guides/parent-agent-integration.md`:
  - hooks の fail-open 記述を「infra failure は fail-open、
    parseable `FILE_TOO_LARGE` は fail-closed（Windsurf exit 2 /
    Copilot `permissionDecision: "deny"` + exit 0）」に修正する
  - 「boundary 外・binary・計測不能な file は fail-open」の
    記述を count/read contract に沿って修正し、file type と
    fail-open を直接結び付けない
  - Copilot JSON asset deploy gap の記述は維持する
- 整合確認: `docs/guides/usage.md` / `docs/guides/providers.md` /
  `README.md` / `cheaper-bulk-read` SKILL.md

### Out of Scope

- implementation code・hook script・test の変更
- `docs/guides/providers.md` の再修正
- Copilot JSON asset gap の解消

## Components and Boundaries

- **SKILL.md（agent-facing instruction）**: command surface・
  envelope contract・exit code・hook target matrix・credential
  配置を parent agent へ教える単位。provider 詳細は
  `docs/guides/providers.md`（canonical guide）への参照に留め、
  複製しない（ADR-066GRjHT22hXEgey2dpq7f）。
- **parent-agent-integration.md（user-facing guide）**: 配布
  primitive・install・target 別挙動・fail-open/closed contract
  を説明する単位。hook の実装分岐を一次情報源として記述する。
- **一次情報源（hook script・CLI help・E2E）**: 記述の正しさの
  判定基準。変更しない。

## Data and Control Flow

1. `cheaper --help` の実 output を skill の command/option 表と
   突合し、auth 系 3 command + `--method` / `--type` を追記する。
2. hook script の分岐（`process.exit(0)` / `exit(2)` / `deny()`）
   を列挙し、guide の fail-open/fail-closed 記述を実装の非対称
   contract に合わせる:
   - fail-open: malformed stdin / 対象外 event・tool / path
     解決不能 / workspace 外 / 非 file・stat 失敗 / 閾値以下の
     fast path / CLI spawn 失敗 / stdout 非 parseable / 非 0
     exit / `!ok`・`tokens` 非数値
   - fail-closed: parseable `FILE_TOO_LARGE` envelope（authoritative
     な large/unmeasurable verdict）→ Windsurf exit 2 +
     stderr guidance、Copilot `deny()` + exit 0
   - 閾値超過の measured tokens → 同じく block
3. target 別挙動を canonical matrix（DISC-034r05iHy2gdnpdai9KGpW 系
   `cross-agent-hook-distribution-and-contract-matrix`）と既存
   guide 記述・E2E に合わせて skill へ簡潔に反映する。
4. credential 記述を `<userConfigDir>/pi/auth.json`
   （`CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` →
   `~/.config/cheaper` で解決される user config dir 配下）と
   一致させ、project-local 誤解を排除する。

## Non-Functional Constraints

- documentation-only: hook script・CLI・test の挙動は変更しない
- 言語: `parent-agent-integration.md` は日本語（guide 群の規約）、
  `SKILL.md` は英語（agent-facing instruction の規約）
- Copilot JSON asset deploy gap の記述を削除・弱化しない

## Risks and Trade-offs

- リスク: skill に auth surface を追加することで将来 CLI が拡張
  された際に drift する。対策: providers.md を canonical source
  とし、skill には最小限の surface のみ記載する。
- リスク: `FILE_TOO_LARGE` fail-closed の説明が複雑化する。
  対策: 「parseable な `FILE_TOO_LARGE` envelope = 計測不能だが
  確実に巨大と authoritative に判定」という一文で実装の意図を
  伝え、infra failure 列挙は簡潔に留める。

## Verification Notes

- [ ] `cheaper --help` の実 output と skill の command/option 表が
  一致する
- [ ] skill の hook 記述が Windsurf functional・Copilot
  functional・他 no-op の matrix と一致する
- [ ] guide が `FILE_TOO_LARGE` fail-closed と infra failure
  fail-open を区別する
- [ ] credential が user-scope `<userConfigDir>/pi/auth.json` と
  記述され、project-local 誤解がない
- [ ] Copilot JSON asset gap の記述が維持されている
- [ ] `audit_docs.js --type all` と `mise run check` が pass する

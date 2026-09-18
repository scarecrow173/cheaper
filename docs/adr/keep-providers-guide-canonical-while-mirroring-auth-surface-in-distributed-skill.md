---
id: "ADR-066GRjHT22hXEgey2dpq7f"
type: "adr"
status: "accepted"
title: "providers guide を canonical とし distributed skill へは auth surface を要約反映する"
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
  implements: []
  implemented-by:
    - "../designs/synchronize-distributed-skill-and-parent-agent-guide.md"
  depends-on:
    - "./provider-auth-guide-auth-status-canonical-surface-auth-type.md"
    - "./copilot-adapter-separate-script-never-nonzero-exit.md"
    - "./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "../designs/copilot-pre-read-hook-adapter.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
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

# providers guide を canonical とし distributed skill へは auth surface を要約反映する

## Context and Problem Statement

Issue #34 / SPEC-003hd7W00xCikuCXnwHA4C: 配布される
`cheaper-usage` skill と `docs/guides/parent-agent-integration.md`
が現行実装から drift している。決めるべきこと: (a) provider/auth
情報を distributed skill にどの粒度で複製するか、(b) hook の
fail-open / fail-closed contract を guide 上でどう表現するか。

## Decision Drivers

- Issue #34 は「#33 の provider/auth guide を canonical source
  として参照する」と指定しており、配布 skill に provider 詳細を
  再複製することを求めていない。
- 一方で parent agent が実際に読む instruction なので、command
  surface（`auth status` / `auth login` / `auth logout` /
  `--method` / `--type`）と credential 保存先
  （`<userConfigDir>/pi/auth.json`、project-local ではない）は
  skill 単体で判別できる必要がある。
- hook の実装（`pre-read-code.js` / `pre-read-code-copilot.js`）は
  「parseable な `FILE_TOO_LARGE` envelope のみ fail-closed、
  それ以外の infra failure はすべて fail-open」という非対称な
  contract を持ち、file type（binary 等）とは無関係に count
  contract だけで判定する。
- 「同じ canonical information を複数文書へ詳細に複製しない」
  が本リポジトリの設計原則。

## Considered Options

### skill への provider/auth 情報の反映粒度

- **A: command surface と credential 保存先の要点のみ skill に
  記載し、provider 詳細は canonical guide（providers.md）への
  参照に留める**（採用）。agent が自律実行に必要な最小限の
  surface（auth 系 command / option、credential が user-scope）
  は skill 内で完結させ、login method 一覧や env var 例は guide
  へ譲る。
- **B: providers.md の内容を skill にほぼ複製する**。agent-facing
  instruction として冗長で、guide との drift リスクを増やす。
- **C: skill から guide へのリンクのみで auth 系 command を記載
  しない**。Issue #34 の acceptance（command surface が current
  CLI と一致、auth 系が skill から確認できる）を満たせない。

### fail-open / fail-closed の表現

- **A: 「計測不能・判定不能は fail-open」の一般化をやめ、
  infra failure（malformed payload / 対象外 event / 不存在・
  非 file / workspace 外 / CLI 起動失敗 / output 非 parseable /
  非 0 exit）= fail-open と parseable `FILE_TOO_LARGE` envelope
  = fail-closed を明確に区別する**（採用 — 実装 contract と
  SPEC-034r05iHK8bA7gjwZTji8t / DESIGN-034r05iHy2gdnoLt52txFG
  の意図に一致）。target 別の表面形
  （Windsurf: exit 2 / Copilot: `permissionDecision: "deny"` +
  exit 0）を併記する。
- **B: 「巨大 file は fail-closed、その他は fail-open」とだけ
  書く**。`FILE_TOO_LARGE` は token 超過とは別の「計測不能だが
  確実に巨大」と authoritative に判定する verdict であり、
  閾値超過 block と混同しやすい。

## Decision Outcome

Chosen option: "auth surface の要点のみ skill に記載して provider
詳細は canonical guide へ委ね、fail-open/fail-closed は実装の
非対称 contract に沿って明確に区別する", because Issue #34 が
providers.md を canonical source とすること・`FILE_TOO_LARGE`
を infra failure と区別することを approved requirement として
指定し、かつ hook 実装・既存 ADR/DESIGN と一致するため。

### Consequences

* Good, because distributed skill が CLI surface として自己完結
  しつつ、provider/auth の canonical source が providers.md の
  一箇所に保たれる。
* Good, because `FILE_TOO_LARGE` fail-closed が意図的仕様である
  ことが guide 上で明確になり、parent agent / 利用者が fail-open
  と誤認しない。
* Bad, because skill から guide への参照に依存する部分（login
  method の詳細等）は skill 単体では分からない（意図的な
  trade-off — 表層 drift 防止を優先）。

### Confirmation

- `cheaper --help` の実 output と skill の command 表を突合する
- `pre-read-code.js` / `pre-read-code-copilot.js` の分岐と guide
  の fail-open/fail-closed 記述を突合する
- `docs/guides/providers.md` の credential 記述と skill の記述を
  突合する

## Implementation Plan

* Affected paths: `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
  （command surface・hook matrix・credential）、
  `docs/guides/parent-agent-integration.md`（fail-open/closed
  contract）
* Patterns to follow: 実 output contract（`cheaper --help`）、
  hook script の分岐を一次情報源とする記述、canonical guide への
  参照
* Patterns to avoid: provider/auth 詳細の skill への複製、
  「計測不能・判定不能は fail-open」の過度な一般化、file type
  と fail-open の直接結合

## Verification

* [ ] `cheaper --help` output と skill command 表の一致
* [ ] hook script の各 `process.exit(0)` / `exit(2)` / `deny()`
  分岐と guide 記述の一致
* [ ] `docs/guides/providers.md` との矛盾がない
* [ ] `mise run check` が pass する

## Pros and Cons of the Options

### 反映粒度: Option A（採用）

* Good, because agent の自律実行に必要な surface は skill で完結
  し、詳細は canonical guide に一元化される。
* Bad, because provider 詳細確認には guide 参照が必要。

### 反映粒度: Option B（複製）

* Good, because skill 単体で provider 詳細まで分かる。
* Bad, because canonical information の複製になり drift 源になる。

### 反映粒度: Option C（リンクのみ）

* Good, because 記述量が最小。
* Bad, because acceptance の「command surface が current CLI と
  一致」「auth 系が skill から確認できる」を満たせない。

### fail-open 表現: Option B（単純二分）

* Good, because 記述が短い。
* Bad, because `FILE_TOO_LARGE` を閾値超過 block と同列に扱い、
  「計測不能だが fail-closed」という非対称仕様が伝わらない。

## More Information

- Issue #34: https://github.com/scarecrow173/cheaper/issues/34
- SPEC-003hd7W00xCikuCXnwHA4C（本 ADR の上流 spec）
- 一次情報源: `packages/apm/.apm/hooks/pre-read-code.js`、
  `packages/apm/.apm/hooks/pre-read-code-copilot.js`、
  `packages/cli/src/auth.ts`
- 関連: ADR-034r05iHrZUE6WizQ4LgPa（Copilot adapter は非 0 exit
  を出さない）、DESIGN-034r05iHy2gdnoLt52txFG 系
  `huge-file-byte-cap-guard-and-fail-closed-hook`（FILE_TOO_LARGE
  fail-closed 仕様）

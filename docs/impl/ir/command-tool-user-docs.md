---
id: "IMPL-VyGgdpBDL0vD8jXbMIVXSy"
type: "impl"
status: "completed"
title: "command tool user docs"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/guides/command.md"}
      - {"type":"doc","path":"docs/impl/ir/command-tool-user-docs.md"}
    modified:
      - {"type":"doc","path":"README.md"}
      - {"type":"doc","path":"docs/README.md"}
      - {"type":"doc","path":"docs/adr/README.md"}
      - {"type":"doc","path":"docs/guides/configuration.md"}
      - {"type":"doc","path":"docs/guides/workers.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/README.md"}
      - {"type":"doc","path":"docs/tasks/command-docs.md"}
      - {"type":"doc","path":"docs/test-specs/README.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/command-docs.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# command tool user docs

## Summary

T6。`command` tool の user-facing 文書を整備した。新規
`docs/guides/command.md` に gate・grant 宣言・argv pattern・実行制御・
trust/sandbox 限界を集約し、`configuration.md` の builtin `policy.yaml`
verbatim embed を `command` section + `tools.allow` の `command` で同期、
`workers.md` を三重ゲート（`tools` 宣言 ∩ `tools.allow` ∩
`command.grants`）+ writeScope 非両立の記述へ更新した。新規 guide に
伴い `docs/README.md` index と root `README.md` の guide links・policy
section 列挙を同期。併せて本 feature の index stale 行（tasks README
T1–T6・test-specs README・ADR README title）を修正した。

## Context

TASK-034EOHYMMH4FnfoVhmfPeP / PLAN-034EO1kmKwJ8MAHSHWyCws。T1–T5
（b706188・2cb6965・9445a64・4cf0e2a・80bb3c6）で実装と挙動が確定済み。
T6 の責務は、確定した信頼境界（grants は trusted layer のみ authoring、
`tools.allow` は exposure toggle）と trust/sandbox 限界の正直な開示
（OS sandbox 無し・ambient PATH provenance・capability 限界）を
user-facing docs に落とすこと。

## Implementation

### Changes

- `docs/guides/command.md`（新規）: `command` tool の canonical guide。
  - gate と信頼境界: Worker 三重ゲート・Workflow 二重ゲート、
    `tools.allow` は exposure toggle で authorization ではない、
    grants は builtin + user-scope のみ authoring 可で project
    `command` key 含有は `InvalidRuntimePolicy` で load 拒否
    （narrowing も不可）、empty grants → fail-closed
    `POLICY_VIOLATION`、非 expose runtime は dormant。
  - grant 宣言: `{executable, argv?, cwd?, env?, timeout?,
    killGrace?, maxOutputBytes?}` の YAML 例・命名規則・override
    畳み込み・executable 解決（bare name = ambient PATH→canonical
    pin、absolute path、workspace-resident 拒否、native magic、
    shell/shim + `command` 専用 deny set の 3 適用点）。
  - argv pattern: literal exact・`prefix*` prefix match・最終要素
    bare `*` の rest marker（zero-or-more）、rest 無しは arg 個数
    完全一致、metachar 拒否（match 前独立 check）、token ≤4096・
    args ≤256。
  - 実行・制御: shared managed-subprocess 基盤・独自 pool
    `"command"`・Exclusive root claim・details
    `{grant, argv, state, exitCode, signal, durationMs, stdout,
    stderr}`・`meta.changes` は実測 write/edit のみ・
    `savings.directlyComparable == false`。
  - trust / sandbox 限界（spec 必須記載事項）: OS sandbox 無し、
    `filesystem.denyPaths`/workspace boundary/writeScope が子の
    副作用を bound しない、argv path operand は filesystem 未検証、
    ambient PATH provenance + per-scope pin の残余 TOCTOU、
    config-driven executable（git config surface・build script）は
    repo content 信頼と等価、deny list は known-bad fail-fast で
    containment でない、grant `env` の injection 変数
    （`LD_PRELOAD`/`NODE_OPTIONS`）、writeScope 非両立（registration
    gate で fail-closed、manifest narrow は op admission で拒否の
    層非対称）、`process.tasks` の project extend との divergence。
- `docs/guides/configuration.md`: verbatim embed に `command`
  section（builtin `policy.yaml` の key 順そのまま）と
  `tools.allow` の `command` を追加。embed 後に `command` field・
  grant shape・argv pattern の要点と project 層の不可を説明し、
  command.md へ link。関連 section に command.md を追加。
- `docs/guides/workers.md`: `tools` 記述を三重ゲートへ更新し
  exposure ≠ authorization（project が flip しても executable を
  認可できない）を明記。`writeScope` 記述に `command` 非両立を追加。
  `tools.allow` bullet に command の認可は grants が担う旨を追加。
  関連 section に command.md。
- `docs/README.md`: User guides index に `command.md` 行を追加。
- `README.md`（root）: Documentation links に command guide を追加、
  `policy.yaml` section 列挙に `command` を追加。
- `docs/tasks/README.md`: T1–T6（+T2 `proc-extraction`）の stale
  `todo` 行を `done` へ同期（task front matter は全て `done`）。
- `docs/test-specs/README.md`: `policy-gated-command-tool.md` 行の
  status を `approved` へ同期（file front matter は T5 で approved
  済み）。
- `docs/adr/README.md`: `command-grant-...` 行の title を file
  front matter の日本語 title へ同期。
- `docs/tasks/command-docs.md`: status → done、Done When・
  Verification の全 check、changes/implemented-by 記録。

### Decisions During Implementation

- **新規 `command.md` を採用**（`managed-process.md` 拡張は却下）:
  `managed-process.md` は `process` tool（固定 argv named task）の
  単一責務 guide であり、`command` は独自の信頼境界・grant 語彙・
  trust 限界を持つ別 tool。plan file map も新規 file を第一選択肢
  としており、guide index への追加で発見性も上がる。
- **index stale 行の修正を同梱**: T1–T5 が file front matter の
  status を更新した際に README index 行を同期していなかったため
  audit が警告を出していた。本 feature の doc 群に限定して
  tasks/test-specs/adr README を同期（本 commit の「index 更新」
  scope の一部として扱う）。
- **canonical tool 一覧の 8→9 更新は該当箇所なし**: user-facing
  docs・root README・packages/apm skills（`cheaper-usage` 他）を
  grep したが、tool vocabulary を列挙するのは
  `configuration.md` の `tools.allow` embed（builtin mirror で
  同期済み）のみ。`workers.md` の built-in worker table は worker
  側の tool 宣言であり vocabulary 一覧ではないため変更なし。

## Related Experiments

### Adopted

なし。

### Rejected

- `managed-process.md` への `command` section 追加 — `process` と
  `command` は信頼境界（project extend 可 vs trusted layer only）が
  本質的に異なり、単一 guide に混ぜると exposure ≠ authorization
  の要点が埋もれる。

## Validation

- `node scripts/check-doc-ids.mjs` — exit 0。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
  — exit 0（本 feature 由来の index 警告 3 件を本 commit で解消。
  残 warning は全て pre-existing の relation backlink 系）。
- `grep -n "command:" docs/guides/configuration.md` — `command:`
  section が builtin `policy.yaml` と同じ key 順・同じ値で embed
  済み（diff 確認: `command` section 挿入位置・`tools.allow` 末尾
  の `command` のみ追加、他行は無変更）。
- `mise run check` は docs-only change のため未実行（task doc の
  Verification matrix でも T6 の pass condition は doc audit +
  embed diff 確認）。packages/apm は未変更のため
  `check-apm-payload.mjs` も未実行。

## Risks

- `command.md` の deny list 概要はカテゴリ＋代表例の記述であり、
  canonical source は `crates/cheaper-core/src/policy/command.rs` の
  `command_denied_executable_re()`。deny set 拡張時に guide との
  drift が起き得る（例示列挙で drift を最小化済み）。
- embed の `command` section コメントは `process` section の
  wording を踏襲した手書きで、builtin file 更新時に docs 側の
  手動同期が必要（embed は verbatim ではなくコメント付き mirror
  という既存 convention を維持）。

## Follow-ups

- plan の status は `approved` のまま — 全 task done 後の plan
  completed 化は本 task の scope 外（plan 運用判断に委ねる）。
- tool-operator worker 本体（issue #102）が land した際は
  `command` の使用面 guide を追記する余地あり。

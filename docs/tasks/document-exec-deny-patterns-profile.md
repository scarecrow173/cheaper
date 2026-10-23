---
id: "TASK-034KLYEMV6mRczcWiAQpy1"
type: "task"
status: "done"
title: "security.execDenyPatterns の guides・builtin policy.yaml 整備"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  implemented-by: []
  depends-on:
    - "docs/plans/opt-in-exec-deny-patterns.md"
    - "docs/tasks/add-core-security-policy-and-schema-wiring.md"
  blocks:
    - "docs/tasks/verify-exec-deny-patterns.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/opt-in-exec-deny-patterns.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  references:
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/discovery/opt-in-exec-deny-patterns.md"
    - "docs/guides/command.md"
    - "docs/guides/configuration.md"
    - "crates/cheaper-cli/resources/policy.yaml"
  defers: []
  deferred-by: []
---

# security.execDenyPatterns の guides・builtin policy.yaml 整備

## Work

plan `docs/plans/opt-in-exec-deny-patterns.md`
（PLAN-034KLWpEpVSlofJ7Idynh5）の T4。#163 で mandatory deny・
magic 判定が除去済みとなり stale 化した `command.md` の記述を
`security.execDenyPatterns`（opt-in deterministic executable
deny-list）の意味論へ書き換え、`configuration.md` の policy.yaml
リファレンスと builtin `resources/policy.yaml` に `security` 節を
追加する。design（DESIGN-034KLwE8GiJQ2zvQY20ob8）の Docs 更新
契約をそのまま実装する — 表現は「deterministic executable
deny-list」とし「保護」「sandbox」「containment」とは呼ばない
（spec R12）。

T1 の semantics 確定後に着手すれば T2/T3 と並列実行可能
（product code を共有しない）。

### 1. `docs/guides/command.md`

- `:32` — 「native magic … `command` 専用の deny list が以下を
  既定で拒否する」の記述は #163 で mandatory deny・magic 判定が
  除去済みのため stale。executable 解説（bare name/absolute
  path・ambient PATH 解決・canonical pin）は残し、deny 部分を
  `security.execDenyPatterns` の記述へ書き換える:
  - 意味論 — operator 宣言の regex 名リスト（既定 `[]` = deny
    無し）が grant executable の deterministic executable
    deny-list として効く。
  - match 規約 — basename 正規化（`/`・`\` 区切りの最終要素 →
    小文字化 → `.exe`/`.com` suffix strip → stem）・stem 完全
    一致（各 pattern は `^(?:pattern)$` 相当 — `sh` は `bash`
    に match しない）・case-insensitive・複数 pattern は OR・
    trusted layer（builtin + user `policy.yaml`）のみ authoring
    可・merge は配列の丸ごと置換。
  - 「deterministic executable deny-list であり containment
    ではない」記述。
  - **旧既定値**一覧 — discovery が git から復元した旧
    `command_denied_executable_re` の verbatim regex。
  - migration seed 例 — verbatim 1 要素版 + 分類別分解
    （interpreter / wrapper / loader / LOLBin / escape / shell）
    yaml。
  - migration 注記 — shell/shim 名（`sh`/`bash` 等）が旧 deny
    一覧に含まれない旨を明示（A8 は別 predicate として除去
    済み）。
- `:45`（`--grant` 節の bullet） — 「deny list 該当の spec は
  `InvalidRuntimePolicy`（`details.layer` = `"invocation"`）で
  reject」の記述を opt-in 前提に restate — merged re-resolve
  で `InvalidRuntimePolicy`、`details.layer` は**付かない**
  accepted gap（deny 元 builtin/user と grant 元 invocation
  が別 layer のため merged-map 評価では帰属不能）を正直に
  明記。
- `:80` — 「deny list は containment 機構ではない」注意を
  opt-in profile の表現へ引き継ぐ。
- 新規記述 — review で確定した補足を同節へ追加:
  1. `.*` は schema 上許容される明示的 deny-all — grants を
     持つ policy は全て load 失敗になるため意図利用のみ
     （substring 意図の唯一の表現でもある）。
  2. `executable_stem` の `\` 区切り正規化は unix でも `\` を
     分隔として扱い `.com` suffix も strip する — unix 上の
     `my\python.exe`・`foo.com` のような奇名が deny に match
     する false-positive（方向は fail-closed — 過剰に拒否する
     が漏らさない）。旧 builtin 由来の規約であり structural
     制約として明記する。
  3. blast radius — 1 つの denied resolved grant でも
     `pin_command_grants` 全体が `Err` となり command-tool
     scope 全体が失敗する（unpinnable grant と同規約 —
     既存意味論が deny で operator-reachable になるだけ）。

### 2. `docs/guides/configuration.md`

- policy.yaml リファレンスの yaml block（:153-159 `route` 節の
  後）に `security` 節を追加:

  ```yaml
  security:
    execDenyPatterns: []   # operator 宣言の executable 名 regex
                          # deny-list（既定: 空 = deny 無し。
                          # trusted layer のみ authoring 可）
  ```

- `route.rewrite.approvalBypass` 小節（:186 以降）と同型の
  `### security.execDenyPatterns` 小節を新設:
  - 既定 `[]`・`null`/未指定/`[]` の等価性。
  - trusted layer（builtin + user `policy.yaml`）のみ
    authoring 可 — project `.cheaper/policy.yaml` からの宣言は
    `InvalidRuntimePolicy`、invocation 層（`--grant` 合成）は
    `InvalidRequest` + `details.layer="invocation"`。
  - merge は配列の丸ごと置換（union しない — builtin `[a]` +
    user `[b]` → effective `[b]`）。
  - match 規約・3 適用点・非 containment の詳細は
    `command.md` へ委譲。

### 3. `crates/cheaper-cli/resources/policy.yaml`

- `route` の後（file 末尾）に `security:` 節を追加 —
  `execDenyPatterns: []` + trusted layer 限定・opt-in・非
  containment の comment。knob 可視化は `route.approvalBypass:
  []` 先例 — **明示 key を置く**（design が spec の実装判断を
  確定）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/opt-in-exec-deny-patterns.md`
  （PLAN-034KLWpEpVSlofJ7Idynh5 — T4）
- `docs/designs/opt-in-exec-deny-patterns.md`
  （DESIGN-034KLwE8GiJQ2zvQY20ob8 — Docs 更新契約が
  AUTHORITATIVE）
- `docs/specs/opt-in-exec-deny-patterns.md`
  （SPEC-034KLgfJ0Gka7fkRKHpfSH — R12 旧既定値記録・
  migration seed・非 containment 表現）
- `docs/discovery/opt-in-exec-deny-patterns.md`
  （DISC-Fad7xt2qdtfpcCHhjMhkVy — 旧 builtin regex の verbatim
  復元 ground truth）
- `docs/test-specs/opt-in-exec-deny-patterns.md`
  （TSPEC-034KLXPTSxUL1oOQI06HM3 — docs contract Rule）

## Dependencies

- `docs/tasks/add-core-security-policy-and-schema-wiring.md`
  （T1） — `security` 節の schema・match 規約・error 分類が
  確定していること。T2/T3 とは並列実行可能（product code を
  共有しない）。

## Affected Files

- `docs/guides/command.md`（modify）
- `docs/guides/configuration.md`（modify）
- `crates/cheaper-cli/resources/policy.yaml`（modify）

## Done When

- [x] `command.md` の stale deny 記述（旧 mandatory deny list・
      native magic）が `security.execDenyPatterns` の意味論・
      match 規約・旧既定値 verbatim 一覧・migration seed 例
      （verbatim 1 要素版 + 分類別分解）へ書き換わり、shell/
      shim 名が旧 deny 一覧に含まれない旨が migration 注記に
      ある
- [x] `--grant` bullet の deny 言及が opt-in 前提（merged
      re-resolve の `InvalidRuntimePolicy`・`details.layer`
      無しの accepted gap）に restate されている
- [x] `command.md` に `.*` deny-all 注意・`\` 区切り正規化の
      false-positive・blast radius の新規記述が存在する
- [x] `configuration.md` が `security` 節 yaml + `###
      security.execDenyPatterns` 小節（既定・等価形・trust
      boundary・error code・merge replace）を持つ
- [x] `resources/policy.yaml` が `security:` 節
      （`execDenyPatterns: []` + comment）を持つ
- [x] 表現が「deterministic executable deny-list」で一貫し、
      「保護」「sandbox」「containment」表現を謳わない
      （非 containment は正直に明記）

## Out of Scope

- product code（T1-T3）
- 文書全体の正直性 sweep・「boundary/sandbox/protection」表現の
  全量訂正（issue #169 の scope）
- `process.tasks`・glob 記法・CLI/env 供給経路の記述（本
  issue の scope 外機能）

## Verification

- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で変更 doc の error 0
- [x] doc review — design Docs 更新契約の各項目（旧既定値・
      migration seed・match 規約・trust boundary・非
      containment・3 補足）が記述されていること

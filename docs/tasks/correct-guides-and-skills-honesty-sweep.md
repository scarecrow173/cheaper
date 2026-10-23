---
id: "TASK-034L7LD2Gc1Num3wT45UMY"
type: "task"
status: "todo"
title: "documentation honesty sweep — guides + skills の in-place 訂正 (G-1〜G-8・off-list 2 件)"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  implemented-by: []
  depends-on:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  blocks:
    - "docs/tasks/verify-documentation-honesty-sweep.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/documentation-honesty-sweep-and-migration-guide.md"
  references:
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "docs/guides/command.md"
    - "docs/guides/managed-process.md"
    - "docs/guides/workers.md"
    - "docs/guides/usage.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep — guides + skills の in-place 訂正 (G-1〜G-8・off-list 2 件)

## Work

plan `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
(PLAN-034L7LkYl6qzs5eZ9mEJZL)の T1。design
(DESIGN-034Lrgl6KUIjrLaU6BBL2U)§(a) canonical disclaimer 配置表・
§(e) guides 訂正文 shape(G-1〜G-9)・§(d-2) off-list 表に従い、
利用者向け単一の真実である guides・配布 skill を in-place 訂正する
(refine 注記節は設けない — guide・skill 規約)。design の旧/新
shape を転記するだけで文言を再解釈しない。deny-list 命名規約
(ADR D2 —「deterministic executable deny-list」・「保護」
「sandbox」「containment」「防御」と呼ばない)を全編集へ適用する。

### 1. `docs/guides/command.md`

- [ ] **G-1(:44)** — executable 受理形の訂正:「workspace 相対
      path は拒否」clause 除去(#163 以降誤り)・「native
      magic（MZ / ELF / Mach-O）を持つ executable のみ実行可能で、
      shell・script shim…は `process` baseline の deny で拒否」
      除去 → 3 形態受理(workspace 相対 `/`・`\` 含有は
      `resolve_workspace_path` で boundary 内へ解決して pin、
      bare name は ambient PATH 検索、absolute path は直接
      viability 判定 — `proc/executable.rs:108-124` 実測根拠)
      + PATH 解決記述は維持 + 末文「executable の内容・名は
      審査しない — spawn viability(realpath・is_file・win32
      `.exe`/`.com`・POSIX `X_OK`)のみ判定。shebang script・
      任意 interpreter・shim 名の file も解決される。拒否する
      唯一の executable 審査は opt-in `security.execDenyPatterns`」。
      同 file :100 migration 注記との自相矛盾を解消
- [ ] **G-2(:133)** — 「全 args token には match 前の独立
      check として shell metachar 拒否…が適用され、token 長
      ≤4096・args 個数 ≤256。pattern 不一致・metachar 含有は
      `POLICY_VIOLATION`」→ 「args 個数 ≤256・各 token ≤4096
      は input schema 層の bound で超過は `INVALID_INPUT`。
      call-time の内容審査は行わない — metachar を含む token
      も pattern 適合なら executable の literal argv として
      渡る(spawn は shell を介さない)。pattern 不一致は
      `POLICY_VIOLATION`」
- [ ] **G-5(:141-153)** — trust/sandbox 限界節の canonical 化:
      節冒頭を「cheaper は OS sandbox ではない。canonical
      file tool(write/edit/ls/read/find/grep)の workspace
      boundary(E1/E2)・`filesystem.denyPaths`・worker
      `writeScope` は cheaper 所有の tool surface で強制される
      が、`command` が spawn した子プロセスの fs 副作用は
      OS sandbox 無しでは強制・完全観測できない」へ揃え(既存
      :143「OS sandbox 無し」・:145 bullet と重複しないよう
      吸収・言い換え)+ shell-free/ENOEXEC 区別(canonical
      項目 3: spawn は shell を介さず argv を直接渡し
      metachar は literal argv — POSIX +X shebang 無しの
      ENOEXEC への libc `/bin/sh` retry は OS/libc 既定動作・
      glibc `execvpe` 系・musl は retry しない・保証対象外)を
      1 bullet として追加
- [ ] :46-100(execDenyPatterns 節・seed・migration 注記)・
      :119-123・:148・:63 は**編集しない**(G-9 変更不要一覧)

### 2. `docs/guides/managed-process.md`

- [ ] **G-3(:23)** — 「任意 command/argv の指定、shell、
      shell 演算子、script shim を拒否する。pipe、semicolon
      等は正規の引数内でも拒否」→ 「呼出し input は `task`
      のみ — model から任意 command/argv は指定できない。
      宣言した `command` 配列は内容審査なしに literal argv
      として spawn される — shell・script file・演算子を含む
      要素も executable の引数としてそのまま渡る(cheaper は
      shell を介さないため `|`・`;` は制御構文として解釈
      されない)。executable 名・argv の正当性は task 作者の
      責務」
- [ ] **G-4(:27)** — 「Windows の `.cmd`/`.bat`/`.ps1` shim
      は実行しない」「native PE/ELF/Mach-O executable のみ
      解決」→ 「executable 名・内容の審査は行わない — spawn
      viability(realpath・is_file・実行 bit)のみ判定。win32
      では CreateProcess が直接 spawn できる `.exe`/`.com`
      のみ解決(`.cmd`/`.bat`/`.ps1` は viability で
      `POLICY_VIOLATION` — 内容審査ではなく OS の制約)。
      pnpm 等は native executable と JS entrypoint の argv。
      unix では +X file を内容無審査で解決 — bare `node` も
      PATH 上の実 `node` へ。**POSIX で shebang を持たない
      +X file の ENOEXEC に libc(glibc `execvpe` 系)が
      `/bin/sh` で retry するのは OS/libc 既定動作であり
      cheaper の保証対象外(musl は retry しない残留差分
      あり)。** 見つからなければ `POLICY_VIOLATION`」
- [ ] **G-6(:47-49)** — caveat 節:「OS sandbox や tracer
      ではない」を canonical 項目 1・2 へ揃え(file tool の
      boundary/denyPaths/writeScope は tool surface で強制、
      `process` 子プロセスの fs 副作用は強制・完全観測
      できない旨)。:49 group escape 非対象・W1 limitation
      は保持。:27 への cross-reference(spawn viability と
      ENOEXEC の区別は executable 解決の節を参照)1 行を
      添えてよい
- [ ] :47 前半・:49 は**編集しない**(G-9 変更不要一覧)

### 3. `docs/guides/workers.md`

- [ ] **G-7(:138-139)** — 「runtime が強制的に禁止する」へ
      「(canonical file tool の write に限る — exec 子
      プロセスの fs 副作用には及ばない)」但書追加。
      :54-65 writeScope tri-state 記述は保持

### 4. `docs/guides/usage.md`

- [ ] **off-list :207** — 「schema・deny list・argv pattern
      の compile」へ `(opt-in — security.execDenyPatterns
      宣言時のみ)` 条件付記(G-9 optional 項目の採用分)

### 5. `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

- [ ] **G-8(:353)** — 「`add` runs the same validation as
      policy load (deny list, argv pattern compile, env/cwd
      rules)」→ 「(opt-in deny list — `security.execDenyPatterns`
      declared のみ, argv pattern compile, env/cwd rules)」形
      へ条件付記(front matter 無し — backlink 不要)

### 6. `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`

- [ ] **off-list :82-85** — 「Deny-list and argv-pattern
      checks are re-applied on the re-run」→ 「argv-pattern
      checks — and the opt-in deny-list when
      `security.execDenyPatterns` is declared — are
      re-applied」形へ条件付記(front matter 無し —
      backlink 不要)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (PLAN-034L7LkYl6qzs5eZ9mEJZL — T1)
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U — §(a)/(e)/(d-2) が
  AUTHORITATIVE: 各 file の対象行と新旧 shape)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT — R1-R4・AC1-AC3)
- `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH — D1 canonical 文言正本・D2
  deny-list 命名)
- `docs/test-specs/documentation-honesty-sweep-and-migration-guide.md`
  (TSPEC-034L7LgKjsa5dwb8PstQX9 — disclaimer 配置・stale
  absence-grep・命名規約 Rules)

## Dependencies

- なし(T2/T3/T4 と file set が disjoint — 並列可能。
  design の推奨順序では本 task が先頭)

## Affected Files

- `docs/guides/command.md`(modify — G-1・G-2・G-5)
- `docs/guides/managed-process.md`(modify — G-3・G-4・G-6)
- `docs/guides/workers.md`(modify — G-7)
- `docs/guides/usage.md`(modify — off-list :207)
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`(modify — G-8)
- `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`
  (modify — off-list :82-85)

## Done When

- [ ] `command.md` の :44 が 3 形態受理 + 内容審査なし +
      opt-in `execDenyPatterns` のみ拒否の記述になり、
      「native magic（MZ」・「workspace 相対 path は拒否」
      が残存しない
- [ ] `command.md` :133 が input schema `INVALID_INPUT`
      bound + literal argv の記述になり、「shell metachar
      拒否」が残存しない
- [ ] `command.md` trust/sandbox 限界節が canonical 項目
      1・2 を含み ENOEXEC 区別 bullet がある(`grep -n
      "ENOEXEC" docs/guides/command.md` hit)
- [ ] `managed-process.md` :23 が task 名のみ input + 宣言
      argv 内容審査なしの記述になり、:27 が viability 制約
      + ENOEXEC 注記を含み、「native PE/ELF/Mach-O」・
      「shell 演算子、script shim を拒否」が残存しない
      (`grep -n "ENOEXEC" docs/guides/managed-process.md` hit)
- [ ] `managed-process.md` caveat 節が canonical 項目 1・2
      へ揃い :49 W1 limitation が保持されている
- [ ] `workers.md:138-139`・`usage.md:207`・両 SKILL.md
      該当行に opt-in 条件付記がある
- [ ] G-9 変更不要一覧の行(`command.md:46-100/:119-123/
      :148/:63`・`managed-process.md:47/:49`)が差分に含まれ
      ない、または既存記述を壊さない均質化に留まっている
      (`git diff` で確認)
- [ ] deny-list 命名規約違反(「保護」「sandbox」
      「containment」「防御」での `execDenyPatterns` 呼称)が
      差分内にない
- [ ] `node scripts/check-doc-ids.mjs` exits 0

## Out of Scope

- 他の 34 file(T2/T3/T4 の責務)・migration guide 本文(T4)
- ADR/design/spec/test-spec への refine 注記・backlink
  (T2/T3)
- code・fixture・test の変更(spec R8 — 「doc が正しく実装が
  誤り」を発見した場合は訂正せず issue へ follow-up 記録、
  design §Data and Control Flow)
- 一覧外の新規 stale 発見への対応(発見次第の継続 sweep —
  ただし同一 file 内の明白な同型残留は注記で補ってよい)
- `parent-agent-integration.md`・`configuration.md`(整合済み
  — 編集しない)

## Verification

- [ ] §1-§6 の全 checkbox を編集し `git diff` が記述/comment
      のみの差分であること
- [ ] `grep -n "OS sandbox" docs/guides/command.md
      docs/guides/managed-process.md` が両 file hit・
      `grep -n "ENOEXEC"` が両 file hit(実測を記録)
- [ ] absence-grep: `command.md` の「native magic（MZ」
      「shell metachar 拒否」「workspace 相対 path は拒否」
      `managed-process.md` の「native PE/ELF/Mach-O」が
      0 hit(実測を記録)
- [ ] `node scripts/check-doc-ids.mjs` exits 0(実測を記録)

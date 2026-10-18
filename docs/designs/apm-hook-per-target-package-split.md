---
id: "DESIGN-kXMpXYHxw0mt0AE0SeAHTM"
type: "design"
status: "approved"
title: "APM hook 宣言の per-target サブパッケージ分離 設計"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
    - "docs/plans/apm-hook-per-target-package-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm-hook-claude/.apm/hooks/hooks.json"
    - "packages/apm/apm.yml"
    - "scripts/check-apm-payload.mjs"
    - "scripts/check-doc-ids.mjs"
    - "scripts/check-version.mjs"
    - "scripts/set-version.mjs"
    - "scripts/test/lib/apm-fixture.mjs"
    - "scripts/test/apm-install-targets.test.mjs"
    - "scripts/test/apm-payload.test.mjs"
    - "scripts/test/apm-package-layout.test.mjs"
    - "scripts/test/launcher-passthrough.test.mjs"
    - "scripts/test/devin-hooks-install.test.mjs"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/distribution.md"
    - "AGENTS.md"
    - "CONTRIBUTING.md"
  defers: []
  deferred-by: []
---

# 設計: APM hook 宣言の per-target サブパッケージ分離

## 変更点

### 1. package 構造

```
packages/
  apm/                              # 親(skills のみ)
    apm.yml                         # + dependencies.apm (5 sibling path deps)
    .apm/skills/...                 # 不変
    .apm/hooks/                     # 削除
  apm-hook-windsurf/
    apm.yml                         # name: cheaper-hook-windsurf / target: windsurf
    .apm/hooks/hooks.json           # pre_read_code / pre_run_command
    .apm/hooks/cheaper.sh           # launcher コピー
    .apm/hooks/cheaper.ps1
  apm-hook-cursor/
    apm.yml                         # name: cheaper-hook-cursor / target: cursor
    .apm/hooks/hooks.json           # beforeReadFile / beforeShellExecution
    .apm/hooks/cheaper.sh
    .apm/hooks/cheaper.ps1
  apm-hook-copilot/
    apm.yml                         # name: cheaper-hook-copilot / target: copilot
    .apm/hooks/hooks.json           # preToolUse (flat, matcher 付き)
    .apm/hooks/cheaper.sh
    .apm/hooks/cheaper.ps1
  apm-hook-claude/
    apm.yml                         # name: cheaper-hook-claude / target: claude
    .apm/hooks/hooks.json           # PreToolUse (nested matcher/hooks)
    .apm/hooks/cheaper.sh
    .apm/hooks/cheaper.ps1
  apm-hook-codex/
    apm.yml                         # name: cheaper-hook-codex / target: codex
    .apm/hooks/hooks.json           # PreToolUse (nested matcher/hooks)
    .apm/hooks/cheaper.sh
    .apm/hooks/cheaper.ps1
```

- 各 sibling manifest の `name:` は親 `cheaper` と重複させない一意名
  (`cheaper-hook-*`)にする — consumer lockfile の `name` metadata で
  同名が並ぶのを避ける。
- `version:` は親と同じ canonical version を宣言し、
  `check-version.mjs` / `set-version.mjs` の対象 manifest に追加して
  release 時の skew を防ぐ(version 省略は APM manifest 上の非必須
  検証が不確かなため採用しない)。

各 hooks.json の entry 内容(command/powershell/matcher/show_output)は
現行 `packages/apm/.apm/hooks/hooks.json` の該当 key を **そのまま**
移す — `cheaper hook <event> --target <t>` の呼び出し契約は不変。

### 2. 親 `packages/apm/apm.yml`

```yaml
name: cheaper
version: "1.0.0"
description: ...   # "hooks" 言及を hook packages へ移した旨に更新
author: scarecrow173
dependencies:
  apm:
    - path: ../apm-hook-windsurf
      targets: [windsurf]
    - path: ../apm-hook-cursor
      targets: [cursor]
    - path: ../apm-hook-copilot
      targets: [copilot]
    - path: ../apm-hook-claude
      targets: [claude]
    - path: ../apm-hook-codex
      targets: [codex]
```

- remote(virtual subpath)経路では `../apm-hook-*` が同一 repo root 内の
  sibling path + 同一 ref に解決される(APM の explicit 規約、
  git-daemon fixture で実証)。consumer の dep 宣言
  (`scarecrow173/cheaper/packages/apm`)は不変で、hook は transitive に
  install される。
- 各 hook package の manifest にも `target:`/`targets:` を宣言する
  (直接 install / consumer による dep-level narrowing 非依存の防御)。
- **依存宣言の syntax**: `dependencies` の group key は `apm` 固定
  (`dependencies.apm: [...]`)。target 名を key にする記法は存在しない
  (fixture で誤記法 → transitive 未解決を確認済み)。

### 2.4 APM version と機構の前提

- transitive `path:` dep(remote 同一 repo/ref、local は `source_path`
  anchor・#857)と per-dep `targets:` は APM ≥0.22 で提供。
  repo が pin する **0.28.0** と issue 報告の **0.30.0** の双方で
  local path dep fixture を実動作確認済み(transitive 解決・target
  routing・lockfile 5 entry)。docs には「apm ≥0.28 推奨」と記載し、
  runtime gate は設けない(非要件)。

### 2.5 テスト追従(現行 contract を pin している箇所)

`mise run check` が `node --test scripts/test/` を含むため、旧 layout を
pin する全 test を同一 PR で直す(AC4)。

- `scripts/test/lib/apm-fixture.mjs` — `makeConsumerLayout` が
  `packages/apm` 単体を `pkg/` へコピーして install する。transitive
  `path: ../apm-hook-*` を解決するため `packages/` 配下の 5 package
  全体を `pkgs/<name>/` としてコピーする構造へ変更する。consumer の
  dep は `path: ../pkgs/apm`。`BUNDLE_DIRS` の dep dir 名は `pkg` →
  各 `apm-hook-*` package 名になる(target → hook package の対応表を
  追加)。local path dep の transitive 解決は 0.28.0/0.30.0 で実証済み。
- `scripts/test/apm-install-targets.test.mjs` — 現行は「全 target に全
  event が verbatim 登録される」ことを assert している(dead entry を
  contract 化した旧 ADR-034r05iHrZUE6W9RDF0V73 時代の pin)。
  「target 毎に native event のみ登録され、他 harness の event key が
  残らない」新 contract へ書き換える(AC1)。claude の copilot 由来
  重複 group(`view|bash|powershell|shell` matcher)不在の assert に
  置き換える。windsurf/cursor/codex では foreign event key の不在を
  明示 assert する。
- `scripts/test/apm-payload.test.mjs` — fixture が
  `packages/apm/.apm/hooks` を `cpSync` している(dir 削除で ENOENT)。
  fixture source を `packages/apm-hook-windsurf/.apm/hooks` 等へ変更し、
  git mode 確認の path も追従させる。
- `scripts/test/apm-package-layout.test.mjs` — `packages/apm` に
  `.apm/hooks` が**存在する**ことを assert している。反転して
  「`packages/apm` に `.apm/hooks` が無く、5 sibling package が
  `.apm/hooks` を持つ」assert に変更。
- `scripts/test/launcher-passthrough.test.mjs` — 削除される
  `packages/apm/.apm/hooks` を `cwd` にしている。
  `packages/apm-hook-windsurf/.apm/hooks` へ変更。
- `scripts/test/devin-hooks-install.test.mjs` —
  `packages/apm/.apm/hooks/hooks.json` を読んで PreToolUse matcher を
  pin している(module load で ENOENT)。参照先を
  `packages/apm-hook-claude/.apm/hooks/hooks.json` へ変更。
- `scripts/check-doc-ids.mjs` — `packages/apm/.apm/**/*.md` のみ走査。
  `packages/*/.apm/**/*.md` に広げ、sibling package 内の `.md` も
  doc-id contract の対象にする。
- 歴史文書(`docs/specs/`・`docs/test-specs/` の過去 ADR/spec 内の
  `packages/apm/.apm/hooks/hooks.json` 言及)は当時の記録として残す。
  現行 contract を述べる test-spec の path 言及のみ整合させる。

### 3. `scripts/check-apm-payload.mjs` の追従

- hardcoded list ではなく `packages/*/.apm/hooks/` を glob 走査する —
  将来の hook package 追加が list 更新漏れで検査を免れるのを防ぐ。
- 現行の検査(必須 nonempty asset、extension allowlist、PE/ELF sniffing、
  BOM/CRLF/shebang/LF、git mode 100755、hooks.json↔launcher 整合)を
  `.apm/hooks/` を持つ各 package へ適用する。
- `packages/apm` については `.apm/hooks/` が存在しないこと
  (hooks.json が消えたこと)を検査し、hooks 専用 dir 化の regress を防ぐ。
- **launcher copy 間の byte 一致**: 5 package の `cheaper.sh` 同士・
  `cheaper.ps1` 同士が byte-identical であることを assert(片方だけ
  修正される drift を機械的に防ぐ)。
- **wiring 検査**: 親 `packages/apm/apm.yml` の `dependencies.apm` が
  5 sibling 全てを宣言し、各 entry の `targets:` が対象 package の
  manifest `target:`/`targets:` と整合することを検査(dep-edge と
  pkg-level の二重制限が drift すると silent non-delivery になるため)。

### 4. 文書更新

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  - 「Locating the CLI」の per-target bundle root 表を
    `.windsurf/hooks/apm-hook-windsurf/` 等の新 dep name に更新。
    hook bundle が per-target package に分かれた旨を追記。
  - `apm update` を案内している段落を修正する — `apm update` では
    orphan が残るため、upgrade 手順は uninstall → install(§5)。
  - 最小 APM version(≥0.28 推奨)を記載。
- `docs/guides/distribution.md`:
  - hook bundle の deploy 先(`<hooks_root>/<dep-name>/`)が
    `apm-hook-*` になる記述へ更新。
  - `targets: [agent-skills]` narrowing が transitive hook dep へ伝播
    しない breaking change(§5.5)と回避策を追記。
  - `packages/apm` が self-contained でなくなった旨を追記 —
    `path:` dep で `packages/apm` dir 単体をコピーした install・
    `apm pack`/`publish` による registry 配布は非対応(git ref 経由
    のみ)。registry 公開は従来から deferred 方針と整合。
- `AGENTS.md` / `CONTRIBUTING.md`: 「consumer へ配布する APM package
  は `packages/apm/`(skills・`hooks.json`・thin launcher)」の記述を
  「skills は `packages/apm`、hook 宣言は per-target sibling package
  `packages/apm-hook-*`」へ更新。
- 旧 contract を記述した既存文書への関連付け:
  `cheaper-usage-skill-hook-path-examples`(SPEC-034DjvgZNwy9e9aLXb2alN)、
  `document-apm-dep-name-deploy-rule-in-cheaper-usage-skill`
  (ADR-034Dkj1V1OguoUSXpwCE3e)、
  `cross-agent-native-routing-adapters-over-apm-distribution`
  (SPEC-034rzTJydXfLvQhTIqiDqW)、
  `cross-agent-distribution-verification`
  (SPEC-034r05iHK8bA7gky93Dal0)、
  `separate-dev-apm-project-from-distributable-package`
  (SPEC-034r05iHK8bA7go4BQtGNS)、
  `thin-path-launchers-and-slim-apm-payload-contract`
  (TSPEC-034DmUtUjWCThCnJulok0c)、
  `cross-agent-hook-distribution-and-contract-matrix`
  (DISC-034r05iHy2gdnpdai9KGpW)
  — いずれも「単一 package が全 target へ verbatim 登録」を前提とした
  記述を含む。spec/design から `related` で相互 link し、本文冒頭に
  「hook 配布 layout は SPEC-2w4XxXeDF0glTfgrBVJnWb で変更済み」の
  note を1行足す(本文書き換えは行わない — 当時の決定記録を保持)。
- 本 repo 内の `.agents/` 配下は gitignore(consumer への配布物は
  `packages/apm/` のみ)なので対象外。

### 5. 移行(既存 consumer の upgrade)

`apm update` では旧 `apm` package が配布した merged hook entry が
除去されない(APM は hook file を失った package の entry を掃除しない —
fixture で実証)。orphan entry は消えた bundle script を参照し、
windsurf では `exit $LASTEXITCODE` で hook failure 化し得る。
旧 bundle script 自体は掃除されるため二重起動にはならないが、dead
path 参照と空の event 登録が残る。

対応: consumer 向けに以下を SKILL.md / distribution guide / issue
コメントで案内する。

```
apm uninstall <dep as declared>   # 例: scarecrow173/cheaper/packages/apm
                                  # global install は apm uninstall -g <dep>
apm install                       # apm.yml の dep を再 install
```

- `<dep>` は consumer が `apm.yml` の `dependencies.apm` に記載した
  もの(github virtual path でも local `path:` でも、その名を使う)。
- **旧 ref pin の注意**: `@<old-sha>`/`@<old-tag>` で pin している
  consumer は、先に dep の ref を post-split commit へ更新しないと
  reinstall で旧 layout が再配布される。
- **uninstall が nonzero exit を返す場合**: managed file が
  edit/symlink 等で changed と判定されると hook cleanup が残る。
  その場合は生成 config(`.windsurf/hooks.json` 等)内で
  `command`/`_apm_source` が旧 dep を指す entry を手動で除去する。
- uninstall が marker(`_apm_source`)で merged entry を完全除去する
  ことは fixture で実証済み。fresh install は clean。

### 5.5 既知の breaking change — dep-edge `targets:` は transitive に伝播しない

APM は consumer が dep-edge に付けた `targets:` を transitive dep の
reach に**伝播しない**(0.30.0 fixture で実証: 親 dep を
`targets: [agent-skills]` に narrow しても transitive hook package は
active harness target へ deploy される)。

影響: `targets: [agent-skills]` で cheaper を narrow した consumer で
harness target も active な場合、従来 hook が deploy されなかったのに
deploy されるようになる。典型的な devin lifecycle recipe 利用者
(agent-skills のみ active)は影響しない。回避策は「install 時の active
target を agent-skills のみに絞る」または「policy の
`dependencies.deny` で `apm-hook-*` を拒否」。

この trade-off は「consumer が dep 宣言を変えずに正しい target routing
を得る」利点と引き換えに受容する(分離しない限り casing warning と
foreign event 登録は解消できない)。

### 6. 残存警告と upstream 案件

- cursor target への install では `beforeReadFile`/`beforeShellExecution`
  が camelCase のため APM の「expected PascalCase」heuristic が警告を出す。
  これらは cursor の正しい native event 名であり cheaper 側では除去不能。
  upstream(microsoft/apm)への issue 起票候補として記録する。
- merged entry の orphan cleanup 欠如も同様に upstream 課題。

## 非変更

- `cheaper hook <event> --target <t>` の runtime 挙動・payload 契約。
- `hook-config`(devin native 登録)経路。
- skills の内容・配布先(全 target)。
- root `apm.yml`(dev project、自 package 非依存)。

## 検証

- fixture consumer に local path dep で install → 各 target config の
  event key を検査(実施済み — native event のみ、警告は cursor のみ)。
- remote 相当(git clone 経路)でも transitive sibling `path:` 解決を
  実証済み。
- `mise run check`(check-apm-payload を含む全段)。

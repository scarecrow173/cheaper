---
id: "SPEC-2w4XxXeDF0glTfgrBVJnWb"
type: "spec"
status: "proposed"
title: "APM hook 宣言を per-target サブパッケージへ分離"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/discovery/apm-hook-per-target-package-split.md"
    - "https://github.com/scarecrow173/cheaper/issues/118"
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
  related: []
  refines:
    - "docs/discovery/apm-hook-per-target-package-split.md"
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/apm-hook-per-target-package-split.md"
    - "docs/adr/apm-hook-per-target-package-split.md"
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

# APM hook 宣言の per-target サブパッケージ分離

## 要件

- R1: `packages/apm` の `.apm/hooks/` 配下の単一 `hooks.json` を廃止し、
  hook 宣言を harness 別の sibling package に分離する。分離単位は
  「同一の native event 名 + entry shape を共有する target 群」:
  - `packages/apm-hook-windsurf`(`target: windsurf`):
    `pre_read_code` / `pre_run_command`(現行 entry 内容を維持)
  - `packages/apm-hook-cursor`(`target: cursor`):
    `beforeReadFile` / `beforeShellExecution`(同)
  - `packages/apm-hook-copilot`(`target: copilot`):
    `preToolUse` flat entry(同)
  - `packages/apm-hook-claude`(`target: claude`):
    `PreToolUse` nested entry(同)
  - `packages/apm-hook-codex`(`target: codex`):
    `PreToolUse` nested entry(同 — claude と同一内容)
- R2: 各 hook package は `.apm/hooks/hooks.json` と launcher
  `cheaper.sh` / `cheaper.ps1` を自 package 内に持つ(hook command の
  `./cheaper.sh` 参照が bundle deploy の trigger である APM の規約上、
  script は package 毎に必要)。
- R3: 親 `packages/apm` の `apm.yml` に `dependencies.apm` 配下
  (group key は `apm` 固定)として 5 つの sibling `path:` dep
  (`../apm-hook-*`)を、対応する per-dep `targets:` 付きで宣言する。
  remote install では同一 repo・同一 ref に解決され、local `path:` dep
  でも transitive 解決される(0.28.0/0.30.0 で実証済み)。親 package 側に
  は `target:` 制限を付けない(skills は全 target へ届く必要がある)。
- R4: `packages/apm/.apm/hooks/` 自体を削除する。skills は従来どおり全
  target に配布される。
- R5: 検証・テスト群を新 layout に追従させる:
  - `scripts/check-apm-payload.mjs` — `packages/*/.apm/hooks/` を glob
    走査し、launcher 契約(BOM/CRLF/shebang/git mode)・
    hooks.json↔launcher 整合・4 launcher copy 間の byte 一致を検査。
    親 `apm.yml` が 5 sibling を正しい `targets:` で宣言している
    wiring 検査も追加する。
  - `scripts/check-doc-ids.mjs` — 走査対象を `packages/apm/.apm/**/*.md`
    から `packages/*/.apm/**/*.md` へ拡張。
  - `scripts/test/lib/apm-fixture.mjs` — `makeConsumerLayout` が
    `packages/` 以下の 5 package 全体をコピーする構造へ変更。
    `BUNDLE_DIRS` の dep dir 名を `pkg` → `apm-hook-*` へ変更。
  - `scripts/test/apm-install-targets.test.mjs` — 「全 target に全 event」
    の pin を「各 target に native event のみ」へ書き換え(AC1 を検証)。
  - `scripts/test/apm-payload.test.mjs` — fixture の hooks source を
    `packages/apm-hook-*/.apm/hooks` 群へ変更。
  - `scripts/test/apm-package-layout.test.mjs` — `packages/apm` に
    `.apm/hooks` が**無い**こと・5 sibling package が存在することを
    assert する形へ反転。
  - `scripts/test/launcher-passthrough.test.mjs` — `cwd` を
    `packages/apm-hook-windsurf/.apm/hooks` 等の残存 dir へ変更。
  - `scripts/test/devin-hooks-install.test.mjs` — hooks.json 参照先を
    `packages/apm-hook-claude/.apm/hooks/hooks.json` へ変更。
  - `scripts/check-version.mjs` / `scripts/set-version.mjs` — 新 manifest
    を version contract の対象に追加(sibling manifest の `version:` は
    canonical と一致させる)。
- R6: 文書を更新する:
  - `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の hook bundle
    path 表(`<hooks_root>/<dep-name>/` が `apm-hook-*` 名になる)と、
    `apm update` を推奨している記述(orphan を残すため uninstall→install
    案内へ修正)
  - `docs/guides/distribution.md` の hook bundle 記述・`targets:
    [agent-skills]` recipe の挙動変化(R9)・local 部分コピーや
    `apm pack`/`publish` が非対応になった旨
  - `AGENTS.md` / `CONTRIBUTING.md` の `packages/apm` 説明
    (skills+hooks 単一 package → skills + 5 sibling hook package)
  - 旧 contract を記述した既存文書への superseded-by/注記(relations
    で相互 link: `keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets`
    ADR は supersedes 済み。その他 approved/implemented の spec・
    test-spec・discovery は「現行 contract でなくなった」注記または
    related link で辿れるようにする)
- R7: 移行手順を文書化する。旧 `apm` package が配布した merged hook
  entry は package が hook file を失っても APM が掃除しないため、
  `apm update` だけでは dead-path 参照 + 二重登録が残る。consumer には
  `apm uninstall <dep>` → `apm install` の手順を案内する
  (SKILL.md / distribution guide / issue コメント)。あわせて次を記載:
  - `<dep>` は consumer が `apm.yml` に書いた dep 名
    (`scarecrow173/cheaper/packages/apm` や `path:` dep 名)
  - `-g` (global) install 済み consumer は `apm uninstall -g` が必要
  - 旧 ref pin(`@<sha>`/`@<tag>`)の consumer は先に ref を post-split
    commit へ更新する必要がある
  - managed file が edit/symlink 等で「changed」と判定されると uninstall
    の hook cleanup が nonzero exit で残留しうる — その場合は生成
    config 内の `_apm_source`/`_local/apm` 由来 entry を手動除去する
- R8: sibling manifest の規約。各 `packages/apm-hook-*/apm.yml` は一意な
  `name:`(`cheaper-hook-windsurf` 等。親 `name: cheaper` と重複させない)
  と canonical version(親と同一、`check-version.mjs` で検査)を持つ。
- R9: 既知の breaking change として記録する — consumer が dep-edge
  `targets:`(例: `[agent-skills]`)で cheaper dep を narrow していても、
  APM は祖先 edge の `targets:` を transitive dep へ伝播しない
  (0.30.0 で実証)。multi-harness active な consumer が narrowing しても
  transitive hook package は deploy される。現行の `targets: [agent-skills]`
  recipe(devin lifecycle 連携)を使う consumer で harness も active な場合、
  従来 hook が deploy されなかったのに deploy されるようになる。
  回避策は「`apm install`/`update` の active target を agent-skills のみに
  絞る」か「policy の `dependencies.deny` で `apm-hook-*` を拒否」。
  distribution.md と移行 note に明記する。

## 非要件

- APM 本体の修正(cursor target に対する camelCase native event の
  false-positive 警告、hook file を失った package の merged entry orphan
  cleanup 欠如) — upstream 案件であり、必要に応じて microsoft/apm へ
  起票する。
- devin hook 登録(`hook-config` 所有)の変更 — 現行どおり。
- hook の runtime 挙動(`cheaper hook <event> --target <t>`)自体の変更 —
  hooks.json が emit する command/option は従来どおり。
- `agent-skills` 等の非 hook target への挙動変更 — hook package は
  pkg-level target により何も deploy しない(現状より正確)。
- gemini / kiro / antigravity / vscode 等、現行5 target 以外への hook
  deploy — 現状も matcher 不一致等で dead entry 化しており、split 後に
  届かなくなるのは dead config の除去であって機能損失ではない
  (意図的な非対応として記録。将来必要になった場合は hook package を
  追加する)。
- `apm pack` / `apm publish` / vendored 部分コピーでの配布 — transitive
  `path:` dep が repo 階層を要求するため非対応。git ref 経由の配布のみを
  サポートする(従来も registry 公開は deferred)。
- 最小 APM version の強制 — transitive `path:` dep と per-dep `targets:`
  は APM ≥0.22 で利用可能。repo が pin する 0.28.0 と issue 報告の 0.30.0
  で実動作を確認済み。docs には「apm ≥0.28 推奨(0.22+ の機能を利用)」と
  記載するが、runtime 側で version gate は設けない。

## Acceptance Criteria

- AC1: 新 layout を install した consumer の `.windsurf/hooks.json` は
  `pre_read_code`/`pre_run_command` のみ、`.cursor/hooks.json` は
  `beforeReadFile`/`beforeShellExecution` のみ、`.claude/settings.json`
  と `.codex/hooks.json` は `PreToolUse` のみ、`.github/hooks/` の
  copilot hook file は `preToolUse` のみを含む(他 harness 由来の
  event key が残らない)。claude `PreToolUse` の cheaper entry は 1 件。
- AC2: install/update 時の casing 警告が claude/codex/windsurf/copilot で
  出ない。cursor は `beforeReadFile`/`beforeShellExecution` が cursor
  native event 名であるため APM の heuristic 警告が残る(upstream 既知
  問題として記録する)。
- AC3: 各 hook command が `<hooks_root>/apm-hook-*/cheaper.sh|.ps1` を指し、
  対応 script が同 dir に deploy される。
- AC4: `mise run check` が pass する(check-apm-payload・check-version・
  `node --test scripts/test/` 全件を含む)。
- AC5: 旧 layout で install 済み consumer が `apm uninstall` →
  `apm install` で旧 marker entry が完全除去される(APM 実動作として
  確認済みの手順を文書で保証)。
- AC6: consumer が dep-edge `targets: [agent-skills]` で narrow した場合
  transitive hook dep へ伝播しない挙動・回避策が distribution.md に記載
  されている。

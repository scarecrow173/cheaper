---
id: "TASK-034LSvSfFCNY9T3y1exy8S"
type: "task"
status: "done"
title: "Update Node boundary docs and stale script comments"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
  changes:
    added: []
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "scripts/check-lockfile.mjs"
      - type: "file"
        path: "scripts/lib/release-artifacts.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  implemented-by: []
  depends-on:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  blocks:
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  defers: []
  deferred-by: []
---

# Update Node boundary docs and stale script comments

## Work

issue #182 Option A(Node 維持)の運用固定として、AGENTS.md の Node 段落を
boundary contract へ揃え、発見済みの stale comment 2 件を実態へ整合させる。
実行 code には一切触れない(doc/comment のみ)。

1. `AGENTS.md` の Node 段落(現 81-83 行目)を下記 replacement 文案で
   置き換える。文案は design §2 の境界 policy の要約であり、転記のみ許可
   (再解釈・文言の自前編集をしない):

   ```markdown
   Node.js は pinned mise dev-tool（`mise.toml` `[tools] node = "26"` +
   `mise.lock`）として repo tooling のためだけに toolchain に残る —
   `scripts/`・`benchmarks/`・`crates/cheaper-tools/gen-glob-tables.cjs`
   （glob table codegen の regex oracle）・deploy 済み doc-suite skills
   （`.agents/skills/`・`.claude/skills/`、upstream 所有）。product・
   distribution・hook 実行経路に Node 依存はない（runtime は Rust
   workspace = `crates/`、agent runtime は Rig）。

   新規の Node 結合は repo tooling 領域に限る（product path・配布
   payload・CI の直接 node 呼出しは不可。`crates/*/tests` からの新規
   ambient `node` 結合は原則不可 — 例外条件は design §2.2/§3 参照）。
   既存の ambient 結合は 3 site 残っており（design §3 参照）、
   `mise exec` を介さない裸 `cargo test` は node 不在環境で
   fail/skip し得る。`mise.toml` の `node` 行除去は required
   entrypoint 全 node-free + design §1 維持 entry の移行完了まで不可。
   領域別 policy・非退行契約・revisit trigger の観測点は
   `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`、
   全使用箇所の inventory は
   `docs/discovery/node-dev-tooling-inventory-issue-182.md` を参照
   （issue #182、ADR-034LRjX5iYZHrubmKgKAL1）。
   ```

   変更理由: 現行段落は「repo tooling(`scripts/`・`benchmarks/`・
   doc-suite skills)のみ・product path 非依存」の骨格を持つが、
   `gen-glob-tables.cjs`(crate 非 src の manual codegen oracle)が
   列挙から抜けており、領域別 policy(§2.2)・ambient 結合禁止(§3.3)・
   除去 gate・canonical 参照への pointer が無い。contributor が最初に
   見る宣言として boundary contract を要約する。

   併せて `AGENTS.md:27` の toolchain 列挙から `python` を除く
   (現行「node・rust・python・apm・prek・git-lfs 等」→
   「node・rust・apm・prek・git-lfs 等」) — `mise.toml:19-21` で
   python は toolchain に含めないと明記済みの stale。

2. `scripts/check-lockfile.mjs:4-6` の header comment を修正。
   「check pipeline の最後尾に置き」は stale(実際は mise.toml:74 で
   10 step 中 9 番目、最後尾は `check-apm-payload.mjs`)。drift 検出の
   根拠(先行 build/test が lockfile を再生成した場合の捕捉)は維持し、
   位置 claim のみを実態へ直す — 例:
   「check pipeline で check-apm-payload の直前に置き」(序数は再び
   stale 化し得るため相対位置のみを書く)。

3. `scripts/lib/release-artifacts.mjs:1-2` の header comment を修正。
   「shared by check-version.mjs / set-version.mjs /
   check-apm-payload.mjs」は stale(`check-apm-payload.mjs` は node
   builtin のみを import し同 lib を使わない — `grep -rn
   "release-artifacts" scripts/*.mjs` で importer =
   check-version/set-version のみを確認済み)。「shared by
   check-version.mjs / set-version.mjs」へ直す。

4. `.agents/skills/cheaper-usage/SKILL.md:43` の旧
   `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` 参照について、
   下記の実測観測を本 task の Notes へ記録する(deploy 物の編集は
   しない — upstream 所有で `apm install` 再実行でも解消しないことを
   2026-10-25 に確認済み: deploy file sha256 `b0edb6…` が
   `apm.lock.yaml` `local_deployed_file_hashes` の記録値と一致、
   repo source `packages/apm/.apm/skills/cheaper-usage/SKILL.md` は
   既に現行 hook path。lock-pinned な upstream stale copy であり
   実 node 依存ではない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md (T1 slice)
- docs/designs/node-dev-tooling-boundary-contract-issue-182.md (§2 境界 policy・§5 F6 — comment 修正を本 PR に吸収する plan deviation 参照)

## Dependencies

- なし(plan のみ — T2 とは独立、並列可)

## Affected Files

- `AGENTS.md`(Node 段落 1 箇所 + :27 toolchain 列挙の `python` 除去)
- `scripts/check-lockfile.mjs`(header comment のみ、実行 code 非接触)
- `scripts/lib/release-artifacts.mjs`(header comment のみ、実行 code 非接触)
- 本 task doc(Notes への drift 観測記録)

対象外: `.agents/skills/`・`.claude/skills/`(upstream deploy 物)、
`packages/**`、product 実行経路の全 file。

## Done When

- [x] AGENTS.md の Node 段落が replacement 文案どおりに更新され、
      `gen-glob-tables.cjs`・design/discovery への参照・「新規 ambient
      結合は原則不可」「node 行除去不可」の記述を含む
      (AGENTS.md:81-100 — 文案を verbatim で転記)
- [x] AGENTS.md:27 の toolchain 列挙から `python` が除かれた
      (mise.toml:19-21 の python 非含有 note と整合 —
      「node・rust・apm・prek・git-lfs 等」へ修正、`grep "python"
      AGENTS.md` は file 全体で 0 件)
- [x] `scripts/check-lockfile.mjs` の header が「最後尾」を名乗らず、
      `check-apm-payload` 直前の実位置を述べ、drift 検出根拠が残る
      (:4-7「check pipeline で check-apm-payload の直前に置き」+
      先行 build/test の lockfile 再生成捕捉根拠を維持)
- [x] `scripts/lib/release-artifacts.mjs` の header が
      `check-version.mjs`・`set-version.mjs` のみを名指す(:1-2)
- [x] 実施時に `apm.lock.yaml` `local_deployed_file_hashes` と
      deployed file の sha256 を再確認し、観測が変化した場合のみ
      Notes を更新した(2026-10-25 再確認 — deployed file sha256
      `b0edb6bc…a74da2` が lock 記録値と一致し観測不変のため
      drift observation 本文は追記なし。再確認の証跡は
      Verification evidence に記録)
- [x] `git diff` が doc/comment のみで実行 code 差分を持たない
      (script 2 file の差分は全行 `//` comment、AGENTS.md は
      markdown 本文のみ)

## Verification

- [x] `grep -n "最後尾" scripts/check-lockfile.mjs` が出力 0 件
- [x] `grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs` が出力 0 件
- [x] `grep -n "gen-glob-tables.cjs" AGENTS.md` が match を返し(:83)、
      `grep -n "python" AGENTS.md` が toolchain 列挙(:27 付近)で
      hit しない(file 全体で 0 件)
- [x] `grep -rn "release-artifacts" scripts/check-version.mjs scripts/set-version.mjs` が両 file の import を返す(check-version.mjs:23・set-version.mjs:23 — 実 importer の実態照合)
- [x] `git diff -- scripts/check-lockfile.mjs scripts/lib/release-artifacts.mjs` の差分が comment 行のみ
- [x] `mise exec -- node scripts/check-doc-ids.mjs` exits 0(1211 markdown files scanned)
- [x] Manual: AGENTS.md 段落が boundary contract の要約として design §2
      と矛盾しない(目視 supplement — §2.1 pinned dev-tool/product
      node-free・§2.2 領域別表・§3 ambient 3 site/removal gate と
      照合済み)

## Notes

### Drift observation(2026-10-25 実測 — 実施時に確認・記録)

- `.agents/skills/cheaper-usage/SKILL.md:43` は旧
  `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` を参照するが、
  これは upstream `scarecrow173/apm-packages` 由来の deploy 物で、
  `apm.lock.yaml` `local_deployed_file_hashes` が pin する
  `sha256:b0edb6…` と file 内容が一致する lock-pinned stale copy。
- `mise exec -- apm install` 実行後も残存することを確認 — design §5
  F6 が想定した「`apm install` 再実行で解消」は成立しない。
  解消には upstream 側の SKILL.md 修正 + apm dep bump が必要。
- repo source `packages/apm/.apm/skills/cheaper-usage/SKILL.md` は
  現行 hook path(`.windsurf/hooks/apm-hook-windsurf/cheaper.sh`)を
  参照済みで正しい。drift は cosmetic であり実 node 依存ではない。
- 本 task では deploy 物を編集しない(upstream 所有)。観測は plan の
  Deferred Follow-up Tracking(F6 行)と本 Notes に記録する。

### Verification evidence(2026-10-25、Windows local、branch `chore/node-deps-inventory-issue-182`)

- Drift observation 再確認(上記観測は不変 — 追記不要と判断):
  - `sha256sum .agents/skills/cheaper-usage/SKILL.md` →
    `b0edb6bc2a1ba200358c23163ab6a772b5acd75290a03505e05ebd56dda74da2`。
    `apm.lock.yaml` `local_deployed_file_hashes` の
    `.agents/skills/cheaper-usage/SKILL.md: sha256:b0edb6bc…a74da2`
    と一致。
  - `.agents/skills/cheaper-usage/SKILL.md:43` は依然
    `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` を参照。
    repo source `packages/apm/.apm/skills/cheaper-usage/SKILL.md` は
    `:80` で現行 hook path
    `.windsurf/hooks/apm-hook-windsurf/cheaper.sh` を参照済み。
  - 結論: lock-pinned な upstream stale copy という観測は変化なし。
    deploy 物は編集しない。
- File 変更:
  - `AGENTS.md` — Node 段落を本 doc Work §1 の replacement 文案へ
    verbatim 置換(現 :81-100)+ :27 toolchain 列挙から `python` 除去。
  - `scripts/check-lockfile.mjs:4-7` — header の位置 claim を
    「check-apm-payload の直前」(相対位置、序数なし)へ修正。drift
    検出根拠行は維持。実行 code 非接触。
  - `scripts/lib/release-artifacts.mjs:1-2` — header の shared-by
    列挙から `check-apm-payload.mjs` を除去(実 importer =
    check-version/set-version のみ)。実行 code 非接触。
- 実態照合の裏付け:
  - `mise.toml:19-21` に python 非含有の明記あり(toolchain 列挙の
    `python` は stale であった)。
  - `mise.toml:63-76` の check pipeline は 10 step で
    `check-lockfile.mjs`(:74)は 9 番目、最後尾は
    `check-apm-payload.mjs`(:75)。
  - `grep -rn "release-artifacts" scripts/` → importer は
    `check-version.mjs:23`・`set-version.mjs:23` のみ。
- Verification 結果(全 pass):
  - `grep -n "最後尾" scripts/check-lockfile.mjs` → 0 件。
  - `grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs`
    → 0 件。
  - `grep -n "gen-glob-tables.cjs" AGENTS.md` → :83 で match。
    `grep -n "python" AGENTS.md` → file 全体で 0 件。
  - `grep -rn "release-artifacts" scripts/check-version.mjs
    scripts/set-version.mjs` → 両 file の import 行を返す。
  - `git diff -- scripts/check-lockfile.mjs
    scripts/lib/release-artifacts.mjs` → 差分は全行 `//` comment
    のみ(実行 code 差分なし)。
  - `mise exec -- node scripts/check-doc-ids.mjs` → exit 0
    (`doc id check OK: 1211 markdown files scanned`)。
  - Manual 照合: AGENTS.md 新段落は design §2.1(pinned mise
    dev-tool・product node-free)・§2.2 領域別表(src/payload/CI
    不可・tests 原則不可・codegen 条件付き可・deploy dir repo
    管理外・node 行 removal gate)・§3(ambient 3 site・裸
    `cargo test` の fail/skip)と矛盾なし。参照先
    `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`・
    `docs/discovery/node-dev-tooling-inventory-issue-182.md`・
    `crates/cheaper-tools/gen-glob-tables.cjs`・ADR-034LRjX5iYZHrubmKgKAL1
    の実在を確認済み。

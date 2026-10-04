# Version と release 手順

製品 version の **canonical source of truth は `packages/cli/package.json`**
（ADR-034r05iHrZUE6WgAk8Cpqq）。`cheaper --version`、配布 manifest `packages/apm/apm.yml`、
bundle 内 `version.json`、各 workspace package の `version` field はすべて
この値と一致しなければならない。

## Version 宣言の一覧

| ファイル | 役割 | 扱い |
| --- | --- | --- |
| `packages/cli/package.json` | canonical source | **ここだけが真の宣言** |
| `packages/core/package.json` | workspace package | canonical と一致必須 |
| `packages/runtime-pi/package.json` | workspace package | canonical と一致必須 |
| `packages/apm/apm.yml` | 配布 APM manifest | canonical と一致必須 |
| `packages/apm/.apm/hooks/cli/version.json` | bundled CLI の version | 生成物（`bundle-cli.mjs` が cli から派生） |
| `apm.yml`（root） | `cheaper-dev` dev project | **対象外** — 製品 version ではなく dev project 識別子。独立してよい |

## 不一致の検出

`node scripts/check-version.mjs` が全宣言を canonical と照合する。
`mise run check`（SPEC-034r05iHK8bA7gpcCJQya9 の canonical verification entrypoint）に
組み込まれているため、CI を含む通常の検証経路で自動的に検出される。
不一致があれば file 名と値を列挙して exit 1。

## Version bump

```bash
node scripts/set-version.mjs <X.Y.Z>     # 全宣言を一括更新
node scripts/bundle-cli.mjs              # version.json を再生成
mise run check                           # 全検証（version check 含む）
```

`set-version.mjs` は上表の宣言 file をすべて書き換え、続けて
`check-version.mjs` を実行する。`version.json` は生成物のため bump
直後は不一致として検出され、bundle 再生成を促される（正常な流れ）。

## Release 前の version / LFS 準備（#73-C まで併存）

公開には新しい `release-acquisition` workflow を **1 回 dispatch** する。
その1回で固定 SHA の3target SEA build → native smoke → `SHA256SUMS` →
APM archive → native APM smoke → GitHub Release 公開までを実行する。
本節は公開前の source/tag 準備であり、現在の Git-based APM 配布と canonical
payload check が要求する LFS round-trip は #73-C の移行完了まで残る。
新経路の `strict_freshness:false` は同runのfresh binaryをarchiveに入れるための設定であり、
Gitに追跡するpayloadや `check-apm-payload.mjs` の検証を廃止するものではない。

1. `node scripts/set-version.mjs <X.Y.Z>` で bump する。
2. `node scripts/bundle-cli.mjs` で `version.json` と bundled CLI を
   再生成する。
3. bump 変更を commit して branch を push する（tag はまだ打たない）。
4. **APM payload binary を再生成する。** GitHub Actions の
   `build-executables` workflow を**手動 dispatch** し、`ref` input に
   bump commit を含む branch（例: `main`）を指定する。3 target
   （Windows x64・Linux x64・Linux arm64）の executable が artifact
   として upload される。
   - release の build 結果と byte 一致する binary を payload に入れる
     唯一の正式経路。local で生成した binary は runner artifact と
     一致しないため使わない。
5. workflow artifact を download し、APM payload に配置する:

   ```bash
   gh run download <run-id> -D /tmp/sea-artifacts
   mkdir /tmp/sea-flat && cp /tmp/sea-artifacts/sea-*/cheaper-* /tmp/sea-flat/
   node scripts/build-apm-bundle.mjs --exe-dir /tmp/sea-flat
   # POSIX 側 binary（cli/bin/linux-*/cheaper）は executable bit を
   # 維持すること（git update-index --chmod=+x で確認・修正）
   ```

6. `mise run check` が全 pass することを確認する（lint → version →
   doc-id → typecheck → test → bundle freshness →
   `check-apm-payload.mjs` の expected target set / pointer /
   host version 検証）。
7. 宣言変更と生成物（`packages/apm/.apm/hooks/cli/`。payload
   executable は Git LFS）を同一 commit にまとめる
   （`release:` または `chore:` prefix）。
8. `node packages/cli/bin/cheaper.js --version` と、bundle 内
   `version.json` が新しい値を返すことを目視確認する。
9. `git tag v<X.Y.Z>` で tag を打ち、push する。

tagのsourceには新workflowが使用するscriptsとlauncher、整合するversion.jsonが
含まれている必要がある。公開後の既存tagを書き換えて再利用しない。

## 1 dispatch で acquisition 互換 Release を公開

1. GitHub Actions の **`release-acquisition`** workflowを手動実行し、
   `tag` inputに準備済み `v<X.Y.Z>` を指定する。自動trigger/local publish経路はない。
2. 同じrunで次の全jobが成功することを確認する。
   - `resolve-release`: tagとcanonical CLI/APM/version.jsonを照合し、focused stdlib
     suitesを実行、source SHAを固定する。
   - `build-executables`: 既存reusableのWindows x64/Linux x64/Linux arm64
     native build、`--version` / `models` smoke。expected_versionをbuild前に照合する。
   - `assemble-release`: Ubuntuで同runのraw3をhashし、strict `SHA256SUMS` と
     plain staged package snapshotを作る。Git上のpayloadは書き換えない。
   - `smoke-bundle`: 各native runnerでarchiveを展開し、全3埋込hashとmanifest、
     APM local install後のNode-free launcher `--version` / `models` を検証する。
     Linux上ではWindows binaryをhash検証のみとし、実行はWindows jobが担う。
   - `publish-release`: tagを再fetchして `FETCH_HEAD^{commit}` を固定SHAと比較。
     GitHub APIのpaginated一覧取得が成功してからexact既存tagを拒否し、全assetを
     draftへuploadして最後に公開する。`latest=false` としlatestを推測しない。
3. Releaseに以下の **5 asset** が揃うことを確認する。

| asset | 内容 |
| --- | --- |
| `cheaper-linux-x64` | Linux x64 SEA |
| `cheaper-linux-arm64` | Linux arm64 SEA |
| `cheaper-windows-x64.exe` | Windows x64 SEA |
| `SHA256SUMS` | raw3のSHA-256。ASCII subset、LF、64hex + ASCII space2個 + basename、空行/重複なし、64KiB以下 |
| `cheaper-apm-v<X.Y.Z>.tar.gz` | root `cheaper/` のAPM snapshot。同run raw3、内部 `cli/SHA256SUMS`、固定version metadata、primitivesを含む |

archiveは `apm pack` の形式ではなく展開してlocal path dependencyとして使う。
[配布guide](distribution.md#release-archive-を-local-path-で-install) を参照。
`SHA256SUMS` はraw3を対象としarchive全体のdigestは含めない。
macOSはこのReleaseの対象外。default権限はcontents:read、公開jobのみcontents:write、
tokenは `GITHUB_TOKEN` のみ使用する。同tagのdispatchはcancelせず直列化する。

## 旧 Release 経路との関係

旧 `release` workflowと手動 `build-executables`、LFS payload、freshness checkは保持する。
旧 `release` は `strict_freshness:true` でcommitted payloadのbyte一致を要求し、raw3だけを
公開する。**SHA256SUMSとAPM archiveを付けない旧経路はacquisition互換Releaseではない**。
sibling executableがない#73-A launcherはmanifest未公開の旧Releaseから取得できずfail closedする。
acquisition用の公開には新 `release-acquisition` を選ぶ。同tagで両workflowを実行しない。

#73-CでGit payload/LFS/CIの移行が完了するまで両経路を併存させる。
履歴rewriteやpayload削除はこの変更に含めない。

## 失敗時の確認と手動復旧

version不一致、tag drift、hash/smoke失敗、API error、既存draft/public Releaseは公開を止める。
既存Releaseの上書き、`--clobber`、自動削除、upload再開は行わない。
draft作成中や公開前に失敗すると未公開draftが残ることがある。operatorが対象runの
errorとtag/SHA、draftのasset状態を確認し、必要な回復を明示的に手動で行ってから
再dispatchする。公開済みReleaseへの再実行は新version/tagを準備する。

## 発火前の検証範囲

`mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs`
でstrict manifest、snapshot/source非変更、smoke失敗境界とworkflow構造を検証できる。
`assemble-release-bundle.mjs --asset-dir <dir> --out-dir <dir> --tag v<X.Y.Z> --dry-run` は
入力version/manifest/raw hashを検査しlayoutを出力する。Node stdlibのみを使用する。
`smoke-release.mjs --package-dir <cheaper-dir> --manifest <SHA256SUMS> --version <X.Y.Z> --dry-run`
は全3埋込hashとmetadataを検証し、APM install/実行を行わない。

Ubuntuでのtar member mode0755、native3の実build/install/smoke、GitHub API/upload/publicationは
CIで初めて確認できる。Windowsでのarchive fixture testはPOSIX modeを検証せずLinux専用testをskipする。
dry-run/構造検証の成功を実Release公開成功と同一視しない。

## 注意

- `version.json`・`bin/cheaper.mjs` は生成物。手編集しない。bundle
  本体に version 定数は埋め込まれない（`version.ts` が実行時に
  package.json → version.json fallback で読む）。
- 宣言を手で直接編集するのは避け、`set-version.mjs` を使う。ずれた
  状態で commit すると `mise run check`（CI）が fail する。

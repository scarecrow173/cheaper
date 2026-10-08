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
| `packages/tool-runtime/package.json` | workspace package | canonical と一致必須 |
| `packages/workflow/package.json` | workspace package | canonical と一致必須 |
| `packages/runtime-pi/package.json` | workspace package | canonical と一致必須 |
| `packages/apm/apm.yml` | 配布 APM manifest | canonical と一致必須 |
| `packages/apm/.apm/hooks/cli/version.json` | bundled CLI の version | 生成物（`bundle-cli.mjs` が cli から派生） |
| `Cargo.toml`（`[workspace.package].version`） | Rust binary の version 宣言 | canonical と一致必須（`cheaper-cli` crate が `version.workspace = true` で継承。canonical は Phase 5 時点でも package.json のまま） |
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

## Release 前の source / tag 準備

公開には `release-acquisition` workflow を **1 回 dispatch** する。
その1回で固定 SHA の cargo release build（Windows x64 / Linux x64 /
Linux arm64 + 検証専用の darwin-arm64）→ size gate・native smoke →
`SHA256SUMS` → APM archive → 4 OS の native APM smoke → GitHub
Release 公開までを実行する。配布 executable の実装は Rust binary
（旧 Node SEA から切替済み）だが、asset 名・`SHA256SUMS` 形式・
launcher の取得 contract は不変。
Git source に platform binary は含めない。`check-apm-payload.mjs` は launcher と
小さい asset の存在・regular file・pointer 拒否・JSON/version 整合を検証する。
native version と実行は同 release run の build/smoke が検証する。

1. `node scripts/set-version.mjs <X.Y.Z>` で bump する。
2. `node scripts/bundle-cli.mjs` で `version.json` と bundled CLI を
   再生成する。
3. `mise run check` が全 pass することを確認する（small asset freshness と source checker を含む）。
4. 宣言変更と小さい生成物を commit して branch を push する。
   `cli/bin/<platform>/` は ignore 対象であり、binary を Git に追加しない。
5. `node packages/cli/bin/cheaper.js --version` と、bundle 内
   `version.json` が新しい値を返すことを目視確認する。
6. `git tag v<X.Y.Z>` で immutable tag を打ち、push する。

tagのsourceには新workflowが使用するscriptsとlauncher、整合するversion.jsonが
含まれている必要がある。公開後の既存tagを書き換えて再利用しない。

## 1 dispatch で acquisition 互換 Release を公開

1. GitHub Actions の **`release-acquisition`** workflowを手動実行し、
   `tag` inputに準備済み `v<X.Y.Z>` を指定する。自動trigger/local publish経路はない。
2. 同じrunで次の全jobが成功することを確認する。
   - `resolve-release`: tagとcanonical CLI/APM/version.json/Cargo.tomlを照合し、
     focused stdlib suitesを実行、source SHAを固定する。
   - `build-executables`: reusable workflow が `cargo build --release` の
     matrix（`x86_64-pc-windows-msvc` / `x86_64-unknown-linux-musl` /
     `aarch64-unknown-linux-musl` / `aarch64-apple-darwin`）で Rust
     binary を build し、asset 名へ rename 後に ≤20MiB の size gate
     （`scripts/check-binary-size.mjs`、build 後・upload 前に fail 伝播）と
     `--version` / `models` smoke を挟んで `bin-<target>` artifact として
     upload する。darwin-arm64 は検証専用で Release asset に含めない。
     expected_versionをbuild前に照合する。
   - `assemble-release`: Ubuntuで同runのraw3をhashし、strict `SHA256SUMS` と
     plain staged package snapshotを作る。Git上のpayloadは書き換えない。
   - `smoke-bundle`: windows-latest / ubuntu-latest / ubuntu-24.04-arm /
     macos-latest の4 runnerでarchiveを展開し、全3埋込hashとmanifest、
     APM local install後のNode-free launcher `--version` / `models` を
     検証する。macOS は acquisition 非対象の sibling 経路:
     `bin-darwin-arm64` artifact を展開 package の
     `cli/bin/darwin-arm64/cheaper` へ 0755 で sibling 配置し、deploy 後の
     hash 同一性と launcher 経由の実行を検証する。
     Linux上ではWindows binaryをhash検証のみとし、実行はWindows jobが担う。
   - `publish-release`: tagを再fetchして `FETCH_HEAD^{commit}` を固定SHAと比較。
     GitHub APIのpaginated一覧取得が成功してからexact既存tagを拒否し、全assetを
     draftへuploadして最後に公開する。`latest=false` としlatestを推測しない。
3. Releaseに以下の **5 asset** が揃うことを確認する。

| asset | 内容 |
| --- | --- |
| `cheaper-linux-x64` | Linux x64 Rust binary（`x86_64-unknown-linux-musl`） |
| `cheaper-linux-arm64` | Linux arm64 Rust binary（`aarch64-unknown-linux-musl`） |
| `cheaper-windows-x64.exe` | Windows x64 Rust binary（`x86_64-pc-windows-msvc`） |
| `SHA256SUMS` | raw3のSHA-256。ASCII subset、LF、64hex + ASCII space2個 + basename、空行/重複なし、64KiB以下 |
| `cheaper-apm-v<X.Y.Z>.tar.gz` | root `cheaper/` のAPM snapshot。同run raw3、内部 `cli/SHA256SUMS`、固定version metadata、primitivesを含む |

archiveは `apm pack` の形式ではなく展開してlocal path dependencyとして使う。
[配布guide](distribution.md#release-archive-を-local-path-で-install) を参照。
`SHA256SUMS` はraw3を対象としarchive全体のdigestは含めない。
macOSはこのReleaseの対象外（darwin binary は build・sibling smoke の
検証用途のみで公開しない）。default権限はcontents:read、公開jobのみcontents:write、
tokenは `GITHUB_TOKEN` のみ使用する。同tagのdispatchはcancelせず直列化する。

## 旧 Release 経路との関係

旧 `release` workflow は contents: read の拒否 job として廃止する。
公開は `release-acquisition` の 1 dispatch のみ。手動 `build-executables` は
検証用 artifact を作るだけで公開しない。committed binary の byte 比較は行わない。
既存 Git LFS 履歴は保持するが、binary 再追加や checksum 不在の旧公開経路へ戻さない。

## 失敗時の確認と手動復旧

version不一致、tag drift、hash/smoke失敗、API error、既存draft/public Releaseは公開を止める。
既存Releaseの上書き、`--clobber`、自動削除、upload再開は行わない。
draft作成中や公開前に失敗すると未公開draftが残ることがある。operatorが対象runの
errorとtag/SHA、draftのasset状態を確認し、必要な回復を明示的に手動で行ってから
回復する。maintainer は失敗を修正し、新しい immutable version/tag で fix-forward する。
既存 tag を書き換えない。

新経路の初回実公開はローカル検証では未実証。全 5 asset と 4 smoke job
（3 OS の acquisition 経路 + macOS sibling 経路）の成功を
確認するまで consumer を新 Git ref へ切り替えない。consumer は切替前に稼働中の
manifest/ref と sibling を含む deploy directory を退避し、取得や hook が失敗したら
その退避を復元する。Copilot の制約は [配布 guide](distribution.md) を参照する。

## 発火前の検証範囲

`mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs scripts/test/binary-size.test.mjs`
でstrict manifest、snapshot/source非変更、smoke失敗境界、size gate の
境界、workflow構造を検証できる。
`assemble-release-bundle.mjs --asset-dir <dir> --out-dir <dir> --tag v<X.Y.Z> --dry-run` は
入力version/manifest/raw hashを検査しlayoutを出力する。Node stdlibのみを使用する。
`smoke-release.mjs --package-dir <cheaper-dir> --manifest <SHA256SUMS> --version <X.Y.Z> --dry-run`
は全3埋込hashとmetadataを検証し、APM install/実行を行わない。sibling-only
platform（darwin-arm64 等）では package 内 sibling の存在も要求する。

cargo matrix 4 target の実 build・size gate、Ubuntuでのtar member mode0755、
smoke-bundle の 4 OS 実 install/smoke（macOS は sibling 経路）、GitHub
API/upload/publicationはCIで初めて確認できる。Windowsでのarchive fixture
testはPOSIX modeを検証せずLinux専用testをskipする。
dry-run/構造検証の成功を実Release公開成功と同一視しない。

## 注意

- `version.json`・`bin/cheaper.mjs` は生成物。手編集しない。bundle
  本体に version 定数は埋め込まれない（`version.ts` が実行時に
  package.json → version.json fallback で読む）。
- 宣言を手で直接編集するのは避け、`set-version.mjs` を使う。ずれた
  状態で commit すると `mise run check`（CI）が fail する。

# Launcher と配布 executable の取得

APM package の `cheaper`（POSIX sh）/ `cheaper.ps1`（Windows）は、sibling の
`cli/bin/<platform>/cheaper[.exe]` を優先する。sibling が無い場合にだけ、
`cli/version.json` に固定された version の GitHub Release を user cache へ取得する。
Node は不要。`latest` を参照せず、取得/検証失敗は stderr の診断と非0 exit で通知する。

取得契約は [詳細設計](../designs/version-pinned-launcher-acquisition.md) を参照。
配布 executable の build pipeline は旧 Node SEA から Rust binary（cargo
release build）へ切替済みだが、Rust binary を含む Release は未公開であり、
初回公開は `release-acquisition` の dispatch が担う。asset 名・`SHA256SUMS`
形式・`--version` probe・取得 contract は不変。
Git source は platform executable を含まない。launcher と小さい CLI asset を install し、
固定 Release から取得する。Git LFS の既存履歴は保持するが、新 binary は Git に追加しない。
`release-acquisition` の1dispatchでraw3 / `SHA256SUMS` / APM archiveを公開する
（[release手順](release.md)）。旧 `release` workflow は拒否 job として廃止済み。
checksum manifest が公開されていない旧Releaseへの初回取得はfail closedする。

## Release archive を local path で install

同じ固定tagの `cheaper-apm-v<version>.tar.gz` をdownloadし、独立directoryへ展開する。
archiveは `apm pack` ではなくroot `cheaper/` を持つplain package snapshotである。
例としてversion1.0.0をconsumerの隣へ配置する:

```sh
mkdir release-v1.0.0
gh release download v1.0.0 --repo scarecrow173/cheaper --pattern cheaper-apm-v1.0.0.tar.gz --dir release-v1.0.0
tar -xzf release-v1.0.0/cheaper-apm-v1.0.0.tar.gz -C release-v1.0.0
mkdir release-v1.0.0/consumer
```

`release-v1.0.0/consumer/apm.yml` にlocal path依存を指定する:

```yaml
name: consumer
version: "0.0.1"
targets:
- windsurf
dependencies:
  apm:
  - path: ../cheaper
```

consumerで `mise exec -- apm install --target windsurf` を実行する（consumer側の
APM環境を使用）。archiveには同runのraw3、内部 `cli/SHA256SUMS`、version.json、
primitivesを含み、Linux binariesは0755で保存する。workflowはWindows/Linux x64/Linux
arm64でlocal install後のNode-free launcher、および macOS で sibling 経路を
検証してから公開する。
これは既存Git-based APM installを置換する強制移行ではなく追加の配布経路である。
公開manifestのdigest対象はraw3でありarchive全体ではない。SHA-256は署名保証ではない。

## 対象と固定 version

| acquisition target | Release asset | cache executable |
| --- | --- | --- |
| linux-x64 | cheaper-linux-x64 | cheaper |
| linux-arm64 | cheaper-linux-arm64 | cheaper |
| win32-x64 | cheaper-windows-x64.exe | cheaper.exe |

tag は `v<cli/version.json の version>`。対応外の OS/arch は自動取得せず、
local sibling を用意するよう診断する。Windows ARM64 を x64 と誤認して取得しない。
macOS（darwin-arm64）は acquisition 非対象で、Release asset・manifest にも
含めない。darwin binary は build・sibling smoke の検証用途のみであり、
macOS の consumer は deploy 済み sibling（`cli/bin/darwin-arm64/cheaper`）を
使う。
existing sibling は従来の platform mapping と動作を維持する。
Git LFS pointer が sibling にある場合も取得に切り替えず、`git lfs install` と
package cache での `git lfs pull`、または APM の再 install を案内する。

## Cache の場所と検証

- POSIX: `${XDG_CACHE_HOME:-$HOME/.cache}/cheaper`
- Windows: `%LOCALAPPDATA%/cheaper/cache`
- override: `CHEAPER_CACHE_DIR`（絶対 path）

cache は deployed hook directory の外に置く。Windows は symlink/junction を
経由する cache path を拒否するので plain user directory を使う。
version parent は lock を作成できる writable directory である必要がある。
credentials や usage log の `CHEAPER_CONFIG_DIR` と cache は別責務。

```text
<cache>/<version>/<platform>/cheaper[.exe]
<cache>/<version>/<platform>/SHA256SUMS
```

初回は同 tag の manifest と raw executable を取得し、SHA-256 と `--version` を
照合してから pair 全体を atomic directory rename で公開する。同 key の lock により
同時 first-run は1回の取得を共有する。cache hit も manifest/hash/version を照合する。
binary-only/manifest-only、hash 不一致、wrong version は実行せず、network で暗黙修復しない。
公開 SHA-256 は破損と誤配送の検査であり publisher の署名保証ではない。

## Proxy と mirror

POSIX には curl、sha256sum または shasum、realpath と標準 sh utilities が必要。
Windows は curl.exe があれば使い、無ければ Invoke-WebRequest を使う。
PowerShell launcher は Windows PowerShell 5.1 と pwsh で読める UTF-8 BOM 付きファイル。

curl は通常の `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` を利用する。
Invoke-WebRequest は OS/.NET proxy、または `HTTPS_PROXY` の明示 proxy を使う。

```sh
export HTTPS_PROXY=http://proxy.example:8080
export CHEAPER_CACHE_DIR="$HOME/.cache/cheaper"
```

```powershell
$env:HTTPS_PROXY = 'http://proxy.example:8080'
$env:CHEAPER_CACHE_DIR = Join-Path $env:LOCALAPPDATA 'cheaper/cache'
```

trusted HTTPS mirror を使う場合は `CHEAPER_RELEASE_BASE_URL` に
`https://mirror.example/cheaper/releases/download` のような base を指定する。
launcher はそこへ `/v<version>/SHA256SUMS` と `/v<version>/<asset>` を追加する。
userinfo/query/fragment は使わない。HTTP base は literal localhost/127.0.0.1/[::1] の
test stub 用に限り、redirect は常に HTTPS-only。proxy credential を report に貼らない。

## Offline preseed

network が使える環境で、package と**同じ固定 tag**の raw asset と `SHA256SUMS` を取得し、
上記 cache key に配置する。例えば version 1.0.0 の Windows x64 は
`<cache>/1.0.0/win32-x64/cheaper.exe` と `SHA256SUMS`。
POSIX では raw asset を `cheaper` に rename して executable bit を付ける。

```sh
chmod u+x "$CHEAPER_CACHE_DIR/1.0.0/linux-x64/cheaper"
```

manifest を手作りして別 binary に合わせる操作は preseed の検証を失わせる。
manifest は同 Release の公開ファイルをそのまま配置する。形式は LF の
`<64桁 hex><ASCII space 2個><Release asset basename>` の行（空行無し、64KiB以下）。
対象 asset の entry が1件必要。hash の大小文字は同一視する。

preseed 後は network 不要だが、version.json と writable cache parent は必要。
不正 key は、実行中の launcher/CLI が無いことを確認して対象 version/platform key だけを
除去し、正しい pair を再配置する。他 version や user configuration を消す必要はない。

## 既存 install と dev local build

既存 install の更新には pinned APM dependency ref を更新して `apm install` を再実行する。
更新後も sibling があればそちらを使う。sibling 不在の consumer は固定 tag の Release が
公開されていることと version metadata が deploy されていることを確認する。

APM 0.28.0 の Copilot deploy は `*.json` を除外し、version.json/package.json が届かない。
sibling 不在かつ version.json 不在の場合は取得も cache 起動もせず明示失敗する。
Git-based binary-free acquisition の対象は Windsurf / Claude / Codex / Cursor。
Copilot は release archive または local build の sibling を保持する。Copilot で
binary-free install を有効化するには JSON deploy gap を別変更で先に解消する必要がある。
offline preseed でもこの制約は変わらない。

root apm.yml は development-only で cheaper 自身の hook を deploy しない。
dev consumer は local build の host executable（`cargo build --release -p
cheaper-cli` の出力、または並走期間中は `scripts/build-executable.mjs` の
SEA 出力）を対応 sibling へ配置して使う。
未 release の package version に Release download fallback を期待しない。
local output は `cli/bin/*/` の ignore 対象。source checker は foreign binary の存在や
byte 一致を要求せず、native build/smoke が executable の version・実行を検証する。

## Git-based consumer の移行と復旧

1. 新経路の初回公開はローカル検証だけでは未実証。移行先の immutable tag について
   全 5 Release asset と 4 smoke job（3 OS の acquisition 経路 + macOS
   sibling 経路）の成功を確認するまで切り替えない。
2. 稼働中の consumer manifest/ref と deploy directory 全体（sibling を含む）を退避する。
3. Windsurf / Claude / Codex / Cursor は APM dependency をその tag/ref に更新し、
   `apm install --target <target>` を再実行する。version.json の deploy を確認する。
4. 古い deploy に sibling が残った場合は、対応する
   `cli/bin/<platform>/cheaper[.exe]` だけを除去し、launcher acquisition を使う。
   Copilot にはこの除去手順を適用しない。
5. 初回 hook は network/proxy と writable cache を必要とする。offline 環境は同 Release の
   pair を preseed、または release archive を local path install する。
   warm verified cache は network を使わない。cold offline は Release/SHA256SUMS、
   proxy/mirror、offline preseed を示す stderr 診断と非 0 exit を返す。
6. install/acquisition/hook が失敗したら、退避した manifest/ref と deploy directory を復元する。
   maintainer は新 immutable version/tag で fix-forward する。binary 再追加、tag rewrite、
   checksum 不在の旧 publication は復旧経路にしない。

launcher の非 0 exit は launcher 層の契約。Cursor の host hook は失敗を
`permission: allow` + exit 0 に変換し、Copilot entry も allow/exit 0 fallback を持つ。
このため取得失敗時に host が deny する保証はなく、hook deny が失われる場合がある。
各 host の実イベントでの挙動確認は local launcher テストとは別に行う。

## 中断と lock の復旧

通常の download 中断、hash/version 不一致、書込失敗では、自身の staging と lock を掃除する。
強制 kill / OS shutdown では残る場合がある。lock は最大120秒待ち、その後 stderr に
`.<platform>.lock` の path と手動復旧を表示する。

lock の `owner` に PID がある。該当 acquisition process が停止したことを確認してから、
同 version parent の該当 `.<platform>.lock` と orphan `.<platform>.staging.*` のみ削除する。
稼働中の lock を削除しない。古そうに見える lock を launcher が自動奪取することはない。

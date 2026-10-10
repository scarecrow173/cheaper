# cheaper CLI の配布と install

`cheaper` CLI は **Cargo source install** で consumer へ配布する
（ADR-034Dmb3kNAO4Ub6FxuQCtC。旧 sibling binary / GitHub Release 取得 /
SHA256SUMS 検証 / user cache モデルは廃止）。Git・Git LFS・GitHub
Release・APM package のいずれにも build 済み executable を含めない。

APM package が配布するのは skills・`hooks.json`・thin launcher
（`cheaper.sh` / `cheaper.ps1`）のみ。launcher は PATH 上に install
済みの `cheaper` を解決して起動するだけで、download・cache・version
pinning・checksum 検証を一切行わない。

## Consumer の install

### 前提

- **Rust toolchain ≥ 1.95**（依存 `rig` が `rust-version` 1.95 を要求）
- **git**
- **OS の C toolchain**（Windows は MSVC build tools）
- Cargo の bin directory が PATH 上にあること:
  - POSIX: `~/.cargo/bin`
  - Windows: `%USERPROFILE%\.cargo\bin`
  - IDE / agent process は起動時の環境を継承するため、PATH を更新した
    場合は **IDE / agent を再起動**してから使う

### install

```bash
cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli
```

`--locked` は commit 済み `Cargo.lock` の dependency graph に pin する
（git ref の pin ではない — repo の HEAD が build される）。特定 ref を
固定したい場合は `--branch` / `--tag` / `--rev` を併用する。

update は同じコマンドに `--force` を付ける:

```bash
cargo install --git https://github.com/scarecrow173/cheaper.git --locked --force cheaper-cli
```

### 動作確認

```bash
cheaper --version   # preflight
cheaper models      # model catalog 解決の smoke
```

### APM package の install

CLI とは別に、parent agent へ skills / hooks を deploy する APM
package は従来どおり `apm install` で install する:

```yaml
# consumer の apm.yml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm#main   # virtual package 参照
  # または local path: - path: ../cheaper/packages/apm
```

```bash
apm install --target windsurf
```

deploy される hook bundle は `<hooks_root>/<dep-name>/`（例:
`.windsurf/hooks/apm/`）に `cheaper.sh` / `cheaper.ps1` /
`hooks.json` を持つ。`cli/` payload は存在しない — worker 定義・
policy・version は binary に embedded されている。

旧 package（`cli/` payload・extensionless `cheaper` launcher を持つ版）
から upgrade する consumer は注意: APM の reinstall は deploy dir 内の
stale file を **prune しない**（APM 0.28.0 実測）。upgrade 後に
`<hooks_root>/<dep-name>/cli/` と拡張子なし `cheaper` が残る場合は
1 回だけ手動で削除する。hook は `sh ./cheaper.sh` しか参照しないため
残存 file は実害なしだが、再 deploy を確実にしたい場合は bundle dir
全体を消して `apm install` を再実行すればよい。

## Launcher の挙動

- **POSIX**（`cheaper.sh`）: `command -v cheaper` で PATH 上の CLI を
  解決し、`exec cheaper "$@"` で argv・stdin・stdout・exit code をその
  まま通す。見つからない場合は install command と PATH 設定の案内を
  stderr に出して exit 1（stdout には何も出さない）。`hooks.json` の
  POSIX entry は `sh ./cheaper.sh` で起動するため、deploy 時に exec
  bit が落ちても動作する。
- **Windows**（`cheaper.ps1`）: `Get-Command cheaper, cheaper.exe
  -CommandType Application` で解決し、`ProcessStartInfo` 経由で子
  process を起動して exit code を返す（PowerShell 5.1 で空引数・
  quote が壊れる既知問題を避けるため `&` 直接呼び出しは使わない）。
  見つからない場合は同様の案内を stderr に出して exit 1。
- launcher は hook 実行時に install / update / download / prompt を
  **一切行わない**。CLI が無い場合の案内出力のみ。

## 開発者向け（repo 内）

repo 内では `cargo build -p cheaper-cli`（または `cargo run -p
cheaper-cli -- <args>`）で CLI を使う。`cargo install --path
crates/cheaper-cli --locked` で local install した binary は hook
launcher からも解決される。

root の `apm.yml` は development-only project であり、cheaper 自身の
skill/hook は deploy されない。consumer へ配布する APM package は
`packages/apm/`（`apm.yml` + `.apm/` primitives）に分離されている。

`mise run check` は slim payload 契約（`scripts/check-apm-payload.mjs`）
と committed lockfile 契約（`scripts/check-lockfile.mjs`）を検証する
— 後者は `cargo metadata --locked` + working-tree の `Cargo.lock`
drift 検査で、consumer の `--locked` install が常に成功することを保証
する。

## 廃止された旧配布経路

以下は issue #85 で撤去された。いずれも復旧経路として使わない:

- `cli/bin/<platform>/` sibling executable と `.gitignore` rule
- `cli/version.json` の固定 version → GitHub Release asset download
- `SHA256SUMS` checksum manifest 検証・user cache（`CHEAPER_CACHE_DIR`）
  ・offline preseed・`CHEAPER_RELEASE_BASE_URL` mirror
- `release-acquisition` / `build-executables` / `release` workflow と
  bundle 系 script（`bundle-cli.mjs`・`assemble-release-bundle.mjs` 等）

旧経路の歴史的記録は superseded 付きで `docs/designs/`・`docs/adr/`
配下に残る。

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
package は従来どおり `apm install` で install する（apm ≥0.28 推奨 —
hook package は transitive `path:` dep と per-dep `targets:` を使う。
0.22 以降で動作する）:

```yaml
# consumer の apm.yml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm#main   # virtual package 参照
```

```bash
apm install --target windsurf
```

issue #118 で hook 宣言は per-target sibling package（`apm-hook-*`）へ
分離された。`packages/apm`（skills）が `dependencies.apm` の transitive
`path:` dep として 4 package を引き込むため、consumer の dep 宣言は
変わらず 1 本でよい。

deploy される hook bundle は `<hooks_root>/<pkg>/` に `cheaper.sh` /
`cheaper.ps1` / `hooks.json` を持つ — `<pkg>` は hook package 名で
target 毎に決まる:

| Target | Hook bundle root |
|---|---|
| Windsurf | `.windsurf/hooks/apm-hook-windsurf/` |
| Claude Code | `.claude/hooks/apm-hook-claude/` |
| OpenAI Codex | `.codex/hooks/apm-hook-codex/` |
| Cursor | `.cursor/hooks/apm-hook-cursor/` |
| GitHub Copilot | `.github/hooks/scripts/apm-hook-copilot/` |

`cli/` payload は存在しない — worker 定義・policy・version は binary に
embedded されている。各 target config には native event のみが登録
される（他 harness の event key は残らない）。

注意点:

- `packages/apm` は self-contained ではなくなった。repo 全体ではなく
  `packages/apm` dir だけをコピーして `path:` dep で install する
  vendored 利用、`apm pack` / `apm publish` による registry 配布は
  非対応（transitive `path:` の解決に sibling dir が要る）。
- dep-edge `targets:`（例: `[agent-skills]`）で dep を narrow しても
  transitive hook package へは**伝播しない**。harness target も
  active な consumer で hook を出したくない場合は、install の active
  target を `agent-skills` のみに絞るか、policy の
  `dependencies.deny` で `apm-hook-*` を拒否する。

旧 package から upgrade する consumer は注意: APM の reinstall /
update は旧 package が merge した hook entry を **除去しない**
（APM は hook file を失った package の entry を掃除しない）。split
前の layout から移行するには `apm uninstall <dep>` → `apm install`
を実行する（global install は `apm uninstall -g <dep>`）。旧 ref pin
（`#<sha>`/`#<tag>`）の consumer は先に ref を post-split commit へ
更新すること。uninstall が managed-file 変更を検出して nonzero で
終わる場合は、生成 config（`.windsurf/hooks.json` 等）内で旧 dep を
指す entry を手動で除去する。

### Devin CLI の native hook 登録（`cheaper hook-config`）

Devin CLI は APM hook target ではなく、native hook は `.devin/config.json`
へ登録する。登録・検査・除去は **CLI 自身の `hook-config` command が担う**
（ADR-034H2WBNtC4We8dyC5lQZJ、issue #107 — 旧 PowerShell script の直接
JSON 編集は CLI への thin wrapper に縮退した）。

```bash
cheaper hook-config verify              # 登録状態の読み取り専用レポート
cheaper hook-config sync                # canonical entry を冪等に upsert
cheaper hook-config remove              # own entry のみ除去（非破壊）
cheaper hook-config sync --scope user   # user scope（~/.config/devin 等）
cheaper hook-config --event-stdin       # APM lifecycle payload を stdin から実行
```

canonical entry は matcher `^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Ee][Xx][Ee][Cc])$`
（read/grep/exec、case-insensitive）+ command
`cheaper hook pre-tool-use --target devin`。旧 `read|grep` matcher を持つ
既存登録は `sync` が canonical matcher へ収束させる。

`sync` の安全条件:

- PATH 上の `cheaper` が `hook targets --json` で `devin` target と
  `hookConfig: true` を報告すること（capability gate — 古い CLI では
  何も書かず拒否する）
- 既知の重複 cheaper read hook（`.windsurf/hooks.json`・`.devin/hooks.json`・
  `.devin/hooks.v1.json`・`.devin/config.local.json`・`.claude/settings*.json`
  や対象 config 内の他登録）を検出した場合は conflict file を報告して停止する
  — 二重発火を防ぐため利用者が明示的に解消する
- config は strict JSON のみ。JSONC/comment・大小文字違いの key・外部変更・
  symlink/reparse 経路は拒否。既存の無関係な key/entry は保全し、書き込みは
  temp file + atomic rename

Devin tool 評価は bounded になった（issue #107）: `read` の `offset`/`limit`
は window のみを計測し、`grep` は検索結果（一致行 + context）を上限付きで
実体化して計測する — 巨大 file の短い範囲読みは通る。shell は Devin の
検証済み tool `exec`（`tool_input.command`）を read-candidate 抽出で評価する。

#### APM lifecycle 連携（opt-in）

consumer の `apm.yml` に project lifecycle script を登録すると、
`apm install`/`update`/`uninstall` で hook-config が収束される:

```yaml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm#main
    targets: [agent-skills]   # 必須 — これが無いと dep 全体が skip される
lifecycle:
  post-install:  cheaper hook-config --event-stdin
  post-update:   cheaper hook-config --event-stdin
  post-uninstall: cheaper hook-config --event-stdin
```

lifecycle payload は stdin JSON で渡り、`hook-config` が event を写像する
（`post-install`/`post-update` → sync+verify、`post-uninstall` → packages に
cheaper が含まれる場合のみ remove、`pre-*`/未知 → noop）。project script は
`apm lifecycle trust` での明示 trust が必要で、`APM_NO_SCRIPTS=1` や trust
未了・script 失敗では自動収束されない — `verify` が残余状態を報告する
fallback として使う。

#### 既存 consumer からの移行

旧 APM 配布（`.windsurf/hooks.json` への cheaper 登録）からの移行条件:

- cheaper dependency が **`targets: [agent-skills]`** を持つこと。無いと
  APM の reconcile が dep 自体を skip し skills まで外れる
- `read_config_from` で `windsurf`/`claude` の自動 import を有効にして
  いる環境では、旧登録が再 import され conflict gate が発火しうる —
  import を明示的に無効化するか、旧 file の cheaper entry を消してから
  `sync` を実行する
- `verify` は残余 artifact・matcher drift・shadow（`.devin/hooks.v1.json`
  等が config.json を shadow する可能性）を検出して remediation を報告する

Windows 向けの preview/`-Apply` helper（`cheaper-usage` skill resource
`install-devin-hooks.ps1`）は残るが、実処理は `cheaper hook-config` へ
委譲する thin wrapper。POSIX では CLI を直接呼ぶ（wrapper 不要）。

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

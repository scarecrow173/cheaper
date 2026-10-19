# AGENTS.md

## プロジェクト概要

`cheaper` は、親 Coding Agent（Claude Code / Codex / Copilot / Cursor 等）から
処理を安価なモデルへ委譲するための汎用 Worker Runtime。
frontier model のコストを抑えるため、巨大コンテキストの処理や機械的なコード生成を
安価なモデル側で実行する。

## セットアップ

```bash
mise install     # mise 管理toolchainを mise.toml + mise.lock の固定バージョンで復元
mise run setup   # fresh checkout を開発可能にする canonical setup contract
mise run check   # CI と同等の canonical verification contract
```

Git source は thin launcher（`cheaper.sh` / `cheaper.ps1`）と skills /
hooks.json のみを保持し、platform executable を一切含まない。CLI 本体は
consumer が `cargo install --git https://github.com/scarecrow173/cheaper.git
--locked cheaper-cli` で PATH 上へ install し、launcher はそれを解決して
起動するだけ（download / cache / Release 取得の機構は無い）。
Git LFS の既存履歴は保持する。consumer の前提・install 手順は
`docs/guides/distribution.md` を参照する。

`mise install` は `mise.toml` と整合する `mise.lock` の固定バージョンとchecksumに従って
mise 管理 toolchain（node・rust・python・apm・prek・git-lfs 等）を復元する。
`mise run setup` はその toolchain を使って
`apm install` → `cargo fetch` → `cargo build --workspace --all-targets` を復元し、
`prek install` で pre-commit hook を有効化し（定義は
`.pre-commit-config.yaml` — 高速な整形・衛生 check のみ、重い検証は
`mise run check` / CI 側）、`git-lfs install --local` で LFS filter を
repo の `.git/config` に登録する（既存履歴の checkout 用。global
`~/.gitconfig` は変更しない）。
`mise run check` は `biome check` → `cargo fmt --check` → `cargo clippy` →
version/doc-id check → `cargo test --workspace` → `node --test`（scripts・
benchmarks）→ committed lockfile check（`node scripts/check-lockfile.mjs`）→
slim payload check（`node scripts/check-apm-payload.mjs`）の Rust pipeline。
setup/check 内の build・clippy・test 系 cargo コマンドは
`--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` を付けて実行する
（dev/test seam は `dev-seam` cargo feature でのみ compile 対象であり、
feature 無しでは seam 依存 test target が検証から抜け落ちる —
ADR-034DyGyITd0YaATg8fkld7）。コマンド実行は `mise exec -- <command>` 経由で
行い、mise.toml が管理するツールバージョンを有効化する。

worktree の作成・sibling worktree の探索・他 worktree の生成物（target /
APM artifact）の独自流用は Agent の責務ではなく、各 worktree は独立した
checkout として扱う。通常の worktree では `.worktreeinclude` に列挙された
gitignore 対象の生成資産（`apm_modules/`・`.agents/`・`.sdp/`・各 agent
tool 配下の deploy 物・`.mcp.json`・`.env` 系）が worktree 作成基盤によって宣言的にコピーされる。これは Agent が独自に他 worktree を
探してコピーする行為とは別物であり、Agent はこの仕組みの有無を前提に
sibling を探索しない。依存定義や `apm.lock.yaml`・`mise.lock` を変更した
場合はコピー済み資産が古くなり得るため、`mise install` → `mise run setup`
で再生成する。完全に fresh な worktree（コピー無しまたは cleanup 済み）では
`target/`・`apm_modules`・APM deploy 生成物が存在しない前提で
`mise install` → `mise run setup` を実行する。

完全に fresh な環境が必要な場合は、対象 worktree のルートでのみ
`git clean -ndX` で ignore 対象の管理外ファイルをプレビューし、確認後に
`git clean -fdX` で削除する。この操作では `.env`・APM 生成物・ローカル
キャッシュも失われる点に注意。ignore 対象でない未追跡ファイルも消す必要が
ある場合に限り `git clean -ndx` で対象を確認してから `git clean -fdx` を
選ぶ（Git 管理ファイルは対象外）。shared path・symlink・外部参照がある
場合は他 worktree への影響を事前に確認する。削除は setup task や worktree
作成処理で無条件に自動実行しない。cleanup 後の再構築は
`mise install` → `mise run setup` → `mise run check`。

root の `apm.yml` は development-only project であり、cheaper 自身の
skill/hook は deploy されない。consumer へ配布する APM package は
`packages/apm/`（skills のみ、`apm.yml` + `.apm/` primitives）と
`packages/apm-hook-*/`（per-target hook package — `apm-hook-windsurf`・
`apm-hook-cursor`・`apm-hook-copilot`・`apm-hook-claude`・
`apm-hook-codex`）に分離
されている。hook 宣言は target 毎の sibling package が持ち、skills
package が `dependencies.apm` の transitive `path:` dep で引き込む
（issue #118 — 単一 `hooks.json` を全 target へ merge した旧構造の
foreign/dead registration を解消）。worker 定義・schema・policy は
`crates/cheaper-cli/resources/` から Rust binary へ embedded されるため、
`cli/` payload は配布しない（issue #85）。

Node.js は repo tooling（`scripts/`・`benchmarks/`・doc-suite skills）のためだけに
toolchain に残る。product・distribution・hook 実行経路に Node 依存はない
（runtime は Rust workspace = `crates/`、agent runtime は Rig）。

### CLI integration test の配置

通常の CLI 契約テストは `crates/cheaper-cli/tests/integration/` に配置し、
新しい module を追加したら `main.rs` に `mod` 宣言を追加する。
共通 helper は既存の `tests/common/mod.rs` を共有する。
個別領域は test target ではなく module filter で指定する（例: hook）。

```bash
mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::
```

`workflow`、`run_dev_seam`、`workflow_dev_seam`、observability 系2 target、
`regression` は独立 target として維持する。自己再実行の `--exact` filter、
プロセス隔離、`required-features` の境界を崩さず、容量目的でテストを無効化しない。

### runtime / workflow integration test の配置

`cheaper-runtime` の通常契約テストは `tests/contracts/` にまとめる。
`cheaper-workflow` は `tests/compile_contracts/`（binding・compiler・interface・依存規約）と
`tests/filesystem_contracts/`（artifact・loader・resource inference）に分ける。
新しい module を追加したら各 `main.rs` に宣言を追加し、個別領域は module filter で指定する。

```bash
mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test contracts auth::
mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test filesystem_contracts loader::
```

runtime の `run_dev_seam` / `run_fixture_test` の feature gate と `ollama_e2e`、
workflow の `engine` / `process` は独立 target として維持する。

## 検証用 Ollama コンテナ

実 provider smoke 検証には Ollama を Docker コンテナで使う（ADR-034r05iHlWosDpQ6Z1LgJo）。ホストへのインストールは行わない。

- 起動: `docker compose up -d`（`ollama/ollama` イメージ、`127.0.0.1:11434` 公開、`ollama` volume でモデル永続化）
- モデル取得: `docker compose exec ollama ollama pull <model>`（例: `qwen3.5:4b` の量子化タグ）
- 稼働確認: `curl http://localhost:11434/v1/models` がモデル一覧 JSON を返せば起動済み。応答しない・接続拒否の場合はコンテナが未起動なので `docker compose up -d` を実行する
- 停止: `docker compose down`（volume は残る。モデルも消す場合は `docker compose down -v`）

コンテナが起動していない状態で実 provider smoke を実行すると provider 到達不能で失敗する。上記の稼働確認で未起動に気づいたら起動してから再実行する。

`crates/cheaper-runtime/tests/ollama_e2e.rs` は `CHEAPER_E2E_OLLAMA=1` が
設定され、かつコンテナ起動 + モデル pull 済み（default `qwen3.5:9b`、
`CHEAPER_E2E_OLLAMA_MODEL`・`CHEAPER_E2E_OLLAMA_BASE` で override 可）であれば
`cargo test --workspace` 内で実 provider 検証（bulk-reader / code-writer /
doc-writer → production session 経路 → Ollama）を実行する。gate 未設定・
未起動・モデル不在のときは早期 return で自動 skip するため、CI では実行されない。

## delegation 計測ログ

`cheaper run`（worker alias 含む）と `count-tokens` の実行結果は、記録対象に限り `<userConfigDir>/usage.jsonl`（既定 `~/.config/cheaper/usage.jsonl`、`CHEAPER_CONFIG_DIR` で変更可）へ 1 行 JSONL で継続記録される（ADR-034r05iHrZUE6WljM1ZMiX）。record は `kind:"run"`（worker・model・usage・durationMs）または `kind:"count"`（path・tokens・bytes）。記録対象は run の success と `meta.model`・`meta.usage` を持つ error（usage-bearing failure）、および `tokens` を返した count-tokens の成功に限られる — worker/model 解決前の error 等 usage を持たない failure、失敗した count-tokens、auth/help/version 等の utility command は記録されない。書き込みは best-effort で、失敗しても exit code・stdout は変わらない。workspace・`.cheaper/` には書き込まない。run record には optional で `turns`・`toolCalls`・`savings`（tool output token 和 vs 返却 result token 和の実測 offload、issue #44。`directlyComparable` で estimated 母数への参加可否を明示）が載る。集計は `cheaper usage`（`--since`/`--worker`/`--provider`/`--model` filter、`--compare-model` で core `resolvePricing` 経由の estimated savings — runtime catalog → `<userConfigDir>/pricing.json` snapshot → bundled snapshot の順に価格を解決し、直接比較可能な worker の run のみ算出）が担う。詳細は `docs/guides/usage.md`。

credential は `<userConfigDir>/credentials.json` の Rust 側 credential store
（`crates/cheaper-runtime/src/auth/store.rs`、temp + rename の atomic write）へ
永続化される。旧 Pi `ModelRuntime` の file（`auth.json` / `models.json` /
`models-store.json`、`<userConfigDir>/pi/` 以下）は一切読まず、
`PI_CODING_AGENT_DIR` も参照しない。`.cheaper/` には credential を置かない。
model catalog は bundled snapshot + bundled plugin provider + user overlay
（`<userConfigDir>/models.json`、旧 Pi `models.json` schema 互換）で構成される。

built-in catalog に無い provider は `CheaperProviderPlugin` contract で明示
bundle する（ADR-034r05iHrZUE6WoaNDNjvX）。`devin` provider は Devin CLI 委譲
auth + `<userConfigDir>/providers/devin/data/` 隔離の bundled plugin
（ADR-034r05iHrZUE6WqcIaH1CU、`crates/cheaper-provider` の `devin` module）— global Devin
credential は使わない。

## 開発原則

SOLID・YAGNI・DRY・KISS に従い、現在の要件に必要な範囲で設計・実装する。

- **SOLID**: 単一責任、開放閉鎖、リスコフの置換、インターフェース分離、依存性逆転を踏まえ、責務と依存関係を明確にする。
- **YAGNI**: 将来使うかもしれない機能や拡張点を先回りして実装しない。
- **DRY**: 同じ知識や業務ルールの重複を避ける。見た目が似ているだけの処理を無理に共通化しない。
- **KISS**: 要件を満たす最も単純で理解しやすい構成を選び、不要な抽象化や複雑さを持ち込まない。

実装内で表現ドリフトを起こさない。同じ情報を複数の表現方法（型・フォーマット・命名）で扱うことを避け、同一概念の表現は一箇所で定義して使い回す。表現の変換が必要な場合も変換処理を一箇所に集約する。

## Git 運用

- ブランチは Conventional Branch に従い、`main` から作業単位の短命ブランチを切る。プレフィックスは `feature/`, `bugfix/`, `hotfix/`, `release/`, `chore/`、名前は小文字の kebab-case（例: `feature/add-worker-runtime`）
- コミットメッセージは Conventional Commits に従う（`feat:`, `fix:`, `docs:`, `chore:` 等 + 命令形の要約）
- コミットは意味のある境界で分割し、適宜行う。無関係な変更を1コミットにまとめず、1コミットが1つの論理的な変更を表すようにする

## Version と release

製品 version の canonical source は root `Cargo.toml` の
`[workspace.package].version`（`cheaper-cli` crate が `version.workspace = true`
で継承し、build.rs が binary へ埋め込む）。bump と release の手順は
`docs/guides/release.md` に従う。version 宣言の不一致は
`mise run check`（`scripts/check-version.mjs`）が検出する。

## 境界

- **Always**: 変更後は関連する検証（型チェック・テスト等）を実行してから完了とする
- **Ask first**: 依存の追加・変更、破壊的な Git 操作（ブランチ削除・履歴書き換え等）
- **Never**: 秘密情報のコミット、このリポジトリ外のファイルの変更

## 言語

文書、コメント、報告は日本語を基本とする。研究ノートに英語版は不要。ユーザーが言語を指定した場合はその指示を優先する。

スキル、エージェントのペルソナ・プロンプト、MCP 定義、自動処理や言語処理の対象となる技術仕様では英語を使用できる。技術用語・製品名は原語のまま記す。

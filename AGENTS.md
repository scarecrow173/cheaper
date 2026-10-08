# AGENTS.md

## プロジェクト概要

`cheaper` は、親 Coding Agent（Claude Code / Codex / Copilot / Cursor 等）から
処理を安価なモデルへ委譲するための汎用 Worker Runtime。
frontier model のコストを抑えるため、巨大コンテキストの処理や機械的なコード生成を
安価なモデル側で実行する。

## セットアップ

```bash
mise run setup   # fresh checkout を開発可能にする canonical setup contract
mise run check   # CI と同等の canonical verification contract
```

Git source は launcher と小さい CLI asset のみを保持し、platform executable を
含まない。hook は固定 version の Release/SHA256SUMS から検証済み user cache へ
取得する。offline は同 Release の pair を preseed、dev は local cargo build
（`cargo build --release -p cheaper-cli`）の出力を sibling へ配置して使う。
`cli/bin/<platform>/` の local output は ignore し、binary を Git に追加しない。
Git LFS の既存履歴は保持する。consumer の対象・移行手順は `docs/guides/distribution.md` を参照する。

`mise run setup` は toolchain（`mise.toml` + `mise.lock` の locked version）に加え、
`apm install` → `cargo fetch` → `cargo build --workspace --all-targets` を復元する。
`mise run check` は `biome check` → `cargo fmt --check` → `cargo clippy` →
version/doc-id check → `cargo test --workspace` → `node --test`（scripts・
benchmarks）→ bundle 再生成（`node scripts/bundle-cli.mjs`）→ payload check →
bundle freshness の Rust pipeline。コマンド実行は `mise exec -- <command>` 経由で
行い、mise.toml が管理するツールバージョンを有効化する。

fresh worktree では `target/`・`apm_modules`・APM deploy 生成物が
存在しない前提で `mise run setup` を実行すること。worktree の作成・sibling
worktree の探索・他 worktree の生成物（target / APM artifact）
の流用は Agent の責務ではなく、各 worktree は独立した checkout として扱う。

root の `apm.yml` は development-only project であり、cheaper 自身の
skill/hook は deploy されない。consumer へ配布する APM package は
`packages/apm/`（`apm.yml` + `.apm/` primitives）に分離されており、
bundled CLI payload（`version.json`・`workers/`・`policy.yaml`）は
`node scripts/bundle-cli.mjs` で `packages/apm/.apm/hooks/cli/` へ生成する。
payload の複写元は `crates/cheaper-cli/resources/`（Rust binary の embedded
resource と同一 source）で、platform executable は payload に含めない。

Node.js は repo tooling（`scripts/`・`benchmarks/`・doc-suite skills）のためだけに
toolchain に残る。product・distribution・hook 実行経路に Node 依存はない
（runtime は Rust workspace = `crates/`、agent runtime は Rig）。

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
（ADR-034r05iHrZUE6WqcIaH1CU、`crates/cheaper-provider-devin`）— global Devin
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

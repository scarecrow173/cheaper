# Contributing

`cheaper` のリポジトリ開発に参加するための入口。

## セットアップ

fresh clone / fresh worktree では repository-owned contract として
以下を実行する（`mise.toml` の `[tasks.*]` で定義、ADR-034r05iHrZUE6WeDAi63F3）:

```bash
mise run setup   # toolchain + 依存 + build outputs を復元
mise run check   # CI と同等の検証（lint / fmt / clippy / test / lockfile・payload check）
```

`mise run setup` は `apm install` → `cargo fetch` →
`cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
を実行する。toolchain 自体は `mise.toml` + `mise.lock`（lockfile
優先解決）で再現される。fresh checkout で `mise` の trust を求められた
場合は `mise trust` を実行する。

`mise run check` は CI と同等の検証を実行する: `biome check` →
`cargo fmt --all -- --check` → `cargo clippy --workspace --all-targets
--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
→ `node scripts/check-version.mjs`（version 宣言の一致）→
`node scripts/check-doc-ids.mjs` → `cargo test --workspace --features
cheaper-runtime/dev-seam,cheaper-cli/dev-seam` →
`node --test scripts/test/` → `node --test benchmarks/test/` →
`node scripts/check-lockfile.mjs`（committed Cargo.lock の整合 +
drift 検査）→ `node scripts/check-apm-payload.mjs`（slim APM
payload 契約）。consumer は `cargo install --git … --locked` で
install するため、lockfile を再生成する変更は commit 済み
`Cargo.lock` に反映してからマージする。version は root `Cargo.toml` の
`[workspace.package].version` が canonical で、bump は
`node scripts/set-version.mjs <version>` で行う
（[release ガイド](docs/guides/release.md)）。

## テスト

dev/test seam（`fake_stream`・fixture DSL・fake provider）は
`dev-seam` cargo feature で gate されている
（ADR-034DyGyITd0YaATg8fkld7 — consumer install の default binary に
seam は含まれない）。seam 依存の test target は `required-features`
で gate されるため、通常の検証は feature 有効で行う。

```bash
# 全 crate の unit + integration test（seam 依存 target 含む）
cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam
cargo fmt --all -- --check          # rustfmt
cargo clippy --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings
biome check                         # repo 内 JS/JSON の lint
node --test scripts/test/           # repo tooling（check/launcher/payload）の test
node --test benchmarks/test/        # benchmark harness の test
```

product の実行経路は Rust binary（`cheaper-cli` crate）のみ。Node.js は
`scripts/`・`benchmarks/` 等の repo tooling に限定され、product・
distribution への依存はない。

### 実 provider 検証

`crates/cheaper-runtime/tests/ollama_e2e.rs` は `CHEAPER_E2E_OLLAMA=1`
が設定され、Ollama コンテナとモデルが利用可能なときに限り
`cargo test --workspace` 内で実 provider 検証を実行する。条件を
満たさない環境では早期 return で自動 skip される（ADR-034r05iHlWosDpQ6Z1LgJo /
ADR-034r05iHrZUE6WjfsVTWjO）。

```bash
docker compose up -d                       # Ollama コンテナ起動
docker compose exec ollama ollama pull qwen3.5:9b   # 既定モデル
curl http://localhost:11434/v1/models      # 稼働確認
CHEAPER_E2E_OLLAMA=1 cargo test -p cheaper-runtime --test ollama_e2e
docker compose down                        # 停止（volume・モデルは残る）
```

`CHEAPER_E2E_OLLAMA_MODEL`・`CHEAPER_E2E_OLLAMA_BASE` で検証対象を
上書きできる。

NVIDIA Container Toolkit の無い環境では `docker-compose.yml` の
`deploy:` セクション（GPU パススルー）が原因で `docker compose up`
が失敗する場合がある。その場合は `deploy:` を外すか toolkit を
install する。

## リポジトリ構造

- `crates/cheaper-cli` — `cheaper` CLI（command・envelope・hook adapter・
  usage.jsonl・embedded resource）。binary version は workspace version を
  継承する唯一の crate
- `crates/cheaper-core` — worker 定義・config・policy・model 解決・
  token count 等の pure-logic コア（Rig・tokio 非依存）
- `crates/cheaper-tools` — canonical tool（read/grep/find/ls/write/edit/
  count_tokens/process）・scheduler・mutation journal
- `crates/cheaper-runtime` — model catalog store・credential store・
  auth service・provider plugin contract・run orchestration
- `crates/cheaper-rig-adapter` — Rig SDK の隔離 adapter（Rig 型の参照は
  この crate と provider crate に限定）
- `crates/cheaper-provider` — bundled provider plugin（`devin` module）
- `crates/cheaper-workflow` — workflow schema・compiler・DAG engine
- `crates/cheaper-observability` — tracing subscriber・sink wiring・
  log activation（`--log` / `CHEAPER_LOG`）の observability 基盤
- `packages/apm` — distributable APM package（consumer へ deploy
  される skills/hooks/thin launcher。CLI 本体は cargo install）
- `scripts/` — version・lockfile・payload 等 check の Node 製 repo tooling
- `benchmarks/` — benchmark harness（Node 製 repo tooling）
- `apm.yml`（root） — development-only の APM project。root の
  `apm install` は cheaper 自身の skill/hook を deploy しない
  （ADR-034r05iHrZUE6WdKBH2eNM）

## 設計・開発記録

`docs/` は document-driven development の記録を含む。分類と
読み方は [docs/README.md](docs/README.md) を参照。

Coding Agent を開発に使う場合は `AGENTS.md`（エージェント向けの
指示: 環境・検証・境界）を参照する。

## コミット

- branch: `feature/`・`bugfix/`・`hotfix/`・`release/`・`chore/`
  prefix の kebab-case
- commit message: Conventional Commits（`feat:`・`fix:`・`docs:`・
  `chore:` 等）
- commit は論理単位で分割し、無関係な変更を混ぜない

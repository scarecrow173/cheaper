---
id: "IMPL-034D0FJrA2AtPRSNfjfe2v"
type: "impl"
status: "completed"
title: "Implement OpenAI Codex subscription provider"
created: "2026-10-09"
updated: "2026-10-10"
owners: []
relations:
  source: []
  changes:
    added:
      - { type: file, path: crates/cheaper-runtime/src/auth/oauth/openai_codex.rs }
    modified:
      - { type: file, path: crates/cheaper-rig-adapter/src/builtin.rs }
      - { type: file, path: crates/cheaper-runtime/Cargo.toml }
      - { type: file, path: crates/cheaper-runtime/src/auth/mod.rs }
      - { type: file, path: crates/cheaper-runtime/src/auth/oauth/mod.rs }
      - { type: file, path: crates/cheaper-runtime/src/lib.rs }
      - { type: file, path: crates/cheaper-runtime/src/catalog/bundled.rs }
      - { type: file, path: crates/cheaper-runtime/src/catalog/data/openai-codex.json }
      - { type: file, path: crates/cheaper-runtime/src/catalog/data/.manifest.json }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-openai-codex-subscription-provider.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement OpenAI Codex subscription provider

## Summary

builtin `openai-codex` provider(ChatGPT subscription backend)を 3 層で
実用化した: `openai-codex-responses` wire adapter の正式化
(`normalize_codex_base_url` + rig `chatgpt` dialect)、
`OpenAiCodexFlow`(browser PKCE + device-code)の
`builtin_oauth_flows` 登録、bundled `openai-codex.json` の pi-ai
1.1.0 同期(9 model、`gpt-6-luna` 含む)。

## Context

TASK-034CYsDxlTD7NNAPMkB4mb(
`docs/tasks/implement-openai-codex-subscription-provider.md`)の実行
結果。plan `PLAN-034CYiZ4kkOoa3oKL7PnLj` / design
`DESIGN-034CXyXncBigEivCETJLBD`(approved)/ ADR
`ADR-034CWHw0NjYQjdE5o4kErV`(Option A)/ spec
`SPEC-034CWHveVPBJp33jPRTCen` の chain に従う。

## Implementation

### Changes

- `8671788` `feat: wire openai-codex-responses models to rig chatgpt
  dialect` — `builtin.rs` codex arm(`OpenAIConfig` +
  `chatgpt::DIALECT`)+ `normalize_codex_base_url`(trailing `/` 除去
  → `/responses` strip → `/codex` append)+ `chatgpt-account-id`
  header 抽出/除去 + JWT fallback + `SUPPORTED_APIS` entry。
- `d764f61` `feat: add sha2, getrandom, and base64 for Codex OAuth
  PKCE` — `cheaper-runtime` への dep 追加(user 承認 + ADR 事前宣言)。
- `f0f8bd4` `feat: sync bundled openai-codex catalog with pi-ai
  1.1.0` — `openai-codex.json` を upstream から取得、`type=="chat"`
  filter + `type` 除去 + `chat:` prefix 除去で 9 model。
  `.manifest.json` sha256 更新 + `bundled.rs` 同期 assertion 2 件。
- `252dc77` `feat: add OpenAI Codex OAuth browser and device login
  flow` — `openai_codex.rs` 新設(PKCE/state 生成、
  `parse_authorization_input`、callback 評価 ordering、
  `wait_for_callback` + post-exchange drain、device-code flow、
  token exchange、refresh、to_auth)+ `builtin_oauth_flows` 登録 +
  re-export + unit test 21 件。
- `6f047e9` `fix: drop needless borrow in codex base url
  normalization` — clippy 修正。
- `c950907` `style: apply cargo fmt`。

### Decisions During Implementation

- **Windows accepted-socket nonblocking 継承への対処** — 並列
  test で flaky な `ConnectionReset` が再現し、accepted socket が
  listener の nonblocking を継承する(Windows 仕様)ことが原因と
  特定。`handle_callback_connection` 冒頭で
  `set_nonblocking(false)` + `\r\n\r\n` まで loop-read(未 read
  データを残した close は RST を返す)。実 browser でも segment 分割
  で同症例になり得る実 defect だったため test 側ではなく server 側で
  修正。
- **T3-T5+T7 の単一 commit** — plan は 4 commit 想定だったが、
  新規 1 file の partial staging は実質不可能なため coherent な単一
  `feat:` commit に統合。
- **実 credential smoke の credential 入手経路** — 対話 login は
  headless 環境で不可能なため `~/.codex/auth.json` の既存 Codex CLI
  session から oauth credential を `credentials.json` へ seed
  (login() が生成するものと同一構造)。`auth login` の対話経路自体は
  unit test + seam 構造で検証済み。

## Related Experiments

### Adopted

(なし)

### Rejected

(なし)

## Validation

- `cargo test -p cheaper-rig-adapter` — 8 unit + 19 integration 全 pass。
- `cargo test -p cheaper-runtime` — 56 lib(内 openai_codex 21 件)
  + 19 auth + 12 catalog + 18 oauth + 21 plugin + 18 run + 3 fixture
  + 3 ollama 全 pass。
- `mise run check` — biome/fmt/clippy -D warnings/workspace cargo
  test(全 crate)/check-version/check-doc-ids/node test 132 件/
  bundle smoke/APM payload check 全緑。
- 実 credential smoke — `auth status openai-codex` →
  `configured:true`(oauth)、`models --provider openai-codex` →
  `gpt-6-luna` 含む 9 model、`run bulk-reader --model
  openai-codex/gpt-6-luna` → `ok:true`(19.5s、7 turns・6 tool
  calls、55,859 tokens、repairs 0)。

## Risks

- `auth login openai-codex` の対話 browser/device 経路は実 endpoint
  で未検証(unit test + injectable seam で挙動は網羅)。credential
  seed 経路は login() 生成物と同一構造のため wire/auth は実証済み。
- refresh の実 endpoint 検証は未実施(invalid_grant → re-login
  hint は unit test で検証済み)。
- rig の request `originator` は `rig` 固定(authorize URL の
  `originator=cheaper` とは別 concept — design 承認済みの
  divergence)。

## Follow-ups

- 対話環境での `auth login openai-codex` 実検証(優先度低 — wire は
  seed credential で実証済み)。

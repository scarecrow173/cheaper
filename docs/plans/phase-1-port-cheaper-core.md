---
id: "PLAN-034BepXMKsB4VzqIP9XV7l"
type: "plan"
status: "approved"
title: "Phase 1 port cheaper-core"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-0-rig-poc-go-no-go-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-differential-golden.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 1 port cheaper-core Implementation Plan

## Goal

`packages/core`(src 約 2.65k 行)の全責務を、Rig 非依存・tokio 非依存の
純ロジック Rust crate `crates/cheaper-core` へ移植し、TS 実装から dump
した golden fixture との差分一致で挙動互換を検証可能にする。

## Scope

1 つの実装ストリーム。境界は「repo root の cargo workspace 初期化 +
`crates/cheaper-core` 全モジュール + TS golden fixture dump 機構 +
`crates/cheaper-core/tests/golden/` replay ハーネス」まで。

- **含む**: worker `.md` frontmatter+schema 解決・検証、policy.yaml
  schema/normalize/merge/restrict/capability、models.yaml・tier 分類・
  `resolveModel`/`resolveExplicitModel`、pricing snapshot・auto 分類、
  duration パース、`CheaperError` code→exitCode/retryable 表
  (`ERROR_TABLE` 写し)、workspace path canonicalize・symlink policy・
  denylist、o200k_base token 計測(`tiktoken-rs`)、resource manifest 型・
  正規化・predicates、tool vocabulary、JS 互換 canonical number
  formatter(整値は小数点なし・非有限は `null`)、`config/dirs` の
  user config dir 解決。
- **含まない**(後続 phase の境界): canonical tool 8 種・journal・
  scheduler(Phase 2 `cheaper-tools`)、run オーケストレーション・model
  catalog store(G1)・credential store(G2)・transient retry(G3)・
  devin provider(G4)(Phase 3)、envelope serializer・usage.jsonl・
  builtin resource `include_dir!` 埋込・`<userConfigDir>/pricing.json`
  読込層(Phase 4 `cheaper-cli`)。`resolvePricing` は user pricing.json
  層を持たない snapshot 注入可能な関数として port するだけに留める。
- API 形状の自明な差異: TS の `fs/promises` 由来の async 関数
  (`resolveWorker`/`loadPolicy`/`loadModelConfig`/`countFileTokens`/
  `normalizeResourceManifest`/`resolveWorkspacePath` 等)は Rust では
  `std::fs` を使う同期関数とする(`cheaper-core` は tokio 非依存の
  設計上の決定。設計書 crate 表の「Rig 非依存・tokio 非依存の純ロジック
  crate」に従う)。

共通事項:

- cargo 操作は `mise exec -- cargo ...` 経由(mise.toml に Rust toolchain
  を追加した後)。TS 側検証は `mise exec -- pnpm ...` 経由。
- 依存クレート追加は AGENTS.md の「Ask first: 依存の追加・変更」に従い、
  `crates/cheaper-core/Cargo.toml` の依存一覧確定時にユーザー承認を取る。
- TS 側 `packages/core` の実装は変更しない(追加するのは fixture dump
  機構のみ)。Phase 0 go 後の TS 新機能凍結ルールは親 plan 共通規則。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (SPEC-034BbgvL9s41K7NflOddZ6)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、共通規則)
- `docs/plans/phase-0-rig-poc-go-no-go-gate.md`(go/no-go ゲート、depends-on)
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  (ADR-034BbhP6hJV7DFtv7k7AGs)
- `docs/research/rig-agent-runtime/migration-plan.md` §3 Phase 1・§3.3
  golden 戦略

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md`
  (DESIGN-034Bbq1qY8WFBdL26tUdBw、approved) — crate 表の `cheaper-core`
  行・境界ルール・Windows path canonicalize 規則・envelope 数値
  フォーマット規則・Non-Functional Constraints・ISS-19(c)/(d)
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
  — ERROR_TABLE・policy schema/load 順・capability の元設計
- `docs/designs/model-resolver.md` —
  model 解決・tier/fallback の元設計
- `docs/designs/model-pricing-catalog-and-curated-exceptions.md` —
  `resolvePricing`・snapshot・curated exceptions の元設計
- `docs/designs/auto-tier-classification.md` —
  effectiveCost・percentile auto 分類の元設計
- `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md` —
  `resolveWorkspacePath` アルゴリズム 1–5 の元設計
- `docs/designs/token-counter-contract.md` —
  `docs/adr/canonical-tokenizer-js-tiktoken.md` と合わせて token 計測の元設計
- `docs/designs/core-resource-manifest-normalization.md` —
  resource manifest 正規化の元設計
- `docs/designs/cli-contract-envelope-exit-codes.md` —
  数値フォーマットの JS 互換要求の元設計

## File Map

### workspace / 生成物

- `Cargo.toml`(repo root、virtual manifest)- `members = ["crates/*"]`、
  workspace `edition = "2021"`(edition/依存 pin 方針の canonical 定義)
- `crates/cheaper-core/Cargo.toml` - lib crate。候補依存:
  `serde` + `serde_json`(`preserve_order` feature、ISS-19(d) の
  key 順序保持規則)、`serde_yml` or `serde_norway`(YAML、選定は実装時
  fixture で決定。設計書 ISS-19(c) は serde_yml 優先を指摘)、
  `jsonschema`(draft 2020-12、Ajv 代替)、`tiktoken-rs`(o200k_base)、
  `dunce`(verbatim prefix 除去)、`regex`(deny pattern・duration 等の
  小規模 pattern)
- `.gitignore` - `/target` 追加
- `mise.toml` - `[tools]` に Rust toolchain を追加(cargo/rustc を
  `mise exec` で有効化。`mise.lock` 更新を伴う)
- `scripts/dump-core-golden.mjs` - TS `@cheaper/core` を import して
  入力→出力(JSON Schema 化された worker/policy/models 解決結果・
  error code/details・duration・deny 判定・token 数・数値フォーマット)
  を JSON fixture として `crates/cheaper-core/tests/golden/fixtures/`
  に書き出す generator

### `crates/cheaper-core/src/`(移植元ファイル対応)

| Rust モジュール | 移植元 | 責務 |
| --- | --- | --- |
| `lib.rs` | `packages/core/src/index.ts` | 全モジュールの公開 re-export |
| `errors.rs` | `errors.ts` | `ERROR_TABLE` 写し(23 code→exitCode/retryable)、`CheaperError` 型(code・message・details・cause)、`ErrorCode` enum |
| `duration.rs` | `duration.ts` | `DURATION_PATTERN`(`^[1-9][0-9]*(ms|s|m|h)$`)+ `parse_duration`(ms 返却)。pattern 文字列は policy JSON schema の `pattern` property にも埋め込む共有定数 |
| `json/number.rs` | (新規、TS 側は `JSON.stringify` 組込) | JS `Number.prototype.toString` 互換の canonical number formatter。整値は小数点なし、`|x| >= 1e21`・`x < 1e-6` で JS 形式の指数表記(`e+`/`e-` 符号付き)、`-0`→`0`、非有限→`null`。serde `Serializer` 統合 helper。envelope/usage.jsonl/report の数値 field の唯一の実装(設計書「cheaper-core の serializer helper」) |
| `config/dirs.rs` | `config/dirs.ts` | `resolve_user_config_dir`(`CHEAPER_CONFIG_DIR`→`XDG_CONFIG_HOME/cheaper`→`<home>/.config/cheaper`、home は `USERPROFILE`→`HOME`→OS homedir 優先順)、`resolve_config_dirs`(`<cwd>/.cheaper` + 注入 `builtin_dir`)。テスト隔離のため env を引数注入する構造を維持 |
| `worker/parse.rs` | `worker/parse.ts` | 行単位 frontmatter 抽出(`---` 開始/終了、body 空拒否、YAML parse error→`INVALID_WORKER_DEFINITION` + reason 文字列写し) |
| `worker/schema.rs` | `worker/schema.ts` | `SchemaRef`(`{file}`/`{inline}` 排他)、`load_schema`(`.json`/`.yaml`/`.yml` 拡張子制限・`NOT_FOUND`/`INVALID_*_SCHEMA` 写し)、`compile_schema`(`$schema` が 2020-12 以外を拒否、`jsonschema` compile、error を `{instancePath,message}` 配列へ写像) |
| `worker/validate.rs` | `worker/validate.ts` | frontmatter 検証(NAME_PATTERN・stem 一致・strict keys `cheaper`/`model`/`workload`/`input`/`output`・tier・`outputPerInput` finite/>=0)、`WorkerDefinition` 型、`is_mutating_tool`・`is_directly_comparable_worker`、`validate_worker_input` |
| `worker/resolve.rs` | `worker/resolve.ts` | `resolve_worker`(project→user→builtin 優先順、`searched` 列挙)、`resolve_worker_file`(cwd resolve + explicit origin)、`INVALID_WORKER_NAME`/`WORKER_NOT_FOUND`/`WORKER_NAME_MISMATCH` |
| `worker/write_scope.rs` | `worker/write-scope.ts` | `extract_write_scope`(input field→string[]/`{path}`[]→literal path 集合、`INVALID_INPUT` reason 写し) |
| `policy/schema.rs` | `policy/schema.ts` | `POLICY_COMPLETE_SCHEMA`/`POLICY_PARTIAL_SCHEMA` 写し(`version: {const 1}`・`additionalProperties: false` 全階層・`propertyNames` pattern を含む)、`validate_policy_schema`、normalize 系(`ConcurrencyPolicy` 既定 `{maxOperations:8,mutations:"serialized"}`、`resolve_concurrency_policy` の safe-integer 検証)、`normalize_policy`、`normalizePolicyTools` 相当(tools allow の canonical/alias/重複検査) |
| `policy/merge.rs` | `policy/merge.ts` | `merge_policy_layers`(object deep merge・scalar/array 全体 replace)+ `restrict-concurrency` mode(数値 min 8 field 列挙・duration-min 3 field 列挙・`mutations` serialized 強制)を `serde_json::Value` 上で再現 |
| `policy/load.rs` | `policy/load.ts` | §23 ロード順(builtin complete→user partial merge→typed policy 補完→project partial `restrict-concurrency` merge→complete 検証→normalize)。ENOENT/ENOTDIR 相当のみ skip、その他 read 失敗は error(下記 Risks の errno 差分を参照) |
| `policy/restrict.rs` | `policy/restrict.ts` | `restrict_policy`(`--timeout` 超過→`POLICY_VIOLATION`、concurrency は不拡大方向のみ許可) |
| `policy/capability.rs` | `policy/capability.ts` | `check_capabilities`(requested tools ⊄ allow→`POLICY_VIOLATION`+missing details、tier>maxTier→`POLICY_VIOLATION`) |
| `policy/process.rs` | `policy/process.ts` | `PROCESS_POLICY_PROPERTIES` 写し、`resolve_process_policy`(既定 lifecycle 値・shell executable 拒否・argv shell syntax 拒否・workspace-relative path 検査・win32 env 大小文字同一視)、`resolve_process_task`、`process_task_resources`、`MAX_PROCESS_OUTPUT_BYTES`/`MAX_TIMER_MS` 定数 |
| `policy/workflow.rs` | `policy/workflow.ts` | `WORKFLOW_POLICY_PROPERTIES` 写し、`resolve_workflow_policy`(既定 artifact cap・safe-integer 検証) |
| `model/tier.rs` | `model/tier.ts` | `TIERS`・`compare_tier`・`is_tier`・`next_tier` |
| `model/model_ref.rs` | `model/ref.ts` | `model_key`・`match_model_pattern`(exact + `provider/*`)・`parse_model_ref`(最初の `/` のみ分割)。`ref` は Rust 予約語のためファイル名は `model_ref.rs` |
| `model/config.rs` | `model/config.ts` | `MODELS_CONFIG_SCHEMA` 写し、`load_model_config`(user→project 後勝ち merge、`exclude` のみ union 集約する分離規則を写し) |
| `model/classify.rs` | `model/classify.ts` | `ModelInfo` 型、`TierSource`/`AutoClassificationDetail`/`AutoTierContext` 型、`effective_cost`・`build_auto_tier_context`・`percentileTier` 相当(midrank percentile・整数演算の境界比較 `3(2·countBelow+countEqual)` vs `2n`/`4n` を写し)・`classify_model`/`classify_models`(優先順位 overrides>curated>auto>unclassified) |
| `model/exceptions.rs` | `model/exceptions.ts` | `MODEL_EXCEPTIONS` 写し(現行 `ollama/*`→low の 1 件)、`find_model_exception`(exact 優先・wildcard は key 辞書順で最初の一致) |
| `model/pricing.rs` | `model/pricing.ts` | `ModelPricing`/`PricingSnapshot`/`PricingSnapshotEntry` 型、`resolve_pricing`(runtime catalog `cost` 優先・snapshot pattern 探索の `Object.keys` 順序=挿入順規則・unpriced→`None`) |
| `model/resolve.rs` | `model/resolve.ts` | `resolve_model`(exclude→maxTier 検査→tier 別 classify→preferences 順序→cost 昇順→provider/id の決定論的順序・`tierFallback:"higher"` 逐次上昇・`candidatesPerTier` details)、`resolve_explicit_model`(exclude/availability/maxTier enforce・selection:"explicit")、`is_excluded_model` |
| `model/data/pricing_snapshot.rs` | `model/data/pricing-snapshot.ts` | bundled `PRICING_SNAPSHOT` 写し(version `2026-09-19`・`models` は現行空 map。snapshot 更新運用は TS 側と同じ手編集) |
| `token/count.rs` | `token/count.ts` | `tiktoken-rs` `o200k_base` で `count_text_tokens`/`count_file_tokens`(byte cap `DEFAULT_COUNT_MAX_BYTES`=16MiB・`FileTooLargeError` 相当型・utf8 lossy decode・`{path,tokens,bytes,tokenizer}` 構造)。`TOKENIZER_NAME="o200k_base"` は維持、`TOKENIZER_IMPLEMENTATION`/`TOKENIZER_VERSION` の値は Open Questions 参照 |
| `tools/vocabulary.rs` | `tools/vocabulary.ts` | `TOOL_NAMES` 8 種・`TOOL_ALIASES` 写し・`normalize_tool_names`(non-empty string array・alias 完全一致・`UNKNOWN_TOOL`・重複検出) |
| `workspace/paths.rs` | `workspace/paths.ts` | `canonicalize_workspace_root`(存在確認→canonicalize)、`is_inside_workspace_root`(win32 で大小文字非依存)、`resolve_workspace_path`(NUL 拒否→lexical `..`/`・` resolve→root 配下判定→component 順に lstat・symlink policy→最深存在祖先 canonicalize + 不存在 tail 連結→POSIX 区切り display)。`dunce` 相当の verbatim prefix 除去、`WorkspaceEscapeError` 相当の plain error 型(input 保持) |
| `workspace/deny_paths.rs` | `workspace/deny-paths.ts` | `DEFAULT_DENY_PATHS` 写し、`deny_pattern_to_regex` 相当の glob→regex 変換(anchor/`**`/basename 規則・win32 `i` flag)、`create_path_denylist`(`./` 除去・isDenied) |
| `resources/types.rs` | `resources/types.ts` | `ResourceScope`/`ResourceClaim`/`ResourceManifest` 型 |
| `resources/predicates.rs` | `resources/predicates.ts` | `resource_key`・`resource_contains`・`resources_overlap`・`claim_contains`・`claims_conflict`・`manifests_conflict`(win32 identity 小文字化を写し) |
| `resources/normalization.rs` | `resources/normalization.ts` | `ScopeDeclaration`/`ResourceManifestDeclaration`/`ResourceManifestInput` 型、declaration/canonical claim 検証・glob prefix 規則・ownership(produces)規則・bucket 混在拒否・dedupe 順序・`normalize_resource_manifest`/`normalize_resource_claim` |

### `crates/cheaper-core/tests/`(TS golden fixture 対応付け)

canonical fixture は `crates/cheaper-core/tests/golden/fixtures/` に
`scripts/dump-core-golden.mjs` が生成する JSON(リポジトリに commit)。
Rust 側は領域別の integration test として replay する。対応付け:

| Rust test | TS 側 test(`packages/core/test/`)/fixture 系 |
| --- | --- |
| `tests/golden/errors.rs` | `errors.test.ts` + `fixtures/errors.json`(全 code→exitCode/retryable) |
| `tests/golden/duration.rs` | `duration.test.ts` + `fixtures/duration.json` |
| `tests/golden/vocabulary.rs` | `tools/vocabulary.test.ts` + `fixtures/vocabulary.json` |
| `tests/golden/dirs.rs` | `config/dirs.test.ts`(env 注入パターンを fixture 化) |
| `tests/golden/worker_parse.rs` | `worker/parse.test.ts` + `fixtures/worker/parse-*.json`(入力 md + 期待結果/error reason) |
| `tests/golden/worker_schema.rs` | `worker/schema.test.ts` + `fixtures/worker/schema-*.json`(schema ref/拡張子/draft 拒否/validation error の instancePath 列) |
| `tests/golden/worker_validate.rs` | `worker/validate.test.ts`・`write-scope.test.ts` + `fixtures/worker/validate-*.json`(strict keys・tier・workload・name/stem・writeScope) |
| `tests/golden/worker_resolve.rs` | `worker/resolve.test.ts`(tmp dir 構築手順を Rust 側でも再現 + origin 優先順/searched 列挙を fixture 化) |
| `tests/golden/policy_schema.rs` | `policy/schema.test.ts`・`policy/concurrency.test.ts` + `fixtures/policy/schema-*.json`(complete/partial・process/workflow 検証) |
| `tests/golden/policy_merge.rs` | `policy/merge.test.ts` + `fixtures/policy/merge-*.json`(restrict-concurrency mode 含む) |
| `tests/golden/policy_load.rs` | `policy/load.test.ts` + `fixtures/policy/load-*.json`(層構造入力→normalize 済出力。built-in 実 file は `packages/cli/policy.yaml` を fixture として両側で読む) |
| `tests/golden/policy_restrict.rs` | `policy/restrict.test.ts`・`policy/capability.test.ts` |
| `tests/golden/policy_process.rs` | `policy/process.test.ts` + `fixtures/policy/process-*.json`(shell 拒否・argv・env 大小文字・cwd/inputs/outputs 検査) |
| `tests/golden/model_classify.rs` | `model/classify.test.ts`・`model/tier.test.ts`・`model/pricing.test.ts` + `fixtures/model/classify-*.json`(母集団・percentile 境界・同値・unpriced 除外) |
| `tests/golden/model_resolve.rs` | `model/resolve.test.ts`・`model/config.test.ts` + `fixtures/model/resolve-*.json`(tier/explicit・fallback・exclude・preferences 順序) |
| `tests/golden/token_count.rs` | `token/count.test.ts` + `fixtures/token/count.json`(代表文字列→token 数:ASCII/CJK/emoji/長文/invalid UTF-8 lossy/byte cap 超過) |
| `tests/golden/workspace_paths.rs` | `workspace/paths.test.ts`(tmp dir + 実 symlink を両側で構築)+ `fixtures/workspace/paths-*.json`(lexical 解決のみ fixture 化可能な分。OS 依存 case は `cfg(windows)` で分岐) |
| `tests/golden/deny_paths.rs` | `workspace/deny-paths.test.ts` + `fixtures/workspace/deny-*.json`(pattern→display path 判定行列) |
| `tests/golden/resources.rs` | `resources.test.ts` + `fixtures/resources-*.json`(declaration/claim/glob/ownership/dedupe/predicate 行列) |
| `tests/golden/number_format.rs` | (TS 側は `JSON.stringify` の出力を dump)+ `fixtures/number-format.json`(整数・小数・境界 `1e21`/`1e-6`・`-0`・非有限) |

## Implementation Sequence

- [ ] 1. `mise.toml` に Rust toolchain(`rust`)を追加し `mise install` で
  cargo/rustc を有効化、`mise.lock` を更新(依存・toolchain 追加の
  事前承認をユーザーに取ってから実施)
- [ ] 2. repo root に virtual manifest `Cargo.toml`(`members=["crates/*"]`)
  を作成し、`.gitignore` に `/target` を追加する
- [ ] 3. `crates/cheaper-core/Cargo.toml`(lib、`serde`/`serde_json`
  のみの最小依存)と `src/lib.rs` skeleton を作り
  `mise exec -- cargo build -p cheaper-core` が空で通ることを確認する
- [ ] 4. `errors.rs`・`duration.rs`・`tools/vocabulary.rs`・
  `config/dirs.rs` を移植し、対応する unit test を `src` 内 `#[cfg(test)]`
  または `tests/` に移植する
- [ ] 5. `workspace/paths.rs` を移植する(NUL 拒否・lexical `..`/`.`
  resolve・root 配下判定・component 順 lstat・symlink policy・最深存在
  祖先 + tail 連結・POSIX display・win32 大小文字非依存)。canonicalize
  は `dunce::canonicalize` 相当で verbatim prefix を除去する
- [ ] 6. `workspace/deny_paths.rs` を移植する(glob→regex 変換規則・
  anchor/`**`/`./` 除去・win32 `i` flag 相当)
- [ ] 7. `resources/` の 3 モジュールを移植する(型・predicates・
  normalization の glob/ownership/dedupe 規則を含む)
- [ ] 8. `worker/` の 5 モジュールを移植する(`worker/schema.rs` は
  `jsonschema` draft 2020-12 で compile・`$schema` 一致検査・error を
  `{instancePath,message}` へ写像。frontmatter の YAML parse は
  選定 YAML crate で行う)
- [ ] 9. `policy/` の 7 モジュールを移植する(complete/partial schema を
  `jsonschema` で compile し restrict/merge/load の層規則を写す)
- [ ] 10. `model/` の 8 モジュールを移植する(`resolve_pricing` の
  snapshot pattern 探索順序=挿入順を `preserve_order` で保証する)
- [ ] 11. `token/count.rs` を `tiktoken-rs` の `o200k_base` で移植する
  (byte cap・utf8 lossy decode・`FileTooLargeError` 相当)
- [ ] 12. `json/number.rs` に JS 互換 canonical number formatter を実装
  する(ryu 系の最短表現を JS 指数表記へ整形・`-0`→`0`・非有限→`null`)
- [ ] 13. `scripts/dump-core-golden.mjs` を実装し `crates/cheaper-core/
  tests/golden/fixtures/` に fixture を生成して commit する
- [ ] 14. `crates/cheaper-core/tests/golden/` の replay ハーネスを実装し
  File Map の対応表どおり fixture 全件を消化させる
- [ ] 15. `mise exec -- cargo test -p cheaper-core` が全 green、
  `cargo clippy -p cheaper-core --all-targets -- -D warnings` と
  `cargo fmt --check` が通る状態にする
- [ ] 16. `cargo tree -p cheaper-core` に `rig*`・`tokio` が含まれない
  ことを確認する(境界ルールの機械的検証)
- [ ] 17. Windows 環境で `cargo test -p cheaper-core` を実行し
  case-insensitive path・deny `i` flag・env 大小文字の `cfg(windows)`
  分岐が fixture 一致することを確認する(下記 Verification の CI 項参照)
- [ ] 18. `mise run check` が現行と変わらず全 green であることを確認する
  (TS 側を破壊していないことの検証)

## Dependencies and Risks

### 依存(確定候補、`Cargo.toml` 確定時にユーザー承認)

| crate | 用途 | 備考 |
| --- | --- | --- |
| `serde` + `serde_json` | 全 schema/merge/fixture の値表現 | `preserve_order` feature 必須(ISS-19(d):key 順序保持が `savings.returnedTokens` 系と snapshot pattern 探索順の規則) |
| `serde_yml` or `serde_norway` | worker frontmatter・policy.yaml・models.yaml の YAML 1.2 parse | ISS-19(c):serde_yml 優先、serde_norway は YAML 1.1 boolean 差に注意。実装時に builtin 実 file(packages/cli/policy.yaml・workers/*.md)と方言 fixture で決定 |
| `jsonschema` | JSON Schema draft 2020-12 compile/validate(Ajv 代替) | `instancePath` のみ契約。message・件数・code は非契約 |
| `tiktoken-rs` | `o200k_base` token 計測 | js-tiktoken との token 数一致を fixture で証明 |
| `dunce` | Windows `\\?\` verbatim prefix 除去 | `std::fs::canonicalize` の戻りをそのまま使わない設計書の固定規則 |
| `regex` | deny pattern 変換結果・`propertyNames`・duration・name pattern | deny glob→regex は自前変換で lookahead/backreference を使わないため `regex` crate で充足(grep tool の JS 方言問題は Phase 2 の論点で core スコープ外) |
| `tempfile`(dev-dep) | golden test の tmp dir/symlink 構築 | TS `test/fixtures/tmp.ts` 相当 |

### Risk Register

| # | リスク | 確率/影響 | 対策 |
| --- | --- | --- | --- |
| 1 | semver 方言差:Cargo の version requirement 記法と npm semver の差異(`=` pin・wildcard・0.x `^` 意味)。設計の「rig `=0.43.x` pin」は Cargo では `=0.43.*` と書く | 低/低 | workspace `Cargo.toml` の pin 方針を本 plan で確定し、core 依存は Cargo.lock commit で再現性を担保。rig 系の pin は Phase 3 以降の論点だが記法だけここで固定 |
| 2 | YAML 方言差:eemeli `yaml`(YAML 1.2)と serde_yml/serde_norway の差(plain scalar boolean・merge key・anchor・非文字列 key・重複 key)。worker frontmatter の `---` 抽出は行単位で先行するため影響は frontmatter 内値と policy/models.yaml に限定 | 中/中 | builtin 実 file + 方言 fixture(`.env` 風の `yes/no/on/off`・数値 key・anchor を含む case)を golden 化し両実装で一致を確認 |
| 3 | path 差分:Node `path.resolve` の lexical `..`/`.` resolve・drive-relative(`C:foo`)・UNC・trailing separator と Rust `Path` の差。`std::fs::canonicalize` は verbatim prefix を返す | 中/高 | 設計書の固定規則(dunce 相当・lexical resolve 自前・最深存在祖先+tail)をそのまま実装。deny/writeScope/display/`root` field の byte 表現を Windows fixture で固定。lexical のみ成立する case を `paths-*.json` に収録 |
| 4 | tokenizer 一致:js-tiktoken と tiktoken-rs の `o200k_base` rank table 一致、および `buf.toString("utf8")` vs `String::from_utf8_lossy` の invalid byte 置換粒度差 | 中/中 | `token/count.json` fixture に ASCII/CJK/emoji/surrogate 相当の invalid UTF-8 byte 列を収録し token 数の厳密一致を検証 |
| 5 | Ajv → `jsonschema` 方言差:Ajv `strict:true` は未知 keyword を拒否するが jsonschema は既定で無視。`allErrors` と error 列挙順・`instancePath` 表記(Ajv は JSON Pointer `"/a/b"`)の差 | 中/中 | `instancePath` のみを契約として fixture 比較(message・件数・code 非契約は spec 確定済)。Ajv strict 相当の未知 keyword 拒否を自前 pre-pass で再現するかは Open Questions で判断 |
| 6 | `localeCompare` 差分:`process.ts` の task/env ソートと `exceptions` の key ソートが ICU collation 依存で、Rust byte 順と完全一致しない可能性 | 低/低 | ASCII 範囲ではほぼ一致。契約上観測可能な順序は fixture で固定し、差が出る入力のみ明示的に扱う |
| 7 | `details.errno` 差分:TS は errno 文字列(`ENOENT` 等)を details に載せるが Rust `io::Error` は `ErrorKind`/`raw_os_error` 数値 | 低/低 | `ErrorKind` 名に写す方針とし、errno field を持つ error fixture では code/reason のみ契約として details の診断値差を normalize 対象に追加する(要 Review Handoff で確認) |
| 8 | `parseDuration` overflow:TS は f64 で巨大値を黙って返すが Rust `u64` では overflow error になる分岐点 | 低/低 | checked arithmetic + overflow→`invalid duration` 拒否で統一し fixture 固定。process policy の `MAX_TIMER_MS` 上限が実害範囲を既に閉じている(下記 Open Questions) |
| 9 | Windows 固有分岐(`process.platform === "win32"` → `cfg!(windows)`)が CI ubuntu で未実行のまま残る | 中/中 | Verification の CI 項で `windows-latest` job 追加を検証項目化し、未追加ならローカル Windows での実測を完了条件に含める |
| 10 | async→sync 化で呼び出し側の契約が変わる | 低/低 | core は tokio 非依存が設計確定。呼び出し側(Phase 2/3)が block を要求する場合の橋渡しはその phase の責務 |

## Task Breakdown

Implementation Sequence の各番号が 1 task 候補。結合・分割の規則:

- 4 は 4 モジュールとも leaf のため「leaf 移植」1 task にまとめてよい。
- 5+6・7・8・9・10 はモジュール群単位で独立 dispatch 可能(worker は
  `tools/vocabulary`+`model/tier` に依存、policy は `duration`+
  `vocabulary`+`tier` に依存、model は `policy/merge` に依存するため、
  4→9 完了後は {5,6}・{7}・{8}・{9}・{10}・{11}・{12} を並行化可能)。
- 13→14→15 は fixture 契約のため全モジュール移植後の直列 task。
- 16/17/18 は検証 task(完了条件)。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` 全 green(unit + golden
  replay)
- [ ] `mise exec -- cargo clippy -p cheaper-core --all-targets --
  -D warnings` と `cargo fmt --check` が green
- [ ] `node scripts/dump-core-golden.mjs` 再実行で fixture が再現され
  `git diff --exit-code -- crates/cheaper-core/tests/golden/fixtures`
  が clean(fixture 鮮度の継続確認)
- [ ] TS 側 golden との unit 差分:`tests/golden/*` replay が全 fixture
  と一致。schema 検証 error は `instancePath` のみ比較
  (message 文言・errno 診断値・tokenizer identity 文字列は normalize
  除外=spec の正規化規則を core 層で先取り適用)
- [ ] `pnpm` 側 fixture 対応確認:`pnpm --filter @cheaper/core test` が
  現行通り green で、dump 対象 API の TS test が fixture の入力集合と
  1:1 対応していること(File Map の対応表をレビューで突合)
- [ ] `cargo tree -p cheaper-core` に `rig*`・`tokio` が無いこと
- [ ] Windows:`windows-latest` runner(`.github/workflows/ci.yml` に
  `cargo test -p cheaper-core` job を追加するか、追加しない場合は
  ローカル Windows 実測)で paths/deny/tokenize の `cfg(windows)` 分岐
  fixture が一致
- [ ] `mise run check` が現行通り全 green(TS 側無破壊)

## Open Questions / Gaps

- `TOKENIZER_IMPLEMENTATION` の値:`"tiktoken-rs"` 表記 + crate version
  を載せるか、`"js-tiktoken"` 互換表示にするか。`result.tokenizer.*`・
  `details.tokenizer.*` は differential 比較の除外対象で最終決定は spec
  の Deferred(Phase 4)。core は定数を持つだけなので一旦
  `"tiktoken-rs"` + Cargo.toml version を採用する方針。
- YAML crate 最終選定:`serde_yml` を第一候補とし、builtin 実 file +
  方言 fixture の一致で決定。serde_norway の YAML 1.1 boolean 差が
  判明した場合は serde_yml に確定(ISS-19(c) 準拠)。
- `parseDuration` overflow:u64 checked で `invalid duration` 拒否
  (TS は f64 で巨大値を返すが `MAX_TIMER_MS`=2^31-1 を超える process
  policy 値は別経路で既に拒否。fixture で差分を固定する)。
- Ajv `strict` 相当の未知 keyword 拒否:`jsonschema` は既定で無視する
  ため、`worker/schema.rs`/`policy/schema.rs` の compile 前に既知
  keyword allowlist の pre-pass を自前で入れるか、差分を許容するか。
  builtin schema は全て検証済みなので user 入力 schema でのみ差が出る。
- `details.errno` の表現:`ErrorKind` debug 名への写しで診断性は保持。
  errno 文字列自体を契約とする必要があるかは Review Handoff で確認。
- cargo workspace 配置:本 plan は repo root `Cargo.toml` + `crates/`
  を採用する(設計書の `rust/`/`packages-rs/` 候補に対する確定。
  File Map が `crates/cheaper-core/` 前提のため。変更する場合は全
  phase plan の File Map に波及する)。

## Self-Review

- [x] Every spec requirement maps to a task.(Phase 1 スコープの全責務を
  File Map・Implementation Sequence に対応付け。envelope 数値
  formatter・ERROR_TABLE・path 規則・tokenizer は設計書の core 担務を
  そのまま写している)
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation(workspace 初期化後の
  モジュール移植は並行 dispatch 可)
- Independent work streams: step 4 完了後、{5,6} / {7} / {8} / {9} / {10}
  / {11} / {12} は相互独立。13-15(fixture 生成→replay→全検証)は
  全モジュール移植後の直列。16-18 は最終検証 task
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提確定事項(レビューで確認してほしい):
  - cargo workspace root を repo root `Cargo.toml` + `crates/` に確定
    した点(設計書は `rust/`/`packages-rs/` を併記しており、本 plan が
    確定を担う)
  - fixture canonical 配置を `crates/cheaper-core/tests/golden/fixtures/`
    にし、TS 側は `scripts/dump-core-golden.mjs` で生成する構造
  - `serde_json` の `preserve_order` を workspace 規則として入れる点
    (ISS-19(d) 由来)
  - `details.errno` 等の診断値差分を normalize 除外に追加する点
    (spec の正規化規則に無い追加分)
- レビュー重点:
  - `ERROR_TABLE`・`DEFAULT_DENY_PATHS`・`MODEL_EXCEPTIONS`・既定値
    (concurrency/process lifecycle/workflow artifacts/duration
    multiplier)の「写し」漏れ
  - `workspace/paths.rs` の lexical resolve・symlink 辿り直し・最深存在
    祖先規則が DESIGN-034r05iHrZUE6WsgJOy6Ny アルゴリズム 1–5 と
    `paths.ts` の実装両方に一致するか
  - `policy/merge.rs` の restrict-concurrency field 列挙と `worker/
    schema.rs` の `$schema` draft 2020-12 一致検査
  - `json/number.rs` の JS 指数表記境界(`1e21`/`1e-6`)と `-0`/非有限
    の扱い
- 移植元の正:`packages/core/src/` 全ファイル(責務把握済)。TS 側
  golden fixture は `packages/core/test/` の test case を dump に
  転記する形で収録する。

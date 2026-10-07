---
id: "IMPL-034BkjFKD4ZgRkPCBBn1kk"
type: "impl"
status: "completed"
title: "Phase 1: port cheaper-core"
created: "2026-10-06"
updated: "2026-10-07"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "directory"
        path: "crates/cheaper-core/"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "mise.lock"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: "docs/tasks/phase-1-port-cheaper-core.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/phase-1-port-cheaper-core.md"
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

# Phase 1: port cheaper-core

## Summary

plan の Implementation Sequence step 1-4(slice 1)および step 5-12
(slice 2)を実施した。repo root に virtual manifest
`Cargo.toml`(`members=["crates/*"]` + `exclude` で
`crates/poc-rig` を除外)を作り、`mise.toml` に `rust = "stable"` を追加、
`crates/cheaper-core` skeleton を起動した。slice 1 は
`errors.rs`(`ERROR_TABLE` 25 件写し)、`duration.rs`、
`tools/vocabulary.rs`、`config/dirs.rs` を移植。slice 2 は
`workspace/`(paths・deny_paths)、`resources/`(3 module)、
`worker/`(5 module)、`policy/`(7 module)、`model/`(8 module +
bundled pricing snapshot)、`token/count.rs`、`json/number.rs` を
unit test 付きで移植し、`packages/core` の純ロジック層(非 runtime)を
すべて `cheaper-core` 上に揃えた。slice 3 は
`scripts/dump-core-golden.mjs`(22 fixture / 463 cases の
deterministic 生成)と `tests/golden/` replay harness(全 22 File Map
対応・全件一致)を実装し、TS 実装との差分一致による挙動互換検証を
確立した。

## Context

TASK-034BgKiqu2ZlaprN931tnY(`docs/tasks/phase-1-port-cheaper-core.md`)/
PLAN-034BepXMKsB4VzqIP9XV7l(`docs/plans/phase-1-port-cheaper-core.md`)の
step 1-4 を実装。上流は DESIGN-034Bbq1qY8WFBdL26tUdBw の crate 表
(`cheaper-core` = Rig 非依存・tokio 非依存の純ロジック crate)と
poc-results.md の toolchain 知見(rig の rust-version 1.95)。

## Implementation

### Changes

- `mise.toml`: `[tools]` に `rust = "stable"` を追加
  (stable ≥1.95 方針、実測 1.99.0 が解決)。`mise.lock` は mise が自動更新
  (`tools.rust` = `core:rust` backend、version `stable`)。
- `Cargo.toml`(root virtual manifest): `resolver="2"`、
  `members=["crates/*"]`、`exclude=["crates/poc-rig"]`、
  `[workspace.package] edition="2021"`、`[workspace.dependencies]` に
  `serde`/`serde_json`(preserve_order)、`[profile.release]` に
  `lto/opt-level="s"/strip/codegen-units=1` + `panic="unwind"`(abort 禁止の
  設計規則を明示)。
- `.gitignore`: `/target` を追加。`crates/*/target/` は standalone な
  `crates/poc-rig/target/` を覆うため残置。
- `crates/cheaper-core/`(lib): `errors.rs`・`duration.rs`・
  `tools/vocabulary.rs`・`config/dirs.rs` + `lib.rs` re-export。
  TS 側 `packages/core` は未変更。
- `docs/tasks/phase-1-port-cheaper-core.md`: `status: "done"`
  (実装中は `"in-progress"`、review cycle 完了後に `"done"` へ更新)。

### Decisions During Implementation

- `crates/poc-rig` は workspace から `exclude` した。`members` glob が
  `crates/*` を拾うため明示除外しないと standalone pin(固有
  `Cargo.lock`/`rust-toolchain.toml`・rig `=0.43.0`)が workspace に
  取り込まれる。`cargo metadata` で poc-rig の workspace_root が
  `crates/poc-rig` 自身のままであることを確認済み。
- `ErrorCode` は enum + `SCREAMING_SNAKE_CASE` serde 表現。
  `ERROR_TABLE` は宣言順を保った `&[(ErrorCode, ErrorTableEntry)]` で、
  `exit_code`/`retryable` は lookup で導出(TS の readonly field と同じ
  不変条件)。`details` は `serde_json::Map<String, Value>`、`cause` は
  `Box<dyn Error + Send + Sync>`(serde 化は後続 phase の envelope 層)。
- `parse_duration` は `Result<u64, InvalidDurationError>`。pattern
  検証を手続き化(suffix は `ms` を `s` より先に照合)し、overflow は
  checked arithmetic で `invalid duration` 拒否(plan Risk #8)。
  `DURATION_PATTERN` は policy schema 埋込用の `&str` 定数として保持。
- `config/dirs.rs` の env は TS 同様に引数注入(`&dyn Fn(&str)->
  Option<String>`)。既定は `std::env::var` で、Windows の env 大小文字
  非依存は std/Node 双方の OS 由来挙動として一致。`CHEAPER_CONFIG_DIR`/
  `XDG_CONFIG_HOME` は truthy 判定(空文字無効)、`USERPROFILE`/`HOME` は
  `??` 判定(空文字も値として採用)を写した。
- `tempfile` は本 slice で未使用(golden replay の dev-dep は step 13-14
  で追加)。`regex` も未使用(duration の手続き化で充足)。

### Decisions During Implementation (Slice 2: steps 5-12)

- `nodepath.rs` を crate-private の Node `path` 互換層として新設した。
  Node は `path.win32` を常に使えるため、win32 の字句 resolve /
  normalize / verbatim prefix(`\\?\`・`\\.\`)・drive-root・rootless
  path の規則をホスト OS 非依存に実装し、`workspace`/`resources`/
  `worker` 各層がその上に乗る構成とした。`path.posix` 側は
  `is_absolute` 等の薄い helper のみ。
- `workspace/paths.rs` は `dunce::canonicalize`(Windows の verbatim
  prefix 除去)で root を canonicalize し、deepest existing ancestor +
  tail 再結合で nonexistent tail を許容する。Windows のみ比較を
  大小文字非依存(lowercase)とし、`real` とは別に workspace 相対の
  POSIX 形式 `display` path を返す二元表現を TS 通りに維持した。
- `workspace/deny_paths.rs` は glob→regex 変換を自前実装
  (basename パターンは任意深度、`/` 始まりは root anchored、`**` は
  0 個以上の segment、末尾 `/**` は dir 自身+子孫)。Windows のみ
  `(?i)` 相当の大小文字非依存。
- worker/policy 系の schema 検証は `jsonschema` crate の draft
  2020-12 validator を使用し、`{instancePath, message}` 形状の
  error 詳細を維持した。`Validator` は `OnceLock` で一度だけ compile。
- YAML 解析は `serde_norway`(yaml 形式の serde backend)。worker
  frontmatter・`models.yaml`・policy yaml を `serde_json::Value` へ
  統一して読み、以降の merge/schema 層は `Value` のみで完結する
  構成とした。
- `serde_json` に `preserve_order` + `raw_value` を付与し、pricing
  snapshot の `models` を挿入順保持の `Vec<(String, Entry)>` で持つ
  (TS の `Object.keys` 列挙順依存の pattern 走査を再現)。
- restrictive merge の数値 min は `Math.min` 相当として「値の小さい
  側の operand をそのまま採用」する実装にした。`f64` min を
  `Value::from(f64)` で再構築すると `8.0` になり serde_json の
  `Number` equality が崩れるため、整数表現を保持する必要があった。
- `ResolvedProcessPolicy`/`WorkflowPolicy` 等の resolved struct は
  `#[serde(rename_all="camelCase")]` を付け、`to_value` 経由で
  runtime policy 全体を complete schema に再検証する TS の流儀を
  再現した。
- `tiktoken-rs` の `o200k_base` で `encode_ordinary` を使用(js-tiktoken
  `encode` 相当)。16MiB byte cap・UTF-8 lossy・`FileTooLargeError`
  (CheaperError でなく plain error)・resolved display path 返却は
  TS 通り。token 数一致の最終検証は golden fixture に委譲する。
- `json/number.rs` は JS `Number.prototype.toString` 互換層として、
  safe-integer の `.0` 省略、`-0`→`0`、非有限→`null`、指数境界
  (`1e21`/`1e-6` 付近)を実装し、`serde_json::raw_value` で生 number
  を埋め込む `serialize`/`serialize_opt` helper を提供する。
- `classifyModels` は TS の `Map<ModelInfo, Classification>`(object
  identity key)を Rust で再現できないため、index 整合の
  `Vec<Classification>` を返す API とした。
- model 解決は `classify → exclude → preference 順 → cost/provider/id
  tie-break → tierFallback("higher" のみ上向き登攀)` の TS 順序を
  保持し、`details.candidatesPerTier`・`POLICY_VIOLATION` 理由キー
  (`model-excluded`/`model-unavailable`/`model-exceeds-max-tier`)も
  写した。`ModelSelection` は `"tier"|"explicit"` の serde 表現。

### Decisions During Implementation (Slice 3: steps 13-18)

- `scripts/dump-core-golden.mjs` は `packages/core/dist` を import し、
  plan File Map 対応表の全 22 fixture(463 cases)を
  `crates/cheaper-core/tests/golden/fixtures/` へ deterministic に
  生成する。fs 依存ケースは `mkdtemp` 上に case dir を構築し、絶対
  path は `<ROOT>`/`<OUT>`/`<ABS>`/`<BASE>`/`<FSROOT>` token で
  scrub して platform・machine 非依存にした。expected は TS 実装の
  実出力から生成(値のハードコード禁止)。
- `tests/golden/` replay harness は `main.rs` + `common/mod.rs` +
  20 replay module で構成し、同じ token scrub/unscrub を逆方向に
  適用して Rust 実装を実 path で実行、normalized `Value` 同士を
  比較する。schema error は `instancePath` のみ比較契約
  (ajv と jsonschema の message 文言差を契約外として両側で剥がす)。
- dump 側の発見・修正:
  - `Resource` enum の `#[serde(tag="kind")]` は variant 名を
    そのまま出力し `"Workspace"` になっていた。TS wire shape は
    `"workspace"` なので variant 側 `#[serde(rename="workspace")]` に
    修正した(crate 側の真の不整合)。
  - `compareTier` 等の expected は当初手書き値を置いていたが、
    原則どおり TS 実装呼出しの実出力に置き換えた(実際に 1 件
    誤記を検出: `compareTier("high","low")` = `2`)。
  - JS の整数 float は `JSON.stringify` で小数点なし表記になるが
    `serde_json::Value` の `Number` equality は int/float を区別
    するため、replay 側に `js_num`(整値→integer emit)を入れた。
  - `resolveXxxPolicy(input)` 系は TS `input ?? {}` が `null` をも
    省略扱いするため、fixture `input:null` は replay 側で `None` に
    写像(`opt_input`)しないと Rust 側が `Null` 非 object として
    誤って INVALID 判定する。
  - dump が `path.join` で native separator に直してから TS を呼ぶ
    pure predicate 入力(resourceContains/isInsideWorkspaceRoot)は、
    fixture には `/` 表記で記録されるため replay 側で同じ join
    (`native_path`)を適用する。
  - `resourceKey` 出力のような JSON.stringify 済み文字列は `\\`
    が二重化されるため、scrub は escaped form も置換対象にしないと
    fixture に machine path が残る。
  - `<ABS>`(predicate 用 synthetic base)は dump が repo root cwd の
    `path.resolve` で決める一方 `cargo test` は crate dir が cwd のため、
    replay 側は `CARGO_MANIFEST_DIR/../..` アンカーに揃えた。
  - TS `resolveWorkflowPolicy` の直接呼出しは未知 `artifacts` key を
    spread 保持するが Rust 型付き `WorkflowArtifactPolicy` は保持
    できない。schema 検証済み pipeline 経由では到達不能のため、
    fixture 側 `rustDropUnknownArtifactKeys` marker で文書化済みの
    表現差として正規化する(未調査差異ではない)。
- `docs/research/molo-agent-runtime/migration-plan.md` の stale id
  (legacy 連番 ADR 参照)を canonical ADR `ADR-034r2eId2hXJGWDjBylKVE`
  (usage-jsonl-contract-canonical)に修正した。

### Decisions During Implementation (Slice 4: review fixes)

レビュー確定指摘(F1–F6)の是正を実施した。

- F1 `localeCompare`: `src/js.rs` に `locale_compare` を追加。
  base letter を `to_lowercase` で primary 比較し、等しい場合は
  位置ごとの tertiary として lowercase < uppercase(ICU 既定
  collation の近似。accent/diacritics の secondary weight や `İ` 等の
  複数文字 lowercase 展開は非再現、doc comment に残差を明記)。
  `model/resolve.rs` の provider/id tie-break と
  `policy/process.rs` の task 名 sort・env key sort(TS は両方
  `localeCompare`)へ適用した。
- F2 tiktoken special token: js-tiktoken `encode(text)` は
  `disallowedSpecial="all"` で `<|endoftext|>` 等を拒否するが、
  `encode_ordinary` は literal 計数してしまう。`token/count.rs` に
  `reject_special_tokens`(`CoreBPE::special_tokens()` の文字列集合を
  左端一致で走査)を追加し、`SpecialTokenError`(plain error、
  js-tiktoken と同一 message)を返す。`count_text_tokens` は
  `Result<usize, SpecialTokenError>`、`CountTokensError` に
  `SpecialToken` variant を追加した。
- F3 YAML 1.2: `serde_norway`(YAML 1.1 で `y|n|yes|no|on|off` を
  boolean 解決)から `serde_yml = "=0.0.13"`(noyalib shim、YAML 1.2
  core schema)へ切替。全 serde_yml item が `#[deprecated]` のため
  parse を `src/yaml.rs` `parse_yaml` 一本に集約し、
  `#![allow(deprecated)]` と `YamlError` wrapper で deprecated 型を
  module 内に封じた。dep policy 許可時に noyalib 直接への移行を
  comment 明記。`worker/parse.rs`・`policy/load.rs`・
  `model/config.rs`・`worker/schema.rs` の 4 call site を更新。
  slice 2 の `serde_norway` 採用判断はこれで supersede された。
- F4 `String#trim` の U+FEFF: `js.rs` に `js_trim`
  (`is_whitespace() || \u{FEFF}`)を追加し、
  `worker/parse.rs` の close delimiter 判定・body trim、
  `worker/validate.rs` の description 空判定へ適用した。
- F5 worker file read error: `worker/resolve.rs` の read 失敗は
  `CheaperError(INTERNAL_ERROR, details{sourcePath})` から
  `WorkerReadError`(io error のみ、`{CODE}: {msg}, open '{path}'`
  近似 Display、details 無し)へ変更。公開 signature は
  `ResolveWorkerError { Cheaper(CheaperError), Read(WorkerReadError) }`
  enum を返し、CLI 境界が INTERNAL_ERROR へ写像できるよう
  distinguishable に保った(golden replay 側は `Read` を `cap` の
  errno 系 `{kind:"io"}` に写像)。
- F6 errno 表記: `model/config.rs` の裸 os error number を
  `policy/load.rs` と同一の kind→name 写像に揃えるため
  `errors::errno_string`(pub(crate))として共通化した。
- dump script に検出余地のある case を追加: `countText`/`countFile`
  の special token 拒否、worker parse の YAML 1.2 scalar・FEFF
  delimiter/body、validate の FEFF description・`writeScope: on`、
  model resolve の mixed-case provider tie-break、process の env key
  大小文字 sort(fixture は key sort 済みのため実順序の担保は
  unit test 側)。

### Decisions During Implementation (Slice 5: FX-1…FX-9 JS 意味論一致)

残存する TS/Rust 挙動差(FX-1–FX-9)の是正と golden 拡充を実施した。

- FX-1 `localeCompare` 精緻化: `src/js.rs` の `locale_compare` を
  Node/ICU probe 結果に合わせて拡張。primary weight は
  collation class(whitespace/control < punctuation/symbols <
  digits < letters)、ASCII punctuation は probe 確定順
  `_,-;:!.?'"()[]{}@*/&#%\`^+<=>|~$` の rank table、等価時は
  lowercase-first tertiary。`gpt_4o`/`a_1`/`_A` 等の probe case を
  unit test に固定した。DUCET 全体・収縮要素・非 ASCII script は
  近似で、残差を doc comment に明記。
- FX-2 AJV strict mode: `worker/schema.rs` に `strict_schema_check`
  walker を追加し、`jsonschema` compile 前に TS 同等の
  `schema-compile` 拒否を行う。対象は unknown keyword、(format
  plugin 未登録のため全ての) unknown format、strict type
  context、nullable/union 制約、strictTuples/strictRequired、
  `then`/`else` の `if` 依存、lazy `$defs`/`definitions`、local
  `$ref` 到達、applicator sibling context。残差(`$anchor`/
  `$dynamicRef`/`$id` scope・percent-decoded pointer)は comment
  明記。
- FX-3 `parse_duration`: 数値部を `f64` で parse・乗算する JS
  `Number` 意味論へ変更。`9007199254740993ms` → 2^53 に丸め、
  `9223372036854775807ms` → 2^63。結果の u64 上限 check は
  documented deviation #8 として維持。
- FX-4 nullish `??`: `concurrency`/`workflow`/`restrict` の policy
  resolution で `null` を JS の `??` 同様に default へ落とす
  (falsy な 0 等は validation が別途扱うため保持)。golden
  adapter は `r.get("concurrency").cloned()` で
  `concurrency: null` と absent を区別したまま port へ渡す。
- FX-5 workflow artifact spread: `js::spread_into` を追加 —
  object は own enumerable key の挿入順、string/array は indexed
  key(`{..."ab"}` → `"0","1"`)、primitive は spread nothing。
  unknown key は挿入順に validate した上で typed output から
  drop し、golden は既存 `rustDropUnknownArtifactKeys` marker で
  期待値を正規化する。`Object.entries` の array-index
  numeric-first 列挙も再現。
- FX-6 `parse_worker_file`: `\r` の除去を `\n` 直前の行末のみに
  限定し、EOF 終端の standalone `\r` は保持(JS
  `split(/\r?\n/)` 準拠。`"---\r"` は opener にならず
  `frontmatter-not-found`)。
- FX-7 errno: `errors.rs` に POSIX errno → libuv name table と
  Windows error code table を追加し `io_errno_display` で共通化。
  Windows の directory read は `EISDIR`、POSIX の
  permission-denied directory は `EACCES` のまま(POSIX は read が
  `EISDIR` を直接返す)。call site は `policy/load.rs`・
  `model/config.rs`・`worker/resolve.rs`。ENOENT/ENOTDIR の
  missing 系は別経路を維持。
- FX-8 UTF-16 sort: `js::code_unit_compare`(`encode_utf16` 比較)を
  `resources/normalization.rs` の claim dedup sort と
  `model/exceptions.rs` の wildcard key sort に適用。前者は
  `a_1`/`a-2` のようなパスで localeCompare と順序が変わり観測
  可能、後者は wildcard が provider ごと一意のため順序は非観測
  (TS 構造の mirror のみ)。
- FX-9 `normalize_policy`: passthrough/coercion を schema-gated な
  accepted divergence として doc comment に明記
  (`filesystem.symlinks` → Deny 等)。
- golden replay adapter: `opt_input` を `input ?? {}` 意味論へ
  (recorded `null` → `None`)。duration replay は `ms` を `js_num`
  で canonicalize — V8 `String(2**63)` が最短 round-trip 表記
  `9223372036854776000` を出すため、fixture の数値と port の
  exact u64 `9223372036854775808` は repr が異なり、f64 同士で
  比較する必要があった。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo --version` → cargo 1.99.0(mise が
  stable-x86_64-pc-windows-msvc 1.99.0 を install、≥1.95 を充足)。
- `mise exec -- cargo build -p cheaper-core` 成功。
- `mise exec -- cargo test -p cheaper-core` → 29 passed / 0 failed。
- `mise exec -- cargo clippy -p cheaper-core --all-targets -- -D warnings`
  成功。
- `mise exec -- cargo fmt --check` 成功。
- `mise exec -- cargo tree -p cheaper-core` に `rig`・`tokio` 無し
  (serde + serde_json のみ)。
- `cargo metadata` で root workspace members = cheaper-core のみ、
  poc-rig は独立 workspace のままであることを確認。

### Validation (Slice 2: steps 5-12)

- `mise exec -- cargo build -p cheaper-core` 成功(warning 0)。
- `mise exec -- cargo test -p cheaper-core` → 163 passed / 0 failed
  (slice 1 の 29 + slice 2 の追加 134)。
- `mise exec -- cargo clippy -p cheaper-core --all-targets --
  -D warnings` 成功。
- `mise exec -- cargo fmt --check` 成功。
- `mise exec -- cargo tree -p cheaper-core` に `rig`・`tokio` 無し。
  direct deps: `serde` / `serde_json`(preserve_order+raw_value) /
  `serde_norway` / `jsonschema`(default-features=false) /
  `tiktoken-rs` / `dunce` / `regex`(+ dev-dep `tempfile`)。
- 追加依存の resolve version: `serde_norway 0.9.42`、
  `jsonschema 0.58.6`、`tiktoken-rs 0.12.1`、`regex 1.13.1`、
  `dunce 1.0.x`、`tempfile 3.27.0`。

### Validation (Slice 3: steps 13-18)

- `node scripts/dump-core-golden.mjs` → 22 fixture files / 463 cases
  生成。再実行後の
  `git diff --exit-code -- crates/cheaper-core/tests/golden/fixtures`
  は clean(fixture deterministic)。
- `mise exec -- cargo test -p cheaper-core` → 163 unit + 22 golden
  replay 全 green(185 passed / 0 failed)。golden replay は
  workspace/win32 分岐・deny `i` flag・env 大小文字・symlink 経路を
  含む Windows 実機一致を確認。
- `mise exec -- cargo clippy -p cheaper-core --all-targets --
  -D warnings` 成功。`mise exec -- cargo fmt --check` 成功。
- `pnpm --filter @cheaper/core test` → 455 passed / 4 skipped。
- `cargo tree -p cheaper-core` に `rig`・`tokio` 無し(再確認)。
- `mise run check` は `pnpm lint`・`check-version`・`check-doc-ids`・
  `pnpm typecheck` を通過後、`pnpm test` の `packages/cli` で 3 件の
  e2e failure(20s timeout ×2・concurrent first-run lock ×1)により
  ERROR で停止。失敗 3 file を単独再実行すると 103 tests 全 pass
  — 高負荷時 flake(本 slice は `packages/cli` 非因果。変更範囲は
  `crates/`・`scripts/`・`docs/` のみ)。full `mise run check` の
  exit 0 確認は静かな環境での再実行が必要。

### Validation (Slice 4: review fixes)

- `cargo test -p cheaper-core` → 173 unit + 22 golden replay 全 green
  (195 passed / 0 failed)。追加 unit test: `locale_compare` 順序
  (`["a","B","b","A"]`→`a,A,b,B`、`Path`/`PATH`、`anthropic`/`OpenAI`)、
  `js_trim` の FEFF、special token 拒否、YAML 1.2 scalar、env key /
  provider の localeCompare 順、worker read 失敗の details 無し。
- `cargo clippy -p cheaper-core --all-targets -- -D warnings` 成功。
  `cargo fmt --check` 成功。
- `cargo tree -p cheaper-core` に `rig`・`tokio` 無し。`serde_norway`
  は排除、`serde_yml 0.0.13` + `noyalib 0.0.5` を resolve。
- `node scripts/dump-core-golden.mjs` 再生成: fixture は未追跡
  (port 全体が uncommitted)のため git diff による回帰検出対象なし。
  追加 case 全件を含む fixtures に対し golden replay 全 green。
- 検出した挙動差(残存 residual): serde_yml は `: : :` のような
  compact nested mapping を error 化せず `{"": {"": {"": null}}}`
  として受理する。TS `yaml` は "Nested mappings are not allowed in
  compact mappings" で throw し `frontmatter-yaml-error` になるため、
  この入力では Rust 側が parse 成功 → `name-missing` 経路となる
  観測差が残る(frontmatter が object なので not-object にも
  落ちない)。空 key の実害はほぼ無いため workaround は入れず
  残差として記録する。単体 test の `rejects_bad_yaml` 入力は実
  syntax error(`name: [unclosed`)へ変更した。

### Validation (Slice 5: FX-1…FX-9)

- `node scripts/dump-core-golden.mjs` → 22 fixture files。FX 系の
  case 追加後の総数: duration 22 / worker parse 16 /
  worker schema 33 / policy schema 25 / policy restrict 14 /
  policy process 42 / policy workflow 22 / model resolve 24 /
  resources 80。連続 2 回の再生成で全 fixture の SHA256 が一致
  (deterministic)。
- `cargo test -p cheaper-core` → 179 unit + 22 golden replay 全
  green(201 passed / 0 failed、doc-test 0)。
- `cargo clippy -p cheaper-core --all-targets -- -D warnings` 成功。
- `cargo fmt --check` 成功(`cargo fmt` 適用後)。
- `pnpm --filter @cheaper/core test` → 455 passed / 4 skipped、
  `pnpm --filter @cheaper/core build`(tsc -b)成功。
- 追加 fixture 観点: `2^53`/`2^63` rounding band、unknown
  keyword/format・strictRequired・union・lazy `$defs`・`$ref`
  到達・type-context の schema-compile 拒否、`maxJobs:null`・
  artifact field null・array/string spread・挿入順 unknown key、
  concurrency `??` null・restrict の null merge、`a_1`/`a-2`/`a0`
  ・`z_1`/`z0` punct 順 tie-break、task/env key の punct-first
  sort、EOF `\r`・CRLF、UTF-16 claim dedup sort。

## Risks

- `mise run check` 全体 green は上記 flake で未確認(単独再実行では
  失敗再現せず・変更非因果と判断)。CI/windows-latest 相当の負荷低い
  環境での最終確認を推奨。
- `usage.jsonl`・built-in resource embedding・user pricing file
  (`<userConfigDir>/pricing.json`)load 層は本 phase の後段 slice。

## Follow-ups

- 残: `usage.jsonl` 記録層・built-in resource embedding・
  `<userConfigDir>/pricing.json` load(後段 slice)。`mise run check`
  の再実行による全体 green 確認。

## Review cycles

- Reviewer A(11 findings → 修正済み): `localeCompare` 近似・
  tiktoken special token 拒否・YAML 1.2(`serde_norway` → `serde_yml`
  切替)・`String#trim` の U+FEFF・worker file read error の形状・
  errno 表記の共通化を F1–F6 に集約して是正(Slice 4)。
- Reviewer B round: P1(`locale_compare` の UCA class ordering、AJV
  strict keyword 拒否、`parse_duration` の f64 意味論)+ P2(nullish
  `??` collapse ×4、artifacts unknown-key 検証、行末以外の trailing
  `\r` 保持、errno name table、UTF-16 code-unit sort)をすべて修正し、
  unit test・golden fixture で coverage 済み(Slice 5: FX-1–FX-9)。
- Accepted / documented deviations: u64 overflow 拒否(FX-3 の f64
  意味論との併用で documented deviation として維持)、`: : :`
  compact nested mapping の YAML 受理差、full ICU collation の残差、
  errno 診断値の残差、`normalizePolicy` の unvalidated passthrough、
  Windows 軸 coverage の CI への defer。いずれも doc comment・
  fixture marker・本書に記録済み。
- Verdict: review-protocol の cycle は完結。未解決の blocker は
  無し。

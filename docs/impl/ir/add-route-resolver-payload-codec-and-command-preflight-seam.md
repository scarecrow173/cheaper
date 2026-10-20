---
id: "IMPL-034Jj3NXbe1CLVRdkxXF6K"
type: "impl"
status: "completed"
title: "Add route resolver payload codec and command preflight seam"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "docs/tasks/add-route-resolver-and-payload.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/payload.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "docs/test-specs/route-resolution.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-resolver-and-payload.md"
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

# Add route resolver payload codec and command preflight seam

## Summary

`route::resolve`(純粋評価の route match + viability 分類)、
`route::payload`(rewrite payload の base64url codec)、共有
preflight seam `check_command_execution`、および dry-run utility
command `cheaper route resolve` を実装した。viability は固定優先順
(unsafe-command → invalid-route → missing-target → no-grant →
stdin-unsupported → runnable)で 1 値を返し、grant/preflight 評価は
`command::execute` と同一 predicate を共有する。

## Context

task `add-route-resolver-and-payload` (B2)。approved design
`DESIGN-034JbqoCWm85pKC033fgfd` の §RouteResolver・§RoutePayload・
§viability(B#3 共有 seam)に従う。upstream は spec
`SPEC-034J7SxWvGL3oSbeBpo8a3`(R4-R12・R18-R20)と ADR
`ADR-034J8eZHNPqARi25HLFOL6`。

## Implementation

### Changes

- `cheaper-tools/src/tools/command/mod.rs` — `pin_command_grants`
  から単一 grant 版 `pin_command_grant` を抽出し、spawn 無し
  preflight `check_command_execution` + `CheckedCommand` を追加。
  `command::execute` は同関数へ委譲するよう改修し、resolver と
  execute が同一 predicate を共有する構造にした(pin 未構築経路
  は declared executable のその場解決 — 同一 pin predicate)。
- `route/resolve.rs` — `ResolveInput{argv,cwd,routes,
  command_policy,unsafe_command}` の純粋評価。env-prefix strip・
  self-bypass(`cheaper`)・longest match・同長 tie の
  invalid-route・`INTERACTIVE_COMMANDS`/`INTERACTIVE_FLAG_COMMANDS`
  の stdin-unsupported・suppressed provenance を実装。`args`
  (post-prefix)と `exec_args`(`argv[1..]`、spawn/grant 照合基準)
  を区別する。
- `route/payload.rs` — JSON → unpadded base64url の encode/decode
  + schema 検証 + `passthroughEnv` allowlist 絞り込みの唯一の
  owner。env-prefix 値は transport せず `payload.command` から
  `route run` が再導出する(design C3)。
- `route_cmd.rs` + `args.rs` + `cli.rs` — `route resolve` arm。
  `--command`/`--argv` は相互排他・どちらか必須、flag allowlist、
  `record_usage` 非呼出の utility command 規約。`--command`/
  `--argv` は route 専用 option として他 command arm で reject。
- `hook/routing_policy.rs` — `is_env_prefix`・`command_name`・
  `CWD_CHANGING_COMMANDS` を `pub(crate)` 化し resolver と共有
  (規約の複製を避ける)。

### Decisions During Implementation

- `ResolveInput.unsafe_command` — `--command` 入力の複数
  segment/unsafe token は argv 形では表現できないため、caller が
  tokenize 結果から立てる flag として入力に含めた(hook eval は
  segment 安全性を事前評価するため `false`)。
- no-match でも正規化 argv/`executable` を返す — dry-run の
  report 性を優先。route/target/grant/viability は no-match で
  省略(R11)。
- test spec `route-resolution.md` の「env-prefix を含む argv は
  route 対象外」は design C3(env-prefix command は route 対象、
  run 時に `payload.command` から再導出)と矛盾していたため、
  design に合わせて test spec 文言を修正した。
- base64 は workspace 共有依存(`base64 = "0.22"`)の宣言 reuse —
  新規外部依存・version 変更なし。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli --lib route::` — 16 pass(resolve
  viability 全分岐・longest match・env-prefix/self-bypass・payload
  round-trip/allowlist/malformed 拒否)。
- `cargo test -p cheaper-cli --test integration` — 226 pass(新規
  `route::` 5 test: R11 shape・no-match 省略・env-prefix 正規化・
  no-grant・usage 非記録 + `args::` route parse 4 test)。
- `cargo test -p cheaper-tools --test proc_command` — 33 pass(新規
  `check_command_execution_shares_execute_predicate`)。
- `cargo check --workspace --tests --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — clean。
- `cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — warning 無し。
- `cargo fmt --all` — 適用済み。

## Risks

- `INTERACTIVE_COMMANDS` は curated list で網羅性を持たない —
  未登録の stdin 依存 command は runnable と分類され得る(設計上
  許容される best-effort 分類。`passthroughEnv` 未宣言でも
  stdin 非接続は構造保証される)。
- `tied`(同長 match.command)は effective set では同 key 衝突を
  loader が隔離済みのため防御的検査 — 現状到達不能想定。

## Follow-ups

- C1-C4: hook eval への resolve 組込み・rewrite target render・
  `Verdict::Rewrite`。
- D1: `route run` — payload decode → 再 resolve → in-process
  workflow 実行。
- B4: `kind:"route"` usage record。

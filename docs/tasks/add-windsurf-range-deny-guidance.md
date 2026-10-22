---
id: "TASK-034KxDm7X0zR41q82TJxVY"
type: "task"
status: "done"
title: "Add windsurf range-drop deny guidance and fix deny-reason assertions"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/161"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
    - "docs/tasks/add-windsurf-gated-pattern-search-normalization.md"
  blocks:
    - "docs/tasks/document-windsurf-tool-info-contract-and-registration-requirements.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md"
  references:
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/adr/defensive-compat-normalization-with-registration-surfacing.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
  defers: []
  deferred-by: []
---

# Add windsurf range-drop deny guidance and fix deny-reason assertions

## Work

plan `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
(PLAN-034KxCQ2FWFt55qhBNp6sR)の T3。windsurf target で
`FileRead{range:None}` が `Deny` となった場合に、deny reason へ
compat firing の範囲不達の可能性と `cheaper hook-config verify`
誘導を付記する。design(DESIGN-034KwO2y86CrXOmaoae1jS)
「§`hook::targets` — windsurf FileRead deny への remediation
付記」をそのまま実装する。

### 1. `WINDSURF_RANGE_DROP_GUIDANCE` 定数 + `windsurf_range_deny_guidance`

文言(reason 文言規約に合わせ英文・1 行追加、design 固定):

```text
If this firing reached the hook through a compatibility registration
(e.g. a legacy `.windsurf/hooks.json` entry under Devin CLI), range
fields may not have been forwarded — run `cheaper hook-config verify`
to inspect the registration state.
```

新規 helper `windsurf_range_deny_guidance(request, verdict)`:

```rust
match (request, verdict) {
    (RoutingRequest::FileRead { range: None, .. }, Verdict::Deny { reason }) =>
        Verdict::Deny { reason: format!("{reason}\n{WINDSURF_RANGE_DROP_GUIDANCE}") },
    (_, v) => v,
}
```

付記は `request` が `FileRead{range:None}` で verdict が `Deny` の
場合のみ — `range:Some` の deny(ranged window 超過)、`Search`/
`Shell` の deny、Allow、Rewrite fallback(FileRead request は
route arm を通らないため Rewrite にならない)、他 target では
従来どおりの reason。

### 2. dispatch arm 接続

`handle_hook` の windsurf arm(`targets.rs:780-786`)で
`evaluate_with_routes` 後に付記する — target 情報を持つのはこの
層であり、eval 層(`routing_policy`)や dedup 層に target 判定を
持ち込まない(ADR「実装時に最小差分で決める」の決定を dispatch
側付記で固定):

```rust
let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
let verdict = windsurf_range_deny_guidance(&request, verdict);
(verdict_name(&verdict), false, windsurf_render(verdict))
```

dedup verdict record(`eval_cache`)には mutate 前の生 verdict が
persist される — guidance は render 直前の firing 毎付記であり、
cache 経由で serve される verdict 文言に混入しない。firing 経路で
config・登録状態は読まない(registered ≠ fired の fail-open を
避ける — ADR Option C 棄却理由)。

実 Windsurf 環境の `range:None` deny にも同じ行が載るが、条件付き
表現("If this firing reached the hook through a compatibility
registration")で実害は軽微(ADR Consequences 受理済み)。

### 3. deny 文言 assert の追随修正

guidance 末尾追加で破損する検証 2 箇所を更新する:

- `tests/integration/hook.rs:300` —
  `deny.stderr.ends_with("workspace-relative path: big.txt\n")` は
  guidance 行が末尾へ付くため成立しなくなる → `contains` 系
  assert へ変更。
- `tests/regression/expected/cli-hook-windsurf-deny.json` — case
  定義 `tests/regression/cases/cli-hook-windsurf-deny.json` が
  `compareStderr:true`・`mode:"raw"` で stderr を byte-exact pin
  しているため、`CHEAPER_REGRESSION_CAPTURE=1` で再 capture し
  diff を review(guidance 行の追加のみであることを確認)。

### 4. integration test(`tests/integration/hook.rs`)

- windsurf `file_path` のみ(range 無し)payload の Deny stderr が
  `cheaper hook-config verify` 誘導を含むこと。
- windsurf `range:Some` の deny(ranged window 超過)・Search/
  Shell deny には誘導行が載らないこと。
- devin/pretooluse の `range:None` deny には誘導行が**載らない**
  こと(他 target の reason 不変)。
- dedup 経由: 同一 firing の 2 回目(cache serve)でも生 verdict
  文言に guidance が混入しないこと(store される verdict が mutate
  前)。※`hook_eval_dedup` の既存規約と照合し、pin 可能なら追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
  (PLAN-034KxCQ2FWFt55qhBNp6sR — T3)
- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS — 「§`hook::targets` — windsurf
  FileRead deny への remediation 付記」・Non-Functional
  Constraints「deny reason の additive 変更のみ」が AUTHORITATIVE)
- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM — R5)
- `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
  (TSPEC-034KxD0IRkMph8hSZbhTRx — 付記条件・dedup 非混入の Rules)

## Dependencies

- `docs/tasks/add-windsurf-gated-pattern-search-normalization.md`
  (T2) — 同一 `targets.rs`・同一 `hook.rs` の直列後続。

## Affected Files

- `crates/cheaper-cli/src/hook/targets.rs`(modify —
  `WINDSURF_RANGE_DROP_GUIDANCE`・`windsurf_range_deny_guidance`・
  `handle_hook` windsurf arm)
- `crates/cheaper-cli/tests/integration/hook.rs`(modify — guidance
  fixture + `:300` `ends_with`→`contains` 修正)
- `crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json`
  (modify — `CHEAPER_REGRESSION_CAPTURE=1` 再 capture)

## Done When

- [x] windsurf `FileRead{range:None}` Deny の stderr に
      `WINDSURF_RANGE_DROP_GUIDANCE` 行が含まれる
- [x] `range:Some` deny・`Search`/`Shell` deny・Allow・他 target
      (devin/pretooluse)の reason が不変
- [x] dedup verdict record に mutate 前の生 verdict が persist
      される(cache serve 時の文言に guidance が混入しない)
- [x] `hook.rs:300` の `ends_with` assert が `contains` 系へ修正
      されている
- [x] `cli-hook-windsurf-deny.json` snapshot が再 capture され、
      diff が guidance 行追加のみであることを review 済み
- [x] firing 経路に config/登録状態の新規 read が無い

## Out of Scope

- `windsurf_read_range`・`pattern` gate・`normalize_search`(T1/T2
  で済み)
- `hook_config.rs` の finding 追加(T4)
- `routing_policy.rs`・`eval_cache.rs` の変更(不変)
- guidance を Devin context へ絞り込む env marker 識別(deferred
  design concern)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0(guidance fixture + 修正済み assert + 既存非回帰) — 実測: 29 passed / 0 failed(新規 `windsurf_range_deny_guidance_conditions_and_dedup`)
- [x] `CHEAPER_REGRESSION_CAPTURE=1 mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test regression` で snapshot 再生成 → その後 capture 無しで同 test が exits 0 — 実測: 両実行とも `regression_golden` 1 passed。recapture diff は `cli-hook-windsurf-deny.json` への guidance 1 行追加のみ(同時 recapture で発生した `cli-usage-empty`/`cli-usage-seeded` の field order churn は本 task 範囲外として revert 済み)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

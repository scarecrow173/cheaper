---
id: "TASK-034KxDr0eOqS1QjXxMazEk"
type: "task"
status: "done"
title: "Add hook-config compat-degraded and windsurf-conflict verify findings"
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
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
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
    - "docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
  defers: []
  deferred-by: []
---

# Add hook-config compat-degraded and windsurf-conflict verify findings

## Work

plan `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
(PLAN-034KxCQ2FWFt55qhBNp6sR)の T4。`hook_config.rs` の
`run_verify`(:591-752)に compat 縮退状態を説明する informational
finding 2 種を追加し、`legacy-conflict:` の dedup 文言を訂正する。
design(DESIGN-034KwO2y86CrXOmaoae1jS)「§`hook::hook_config` —
compat-degraded / windsurf-conflict verify finding」をそのまま実装
する。`own_state`(:625-670)と `legacy_conflicts` 結果(:673-685)
の両方が確定した後(`legacyConflicts` insert 直後)に評価する。

### 1. windsurf 系登録の識別 + import 無効化判定

- **windsurf 経路の識別**: `conflicts` の path(separator を `/`
  へ normalize して比較)が `.windsurf/hooks.json` で終わるものを
  windsurf 経路とみなす(`KNOWN_LEGACY_FILES` :42-49 の中で
  `.windsurf/hooks.json` のみが windsurf 経路 — `legacy_conflicts`
  が path 文字列のみを返す現契約を保ち、構造化への変更は行わ
  ない)。
- **import 無効化の検出**: loaded config(`loaded` が `Some` の
  場合のみ判定可能)の `root.read_config_from.windsurf` が明示の
  JSON `false` なら windsurf import が無効化されており、両 finding
  とも emit しない(実 Windsurf 専用の登録であり、既存
  `legacy-conflict:` finding が引き続き residual を報告する)。
  `read_config_from` key 欠落・非 object・`windsurf` が `false`
  以外の値は全て「無効化されていない」として扱う — `false` のみ
  を抑止条件とする保守判定。`loaded` が `None`(config 不在・
  読取不能)も「無効化なし」扱い。

### 2. finding 1 — `compat-degraded`(informational)

`own_state` が `"absent"` または `"missing"`、かつ windsurf
conflict があり、かつ import が無効化されていない場合に emit。
文言は design 固定形をそのまま使う(要点: 「native Devin
registration could not be confirmed」+ compat 経路では bounded
evaluation が発動しない + Devin 利用時は `hook-config sync` で
native 登録後に legacy 除去または `read_config_from` で windsurf
import 無効化 + 実 Windsurf install では削除しない旨の hedge)。

informational とし `blocker`/`clean`/exit code に影響しない —
absent/missing 状態では既存 `absent-config:`/`missing:` finding が
既に blocker で exit 2 のため、本 finding は説明追加のみ。

### 3. finding 2 — `windsurf-conflict`(informational)

`own_state` が `"absent"`/`"missing"` 以外(`present`/`drifted`/
`modified`/`duplicate`)、かつ windsurf conflict、かつ import
無効化でない場合に emit。文言は design 固定形をそのまま使う
(要点: legacy `.windsurf/hooks.json` 登録が native entry と併存
発火し続ける + fingerprint が `range` を含むため compat firing は
native firing と dedup されず range-dropping compat deny が verdict
union で勝つ + legacy 除去または import 無効化の remediation +
実 Windsurf install では残す旨の hedge)。

これは remediation loop の false-green を塞ぐもの — `sync` で
native 登録を足しても legacy を残す限り `compat-degraded` は消える
が dual-registration trap(`eval_cache.rs:436-438` の fingerprint
が `range` を含む)は存続する。本 finding が trap の存続を報告
し続ける。severity は informational — blocker にすると
`post-install`/`post-update` lifecycle の verify code 透過
(conflict 残余を exit 0 で返す既存 contract、`legacy-conflict`
の informational 規約)を破壊する。

### 4. `legacy-conflict` 文言の訂正

`hook_config.rs:674-681` の「duplicate firings are deduplicated at
eval time for identical requests」は、range を落とす compat firing
に対しては偽 — fingerprint が `range` を含むため range-diverged
した firing は dedup されない。文言を「dedup は同一 canonical
request(fingerprint が一致する firing)に限り発動し、range field
を落とす compat 経由の firing は dedup 対象外」方向へ訂正し、
dual-registration trap の説明は `windsurf-conflict` finding と
distribution.md 側へ集約する(発火条件・informational 規約・
exit code は不変)。

### 5. integration test(`tests/integration/hook_config.rs`)

既存 `write_legacy_conflict`(`.windsurf/hooks.json` に cheaper
登録)・`write_config_with_groups` fixture を再利用:

- absent(config 不在)+`.windsurf` conflict → `compat-degraded:`
  finding + exit 2(既存 `absent-config:` blocker と併存)。
- missing(空 config)+conflict → `compat-degraded:` finding。
- missing+conflict+`read_config_from:{windsurf:false}` → 両 finding
  無し(`legacy-conflict:` のみ残る)。
- present+conflict → `windsurf-conflict:` finding + exit 0・
  `clean:true`。
- `.claude/settings.json` のみ conflict+absent → 両 finding 無し。
- `legacy-conflict:` の訂正後文言(dedup は同一 canonical
  request/fingerprint 一致に限定・range を落とす compat firing は
  対象外)を検証し、「identical requests」への旧言い回しが残って
  いないことを確認。

`.claude` conflict fixture は `KNOWN_LEGACY_FILES` 内の
`.claude/settings.json` に cheaper hook 登録文字列を含む file を
書く形で新設してよい。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
  (PLAN-034KxCQ2FWFt55qhBNp6sR — T4)
- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS — 「§`hook::hook_config` —
  compat-degraded / windsurf-conflict verify finding」が
  AUTHORITATIVE — 発火条件・文言・severity・抑止判定)
- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM — R6)
- `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
  (TSPEC-034KxD0IRkMph8hSZbhTRx — 発火条件・抑止・severity 規約)

## Dependencies

- なし(T1-T3 の `targets.rs` 系変更とファイル非重複のため並列
  実行可能)。

## Affected Files

- `crates/cheaper-cli/src/hook_config.rs`(modify — `run_verify`
  finding 追加・`legacy-conflict` 文言訂正)
- `crates/cheaper-cli/tests/integration/hook_config.rs`(modify —
  finding fixture・`.claude` conflict fixture)

## Done When

- [x] `own_state` absent/missing + windsurf conflict + 非無効化で
      `compat-degraded:` finding が emit される(informational、
      exit code 不変)
- [x] `own_state` present 系 + windsurf conflict + 非無効化で
      `windsurf-conflict:` finding が emit される(informational、
      `clean:true` ・exit 0)
- [x] `read_config_from.windsurf == false` で両 finding が抑止
      される(`read_config_from` 欠落・非 object・`windsurf` が
      `false` 以外は抑止しない)
- [x] windsurf conflict が無い場合(`.claude` のみ等)は両 finding
      が emit されない
- [x] `legacy-conflict:` の文言が dedup 限定条件(同一 canonical
      request/fingerprint 一致・range を落とす compat firing 対象外)
      へ訂正されている
- [x] `sync`/`remove`/`--event-stdin`・own-group 検証・capability
      gate・orphan sweep・他 finding 文言が不変

## Out of Scope

- `targets.rs` 系の全変更(T1-T3)
- `distribution.md` 追記(T5)
- `sync`/`remove`/`--event-stdin` の動作変更(不変)
- windsurf conflict 検出範囲の拡大(user scope・親 dir・大小文字
  variant — 既知の限界として design Deferred Design Concerns)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` exits 0(新規 finding fixture + 既存 contract 非回帰) — 実測: 26 passed / 0 failed(新規 5 test: `hook_config_verify_compat_degraded_findings`・`hook_config_verify_windsurf_conflict_finding_informational`・`hook_config_verify_read_config_from_suppression`・`hook_config_verify_non_windsurf_conflict_emits_neither`・`hook_config_verify_legacy_conflict_wording_corrected`)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

---
id: "TSPEC-034IpNMbAuIAGTxh12swmj"
type: "test-spec"
status: "draft"
title: "Hook eval dedup verdict cache contract"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
  derived-by: []
  verifies:
    - "docs/specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Hook eval dedup verdict cache contract

## Purpose

issue #127(重複 hook firing)に対して、PLAN-034IfA0UHILhGW5DsirUEp が実装する
`hook::eval_cache`(canonical request fingerprint verdict cache + leader
election)が SPEC-034Ie1ZEdUR2aBFVoqkBay の AC1–AC6 を満たすことを固定する
test contract。具体的には「同一 canonical request・同一 file 状態 signature・
同一 policy/env 入力の重複 firing が高々 1 回の高コスト評価へ収束し、全
firing が同一 verdict を自 target の native contract で render して返す」
こと、および協調機構(verdict record・leader lock・TTL sweep)の全失敗が
通常評価への fail-open に倒れることを保証する。

本 test spec は、dedup 機構が除去・別構造へ置換されたとき、または
`evaluate` が観測可能入力に対して非決定的になり verdict 共有が厳密に
正しくなくなったときに retire してよい。それまでは spec/design が再編
されても「重複発火を高々 1 評価へ dedup し、fail-open と per-target
render を壊さない」という意図を保存する役割を持つ。

## Feature

`cheaper hook` の normalize → evaluate の間に挿入される dedup 層: 同一
fingerprint(16 hex、`<userConfigDir>/hook-eval/<fp>` を file 名とする)を
持つ firing 群のうち leader のみが `evaluate` を実行して verdict record を
atomic publish し、follower は bounded wait で共有 verdict を serve する。
lock 取得失敗・record 破損・TTL 超過等のあらゆる失敗は `eval_fn` 直接
呼出し(通常評価)へ畳まれる。

## Rules

### Test layer 分割と隔離(全 Rule に共通する前提)

- 検証は 2 層で行う。
  - **module 層 seam**: `cheaper_cli::hook::eval_cache::evaluate_dedup_with`
    (`#[doc(hidden)] pub`、integration test crate から到達可)へ
    `Arc<AtomicUsize>` で計数する `eval_fn`(200ms sleep で重なり確定)と
    ms スケールへ縮めた `DedupLimits` を注入し、協調・fingerprint
    invalidation・fallback を timing race に依存せず決定的に検証する。
    fingerprint は最初の 1 firing で `<cfg>/hook-eval/` 内に生成される
    `<16hex>.json` の file 名から取得し、lock/record の手配置に使う。
  - **binary 層**: `env!("CARGO_BIN_EXE_cheaper")` を `cheaper hook`
    として spawn(hook_config.rs 由来の PATH shim/spawn パターン)し、
    per-target render・exit code・`<userConfigDir>` 配下の file 残存
    といった実 process 契約を検証する。
- 隔離: 全 test は `CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE` を tempdir
  へ向ける。`hook.rs` の `pub(crate)` 化された `Fixture` が供給する env
  map を再利用し、test が実 `~/.config/cheaper/hook-eval/` を一切触れ
  ないことを構造的に保証する。`#[tokio::test(flavor =
  "current_thread")]` と `tests/common` の fixture/helper を踏襲する。

### AC 対応 Rule

- Rule(AC1 / R1,R3): 同一 canonical request・同一 file stat・同一
  threshold の N firing で、token 計測・bounded materialization・search
  scan は高々 1 回。残りの firing は共有 verdict record から serve され、
  fingerprint が同一なら target・`source_tool` が異なっても dedup される
  (fingerprint に target・`source_tool`・session・pid・raw `path`/`cwd`・
  config-dir 系 env・hook event 名を含めない — R1)。
- Rule(AC2 / R4,R9): 全 firing が同一内容の verdict(decision および
  reason)を受け取り、それぞれが自 target adapter の native response
  contract で render する。cache されるのは canonical `Verdict` のみで、
  render 済み応答は共有しない。
- Rule(AC3 / R1,R2): verdict を変え得る観測可能入力 — 対象 file の stat
  signature(mtime・size)・`CHEAPER_READ_TOKEN_THRESHOLD`・
  `CHEAPER_COUNT_MAX_BYTES`・builtin/user/project 各 policy 層の stat —
  の変化は別 fingerprint を生成し独立に評価される。逆に絶対 cwd
  spelling 差(`./x`・語尾 `/`・symlink 経由)は同一 fingerprint へ収束
  する。
- Rule(AC4 / R5): leader crash 相当の stale lock・fresh lock + verdict
  未 commit(crash mid-eval)では、follower が `lock_stale` takeover また
  は `follower_wait` deadline の self-eval で必ず verdict を返す。stale
  takeover は rename-claim(`<fp>.lock` → `<fp>.lock.claim-<pid>-<nonce>`
  + 観測 owner token 照合)で atomic に行い、leader の release は
  delete-if-owner(token 照合)に限定される。token 不一致・unparseable
  payload の lock は削除しない(保守的)。非 file lock は協調不能で
  即時 self-eval。
- Rule(AC5 / R5,R6): 破損 record(非 JSON・未知 `schemaVersion`・
  `fingerprint` field 不一致・TTL 超過)・cache dir 作成不能等の失敗は
  「存在しない」ものとして通常評価へ fallback し、fail-open を要求する
  contract(copilot の never-nonzero-exit)で非 0 exit を生じない。
- Rule(AC6 / R7,R8): verdict record(`verdict_ttl`)・lock
  (`lock_stale`)・orphan `.tmp-*`・rename-claim 残骸 `.lock.claim-*`
  は checked-age 規則で期限切れ判定され、serve されず sweep で除去
  される。record の未来 mtime は常に not-served。lock の未来 mtime
  は skew 範囲内(`now + lock_stale` 以下)は live 扱いで削除せず、
  `now + lock_stale` 超過のみ takeover・sweep 対象とする。dedup
  状態は workspace・`.cheaper/` に一切書かない。

## Examples

- `concurrent_firings_single_evaluation`(AC1)— module seam: 同一
  `RoutingRequest`・同一 `CHEAPER_CONFIG_DIR` で 2 つの
  `evaluate_dedup_with` を `tokio::join!` — `eval_fn` 計数 = 1、両
  verdict が同一内容。3 発目は計数不変の純粋 cache hit。
- `binary_dual_spawn_same_verdict`(AC1,AC2)— binary 層: `--target
  devin` 1 process + `--target pretooluse` 1 process を同一
  `CHEAPER_CONFIG_DIR`・同一 cwd で同時 spawn し、両 process へ同一の
  devin-shaped payload(`{"hook_event_name":"PreToolUse","tool_name":
  "read","tool_input":{"file_path":...}}`)を流す — 両 child の
  stdin を **wait 前に全て書き切って**真に重なった実行にする
  (write→wait の逐次 spawn では overlap が起きず dedup を実証しない)。
  余分な key は `tool_use_normalize` で無視され、両 target が同一
  FileRead request へ normalize されるため cross-target dedup を証明する。`file_path` は
  threshold 超過の実 file を pin し、deny verdict が token 計測経路を
  通る(optional fast-path gate 採用時も skip されない)ことを保証。
  assert: 両 process が各 target contract で同一 deny を render(devin
  `decision:"block"` / pretooluse `hookSpecificOutput`)、
  `hook-eval/<fp>.json` が 1 件・`.lock` 非残存。
- `per_target_render_identity`(AC2)— 同一 payload 2 連続実行で 1 回目
  (leader 評価)と 2 回目(cache serve)の stdout/stderr/exit が byte
  同一 — windsurf(stderr + exit 2)・copilot(deny JSON + exit 0)・
  pretooluse(`hookSpecificOutput`)・devin(`decision:"block"`)・
  cursor(permission JSON)の 5 target。
- `stat_change_new_fingerprint`(AC3)— file の size または mtime 変化で
  別 fingerprint → 再評価(計数増・別 `<fp>.json`)。
- `threshold_change_new_fingerprint`(AC3)—
  `CHEAPER_READ_TOKEN_THRESHOLD` 変化で別 fingerprint → 再評価。
- `policy_and_count_cap_new_fingerprint`(AC3)— user
  `<userConfigDir>/policy.yaml` の stat 変化(mtime/len)と
  `CHEAPER_COUNT_MAX_BYTES` 変化がそれぞれ別 fingerprint を生成し独立
  評価される(R2 — verdict を変え得る観測可能入力の同一性キー反映)。
- `cwd_spelling_same_fingerprint`(AC3,R1 補完)— tempdir 配下の**絶対**
  spelling 差(`<tmp>/./sub`・`<tmp>/sub/`・symlink `<tmp>/link` →
  `<tmp>/sub`)が `canonicalize_workspace_root` + canonicalize 済み
  resolved path で同一 fingerprint に収束。相対 spelling(`./sub`・
  `sub`)は `set_current_dir` が要り global state race を招くため使わ
  ない。
- `stale_lock_takeover`(AC4)— `<fp>.lock` 手配置 +
  `std::fs::File::set_modified` で `lock_stale` 超過の古い mtime →
  follower が takeover して評価完遂。
- `fresh_lock_stale_crossing_takeover`(AC4)— **fresh** mtime で
  植えた lock が follower の poll 中に `lock_stale` を跨ぐ経路
  (`DedupLimits{lock_stale: ~50ms, follower_wait: ~2s}`)。初回
  tick は Fresh → Wait、stale 越えの tick で Stale →
  rename-claim → takeover が決定的に走る — backdate 手配置では
  到達しない mid-poll の Fresh→Stale 遷移と claim 経路を固定する。
- `fresh_lock_no_verdict_self_eval`(AC4)— fresh lock + verdict 無し
  (crash mid-eval 模擬)で follower が `follower_wait` deadline 後に
  self-eval(計数 = 2)。
- `live_leader_delete_if_owner`(AC4)— delete-if-owner primitive を直接
  unit test して決定的に検証: owner token T2 を含む lock file に T1 で
  remove を試みると file が残存し、T1 を含む lock file は T1 で除去
  される。unparseable payload の非削除規則もここで固定(3-party
  timing race 観察には依存しない)。
- `corrupt_record_fallback`(AC5)— 非 JSON・未知 `schemaVersion`・
  `fingerprint` field 不一致・TTL 超過 backdated mtime の record が
  serve されず通常評価へ fallback。
- `copilot_unwritable_cache_exit_zero`(AC5)— `CHEAPER_CONFIG_DIR` を
  file path へ向け cache dir 作成不能にしても copilot target が常に
  exit 0 + schema-valid JSON。
- `future_mtime_not_served`(AC6)— 未来 mtime の record / lock が serve
  されず stale 扱いで sweep 対象。
- `sweep_removes_stale`(AC6)— TTL 超過 `.json`・stale `.lock`・
  orphan `.tmp-*` が次回起動で除去。
- `dir_nested_edit_ttl_bound`(staleness bound の明文化)— dir target の
  nested file 編集は dir stat signature に現れず、TTL 内では同一
  verdict が serve される(staleness bound = TTL + 評価時間を固定)。
- `no_workspace_writes`(R8)— workspace 内に `hook-eval/` が生成され
  ない(`<ws>` 配下に新規 file/dir が無いことを assert)。

## Guarantees

- 同一の観測可能入力を持つ firing は高々 1 回の高コスト評価へ収束し、
  全 firing が同一内容の verdict(decision + reason)を得る — 共有は
  近似ではなく厳密に正しい(deny を allow へ弱めない)。
- dedup 経路のあらゆる失敗(fingerprint 確定・record read・lock 取得・
  record write・dir 作成・未来 mtime・破損 payload)が通常評価への
  fail-open であり、eval_cache 内に panic 経路・新たな非 0 exit・deny
  強化を生じない(copilot never-nonzero 契約)。
- 協調状態(verdict record・lock・tmp orphan)は bounded TTL で管理
  され、期限切れ entry は serve も残存もしない — 無制限蓄積しない。
- cache されるのは canonical `Verdict` のみで render 済み
  `HookResponse` は共有しない — 5 target の応答契約(stdout/stderr/
  exit code)が dedup 有無で byte 同一に保存される。
- test は隔離された tempdir のみを使い、実 user config dir・workspace・
  `.cheaper/` を汚染しない。
- fingerprint 構成に `target`・`source_tool`・session・pid・raw
  `path`/`cwd`・config-dir 系 env・hook event 名が混入しない(R1 —
  混入すると firing 間で key が割れて dedup 効果が消える)。

## Non-goals

- `cheaper hook` process spawn 自体の削減 — 登録数分の spawn overhead
  は残る既知の受容限界であり、本 spec は計測・実体化コストのみを対象
  とする。
- `evaluate` の verdict semantics・deny reason・threshold 既定値・
  fail-open/fail-closed 境界・`resolve_target` containment 規則 —
  既存 `hook::` integration test が担う回帰範囲であり、本 spec は
  dedup がそれを変えないこと(render identity・hook:: 全件回帰)のみを
  固定する。
- fingerprint に現れない内容変化(mtime granularity 内の in-place
  write・dir target の nested file 編集)の即時検出 — TTL で bound さ
  れる受容済み残差であり、`dir_nested_edit_ttl_bound` は bound の
  存在のみを固定する(即時 invalidation は要求しない)。
- `hook-config sync` legacy-conflict gate の blocker → warning 降格、
  Devin 側の import merge / firing dedup — 別 follow-up・upstream
  責務であり検証対象外。
- live leader 誤判定時の冗長評価や 3-party timing race の再現検証 —
  `live_leader_delete_if_owner` の primitive 層 coverage が race
  observation を置き換える。
- 実 host(Devin CLI 等)からの end-to-end 発火 — binary 層は
  `CARGO_BIN_EXE` spawn のみを対象とする。
- 新規 env var・設定 knob の振る舞い — 時間定数は `DedupLimits` 注入
  で検証し、production 値自体の妥当性計測は対象外。

## Risk

- 本 contract を外すと以下を回帰として検出できない: (a) fingerprint
  への `target`/`source_tool`/session/pid/raw path/cwd 混入による key
  分裂で dedup が完全に効かなくなる退行、(b) stat/threshold/policy
  層/count cap の fingerprint 未反映による異なる入力への誤 verdict
  共有(stale deny/allow の供給)、(c) 失敗経路の非 fail-open 化 —
  copilot で非 0 exit が発生すれば hook 全体が停止相当の障害になる、
  (d) lock/record の無制限蓄積・期限切れ serve・stale lock による
  follower ハング、(e) workspace/`.cheaper/` への書き込み混入。
  いずれも「重複発火が登録数比例コストへ戻る」か「誤 verdict を
  共有する」形で host 観測挙動または cost 目標を静かに壊す。

---
id: "SPEC-034Ie1ZEdUR2aBFVoqkBay"
type: "spec"
status: "approved"
title: "Hook duplicate firing single-evaluation dedup"
created: "2026-10-18"
updated: "2026-10-19"
owners: ["scarecrow173"]
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
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "./cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "./huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "../adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
    - "../discovery/bounded-hook-read-request-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook/mod.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook_config.rs"
  defers: []
  deferred-by: []
---

# Hook duplicate firing single-evaluation dedup

## What Should Be Built

同一の tool call に対して複数の cheaper hook 登録が発火しても、
高コストな評価（file token 計測・bounded materialization・
search scan）を高々 1 回だけ実行し、全 firing が同一 verdict
を共有する dedup 機構を `cheaper hook` の評価経路に提供する。

求める振る舞い:

1. 各 firing が持つ normalize 済み canonical request・対象 file
   の状態 signature・threshold から「同一評価」を識別する
   同一性キー（canonical request fingerprint）が決まり、同一
   キーの firing 同士は 1 つの評価結果を共有する。
2. 同一評価グループでは 1 つの firing（leader）だけが実際の
   評価を実行し、他の firing（follower）は共有された verdict
   を利用する。leader の crash・stale lock・待機 timeout では
   follower が独立した通常評価へ fallback し、必ず verdict を
   返す。
3. 共有されるのは canonical verdict であり、各 firing は自
   target の adapter で native response contract へ render
   する。host（Devin 等）が観測する応答の件数・内容は現状と
   変わらない。
4. 共有状態（verdict entry・協調用 lock）は有界な lifetime
   を持ち、破損・filesystem 失敗はすべて通常評価への
   fail-open fallback となる。

## Why Now

- Devin CLI は `read_config_from` で他 agent の hook 設定
  （`.claude/settings.json`・`.windsurf/hooks.json` 等）を
  merge import し、自 session の event として発火する。
  import-path firing が end-to-end で実動作することは live
  検証済み（Devin session が hooks を
  `by_provider=[claude=1,devin=8,windsurf=2]` で load し、
  12,073 token の file read が hook deny → `cheaper bulk-read`
  delegation まで完走 — 2026-10-18/19 検証、upstream
  discovery 記録）。
- `.devin/config.json` への cheaper native 登録（canonical
  matcher `read|grep|exec`）を進めると、native 系統と import
  された claude 系統の両方が同一 tool call で live firing
  になる構造が確定的。1 回の tool call で複数の `cheaper
  hook` プロセスが spawn し、それぞれが独立に file token
  計測と bounded materialization を実行するため、コストが
  登録数に比例する（issue #127）。
- 現状の防御は `hook-config sync` の legacy-conflict gate
  （legacy file に cheaper 互換 read hook 登録を検出すると
  sync を拒否）による重複発火の未然防止のみ。しかし Devin
  の import merge は cheaper 側から制御不能であり、単一登録
  gate ではこの重複を吸収できない。dedup 機構があれば重複
  発火は無害化され、将来 gate を blocker → warning へ降格
  して native + import 共存を support する余地が生まれる
  （別 follow-up として扱う）。

## Users and Value

- **multi-agent 環境で動作する parent coding agent（Devin +
  Claude Code 等）**: 同一 tool call で複数登録が発火しても
  計測・実体化コストが 1 回分に抑えられ、hook の応答
  latency と tokenizer / IO 負荷が登録数に比例しなくなる。
- **cheaper CLI maintainer**: 発火の重複は host（Devin
  importer）由来で cheaper からは制御不能。評価結果の共有
  により「重複して発火されても正しく・安く振る舞う」ことを
  cheaper 自身の責務として保証できる。
- **consumer**: verdict の意味論・per-target 応答 contract・
  host 観測挙動は不変のまま、コストだけが下がる。

## Scope

### In Scope

- 同一 canonical request・同一 file 状態・同一 threshold の
  firing 間で、file token 計測 / bounded materialization /
  bounded search scan を高々 1 回へ dedup する共有 verdict
  機構（評価は 1 回、応答は全 firing が返す）。
- firing 間の協調（単一評価を担う leader の選定・結果共有・
  follower の有界な待機）と、leader crash / stale lock /
  timeout / filesystem 失敗に対する独立評価への fallback。
- 共有 verdict の per-target render 分離 — cache するのは
  canonical verdict であり、応答は各 firing の target
  contract（windsurf = stderr+exit 2 / pretooluse =
  `hookSpecificOutput` / devin = `decision:block` / cursor =
  permission JSON / copilot = deny JSON+exit 0）で render
  されること。
- cache・lock 等の協調状態に対する bounded lifetime（TTL・
  stale entry の sweep / 失効）。
- dedup 状態の配置境界 — workspace・`.cheaper/` には書かず、
  user config dir 配下に best-effort で保持し、書き込み失敗
  が hook 応答や exit code に影響しないこと。

### Out of Scope

- `hook-config sync` の legacy-conflict gate を blocker から
  warning へ降格する変更。dedup はその降格を可能にする前提
  だが、gate 自体の変更は本 spec の dedup 実証後の別
  follow-up として扱う。
- `.devin/config.json` の内容・worktree への hook 登録・
  install 手順の変更（外部 mise setup task の所有物であり、
  本 repository の変更境界外）。
- Devin 側の import merge / firing dedup。発火の重複は host
  が作っており責務 layer としては upstream が正しいが、
  Devin 側の変更が必要で cheaper 単独では実現不能（長期
  option として upstream 待ち）。
- verdict 意味論自体の変更。allow/deny の判定基準、fail-open /
  fail-closed の境界、threshold 既定値は現行のまま維持する。
- `cheaper hook` プロセス spawn 自体の削減。dedup で削減する
  のは計測・実体化コストのみであり、登録数分の spawn
  overhead が残ることは既知の限界として受容する。

## Requirements

- R1: 2 つの firing が「同一評価」と見なされるのは、
  normalize 済み canonical request・対象 file の状態
  signature（mtime・size）・threshold が一致する場合に
  限る。同一性キーに target・source_tool・session・process
  固有値を含めてはならない（含めると firing 間でキーが
  割れて dedup 効果が消える）。
- R2: verdict を変え得る全ての**観測可能な**入力（request の
  各 field・resolved target の stat signature・threshold・
  count cap・builtin / user / project の各 policy 層・実効
  cwd）は同一性キーに反映されるか、dedup の適用外としなけ
  ればならない。共有 verdict の reuse は同一の観測可能入力
  に対して近似ではなく厳密に正しいこと — stat signature
  に現れない内容変化（mtime granularity 内の in-place 変更・
  dir target の nested file 編集等）は TTL で bound される
  受容済みの残差であり、contract 違反とは見なさない。なお
  `evaluate` は計測の transient 失敗（timeout・IO error・
  import 失敗）で失敗由来の `Allow` を非決定的に返し得る
  が、この `Allow` は fail-open 方向への共有として TTL 内
  で reuse してよい（受容済み）。
- R3: 同一評価グループ内で高コストな評価処理（token 計測・
  bounded materialization・search scan）は高々 1 回だけ
  実行される。最初に評価を完遂した firing が結果を共有し、
  残りはそれを利用する。
- R4: 各 firing は同一内容の verdict（decision および
  reason）を受け取り、自 target の adapter で render する。
  共有物は canonical verdict であり、render 済み出力では
  ない。
- R5: leader の crash・stale lock・follower 待機の timeout・
  共有 entry の破損・filesystem 失敗は、すべて独立した通常
  評価への fallback とし、どの firing も必ず verdict を返す。
  破損・別キー・期限切れの共有 entry は「存在しない」もの
  として扱い serve しない。
- R6: dedup 経路の如何なる失敗も、fail-open を要求する
  target contract（copilot の never-nonzero-exit 等）で非 0
  exit や deny 相当の挙動を新たに生み出してはならない。
- R7: 共有 entry と協調用 lock は有界な TTL を持ち、期限切れ
  entry は serve せず、sweep または自然失効により無制限に
  蓄積しない。TTL 値は設計で確定する有界値とする（暫定指針
  は upstream discovery の ~60 秒）。
- R8: dedup 状態は user config dir 配下に best-effort で保持
  し、workspace・`.cheaper/` には一切書き込まない。協調・
  共有に関わる書き込み失敗は hook 応答・exit code に影響
  しない。
- R9: host が観測する挙動は不変 — N firing に対し N 件の
  verdict が返り、各 verdict の内容は同一の観測可能入力が
  与えられた場合に dedup 無しの場合と同一である（deny を
  allow へ弱める近似をしない）。signature に現れない入力
  変化の取りこぼしによる残差は Deferred Design Concerns の
  mtime granularity・clock skew 項目のとおり TTL で bound
  される受容済みのものとする。

## Acceptance Criteria

- [ ] 同一 tool call に対して N 個の cheaper hook 登録が
  発火する場合（同一 canonical request・file stat 不変・
  同一 threshold）、token 計測 / bounded materialization
  の実行が高々 1 回であり、残りの firing が共有結果から
  serve されることが test で固定される。
- [ ] 全 firing が同一内容の verdict を返し、それぞれが自
  target の native response contract で render されること
  が test で固定される（例: devin 系 firing は devin の
  decision contract、claude 系 firing は `hookSpecificOutput`
  contract で応答）。
- [ ] **File target** の stat（mtime または size）の変化、
  または異なる threshold が異なる fingerprint を生成し、
  独立に評価されることが test で固定される。Dir target の
  nested file 編集は dir の stat signature に現れないため、
  同一 fingerprint 内では TTL のみで bound される（別
  fingerprint への切替は要求しない）。
- [ ] leader プロセスの crash または lock の stale 化が
  あっても follower は独立評価へ fallback し verdict を返す
  ことが test で固定される。
- [ ] cache / lock の破損や filesystem 失敗が、fail-open を
  要求する contract（copilot 等）で非 0 exit を生じず、
  当該 firing が通常評価へ fallback することが test で
  固定される。
- [ ] stale な cache・lock entry が bounded TTL 内に sweep
  または失効し、期限切れ entry が serve されないことが
  test で固定される。

## Deferred Design Concerns

- legacy-conflict gate の blocker → warning 降格 | dedup で
  重複発火が無害化された後に意味を持つ変更 | 本 spec の
  dedup が実装・実証された時点 | 降格しない限り native +
  import 登録の共存は sync で拒否され続ける（現状維持は
  安全側）。
- verdict 共有の TTL 値・follower 待機の bounded wait 上限・
  sweep 起点 | importer firing の時間分散・連続 read の実
  interval 実測が根拠として必要 | ADR / design で確定 |
  TTL が長すぎると古い file 状態を serve するリスク（stat
  signature が一致する限り実害は限定される）。
- mtime granularity・clock skew の扱い | stat signature が
  fast successive write や host clock ずれを取りこぼす
  可能性 | design で許容方針を決定 | 同一 fingerprint 内の
  file 変化を見逃すと stale verdict を serve し得るため、
  取りこぼし時は安全側（再評価）へ倒す方針の確認が必要。
- Devin 側 import dedup | 発火の重複は host の責務であり
  cheaper 制御外 | upstream（Devin CLI）に import merge の
  dedup が入った場合 | 本 spec の機構はその場合も無害に
  動作するため blocking リスクは無い。
- N 回の `cheaper hook` spawn overhead | dedup の効果は
  計測・実体化コストに限定される | spawn 自体が測定上の
  問題になった場合 | 登録数分のプロセス起動コストは残る。

## Sources and Relations

- issue #127: <https://github.com/scarecrow173/cheaper/issues/127>
- upstream discovery: `docs/discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md`
  （DISC-034IdYnTDqD7Ao1nG3aoY9）— 重複発火の live 検証証拠、
  option 比較、fingerprint / leader election hypothesis、
  open questions の記録。本 spec はその promotion candidate
  「spec-doc needed」を引き継ぐ。
- 関連 spec / ADR / design / discovery:
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  （canonical request・bounded evaluate・managed registration
  の先行 contract）、
  `docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md`
  （per-target adapter と native contract matrix）、
  `docs/specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md`
  （fail-open / fail-closed 境界）、
  `docs/adr/copilot-adapter-separate-script-never-nonzero-exit.md`
  （copilot never-nonzero 制約 — cache 経路失敗の fail-open
  要求の根拠）、
  `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`
  （canonical request normalize — fingerprint の入力となる
  request の定義元）、
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  （hook-config sync と legacy-conflict gate — follow-up で
  降格対象となる gate の定義元）、
  `docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md`
  （本 spec と並行して作成された fingerprint dedup の構造決定
  ADR）、
  `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`・
  `docs/adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md`・
  `docs/adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md`・
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`・
  `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`・
  `docs/discovery/bounded-hook-read-request-contract.md`。
- code: `crates/cheaper-cli/src/hook/{mod,targets,routing_policy}.rs`、
  `crates/cheaper-cli/src/hook_config.rs`（`KNOWN_LEGACY_FILES` +
  `CONFLICT_PATTERN`）。

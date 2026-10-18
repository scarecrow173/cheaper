---
id: "ADR-034Ie3HFd6tIJms6Z2NIow"
type: "adr"
status: "proposed"
title: "Dedup concurrent hook firings via canonical request fingerprint verdict cache"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
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
  depends-on:
    - "./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "./copilot-adapter-separate-script-never-nonzero-exit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "./apm-distributed-devin-native-hook-bootstrap.md"
    - "./range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "./conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
    - "./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "./usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../specs/hook-duplicate-firing-single-evaluation-dedup.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook/mod.rs"
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-core/src/fs_util.rs"
    - "../../crates/cheaper-core/src/config/dirs.rs"
    - "../../crates/cheaper-cli/src/usage_log.rs"
    - "../../crates/cheaper-cli/src/embedded.rs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# Dedup concurrent hook firings via canonical request fingerprint verdict cache

## Context and Problem Statement

Devin CLI は `.claude/settings.json`・`.windsurf/hooks.json` 等の他
agent の hook config を importer で merge し、自身の session で
発火させる。その結果、1 回の tool call に対して複数の cheaper hook
登録が発火し得、各 firing は独立した `cheaper hook` process を
spawn して同一 file の token 計測・bounded materialization を
個別に実行する — 登録数に比例した冗長コストになる(issue #127)。

import 経由の発火は end-to-end で実動作することが live 検証済み
(2026-10-18/19: `by_provider=[claude=1,devin=8,windsurf=2]` で
hook が load され、12,073 token の file read が deny →
`cheaper bulk-read` delegation まで完走)。`.devin/config.json` への
native 登録(ADR-034GssAufIiId3hgh4jrSQ /
ADR-034H2WBNtC4We8dyC5lQZJ、canonical matcher `read|grep|exec`)を
進めると、native 系統と import された claude 系統の 2 系統が確定的に
live firing になる。

現状の唯一の防御は `cheaper hook-config sync` の legacy gate —
`KNOWN_LEGACY_FILES` + `CONFLICT_PATTERN`
(`crates/cheaper-cli/src/hook_config.rs`)で旧登録を検出した場合
sync を拒否する blocker であり、native 登録と import 系統の共存を
許さない。Devin 側の import merge は cheaper が制御できないため、
発火の重複は runtime 側で吸収する必要がある。

`handle_hook`(`crates/cheaper-cli/src/hook/targets.rs`)は target 別
`normalize → evaluate → render` の構造を持ち、
`evaluate`(`routing_policy.rs`)の `Verdict` は canonical
`RoutingRequest`・対象 file 状態・threshold
(`CHEAPER_READ_TOKEN_THRESHOLD`)・計測 cap
(`CHEAPER_COUNT_MAX_BYTES`)の決定的関数で、target / `source_tool` /
session には依存しない。この決定性が verdict cache を「近似」では
なく「厳密に正しい」ものにする前提条件である。

## Decision Drivers

* **correctness**: cache reuse は verdict を変えてはならない。
  fingerprint は `evaluate` が読む全入力(canonical request の
  policy 関連 field・対象 file 状態・threshold・count cap・
  builtin dir 識別)を網羅し、target / `source_tool` / session /
  process id は含めない。
* **fail-open**: copilot は「いかなる」非 0 exit も deny と解釈する
  (ADR-034r05iHrZUE6WizQ4LgPa)。cache / lock / fs のあらゆる失敗は
  通常評価へ縮退し、cache 経路が新たな非 0 exit・panic・deny 化
  する経路を作らない。
* **per-target contract**: cache するのは canonical `Verdict`
  のみ。render は各 firing が自分の adapter で行い、byte 互換 JSON
  出力契約(targets.rs、`cheaper_core::json::stringify` + `"\n"`)を
  侵さない。
* **boundedness**: TTL・lock stale 判定・sweep で cache dir の肥大と
  follower の無限 wait を防ぐ。follower wait は計測の worst-case
  (`MEASURE_TIMEOUT` = 30s)を暫定上限とする。
* **minimal surface**: 新規依存を増やさず、既存の atomic write・
  userConfigDir 解決・fingerprint 前例を再利用する(KISS / YAGNI /
  DRY)。

## Considered Options

* Option 1: verdict cache(canonical request fingerprint + leader
  election)を `handle_hook` の normalize → evaluate の間に挿入する。
* Option 2: 現状維持 — `hook-config sync` の legacy gate で
  「1 project 1 登録」を強制し続ける。
* Option 3: Devin 側の import dedup — 発火元(host)で重複 firing を
  統合する。
* Option 4: 2 発目以降の firing で abstain / allow を返す
  (**rejected**)。

## Decision Outcome

Chosen option: "Option 1 — verdict cache via canonical request
fingerprint + leader election"。N firing を 1 回の計測 + N-1 回の
cache hit へ集約でき、host が観測する挙動(N 件の同一 verdict が
各 target contract で返る)は不変であり、verdict の決定性を前提に
correctness を保ったままコストだけ削減できるため。

### Design

- **挿入位置**: `handle_hook` の各 arm で `evaluate(&request, ...)`
  を呼ぶ箇所を dedup wrapper(新規 `hook/eval_cache.rs` の
  `evaluate_dedup`)経由に置き換える。normalize・render・最深部
  catch-all(`execute_hook` / `fallback_allow`)は変更しない。
- **fingerprint**: `DefaultHasher`(`embedded_fingerprint()` と同じ
  `{:016x}` 形式)で `hash(canonical request + stat signature +
  policy inputs)` を算出し、`<fingerprint>` を file 名に使う。
  - canonical request: `RoutingRequest` の policy が読む field
    (kind・path・cwd・range・pattern・max_results・context・glob・
    output_mode・case_insensitive・command)を field 単位で明示
    列挙して hash 入力とする。`source_tool`・target・session・pid
    は**含めない**(含めると firing 間で key が割れて dedup 効果が
    消える)。`source_tool` を保持する enum への `derive(Hash)` は
    使わない。
  - stat signature: request が解決する filesystem target それぞれの
    `(resolved path, mtime, len)`。FileRead / Search は
    `resolve_target` と同じ containment 規則で解決した path、
    Shell は `extract_read_candidates` の各 operand を同規則で cwd
    解決した集合とする。解決結果が 0 件(out-of-scope・missing・
    候補無し)の request は cache を介さず直接 `evaluate` する
    (その verdict は定数 allow で、cache する意味も stat 署名も
    無い)。
  - policy inputs: `threshold(opts.env)`
    (`CHEAPER_READ_TOKEN_THRESHOLD`)、`count_max_bytes(opts.env)`
    (`CHEAPER_COUNT_MAX_BYTES`)、`builtin_dir` 識別(`Some(path)` の
    path 文字列 / `None`)。`CHEAPER_CONFIG_DIR` /
    `XDG_CONFIG_HOME` / `USERPROFILE` / `HOME` は cache root
    (`<userConfigDir>`)自体を分岐するため、store 配置で構造的に
    分離済みであり fingerprint への追加は不要。
  - hash は同一 binary 内で決定的であればよい — toolchain 差で値が
    変わっても効果は hit rate 低下のみで correctness には影響しない。
    新規 hash 依存(sha2 等)は追加しない。
- **store**: `<userConfigDir>/hook-eval/<fingerprint>.json` に
  canonical verdict を JSON で保存する(record 例:
  `{schemaVersion: 1, createdAt, verdict}`)。`<userConfigDir>` は
  `resolve_user_config_dir` 経由で解決し、workspace・`.cheaper/`
  には書かない。TTL は暫定 ~60 秒(`createdAt` / file mtime で判定、
  discovery の暫定指針。実測ベースの調整は spec / follow-up の
  責務)。read/write は全て best-effort — `usage_log.rs` と同じ
  「失敗しても exit code・stdout を変えない」契約。
- **leader election**: `<fingerprint>.lock` を
  `OpenOptions::new().create_new(true)`(O_EXCL 相当の atomic
  create)で取得する。取得した process(leader)が `evaluate` を実行
  し、verdict file を `cheaper_core::fs_util::atomic_write`
  (temp+rename)で書いてから lock file を remove する。lock file
  には pid + timestamp を書き stale 判定に使う。
- **follower**: lock 取得に失敗した process は verdict file を一定
  間隔で bounded-poll し、出現した fresh entry(TTL 内)を schema
  検証のうえ採用する。poll timeout・lock が stale(mtime が閾値
  超過)・fs 失敗のいずれでも**自前評価へ fallback** する —
  correctness は常に維持される。
- **render 分離**: cache するのは `Verdict`(`Allow` /
  `Deny{reason}`)のみ。各 firing は取得した verdict を自分の
  target adapter(`windsurf_render` / `copilot_render` /
  `pretooluse_render` / `devin_render` / `cursor_render`)で render
  するため、per-target の応答 contract は不変。
- **fail-open**: cache dir 解決・create・read・write・lock・poll・
  parse の全失敗は「cache 無しで `evaluate`」へ縮退する。copilot
  経路で新たな非 0 exit を生じさせない。
- **sweep**: hook 起動時(または cache write 時)に TTL 超過 entry
  と stale lock を best-effort で削除し、dir の肥大を抑える。

### Scope and Non-Goals

- `hook-config` の legacy gate の blocker → warning 降格は
  **out of scope** — dedup が実証された後の follow-up change とする
  (本 ADR はその前提条件を作るが、gate 自体は変更しない)。
- `.devin/config.json` の登録内容・matcher・lifecycle
  (`hook_config.rs`)の変更は行わない。
- Devin 側の import dedup は upstream の責務であり本 ADR では
  扱わない。
- `evaluate` の verdict semantics・deny reason・threshold・
  fail-open/fail-closed 原則は一切変更しない。

### Consequences

* Good, because N firing → 計測 1 回 + cache hit N-1 回となり、
  登録数に比例した冗長コスト(token 計測・bounded materialization・
  dir scan)が消える。
* Good, because 副次効果として同一 file の連続 read(turn 跨ぎ・
  別 agent 由来を含む)も stat 不変なら cache hit で高速化される。
* Good, because host が観測する挙動は不変 — 各 firing に同一
  verdict が返り、agent 側の redirect guidance 動作は実測どおり
  維持される。
* Good, because 重複発火が無害化されることで、legacy gate を
  blocker → warning へ降格し native 登録 + claude import 系統の
  共存を support する follow-up の余地が生まれる。
* Bad, because follower は leader の評価完了を bounded-poll で待つ
  ため、TTL 内の重複 firing に最大 wait budget 分の latency が
  追加される。
* Bad, because lock・TTL・sweep・fallback という concurrency 機構が
  新たに入り、test 対象の失敗 mode(leader crash・stale lock・
  corrupt entry)が増える。
* Bad, because `cheaper hook` process の spawn 自体は N 回分残る —
  削減できるのは計測・実体化コストのみ。
* Bad, because dir を対象とする Search の stat signature
  (mtime+size)は dir 直下の entry 変更でしか変わらず、nested file
  の内容変更を捉えられない。fast successive write に対する mtime
  granularity 限界も同様であり、staleness は TTL で bound する
  設計上の妥協とする。

### Confirmation

* concurrent duplicate firing で計測が 1 回だけ行われることを、
  計測回数を観測できる seam 経由の test で固定する。
* fingerprint 無効化(stat 変更・threshold 変更)・leader crash・
  corrupt cache の各 fallback、per-target render の同一性、copilot
  の exit 0 契約を test で固定する。
* `mise run check`(fmt / clippy / test を含む canonical pipeline)が
  pass すること。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-cli/src/hook/eval_cache.rs`(新規)— fingerprint
    算出・verdict store read/write・leader lock・bounded poll・
    TTL sweep・`evaluate_dedup` wrapper。
  - `crates/cheaper-cli/src/hook/mod.rs` — `pub mod eval_cache;`
    宣言の追加。
  - `crates/cheaper-cli/src/hook/targets.rs` — `handle_hook` 各 arm
    の `evaluate(&request, &deps.eval_opts())` 呼び出しを
    `eval_cache::evaluate_dedup(&request, &deps.eval_opts())` へ
    差し替え。
  - `crates/cheaper-cli/src/hook/routing_policy.rs` — `Verdict` へ
    serde の `Serialize` / `Deserialize` derive を追加(cache
    record 化のため)。fingerprint 用に `resolve_target` と同じ
    containment 規則で resolved path を返す `pub(crate)` helper を
    公開し、規則の複製を避ける。`extract_read_candidates` は既に
    `pub`。
  - `crates/cheaper-cli/tests/integration/` — dedup 用 test module
    (例 `hook_eval_dedup.rs`)を追加し、
    `tests/integration/main.rs` に `mod` 宣言を追加する
    (AGENTS.md の CLI integration test 配置規約)。
* Patterns to follow:
  - verdict file 書き込みは `cheaper_core::fs_util::atomic_write`
    (temp+rename、失敗時 temp を best-effort remove)—
    `auth/store.rs` の `credentials.json` 経路と同じ helper を使う。
    `windows_remove_first: false`(cache entry の置換は rename で
    済む)。
  - `<userConfigDir>` 解決は
    `cheaper_core::config::dirs::resolve_user_config_dir(
    UserConfigDirOptions { env: Some(opts.env), .. })` — 解決
    policy を複製しない。
  - best-effort 書き込み契約は `crates/cheaper-cli/src/usage_log.rs`
    の `record_usage`(失敗時も exit code / stdout 不変)に倣う。
  - fingerprint hash は `embedded.rs` の `embedded_fingerprint()`
    (`DefaultHasher` → `format!("{:016x}", ...)`)の前例に倣う。
  - lock は `std::fs::OpenOptions::new().create_new(true)`
    `.write(true)` の atomic create — pid + timestamp を書き込み、
    stale 判定(lock mtime が閾値超過)に使う。
  - follower wait は `tokio::time::sleep` による async poll とし、
    fs 操作は `tokio::task::spawn_blocking` へ逃がす
    (`measure_tokens` の blocking offload と同じ構造)。wait 上限は
    `MEASURE_TIMEOUT`(30s)を暫定上限とする。
  - verdict record は `schemaVersion` を持たせ、`usage.jsonl` と
    同じ forward-compat 規約(未知 version・parse 失敗は破棄して
    再評価)に従う。
* Patterns to avoid:
  - render 済み `HookResponse`(stdout/stderr text・exit code)の
    cache — per-target render は firing ごとに行う。
  - fingerprint への target / `source_tool` / session / pid の混入。
  - cache / lock 経路での panic・`unwrap` — 全失敗は「cache 無しで
    `evaluate`」へ畳む。
  - blocking `std::thread::sleep` での poll(async runtime を占有
    しない)。
  - workspace・`.cheaper/` への書き込み、新規 crate / hash 依存、
    deny verdict の弱化(abstain 化)。
* Dependencies or configuration: 新規依存なし(std の `OpenOptions`・
  `DefaultHasher`、既存の `serde` / `tokio` /
  `cheaper_core::fs_util` / `cheaper_core::config::dirs` のみ)。
  新規 env var も必須でない — TTL・poll 間隔・wait 上限は module
  内 const とし調整余地を残す。
* Expected tests:
  - 同一 canonical request の並行 firing で計測呼び出しが 1 回に
    なること(計測回数を観測する seam / counter を介して検証。
    `CHEAPER_CONFIG_DIR` で cache root を temp dir へ隔離する)。
  - stat signature 変化(mtime または size)・threshold 変化で
    fingerprint が変わり再評価されること。
  - leader が lock を残して死んだ場合(stale lock)に follower が
    自前評価へ fallback すること。
  - corrupt / partial verdict file を読んだ場合に通常評価へ
    fallback すること。
  - cache 済み同一 verdict が全 target で既存 render と byte 同一の
    応答になること(windsurf = stderr + exit 2、copilot =
    `permissionDecision` JSON + exit 0、pretooluse =
    `hookSpecificOutput`、devin = `decision:"block"`、cursor =
    `permission` JSON)。
  - cache / lock の全失敗注入で copilot target が常に exit 0 の
    ままであること。
  - TTL 超過 entry が read されず sweep で除去されること。

## Verification

* [ ] 同一 tool call に対する複数 firing(同一 canonical request)
      が計測を 1 回だけ実行することを示す test が
      `cargo test -p cheaper-cli` で pass する。
* [ ] file stat 変化・threshold 変化で cache miss → 再評価される
      test が pass する。
* [ ] stale lock / leader crash・corrupt cache entry の test で
      follower が自前評価へ fallback することを確認する。
* [ ] 同一 canonical verdict に対する per-target render が cache
      経由でも既存出力と byte 同一であることを確認する test が
      pass する。
* [ ] cache / lock / fs 失敗の注入で `cheaper hook --target copilot`
      が非 0 exit を返さないことを確認する。
* [ ] cache dir 内で TTL 超過 entry・stale lock が sweep される
      ことを確認する。
* [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration` と `mise run check` が pass する。
* [ ] `cheaper hook` 経路が workspace・`.cheaper/` に書き込まず、
      `<userConfigDir>/hook-eval/` のみに cache を置くことを code
      review / test で確認する。

## Pros and Cons

### Option 1 — verdict cache(canonical request fingerprint + leader election)

* Good, because 計測は 1 回・N-1 回は cache hit にでき、同一 file
  の連続 read にも効く。verdict の決定性を前提にするため
  correctness を損なわずコストだけ削減でき、host 観測挙動
  (N 件の同一 verdict)は不変。
* Bad, because lock・TTL・sweep・fallback の実装複雑性と、
  fingerprint 設計(決定性の前提)の正しさが要件になる。follower
  には bounded wait 分の latency が乗る。

### Option 2 — 現状維持(単一登録 gate)

* Good, because 変更不要であり、重複発火そのものを未然に防げる。
* Bad, because multi-agent 環境で「どこに登録するか」の選択を
  ユーザーへ強制し続ける。Devin import は cheaper 側が制御不能で
  あり、native 登録と import 系統の共存を構造的に阻害する。

### Option 3 — Devin 側 import dedup(発火元で重複統合)

* Good, because 責務の layer としては正しい — firing の重複は
  host が作っている。
* Bad, because Devin 側の変更が必要で cheaper 単独では実現不能。
  upstream 待ちになり、その間の冗長コストを解決できない。

### Option 4 — 2 発目以降の firing で abstain / allow(rejected)

* Good, because 実装が最も軽い。
* Bad, because host 側の verdict 合成順・AND/OR semantics が未知で、
  deny verdict を allow へ弱める可能性がある。同一 verdict を返す
  方式だけが安全と判断し棄却。

## More Information

* issue #127:
  <https://github.com/scarecrow173/cheaper/issues/127>
* upstream discovery:
  [hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md](../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md)
  (DISC-034IdYnTDqD7Ao1nG3aoY9)— live 検証メモ・alternatives・
  open questions(TTL 値・clock skew・stale lock・poisoning 評価)を
  含む。
* 関連 ADR/spec: ADR-034rzTJYDRmre6D5OS8HDW(canonical
  `RoutingRequest` / per-target adapter)、ADR-034H2WBlp8VXoEE7KVlGt5
  (range / search request)、ADR-034r05iHrZUE6WizQ4LgPa(copilot
  never-nonzero)、ADR-034H2WBNtC4We8dyC5lQZJ(hook-config
  lifecycle / legacy gate)、ADR-034r05iHrZUE6WljM1ZMiX(usage.jsonl
  best-effort append)、SPEC-034H2WAU6TQG6N9dmmmKNA(bounded hook
  evaluation)。
* cache poisoning: `<userConfigDir>` は user-trusted 境界であり
  悪意ある書き込みは threat model 外とするが、verdict record の
  schema 検証(parse 失敗・未知 `schemaVersion`・TTL 超過は破棄
  して再評価)で堅牢化する。なお本 hook の guard は security
  boundary ではなく cost/routing 制御(delegation 誘導)であるため、
  forged verdict の影響は読み取り誘導の強弱に限定される。

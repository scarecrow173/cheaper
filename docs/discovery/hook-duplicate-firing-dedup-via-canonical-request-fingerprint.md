---
id: "DISC-034IdYnTDqD7Ao1nG3aoY9"
type: "discovery"
status: "resolved"
title: "Hook duplicate firing dedup via canonical request fingerprint"
created: "2026-10-18"
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
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/apm-distributed-devin-native-hook-bootstrap.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
    - "../adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
    - "../specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "./bounded-hook-read-request-contract.md"
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/mod.rs"
    - "crates/cheaper-cli/src/hook_config.rs"
  defers: []
  deferred-by: []
---

# Hook duplicate firing dedup via canonical request fingerprint

## Exploration Goal

Devin CLI は importer 経由で他エージェントの hook 設定(`.claude/settings.json`・`.windsurf/hooks.json` 等)を merge し、自セッションの event として発火する。その結果、1 回の tool call に対して複数の cheaper hook 登録が発火し得、各発火は独立した `cheaper hook` プロセスを spawn して個別に file token 計測と bounded materialization を実行する — 登録数に比例した冗長コストになる(issue #127)。

本 discovery が扱う問い: **複数の firing path を単一の評価結果に集約し、冗長な計測・実体化を dedup するにはどうするか。**

なぜ今か: Devin native 登録(`.devin/config.json`、canonical matcher `read|grep|exec`)を進めると、native 系統と import された claude 系統の両方が live firing になる状態が確定的であり、単一登録 gate だけではこの重複を吸収できない。

## Key Issues and Assumptions

- **import-path firing は end-to-end で実動作する(confirmed、2026-10-18/19 live 検証)**: code-knowledge の Devin session が hooks を `by_provider=[claude=1,devin=8,windsurf=2]` で load し、12,073 token の file read が hook deny → `cheaper bulk-read` delegation まで完走した。`usage.jsonl` に `kind:"run"`・`worker:"bulk-reader"` の record(2026-10-18T19:28:08Z)を確認済み。
- **windsurf 式 event は Devin 上で dormant と推定(hypothesis)**: Devin の hook event vocabulary は `PreToolUse` / `PostToolUse` / `UserPromptSubmit` / `Stop` / `SessionStart` 等であり、`pre_read_code` / `pre_run_command` 形式の windsurf event は発火しない可能性が高い(非 Windsurf target で verbatim `pre_read_code` を no-op とする `adr/keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md` の判断と整合)。
- **native 登録を追加すると live firing 系統が 2 本になる(confirmed 構造)**: `.devin/config.json` への native 登録後は、native(matcher `read|grep|exec`)と imported claude 登録が同一 tool call で発火し得る。
- **legacy gate の存在意義は重複発火の防止(confirmed)**: `hook-config sync` は legacy file(`.claude/settings.json` 等)に cheaper の read-hook 登録(`KNOWN_LEGACY_FILES` + `CONFLICT_PATTERN`、`crates/cheaper-cli/src/hook_config.rs`)を検出すると sync を拒否する。dedup 機構が存在すれば重複発火は無害化するため、gate を blocker → warning へ降格し native + claude import の共存を support できる可能性がある(hypothesis)。
- **verdict は request + file state の決定的関数という前提(要検証)**: `evaluate`(`routing_policy.rs`)の `Verdict` は canonical `RoutingRequest`・対象 file の状態・threshold で一意に決まり、target / source_tool / session に依存しない。この決定性が cache reuse を「近似」ではなく「厳密に正しい」ものにする前提条件であり、env・cwd・policy 入力の取りこぼしがないかを spec/adr で確認する必要がある。
- **fail-open 制約(confirmed)**: copilot adapter は「nonzero exit が deny と見なされ得る」ため stdout JSON + 常に exit 0 の contract(`adr/copilot-adapter-separate-script-never-nonzero-exit.md`)。cache 経路の失敗が異常終了や deny 相当の挙動を引き起こしてはならず、cache-path の failure は全て fail-open(通常評価へ fall back)である必要がある。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) verdict cache(canonical request fingerprint + leader election)を normalize → evaluate の間に挿入 | N firing → 1 計測 + N-1 cache hit。同一 file の連続 read にも効く。agent には N 件の同一 verdict が返るため host 観測挙動は不変。correctness を保ったままコストだけ削減できる | lock・TTL・sweep の実装複雑性。fingerprint 設計(決定性の前提)の正しさが要件 | yes |
| (b) 現状維持 — 単一登録 gate(legacy file 検出時に sync 拒否) | 変更不要。重複発火そのものを未然に防ぐ | multi-agent 環境で「どこに登録するか」の選択をユーザーへ強制し続ける。Devin import は cheaper 側が制御不能で、native 登録と import 系統の共存を阻害する | no |
| (c) Devin 側 import dedup(発火元で重複を統合) | 責務の layer としては正しい(発火の重複は host が作っている) | Devin 側の変更が必要で cheaper 単独では実現不能。upstream 待ちになる | maybe(長期) |
| (d) 2 発目以降の firing で abstain / allow を返す | 実装が軽い | host 側の verdict 合成順・AND/OR semantics が未知で、deny を弱める(allow へ倒す)リスクがあるため **却下** | no(rejected) |

## Tentative Conclusions and Hypotheses

最有力 hypothesis は **normalize → evaluate の間に verdict cache を挿入する (a)** である。構成要素:

- **fingerprint**(hypothesis): `hash(canonical RoutingRequest + file stat signature(mtime + size) + threshold 値)`。verdict が request + file 状態の決定的関数である限り、cache 適用は近似ではなく厳密に正しい。key に target / source_tool / session を**含めない**(含めると firing 間で key が割れて dedup 効果が消える)。
- **store**(hypothesis): `<userConfigDir>/hook-eval/<fingerprint>.json`(TTL ~60 秒を暫定指針)。`usage.jsonl` と同じ user-config 境界・best-effort write で、workspace や `.cheaper/` には書かない。
- **leader election**(hypothesis): `<fingerprint>.lock` を atomic `create_new` で取得。winner が evaluate → verdict 書き込み → release。follower は bounded wait で verdict file を poll し、timeout / stale lock / fs 失敗時は自前評価へ fall back する(correctness は常に維持)。
- **cache するのは canonical verdict**(hypothesis): 各 firing は自 target の adapter で render する(windsurf=stderr+exit2 / pretooluse=`hookSpecificOutput` / devin=`decision:block` / cursor=permission JSON)ため、per-target の出力契約は侵さない。
- **safety**(hypothesis): 全ての fs 失敗・stale lock・orphan entry は通常評価へ fall back(fail-open)。copilot の「nonzero exit = deny」と見なされ得る経路を新たに作らない。

効果と限界(見込み、spec で検証対象):

- N firing → 計測 1 回 + cache hit N-1 回。同一 file の連続 read(別 agent 由来を含む)にも効果がある。
- agent は引き続き N 件の同一 verdict を受け取る — host 観測挙動は不変。
- 削減できるのは計測・実体化コストのみで、`cheaper hook` プロセス spawn 自体の N 回分のオーバーヘッドは残る。

## Open Questions

- verdict cache の TTL 値(60 秒は暫定指針。importer firing の時間分散実測や、連続 read の実 interval から根拠を決める必要がある)。
- clock skew / staleness の扱い(mtime ベース判定で host clock ずれが起きた場合の許容方針)。
- lock contention の bounded wait 上限(N 並列 firing が leader の評価完了をどれだけ待つか。hook timeout 予算との兼ね合い)。
- legacy gate の blocker → warning 降格を同一 change に含めるか、dedup 実証後の follow-up に分けるか。
- fast successive write に対する mtime granularity リスク(同一 fingerprint 内で file が変わるケースをどこまで stat signature で吸収できるか)。
- cache entry の cleanup/sweep 起点(hook 起動時の best-effort sweep か、別 trigger か)。
- cache poisoning リスク評価(`<userConfigDir>` は user-trusted 境界内だが、悪意ある書き込みで verdict を汚染できるかの確認)。

## Promotion Candidates

- [ ] spec-doc needed: verdict cache の要件・受入条件(fingerprint 構成・TTL・leader/follower 挙動・fail-open 規約・per-target render 分離)を formalize する
- [ ] adr-doc needed: fingerprint による dedup + leader election というアーキテクチャ判断(cache 層の位置づけ、key 設計、lock 方式、store 配置)を記録する

## Sources and Relations

- issue #127: <https://github.com/scarecrow173/cheaper/issues/127>
- live 検証メモ(2026-10-18/19): Devin session での `by_provider=[claude=1,devin=8,windsurf=2]` hook load、12,073 token file read の deny → `cheaper bulk-read` delegation、`usage.jsonl` の `kind:"run"` / `worker:"bulk-reader"` record(2026-10-18T19:28:08Z)
- upstream discovery: `bounded-hook-read-request-contract.md`(evaluate pipeline の定義元)、`cross-agent-hook-distribution-and-contract-matrix.md`(per-agent hook contract matrix)
- 関連 spec/ADR: `specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`、`adr/apm-distributed-devin-native-hook-bootstrap.md`、`adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`、`adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md` 他(relations 参照)
- code: `crates/cheaper-cli/src/hook/{mod,targets,routing_policy}.rs`、`crates/cheaper-cli/src/hook_config.rs`(`KNOWN_LEGACY_FILES` + `CONFLICT_PATTERN`)

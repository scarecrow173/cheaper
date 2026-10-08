---
id: "DISC-034r05iHy2gdnpb598KIca"
type: "discovery"
status: "resolved"
title: "bulk-read delegation benchmark and large-read threshold evidence"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "./v2-productization-initiative-briefing.md"
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
    - "../adr/default-large-read-threshold.md"
    - "../specs/apm-delegation-skills-and-read-hook.md"
  refines: []
  refined-by: []
  derives-from:
    - "./v2-productization-initiative-briefing.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "bulk-read 委譲で実際にどれだけ parent context/token を削減できるか。大きなファイルの threshold として何 token が合理的か"
    scope: "本リポジトリの tracked file 分布 + Ollama 実 provider での bulk-read 実測"
    outcome: "実測に基づき default threshold 8192 tokens を維持確定（ADR-034r05iHrZUE6W4XOqUBih）"
---

# DISC-034r05iHy2gdnpb598KIca: bulk-read delegation benchmark and large-read threshold evidence

## 背景

`pre_read_code` hook の `CHEAPER_READ_TOKEN_THRESHOLD` default 8192 は
実測前の provisional 値だった（DESIGN-034r05iHy2gdnoFHbdllXV）。本 discovery は
「何 token 以上を large file として扱うか」の実測根拠を取り、
threshold を確定する。

## 方法

1. **コーパス分布**: 本リポジトリの `git ls-files`（`.apm/hooks/cli/` の
   生成 bundle を除く 293 file）を `countFileTokens` で計測し、
   ファイル数・token 分布・percentile・閾値別カバレッジを集計。
2. **実委譲測定**: Ollama `qwen3.5:4b`（実 provider、cost 0）で
   `bulk-read` を 2 件実行し、file tokens・parent が受け取る result
   tokens・worker usage・wall time を計測。
   設定は `docs/impl/exp/ollama-real-provider-smoke-verification.jsonl` と同一（`CHEAPER_CONFIG_DIR` で smoke config を指定）。

## 結果 1: コーパス分布（cheaper 本体、293 files / 417k tokens）

| percentile | tokens |
|---|---|
| p50 | 699 |
| p75 | 1,295 |
| p90 | 1,974 |
| p95 | 2,814 |
| p99 | 6,267 |

| threshold | files over | corpus tokens over |
|---|---|---|
| > 2,048 | 27 (9.2%) | 50.7% |
| > 4,096 | 6 (2.0%) | 36.7% |
| > 8,192 | 2 (0.7%) | 31.7% |
| > 16,384 | 2 (0.7%) | 31.7% |

観察: 本リポジトリで 8k 超は lockfile（`apm.lock.yaml` 94k,
`pnpm-lock.yaml` 38k）のみ。通常 source/docs は p95 ≈ 2.8k。

## 結果 2: 実委譲測定（Ollama qwen3.5:4b, reasoning_effort:none, temp 0）

| file | file tokens | parent 受信 result tokens | 削減率 | worker total tokens | wall |
|---|---|---|---|---|---|
| `designs/0004` (md) | 3,013 | ~1,230 | 59% | 5,584 | 15.8s |
| `handoff` (md) | 6,267 | ~260 | 96% | 4,582 | 14.1s |

- parent が送る task JSON は ~30-40 tokens で result より遥かに小さい。
- worker input tokens は file tokens より小さい場合がある
  （worker の read tool が長大 file を truncate するため）。summary 目的では
  部分 read でも有用な回答が得られた。
- 2 件とも exit 0・schema 適合（repair ≤1）。

## 分析

### コスト損益分岐

frontier ~$2.5/Mtok・cheap ~$0.4/Mtok（OpenAI 系の代表的比 6-10x）と仮定:

- direct read: `T × 2.5e-6`
- delegated: `T × 0.4e-6 (worker in) + workerOut × 0.4e-6 + (task+result) × 2.5e-6`
- break-even T ≈ 850-3,100 tokens（result サイズ依存、典型 ~2,000）

純コストだけなら ~2k で既に delegated が得になる。

### 非コスト要因が binding

- **latency**: bulk-read は ~14-16s。直接 read は ~数十 ms。
  小さい file で毎回 block されると開発フローが破壊される。
- **false positive の害**: threshold 以下でも上でも、parent が file の
  正確な内容を必要とする場面（編集対象 source の精読）は存在する。
  4-8k 帯は code-writer シナリオの編集対象として普通に出現し、
  そこを block すると「要約では足りず直接読みたい」往復が発生する。
- **8k 超の意味**: 8k tokens ≈ 32KB。どの基準でも明確に「大きい」file で、
  要約目的の read が典型。false positive 率が実質最小。
- **役割分担**: 積極的委譲は parent が skill（cheaper-bulk-read）を
  自発的に使う経路で行う。hook は safety net として
  「明確に大きい file だけ」ブロックする位置付けが設計意図と整合する。

## 結論

default `CHEAPER_READ_TOKEN_THRESHOLD` = **8192 tokens** を確定。
根拠と trade-off は ADR-034r05iHrZUE6W4XOqUBih に記録。env 上書きで per-deployment
調整可能なため、将来 telemetry で下げる判断も残る。

## Evidence

- `docs/impl/exp/bulk-read-delegation-benchmark.jsonl`
- `tmp/bench/ws/run1.json`, `run2.json`（envelope 実測値）

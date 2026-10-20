---
id: "TSPEC-034Jg5XonLxlwll8pJ15V0"
type: "test-spec"
status: "draft"
title: "Hook to route run end-to-end delegation"
created: "2026-10-20"
updated: "2026-10-21"
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
  implements: []
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
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Hook to route run end-to-end delegation

## Purpose

PreToolUse firing → rewrite → `route run` → workflow exec →
exit mirror の e2e が spec R1-R40 の end state を満たすことを
pin する。

## Feature

対象 command が hook で rewrite され delegated workflow で実行される。

## Rules

- hook が発行する payload が direct CLI `route run --payload` と
  同一 contract で処理される。
- hook eval の resolve が `route resolve` CLI の出力と一致する。
- e2e で cwd・env-prefix・bounded output・exit code が保持される。
- 統合経路は組み込み `routed-exec.workflow.json` を改変せず利用する。
  analyze job の無い専用 fixture で conditional worker の skip を
  証明したことにしてはならない。
- hook で発行した payload の取得後に grant が失効した場合、
  `route run` は policy error となり command を一度も実行しない。
- 親 CLI の実 stdin pipe に値があっても routed child は EOF を受ける。
  in-process stdin mock のみでは OS stdin の非接続を証明しない。
- worker の route log read は取得 text の実 token 計測で観測する。
  固定の fake summary が出ただけでは read 成功と判定しない。
- fallback verdict を持つ non-rewrite target が従来挙動を維持する。
- 全 spec R と ADR verification item が少なくとも 1 test へ写像済み。

## Examples

- fixture grant pin command を hook eval → updatedInput.command →
  route run 実行 → 同一 exit・出力を得る。
- hook payload を取得 → grant 削除 → 同じ token で `route run` →
  policy error、marker / route record は生成されない。
- 子の出力 cap を超える tail は親返却から切り落とされても、route log と
  bounded refetch には残る。
- condense worker が log を read → `sourceTokens` が投入 text の
  canonical token 数と一致。read cap を request 未満にすると実 tool call
  は観測されるが `sourceTokens` は 0 となる。

## Guarantees

- hook 経由と direct CLI で resolve/run の乖離が無い。

## Non-goals

- GUI/実 provider E2E(ollama_e2e 相当)。

## Risk

- e2e 欠如は各層の単体 pass にも関わらず統合で破綻する。

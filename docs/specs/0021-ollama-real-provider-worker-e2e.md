---
id: "SPEC-0021"
type: "spec"
status: "approved"
title: "Real-provider E2E for built-in workers via Ollama container"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/12"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/0026-ollama-real-provider-worker-e2e.md"
  depends-on:
    - "./0004-ollama-real-provider-smoke-verification.md"
    - "./0016-separate-dev-apm-project-from-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/0009-use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
    - "../impl/ir/0008-run-real-provider-smoke-test-with-ollama.md"
    - "../designs/0018-cross-agent-distribution-verification.md"
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
  spec:
    priority: "should"
---

# Real-provider E2E for built-in workers via Ollama container

## What Should Be Built

`bulk-read`・`code-write`・`doc-write` の各 built-in worker が、
`apm install` で deploy された bundled CLI 経由で実 provider
（Docker Ollama コンテナ）へ実到達し、envelope・usage・成果物を
返すことを自動検証する e2e test。Ollama 未到達時は skip し、CI
必須ゲートから分離する。

## Why Now

現行 e2e は apm install・delegation-loop・hook block までを隔離環境で
検証するが、worker が実 model を呼ぶ経路は `CHEAPER_TEST_FAKE_STREAM`
stub でしか検証されていない。IMPL-0008 は manual smoke（記録のみ・
再実行不可）で `bulk-read`/`cheap-write` の Ollama 到達を一度確認
したが、自動化・再現化されていない。Issue #12 がこの gap を指摘する。

## Users and Value

- 開発者: worker の実 provider 経路（model resolve → Pi provider →
  structured output → repair）を壊す変更を merge 前に検出できる。
- consumer: 配布 bundle 内の CLI が real provider で動くことが
  継続的に保証される。

## Scope

### In Scope

- Ollama 到達性・model 有無の auto-detect skip（opt-in flag 不要 —
  未起動時は deterministic skip）。
- `apm install` した隔離 consumer 環境での bundled CLI 実行
  （repo 内 `.apm/hooks`・cheaper-* skill を使わない — 開発中 hook
  の誤作動防止、issue の指定）。
- Pi custom provider 用 `models.json`・`models.yaml`（overrides +
  preferences）・`policy.yaml`（timeout・maxTurns・repairAttempts
  延長）の test fixture 生成 — IMPL-0008 の実測済み recipe を
  再現する。
- `bulk-read`・`code-write`・`doc-write` の実 provider 実行と
  envelope/meta/成果物の assert。

### Out of Scope

- CI での Ollama コンテナ起動（CI ゲートから分離 — ADR-0009 方針）。
- provider 品質・出力内容の評価（到達と契約 shape のみ検証。
  小モデルの出力品質は assertion 対象外）。
- Ollama container・model pull の自動化（手順は AGENTS.md 既存。
  test は「あれば使う」形）。
- token/cost 計測の記録基盤（Issue #13 の scope）。

## Requirements

1. **skip 条件**: Ollama `/v1/models` 未到達、または対象 model が
   pull 済み一覧に無い場合、test は skip する（fail にしない）。
   apm CLI 不在時も skip（既存 fixture と同じ）。
2. **隔離**: consumer workspace・Pi agent dir・cheaper config dir は
   tmpdir 配下に生成し、実 HOME/実 user config・repo の `.apm/hooks`
   を参照しない。
3. **実到達の証明**: `meta.model.provider` が models.json で定義した
   provider id、`meta.usage` の token 数が正の実測値であることを
   assert する。
4. **worker 成果物**: bulk-read は findings、code-write・doc-write は
   `meta.changes` と実 file 作成を assert する。
5. **再現性**: IMPL-0008 で確立した recipe（`reasoning_effort:
   "none"`・`temperature: 0`・policy 延長・overrides/preferences
   による tier pin）を fixture として固定する。model 名は env で
   override 可能にする（default `qwen3.5:9b`）。
6. **回帰安全**: Ollama 不在環境（CI 含む）では test file 全体が
   skip となり `pnpm test`・`mise run check` の既存 contract を
   変えない。

## Acceptance Criteria

- [ ] Ollama 起動 + model pull 済み環境で 3 worker の e2e が pass
      する（envelope ok・provider id・usage 実測・成果物 assert）。
- [ ] Ollama 未起動環境では同 test が skip し、suite 全体は green。
- [ ] test が repo の `.apm/hooks`・cheaper-* skill・実 HOME 配下の
      config に依存しない。
- [ ] `pnpm typecheck`・`pnpm lint`・`pnpm test`（Ollama 不在時の
      skip 経路含む）が pass する。

## Sources and Relations

- Issue #12・ADR-0009・IMPL-0008: 方針と実測 recipe の起点。
- SPEC-0004: manual smoke の spec（本仕様はその自動化・拡張）。
- SPEC-0016 / DESIGN-0021: consumer fixture は `packages/apm` を
  コピーする（dev/distribution 分離後の layout）。
- DESIGN-0018: consumer e2e fixture（`makeConsumerLayout`・
  `apmInstall`・`bundledCli`）を再利用する。

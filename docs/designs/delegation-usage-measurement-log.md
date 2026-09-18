---
id: "DESIGN-034r05iHy2gdnoXu6h71XL"
type: "design"
status: "approved"
title: "Delegation usage measurement log (usage.jsonl)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/delegation-usage-measurement-log.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/delegation-usage-measurement-log.md"
  implemented-by:
    - "../plans/delegation-usage-measurement-log.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./cli-contract-envelope-exit-codes.md"
    - "./ollama-real-provider-worker-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Delegation usage measurement log (usage.jsonl)

## Context

SPEC-034r05iHK8bA7gveeDA5HC / ADR-034r05iHrZUE6WljM1ZMiX の実装設計。CLI の envelope 確定点で
usage 実測（run）・事前見積（count-tokens）を `<userConfigDir>/
usage.jsonl` へ best-effort append する。stdout contract・exit
code は不変。

## Scope

### In Scope

- `packages/cli/src/usage-log.ts`（新規）: record 構築と append
- `packages/cli/src/cli.ts`: run・count-tokens 経路からの呼出し
- `packages/cli/test/usage-log.test.ts`（新規）: record shape・
  append・失敗時 swallow の unit test
- `packages/cli/test/e2e/exec.test.ts`（または新規 e2e）:
  `CHEAPER_CONFIG_DIR` 隔離下での log append assert
- `packages/cli/test/e2e/real-provider-ollama.test.ts`: run record
  の実測 assert 追加
- `AGENTS.md`: 計測 log の場所・record shape の簡潔な記載

### Out of Scope

- 集計・summary コマンド、rotation（follow-up）
- envelope schema 自体の変更
- runtime-pi / core の変更（usage は既に `meta.usage` で取得済み）

## Design

### 配置と呼出し点

`runCli`（`packages/cli/src/cli.ts`）は全 envelope の単一 choke
point。envelope を stdout に書く直前に `recordUsage()` を呼ぶ:

- `count-tokens` 分岐: utility envelope 確定後に `kind:"count"`
  record（path/tokens/bytes を result から採る）。
- `run` 成功: `executeRun` が返す success envelope から
  `kind:"run"` record（meta.model・meta.usage・durationMs）。
- `run` error: `runCli` の catch で組み立てる error envelope に
  `meta.usage` があれば `kind:"run"`・`ok:false`・`error.code`
  を記録（WorkerRunError 由来の部分 usage も費用実績として価値
  がある）。
- auth・help・version は記録しない。

`root`（workspace abs path）は `executeRun` 内で canonicalize
済み — `workerName` と同じ out-param で `runCli` へ返し、error
path でも record に含められるようにする。count-tokens 経路は
分岐内で解決済みの `root` をそのまま使う。

### `usage-log.ts`

```ts
export interface UsageLogRecord {
  schemaVersion: 1;
  ts: string;                 // ISO 8601
  kind: "run" | "count";
  worker: string | null;      // count は null
  root: string;               // workspace abs path
  ok: boolean;
  durationMs: number | null;
  model?: { provider: string; id: string; tier: string };
  usage?: WorkerUsage;
  error?: { code: string };
  result?: { path?: string; tokens?: number; bytes?: number };
}

export function buildUsageRecord(
  env: CliEnvelope, ctx: { root: string },
): UsageLogRecord | null;

export function appendUsageLog(opts: {
  userConfigDir: string;
  record: UsageLogRecord;
}): void;  // throws しない設計ではなく、呼出し側が catch
```

- `buildUsageRecord` は envelope から record を構築。計測対象外
  （auth の utility envelope 等、情報不足時）は `null` を返し
  呼出し側は何もしない。
- `appendUsageLog` は `mkdirSync(userConfigDir, {recursive:true})`
  + `appendFileSync(<dir>/usage.jsonl, JSON.stringify(record)+"\n")`。
- 呼出し側（cli.ts）は `try/catch` で囲み、失敗時は `quiet` で
  なければ stderr に `usage-log: <message>` 1 行。exit code・
  stdout には一切触れない。
- DI: unit test 向けに file 書き込みは `appendUsageLog` に集約し、
  path・内容は引数注入。時刻は `hooks.now` ではなく record 生成
  時の `new Date().toISOString()`（test では injectable clock を
  `buildUsageRecord` の ctx に含められるよう `now?: () => Date`
  を受ける）。

### 実装位置の根拠

- core ではなく cli 層: 記録は「CLI 実行の副作用」であり、
  envelope 型（cli 層）への依存が自然。core/runtime-pi は
  変更しない（`meta.usage` をそのまま利用）。
- bundled CLI に含まれるため、consumer 配布でも同じ計測が
  働く（skill 経由の delegation も自動で記録される）。

## Validation

- unit test（`usage-log.test.ts`）:
  - success run envelope → `kind:"run"`・model/usage 転写
  - error envelope（meta.usage あり）→ `ok:false`・`error.code`
  - count-tokens utility envelope → `kind:"count"`・tokens/bytes
  - 対象外 envelope → `null`
  - `appendUsageLog` が 1 行 JSONL を append し、2 回目が追記に
    なること
  - 書き込み不可 path でも呼出し側が swallow すること（cli.ts
    経由の e2e または seam test で exit code/stdout 不変を確認）
- e2e（fake-stream）: `CHEAPER_CONFIG_DIR` tmpdir 化した consumer
  run で `usage.jsonl` に run record があること。count-tokens で
  count record があること。
- real-provider e2e: 既存 test に「`usage.jsonl` に
  `kind:"run"`・`model.provider==="ollama"`・`usage.input>0` の
  record が append される」assert を追加。
- `mise run check`: lint・typecheck・全 test・bundle freshness。

## Risks

- 複数 process の同時 append: 行単位 append は POSIX では実質
  安全、Windows でも短い write で interleave 稀。破損行が混じっ
  ても consumer 側は skip 可能（per-line parse）。rotation 無し
  のため長期で肥大化し得る — follow-up で対処。
- `userConfigDir` 未存在環境で `mkdir -p` が副作用として dir を
  作る — config と同じ場所なので許容（記録しない選択肢より
  継続計測を優先）。
- record の `root` に workspace path が残る — user 自身の log
  dir 内のみで、外部送信は無い。
